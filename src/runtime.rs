//! Runtime trait and common types
//!
//! All runtime adapters (Podman, Kubernetes, KubeVirt, Metal3)
//! must implement this trait.

use crate::spec::Workload;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Universal runtime trait
#[async_trait]
pub trait Runtime: Send + Sync {
    /// Build an image from the workload spec
    async fn build(&self, spec: &Workload) -> crate::Result<Image>;

    /// Run a workload instance
    async fn run(&self, image: &Image, spec: &Workload) -> crate::Result<Instance>;

    /// Stop a running instance
    async fn stop(&self, instance: &Instance) -> crate::Result<()>;

    /// Get status of an instance
    async fn status(&self, instance: &Instance) -> crate::Result<Status>;

    /// Get logs from an instance
    async fn logs(&self, instance: &Instance, follow: bool) -> crate::Result<String>;

    /// Delete an instance
    async fn delete(&self, instance: &Instance) -> crate::Result<()>;

    /// List all instances managed by this runtime
    async fn list(&self) -> crate::Result<Vec<Instance>>;
}

/// Built image reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Image {
    pub name: String,
    pub tag: String,
    pub digest: Option<String>,
    pub runtime: RuntimeKind,
}

impl Image {
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }
}

/// Running instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub runtime: RuntimeKind,
    pub image: String,
    pub created_at: String,
}

impl Instance {
    /// Create a new instance, setting `created_at` to the current time.
    pub fn new(id: String, name: String, runtime: RuntimeKind, image: String) -> Self {
        Self {
            id,
            name,
            runtime,
            image,
            created_at: crate::resources::now_rfc3339(),
        }
    }
}

/// Instance status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub state: InstanceState,
    pub ready: bool,
    pub message: Option<String>,
    pub restart_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InstanceState {
    Pending,
    Running,
    Stopped,
    Failed,
    Unknown,
}

impl fmt::Display for InstanceState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InstanceState::Pending => write!(f, "pending"),
            InstanceState::Running => write!(f, "running"),
            InstanceState::Stopped => write!(f, "stopped"),
            InstanceState::Failed => write!(f, "failed"),
            InstanceState::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    Podman,
    Kubernetes,
    KubeVirt,
    Metal3,
}

impl RuntimeKind {
    /// All supported runtime variants.
    pub const ALL: [RuntimeKind; 4] = [
        RuntimeKind::Podman,
        RuntimeKind::Kubernetes,
        RuntimeKind::KubeVirt,
        RuntimeKind::Metal3,
    ];
}

impl fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RuntimeKind::Podman => write!(f, "podman"),
            RuntimeKind::Kubernetes => write!(f, "kubernetes"),
            RuntimeKind::KubeVirt => write!(f, "kubevirt"),
            RuntimeKind::Metal3 => write!(f, "metal3"),
        }
    }
}

impl std::str::FromStr for RuntimeKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "podman" | "container" => Ok(RuntimeKind::Podman),
            "kubernetes" | "kube" | "k8s" => Ok(RuntimeKind::Kubernetes),
            "kubevirt" | "vm" => Ok(RuntimeKind::KubeVirt),
            "metal3" | "metal" | "bare-metal" => Ok(RuntimeKind::Metal3),
            _ => Err(anyhow::anyhow!("Unknown runtime: '{}'. Valid: podman, kubernetes, kubevirt, metal3", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_kind_from_str_valid() {
        assert_eq!("podman".parse::<RuntimeKind>().unwrap(), RuntimeKind::Podman);
        assert_eq!("container".parse::<RuntimeKind>().unwrap(), RuntimeKind::Podman);
        assert_eq!("kubernetes".parse::<RuntimeKind>().unwrap(), RuntimeKind::Kubernetes);
        assert_eq!("kube".parse::<RuntimeKind>().unwrap(), RuntimeKind::Kubernetes);
        assert_eq!("k8s".parse::<RuntimeKind>().unwrap(), RuntimeKind::Kubernetes);
        assert_eq!("kubevirt".parse::<RuntimeKind>().unwrap(), RuntimeKind::KubeVirt);
        assert_eq!("vm".parse::<RuntimeKind>().unwrap(), RuntimeKind::KubeVirt);
        assert_eq!("metal3".parse::<RuntimeKind>().unwrap(), RuntimeKind::Metal3);
        assert_eq!("metal".parse::<RuntimeKind>().unwrap(), RuntimeKind::Metal3);
        assert_eq!("bare-metal".parse::<RuntimeKind>().unwrap(), RuntimeKind::Metal3);
    }

    #[test]
    fn test_runtime_kind_from_str_case_insensitive() {
        assert_eq!("PODMAN".parse::<RuntimeKind>().unwrap(), RuntimeKind::Podman);
        assert_eq!("Kubernetes".parse::<RuntimeKind>().unwrap(), RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_runtime_kind_from_str_invalid() {
        assert!("unknown".parse::<RuntimeKind>().is_err());
        assert!("docker".parse::<RuntimeKind>().is_err());
    }

    #[test]
    fn test_runtime_kind_display_roundtrip() {
        for rt in RuntimeKind::ALL {
            let s = rt.to_string();
            assert_eq!(s.parse::<RuntimeKind>().unwrap(), rt);
        }
    }

    #[test]
    fn test_instance_state_display() {
        assert_eq!(InstanceState::Running.to_string(), "running");
        assert_eq!(InstanceState::Stopped.to_string(), "stopped");
    }
}

/// Create a runtime instance for the given RuntimeKind.
pub async fn create_runtime(kind: &RuntimeKind) -> crate::Result<Box<dyn Runtime>> {
    use crate::adapters::{KubeVirtRuntime, KubernetesRuntime, Metal3Runtime, PodmanRuntime};

    match kind {
        RuntimeKind::Podman => Ok(Box::new(PodmanRuntime::new()?)),
        RuntimeKind::Kubernetes => Ok(Box::new(KubernetesRuntime::new().await?)),
        RuntimeKind::KubeVirt => Ok(Box::new(KubeVirtRuntime::new().await?)),
        RuntimeKind::Metal3 => Ok(Box::new(Metal3Runtime::new().await?)),
    }
}
