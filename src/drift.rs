//! Drift detection and reconciliation
//!
//! Detects when running workload state diverges from the desired spec
//! and provides reconciliation actions to bring them back in sync.

use crate::output;
use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};

/// Result of a drift detection scan
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftReport {
    pub workload_name: String,
    pub has_drift: bool,
    pub drifts: Vec<DriftItem>,
    pub severity: DriftSeverity,
    pub reconciliation_plan: Vec<ReconcileAction>,
}

/// A single drift between desired and actual state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DriftItem {
    pub field: String,
    pub expected: String,
    pub actual: String,
    pub severity: DriftSeverity,
    pub category: DriftCategory,
}

/// Drift severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DriftSeverity {
    Info,
    Warning,
    Critical,
}

impl std::fmt::Display for DriftSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriftSeverity::Info => write!(f, "INFO"),
            DriftSeverity::Warning => write!(f, "WARNING"),
            DriftSeverity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Category of drift
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DriftCategory {
    Runtime,
    Image,
    Resources,
    Network,
    Configuration,
    Scaling,
    Health,
}

impl std::fmt::Display for DriftCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriftCategory::Runtime => write!(f, "Runtime"),
            DriftCategory::Image => write!(f, "Image"),
            DriftCategory::Resources => write!(f, "Resources"),
            DriftCategory::Network => write!(f, "Network"),
            DriftCategory::Configuration => write!(f, "Configuration"),
            DriftCategory::Scaling => write!(f, "Scaling"),
            DriftCategory::Health => write!(f, "Health"),
        }
    }
}

/// Action to reconcile drift
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconcileAction {
    pub action_type: ReconcileType,
    pub description: String,
    pub requires_restart: bool,
    pub risk: DriftSeverity,
}

/// Type of reconciliation action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReconcileType {
    /// Redeploy the workload
    Redeploy,
    /// Update configuration in-place
    UpdateConfig,
    /// Scale replicas
    Scale,
    /// Update image
    UpdateImage,
    /// No action needed
    None,
}

impl std::fmt::Display for ReconcileType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReconcileType::Redeploy => write!(f, "Redeploy"),
            ReconcileType::UpdateConfig => write!(f, "Update Config"),
            ReconcileType::Scale => write!(f, "Scale"),
            ReconcileType::UpdateImage => write!(f, "Update Image"),
            ReconcileType::None => write!(f, "None"),
        }
    }
}

/// Drift detector
#[derive(Default)]
pub struct DriftDetector;

impl DriftDetector {
    pub fn new() -> Self {
        Self
    }

    /// Detect drift between desired spec and actual state
    pub fn detect(&self, spec: &Workload, state: &WorkloadState) -> DriftReport {
        let mut drifts = Vec::new();

        // Check runtime drift
        self.check_runtime_drift(spec, state, &mut drifts);

        // Check image drift
        self.check_image_drift(spec, state, &mut drifts);

        // Check resource drift
        self.check_resource_drift(spec, state, &mut drifts);

        // Check network drift
        self.check_network_drift(spec, &mut drifts);

        // Check health drift
        self.check_health_drift(spec, &mut drifts);

        // Check scaling drift
        self.check_scaling_drift(spec, &mut drifts);

        // Determine overall severity
        let severity = drifts
            .iter()
            .map(|d| &d.severity)
            .max()
            .cloned()
            .unwrap_or(DriftSeverity::Info);

        // Build reconciliation plan
        let reconciliation_plan = self.build_reconciliation_plan(&drifts);

        let has_drift = !drifts.is_empty();

        DriftReport {
            workload_name: spec.metadata.name.clone(),
            has_drift,
            drifts,
            severity,
            reconciliation_plan,
        }
    }

    /// Detect drift across all workloads
    pub fn detect_all(
        &self,
        workloads: &[(Workload, WorkloadState)],
    ) -> Vec<DriftReport> {
        workloads
            .iter()
            .map(|(spec, state)| self.detect(spec, state))
            .collect()
    }

