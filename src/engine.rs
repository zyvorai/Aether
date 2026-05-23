//! Runtime decision engine
//!
//! Determines which runtime to use based on workload requirements

use crate::runtime::RuntimeKind;
use crate::spec::{RuntimePreference, RuntimeType, Workload};

/// Decision engine for selecting appropriate runtime
pub struct Engine;

impl Engine {
    pub fn new() -> Self {
        Self
    }

    /// Decide which runtime to use based on workload spec
    pub fn decide(&self, spec: &Workload) -> anyhow::Result<RuntimeKind> {
        match spec.runtime.preferred {
            RuntimePreference::Auto => self.auto_decide(spec),
            RuntimePreference::Container => Ok(RuntimeKind::Podman),
            RuntimePreference::Kube => Ok(RuntimeKind::Kubernetes),
            RuntimePreference::Kubevirt => Ok(RuntimeKind::KubeVirt),
            RuntimePreference::Metal => Ok(RuntimeKind::Metal3),
        }
    }

    /// Automatic runtime selection based on requirements
    fn auto_decide(&self, spec: &Workload) -> anyhow::Result<RuntimeKind> {
        // Rule 0: Intent compliance — isolation requirement overrides all other rules
        if let Some(ref intent) = spec.intent {
            if let Some(ref compliance) = intent.compliance {
                if compliance.isolation_required {
                    if self.is_allowed(spec, RuntimeType::Kubevirt) {
                        return Ok(RuntimeKind::KubeVirt);
                    }
                    if self.is_allowed(spec, RuntimeType::Metal) {
                        return Ok(RuntimeKind::Metal3);
                    }
                    // Isolation requested but neither VM nor bare-metal allowed —
                    // fall through to normal rules with a warning
                    tracing::warn!(
                        "Intent requires isolation but KubeVirt/Metal3 not in allowed list"
                    );
                }
            }
        }

        // Rule 1: GPU required -> KubeVirt
        if spec.requirements.gpu.is_some() && self.is_allowed(spec, RuntimeType::Kubevirt) {
            return Ok(RuntimeKind::KubeVirt);
        }

        // Rule 2: Large resource requirements -> Metal3
        if self.needs_bare_metal(spec) && self.is_allowed(spec, RuntimeType::Metal) {
            return Ok(RuntimeKind::Metal3);
        }

        // Rule 3: Network service enabled -> Kubernetes
        if spec.network.service && self.is_allowed(spec, RuntimeType::Kube) {
            return Ok(RuntimeKind::Kubernetes);
        }

        // Rule 4: Persistence enabled -> Kubernetes
        if spec.persistence.enabled && self.is_allowed(spec, RuntimeType::Kube) {
            return Ok(RuntimeKind::Kubernetes);
        }

        // Default: Container (Podman) for local development
        if self.is_allowed(spec, RuntimeType::Container) {
            return Ok(RuntimeKind::Podman);
        }

        // Fallback: First allowed runtime
        if let Some(first) = spec.runtime.allow.first() {
            return Ok(self.runtime_type_to_kind(first));
        }

        anyhow::bail!("No suitable runtime found for workload")
    }

    /// Check if runtime type is allowed
    fn is_allowed(&self, spec: &Workload, runtime_type: RuntimeType) -> bool {
        spec.runtime.allow.contains(&runtime_type)
    }

    /// Determine if workload needs bare metal
    fn needs_bare_metal(&self, spec: &Workload) -> bool {
        // Parse CPU requirements
        let cpu = self.parse_cpu(&spec.requirements.cpu);
        let memory = self.parse_memory(&spec.requirements.memory);

        // Large resources threshold — workloads exceeding these are candidates
        // for bare-metal provisioning via Metal3.
        const BARE_METAL_CPU_THRESHOLD: f64 = 16.0;    // cores
        const BARE_METAL_MEM_THRESHOLD: f64 = 64.0 * 1024.0 * 1024.0 * 1024.0; // 64 GiB
        cpu > BARE_METAL_CPU_THRESHOLD || memory > BARE_METAL_MEM_THRESHOLD
    }

