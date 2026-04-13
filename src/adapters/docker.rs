//! Docker container runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::Workload;
use async_trait::async_trait;
use tokio::process::Command;

/// Default timeout for docker commands (10 minutes).
const DOCKER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(600);

/// Execute a pre-built docker [`Command`], returning its output on success.
/// On failure, bails with a message that includes the sub-command name and stderr.
/// Commands are subject to a 10-minute timeout to prevent indefinite hangs.
async fn exec_docker(mut cmd: Command, subcmd: &str) -> crate::Result<std::process::Output> {
    let output = tokio::time::timeout(DOCKER_TIMEOUT, cmd.output())
        .await
        .map_err(|_| anyhow::anyhow!("Docker {} timed out after {:?}", subcmd, DOCKER_TIMEOUT))??;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Docker {} failed: {}", subcmd, stderr);
    }
    Ok(output)
}

/// Docker runtime implementation
pub struct DockerRuntime;

impl DockerRuntime {
    pub fn new() -> anyhow::Result<Self> {
        // Check if docker is available
        which::which("docker").map_err(|_| {
            anyhow::anyhow!("Docker not found. Please install docker first.")
        })?;

        Ok(Self)
    }
}

impl Default for DockerRuntime {
    fn default() -> Self {
        match Self::new() {
            Ok(rt) => rt,
            Err(e) => {
                tracing::warn!("Docker not available ({}), creating stub runtime", e);
                Self
            }
        }
    }
}

#[async_trait]
impl Runtime for DockerRuntime {
    async fn build(&self, spec: &Workload) -> crate::Result<Image> {
        let image_name = spec.image_name();

        tracing::info!("Building image with Docker: {}", image_name);

        let mut cmd = Command::new("docker");
        cmd.args(["build", "-t", &image_name])
            .arg("-f")
            .arg(&spec.build.dockerfile)
            .arg(&spec.build.context);

        for (key, value) in &spec.build.build_args {
            cmd.arg("--build-arg").arg(format!("{}={}", key, value));
        }

        exec_docker(cmd, "build").await?;

        Ok(Image {
            name: spec.metadata.name.clone(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::Docker,
        })
    }

    async fn run(&self, image: &Image, spec: &Workload) -> crate::Result<Instance> {
        tracing::info!("Running container: {}", image.full_name());

        let mut cmd = Command::new("docker");
        cmd.args(["run", "-d", "--name", &spec.metadata.name]);

        // Label containers for aether management (used by list filtering)
        cmd.arg("--label").arg("aether-managed=true");
        cmd.arg("--label").arg(format!("app={}", spec.metadata.name));

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

        // Health probes (docker native health checks)
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
                        command.iter()
                            .map(|arg| format!("'{}'", arg.replace('\'', "'\\''")))
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

        let output = exec_docker(cmd, "run").await?;
        let container_id = String::from_utf8_lossy(&output.stdout).trim().to_string();

        Ok(Instance::new(container_id, spec.metadata.name.clone(), RuntimeKind::Docker, image.full_name()))
    }

    async fn stop(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Stopping container: {}", instance.name);

        let mut cmd = Command::new("docker");
        cmd.args(["stop", &instance.id]);
        exec_docker(cmd, "stop").await?;

        Ok(())
    }

    async fn status(&self, instance: &Instance) -> crate::Result<Status> {
        let output = Command::new("docker")
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
        let mut cmd = Command::new("docker");
        cmd.args(["logs", &instance.id]);
        if follow {
            cmd.arg("--follow");
        }

        let output = exec_docker(cmd, "logs").await?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    async fn delete(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Deleting container: {}", instance.name);

        let mut cmd = Command::new("docker");
        cmd.args(["rm", "-f", &instance.id]);
        exec_docker(cmd, "rm").await?;

        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        let mut cmd = Command::new("docker");
        cmd.args(["ps", "-a", "--filter", "label=aether-managed=true", "--format", "json"]);
        let output = exec_docker(cmd, "ps").await?;

        let json_str = String::from_utf8_lossy(&output.stdout);

        // Docker outputs one JSON object per line (JSONL), not a JSON array
        let mut instances = Vec::new();
        for line in json_str.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(c) = serde_json::from_str::<serde_json::Value>(line) {
                // Docker uses "ID" (uppercase) and "Names" is a plain string
                let id = c["ID"].as_str().unwrap_or("").to_string();
                let name = c["Names"].as_str().unwrap_or("").to_string();

                // Skip containers with missing critical fields
                if id.is_empty() || name.is_empty() {
                    tracing::warn!(
                        "Skipping container with missing id or name (id='{}', name='{}')",
                        id, name
                    );
                    continue;
                }

                instances.push(Instance {
                    id,
                    name,
                    runtime: RuntimeKind::Docker,
                    image: c["Image"].as_str().unwrap_or("unknown").to_string(),
                    created_at: c["CreatedAt"].as_str().unwrap_or("").to_string(),
                });
            }
        }

        Ok(instances)
    }

    async fn capacity(&self) -> crate::Result<Option<crate::runtime::Capacity>> {
        let mut cmd = Command::new("docker");
        cmd.args(["info", "--format", "json"]);
        match exec_docker(cmd, "info").await {
            Ok(output) => {
                let json_str = String::from_utf8_lossy(&output.stdout);
                if let Ok(info) = serde_json::from_str::<serde_json::Value>(&json_str) {
                    let cpus = info["NCPU"].as_f64().unwrap_or(0.0);
                    let mem_total = info["MemTotal"].as_u64().unwrap_or(0) / (1024 * 1024);
                    Ok(Some(crate::runtime::Capacity {
                        total_cpu: cpus,
                        available_cpu: cpus,
                        total_memory_mb: mem_total,
                        available_memory_mb: mem_total, // Docker doesn't report free memory
                    }))
                } else {
                    Ok(None)
                }
            }
            Err(_) => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_struct() {
        let image = Image {
            name: "test-app".to_string(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::Docker,
        };
        assert_eq!(image.full_name(), "test-app:latest");
        assert_eq!(image.runtime, RuntimeKind::Docker);
    }

    #[test]
    fn test_instance_struct() {
        let instance = Instance {
            id: "abc123".to_string(),
            name: "my-container".to_string(),
            runtime: RuntimeKind::Docker,
            image: "test-app:latest".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        };
        assert_eq!(instance.id, "abc123");
        assert_eq!(instance.name, "my-container");
        assert_eq!(instance.runtime, RuntimeKind::Docker);
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
}