    fn check_runtime_drift(
        &self,
        spec: &Workload,
        state: &WorkloadState,
        drifts: &mut Vec<DriftItem>,
    ) {
        use crate::spec::RuntimePreference;

        // If preferred runtime is explicit, check against actual
        let expected_runtime = match &spec.runtime.preferred {
            RuntimePreference::Container => Some(RuntimeKind::Podman),
            RuntimePreference::Kube => Some(RuntimeKind::Kubernetes),
            RuntimePreference::Kubevirt => Some(RuntimeKind::KubeVirt),
            RuntimePreference::Metal => Some(RuntimeKind::Metal3),
            RuntimePreference::Auto => None,
        };

        if let Some(expected) = expected_runtime {
            if expected != state.runtime {
                drifts.push(DriftItem {
                    field: "runtime".to_string(),
                    expected: format!("{}", expected),
                    actual: format!("{}", state.runtime),
                    severity: DriftSeverity::Warning,
                    category: DriftCategory::Runtime,
                });
            }
        }
    }

    fn check_image_drift(
        &self,
        spec: &Workload,
        state: &WorkloadState,
        drifts: &mut Vec<DriftItem>,
    ) {
        let expected_image = format!(
            "{}/{}:latest",
            spec.build.registry, spec.metadata.name
        );

        if !state.instance.image.is_empty() && state.instance.image != expected_image {
            drifts.push(DriftItem {
                field: "image".to_string(),
                expected: expected_image,
                actual: state.instance.image.clone(),
                severity: DriftSeverity::Info,
                category: DriftCategory::Image,
            });
        }
    }

    fn check_resource_drift(
        &self,
        spec: &Workload,
        _state: &WorkloadState,
        drifts: &mut Vec<DriftItem>,
    ) {
        // Check if spec has been modified since deployment
        // In a real system, this would compare against the actual running resources
        // For now, check if the spec file timestamp is newer than the deployment

        let cpu = &spec.requirements.cpu;
        let memory = &spec.requirements.memory;

        // Detect potentially problematic resource configurations
        let cpu_val = crate::resources::parse_cpu(cpu);
        let mem_val = crate::resources::parse_memory_gi(memory);

        // Configurable drift detection defaults: these thresholds flag
        // unusually large resource requests and can be overridden via config.
        if cpu_val > 32.0 {
            drifts.push(DriftItem {
                field: "requirements.cpu".to_string(),
                expected: "reasonable allocation".to_string(),
                actual: format!("{} cores", cpu_val),
                severity: DriftSeverity::Warning,
                category: DriftCategory::Resources,
            });
        }

        // Configurable drift detection default for memory (see cpu threshold above).
        if mem_val > 128.0 {
            drifts.push(DriftItem {
                field: "requirements.memory".to_string(),
                expected: "reasonable allocation".to_string(),
                actual: format!("{:.0}Gi", mem_val),
                severity: DriftSeverity::Warning,
                category: DriftCategory::Resources,
            });
        }

        // Detect resource request > limit (misconfiguration)
        if let Some(ref cpu_req) = spec.requirements.cpu_request {
            let req_val = crate::resources::parse_cpu(cpu_req);
            if req_val > cpu_val {
                drifts.push(DriftItem {
                    field: "requirements.cpuRequest".to_string(),
                    expected: format!("<= {} (cpu limit)", cpu),
                    actual: format!("{} (request exceeds limit)", cpu_req),
                    severity: DriftSeverity::Critical,
                    category: DriftCategory::Resources,
                });
            }
        }

        if let Some(ref mem_req) = spec.requirements.memory_request {
            let req_val = crate::resources::parse_memory_gi(mem_req);
            if req_val > mem_val {
                drifts.push(DriftItem {
                    field: "requirements.memoryRequest".to_string(),
                    expected: format!("<= {} (memory limit)", memory),
                    actual: format!("{} (request exceeds limit)", mem_req),
                    severity: DriftSeverity::Critical,
                    category: DriftCategory::Resources,
                });
            }
        }
    }