    /// Parse CPU string (e.g., "2", "2000m"), defaulting to 1.0 on bad input
    fn parse_cpu(&self, cpu: &str) -> f64 {
        let v = crate::resources::parse_cpu(cpu);
        if v > 0.0 {
            v
        } else {
            tracing::warn!(
                "Could not parse CPU value '{}', defaulting to 1.0 core for runtime decision",
                cpu
            );
            1.0
        }
    }

    /// Parse memory string (e.g., "4Gi", "4096Mi") to bytes, defaulting to 1Gi on bad input
    fn parse_memory(&self, memory: &str) -> f64 {
        let gi = crate::resources::parse_memory_gi(memory);
        if gi > 0.0 {
            gi * 1024.0 * 1024.0 * 1024.0
        } else {
            tracing::warn!(
                "Could not parse memory value '{}', defaulting to 1Gi for runtime decision",
                memory
            );
            1024.0 * 1024.0 * 1024.0
        }
    }

    /// Convert RuntimeType to RuntimeKind
    fn runtime_type_to_kind(&self, runtime_type: &RuntimeType) -> RuntimeKind {
        match runtime_type {
            RuntimeType::Container => RuntimeKind::Podman,
            RuntimeType::Kube => RuntimeKind::Kubernetes,
            RuntimeType::Kubevirt => RuntimeKind::KubeVirt,
            RuntimeType::Metal => RuntimeKind::Metal3,
        }
    }
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    #[test]
    fn test_auto_decide_container() {
        let engine = Engine::new();
        let spec = create_test_workload(RuntimePreference::Auto, vec![
            RuntimeType::Container,
            RuntimeType::Kube,
        ]);

        let runtime = engine.decide(&spec).unwrap();
        assert_eq!(runtime, RuntimeKind::Podman);
    }

