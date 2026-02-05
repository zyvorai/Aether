//! Metal3 bare metal runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::Workload;
use async_trait::async_trait;

/// Metal3 runtime implementation (stub)
pub struct Metal3Runtime;

impl Metal3Runtime {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self)
    }
}

impl Default for Metal3Runtime {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Metal3 runtime")
    }
}

#[async_trait]
impl Runtime for Metal3Runtime {
    async fn build(&self, spec: &Workload) -> crate::Result<Image> {
        // TODO: Push image to registry
        tracing::warn!("Metal3 build not yet implemented");
        Ok(Image {
            name: spec.metadata.name.clone(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::Metal3,
        })
    }

    async fn run(&self, _image: &Image, spec: &Workload) -> crate::Result<Instance> {
        // TODO: Provision BareMetalHost
        tracing::warn!("Metal3 run not yet implemented");
        Ok(Instance {
            id: format!("metal-{}", spec.metadata.name),
            name: spec.metadata.name.clone(),
            runtime: RuntimeKind::Metal3,
            image: spec.image_name(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn stop(&self, _instance: &Instance) -> crate::Result<()> {
        tracing::warn!("Metal3 stop not yet implemented");
        Ok(())
    }

    async fn status(&self, _instance: &Instance) -> crate::Result<Status> {
        tracing::warn!("Metal3 status not yet implemented");
        Ok(Status {
            state: InstanceState::Unknown,
            ready: false,
            message: Some("Not implemented".to_string()),
            restart_count: 0,
        })
    }

    async fn logs(&self, _instance: &Instance, _follow: bool) -> crate::Result<String> {
        tracing::warn!("Metal3 logs not yet implemented");
        Ok("Logs not available".to_string())
    }

    async fn delete(&self, _instance: &Instance) -> crate::Result<()> {
        tracing::warn!("Metal3 delete not yet implemented");
        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        tracing::warn!("Metal3 list not yet implemented");
        Ok(vec![])
    }
}