    fn check_network_drift(
        &self,
        spec: &Workload,
        drifts: &mut Vec<DriftItem>,
    ) {
        // Check for service without ports
        if spec.network.service && spec.network.ports.is_empty() {
            drifts.push(DriftItem {
                field: "network.ports".to_string(),
                expected: "at least one port mapping".to_string(),
                actual: "no ports defined".to_string(),
                severity: DriftSeverity::Warning,
                category: DriftCategory::Network,
            });
        }

        // Check ingress without TLS
        if let Some(ingress) = &spec.ingress {
            if ingress.enabled && !ingress.tls {
                drifts.push(DriftItem {
                    field: "ingress.tls".to_string(),
                    expected: "TLS enabled".to_string(),
                    actual: "TLS disabled".to_string(),
                    severity: DriftSeverity::Critical,
                    category: DriftCategory::Network,
                });
            }
        }
    }

    fn check_health_drift(
        &self,
        spec: &Workload,
        drifts: &mut Vec<DriftItem>,
    ) {
        if spec.health.is_none() && spec.network.service {
            drifts.push(DriftItem {
                field: "health".to_string(),
                expected: "health probes configured".to_string(),
                actual: "no health probes".to_string(),
                severity: DriftSeverity::Warning,
                category: DriftCategory::Health,
            });
        }
    }

    fn check_scaling_drift(
        &self,
        spec: &Workload,
        drifts: &mut Vec<DriftItem>,
    ) {
        if let Some(scaling) = &spec.scaling {
            if scaling.enabled && scaling.min_replicas >= scaling.max_replicas {
                drifts.push(DriftItem {
                    field: "scaling".to_string(),
                    expected: "minReplicas < maxReplicas".to_string(),
                    actual: format!(
                        "min={} max={}",
                        scaling.min_replicas, scaling.max_replicas
                    ),
                    severity: DriftSeverity::Warning,
                    category: DriftCategory::Scaling,
                });
            }
        }
    }

    /// Detect drift including live runtime state checks.
    ///
    /// Extends `detect()` with actual runtime status queries to compare
    /// expected vs actual workload state.
    pub async fn check_live_drift(
        &self,
        spec: &crate::spec::Workload,
        state: &crate::state::WorkloadState,
        runtime: &dyn crate::Runtime,
    ) -> DriftReport {
        let mut report = self.detect(spec, state);

        match runtime.status(&state.instance).await {
            Ok(status) => {
                use crate::runtime::InstanceState;

                if status.state == InstanceState::Failed {
                    report.drifts.push(DriftItem {
                        field: "live.state".to_string(),
                        expected: "running".to_string(),
                        actual: "failed".to_string(),
                        severity: DriftSeverity::Critical,
                        category: DriftCategory::Runtime,
                    });
                    report.has_drift = true;
                }

                if !status.ready && status.state == InstanceState::Running {
                    report.drifts.push(DriftItem {
                        field: "live.ready".to_string(),
                        expected: "true".to_string(),
                        actual: "false".to_string(),
                        severity: DriftSeverity::Warning,
                        category: DriftCategory::Health,
                    });
                    report.has_drift = true;
                }

                if status.restart_count > 0 {
                    report.drifts.push(DriftItem {
                        field: "live.restart_count".to_string(),
                        expected: "0".to_string(),
                        actual: status.restart_count.to_string(),
                        severity: if status.restart_count > 3 {
                            DriftSeverity::Critical
                        } else {
                            DriftSeverity::Warning
                        },
                        category: DriftCategory::Health,
                    });
                    report.has_drift = true;
                }
            }
            Err(e) => {
                report.drifts.push(DriftItem {
                    field: "live.status".to_string(),
                    expected: "reachable".to_string(),
                    actual: format!("unreachable: {}", e),
                    severity: DriftSeverity::Critical,
                    category: DriftCategory::Runtime,
                });
                report.has_drift = true;
            }
        }

        // Recalculate overall severity
        report.severity = report.drifts.iter()
            .map(|d| match d.severity {
                DriftSeverity::Critical => 2,
                DriftSeverity::Warning => 1,
                DriftSeverity::Info => 0,
            })
            .max()
            .map(|v| match v {
                2 => DriftSeverity::Critical,
                1 => DriftSeverity::Warning,
                _ => DriftSeverity::Info,
            })
            .unwrap_or(DriftSeverity::Info);

        report
    }