    #[test]
    fn test_auto_decide_gpu() {
        let engine = Engine::new();
        let mut spec = create_test_workload(RuntimePreference::Auto, vec![
            RuntimeType::Container,
            RuntimeType::Kubevirt,
        ]);
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
        });

        let runtime = engine.decide(&spec).unwrap();
        assert_eq!(runtime, RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_explicit_runtime() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Kube,
            vec![RuntimeType::Kube, RuntimeType::Container],
        );

        let runtime = engine.decide(&spec).unwrap();
        assert_eq!(runtime, RuntimeKind::Kubernetes);
    }

    fn create_test_workload(
        preferred: RuntimePreference,
        allow: Vec<RuntimeType>,
    ) -> Workload {
        Workload {
            api_version: "aether/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "test".to_string(),
                owner: "test".to_string(),
                project: "test".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/test".to_string(),
                build_args: HashMap::new(),
            },
            requirements: ResourceRequirements {
                cpu: "2".to_string(),
                memory: "4Gi".to_string(),
                storage: "20Gi".to_string(),
                gpu: None,
                cpu_request: None,
                memory_request: None,
            },
            runtime: RuntimeSpec { preferred, allow },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
            intent: None,
            schedule: None,
        kubernetes: None,
        }
    }

    // ---------------------------------------------------------------
    // Explicit runtime preference tests
    // ---------------------------------------------------------------

    #[test]
    fn test_explicit_container_preference() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Container,
            vec![RuntimeType::Container],
        );
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_explicit_kubevirt_preference() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Kubevirt,
            vec![RuntimeType::Kubevirt],
        );
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_explicit_metal_preference() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Metal,
            vec![RuntimeType::Metal],
        );
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_explicit_kube_preference() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Kube,
            vec![RuntimeType::Kube],
        );
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    // ---------------------------------------------------------------
    // GPU detection tests
    // ---------------------------------------------------------------

    #[test]
    fn test_gpu_nvidia_selects_kubevirt() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kubevirt],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 2,
            vendor: "nvidia".to_string(),
        });
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_gpu_amd_selects_kubevirt() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kubevirt],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "amd".to_string(),
        });
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_gpu_intel_selects_kubevirt() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kubevirt],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 4,
            vendor: "intel".to_string(),
        });
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_gpu_required_but_kubevirt_not_allowed_falls_through() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
        });
        // GPU is set but KubeVirt is not in the allow list,
        // so the engine should fall through to the next rule.
        // No network service or persistence, so it should pick Container.
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_gpu_none_does_not_trigger_kubevirt() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kubevirt],
        );
        // gpu is None by default
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    // ---------------------------------------------------------------
    // Bare-metal / high-resource detection tests
    // ---------------------------------------------------------------

    #[test]
    fn test_high_cpu_selects_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        spec.requirements.cpu = "32".to_string();
        spec.requirements.memory = "4Gi".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_cpu_at_threshold_does_not_select_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        spec.requirements.cpu = "16".to_string(); // exactly 16, threshold is > 16
        spec.requirements.memory = "4Gi".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_cpu_just_above_threshold_selects_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        spec.requirements.cpu = "17".to_string();
        spec.requirements.memory = "4Gi".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_high_memory_selects_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        spec.requirements.cpu = "2".to_string();
        spec.requirements.memory = "128Gi".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_memory_at_threshold_does_not_select_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        spec.requirements.cpu = "2".to_string();
        spec.requirements.memory = "64Gi".to_string(); // exactly 64Gi, threshold is > 64Gi
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_memory_just_above_threshold_selects_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        spec.requirements.cpu = "2".to_string();
        spec.requirements.memory = "65Gi".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_high_cpu_millicores_selects_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        // 20000m == 20 cores, above 16-core threshold
        spec.requirements.cpu = "20000m".to_string();
        spec.requirements.memory = "4Gi".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_cpu_millicores_at_threshold_does_not_select_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        // 16000m == 16 cores, exactly at threshold
        spec.requirements.cpu = "16000m".to_string();
        spec.requirements.memory = "4Gi".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_high_memory_mi_selects_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        spec.requirements.cpu = "2".to_string();
        // 65536Mi == 64Gi, at threshold (not above), so NOT metal3
        spec.requirements.memory = "65536Mi".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_high_memory_mi_above_threshold_selects_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        spec.requirements.cpu = "2".to_string();
        // 65537Mi is just above 64Gi
        spec.requirements.memory = "65537Mi".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_bare_metal_needed_but_not_allowed_falls_through() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.requirements.cpu = "32".to_string();
        spec.requirements.memory = "128Gi".to_string();
        // Metal is not allowed, so it should fall through to Container
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    // ---------------------------------------------------------------
    // Network service detection tests
    // ---------------------------------------------------------------

    #[test]
    fn test_network_service_selects_kubernetes() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.network.service = true;
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_network_service_false_does_not_select_kubernetes() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.network.service = false;
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_network_service_but_kube_not_allowed() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        spec.network.service = true;
        // Kube not allowed, so should fall through to Container
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    // ---------------------------------------------------------------
    // Persistence detection tests
    // ---------------------------------------------------------------

    #[test]
    fn test_persistence_enabled_selects_kubernetes() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.persistence.enabled = true;
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_persistence_disabled_does_not_select_kubernetes() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.persistence.enabled = false;
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_persistence_enabled_but_kube_not_allowed() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        spec.persistence.enabled = true;
        // Kube not allowed, falls through to Container
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    // ---------------------------------------------------------------
    // Rule priority / ordering tests
    // ---------------------------------------------------------------

    #[test]
    fn test_gpu_takes_priority_over_bare_metal() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kubevirt, RuntimeType::Metal, RuntimeType::Container],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
        });
        spec.requirements.cpu = "32".to_string();
        spec.requirements.memory = "128Gi".to_string();
        // GPU rule (Rule 1) should take priority over bare metal (Rule 2)
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_gpu_takes_priority_over_network_service() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kubevirt, RuntimeType::Kube, RuntimeType::Container],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
        });
        spec.network.service = true;
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_bare_metal_takes_priority_over_network_service() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Metal, RuntimeType::Kube, RuntimeType::Container],
        );
        spec.requirements.cpu = "32".to_string();
        spec.network.service = true;
        // Bare metal (Rule 2) should take priority over network service (Rule 3)
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_bare_metal_takes_priority_over_persistence() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Metal, RuntimeType::Kube, RuntimeType::Container],
        );
        spec.requirements.cpu = "32".to_string();
        spec.persistence.enabled = true;
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_network_service_takes_priority_over_persistence() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kube, RuntimeType::Container],
        );
        spec.network.service = true;
        spec.persistence.enabled = true;
        // Both trigger Kubernetes, but network service (Rule 3) is checked first
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    // ---------------------------------------------------------------
    // Edge cases: empty / minimal specs
    // ---------------------------------------------------------------

    #[test]
    fn test_empty_allow_list_returns_error() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![],
        );
        let result = engine.decide(&spec);
        assert!(result.is_err());
        assert!(
            result.unwrap_err().to_string().contains("No suitable runtime"),
            "Expected 'No suitable runtime' error"
        );
    }

    #[test]
    fn test_single_allowed_runtime_container() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_single_allowed_runtime_kube() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kube],
        );
        // No network service or persistence, Container not allowed,
        // fallback picks first allowed runtime
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_single_allowed_runtime_kubevirt() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kubevirt],
        );
        // No GPU, no bare metal, no service, no persistence, Container not allowed
        // Fallback: first allowed = Kubevirt
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_single_allowed_runtime_metal() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Metal],
        );
        // Fallback: first allowed = Metal
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_all_runtimes_allowed_defaults_to_container() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![
                RuntimeType::Container,
                RuntimeType::Kube,
                RuntimeType::Kubevirt,
                RuntimeType::Metal,
            ],
        );
        // No special requirements, Container is allowed -> Podman
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_all_runtimes_allowed_with_gpu() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![
                RuntimeType::Container,
                RuntimeType::Kube,
                RuntimeType::Kubevirt,
                RuntimeType::Metal,
            ],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
        });
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_all_runtimes_allowed_with_high_cpu() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![
                RuntimeType::Container,
                RuntimeType::Kube,
                RuntimeType::Kubevirt,
                RuntimeType::Metal,
            ],
        );
        spec.requirements.cpu = "64".to_string();
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    // ---------------------------------------------------------------
    // parse_cpu tests
    // ---------------------------------------------------------------

    #[test]
    fn test_parse_cpu_whole_cores() {
        let engine = Engine::new();
        assert!((engine.parse_cpu("1") - 1.0).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("4") - 4.0).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("16") - 16.0).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("128") - 128.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_cpu_millicores() {
        let engine = Engine::new();
        assert!((engine.parse_cpu("500m") - 0.5).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("1000m") - 1.0).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("2000m") - 2.0).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("250m") - 0.25).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("16000m") - 16.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_cpu_zero_defaults_to_one() {
        let engine = Engine::new();
        assert!((engine.parse_cpu("0") - 1.0).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("0m") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_cpu_invalid_defaults_to_one() {
        let engine = Engine::new();
        assert!((engine.parse_cpu("") - 1.0).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("abc") - 1.0).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("abcm") - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_cpu_fractional() {
        let engine = Engine::new();
        assert!((engine.parse_cpu("0.5") - 0.5).abs() < f64::EPSILON);
        assert!((engine.parse_cpu("2.5") - 2.5).abs() < f64::EPSILON);
    }

    // ---------------------------------------------------------------
    // parse_memory tests
    // ---------------------------------------------------------------

    #[test]
    fn test_parse_memory_gi() {
        let engine = Engine::new();
        let gi = 1024.0 * 1024.0 * 1024.0;
        assert!((engine.parse_memory("1Gi") - gi).abs() < 1.0);
        assert!((engine.parse_memory("4Gi") - 4.0 * gi).abs() < 1.0);
        assert!((engine.parse_memory("64Gi") - 64.0 * gi).abs() < 1.0);
        assert!((engine.parse_memory("128Gi") - 128.0 * gi).abs() < 1.0);
    }

    #[test]
    fn test_parse_memory_mi() {
        let engine = Engine::new();
        let mi = 1024.0 * 1024.0;
        assert!((engine.parse_memory("512Mi") - 512.0 * mi).abs() < 1.0);
        assert!((engine.parse_memory("4096Mi") - 4096.0 * mi).abs() < 1.0);
        assert!((engine.parse_memory("1024Mi") - 1024.0 * mi).abs() < 1.0);
    }

    #[test]
    fn test_parse_memory_ki() {
        let engine = Engine::new();
        let ki = 1024.0;
        assert!((engine.parse_memory("1024Ki") - 1024.0 * ki).abs() < 1.0);
        assert!((engine.parse_memory("512Ki") - 512.0 * ki).abs() < 1.0);
    }

    #[test]
    fn test_parse_memory_plain_bytes_defaults_to_1gi() {
        let engine = Engine::new();
        let one_gi = 1024.0 * 1024.0 * 1024.0;
        // Plain numbers without a suffix are invalid; engine defaults to 1Gi
        assert!((engine.parse_memory("1048576") - one_gi).abs() < 1.0);
        assert!((engine.parse_memory("0") - one_gi).abs() < 1.0);
    }

    #[test]
    fn test_parse_memory_invalid_defaults_to_1gi() {
        let engine = Engine::new();
        let one_gi = 1024.0 * 1024.0 * 1024.0;
        assert!((engine.parse_memory("") - one_gi).abs() < 1.0);
        assert!((engine.parse_memory("abc") - one_gi).abs() < 1.0);
        assert!((engine.parse_memory("abcGi") - one_gi).abs() < 1.0);
    }

    #[test]
    fn test_parse_memory_with_whitespace() {
        let engine = Engine::new();
        let gi = 1024.0 * 1024.0 * 1024.0;
        assert!((engine.parse_memory("  4Gi  ") - 4.0 * gi).abs() < 1.0);
        assert!((engine.parse_memory(" 128Gi ") - 128.0 * gi).abs() < 1.0);
    }

    #[test]
    fn test_parse_memory_fractional_gi() {
        let engine = Engine::new();
        let gi = 1024.0 * 1024.0 * 1024.0;
        assert!((engine.parse_memory("0.5Gi") - 0.5 * gi).abs() < 1.0);
        assert!((engine.parse_memory("1.5Gi") - 1.5 * gi).abs() < 1.0);
    }

    // ---------------------------------------------------------------
    // needs_bare_metal tests
    // ---------------------------------------------------------------

    #[test]
    fn test_needs_bare_metal_false_for_small_workload() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        assert!(!engine.needs_bare_metal(&spec));
    }

    #[test]
    fn test_needs_bare_metal_true_for_high_cpu() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        spec.requirements.cpu = "32".to_string();
        assert!(engine.needs_bare_metal(&spec));
    }

    #[test]
    fn test_needs_bare_metal_true_for_high_memory() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        spec.requirements.memory = "128Gi".to_string();
        assert!(engine.needs_bare_metal(&spec));
    }

    #[test]
    fn test_needs_bare_metal_true_for_both_high() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        spec.requirements.cpu = "64".to_string();
        spec.requirements.memory = "256Gi".to_string();
        assert!(engine.needs_bare_metal(&spec));
    }

    #[test]
    fn test_needs_bare_metal_boundary_cpu_16() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        spec.requirements.cpu = "16".to_string();
        spec.requirements.memory = "4Gi".to_string();
        // Exactly 16 is NOT > 16, so false
        assert!(!engine.needs_bare_metal(&spec));
    }

    #[test]
    fn test_needs_bare_metal_boundary_memory_64gi() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        spec.requirements.cpu = "2".to_string();
        spec.requirements.memory = "64Gi".to_string();
        // Exactly 64Gi is NOT > 64Gi, so false
        assert!(!engine.needs_bare_metal(&spec));
    }

    // ---------------------------------------------------------------
    // is_allowed tests
    // ---------------------------------------------------------------

    #[test]
    fn test_is_allowed_with_matching_type() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        assert!(engine.is_allowed(&spec, RuntimeType::Container));
        assert!(engine.is_allowed(&spec, RuntimeType::Kube));
    }

    #[test]
    fn test_is_allowed_with_non_matching_type() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        assert!(!engine.is_allowed(&spec, RuntimeType::Kube));
        assert!(!engine.is_allowed(&spec, RuntimeType::Kubevirt));
        assert!(!engine.is_allowed(&spec, RuntimeType::Metal));
    }

    #[test]
    fn test_is_allowed_empty_allow_list() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![],
        );
        assert!(!engine.is_allowed(&spec, RuntimeType::Container));
        assert!(!engine.is_allowed(&spec, RuntimeType::Kube));
        assert!(!engine.is_allowed(&spec, RuntimeType::Kubevirt));
        assert!(!engine.is_allowed(&spec, RuntimeType::Metal));
    }

    // ---------------------------------------------------------------
    // runtime_type_to_kind tests
    // ---------------------------------------------------------------

    #[test]
    fn test_runtime_type_to_kind_container() {
        let engine = Engine::new();
        assert_eq!(
            engine.runtime_type_to_kind(&RuntimeType::Container),
            RuntimeKind::Podman
        );
    }

    #[test]
    fn test_runtime_type_to_kind_kube() {
        let engine = Engine::new();
        assert_eq!(
            engine.runtime_type_to_kind(&RuntimeType::Kube),
            RuntimeKind::Kubernetes
        );
    }

    #[test]
    fn test_runtime_type_to_kind_kubevirt() {
        let engine = Engine::new();
        assert_eq!(
            engine.runtime_type_to_kind(&RuntimeType::Kubevirt),
            RuntimeKind::KubeVirt
        );
    }

    #[test]
    fn test_runtime_type_to_kind_metal() {
        let engine = Engine::new();
        assert_eq!(
            engine.runtime_type_to_kind(&RuntimeType::Metal),
            RuntimeKind::Metal3
        );
    }

    // ---------------------------------------------------------------
    // Fallback behavior tests
    // ---------------------------------------------------------------

    #[test]
    fn test_fallback_picks_first_allowed_when_container_not_in_list() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Metal],
        );
        // No rules matched (no GPU, no high resources, no service, no persistence)
        // Container not allowed, so fallback to first in allow list = Metal
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_fallback_picks_first_allowed_kubevirt() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kubevirt, RuntimeType::Metal],
        );
        // No rules matched; Container not allowed; first in allow list = Kubevirt
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_fallback_picks_first_allowed_kube() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kube, RuntimeType::Metal],
        );
        // No special requirements trigger Kube (no network service, no persistence)
        // Container not allowed; first allowed = Kube
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    // ---------------------------------------------------------------
    // Combined scenario tests
    // ---------------------------------------------------------------

    #[test]
    fn test_gpu_and_network_service_selects_kubevirt() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![
                RuntimeType::Container,
                RuntimeType::Kube,
                RuntimeType::Kubevirt,
            ],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 2,
            vendor: "nvidia".to_string(),
        });
        spec.network.service = true;
        // GPU rule fires first
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_high_resources_and_persistence_selects_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![
                RuntimeType::Container,
                RuntimeType::Kube,
                RuntimeType::Metal,
            ],
        );
        spec.requirements.cpu = "32".to_string();
        spec.persistence.enabled = true;
        // Bare metal (Rule 2) takes priority over persistence (Rule 4)
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_network_service_and_persistence_selects_kubernetes() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.network.service = true;
        spec.persistence.enabled = true;
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_all_triggers_active_all_runtimes_allowed() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![
                RuntimeType::Container,
                RuntimeType::Kube,
                RuntimeType::Kubevirt,
                RuntimeType::Metal,
            ],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 4,
            vendor: "nvidia".to_string(),
        });
        spec.requirements.cpu = "64".to_string();
        spec.requirements.memory = "256Gi".to_string();
        spec.network.service = true;
        spec.persistence.enabled = true;
        // GPU rule fires first regardless of all other criteria
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_all_triggers_except_gpu_all_runtimes_allowed() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![
                RuntimeType::Container,
                RuntimeType::Kube,
                RuntimeType::Kubevirt,
                RuntimeType::Metal,
            ],
        );
        spec.requirements.cpu = "64".to_string();
        spec.requirements.memory = "256Gi".to_string();
        spec.network.service = true;
        spec.persistence.enabled = true;
        // No GPU -> bare metal (Rule 2) fires next
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    // ---------------------------------------------------------------
    // Engine construction tests
    // ---------------------------------------------------------------

    #[test]
    fn test_engine_new() {
        let _engine = Engine::new();
        // Just verifying it constructs without panic
    }

    #[test]
    fn test_engine_construction() {
        let _engine = Engine::new();
        // Just verifying construction works
    }

    // ---------------------------------------------------------------
    // Cascade: all higher-priority rules disallowed
    // ---------------------------------------------------------------

    #[test]
    fn test_cascade_gpu_and_metal_disallowed_falls_to_kube_via_service() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
        });
        spec.requirements.cpu = "32".to_string();
        spec.network.service = true;
        // GPU triggers Rule 1 but Kubevirt not allowed
        // High CPU triggers Rule 2 but Metal not allowed
        // Network service triggers Rule 3 and Kube IS allowed
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_cascade_all_rules_disallowed_falls_to_container() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
        });
        spec.requirements.cpu = "32".to_string();
        spec.network.service = true;
        spec.persistence.enabled = true;
        // All specific rules disallowed, Container is allowed
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_cascade_only_persistence_path_available() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kube],
        );
        // No GPU, no high resources, no network service
        spec.persistence.enabled = true;
        // Persistence triggers Rule 4, Kube allowed
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    // ---------------------------------------------------------------
    // Network spec variants
    // ---------------------------------------------------------------

    #[test]
    fn test_network_with_ports_but_no_service_flag() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.network.service = false;
        spec.network.ports = vec![PortMapping {
            container_port: 8080,
            service_port: 80,
            protocol: "TCP".to_string(),
        }];
        // service flag is false, so no Kubernetes
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_network_with_loadbalancer_and_service() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.network.service = true;
        spec.network.service_type = ServiceType::LoadBalancer;
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    // ---------------------------------------------------------------
    // Persistence spec variants
    // ---------------------------------------------------------------

    #[test]
    fn test_persistence_with_storage_class() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.persistence = PersistenceSpec {
            enabled: true,
            size: "100Gi".to_string(),
            access_mode: AccessMode::ReadWriteMany,
            storage_class: Some("fast-ssd".to_string()),
        };
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_persistence_with_readonly_access_mode() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.persistence = PersistenceSpec {
            enabled: true,
            size: "10Gi".to_string(),
            access_mode: AccessMode::ReadOnlyMany,
            storage_class: None,
        };
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    // ---------------------------------------------------------------
    // Intent compliance gate tests
    // ---------------------------------------------------------------

    use crate::spec::{IntentSpec, IntentGoal, ComplianceSpec};

    #[test]
    fn test_isolation_required_selects_kubevirt() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube, RuntimeType::Kubevirt],
        );
        spec.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: None,
            budget: None,
            resilience: None,
            compliance: Some(ComplianceSpec {
                isolation_required: true,
                encryption_required: false,
            }),
            trust: None,
        });
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_isolation_required_falls_back_to_metal3() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Metal],
        );
        spec.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: None,
            budget: None,
            resilience: None,
            compliance: Some(ComplianceSpec {
                isolation_required: true,
                encryption_required: false,
            }),
            trust: None,
        });
        // KubeVirt not allowed, should fall back to Metal3
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_isolation_falls_through_when_neither_allowed() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container],
        );
        spec.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: None,
            budget: None,
            resilience: None,
            compliance: Some(ComplianceSpec {
                isolation_required: true,
                encryption_required: false,
            }),
            trust: None,
        });
        // Neither KubeVirt nor Metal3 allowed — falls through to Podman
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_no_intent_unchanged_behavior() {
        let engine = Engine::new();
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        // No intent — should behave exactly as before (Podman for simple workload)
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_intent_without_compliance_unchanged() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );
        spec.intent = Some(IntentSpec {
            goal: IntentGoal::CostOptimized,
            sla: None,
            budget: None,
            resilience: None,
            compliance: None,
            trust: None,
        });
        // Intent without compliance doesn't affect rule-based engine
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_isolation_takes_priority_over_gpu() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kubevirt, RuntimeType::Metal],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
        });
        spec.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: None,
            budget: None,
            resilience: None,
            compliance: Some(ComplianceSpec {
                isolation_required: true,
                encryption_required: false,
            }),
            trust: None,
        });
        // Rule 0 (intent isolation) fires before Rule 1 (GPU)
        // Both would select KubeVirt, so the result is the same
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }
}
