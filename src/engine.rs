// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

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
                    // Isolation requested but KubeVirt not allowed —
                    // fall through to normal rules with a warning
                    tracing::warn!("Intent requires isolation but KubeVirt not in allowed list");
                }
            }
        }

        // Rule 1: GPU required -> KubeVirt
        if spec.requirements.gpu.is_some() && self.is_allowed(spec, RuntimeType::Kubevirt) {
            return Ok(RuntimeKind::KubeVirt);
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

    /// Convert RuntimeType to RuntimeKind
    fn runtime_type_to_kind(&self, runtime_type: &RuntimeType) -> RuntimeKind {
        match runtime_type {
            RuntimeType::Container => RuntimeKind::Podman,
            RuntimeType::Kube => RuntimeKind::Kubernetes,
            RuntimeType::Kubevirt => RuntimeKind::KubeVirt,
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
        let spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kube],
        );

        let runtime = engine.decide(&spec).unwrap();
        assert_eq!(runtime, RuntimeKind::Podman);
    }

    #[test]
    fn test_auto_decide_gpu() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Container, RuntimeType::Kubevirt],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
            vgpu_profile: None,
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

    fn create_test_workload(preferred: RuntimePreference, allow: Vec<RuntimeType>) -> Workload {
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
            runtime: RuntimeSpec { preferred, allow },
            network: NetworkSpec::default(),
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
            kubevirt: None,
        }
    }

    // ---------------------------------------------------------------
    // Explicit runtime preference tests
    // ---------------------------------------------------------------

    #[test]
    fn test_explicit_container_preference() {
        let engine = Engine::new();
        let spec = create_test_workload(RuntimePreference::Container, vec![RuntimeType::Container]);
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_explicit_kubevirt_preference() {
        let engine = Engine::new();
        let spec = create_test_workload(RuntimePreference::Kubevirt, vec![RuntimeType::Kubevirt]);
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_explicit_kube_preference() {
        let engine = Engine::new();
        let spec = create_test_workload(RuntimePreference::Kube, vec![RuntimeType::Kube]);
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
            vgpu_profile: None,
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
            vgpu_profile: None,
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
            vgpu_profile: None,
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
            vgpu_profile: None,
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
        let mut spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Container]);
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
        let mut spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Container]);
        spec.persistence.enabled = true;
        // Kube not allowed, falls through to Container
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    // ---------------------------------------------------------------
    // Rule priority / ordering tests
    // ---------------------------------------------------------------

    #[test]
    fn test_gpu_takes_priority_over_network_service() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![
                RuntimeType::Kubevirt,
                RuntimeType::Kube,
                RuntimeType::Container,
            ],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
            vgpu_profile: None,
        });
        spec.network.service = true;
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
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
        let spec = create_test_workload(RuntimePreference::Auto, vec![]);
        let result = engine.decide(&spec);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No suitable runtime"),
            "Expected 'No suitable runtime' error"
        );
    }

    #[test]
    fn test_single_allowed_runtime_container() {
        let engine = Engine::new();
        let spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Container]);
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_single_allowed_runtime_kube() {
        let engine = Engine::new();
        let spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Kube]);
        // No network service or persistence, Container not allowed,
        // fallback picks first allowed runtime
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_single_allowed_runtime_kubevirt() {
        let engine = Engine::new();
        let spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Kubevirt]);
        // No GPU, no service, no persistence, Container not allowed
        // Fallback: first allowed = Kubevirt
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
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
            ],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
            vgpu_profile: None,
        });
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
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
        let spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Container]);
        assert!(!engine.is_allowed(&spec, RuntimeType::Kube));
        assert!(!engine.is_allowed(&spec, RuntimeType::Kubevirt));
    }

    #[test]
    fn test_is_allowed_empty_allow_list() {
        let engine = Engine::new();
        let spec = create_test_workload(RuntimePreference::Auto, vec![]);
        assert!(!engine.is_allowed(&spec, RuntimeType::Container));
        assert!(!engine.is_allowed(&spec, RuntimeType::Kube));
        assert!(!engine.is_allowed(&spec, RuntimeType::Kubevirt));
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

    // ---------------------------------------------------------------
    // Fallback behavior tests
    // ---------------------------------------------------------------

    #[test]
    fn test_fallback_picks_first_allowed_kube() {
        let engine = Engine::new();
        let spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Kube]);
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
            vgpu_profile: None,
        });
        spec.network.service = true;
        // GPU rule fires first
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
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
            ],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 4,
            vendor: "nvidia".to_string(),
            vgpu_profile: None,
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
            ],
        );
        spec.requirements.cpu = "64".to_string();
        spec.requirements.memory = "256Gi".to_string();
        spec.network.service = true;
        spec.persistence.enabled = true;
        // No GPU -> network service (Rule 3) fires next
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
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
            vgpu_profile: None,
        });
        spec.requirements.cpu = "32".to_string();
        spec.network.service = true;
        // GPU triggers Rule 1 but Kubevirt not allowed
        // Network service triggers Rule 3 and Kube IS allowed
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_cascade_all_rules_disallowed_falls_to_container() {
        let engine = Engine::new();
        let mut spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Container]);
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
            vgpu_profile: None,
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
        let mut spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Kube]);
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

    use crate::spec::{ComplianceSpec, IntentGoal, IntentSpec};

    #[test]
    fn test_isolation_required_selects_kubevirt() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![
                RuntimeType::Container,
                RuntimeType::Kube,
                RuntimeType::Kubevirt,
            ],
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
            storage: None,
        });
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }

    #[test]
    fn test_isolation_falls_through_when_neither_allowed() {
        let engine = Engine::new();
        let mut spec = create_test_workload(RuntimePreference::Auto, vec![RuntimeType::Container]);
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
            storage: None,
        });
        // KubeVirt not allowed — falls through to Podman
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
            storage: None,
        });
        // Intent without compliance doesn't affect rule-based engine
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::Podman);
    }

    #[test]
    fn test_isolation_takes_priority_over_gpu() {
        let engine = Engine::new();
        let mut spec = create_test_workload(
            RuntimePreference::Auto,
            vec![RuntimeType::Kubevirt],
        );
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
            vgpu_profile: None,
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
            storage: None,
        });
        // Rule 0 (intent isolation) fires before Rule 1 (GPU)
        // Both would select KubeVirt, so the result is the same
        assert_eq!(engine.decide(&spec).unwrap(), RuntimeKind::KubeVirt);
    }
}
