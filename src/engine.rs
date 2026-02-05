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
        // Rule 1: GPU required -> KubeVirt
        if spec.requirements.gpu.is_some() {
            if self.is_allowed(spec, RuntimeType::Kubevirt) {
                return Ok(RuntimeKind::KubeVirt);
            }
        }

        // Rule 2: Large resource requirements -> Metal3
        if self.needs_bare_metal(spec) {
            if self.is_allowed(spec, RuntimeType::Metal) {
                return Ok(RuntimeKind::Metal3);
            }
        }

        // Rule 3: Network service enabled -> Kubernetes
        if spec.network.service {
            if self.is_allowed(spec, RuntimeType::Kube) {
                return Ok(RuntimeKind::Kubernetes);
            }
        }

        // Rule 4: Persistence enabled -> Kubernetes
        if spec.persistence.enabled {
            if self.is_allowed(spec, RuntimeType::Kube) {
                return Ok(RuntimeKind::Kubernetes);
            }
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

        // Large resources threshold
        cpu > 16.0 || memory > 64.0 * 1024.0 * 1024.0 * 1024.0 // 64Gi
    }

    /// Parse CPU string (e.g., "2", "2000m")
    fn parse_cpu(&self, cpu: &str) -> f64 {
        if let Some(stripped) = cpu.strip_suffix('m') {
            stripped.parse::<f64>().unwrap_or(0.0) / 1000.0
        } else {
            cpu.parse::<f64>().unwrap_or(0.0)
        }
    }

    /// Parse memory string (e.g., "4Gi", "4096Mi")
    fn parse_memory(&self, memory: &str) -> f64 {
        let memory = memory.trim();
        if let Some(stripped) = memory.strip_suffix("Gi") {
            stripped.parse::<f64>().unwrap_or(0.0) * 1024.0 * 1024.0 * 1024.0
        } else if let Some(stripped) = memory.strip_suffix("Mi") {
            stripped.parse::<f64>().unwrap_or(0.0) * 1024.0 * 1024.0
        } else if let Some(stripped) = memory.strip_suffix("Ki") {
            stripped.parse::<f64>().unwrap_or(0.0) * 1024.0
        } else {
            memory.parse::<f64>().unwrap_or(0.0)
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
            api_version: "orchestr8/v1".to_string(),
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
            },
            runtime: RuntimeSpec { preferred, allow },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
        }
    }
}
