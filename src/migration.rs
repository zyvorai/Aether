//! Migration engine for moving workloads between runtimes

use crate::runtime::{Instance, Runtime, RuntimeKind};
use crate::spec::Workload;
use crate::state::StateStore;
use anyhow::Result;
use std::path::PathBuf;
use std::time::Duration;

/// Migration strategy
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum MigrationStrategy {
    /// Immediate migration (stops source, starts target)
    Immediate,
    /// Blue-Green deployment (start target, switch traffic, stop source)
    BlueGreen,
    /// Rolling migration (gradual transition with validation)
    Rolling,
    /// Canary deployment (gradual traffic shift with health-gated steps)
    Canary,
}

/// Configuration for canary deployment steps
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CanaryConfig {
    /// Traffic percentage steps (e.g., [10, 25, 50, 75, 100])
    pub steps: Vec<u32>,
    /// Duration in seconds to observe each step before proceeding
    pub step_interval_secs: u64,
    /// Maximum error rate (0.0-1.0) before triggering rollback
    pub error_threshold: f64,
    /// Number of health checks at each step
    pub health_checks_per_step: u32,
}

impl std::fmt::Display for MigrationStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationStrategy::Immediate => write!(f, "immediate"),
            MigrationStrategy::BlueGreen => write!(f, "blue-green"),
            MigrationStrategy::Rolling => write!(f, "rolling"),
            MigrationStrategy::Canary => write!(f, "canary"),
        }
    }
}

impl std::str::FromStr for MigrationStrategy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "immediate" => Ok(MigrationStrategy::Immediate),
            "blue-green" | "bluegreen" => Ok(MigrationStrategy::BlueGreen),
            "rolling" => Ok(MigrationStrategy::Rolling),
            "canary" => Ok(MigrationStrategy::Canary),
            _ => Err(anyhow::anyhow!("Unknown migration strategy: '{}'. Valid: immediate, blue-green, rolling, canary", s)),
        }
    }
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
    /// Graceful shutdown delay before starting target (default: 5s)
    pub shutdown_delay: Duration,
    /// Delay between traffic-shift steps in rolling migration (default: 5s)
    pub traffic_shift_interval: Duration,
    /// Delay after stopping source before cleanup (default: 2s)
    pub cleanup_delay: Duration,
    /// Maximum health check attempts during rolling validation (default: 3)
    pub max_health_retries: u32,
    /// Interval between health check retries with exponential backoff (default: 2s base)
    pub health_retry_base_interval: Duration,
    /// Canary deployment configuration (required for Canary strategy)
    pub canary_config: Option<CanaryConfig>,
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

impl MigrationPlan {
    /// Create a new migration plan with sensible defaults for timing parameters.
    pub fn new(
        workload_name: String,
        source_runtime: RuntimeKind,
        target_runtime: RuntimeKind,
        strategy: MigrationStrategy,
        rollback_on_failure: bool,
    ) -> Self {
        Self {
            workload_name,
            source_runtime,
            target_runtime,
            strategy,
            validation_delay: Duration::from_secs(10),
            rollback_on_failure,
            shutdown_delay: Duration::from_secs(5),
            traffic_shift_interval: Duration::from_secs(5),
            cleanup_delay: Duration::from_secs(2),
            max_health_retries: 3,
            health_retry_base_interval: Duration::from_secs(2),
            canary_config: None,
        }
    }
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
        // Guard: reject same-runtime migration
        if plan.source_runtime == plan.target_runtime {
            return Err(anyhow::anyhow!(
                "Source and target runtimes are the same ({}). Migration is unnecessary.\n\
                 Hint: Use `aether rollback {}` to redeploy on the same runtime.",
                plan.source_runtime,
                plan.workload_name,
            ));
        }

        // Guard: reject empty workload name
        if plan.workload_name.is_empty() {
            return Err(anyhow::anyhow!("Workload name cannot be empty"));
        }

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
            MigrationStrategy::Canary => self.migrate_canary(plan).await,
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

        // Wait for graceful shutdown (configurable)
        tokio::time::sleep(plan.shutdown_delay).await;

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
                if let Err(e) = target_runtime.delete(&target_instance).await {
                    tracing::error!("Failed to delete target instance during rollback: {}", e);
                }
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
            workload_state.migrated(plan.target_runtime, target_instance.clone()),
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
            if let Err(e) = target_runtime.delete(&target_instance).await {
                tracing::error!("Failed to clean up failed green deployment: {}", e);
            }

