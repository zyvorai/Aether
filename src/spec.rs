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
    #[serde(default)]
    pub config: Option<ConfigSpec>,
    #[serde(default)]
    pub ingress: Option<IngressSpec>,
    #[serde(default)]
    pub scaling: Option<ScalingSpec>,
    #[serde(default)]
    pub mesh: Option<MeshConfig>,
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
    #[serde(default)]
    pub network_policy: Option<NetworkPolicyConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPolicyConfig {
    /// Allow ingress from specific labels
    #[serde(default)]
    pub allow_from: Vec<String>,
    /// Allow egress to specific labels
    #[serde(default)]
    pub allow_to: Vec<String>,
    /// Deny all ingress by default
    #[serde(default)]
    pub deny_all_ingress: bool,
    /// Deny all egress by default
    #[serde(default)]
    pub deny_all_egress: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ServiceType {
    #[default]
    ClusterIP,
    NodePort,
    LoadBalancer,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum AccessMode {
    #[default]
    ReadWriteOnce,
    ReadOnlyMany,
    ReadWriteMany,
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
        if self.api_version != "aether/v1" {
            anyhow::bail!("Unsupported apiVersion: {}", self.api_version);
        }

        // Check kind
        if self.kind != "Workload" {
            anyhow::bail!("Unsupported kind: {}", self.kind);
        }

        // Validate name is a valid DNS label (RFC 1123)
        Self::validate_dns_label(&self.metadata.name, "metadata.name")?;

        // Validate resource requirements
        Self::validate_cpu(&self.requirements.cpu)?;
        Self::validate_memory(&self.requirements.memory)?;
        Self::validate_storage(&self.requirements.storage)?;

        // Validate GPU if present
        if let Some(ref gpu) = self.requirements.gpu {
            if gpu.count == 0 {
                anyhow::bail!("requirements.gpu.count must be > 0");
            }
            if gpu.vendor.is_empty() {
                anyhow::bail!("requirements.gpu.vendor cannot be empty");
            }
        }

        // Validate network ports
        for (i, port) in self.network.ports.iter().enumerate() {
            if port.container_port == 0 {
                anyhow::bail!("network.ports[{}].containerPort must be > 0", i);
            }
            if port.service_port == 0 {
                anyhow::bail!("network.ports[{}].servicePort must be > 0", i);
            }
        }

        // Validate scaling spec
        if let Some(ref scaling) = self.scaling {
            if scaling.min_replicas == 0 {
                anyhow::bail!("scaling.minReplicas must be > 0");
            }
            if scaling.max_replicas == 0 {
                anyhow::bail!("scaling.maxReplicas must be > 0");
            }
            if scaling.min_replicas > scaling.max_replicas {
                anyhow::bail!(
                    "scaling.minReplicas ({}) must be <= scaling.maxReplicas ({})",
                    scaling.min_replicas,
                    scaling.max_replicas
                );
            }
        }

        // Validate ingress host
        if let Some(ref ingress) = self.ingress {
            if ingress.enabled && ingress.host.is_empty() {
                anyhow::bail!("ingress.host cannot be empty when ingress is enabled");
            }
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

    /// Validate a string as a DNS label (RFC 1123):
    /// - 1-63 characters
    /// - lowercase alphanumeric and hyphens only
    /// - must start and end with alphanumeric
    fn validate_dns_label(name: &str, field: &str) -> anyhow::Result<()> {
        if name.is_empty() {
            anyhow::bail!("{} cannot be empty", field);
        }
        if name.len() > 63 {
            anyhow::bail!("{} must be at most 63 characters, got {}", field, name.len());
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            anyhow::bail!(
                "{} must contain only lowercase alphanumeric characters or '-', got '{}'",
                field,
                name
            );
        }
        if name.starts_with('-') || name.ends_with('-') {
            anyhow::bail!(
                "{} must start and end with an alphanumeric character, got '{}'",
                field,
                name
            );
        }
        Ok(())
    }

    /// Validate CPU resource string (e.g., "2", "500m", "0.5")
    fn validate_cpu(cpu: &str) -> anyhow::Result<()> {
        if cpu.is_empty() {
            anyhow::bail!("requirements.cpu cannot be empty");
        }
        if let Some(milli) = cpu.strip_suffix('m') {
            let val = milli
                .parse::<u64>()
                .map_err(|_| anyhow::anyhow!("requirements.cpu: invalid millicore value '{}'", cpu))?;
            if val == 0 {
                anyhow::bail!("requirements.cpu must be > 0");
            }
        } else {
            let val = cpu
                .parse::<f64>()
                .map_err(|_| anyhow::anyhow!("requirements.cpu: invalid value '{}', expected number or millicore (e.g., '2' or '500m')", cpu))?;
            if val <= 0.0 {
                anyhow::bail!("requirements.cpu must be > 0");
            }
        }
        Ok(())
    }

    /// Validate memory resource string (e.g., "4Gi", "512Mi")
    fn validate_memory(memory: &str) -> anyhow::Result<()> {
        Self::validate_byte_quantity(memory, "requirements.memory")
    }

    /// Validate storage resource string (e.g., "20Gi", "100Mi")
    fn validate_storage(storage: &str) -> anyhow::Result<()> {
        Self::validate_byte_quantity(storage, "requirements.storage")
    }

    /// Validate a Kubernetes-style byte quantity (e.g., "4Gi", "512Mi", "100G", "256M", "1.5Gi")
    fn validate_byte_quantity(value: &str, field: &str) -> anyhow::Result<()> {
        if value.is_empty() {
            anyhow::bail!("{} cannot be empty", field);
        }
        let suffixes = ["Gi", "Mi", "Ti", "Ki", "G", "M", "T", "K"];
        for suffix in &suffixes {
            if let Some(num) = value.strip_suffix(suffix) {
                let val = num.parse::<f64>().map_err(|_| {
                    anyhow::anyhow!("{}: invalid numeric value in '{}'", field, value)
                })?;
                if val <= 0.0 {
                    anyhow::bail!("{} must be > 0", field);
                }
                return Ok(());
            }
        }
        anyhow::bail!(
            "{}: invalid format '{}', expected value with suffix (e.g., '4Gi', '512Mi', '1.5Gi')",
            field,
            value
        );
    }

    /// Get full image name with registry
    pub fn image_name(&self) -> String {
        format!("{}/{}:latest", self.build.registry, self.metadata.name)
    }

    /// Return a copy with additional environment variables injected.
    ///
    /// Creates a synthetic "compose-env" ConfigMap in the config section,
    /// which adapters already know how to process as environment variables.
    pub fn with_env(mut self, env: &std::collections::HashMap<String, String>) -> Self {
        if env.is_empty() {
            return self;
        }
        let config = self.config.get_or_insert_with(|| ConfigSpec {
            config_maps: vec![],
            secrets: vec![],
            env_from: vec![],
        });
        if let Some(cm) = config.config_maps.iter_mut().find(|c| c.name == "compose-env") {
            cm.data.extend(env.clone());
        } else {
            config.config_maps.push(ConfigMapSpec {
                name: "compose-env".to_string(),
                data: env.clone(),
                mount_path: None,
            });
        }
        self
    }
}

/// Configuration specification (ConfigMaps and Secrets)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigSpec {
    #[serde(default)]
    pub config_maps: Vec<ConfigMapSpec>,
    #[serde(default)]
    pub secrets: Vec<SecretSpec>,
    #[serde(default)]
    pub env_from: Vec<EnvFromSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConfigMapSpec {
    pub name: String,
    pub data: HashMap<String, String>,
    #[serde(default)]
    pub mount_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SecretSpec {
    pub name: String,
    pub data: HashMap<String, String>,
    #[serde(default)]
    pub mount_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct EnvFromSource {
    pub source_type: EnvSourceType,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EnvSourceType {
    ConfigMap,
    Secret,
}

/// Ingress specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngressSpec {
    pub enabled: bool,
    pub host: String,
    #[serde(default)]
    pub paths: Vec<IngressPath>,
    #[serde(default)]
    pub tls: bool,
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngressPath {
    pub path: String,
    #[serde(default = "default_path_type")]
    pub path_type: String,
    pub port: u16,
}

fn default_path_type() -> String {
    "Prefix".to_string()
}

/// Scaling specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScalingSpec {
    pub enabled: bool,
    pub min_replicas: u32,
    pub max_replicas: u32,
    #[serde(default)]
    pub metrics: Vec<ScalingMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScalingMetric {
    pub metric_type: MetricType,
    pub target_value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MetricType {
    CPU,
    Memory,
    Custom,
}

/// Service mesh sidecar injection configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MeshConfig {
    /// Service mesh provider: istio, linkerd, consul
    pub provider: String,
    /// Enable sidecar injection
    #[serde(default = "default_true")]
    pub inject: bool,
    /// Additional mesh-specific annotations
    #[serde(default)]
    pub annotations: std::collections::HashMap<String, String>,
}

fn default_true() -> bool { true }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workload_validation() {
        let workload = Workload {
            api_version: "aether/v1".to_string(),
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
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
        };

        assert!(workload.validate().is_ok());
    }

    #[test]
    fn test_image_name() {
        let workload = Workload {
            api_version: "aether/v1".to_string(),
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
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
        };

        assert_eq!(workload.image_name(), "ghcr.io/yourorg/my-app:latest");
    }

    #[test]
    fn test_invalid_api_version() {
        let workload = Workload {
            api_version: "aether/v2".to_string(),
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
                allow: vec![RuntimeType::Container],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
        };

        let result = workload.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("apiVersion"));
    }

    #[test]
    fn test_invalid_kind() {
        let workload = Workload {
            api_version: "aether/v1".to_string(),
            kind: "Service".to_string(),
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
                allow: vec![RuntimeType::Container],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
        };

        let result = workload.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("kind"));
    }

    #[test]
    fn test_empty_name() {
        let workload = Workload {
            api_version: "aether/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "".to_string(),
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
                allow: vec![RuntimeType::Container],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
        };

        let result = workload.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("name"));
    }

    #[test]
    fn test_preferred_not_in_allow() {
        let workload = Workload {
            api_version: "aether/v1".to_string(),
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
                preferred: RuntimePreference::Kube,
                allow: vec![RuntimeType::Container],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
        };

        let result = workload.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not in allowed"));
    }

    // ---------------------------------------------------------------
    // Helper for new validation tests
    // ---------------------------------------------------------------

    fn make_valid_workload() -> Workload {
        Workload {
            api_version: "aether/v1".to_string(),
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
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
        }
    }

    // ---------------------------------------------------------------
    // DNS label validation
    // ---------------------------------------------------------------

    #[test]
    fn test_name_valid_dns_labels() {
        for name in &["a", "abc", "my-app", "app-123", "a1b2c3"] {
            let mut w = make_valid_workload();
            w.metadata.name = name.to_string();
            assert!(w.validate().is_ok(), "expected '{}' to be valid", name);
        }
    }

    #[test]
    fn test_name_uppercase_rejected() {
        let mut w = make_valid_workload();
        w.metadata.name = "MyApp".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("lowercase"), "{}", err);
    }

    #[test]
    fn test_name_underscore_rejected() {
        let mut w = make_valid_workload();
        w.metadata.name = "my_app".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("lowercase"), "{}", err);
    }

    #[test]
    fn test_name_starts_with_hyphen_rejected() {
        let mut w = make_valid_workload();
        w.metadata.name = "-app".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("start and end"), "{}", err);
    }

    #[test]
    fn test_name_ends_with_hyphen_rejected() {
        let mut w = make_valid_workload();
        w.metadata.name = "app-".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("start and end"), "{}", err);
    }

    #[test]
    fn test_name_too_long_rejected() {
        let mut w = make_valid_workload();
        w.metadata.name = "a".repeat(64);
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("63"), "{}", err);
    }

    #[test]
    fn test_name_exactly_63_chars_valid() {
        let mut w = make_valid_workload();
        w.metadata.name = "a".repeat(63);
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_name_with_space_rejected() {
        let mut w = make_valid_workload();
        w.metadata.name = "my app".to_string();
        assert!(w.validate().is_err());
    }

    #[test]
    fn test_name_with_dot_rejected() {
        let mut w = make_valid_workload();
        w.metadata.name = "my.app".to_string();
        assert!(w.validate().is_err());
    }

    // ---------------------------------------------------------------
    // CPU validation
    // ---------------------------------------------------------------

    #[test]
    fn test_cpu_valid_whole_number() {
        let mut w = make_valid_workload();
        w.requirements.cpu = "4".to_string();
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_cpu_valid_millicore() {
        let mut w = make_valid_workload();
        w.requirements.cpu = "500m".to_string();
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_cpu_valid_fractional() {
        let mut w = make_valid_workload();
        w.requirements.cpu = "0.5".to_string();
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_cpu_empty_rejected() {
        let mut w = make_valid_workload();
        w.requirements.cpu = "".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("cpu"), "{}", err);
    }

    #[test]
    fn test_cpu_zero_rejected() {
        let mut w = make_valid_workload();
        w.requirements.cpu = "0".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("cpu"), "{}", err);
    }

    #[test]
    fn test_cpu_zero_millicore_rejected() {
        let mut w = make_valid_workload();
        w.requirements.cpu = "0m".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("cpu"), "{}", err);
    }

    #[test]
    fn test_cpu_negative_rejected() {
        let mut w = make_valid_workload();
        w.requirements.cpu = "-1".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("cpu"), "{}", err);
    }

    #[test]
    fn test_cpu_garbage_rejected() {
        let mut w = make_valid_workload();
        w.requirements.cpu = "abc".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("cpu"), "{}", err);
    }

    // ---------------------------------------------------------------
    // Memory validation
    // ---------------------------------------------------------------

    #[test]
    fn test_memory_valid_gi() {
        let mut w = make_valid_workload();
        w.requirements.memory = "8Gi".to_string();
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_memory_valid_mi() {
        let mut w = make_valid_workload();
        w.requirements.memory = "512Mi".to_string();
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_memory_empty_rejected() {
        let mut w = make_valid_workload();
        w.requirements.memory = "".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("memory"), "{}", err);
    }

    #[test]
    fn test_memory_no_suffix_rejected() {
        let mut w = make_valid_workload();
        w.requirements.memory = "4096".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("memory"), "{}", err);
    }

    #[test]
    fn test_memory_zero_rejected() {
        let mut w = make_valid_workload();
        w.requirements.memory = "0Gi".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("memory"), "{}", err);
    }

    #[test]
    fn test_memory_invalid_number_rejected() {
        let mut w = make_valid_workload();
        w.requirements.memory = "abcGi".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("memory"), "{}", err);
    }

    // ---------------------------------------------------------------
    // Storage validation
    // ---------------------------------------------------------------

    #[test]
    fn test_storage_valid_gi() {
        let mut w = make_valid_workload();
        w.requirements.storage = "100Gi".to_string();
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_storage_empty_rejected() {
        let mut w = make_valid_workload();
        w.requirements.storage = "".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("storage"), "{}", err);
    }

    #[test]
    fn test_storage_no_suffix_rejected() {
        let mut w = make_valid_workload();
        w.requirements.storage = "500".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("storage"), "{}", err);
    }

    #[test]
    fn test_storage_zero_rejected() {
        let mut w = make_valid_workload();
        w.requirements.storage = "0Gi".to_string();
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("storage"), "{}", err);
    }

    // ---------------------------------------------------------------
    // GPU validation
    // ---------------------------------------------------------------

    #[test]
    fn test_gpu_valid() {
        let mut w = make_valid_workload();
        w.requirements.gpu = Some(GpuRequirements {
            count: 2,
            vendor: "nvidia".to_string(),
        });
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_gpu_zero_count_rejected() {
        let mut w = make_valid_workload();
        w.requirements.gpu = Some(GpuRequirements {
            count: 0,
            vendor: "nvidia".to_string(),
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("gpu.count"), "{}", err);
    }

    #[test]
    fn test_gpu_empty_vendor_rejected() {
        let mut w = make_valid_workload();
        w.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "".to_string(),
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("gpu.vendor"), "{}", err);
    }

    // ---------------------------------------------------------------
    // Port validation
    // ---------------------------------------------------------------

    #[test]
    fn test_ports_valid() {
        let mut w = make_valid_workload();
        w.network.ports = vec![PortMapping {
            container_port: 8080,
            service_port: 80,
            protocol: "TCP".to_string(),
        }];
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_port_zero_container_port_rejected() {
        let mut w = make_valid_workload();
        w.network.ports = vec![PortMapping {
            container_port: 0,
            service_port: 80,
            protocol: "TCP".to_string(),
        }];
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("containerPort"), "{}", err);
    }

    #[test]
    fn test_port_zero_service_port_rejected() {
        let mut w = make_valid_workload();
        w.network.ports = vec![PortMapping {
            container_port: 8080,
            service_port: 0,
            protocol: "TCP".to_string(),
        }];
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("servicePort"), "{}", err);
    }

    // ---------------------------------------------------------------
    // Scaling validation
    // ---------------------------------------------------------------

    #[test]
    fn test_scaling_valid() {
        let mut w = make_valid_workload();
        w.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 2,
            max_replicas: 10,
            metrics: vec![],
        });
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_scaling_min_zero_rejected() {
        let mut w = make_valid_workload();
        w.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 0,
            max_replicas: 5,
            metrics: vec![],
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("minReplicas"), "{}", err);
    }

    #[test]
    fn test_scaling_max_zero_rejected() {
        let mut w = make_valid_workload();
        w.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 0,
            metrics: vec![],
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("maxReplicas"), "{}", err);
    }

    #[test]
    fn test_scaling_min_greater_than_max_rejected() {
        let mut w = make_valid_workload();
        w.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 10,
            max_replicas: 5,
            metrics: vec![],
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("minReplicas"), "{}", err);
        assert!(err.contains("maxReplicas"), "{}", err);
    }

    #[test]
    fn test_scaling_min_equals_max_valid() {
        let mut w = make_valid_workload();
        w.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 3,
            max_replicas: 3,
            metrics: vec![],
        });
        assert!(w.validate().is_ok());
    }

    // ---------------------------------------------------------------
    // Ingress validation
    // ---------------------------------------------------------------

    #[test]
    fn test_ingress_enabled_with_host_valid() {
        let mut w = make_valid_workload();
        w.ingress = Some(IngressSpec {
            enabled: true,
            host: "app.example.com".to_string(),
            paths: vec![],
            tls: false,
            annotations: HashMap::new(),
        });
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_ingress_enabled_empty_host_rejected() {
        let mut w = make_valid_workload();
        w.ingress = Some(IngressSpec {
            enabled: true,
            host: "".to_string(),
            paths: vec![],
            tls: false,
            annotations: HashMap::new(),
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("ingress.host"), "{}", err);
    }

    #[test]
    fn test_ingress_disabled_empty_host_allowed() {
        let mut w = make_valid_workload();
        w.ingress = Some(IngressSpec {
            enabled: false,
            host: "".to_string(),
            paths: vec![],
            tls: false,
            annotations: HashMap::new(),
        });
        assert!(w.validate().is_ok());
    }

    // ---------------------------------------------------------------
    // validate_dns_label unit tests
    // ---------------------------------------------------------------

    #[test]
    fn test_dns_label_single_char() {
        assert!(Workload::validate_dns_label("a", "test").is_ok());
    }

    #[test]
    fn test_dns_label_digit_only() {
        assert!(Workload::validate_dns_label("123", "test").is_ok());
    }

    #[test]
    fn test_dns_label_hyphen_in_middle() {
        assert!(Workload::validate_dns_label("a-b", "test").is_ok());
    }

    #[test]
    fn test_validate_byte_quantity_ti_suffix() {
        assert!(Workload::validate_byte_quantity("1Ti", "test").is_ok());
    }

    #[test]
    fn test_validate_byte_quantity_g_suffix() {
        assert!(Workload::validate_byte_quantity("100G", "test").is_ok());
    }

    #[test]
    fn test_validate_byte_quantity_m_suffix() {
        assert!(Workload::validate_byte_quantity("512M", "test").is_ok());
    }

    #[test]
    fn test_validate_byte_quantity_t_suffix() {
        assert!(Workload::validate_byte_quantity("2T", "test").is_ok());
    }
}
