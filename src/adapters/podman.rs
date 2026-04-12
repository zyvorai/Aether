//! Podman container runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::Workload;
use async_trait::async_trait;
use tokio::process::Command;

/// Default timeout for podman commands (10 minutes).
const PODMAN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// Execute a pre-built podman [`Command`], returning its output on success.
/// On failure, bails with a message that includes the sub-command name and stderr.
/// Commands are subject to a 10-minute timeout to prevent indefinite hangs.
async fn exec_podman(mut cmd: Command, subcmd: &str) -> crate::Result<std::process::Output> {
    let output = tokio::time::timeout(PODMAN_TIMEOUT, cmd.output())
        .await
        .map_err(|_| anyhow::anyhow!("Podman {} timed out after {:?}", subcmd, PODMAN_TIMEOUT))??;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Podman {} failed: {}", subcmd, stderr);
    }
    Ok(output)
}

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
            Err(e) => {
                tracing::warn!("Podman not available ({}), creating stub runtime", e);
                Self
            }
        }
    }
}

#[async_trait]
impl Runtime for PodmanRuntime {
    async fn build(&self, spec: &Workload) -> crate::Result<Image> {
        let image_name = spec.image_name();

        tracing::info!("Building image with Podman: {}", image_name);

        let mut cmd = Command::new("podman");
        cmd.args(["build", "-t", &image_name])
            .arg("-f")
            .arg(&spec.build.dockerfile)
            .arg(&spec.build.context);

        for (key, value) in &spec.build.build_args {
            cmd.arg("--build-arg").arg(format!("{}={}", key, value));
        }

        exec_podman(cmd, "build").await?;

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

        for port in &spec.network.ports {
            cmd.arg("-p").arg(format!(
                "{}:{}",
                port.service_port, port.container_port
            ));
        }

        // Inject environment variables from config maps
        if let Some(config) = &spec.config {
            for cm in &config.config_maps {
                for (k, v) in &cm.data {
                    cmd.arg("-e").arg(format!("{}={}", k, v));
                }
            }
        }

        // Health probes (podman native health checks)
        if let Some(health) = &spec.health {
            if let Some(liveness) = &health.liveness {
                let health_cmd = match &liveness.probe_type {
                    crate::spec::ProbeType::HttpGet { path, port } => {
                        format!("curl -sf http://localhost:{}{} || exit 1", port, path)
                    }
                    crate::spec::ProbeType::TcpSocket { port } => {
                        format!("bash -c '</dev/tcp/localhost/{}' || exit 1", port)
                    }
                    crate::spec::ProbeType::Exec { command } => {
                        // Shell-escape each argument to prevent command injection
                        command.iter()
                            .map(|arg| {
                                if arg.contains(|c: char| c.is_whitespace() || c == '\'' || c == '"' || c == '\\' || c == '$' || c == '`' || c == '!' || c == ';' || c == '&' || c == '|') {
                                    format!("'{}'", arg.replace('\'', "'\\''"))
                                } else {
                                    arg.clone()
                                }
                            })
                            .collect::<Vec<_>>()
                            .join(" ")
                    }
                };
                cmd.arg("--health-cmd").arg(&health_cmd);
                cmd.arg("--health-interval")
                    .arg(format!("{}s", liveness.period_seconds));
                cmd.arg("--health-start-period")
                    .arg(format!("{}s", liveness.initial_delay_seconds));
            }
        }

        // Restart policy for resilience
        cmd.arg("--restart").arg("on-failure:3");

        cmd.arg("--cpus").arg(&spec.requirements.cpu);
        cmd.arg("--memory").arg(&spec.requirements.memory);
        cmd.arg(image.full_name());

        let output = exec_podman(cmd, "run").await?;
        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(Instance::new(container_id, spec.metadata.name.clone(), RuntimeKind::Podman, image.full_name()))
    }

    async fn stop(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Stopping container: {}", instance.name);

        let mut cmd = Command::new("podman");
        cmd.args(["stop", &instance.id]);
        exec_podman(cmd, "stop").await?;

        Ok(())
    }

    async fn status(&self, instance: &Instance) -> crate::Result<Status> {
        let output = Command::new("podman")
            .args([
                "inspect", "--format",
                "{{.State.Status}}|{{if .State.Health}}{{.State.Health.Status}}{{else}}none{{end}}|{{.RestartCount}}",
                &instance.id,
            ])
            .output().await?;

        if !output.status.success() {
            return Ok(Status {
                state: InstanceState::Unknown,
                ready: false,
                message: Some("Container not found".to_string()),
                restart_count: 0,
            });
        }

        let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let parts: Vec<&str> = raw.splitn(3, '|').collect();
        let status_str = parts.first().unwrap_or(&"unknown");
        let health_str = parts.get(1).unwrap_or(&"none");
        let restart_str = parts.get(2).unwrap_or(&"0");

        let state = match *status_str {
            "running" => InstanceState::Running,
            "exited" => InstanceState::Stopped,
            "created" => InstanceState::Pending,
            _ => InstanceState::Unknown,
        };

        let restart_count = restart_str.parse::<u32>().unwrap_or(0);

        // Health-aware readiness: use health status when available
        let ready = match *health_str {
            "healthy" => true,
            "unhealthy" => false,
            "starting" => false,
            "none" => matches!(state, InstanceState::Running),
            _ => matches!(state, InstanceState::Running),
        };

        let message = match *health_str {
            "unhealthy" => Some("Health check failing".to_string()),
            "starting" => Some("Health check starting".to_string()),
            _ => None,
        };

        Ok(Status {
            state,
            ready,
            message,
            restart_count,
        })
    }

    async fn logs(&self, instance: &Instance, follow: bool) -> crate::Result<String> {
        let mut cmd = Command::new("podman");
        cmd.args(["logs", &instance.id]);
        if follow {
            cmd.arg("--follow");
        }

        let output = exec_podman(cmd, "logs").await?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn delete(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Deleting container: {}", instance.name);

        let mut cmd = Command::new("podman");
        cmd.args(["rm", "-f", &instance.id]);
        exec_podman(cmd, "rm").await?;

        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        let mut cmd = Command::new("podman");
        cmd.args(["ps", "-a", "--format", "json"]);
        let output = exec_podman(cmd, "ps").await?;

        let json_str = String::from_utf8_lossy(&output.stdout);
        let containers: Vec<serde_json::Value> = serde_json::from_str(&json_str)?;

        let instances: Vec<Instance> = containers
            .iter()
            .filter_map(|c| {
                let id = c["Id"].as_str().unwrap_or("").to_string();
                let name = c["Names"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // Skip containers with missing critical fields
                if id.is_empty() || name.is_empty() {
                    tracing::warn!(
                        "Skipping container with missing id or name (id='{}', name='{}')",
                        id, name
                    );
                    return None;
                }

                Some(Instance {
                    id,
                    name,
                    runtime: RuntimeKind::Podman,
                    image: c["Image"].as_str().unwrap_or("unknown").to_string(),
                    created_at: c["CreatedAt"].as_str().unwrap_or("").to_string(),
                })
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
            api_version: "aether/v1".to_string(),
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