            return Ok(MigrationResult {
                success: false,
                source_instance: Some(workload_state.instance),
                target_instance: None,
                error: Some("Green deployment failed health check".to_string()),
                rollback_performed: false,
            });
        }

        tracing::info!("Green deployment healthy, switching traffic");

        // Traffic switch: In Kubernetes, the Service selector already points to the app label,
        // which both blue and green deployments share. Once the green deployment's pods are ready
        // and the blue deployment is deleted, traffic naturally flows to green.
        // For explicit Service selector updates in complex scenarios, use `kubectl patch`.
        tracing::info!("Traffic switch: green deployment validated, proceeding to remove blue");
        let drain_delay = plan.validation_delay.min(std::time::Duration::from_secs(30));
        tracing::info!("Connection draining ({:?})", drain_delay);
        tokio::time::sleep(drain_delay).await;

        // Stop and delete blue (source) deployment
        tracing::info!("Stopping blue deployment (source)");
        source_runtime.stop(&workload_state.instance).await?;
        tokio::time::sleep(plan.cleanup_delay).await;
        source_runtime.delete(&workload_state.instance).await?;

        // Update state
        state.upsert(
            plan.workload_name.clone(),
            workload_state.migrated(plan.target_runtime, target_instance.clone()),
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

        // Phase 2: Validate target with exponential backoff
        tracing::info!("Phase 2: Validating target deployment");
        tokio::time::sleep(plan.validation_delay).await;

        let mut validation_attempts = 0;
        let max_attempts = plan.max_health_retries;
        let mut target_ready = false;
        let base_interval = plan.health_retry_base_interval;

        while validation_attempts < max_attempts {
            let status = target_runtime.status(&target_instance).await?;
            if status.ready {
                target_ready = true;
                break;
            }

            validation_attempts += 1;
            if validation_attempts < max_attempts {
                // Exponential backoff: base * 2^attempt, capped at 30s
                let backoff = base_interval
                    .mul_f64(2.0_f64.powi(validation_attempts as i32 - 1))
                    .min(Duration::from_secs(30));
                tracing::info!(
                    "Target not ready, retrying ({}/{}) in {:?}",
                    validation_attempts, max_attempts, backoff
                );
                tokio::time::sleep(backoff).await;
            }
        }

        if !target_ready {
            tracing::warn!("Target failed validation, rolling back");
            if let Err(e) = target_runtime.delete(&target_instance).await {
                tracing::error!("Failed to delete target instance during rollback: {}", e);
            }

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
            tokio::time::sleep(plan.traffic_shift_interval).await;

            // Verify target still healthy
            let status = target_runtime.status(&target_instance).await?;
            if !status.ready {
                tracing::error!("Target became unhealthy during traffic shift, rolling back");
                if let Err(e) = target_runtime.delete(&target_instance).await {
                    tracing::error!("Failed to delete target instance during rollback: {}", e);
                }

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
        tokio::time::sleep(plan.shutdown_delay).await;
        source_runtime.delete(&workload_state.instance).await?;

        // Update state
        state.upsert(
            plan.workload_name.clone(),
            workload_state.migrated(plan.target_runtime, target_instance.clone()),
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

    /// Canary migration strategy
    ///
    /// Deploys a canary instance alongside the stable instance and gradually
    /// validates it at each traffic percentage step. Rolls back if health
    /// checks fail at any step.
    async fn migrate_canary(&self, plan: MigrationPlan) -> Result<MigrationResult> {
        tracing::info!("Using canary migration strategy");

        let canary_config = plan.canary_config.clone().unwrap_or(CanaryConfig {
            steps: vec![10, 25, 50, 75, 100],
            step_interval_secs: 10,
            error_threshold: 0.1,
            health_checks_per_step: 2,
        });

        // Validate canary steps
        if canary_config.steps.is_empty() {
            anyhow::bail!("Canary config must have at least one step");
        }
        for (i, step) in canary_config.steps.iter().enumerate() {
            if *step > 100 {
                anyhow::bail!("Canary step {} has value {}%, which exceeds 100%", i + 1, step);
            }
            if i > 0 && *step <= canary_config.steps[i - 1] {
                anyhow::bail!(
                    "Canary steps must be strictly increasing: step {} ({}%) <= step {} ({}%)",
                    i + 1, step, i, canary_config.steps[i - 1]
                );
            }
        }

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

        // Phase 1: Deploy canary to target runtime
        tracing::info!("Phase 1: Deploying canary to target runtime");
        let image = target_runtime.build(&workload).await?;
        let canary_instance = match target_runtime.run(&image, &workload).await {
            Ok(instance) => instance,
            Err(e) => {
                return Ok(MigrationResult {
                    success: false,
                    source_instance: Some(workload_state.instance),
                    target_instance: None,
                    error: Some(format!("Canary deployment failed: {}", e)),
                    rollback_performed: false,
                });
            }
        };

        // Phase 2: Initial validation
        tracing::info!("Phase 2: Validating canary deployment");
        tokio::time::sleep(plan.validation_delay).await;

        let status = target_runtime.status(&canary_instance).await?;
        if !status.ready {
            tracing::warn!("Canary not ready, rolling back");
            if let Err(e) = target_runtime.delete(&canary_instance).await {
                tracing::error!("Failed to delete canary during rollback: {}", e);
            }
            return Ok(MigrationResult {
                success: false,
                source_instance: Some(workload_state.instance),
                target_instance: None,
                error: Some("Canary failed initial health check".to_string()),
                rollback_performed: true,
            });
        }

        // Phase 3: Step through traffic percentages with health validation
        tracing::info!("Phase 3: Canary traffic steps");
        let step_duration = Duration::from_secs(canary_config.step_interval_secs);

        for step_pct in &canary_config.steps {
            tracing::info!("Canary step: {}% traffic to target", step_pct);
            tokio::time::sleep(step_duration).await;

            // Perform health checks at this step
            let mut failures = 0;
            for check in 0..canary_config.health_checks_per_step {
                let status = target_runtime.status(&canary_instance).await?;
                if !status.ready {
                    failures += 1;
                    tracing::warn!(
                        "Canary health check {}/{} failed at {}% step",
                        check + 1, canary_config.health_checks_per_step, step_pct
                    );
                }
            }

            let failure_rate = if canary_config.health_checks_per_step > 0 {
                failures as f64 / canary_config.health_checks_per_step as f64
            } else {
                0.0
            };

            if failure_rate > canary_config.error_threshold {
                tracing::error!(
                    "Canary error rate {:.1}% exceeds threshold {:.1}% at {}% step, rolling back",
                    failure_rate * 100.0,
                    canary_config.error_threshold * 100.0,
                    step_pct
                );
                if let Err(e) = target_runtime.delete(&canary_instance).await {
                    tracing::error!("Failed to delete canary during rollback: {}", e);
                }
                return Ok(MigrationResult {
                    success: false,
                    source_instance: Some(workload_state.instance),
                    target_instance: None,
                    error: Some(format!(
                        "Canary failed at {}% step: error rate {:.1}% > threshold {:.1}%",
                        step_pct,
                        failure_rate * 100.0,
                        canary_config.error_threshold * 100.0
                    )),
                    rollback_performed: true,
                });
            }

            tracing::info!("Canary healthy at {}% step", step_pct);
        }

        // Phase 4: Full promotion — stop source, keep canary as new stable
        tracing::info!("Phase 4: Promoting canary, removing source");
        source_runtime.stop(&workload_state.instance).await?;
        tokio::time::sleep(plan.shutdown_delay).await;
        source_runtime.delete(&workload_state.instance).await?;

        // Update state
        state.upsert(
            plan.workload_name.clone(),
            workload_state.migrated(plan.target_runtime, canary_instance.clone()),
        );
        state.save(&self.state_path)?;

        tracing::info!("Canary migration completed successfully");

        Ok(MigrationResult {
            success: true,
            source_instance: None,
            target_instance: Some(canary_instance),
            error: None,
            rollback_performed: false,
        })
    }

    /// Get runtime instance
    async fn get_runtime(&self, kind: &RuntimeKind) -> Result<Box<dyn Runtime>> {
        crate::runtime::create_runtime(kind).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Instance;

    // ---------------------------------------------------------------
    // Helper: build a reusable MigrationPlan
    // ---------------------------------------------------------------
    fn make_plan(
        name: &str,
        source: RuntimeKind,
        target: RuntimeKind,
        strategy: MigrationStrategy,
        rollback: bool,
    ) -> MigrationPlan {
        MigrationPlan::new(
            name.to_string(),
            source,
            target,
            strategy,
            rollback,
        )
    }

    // Helper: build a test Instance
    fn make_instance(id: &str, name: &str, runtime: RuntimeKind) -> Instance {
        Instance {
            id: id.to_string(),
            name: name.to_string(),
            runtime,
            image: format!("{}:latest", name),
            created_at: "2024-06-01T00:00:00Z".to_string(),
        }
    }

    // ---------------------------------------------------------------
    // MigrationPlan construction
    // ---------------------------------------------------------------

    #[test]
    fn test_migration_plan_creation() {
        let plan = MigrationPlan::new(
            "test-app".to_string(),
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::BlueGreen,
            true,
        );

        assert_eq!(plan.workload_name, "test-app");
        assert_eq!(plan.source_runtime, RuntimeKind::Podman);
        assert_eq!(plan.target_runtime, RuntimeKind::Kubernetes);
        assert_eq!(plan.strategy, MigrationStrategy::BlueGreen);
        assert!(plan.rollback_on_failure);
        // Verify defaults
        assert_eq!(plan.shutdown_delay, Duration::from_secs(5));
        assert_eq!(plan.traffic_shift_interval, Duration::from_secs(5));
        assert_eq!(plan.cleanup_delay, Duration::from_secs(2));
        assert_eq!(plan.max_health_retries, 3);
    }

    #[test]
    fn test_migration_plan_all_runtime_combinations() {
        let runtimes = [
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            RuntimeKind::KubeVirt,
            RuntimeKind::Metal3,
        ];

        for source in &runtimes {
            for target in &runtimes {
                let plan = make_plan("app", *source, *target, MigrationStrategy::Immediate, false);
                assert_eq!(plan.source_runtime, *source);
                assert_eq!(plan.target_runtime, *target);
            }
        }
    }

    #[test]
    fn test_migration_plan_same_source_and_target() {
        // Same-runtime plans can still be constructed (guard is in migrate())
        let plan = make_plan(
            "loop-app",
            RuntimeKind::Podman,
            RuntimeKind::Podman,
            MigrationStrategy::Immediate,
            true,
        );
        assert_eq!(plan.source_runtime, plan.target_runtime);
    }

    #[tokio::test]
    async fn test_migrate_rejects_same_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let engine = MigrationEngine::new(state_path);
        let plan = make_plan(
            "same-rt-app",
            RuntimeKind::Podman,
            RuntimeKind::Podman,
            MigrationStrategy::Immediate,
            false,
        );
        let err = engine.migrate(plan).await.unwrap_err();
        assert!(err.to_string().contains("same"));
    }

    #[tokio::test]
    async fn test_migrate_rejects_empty_name() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let engine = MigrationEngine::new(state_path);
        let plan = make_plan(
            "",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            false,
        );
        let err = engine.migrate(plan).await.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn test_migration_plan_zero_validation_delay() {
        let mut plan = MigrationPlan::new(
            "fast-app".to_string(),
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            false,
        );
        plan.validation_delay = Duration::ZERO;
        assert_eq!(plan.validation_delay, Duration::ZERO);
    }

    #[test]
    fn test_migration_plan_large_validation_delay() {
        let mut plan = MigrationPlan::new(
            "slow-app".to_string(),
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Rolling,
            true,
        );
        plan.validation_delay = Duration::from_secs(3600);
        assert_eq!(plan.validation_delay, Duration::from_secs(3600));
    }

    #[test]
    fn test_migration_plan_rollback_disabled() {
        let plan = make_plan(
            "no-rollback",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            false,
        );
        assert!(!plan.rollback_on_failure);
    }

    #[test]
    fn test_migration_plan_clone() {
        let plan = make_plan(
            "clone-test",
            RuntimeKind::KubeVirt,
            RuntimeKind::Metal3,
            MigrationStrategy::Rolling,
            true,
        );
        let cloned = plan.clone();

        assert_eq!(plan.workload_name, cloned.workload_name);
        assert_eq!(plan.source_runtime, cloned.source_runtime);
        assert_eq!(plan.target_runtime, cloned.target_runtime);
        assert_eq!(plan.strategy, cloned.strategy);
        assert_eq!(plan.validation_delay, cloned.validation_delay);
        assert_eq!(plan.rollback_on_failure, cloned.rollback_on_failure);
    }

    #[test]
    fn test_migration_plan_debug_format() {
        let plan = make_plan(
            "debug-app",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::BlueGreen,
            true,
        );
        let debug_str = format!("{:?}", plan);
        assert!(debug_str.contains("debug-app"));
        assert!(debug_str.contains("Podman"));
        assert!(debug_str.contains("Kubernetes"));
        assert!(debug_str.contains("BlueGreen"));
    }

    #[test]
    fn test_migration_plan_empty_workload_name() {
        let plan = make_plan(
            "",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            false,
        );
        assert_eq!(plan.workload_name, "");
    }

    #[test]
    fn test_migration_plan_unicode_workload_name() {
        let plan = make_plan(
            "app-\u{1F680}-service",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            false,
        );
        assert!(plan.workload_name.contains('\u{1F680}'));
    }

    // ---------------------------------------------------------------
    // MigrationStrategy enum
    // ---------------------------------------------------------------

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

    #[test]
    fn test_strategy_all_variants_are_distinct() {
        let immediate = MigrationStrategy::Immediate;
        let blue_green = MigrationStrategy::BlueGreen;
        let rolling = MigrationStrategy::Rolling;

        assert_ne!(immediate, blue_green);
        assert_ne!(immediate, rolling);
        assert_ne!(blue_green, rolling);
    }

    #[test]
    fn test_strategy_self_equality() {
        assert_eq!(MigrationStrategy::Immediate, MigrationStrategy::Immediate);
        assert_eq!(MigrationStrategy::BlueGreen, MigrationStrategy::BlueGreen);
        assert_eq!(MigrationStrategy::Rolling, MigrationStrategy::Rolling);
    }

    #[test]
    fn test_strategy_clone_preserves_equality() {
        let strategies = [
            MigrationStrategy::Immediate,
            MigrationStrategy::BlueGreen,
            MigrationStrategy::Rolling,
            MigrationStrategy::Canary,
        ];
        for s in &strategies {
            assert_eq!(*s, s.clone());
        }
    }

    #[test]
    fn test_strategy_debug_output() {
        assert_eq!(format!("{:?}", MigrationStrategy::Immediate), "Immediate");
        assert_eq!(format!("{:?}", MigrationStrategy::BlueGreen), "BlueGreen");
        assert_eq!(format!("{:?}", MigrationStrategy::Rolling), "Rolling");
    }

    // ---------------------------------------------------------------
    // MigrationStrategy serde serialization / deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_strategy_serialize_immediate() {
        let json = serde_json::to_string(&MigrationStrategy::Immediate).unwrap();
        assert_eq!(json, "\"Immediate\"");
    }

    #[test]
    fn test_strategy_serialize_blue_green() {
        let json = serde_json::to_string(&MigrationStrategy::BlueGreen).unwrap();
        assert_eq!(json, "\"BlueGreen\"");
    }

    #[test]
    fn test_strategy_serialize_rolling() {
        let json = serde_json::to_string(&MigrationStrategy::Rolling).unwrap();
        assert_eq!(json, "\"Rolling\"");
    }

    #[test]
    fn test_strategy_deserialize_immediate() {
        let s: MigrationStrategy = serde_json::from_str("\"Immediate\"").unwrap();
        assert_eq!(s, MigrationStrategy::Immediate);
    }

    #[test]
    fn test_strategy_deserialize_blue_green() {
        let s: MigrationStrategy = serde_json::from_str("\"BlueGreen\"").unwrap();
        assert_eq!(s, MigrationStrategy::BlueGreen);
    }

    #[test]
    fn test_strategy_deserialize_rolling() {
        let s: MigrationStrategy = serde_json::from_str("\"Rolling\"").unwrap();
        assert_eq!(s, MigrationStrategy::Rolling);
    }

    #[test]
    fn test_strategy_roundtrip_serde() {
        let strategies = [
            MigrationStrategy::Immediate,
            MigrationStrategy::BlueGreen,
            MigrationStrategy::Rolling,
            MigrationStrategy::Canary,
        ];
        for original in &strategies {
            let json = serde_json::to_string(original).unwrap();
            let deserialized: MigrationStrategy = serde_json::from_str(&json).unwrap();
            assert_eq!(*original, deserialized);
        }
    }

    #[test]
    fn test_strategy_deserialize_invalid_value() {
        let result = serde_json::from_str::<MigrationStrategy>("\"Gradual\"");
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_deserialize_wrong_type() {
        let result = serde_json::from_str::<MigrationStrategy>("42");
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_deserialize_null() {
        let result = serde_json::from_str::<MigrationStrategy>("null");
        assert!(result.is_err());
    }

    #[test]
    fn test_strategy_yaml_roundtrip() {
        let strategies = [
            MigrationStrategy::Immediate,
            MigrationStrategy::BlueGreen,
            MigrationStrategy::Rolling,
            MigrationStrategy::Canary,
        ];
        for original in &strategies {
            let yaml = serde_yaml::to_string(original).unwrap();
            let deserialized: MigrationStrategy = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(*original, deserialized);
        }
    }

    // ---------------------------------------------------------------
    // MigrationResult construction and state tracking
    // ---------------------------------------------------------------

    #[test]
    fn test_migration_result_success() {
        let target = make_instance("t-1", "app", RuntimeKind::Kubernetes);
        let result = MigrationResult {
            success: true,
            source_instance: None,
            target_instance: Some(target.clone()),
            error: None,
            rollback_performed: false,
        };

        assert!(result.success);
        assert!(result.source_instance.is_none());
        assert!(result.target_instance.is_some());
        assert_eq!(result.target_instance.unwrap().id, "t-1");
        assert!(result.error.is_none());
        assert!(!result.rollback_performed);
    }

    #[test]
    fn test_migration_result_failure_no_rollback() {
        let source = make_instance("s-1", "app", RuntimeKind::Podman);
        let result = MigrationResult {
            success: false,
            source_instance: Some(source.clone()),
            target_instance: None,
            error: Some("Deployment failed: timeout".to_string()),
            rollback_performed: false,
        };

        assert!(!result.success);
        assert!(result.source_instance.is_some());
        assert_eq!(result.source_instance.unwrap().id, "s-1");
        assert!(result.target_instance.is_none());
        assert_eq!(result.error.as_deref(), Some("Deployment failed: timeout"));
        assert!(!result.rollback_performed);
    }

    #[test]
    fn test_migration_result_failure_with_rollback() {
        let source = make_instance("s-2", "app", RuntimeKind::Podman);
        let result = MigrationResult {
            success: false,
            source_instance: Some(source),
            target_instance: None,
            error: Some("Target instance failed health check".to_string()),
            rollback_performed: true,
        };

        assert!(!result.success);
        assert!(result.rollback_performed);
        assert!(result.error.is_some());
        assert!(
            result
                .error
                .as_ref()
                .unwrap()
                .contains("health check")
        );
    }

    #[test]
    fn test_migration_result_both_instances_present() {
        // Edge case: both source and target may be present during transition
        let source = make_instance("s-3", "app-source", RuntimeKind::Podman);
        let target = make_instance("t-3", "app-target", RuntimeKind::Kubernetes);
        let result = MigrationResult {
            success: false,
            source_instance: Some(source),
            target_instance: Some(target),
            error: Some("Partial migration".to_string()),
            rollback_performed: false,
        };

        assert!(result.source_instance.is_some());
        assert!(result.target_instance.is_some());
        assert_eq!(result.source_instance.unwrap().runtime, RuntimeKind::Podman);
        assert_eq!(
            result.target_instance.unwrap().runtime,
            RuntimeKind::Kubernetes
        );
    }

    #[test]
    fn test_migration_result_no_instances() {
        let result = MigrationResult {
            success: false,
            source_instance: None,
            target_instance: None,
            error: Some("Complete failure".to_string()),
            rollback_performed: false,
        };

        assert!(result.source_instance.is_none());
        assert!(result.target_instance.is_none());
    }

    #[test]
    fn test_migration_result_debug_format() {
        let result = MigrationResult {
            success: true,
            source_instance: None,
            target_instance: None,
            error: None,
            rollback_performed: false,
        };
        let debug_str = format!("{:?}", result);
        assert!(debug_str.contains("success: true"));
        assert!(debug_str.contains("rollback_performed: false"));
    }

    #[test]
    fn test_migration_result_error_message_preservation() {
        let long_error = "x".repeat(1000);
        let result = MigrationResult {
            success: false,
            source_instance: None,
            target_instance: None,
            error: Some(long_error.clone()),
            rollback_performed: false,
        };
        assert_eq!(result.error.unwrap().len(), 1000);
    }

    // ---------------------------------------------------------------
    // MigrationEngine creation and configuration
    // ---------------------------------------------------------------

    #[test]
    fn test_engine_creation_with_path() {
        let path = PathBuf::from("/tmp/test-state.json");
        let engine = MigrationEngine::new(path.clone());
        assert_eq!(engine.state_path, path);
    }

    #[test]
    fn test_engine_creation_with_relative_path() {
        let path = PathBuf::from("state.json");
        let engine = MigrationEngine::new(path.clone());
        assert_eq!(engine.state_path, path);
    }

    #[test]
    fn test_engine_creation_with_nested_path() {
        let path = PathBuf::from("/var/lib/aether/data/state.json");
        let engine = MigrationEngine::new(path.clone());
        assert_eq!(engine.state_path, path);
    }

    #[test]
    fn test_engine_creation_with_empty_path() {
        let path = PathBuf::from("");
        let engine = MigrationEngine::new(path.clone());
        assert_eq!(engine.state_path, path);
    }

    #[test]
    fn test_engine_different_paths_are_independent() {
        let engine1 = MigrationEngine::new(PathBuf::from("/tmp/state1.json"));
        let engine2 = MigrationEngine::new(PathBuf::from("/tmp/state2.json"));
        assert_ne!(engine1.state_path, engine2.state_path);
    }

    // ---------------------------------------------------------------
    // Migration step / scenario tracking
    // ---------------------------------------------------------------

    #[test]
    fn test_plan_with_each_strategy() {
        let strategies = [
            MigrationStrategy::Immediate,
            MigrationStrategy::BlueGreen,
            MigrationStrategy::Rolling,
            MigrationStrategy::Canary,
        ];

        for strategy in &strategies {
            let plan = make_plan(
                "step-app",
                RuntimeKind::Podman,
                RuntimeKind::Kubernetes,
                strategy.clone(),
                true,
            );
            assert_eq!(&plan.strategy, strategy);
        }
    }

    #[test]
    fn test_plan_podman_to_kubernetes() {
        let plan = make_plan(
            "migrate-k8s",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::BlueGreen,
            true,
        );
        assert_eq!(plan.source_runtime, RuntimeKind::Podman);
        assert_eq!(plan.target_runtime, RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_plan_kubernetes_to_kubevirt() {
        let plan = make_plan(
            "migrate-kv",
            RuntimeKind::Kubernetes,
            RuntimeKind::KubeVirt,
            MigrationStrategy::Rolling,
            false,
        );
        assert_eq!(plan.source_runtime, RuntimeKind::Kubernetes);
        assert_eq!(plan.target_runtime, RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_plan_kubevirt_to_metal3() {
        let plan = make_plan(
            "migrate-metal",
            RuntimeKind::KubeVirt,
            RuntimeKind::Metal3,
            MigrationStrategy::Immediate,
            true,
        );
        assert_eq!(plan.source_runtime, RuntimeKind::KubeVirt);
        assert_eq!(plan.target_runtime, RuntimeKind::Metal3);
    }

    #[test]
    fn test_plan_metal3_to_podman() {
        let plan = make_plan(
            "migrate-back",
            RuntimeKind::Metal3,
            RuntimeKind::Podman,
            MigrationStrategy::Immediate,
            false,
        );
        assert_eq!(plan.source_runtime, RuntimeKind::Metal3);
        assert_eq!(plan.target_runtime, RuntimeKind::Podman);
    }

    // ---------------------------------------------------------------
    // Rollback plan generation (simulated via MigrationResult)
    // ---------------------------------------------------------------

    #[test]
    fn test_rollback_plan_result_matches_source() {
        // When a rollback occurs, the source_instance should be repopulated
        let source = make_instance("rollback-src", "web-app", RuntimeKind::Podman);
        let result = MigrationResult {
            success: false,
            source_instance: Some(source.clone()),
            target_instance: None,
            error: Some("Deployment failed: image pull error".to_string()),
            rollback_performed: true,
        };

        assert!(result.rollback_performed);
        assert!(!result.success);
        let src = result.source_instance.unwrap();
        assert_eq!(src.id, "rollback-src");
        assert_eq!(src.runtime, RuntimeKind::Podman);
    }

    #[test]
    fn test_rollback_with_health_check_failure() {
        let source = make_instance("hc-src", "api-svc", RuntimeKind::Kubernetes);
        let result = MigrationResult {
            success: false,
            source_instance: Some(source),
            target_instance: None,
            error: Some("Target instance failed health check".to_string()),
            rollback_performed: true,
        };

        assert!(result.rollback_performed);
        assert!(result.error.unwrap().contains("health check"));
    }

    #[test]
    fn test_no_rollback_when_disabled() {
        let result = MigrationResult {
            success: false,
            source_instance: None,
            target_instance: None,
            error: Some("Target instance not ready".to_string()),
            rollback_performed: false,
        };

        assert!(!result.rollback_performed);
        assert!(!result.success);
    }

    #[test]
    fn test_rollback_flag_independent_of_success() {
        // Theoretically, rollback_performed and success are independent booleans
        let result = MigrationResult {
            success: true,
            source_instance: None,
            target_instance: None,
            error: None,
            rollback_performed: true,
        };
        // This is a strange but valid state
        assert!(result.success);
        assert!(result.rollback_performed);
    }

    // ---------------------------------------------------------------
    // Health check validation logic scenarios
    // ---------------------------------------------------------------

    #[test]
    fn test_health_check_pass_scenario() {
        // Simulates what happens when health check passes:
        // success = true, no error, no rollback
        let target = make_instance("healthy-1", "api", RuntimeKind::Kubernetes);
        let result = MigrationResult {
            success: true,
            source_instance: None,
            target_instance: Some(target),
            error: None,
            rollback_performed: false,
        };

        assert!(result.success);
        assert!(result.target_instance.is_some());
        assert!(result.error.is_none());
        assert!(!result.rollback_performed);
    }

    #[test]
    fn test_health_check_fail_with_rollback_scenario() {
        // Simulates immediate strategy: health check fails, rollback enabled
        let plan = make_plan(
            "hc-app",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            true,
        );
        assert!(plan.rollback_on_failure);

        let source = make_instance("hc-src", "hc-app", RuntimeKind::Podman);
        let result = MigrationResult {
            success: false,
            source_instance: Some(source),
            target_instance: None,
            error: Some("Target instance failed health check".to_string()),
            rollback_performed: true,
        };

        assert!(!result.success);
        assert!(result.rollback_performed);
        assert!(result.source_instance.is_some());
    }

    #[test]
    fn test_health_check_fail_no_rollback_scenario() {
        let plan = make_plan(
            "hc-app2",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            false,
        );
        assert!(!plan.rollback_on_failure);

        // When rollback_on_failure is false the engine would return an error,
        // not a MigrationResult. But we can model the result for failed health:
        let result = MigrationResult {
            success: false,
            source_instance: None,
            target_instance: None,
            error: Some("Target instance not ready".to_string()),
            rollback_performed: false,
        };
        assert!(!result.rollback_performed);
    }

    #[test]
    fn test_blue_green_health_check_fail_scenario() {
        // Blue-green: target (green) fails health, source (blue) remains
        let source = make_instance("bg-blue", "bg-app", RuntimeKind::Podman);
        let result = MigrationResult {
            success: false,
            source_instance: Some(source),
            target_instance: None,
            error: Some("Green deployment failed health check".to_string()),
            rollback_performed: false,
        };

        assert!(!result.success);
        assert!(result.source_instance.is_some());
        assert!(result.error.unwrap().contains("Green deployment"));
    }

    #[test]
    fn test_rolling_validation_failure_scenario() {
        // Rolling strategy: validation retries exhausted
        let source = make_instance("roll-src", "roll-app", RuntimeKind::Podman);
        let result = MigrationResult {
            success: false,
            source_instance: Some(source),
            target_instance: None,
            error: Some("Target failed validation after retries".to_string()),
            rollback_performed: false,
        };

        assert!(!result.success);
        assert!(result.error.unwrap().contains("after retries"));
    }

    #[test]
    fn test_rolling_unhealthy_during_traffic_shift_scenario() {
        let source = make_instance("roll-ts", "roll-app", RuntimeKind::Podman);
        let result = MigrationResult {
            success: false,
            source_instance: Some(source),
            target_instance: None,
            error: Some("Target became unhealthy during migration".to_string()),
            rollback_performed: false,
        };

        assert!(!result.success);
        assert!(result.error.unwrap().contains("unhealthy during migration"));
    }

    // ---------------------------------------------------------------
    // Validation delay configuration
    // ---------------------------------------------------------------

    #[test]
    fn test_validation_delay_millis() {
        let mut plan = MigrationPlan::new(
            "millis-app".to_string(),
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            false,
        );
        plan.validation_delay = Duration::from_millis(500);
        assert_eq!(plan.validation_delay.as_millis(), 500);
    }

    #[test]
    fn test_validation_delay_seconds() {
        let mut plan = MigrationPlan::new(
            "secs-app".to_string(),
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::BlueGreen,
            true,
        );
        plan.validation_delay = Duration::from_secs(60);
        assert_eq!(plan.validation_delay.as_secs(), 60);
    }

    // ---------------------------------------------------------------
    // Instance fields inside MigrationResult
    // ---------------------------------------------------------------

    #[test]
    fn test_result_target_instance_fields() {
        let target = Instance {
            id: "abc-123-def".to_string(),
            name: "my-service".to_string(),
            runtime: RuntimeKind::Kubernetes,
            image: "ghcr.io/org/my-service:v2".to_string(),
            created_at: "2024-12-01T12:00:00Z".to_string(),
        };

        let result = MigrationResult {
            success: true,
            source_instance: None,
            target_instance: Some(target),
            error: None,
            rollback_performed: false,
        };

        let inst = result.target_instance.unwrap();
        assert_eq!(inst.id, "abc-123-def");
        assert_eq!(inst.name, "my-service");
        assert_eq!(inst.runtime, RuntimeKind::Kubernetes);
        assert_eq!(inst.image, "ghcr.io/org/my-service:v2");
        assert_eq!(inst.created_at, "2024-12-01T12:00:00Z");
    }

    #[test]
    fn test_result_source_instance_fields() {
        let source = Instance {
            id: "pod-xyz".to_string(),
            name: "legacy-svc".to_string(),
            runtime: RuntimeKind::Podman,
            image: "localhost/legacy-svc:latest".to_string(),
            created_at: "2023-01-15T08:30:00Z".to_string(),
        };

        let result = MigrationResult {
            success: false,
            source_instance: Some(source),
            target_instance: None,
            error: Some("failed".to_string()),
            rollback_performed: true,
        };

        let inst = result.source_instance.unwrap();
        assert_eq!(inst.id, "pod-xyz");
        assert_eq!(inst.runtime, RuntimeKind::Podman);
    }

    // ---------------------------------------------------------------
    // Multiple migration plans in sequence (state tracking)
    // ---------------------------------------------------------------

    #[test]
    fn test_sequential_migration_plans() {
        // Simulate planning a chain: Podman -> K8s -> KubeVirt
        let plan1 = make_plan(
            "chain-app",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::BlueGreen,
            true,
        );
        let plan2 = make_plan(
            "chain-app",
            RuntimeKind::Kubernetes,
            RuntimeKind::KubeVirt,
            MigrationStrategy::Rolling,
            true,
        );

        assert_eq!(plan1.target_runtime, plan2.source_runtime);
        assert_eq!(plan1.workload_name, plan2.workload_name);
    }

    // ---------------------------------------------------------------
    // Edge cases
    // ---------------------------------------------------------------

    #[test]
    fn test_plan_with_special_chars_in_name() {
        let plan = make_plan(
            "app-with-dashes_and_underscores.v2",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            false,
        );
        assert_eq!(plan.workload_name, "app-with-dashes_and_underscores.v2");
    }

    #[test]
    fn test_plan_with_very_long_name() {
        let long_name = "a".repeat(256);
        let plan = make_plan(
            &long_name,
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            false,
        );
        assert_eq!(plan.workload_name.len(), 256);
    }

    #[test]
    fn test_migration_result_empty_error_string() {
        let result = MigrationResult {
            success: false,
            source_instance: None,
            target_instance: None,
            error: Some(String::new()),
            rollback_performed: false,
        };
        assert_eq!(result.error, Some(String::new()));
    }

    #[test]
    fn test_engine_state_path_preserved() {
        let paths = [
            PathBuf::from("/home/user/.aether/state.json"),
            PathBuf::from("/etc/aether/state.json"),
            PathBuf::from("./local-state.json"),
        ];
        for path in &paths {
            let engine = MigrationEngine::new(path.clone());
            assert_eq!(&engine.state_path, path);
        }
    }

    // ---------------------------------------------------------------
    // Strategy used in plan determines migration path
    // ---------------------------------------------------------------

    #[test]
    fn test_immediate_strategy_plan() {
        let plan = make_plan(
            "imm-app",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Immediate,
            true,
        );
        assert_eq!(plan.strategy, MigrationStrategy::Immediate);
        // Immediate = stop source first, then start target
    }

    #[test]
    fn test_blue_green_strategy_plan() {
        let plan = make_plan(
            "bg-app",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::BlueGreen,
            true,
        );
        assert_eq!(plan.strategy, MigrationStrategy::BlueGreen);
        // BlueGreen = start target (green), validate, then stop source (blue)
    }

    #[test]
    fn test_rolling_strategy_plan() {
        let plan = make_plan(
            "roll-app",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Rolling,
            false,
        );
        assert_eq!(plan.strategy, MigrationStrategy::Rolling);
        // Rolling = deploy target, validate with retries, gradual traffic shift
    }

    // ---------------------------------------------------------------
    // Combinations: strategy + rollback flag
    // ---------------------------------------------------------------

    #[test]
    fn test_immediate_with_rollback() {
        let plan = make_plan(
            "ir-app",
            RuntimeKind::Podman,
            RuntimeKind::Metal3,
            MigrationStrategy::Immediate,
            true,
        );
        assert_eq!(plan.strategy, MigrationStrategy::Immediate);
        assert!(plan.rollback_on_failure);
    }

    #[test]
    fn test_immediate_without_rollback() {
        let plan = make_plan(
            "inr-app",
            RuntimeKind::Podman,
            RuntimeKind::Metal3,
            MigrationStrategy::Immediate,
            false,
        );
        assert_eq!(plan.strategy, MigrationStrategy::Immediate);
        assert!(!plan.rollback_on_failure);
    }

    #[test]
    fn test_blue_green_with_rollback() {
        let plan = make_plan(
            "bgr-app",
            RuntimeKind::KubeVirt,
            RuntimeKind::Kubernetes,
            MigrationStrategy::BlueGreen,
            true,
        );
        assert_eq!(plan.strategy, MigrationStrategy::BlueGreen);
        assert!(plan.rollback_on_failure);
    }

    #[test]
    fn test_rolling_with_rollback() {
        let plan = make_plan(
            "rr-app",
            RuntimeKind::Metal3,
            RuntimeKind::Podman,
            MigrationStrategy::Rolling,
            true,
        );
        assert_eq!(plan.strategy, MigrationStrategy::Rolling);
        assert!(plan.rollback_on_failure);
    }

    #[test]
    fn test_migration_strategy_from_str() {
        assert_eq!("immediate".parse::<MigrationStrategy>().unwrap(), MigrationStrategy::Immediate);
        assert_eq!("blue-green".parse::<MigrationStrategy>().unwrap(), MigrationStrategy::BlueGreen);
        assert_eq!("bluegreen".parse::<MigrationStrategy>().unwrap(), MigrationStrategy::BlueGreen);
        assert_eq!("rolling".parse::<MigrationStrategy>().unwrap(), MigrationStrategy::Rolling);
        assert_eq!("canary".parse::<MigrationStrategy>().unwrap(), MigrationStrategy::Canary);
        assert_eq!("IMMEDIATE".parse::<MigrationStrategy>().unwrap(), MigrationStrategy::Immediate);
        assert!("unknown".parse::<MigrationStrategy>().is_err());
    }

    #[test]
    fn test_migration_strategy_display() {
        assert_eq!(MigrationStrategy::Immediate.to_string(), "immediate");
        assert_eq!(MigrationStrategy::BlueGreen.to_string(), "blue-green");
        assert_eq!(MigrationStrategy::Rolling.to_string(), "rolling");
        assert_eq!(MigrationStrategy::Canary.to_string(), "canary");
    }

    // ---------------------------------------------------------------
    // Canary migration
    // ---------------------------------------------------------------

    #[test]
    fn test_canary_config_defaults() {
        let config = CanaryConfig {
            steps: vec![10, 25, 50, 75, 100],
            step_interval_secs: 10,
            error_threshold: 0.1,
            health_checks_per_step: 2,
        };
        assert_eq!(config.steps.len(), 5);
        assert_eq!(config.steps[0], 10);
        assert_eq!(config.steps[4], 100);
        assert!(config.error_threshold > 0.0 && config.error_threshold < 1.0);
    }

    #[test]
    fn test_canary_plan_with_config() {
        let mut plan = make_plan(
            "canary-app",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Canary,
            true,
        );
        plan.canary_config = Some(CanaryConfig {
            steps: vec![5, 20, 50, 100],
            step_interval_secs: 30,
            error_threshold: 0.05,
            health_checks_per_step: 3,
        });
        assert_eq!(plan.strategy, MigrationStrategy::Canary);
        assert!(plan.canary_config.is_some());
        assert_eq!(plan.canary_config.as_ref().unwrap().steps.len(), 4);
    }

    #[tokio::test]
    async fn test_migrate_canary_rejects_same_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let engine = MigrationEngine::new(state_path);
        let plan = make_plan(
            "canary-same-rt",
            RuntimeKind::Kubernetes,
            RuntimeKind::Kubernetes,
            MigrationStrategy::Canary,
            true,
        );
        let err = engine.migrate(plan).await.unwrap_err();
        assert!(err.to_string().contains("same"));
    }

    #[test]
    fn test_canary_config_serde_roundtrip() {
        let config = CanaryConfig {
            steps: vec![10, 50, 100],
            step_interval_secs: 15,
            error_threshold: 0.2,
            health_checks_per_step: 5,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: CanaryConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.steps, vec![10, 50, 100]);
        assert_eq!(parsed.step_interval_secs, 15);
    }

    #[test]
    fn test_blue_green_plan_defaults() {
        let plan = make_plan(
            "bg-test",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::BlueGreen,
            true,
        );
        // validation_delay is 10s by default
        assert_eq!(plan.validation_delay, Duration::from_secs(10));
        // cleanup_delay is 2s
        assert_eq!(plan.cleanup_delay, Duration::from_secs(2));
    }

    #[test]
    fn test_drain_delay_capped_at_30s() {
        let mut plan = make_plan(
            "drain-test",
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            MigrationStrategy::BlueGreen,
            true,
        );
        plan.validation_delay = Duration::from_secs(120);
        let drain_delay = plan.validation_delay.min(Duration::from_secs(30));
        assert_eq!(drain_delay, Duration::from_secs(30));
    }
}
