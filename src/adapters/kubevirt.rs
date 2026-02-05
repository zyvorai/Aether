//! KubeVirt VM runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::Workload;
use async_trait::async_trait;

/// KubeVirt runtime implementation (stub)
pub struct KubeVirtRuntime;

impl KubeVirtRuntime {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }
}

impl Default for KubeVirtRuntime {
    fn default() -> Self {
        Self::new().expect("Failed to initialize KubeVirt runtime")
    }
}

#[async_trait]
impl Runtime for KubeVirtRuntime {
    async fn build(&self, spec: &Workload) -> crate::Result<Image> {
        // TODO: Create DataVolume
        tracing::warn!("KubeVirt build not yet implemented");
        Ok(Image {
            name: spec.metadata.name.clone(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::KubeVirt,
        })
    }

    async fn run(&self, _image: &Image, spec: &Workload) -> crate::Result<Instance> {
        // TODO: Create VirtualMachine
        tracing::warn!("KubeVirt run not yet implemented");
        Ok(Instance {
            id: format!("vm-{}", spec.metadata.name),
            name: spec.metadata.name.clone(),
            runtime: RuntimeKind::KubeVirt,
            image: spec.image_name(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn stop(&self, _instance: &Instance) -> crate::Result<()> {
        tracing::warn!("KubeVirt stop not yet implemented");
        Ok(())
    }

    async fn status(&self, _instance: &Instance) -> crate::Result<Status> {
        tracing::warn!("KubeVirt status not yet implemented");
        Ok(Status {
            state: InstanceState::Unknown,
            ready: false,
            message: Some("Not implemented".to_string()),
            restart_count: 0,
        })
    }

    async fn logs(&self, _instance: &Instance, _follow: bool) -> crate::Result<String> {
        tracing::warn!("KubeVirt logs not yet implemented");
        Ok("Logs not available".to_string())
    }

    async fn delete(&self, _instance: &Instance) -> crate::Result<()> {
        tracing::warn!("KubeVirt delete not yet implemented");
        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        tracing::warn!("KubeVirt list not yet implemented");
        Ok(vec![])
    }
}
