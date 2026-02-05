//! Migration engine for moving workloads between runtimes

use crate::runtime::{Instance, Runtime, RuntimeKind};
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

/// Migration strategy
#[derive(Debug, Clone, PartialEq)]
pub enum MigrationStrategy {
    /// Immediate migration (stops source, starts target)
    Immediate,
    /// Blue-Green deployment (start target, switch traffic, stop source)
    BlueGreen,
    /// Rolling migration (gradual transition with validation)
    Rolling,
}

/// Migration plan
#[derive(Debug, Clone)]
pub struct MigrationPlan {
    pub workload_name: String,
    pub source_runtime: RuntimeKind,
    pub target_runtime: RuntimeKind,
    pub strategy: MigrationStrategy,
    pub validation_delay: Duration,
    pub rollback_on_failure: bool,
}

/// Migration result
#[derive(Debug)]
pub struct MigrationResult {
    pub success: bool,
    pub source_instance: Option<Instance>,
    pub target_instance: Option<Instance>,
    pub error: Option<String>,
    pub rollback_performed: bool,
}

/// Migration engine
pub struct MigrationEngine {
    state_path: PathBuf,
}

impl MigrationEngine {
    /// Create new migration engine
    pub fn new(state_path: PathBuf) -> Self {
        Self { state_path }
    }

    /// Execute migration
    pub async fn migrate(&self, plan: MigrationPlan) -> Result<MigrationResult> {
        tracing::info!(
            "Starting migration: {} from {} to {}",
            plan.workload_name,
            plan.source_runtime,
            plan.target_runtime
        );

        match plan.strategy {
            MigrationStrategy::Immediate => self.migrate_immediate(plan).await,
            MigrationStrategy::BlueGreen => self.migrate_blue_green(plan).await,
            MigrationStrategy::Rolling => self.migrate_rolling(plan).await,
        }
    }

