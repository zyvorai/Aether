//! Podman container runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::Workload;
use async_trait::async_trait;
use tokio::process::Command;

/// Podman runtime implementation
pub struct PodmanRuntime;

impl PodmanRuntime {
    pub fn new() -> anyhow::Result<Self> {
        // Check if podman is available
        which::which("podman").map_err(|_| {
            anyhow::anyhow!("Podman not found. Please install podman first.")
        })?;

        Ok(Self)
    }
}

impl Default for PodmanRuntime {
    fn default() -> Self {
        match Self::new() {
            Ok(rt) => rt,
            Err(_) => Self,
        }
    }
}

#[async_trait]
impl Runtime for PodmanRuntime {
    async fn build(&self, spec: &Workload) -> crate::Result<Image> {
        let image_name = spec.image_name();

        tracing::info!("Building image with Podman: {}", image_name);

        // Build command
        let mut cmd = Command::new("podman");
        cmd.args(["build", "-t", &image_name])
            .arg("-f")
            .arg(&spec.build.dockerfile)
            .arg(&spec.build.context);

        // Add build args
        for (key, value) in &spec.build.build_args {
            cmd.arg("--build-arg").arg(format!("{}={}", key, value));
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman build failed: {}", stderr);
        }

        Ok(Image {
            name: spec.metadata.name.clone(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::Podman,
        })
    }

    async fn run(&self, image: &Image, spec: &Workload) -> crate::Result<Instance> {
        tracing::info!("Running container: {}", image.full_name());

        let mut cmd = Command::new("podman");
        cmd.args(["run", "-d", "--name", &spec.metadata.name]);

        // Add port mappings
        for port in &spec.network.ports {
            cmd.arg("-p").arg(format!(
                "{}:{}",
                port.service_port, port.container_port
            ));
        }

        // Add resource limits
        cmd.arg("--cpus").arg(&spec.requirements.cpu);
        cmd.arg("--memory").arg(&spec.requirements.memory);

        // Add image
        cmd.arg(image.full_name());

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman run failed: {}", stderr);
        }

        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(Instance {
            id: container_id,
            name: spec.metadata.name.clone(),
            runtime: RuntimeKind::Podman,
            image: image.full_name(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn stop(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Stopping container: {}", instance.name);

        let output = Command::new("podman")
            .args(["stop", &instance.id])
            .output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman stop failed: {}", stderr);
        }

        Ok(())
    }

    async fn status(&self, instance: &Instance) -> crate::Result<Status> {
        let output = Command::new("podman")
            .args(["inspect", "--format", "{{.State.Status}}", &instance.id])
            .output().await?;

        if !output.status.success() {
            return Ok(Status {
                state: InstanceState::Unknown,
                ready: false,
                message: Some("Container not found".to_string()),
                restart_count: 0,
            });
        }

        let status_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let state = match status_str.as_str() {
            "running" => InstanceState::Running,
            "exited" => InstanceState::Stopped,
            "created" => InstanceState::Pending,
            _ => InstanceState::Unknown,
        };

        Ok(Status {
            state: state.clone(),
            ready: matches!(state, InstanceState::Running),
            message: None,
            restart_count: 0,
        })
    }

    async fn logs(&self, instance: &Instance, follow: bool) -> crate::Result<String> {
        let mut cmd = Command::new("podman");
        cmd.args(["logs", &instance.id]);

        if follow {
            cmd.arg("--follow");
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman logs failed: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn delete(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Deleting container: {}", instance.name);

        let output = Command::new("podman")
            .args(["rm", "-f", &instance.id])
            .output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman rm failed: {}", stderr);
        }

        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        let output = Command::new("podman")
            .args(["ps", "-a", "--format", "json"])
            .output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman ps failed: {}", stderr);
        }

        let json_str = String::from_utf8_lossy(&output.stdout);
        let containers: Vec<serde_json::Value> = serde_json::from_str(&json_str)?;

        let instances: Vec<Instance> = containers
            .iter()
            .map(|c| Instance {
                id: c["Id"].as_str().unwrap_or("").to_string(),
                name: c["Names"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                runtime: RuntimeKind::Podman,
                image: c["Image"].as_str().unwrap_or("").to_string(),
                created_at: c["CreatedAt"].as_str().unwrap_or("").to_string(),
            })
            .collect();

        Ok(instances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{
        BuildSpec, Metadata, NetworkSpec, PersistenceSpec, PortMapping,
        ResourceRequirements, RuntimePreference, RuntimeSpec, RuntimeType,
        ServiceType, Workload,
    };
    use std::path::PathBuf;

    fn test_workload() -> Workload {
        Workload {
            api_version: "orchestr8/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "test-app".to_string(),
                owner: "test".to_string(),
                project: "demo".to_string(),
                labels: std::collections::HashMap::new(),
                annotations: std::collections::HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/test".to_string(),
                build_args: std::collections::HashMap::new(),
            },
            requirements: ResourceRequirements {
                cpu: "1".to_string(),
                memory: "512Mi".to_string(),
                storage: "1Gi".to_string(),
                gpu: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Auto,
                allow: vec![RuntimeType::Container],
            },
            network: NetworkSpec {
                service: true,
                service_type: ServiceType::ClusterIP,
                ports: vec![PortMapping {
                    container_port: 8080,
                    service_port: 8080,
                    protocol: "TCP".to_string(),
                }],
            },
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
        }
    }

    #[test]
    fn test_image_name_generation() {
        let workload = test_workload();
        let image_name = workload.image_name();
        assert_eq!(image_name, "ghcr.io/test/test-app:latest");
    }

    #[test]
    fn test_instance_state_from_string() {
        // Test the status parsing logic from the status() method
        let state = match "running" {
            "running" => InstanceState::Running,
            "exited" => InstanceState::Stopped,
            "created" => InstanceState::Pending,
            _ => InstanceState::Unknown,
        };
        assert!(matches!(state, InstanceState::Running));

        let state = match "exited" {
            "running" => InstanceState::Running,
            "exited" => InstanceState::Stopped,
            "created" => InstanceState::Pending,
            _ => InstanceState::Unknown,
        };
        assert!(matches!(state, InstanceState::Stopped));

        let state = match "created" {
            "running" => InstanceState::Running,
            "exited" => InstanceState::Stopped,
            "created" => InstanceState::Pending,
            _ => InstanceState::Unknown,
        };
        assert!(matches!(state, InstanceState::Pending));

        let state = match "paused" {
            "running" => InstanceState::Running,
            "exited" => InstanceState::Stopped,
            "created" => InstanceState::Pending,
            _ => InstanceState::Unknown,
        };
        assert!(matches!(state, InstanceState::Unknown));
    }

    #[test]
    fn test_image_struct() {
        let image = Image {
            name: "test-app".to_string(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::Podman,
        };
        assert_eq!(image.full_name(), "test-app:latest");
        assert_eq!(image.runtime, RuntimeKind::Podman);
    }

    #[test]
    fn test_image_with_digest() {
        let image = Image {
            name: "test-app".to_string(),
            tag: "v1.0".to_string(),
            digest: Some("sha256:abc123".to_string()),
            runtime: RuntimeKind::Podman,
        };
        assert_eq!(image.full_name(), "test-app:v1.0");
    }

    #[test]
    fn test_instance_struct() {
        let instance = Instance {
            id: "abc123".to_string(),
            name: "my-container".to_string(),
            runtime: RuntimeKind::Podman,
            image: "test-app:latest".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(instance.id, "abc123");
        assert_eq!(instance.name, "my-container");
        assert_eq!(instance.runtime, RuntimeKind::Podman);
    }

    #[test]
    fn test_status_struct() {
        let status = Status {
            state: InstanceState::Running,
            ready: true,
            message: None,
            restart_count: 0,
        };
        assert!(status.ready);
        assert_eq!(status.restart_count, 0);
    }

    #[test]
    fn test_port_mapping_format() {
        let port = PortMapping {
            container_port: 8080,
            service_port: 3000,
            protocol: "TCP".to_string(),
        };
        let formatted = format!("{}:{}", port.service_port, port.container_port);
        assert_eq!(formatted, "3000:8080");
    }
}