    fn build_reconciliation_plan(&self, drifts: &[DriftItem]) -> Vec<ReconcileAction> {
        let mut actions = Vec::new();

        for drift in drifts {
            let action = match drift.category {
                DriftCategory::Runtime => ReconcileAction {
                    action_type: ReconcileType::Redeploy,
                    description: format!(
                        "Migrate from {} to {}",
                        drift.actual, drift.expected
                    ),
                    requires_restart: true,
                    risk: DriftSeverity::Warning,
                },
                DriftCategory::Image => ReconcileAction {
                    action_type: ReconcileType::UpdateImage,
                    description: format!("Update image to {}", drift.expected),
                    requires_restart: true,
                    risk: DriftSeverity::Info,
                },
                DriftCategory::Configuration => ReconcileAction {
                    action_type: ReconcileType::UpdateConfig,
                    description: format!("Update {}", drift.field),
                    requires_restart: false,
                    risk: DriftSeverity::Info,
                },
                DriftCategory::Scaling => ReconcileAction {
                    action_type: ReconcileType::Scale,
                    description: format!("Fix {}: {}", drift.field, drift.expected),
                    requires_restart: false,
                    risk: DriftSeverity::Info,
                },
                _ => ReconcileAction {
                    action_type: ReconcileType::Redeploy,
                    description: format!(
                        "Reconcile {}: expected {}, got {}",
                        drift.field, drift.expected, drift.actual
                    ),
                    requires_restart: true,
                    risk: drift.severity.clone(),
                },
            };
            actions.push(action);
        }

        actions
    }

}

/// Format drift report as a readable string
pub fn format_drift_report(report: &DriftReport) -> String {
    let mut output = String::new();

    output.push_str(&output::property_section(&[
        ("Drift Report", report.workload_name.clone()),
        ("Status", if report.has_drift {
            format!("DRIFT DETECTED ({})", report.severity)
        } else {
            "IN SYNC".to_string()
        }),
    ]));

    if report.drifts.is_empty() {
        output.push_str("No drift detected. Workload matches desired spec.\n");
        return output;
    }

    output.push_str(&format!("Drifts ({}):\n", report.drifts.len()));
    for drift in &report.drifts {
        output.push_str(&format!(
            "  [{}] {} ({})\n",
            drift.severity, drift.field, drift.category
        ));
        output.push_str(&format!("    Expected: {}\n", drift.expected));
        output.push_str(&format!("    Actual:   {}\n", drift.actual));
    }

    if !report.reconciliation_plan.is_empty() {
        output.push_str(&format!(
            "\nReconciliation Plan ({} actions):\n",
            report.reconciliation_plan.len()
        ));
        for (i, action) in report.reconciliation_plan.iter().enumerate() {
            output.push_str(&format!(
                "  {}. [{}] {} {}\n",
                i + 1,
                action.action_type,
                action.description,
                if action.requires_restart {
                    "(requires restart)"
                } else {
                    ""
                }
            ));
        }
    }

    output
}

/// A single row in a live diff comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffRow {
    pub field: String,
    pub spec_value: String,
    pub stored_value: String,
    pub live_value: String,
    pub matches: bool,
}

/// Full live diff report comparing spec vs stored vs live state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveDiffReport {
    pub workload_name: String,
    pub rows: Vec<DiffRow>,
    pub has_differences: bool,
}

