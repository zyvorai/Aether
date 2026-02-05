//! Podman container runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::Workload;
use async_trait::async_trait;
use std::process::Command;

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
        Self::new().expect("Failed to initialize Podman runtime")
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

        let output = cmd.output()?;

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

        let output = cmd.output()?;

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
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman stop failed: {}", stderr);
        }

        Ok(())
    }

    async fn status(&self, instance: &Instance) -> crate::Result<Status> {
        let output = Command::new("podman")
            .args(["inspect", "--format", "{{.State.Status}}", &instance.id])
            .output()?;

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

        let output = cmd.output()?;

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
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Podman rm failed: {}", stderr);
        }

        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        let output = Command::new("podman")
            .args(["ps", "-a", "--format", "json"])
            .output()?;

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
