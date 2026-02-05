//! Workload specification schema
//!
//! This module defines the universal workload spec that can be deployed
//! to any runtime (Container, Kubernetes, KubeVirt, Metal3).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Universal workload specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Workload {
    pub api_version: String,
    pub kind: String,
    pub metadata: Metadata,
    pub build: BuildSpec,
    pub requirements: ResourceRequirements,
    pub runtime: RuntimeSpec,
    #[serde(default)]
    pub network: NetworkSpec,
    #[serde(default)]
    pub persistence: PersistenceSpec,
    #[serde(default)]
    pub health: Option<HealthSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Metadata {
    pub name: String,
    pub owner: String,
    pub project: String,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BuildSpec {
    pub context: PathBuf,
    pub dockerfile: PathBuf,
    pub registry: String,
    #[serde(default)]
    pub build_args: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceRequirements {
    pub cpu: String,       // e.g., "2" or "2000m"
    pub memory: String,    // e.g., "4Gi"
    pub storage: String,   // e.g., "20Gi"
    #[serde(default)]
    pub gpu: Option<GpuRequirements>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpuRequirements {
    pub count: u32,
    pub vendor: String, // "nvidia", "amd", "intel"
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeSpec {
    pub preferred: RuntimePreference,
    pub allow: Vec<RuntimeType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuntimePreference {
    Auto,
    Container,
    Kube,
    Kubevirt,
    Metal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeType {
    Container,
    Kube,
    Kubevirt,
    Metal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct NetworkSpec {
    #[serde(default)]
    pub service: bool,
    #[serde(default)]
    pub service_type: ServiceType,
    #[serde(default)]
    pub ports: Vec<PortMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ServiceType {
    ClusterIP,
    NodePort,
    LoadBalancer,
}

impl Default for ServiceType {
    fn default() -> Self {
        ServiceType::ClusterIP
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PortMapping {
    pub container_port: u16,
    pub service_port: u16,
    #[serde(default = "default_protocol")]
    pub protocol: String,
}

fn default_protocol() -> String {
    "TCP".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct PersistenceSpec {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub size: String,
    #[serde(default)]
    pub access_mode: AccessMode,
    #[serde(default)]
    pub storage_class: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AccessMode {
    ReadWriteOnce,
    ReadOnlyMany,
    ReadWriteMany,
}

impl Default for AccessMode {
    fn default() -> Self {
        AccessMode::ReadWriteOnce
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HealthSpec {
    #[serde(default)]
    pub liveness: Option<HealthProbe>,
    #[serde(default)]
    pub readiness: Option<HealthProbe>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HealthProbe {
    #[serde(flatten)]
    pub probe_type: ProbeType,
    #[serde(default = "default_initial_delay")]
    pub initial_delay_seconds: u32,
    #[serde(default = "default_period")]
    pub period_seconds: u32,
}

fn default_initial_delay() -> u32 {
    10
}

fn default_period() -> u32 {
    10
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum ProbeType {
    HttpGet { path: String, port: u16 },
    TcpSocket { port: u16 },
    Exec { command: Vec<String> },
}

impl Workload {
    /// Load workload from YAML file
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let workload: Workload = serde_yaml::from_str(&content)?;
        workload.validate()?;
        Ok(workload)
    }

    /// Validate workload specification
    pub fn validate(&self) -> anyhow::Result<()> {
        // Check API version
        if self.api_version != "orchestr8/v1" {
            anyhow::bail!("Unsupported apiVersion: {}", self.api_version);
        }

        // Check kind
        if self.kind != "Workload" {
            anyhow::bail!("Unsupported kind: {}", self.kind);
        }

        // Validate name
        if self.metadata.name.is_empty() {
            anyhow::bail!("metadata.name cannot be empty");
        }

        // Validate runtime preference is in allowed list
        let preferred_in_allow = match self.runtime.preferred {
            RuntimePreference::Auto => true,
            RuntimePreference::Container => {
                self.runtime.allow.contains(&RuntimeType::Container)
            }
            RuntimePreference::Kube => self.runtime.allow.contains(&RuntimeType::Kube),
            RuntimePreference::Kubevirt => {
                self.runtime.allow.contains(&RuntimeType::Kubevirt)
            }
            RuntimePreference::Metal => self.runtime.allow.contains(&RuntimeType::Metal),
        };

        if !preferred_in_allow && self.runtime.preferred != RuntimePreference::Auto {
            anyhow::bail!(
                "Preferred runtime {:?} is not in allowed list",
                self.runtime.preferred
            );
        }

        Ok(())
    }

    /// Get full image name with registry
    pub fn image_name(&self) -> String {
        format!("{}/{}:latest", self.build.registry, self.metadata.name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_validation() {
        let workload = Workload {
            api_version: "orchestr8/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "test-app".to_string(),
                owner: "test".to_string(),
                project: "demo".to_string(),
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
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Auto,
                allow: vec![RuntimeType::Container, RuntimeType::Kube],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
        };

        assert!(workload.validate().is_ok());
    }

    #[test]
    fn test_image_name() {
        let workload = Workload {
            api_version: "orchestr8/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "my-app".to_string(),
                owner: "test".to_string(),
                project: "demo".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/yourorg".to_string(),
                build_args: HashMap::new(),
            },
            requirements: ResourceRequirements {
                cpu: "2".to_string(),
                memory: "4Gi".to_string(),
                storage: "20Gi".to_string(),
                gpu: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Auto,
                allow: vec![RuntimeType::Container],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
        };

        assert_eq!(workload.image_name(), "ghcr.io/yourorg/my-app:latest");
    }
}