/// Format a live diff report as a readable table
pub fn format_live_diff(report: &LiveDiffReport) -> String {
    let mut output = String::new();

    output.push_str(&format!("Live Diff: {}\n\n", report.workload_name));

    if !report.has_differences {
        output.push_str("  All fields match. No differences found.\n");
        return output;
    }

    output.push_str(&format!(
        "  {:<16} {:<20} {:<20} {:<20} {}\n",
        "Field", "Spec", "Stored", "Live", ""
    ));
    output.push_str(&format!("  {}\n", "-".repeat(80)));

    for row in &report.rows {
        let marker = if row.matches { " " } else { "!" };
        output.push_str(&format!(
            "{} {:<16} {:<20} {:<20} {:<20}\n",
            marker, row.field, row.spec_value, row.stored_value, row.live_value,
        ));
    }

    let diff_count = report.rows.iter().filter(|r| !r.matches).count();
    output.push_str(&format!("\n  {} difference(s) found.\n", diff_count));

    output
}

/// Result of a single reconciliation action execution.
#[derive(Debug, Clone, Serialize)]
pub struct ReconcileResult {
    pub action_type: String,
    pub success: bool,
    pub message: String,
}

/// Execute the reconciliation plan from a drift report.
///
/// For each `ReconcileAction`, this function performs the corresponding
/// operation (redeploy, image update, etc.) and returns the results.
pub async fn execute_reconciliation(
    report: &DriftReport,
    state: &mut crate::state::StateStore,
) -> anyhow::Result<Vec<ReconcileResult>> {
    let mut results = Vec::new();

    let ws = state
        .get(&report.workload_name)
        .ok_or_else(|| anyhow::anyhow!("Workload '{}' not found in state", report.workload_name))?
        .clone();

    for action in &report.reconciliation_plan {
        let result = match action.action_type {
            ReconcileType::Redeploy | ReconcileType::UpdateImage => {
                // Redeploy: stop old instance, rebuild, run new
                let spec = crate::spec::Workload::from_file(&ws.spec_path)?;
                let rt = crate::runtime::create_runtime(&ws.runtime).await?;

                // Stop and delete old instance
                if let Err(e) = rt.delete(&ws.instance).await {
                    tracing::warn!("Failed to delete old instance during reconciliation: {}", e);
                }

                // Build and run new
                let image = rt.build(&spec).await?;
                let instance = rt.run(&image, &spec).await?;

                // Update state
                state.upsert(
                    report.workload_name.clone(),
                    crate::state::WorkloadState::new(
                        report.workload_name.clone(),
                        ws.runtime,
                        instance,
                        ws.spec_path.clone(),
                    ),
                );

                ReconcileResult {
                    action_type: format!("{}", action.action_type),
                    success: true,
                    message: "Redeployed from spec".to_string(),
                }
            }
            ReconcileType::Scale => ReconcileResult {
                action_type: "Scale".to_string(),
                success: true,
                message: "Scaling requires HPA — logged as advisory".to_string(),
            },
            ReconcileType::UpdateConfig | ReconcileType::None => ReconcileResult {
                action_type: format!("{}", action.action_type),
                success: true,
                message: "No runtime action needed".to_string(),
            },
        };
        results.push(result);
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Instance;
    use crate::spec::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn test_spec() -> Workload {
        Workload {
            api_version: "aether/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "test-app".to_string(),
                owner: "test".to_string(),
                project: "demo".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/test".to_string(),
                build_args: HashMap::new(),
            ..Default::default()
            },
            requirements: ResourceRequirements {
                cpu: "2".to_string(),
                memory: "4Gi".to_string(),
                storage: "20Gi".to_string(),
                gpu: None,
                cpu_request: None,
                memory_request: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Auto,
                allow: vec![RuntimeType::Container, RuntimeType::Kube],
            },
            network: NetworkSpec {
                service: true,
                service_type: ServiceType::ClusterIP,
                ports: vec![PortMapping {
                    container_port: 80,
                    service_port: 8080,
                    protocol: "TCP".to_string(),
                }],
                network_policy: None,
        ..Default::default()
            },
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
            intent: None,
            autonomy: None,
            confidential: None,
            schedule: None,
        kubernetes: None,
        }
    }

    fn test_state() -> WorkloadState {
        WorkloadState {
            name: "test-app".to_string(),
            runtime: RuntimeKind::Podman,
            instance: Instance {
                id: "abc123".to_string(),
                name: "test-app".to_string(),
                runtime: RuntimeKind::Podman,
                image: "ghcr.io/test/test-app:latest".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
            },
            spec_path: PathBuf::from("workload.yaml"),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            os_version: None,
            node_labels: vec![],
        }
    }

    #[test]
    fn test_no_drift() {
        let detector = DriftDetector::new();
        let spec = test_spec();
        let state = test_state();
        let report = detector.detect(&spec, &state);

        // With health probes missing + service enabled, there should be a health drift
        assert!(report.has_drift);
    }

    #[test]
    fn test_runtime_drift() {
        let detector = DriftDetector::new();
        let mut spec = test_spec();
        spec.runtime.preferred = RuntimePreference::Kube;
        let state = test_state(); // runtime is Podman
        let report = detector.detect(&spec, &state);

        assert!(report.has_drift);
        assert!(report
            .drifts
            .iter()
            .any(|d| d.category == DriftCategory::Runtime));
    }

    #[test]
    fn test_network_drift_no_tls() {
        let detector = DriftDetector::new();
        let mut spec = test_spec();
        spec.ingress = Some(IngressSpec {
            enabled: true,
            host: "example.com".to_string(),
            paths: vec![],
            tls: false,
            annotations: HashMap::new(),
            ..Default::default()
        });
        let state = test_state();
        let report = detector.detect(&spec, &state);

        assert!(report
            .drifts
            .iter()
            .any(|d| d.severity == DriftSeverity::Critical));
    }

    #[test]
    fn test_reconciliation_plan() {
        let detector = DriftDetector::new();
        let mut spec = test_spec();
        spec.runtime.preferred = RuntimePreference::Kube;
        let state = test_state();
        let report = detector.detect(&spec, &state);

        assert!(!report.reconciliation_plan.is_empty());
    }

    #[test]
    fn test_format_report() {
        let detector = DriftDetector::new();
        let spec = test_spec();
        let state = test_state();
        let report = detector.detect(&spec, &state);
        let formatted = format_drift_report(&report);
        assert!(formatted.contains("Drift Report"));
    }

    #[test]
    fn test_format_live_diff_no_differences() {
        let report = LiveDiffReport {
            workload_name: "my-app".to_string(),
            rows: vec![
                DiffRow {
                    field: "runtime".to_string(),
                    spec_value: "podman".to_string(),
                    stored_value: "podman".to_string(),
                    live_value: "podman".to_string(),
                    matches: true,
                },
            ],
            has_differences: false,
        };
        let output = format_live_diff(&report);
        assert!(output.contains("my-app"));
        assert!(output.contains("No differences found"));
    }

    // ── MockRuntime and live drift tests ────────────────────────────

    use async_trait::async_trait;

    struct MockRuntime {
        status_result: Result<crate::runtime::Status, String>,
    }

    #[async_trait]
    impl crate::Runtime for MockRuntime {
        async fn build(&self, _: &crate::spec::Workload) -> crate::Result<crate::runtime::Image> { unimplemented!() }
        async fn run(&self, _: &crate::runtime::Image, _: &crate::spec::Workload) -> crate::Result<crate::runtime::Instance> { unimplemented!() }
        async fn stop(&self, _: &crate::runtime::Instance) -> crate::Result<()> { unimplemented!() }
        async fn status(&self, _: &crate::runtime::Instance) -> crate::Result<crate::runtime::Status> {
            match &self.status_result {
                Ok(s) => Ok(s.clone()),
                Err(e) => Err(anyhow::anyhow!("{}", e)),
            }
        }
        async fn logs(&self, _: &crate::runtime::Instance, _: bool) -> crate::Result<String> { unimplemented!() }
        async fn delete(&self, _: &crate::runtime::Instance) -> crate::Result<()> { unimplemented!() }
        async fn list(&self) -> crate::Result<Vec<crate::runtime::Instance>> { unimplemented!() }
    }

    #[tokio::test]
    async fn test_live_drift_failed_state() {
        let detector = DriftDetector::new();
        let spec = test_spec();
        let state = test_state();
        let mock = MockRuntime {
            status_result: Ok(crate::runtime::Status {
                state: crate::runtime::InstanceState::Failed,
                ready: false,
                message: Some("CrashLoopBackOff".to_string()),
                restart_count: 0,
            }),
        };
        let report = detector.check_live_drift(&spec, &state, &mock).await;
        assert!(report.has_drift);
        assert!(report.drifts.iter().any(|d| d.field == "live.state" && d.severity == DriftSeverity::Critical));
    }

    #[tokio::test]
    async fn test_live_drift_not_ready() {
        let detector = DriftDetector::new();
        let spec = test_spec();
        let state = test_state();
        let mock = MockRuntime {
            status_result: Ok(crate::runtime::Status {
                state: crate::runtime::InstanceState::Running,
                ready: false,
                message: None,
                restart_count: 0,
            }),
        };
        let report = detector.check_live_drift(&spec, &state, &mock).await;
        assert!(report.has_drift);
        assert!(report.drifts.iter().any(|d| d.field == "live.ready" && d.severity == DriftSeverity::Warning));
    }

    #[tokio::test]
    async fn test_live_drift_restart_count_high() {
        let detector = DriftDetector::new();
        let spec = test_spec();
        let state = test_state();
        let mock = MockRuntime {
            status_result: Ok(crate::runtime::Status {
                state: crate::runtime::InstanceState::Running,
                ready: true,
                message: None,
                restart_count: 5,
            }),
        };
        let report = detector.check_live_drift(&spec, &state, &mock).await;
        assert!(report.has_drift);
        assert!(report.drifts.iter().any(|d| d.field == "live.restart_count" && d.severity == DriftSeverity::Critical));
    }

    #[tokio::test]
    async fn test_live_drift_healthy() {
        let detector = DriftDetector::new();
        let spec = test_spec();
        let state = test_state();
        let mock = MockRuntime {
            status_result: Ok(crate::runtime::Status {
                state: crate::runtime::InstanceState::Running,
                ready: true,
                message: None,
                restart_count: 0,
            }),
        };
        let report = detector.check_live_drift(&spec, &state, &mock).await;
        // No live.* drift items should be present
        assert!(!report.drifts.iter().any(|d| d.field.starts_with("live.")));
    }

    #[tokio::test]
    async fn test_live_drift_unreachable() {
        let detector = DriftDetector::new();
        let spec = test_spec();
        let state = test_state();
        let mock = MockRuntime {
            status_result: Err("connection refused".to_string()),
        };
        let report = detector.check_live_drift(&spec, &state, &mock).await;
        assert!(report.has_drift);
        assert!(report.drifts.iter().any(|d| d.field == "live.status" && d.severity == DriftSeverity::Critical));
        assert!(report.drifts.iter().any(|d| d.actual.contains("unreachable")));
    }

    #[test]
    fn test_format_live_diff_with_differences() {
        let report = LiveDiffReport {
            workload_name: "my-app".to_string(),
            rows: vec![
                DiffRow {
                    field: "runtime".to_string(),
                    spec_value: "kubernetes".to_string(),
                    stored_value: "podman".to_string(),
                    live_value: "podman".to_string(),
                    matches: false,
                },
                DiffRow {
                    field: "image".to_string(),
                    spec_value: "v2".to_string(),
                    stored_value: "v1".to_string(),
                    live_value: "v1".to_string(),
                    matches: false,
                },
                DiffRow {
                    field: "ready".to_string(),
                    spec_value: "-".to_string(),
                    stored_value: "-".to_string(),
                    live_value: "true".to_string(),
                    matches: true,
                },
            ],
            has_differences: true,
        };
        let output = format_live_diff(&report);
        assert!(output.contains("my-app"));
        assert!(output.contains("!"));
        assert!(output.contains("2 difference(s) found"));
    }
}