    /// Immediate migration strategy
    async fn migrate_immediate(&self, plan: MigrationPlan) -> Result<MigrationResult> {
        tracing::info!("Using immediate migration strategy");

        // Load current state
        let mut state = StateStore::load(&self.state_path)?;
        let workload_state = state
            .get(&plan.workload_name)
            .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", plan.workload_name))?
            .clone();

        // Verify source runtime matches
        if workload_state.runtime != plan.source_runtime {
            return Err(anyhow::anyhow!(
                "Source runtime mismatch: expected {}, found {}",
                plan.source_runtime,
                workload_state.runtime
            ));
        }

        // Load workload spec
        let workload = Workload::from_file(&workload_state.spec_path)?;

        // Get source runtime
        let source_runtime = self.get_runtime(&plan.source_runtime).await?;

        // Stop source instance
        tracing::info!("Stopping source instance on {}", plan.source_runtime);
        source_runtime.stop(&workload_state.instance).await?;

        // Wait for graceful shutdown
        tokio::time::sleep(Duration::from_secs(5)).await;

        // Get target runtime
        let target_runtime = self.get_runtime(&plan.target_runtime).await?;

        // Build image for target runtime
        tracing::info!("Building image for {}", plan.target_runtime);
        let image = target_runtime.build(&workload).await?;

        // Deploy to target runtime
        tracing::info!("Deploying to {}", plan.target_runtime);
        let target_instance = match target_runtime.run(&image, &workload).await {
            Ok(instance) => instance,
            Err(e) => {
                // Rollback if requested
                if plan.rollback_on_failure {
                    tracing::warn!("Deployment failed, rolling back to source runtime");
                    let rollback_image = source_runtime.build(&workload).await?;
                    let source_instance = source_runtime.run(&rollback_image, &workload).await?;

                    return Ok(MigrationResult {
                        success: false,
                        source_instance: Some(source_instance),
                        target_instance: None,
                        error: Some(format!("Deployment failed: {}", e)),
                        rollback_performed: true,
                    });
                } else {
                    return Err(e);
                }
            }
        };

        // Validate target instance
        tracing::info!("Validating target instance...");
        tokio::time::sleep(plan.validation_delay).await;

        let status = target_runtime.status(&target_instance).await?;
        if !status.ready {
            // Rollback if not ready and rollback enabled
            if plan.rollback_on_failure {
                tracing::warn!("Target instance not ready, rolling back");
                let _ = target_runtime.delete(&target_instance).await;
                let rollback_image = source_runtime.build(&workload).await?;
                let source_instance = source_runtime.run(&rollback_image, &workload).await?;

                return Ok(MigrationResult {
                    success: false,
                    source_instance: Some(source_instance),
                    target_instance: None,
                    error: Some("Target instance failed health check".to_string()),
                    rollback_performed: true,
                });
            } else {
                return Err(anyhow::anyhow!("Target instance not ready"));
            }
        }

        // Delete source instance
        tracing::info!("Cleaning up source instance");
        source_runtime.delete(&workload_state.instance).await?;

        // Update state
        state.upsert(
            plan.workload_name.clone(),
            WorkloadState {
                name: plan.workload_name.clone(),
                runtime: plan.target_runtime,
                instance: target_instance.clone(),
                spec_path: workload_state.spec_path,
                created_at: workload_state.created_at,
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
        );
        state.save(&self.state_path)?;

        tracing::info!("Migration completed successfully");

        Ok(MigrationResult {
            success: true,
            source_instance: None,
            target_instance: Some(target_instance),
            error: None,
            rollback_performed: false,
        })
    }

    /// Blue-Green migration strategy
    async fn migrate_blue_green(&self, plan: MigrationPlan) -> Result<MigrationResult> {
        tracing::info!("Using blue-green migration strategy");

        // Load current state
        let mut state = StateStore::load(&self.state_path)?;
        let workload_state = state
            .get(&plan.workload_name)
            .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", plan.workload_name))?
            .clone();

        // Load workload spec
        let workload = Workload::from_file(&workload_state.spec_path)?;

        // Get source and target runtimes
        let source_runtime = self.get_runtime(&plan.source_runtime).await?;
        let target_runtime = self.get_runtime(&plan.target_runtime).await?;

        // Deploy to target runtime (green) while source (blue) still running
        tracing::info!("Deploying to target runtime (green deployment)");
        let image = target_runtime.build(&workload).await?;
        let target_instance = match target_runtime.run(&image, &workload).await {
            Ok(instance) => instance,
            Err(e) => {
                return Ok(MigrationResult {
                    success: false,
                    source_instance: Some(workload_state.instance),
                    target_instance: None,
                    error: Some(format!("Green deployment failed: {}", e)),
                    rollback_performed: false,
                });
            }
        };

        // Wait for target to be ready
        tracing::info!("Waiting for green deployment to be ready...");
        tokio::time::sleep(plan.validation_delay).await;

        let status = target_runtime.status(&target_instance).await?;
        if !status.ready {
            tracing::warn!("Green deployment not ready, cleaning up");
            let _ = target_runtime.delete(&target_instance).await;

            return Ok(MigrationResult {
                success: false,
                source_instance: Some(workload_state.instance),
                target_instance: None,
                error: Some("Green deployment failed health check".to_string()),
                rollback_performed: false,
            });
        }

        tracing::info!("Green deployment healthy, switching traffic");

        // In a real implementation, you would:
        // 1. Update load balancer/service to point to green
        // 2. Wait for connection draining
        // 3. Monitor for errors
        // For now, we simulate a traffic switch delay
        tokio::time::sleep(Duration::from_secs(10)).await;

        // Stop and delete blue (source) deployment
        tracing::info!("Stopping blue deployment (source)");
        source_runtime.stop(&workload_state.instance).await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        source_runtime.delete(&workload_state.instance).await?;

        // Update state
        state.upsert(
            plan.workload_name.clone(),
            WorkloadState {
                name: plan.workload_name.clone(),
                runtime: plan.target_runtime,
                instance: target_instance.clone(),
                spec_path: workload_state.spec_path,
                created_at: workload_state.created_at,
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
        );
        state.save(&self.state_path)?;

        tracing::info!("Blue-green migration completed successfully");

        Ok(MigrationResult {
            success: true,
            source_instance: None,
            target_instance: Some(target_instance),
            error: None,
            rollback_performed: false,
        })
    }

    /// Rolling migration strategy
    async fn migrate_rolling(&self, plan: MigrationPlan) -> Result<MigrationResult> {
        tracing::info!("Using rolling migration strategy");

        // Load current state
        let mut state = StateStore::load(&self.state_path)?;
        let workload_state = state
            .get(&plan.workload_name)
            .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found", plan.workload_name))?
            .clone();

        // Load workload spec
        let workload = Workload::from_file(&workload_state.spec_path)?;

        // Get runtimes
        let source_runtime = self.get_runtime(&plan.source_runtime).await?;
        let target_runtime = self.get_runtime(&plan.target_runtime).await?;

        // Phase 1: Deploy to target
        tracing::info!("Phase 1: Deploying to target runtime");
        let image = target_runtime.build(&workload).await?;
        let target_instance = target_runtime.run(&image, &workload).await?;

        // Phase 2: Validate target
        tracing::info!("Phase 2: Validating target deployment");
        tokio::time::sleep(plan.validation_delay).await;

        let mut validation_attempts = 0;
        let max_attempts = 3;
        let mut target_ready = false;

        while validation_attempts < max_attempts {
            let status = target_runtime.status(&target_instance).await?;
            if status.ready {
                target_ready = true;
                break;
            }

            validation_attempts += 1;
            if validation_attempts < max_attempts {
                tracing::info!("Target not ready, retrying ({}/{})", validation_attempts, max_attempts);
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }

        if !target_ready {
            tracing::warn!("Target failed validation, rolling back");
            let _ = target_runtime.delete(&target_instance).await;

            return Ok(MigrationResult {
                success: false,
                source_instance: Some(workload_state.instance),
                target_instance: None,
                error: Some("Target failed validation after retries".to_string()),
                rollback_performed: false,
            });
        }

        // Phase 3: Gradual traffic shift (simulated)
        tracing::info!("Phase 3: Gradual traffic shift");
        for percentage in [25, 50, 75, 100] {
            tracing::info!("Shifting {}% traffic to target", percentage);
            tokio::time::sleep(Duration::from_secs(5)).await;

            // Verify target still healthy
            let status = target_runtime.status(&target_instance).await?;
            if !status.ready {
                tracing::error!("Target became unhealthy during traffic shift, rolling back");
                let _ = target_runtime.delete(&target_instance).await;

                return Ok(MigrationResult {
                    success: false,
                    source_instance: Some(workload_state.instance),
                    target_instance: None,
                    error: Some("Target became unhealthy during migration".to_string()),
                    rollback_performed: false,
                });
            }
        }

        // Phase 4: Cleanup source
        tracing::info!("Phase 4: Cleaning up source deployment");
        source_runtime.stop(&workload_state.instance).await?;
        tokio::time::sleep(Duration::from_secs(5)).await;
        source_runtime.delete(&workload_state.instance).await?;

        // Update state
        state.upsert(
            plan.workload_name.clone(),
            WorkloadState {
                name: plan.workload_name.clone(),
                runtime: plan.target_runtime,
                instance: target_instance.clone(),
                spec_path: workload_state.spec_path,
                created_at: workload_state.created_at,
                updated_at: chrono::Utc::now().to_rfc3339(),
            },
        );
        state.save(&self.state_path)?;

        tracing::info!("Rolling migration completed successfully");

        Ok(MigrationResult {
            success: true,
            source_instance: None,
            target_instance: Some(target_instance),
            error: None,
            rollback_performed: false,
        })
    }

    /// Get runtime instance
    async fn get_runtime(&self, kind: &RuntimeKind) -> Result<Box<dyn Runtime>> {
        match kind {
            RuntimeKind::Podman => {
                let runtime = crate::adapters::PodmanRuntime::new()?;
                Ok(Box::new(runtime))
            }
            RuntimeKind::Kubernetes => {
                let runtime = crate::adapters::KubernetesRuntime::new().await?;
                Ok(Box::new(runtime))
            }
            RuntimeKind::KubeVirt => {
                let runtime = crate::adapters::KubeVirtRuntime::new().await?;
                Ok(Box::new(runtime))
            }
            RuntimeKind::Metal3 => {
                let runtime = crate::adapters::Metal3Runtime::new().await?;
                Ok(Box::new(runtime))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_plan_creation() {
        let plan = MigrationPlan {
            workload_name: "test-app".to_string(),
            source_runtime: RuntimeKind::Podman,
            target_runtime: RuntimeKind::Kubernetes,
            strategy: MigrationStrategy::BlueGreen,
            validation_delay: Duration::from_secs(30),
            rollback_on_failure: true,
        };

        assert_eq!(plan.workload_name, "test-app");
        assert_eq!(plan.source_runtime, RuntimeKind::Podman);
        assert_eq!(plan.target_runtime, RuntimeKind::Kubernetes);
        assert_eq!(plan.strategy, MigrationStrategy::BlueGreen);
        assert!(plan.rollback_on_failure);
    }

    #[test]
    fn test_migration_strategies() {
        assert_eq!(
            MigrationStrategy::Immediate,
            MigrationStrategy::Immediate
        );
        assert_ne!(
            MigrationStrategy::Immediate,
            MigrationStrategy::BlueGreen
        );
    }
}
