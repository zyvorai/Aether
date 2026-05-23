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
    #[serde(default)]
    pub intent: Option<IntentSpec>,
    #[serde(default)]
    pub schedule: Option<ScheduleSpec>,
    /// Kubernetes-only options (workload kind, scheduling, security, Gateway API, etc.)
    #[serde(default)]
    pub kubernetes: Option<KubernetesSpec>,
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
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    pub cpu: String,       // e.g., "2" or "2000m" (used as limit)
    pub memory: String,    // e.g., "4Gi" (used as limit)
    pub storage: String,   // e.g., "20Gi"
    #[serde(default)]
    pub gpu: Option<GpuRequirements>,
    /// CPU request (defaults to cpu limit if absent, enabling burstable QoS)
    #[serde(default)]
    pub cpu_request: Option<String>,
    /// Memory request (defaults to memory limit if absent, enabling burstable QoS)
    #[serde(default)]
    pub memory_request: Option<String>,
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
    #[serde(default)]
    pub startup: Option<HealthProbe>,
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

        // Validate optional resource requests
        if let Some(ref cpu_req) = self.requirements.cpu_request {
            Self::validate_cpu(cpu_req).map_err(|e| {
                anyhow::anyhow!("requirements.cpuRequest: {}", e)
            })?;
        }
        if let Some(ref mem_req) = self.requirements.memory_request {
            Self::validate_memory(mem_req).map_err(|e| {
                anyhow::anyhow!("requirements.memoryRequest: {}", e)
            })?;
        }

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

        // Validate intent spec
        if let Some(ref intent) = self.intent {
            intent.validate()?;
        }

        // Validate schedule spec
        if let Some(ref schedule) = self.schedule {
            Self::validate_cron(&schedule.cron)?;
            if let Some(deadline) = schedule.active_deadline_seconds {
                if deadline <= 0 {
                    anyhow::bail!("schedule.activeDeadlineSeconds must be > 0");
                }
            }
        }

        let k8s_kind = self.resolved_k8s_workload_kind();
        if k8s_kind == K8sWorkloadKind::CronJob && self.schedule.is_none() {
            anyhow::bail!("kubernetes.workloadKind cronJob requires schedule.cron");
        }
        if self.schedule.is_some() && k8s_kind == K8sWorkloadKind::Job {
            anyhow::bail!("schedule and kubernetes.workloadKind job are mutually exclusive");
        }
        if k8s_kind == K8sWorkloadKind::DaemonSet {
            if self.scaling.as_ref().is_some_and(|s| s.enabled) {
                anyhow::bail!("scaling is not supported for kubernetes.workloadKind daemonSet");
            }
        }
        if let Some(ref k8s) = self.kubernetes {
            if let Some(ref pdb) = k8s.pod_disruption_budget {
                if pdb.min_available.is_none() && pdb.max_unavailable.is_none() {
                    anyhow::bail!(
                        "kubernetes.podDisruptionBudget requires minAvailable or maxUnavailable"
                    );
                }
            }
            if let Some(ref gw) = k8s.gateway {
                if gw.enabled && gw.gateway_name.is_empty() {
                    anyhow::bail!("kubernetes.gateway.gatewayName cannot be empty");
                }
            }
            if let Some(ref keda) = k8s.keda {
                if keda.enabled && keda.max_replica_count == 0 {
                    anyhow::bail!("kubernetes.keda.maxReplicaCount must be > 0 when enabled");
                }
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

    /// Resolve the Kubernetes controller kind for this workload.
    pub fn resolved_k8s_workload_kind(&self) -> K8sWorkloadKind {
        if let Some(kind) = self
            .kubernetes
            .as_ref()
            .and_then(|k| k.workload_kind.clone())
        {
            return kind;
        }
        if self.schedule.is_some() {
            return K8sWorkloadKind::CronJob;
        }
        K8sWorkloadKind::Deployment
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

    /// Validate a cron expression (5 fields: minute hour day-of-month month day-of-week).
    ///
    /// Checks structure (5 fields) and basic range limits for numeric values.
    fn validate_cron(expr: &str) -> anyhow::Result<()> {
        if expr.is_empty() {
            anyhow::bail!("schedule.cron cannot be empty");
        }
        let fields: Vec<&str> = expr.split_whitespace().collect();
        if fields.len() != 5 {
            anyhow::bail!(
                "schedule.cron must have exactly 5 fields (minute hour dom month dow), got {}",
                fields.len()
            );
        }

        // Validate ranges: minute(0-59), hour(0-23), dom(1-31), month(1-12), dow(0-7)
        let ranges: [(u32, u32); 5] = [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)];
        let names = ["minute", "hour", "day-of-month", "month", "day-of-week"];

        for (i, (field, (min, max))) in fields.iter().zip(ranges.iter()).enumerate() {
            // Skip wildcards and complex expressions (*/N, ranges, lists)
            if field.contains('*') || field.contains('/') || field.contains(',') || field.contains('-') {
                continue;
            }
            // Validate plain numeric values
            if let Ok(val) = field.parse::<u32>() {
                if val < *min || val > *max {
                    anyhow::bail!(
                        "schedule.cron {} field value {} is out of range ({}-{})",
                        names[i], val, min, max
                    );
                }
            }
        }

        Ok(())
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

/// Schedule specification for CronJob/Job workloads
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleSpec {
    /// Cron expression (e.g., "*/5 * * * *")
    pub cron: String,
    /// How to treat concurrent executions
    #[serde(default)]
    pub concurrency_policy: ConcurrencyPolicy,
    /// Number of retries before marking as failed
    #[serde(default = "default_backoff_limit")]
    pub backoff_limit: u32,
    /// Maximum time in seconds for the job to run
    #[serde(default)]
    pub active_deadline_seconds: Option<i64>,
    /// Pod restart policy for job containers
    #[serde(default)]
    pub restart_policy: JobRestartPolicy,
}

fn default_backoff_limit() -> u32 {
    3
}

/// Concurrency policy for CronJob
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum ConcurrencyPolicy {
    /// Allow concurrent runs (default)
    #[default]
    Allow,
    /// Skip new run if previous is still active
    Forbid,
    /// Replace currently running job with new one
    Replace,
}

/// Restart policy for Job containers
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum JobRestartPolicy {
    /// Never restart (default for jobs)
    #[default]
    Never,
    /// Restart on failure
    OnFailure,
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
    /// Custom metric name (required when metric_type is Custom)
    #[serde(default)]
    pub metric_name: Option<String>,
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

/// Kubernetes controller kind for `runtime.preferred: kube`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub enum K8sWorkloadKind {
    #[default]
    Deployment,
    StatefulSet,
    DaemonSet,
    Job,
    CronJob,
}

/// Kubernetes-specific workload options (ignored by Podman/Docker/KubeVirt/Metal3).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct KubernetesSpec {
    /// Explicit controller kind. When omitted: `schedule` → CronJob, else Deployment.
    #[serde(default)]
    pub workload_kind: Option<K8sWorkloadKind>,
    #[serde(default)]
    pub image_pull_secrets: Vec<String>,
    #[serde(default)]
    pub service_account_name: Option<String>,
    #[serde(default)]
    pub priority_class_name: Option<String>,
    #[serde(default)]
    pub node_selector: HashMap<String, String>,
    #[serde(default)]
    pub tolerations: Vec<K8sTolerationSpec>,
    #[serde(default)]
    pub affinity: Option<K8sAffinitySpec>,
    #[serde(default)]
    pub pod_disruption_budget: Option<K8sPdbSpec>,
    #[serde(default)]
    pub pod_security_context: Option<K8sPodSecurityContextSpec>,
    #[serde(default)]
    pub container_security_context: Option<K8sContainerSecurityContextSpec>,
    #[serde(default)]
    pub init_containers: Vec<K8sContainerSpec>,
    #[serde(default)]
    pub sidecars: Vec<K8sContainerSpec>,
    #[serde(default)]
    pub extra_volumes: Vec<K8sExtraVolumeSpec>,
    #[serde(default)]
    pub gateway: Option<K8sGatewaySpec>,
    #[serde(default)]
    pub vertical_pod_autoscaler: Option<K8sVpaSpec>,
    #[serde(default)]
    pub keda: Option<K8sKedaSpec>,
    /// One-off Job settings when `workloadKind: job`.
    #[serde(default)]
    pub job: Option<K8sJobSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct K8sTolerationSpec {
    pub key: String,
    #[serde(default = "default_toleration_operator")]
    pub operator: String,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub effect: Option<String>,
}

fn default_toleration_operator() -> String {
    "Equal".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct K8sAffinitySpec {
    #[serde(default)]
    pub pod_affinity: Vec<K8sPodAffinityTermSpec>,
    #[serde(default)]
    pub pod_anti_affinity: Vec<K8sPodAffinityTermSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct K8sPodAffinityTermSpec {
    pub topology_key: String,
    #[serde(default)]
    pub label_selector: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct K8sPdbSpec {
    #[serde(default)]
    pub min_available: Option<String>,
    #[serde(default)]
    pub max_unavailable: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct K8sPodSecurityContextSpec {
    #[serde(default)]
    pub run_as_non_root: Option<bool>,
    #[serde(default)]
    pub run_as_user: Option<i64>,
    #[serde(default)]
    pub fs_group: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct K8sContainerSecurityContextSpec {
    #[serde(default)]
    pub run_as_non_root: Option<bool>,
    #[serde(default)]
    pub run_as_user: Option<i64>,
    #[serde(default)]
    pub read_only_root_filesystem: Option<bool>,
    #[serde(default)]
    pub allow_privilege_escalation: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct K8sContainerSpec {
    pub name: String,
    pub image: String,
    #[serde(default)]
    pub command: Vec<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct K8sExtraVolumeSpec {
    pub name: String,
    pub mount_path: String,
    #[serde(default)]
    pub read_only: bool,
    #[serde(flatten)]
    pub source: K8sVolumeSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
#[serde(tag = "volumeType", content = "volumeConfig")]
pub enum K8sVolumeSource {
    EmptyDir {
        #[serde(default)]
        medium: Option<String>,
    },
    HostPath {
        path: String,
        #[serde(default = "default_host_path_type")]
        host_path_type: String,
    },
}

fn default_host_path_type() -> String {
    "Directory".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct K8sGatewaySpec {
    pub enabled: bool,
    pub gateway_name: String,
    #[serde(default = "default_gateway_namespace")]
    pub gateway_namespace: String,
    pub host: String,
    #[serde(default)]
    pub paths: Vec<IngressPath>,
}

fn default_gateway_namespace() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct K8sVpaSpec {
    pub enabled: bool,
    #[serde(default = "default_vpa_update_mode")]
    pub update_mode: String,
}

fn default_vpa_update_mode() -> String {
    "Auto".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct K8sKedaSpec {
    pub enabled: bool,
    #[serde(default)]
    pub min_replica_count: u32,
    pub max_replica_count: u32,
    #[serde(default)]
    pub triggers: Vec<K8sKedaTriggerSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct K8sKedaTriggerSpec {
    #[serde(rename = "type")]
    pub trigger_type: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct K8sJobSpec {
    #[serde(default = "default_backoff_limit")]
    pub backoff_limit: u32,
    #[serde(default = "default_job_completions")]
    pub completions: u32,
    #[serde(default = "default_job_parallelism")]
    pub parallelism: u32,
    #[serde(default)]
    pub active_deadline_seconds: Option<i64>,
    #[serde(default)]
    pub ttl_seconds_after_finished: Option<i32>,
    #[serde(default)]
    pub restart_policy: JobRestartPolicy,
}

fn default_job_completions() -> u32 {
    1
}

fn default_job_parallelism() -> u32 {
    1
}

/// Intent-based deployment specification
///
/// Declares high-level goals (latency, budget, resilience, compliance) that
/// drive runtime selection via the scoring engine instead of explicit runtime choice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentSpec {
    /// Primary optimization goal
    pub goal: IntentGoal,
    /// Service-level targets (latency, availability)
    #[serde(default)]
    pub sla: Option<IntentSla>,
    /// Monthly cost ceiling
    #[serde(default)]
    pub budget: Option<IntentBudget>,
    /// Resilience / high-availability tier
    #[serde(default)]
    pub resilience: Option<ResilienceLevel>,
    /// Compliance and isolation requirements
    #[serde(default)]
    pub compliance: Option<ComplianceSpec>,
    /// Trust level — controls node attestation and security requirements
    #[serde(default)]
    pub trust: Option<TrustLevel>,
}

impl IntentSpec {
    /// Validate intent constraints are internally consistent.
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(ref sla) = self.sla {
            if let Some(latency) = sla.max_latency_ms {
                if latency == 0 {
                    anyhow::bail!("intent.sla.maxLatencyMs must be > 0");
                }
            }
            if let Some(avail) = sla.min_availability_pct {
                if avail <= 0.0 || avail > 100.0 {
                    anyhow::bail!(
                        "intent.sla.minAvailabilityPct must be in (0, 100], got {}",
                        avail
                    );
                }
            }
        }
        if let Some(ref budget) = self.budget {
            if budget.max_monthly_usd <= 0.0 {
                anyhow::bail!("intent.budget.maxMonthlyUsd must be > 0");
            }
        }
        Ok(())
    }
}

/// Primary optimization goal for intent-based deployment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum IntentGoal {
    /// Minimize response latency — favors performance weight
    LowLatency,
    /// Maximize throughput — favors performance + availability
    HighThroughput,
    /// Minimize cloud spend — favors cost weight
    CostOptimized,
    /// Equal weight across all dimensions
    Balanced,
}

/// Service-level agreement targets
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentSla {
    /// Maximum acceptable p99 latency in milliseconds
    #[serde(default)]
    pub max_latency_ms: Option<u32>,
    /// Minimum acceptable uptime percentage (e.g. 99.9)
    #[serde(default)]
    pub min_availability_pct: Option<f64>,
}

/// Monthly cost ceiling
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntentBudget {
    /// Maximum monthly cost in USD
    pub max_monthly_usd: f64,
}

/// Resilience / high-availability tier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum ResilienceLevel {
    /// No HA guarantees — acceptable for dev/test
    BestEffort,
    /// Standard redundancy (default for production)
    Standard,
    /// Full HA — multi-replica, auto-scaling required
    High,
}

/// Compliance and isolation requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ComplianceSpec {
    /// Workload must run in an isolated environment (VM or bare metal)
    #[serde(default)]
    pub isolation_required: bool,
    /// Workload data must be encrypted at rest
    #[serde(default)]
    pub encryption_required: bool,
}

/// Trust level — determines node attestation and security requirements
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum TrustLevel {
    /// No trust requirements — any node is acceptable
    None,
    /// Standard trust — basic security (default)
    Standard,
    /// Strict trust — requires TPM, secure boot, attested nodes only.
    /// Filters to KubeVirt (VM isolation) or Metal3 (dedicated hardware).
    Strict,
}

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
                cpu_request: None,
                memory_request: None,
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
            intent: None,
            schedule: None,
            kubernetes: None,
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
                cpu_request: None,
                memory_request: None,
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
            intent: None,
            schedule: None,
            kubernetes: None,
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
                cpu_request: None,
                memory_request: None,
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
            intent: None,
            schedule: None,
            kubernetes: None,
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
                cpu_request: None,
                memory_request: None,
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
            intent: None,
            schedule: None,
            kubernetes: None,
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
                cpu_request: None,
                memory_request: None,
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
            intent: None,
            schedule: None,
            kubernetes: None,
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
                cpu_request: None,
                memory_request: None,
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
            intent: None,
            schedule: None,
            kubernetes: None,
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
                cpu_request: None,
                memory_request: None,
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
            intent: None,
            schedule: None,
            kubernetes: None,
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

    // ---------------------------------------------------------------
    // Intent spec validation
    // ---------------------------------------------------------------

    #[test]
    fn test_intent_spec_validation_valid() {
        let mut w = make_valid_workload();
        w.intent = Some(IntentSpec {
            goal: IntentGoal::LowLatency,
            sla: Some(IntentSla {
                max_latency_ms: Some(50),
                min_availability_pct: Some(99.9),
            }),
            budget: Some(IntentBudget {
                max_monthly_usd: 500.0,
            }),
            resilience: Some(ResilienceLevel::High),
            compliance: Some(ComplianceSpec {
                isolation_required: true,
                encryption_required: false,
            }),
            trust: None,
        });
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_intent_budget_zero_rejected() {
        let mut w = make_valid_workload();
        w.intent = Some(IntentSpec {
            goal: IntentGoal::CostOptimized,
            sla: None,
            budget: Some(IntentBudget {
                max_monthly_usd: 0.0,
            }),
            resilience: None,
            compliance: None,
            trust: None,
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("maxMonthlyUsd"), "{}", err);
    }

    #[test]
    fn test_intent_budget_negative_rejected() {
        let mut w = make_valid_workload();
        w.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: None,
            budget: Some(IntentBudget {
                max_monthly_usd: -100.0,
            }),
            resilience: None,
            compliance: None,
            trust: None,
        });
        assert!(w.validate().is_err());
    }

    #[test]
    fn test_intent_availability_over_100_rejected() {
        let mut w = make_valid_workload();
        w.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: Some(IntentSla {
                max_latency_ms: None,
                min_availability_pct: Some(101.0),
            }),
            budget: None,
            resilience: None,
            compliance: None,
            trust: None,
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("minAvailabilityPct"), "{}", err);
    }

    #[test]
    fn test_intent_availability_zero_rejected() {
        let mut w = make_valid_workload();
        w.intent = Some(IntentSpec {
            goal: IntentGoal::Balanced,
            sla: Some(IntentSla {
                max_latency_ms: None,
                min_availability_pct: Some(0.0),
            }),
            budget: None,
            resilience: None,
            compliance: None,
            trust: None,
        });
        assert!(w.validate().is_err());
    }

    #[test]
    fn test_intent_latency_zero_rejected() {
        let mut w = make_valid_workload();
        w.intent = Some(IntentSpec {
            goal: IntentGoal::LowLatency,
            sla: Some(IntentSla {
                max_latency_ms: Some(0),
                min_availability_pct: None,
            }),
            budget: None,
            resilience: None,
            compliance: None,
            trust: None,
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("maxLatencyMs"), "{}", err);
    }

    #[test]
    fn test_intent_none_backward_compat() {
        let w = make_valid_workload();
        assert!(w.intent.is_none());
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_intent_serde_roundtrip() {
        let mut w = make_valid_workload();
        w.intent = Some(IntentSpec {
            goal: IntentGoal::HighThroughput,
            sla: Some(IntentSla {
                max_latency_ms: Some(100),
                min_availability_pct: Some(99.5),
            }),
            budget: Some(IntentBudget {
                max_monthly_usd: 1000.0,
            }),
            resilience: Some(ResilienceLevel::Standard),
            compliance: None,
            trust: None,
        });
        let yaml = serde_yaml::to_string(&w).unwrap();
        let parsed: Workload = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(w.intent, parsed.intent);
    }

    #[test]
    fn test_intent_goal_all_variants_serialize() {
        for goal in [
            IntentGoal::LowLatency,
            IntentGoal::HighThroughput,
            IntentGoal::CostOptimized,
            IntentGoal::Balanced,
        ] {
            let yaml = serde_yaml::to_string(&goal).unwrap();
            let parsed: IntentGoal = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(goal, parsed);
        }
    }

    #[test]
    fn test_intent_resilience_all_variants_serialize() {
        for level in [
            ResilienceLevel::BestEffort,
            ResilienceLevel::Standard,
            ResilienceLevel::High,
        ] {
            let yaml = serde_yaml::to_string(&level).unwrap();
            let parsed: ResilienceLevel = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(level, parsed);
        }
    }

    // ---------------------------------------------------------------
    // Schedule / CronJob validation
    // ---------------------------------------------------------------

    #[test]
    fn test_schedule_valid_cron() {
        let mut w = make_valid_workload();
        w.schedule = Some(ScheduleSpec {
            cron: "*/5 * * * *".to_string(),
            concurrency_policy: ConcurrencyPolicy::Allow,
            backoff_limit: 3,
            active_deadline_seconds: None,
            restart_policy: JobRestartPolicy::Never,
        });
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_schedule_empty_cron_rejected() {
        let mut w = make_valid_workload();
        w.schedule = Some(ScheduleSpec {
            cron: "".to_string(),
            concurrency_policy: ConcurrencyPolicy::Allow,
            backoff_limit: 3,
            active_deadline_seconds: None,
            restart_policy: JobRestartPolicy::Never,
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("cron"), "{}", err);
    }

    #[test]
    fn test_schedule_wrong_field_count_rejected() {
        let mut w = make_valid_workload();
        w.schedule = Some(ScheduleSpec {
            cron: "* * *".to_string(),
            concurrency_policy: ConcurrencyPolicy::Allow,
            backoff_limit: 3,
            active_deadline_seconds: None,
            restart_policy: JobRestartPolicy::Never,
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("5 fields"), "{}", err);
    }

    #[test]
    fn test_schedule_negative_deadline_rejected() {
        let mut w = make_valid_workload();
        w.schedule = Some(ScheduleSpec {
            cron: "0 * * * *".to_string(),
            concurrency_policy: ConcurrencyPolicy::Forbid,
            backoff_limit: 3,
            active_deadline_seconds: Some(-1),
            restart_policy: JobRestartPolicy::OnFailure,
        });
        let err = w.validate().unwrap_err().to_string();
        assert!(err.contains("activeDeadlineSeconds"), "{}", err);
    }

    #[test]
    fn test_schedule_none_backward_compat() {
        let w = make_valid_workload();
        assert!(w.schedule.is_none());
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_schedule_serde_roundtrip() {
        let mut w = make_valid_workload();
        w.schedule = Some(ScheduleSpec {
            cron: "0 2 * * *".to_string(),
            concurrency_policy: ConcurrencyPolicy::Replace,
            backoff_limit: 5,
            active_deadline_seconds: Some(3600),
            restart_policy: JobRestartPolicy::OnFailure,
        });
        let yaml = serde_yaml::to_string(&w).unwrap();
        let parsed: Workload = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(w.schedule, parsed.schedule);
    }

    #[test]
    fn test_concurrency_policy_all_variants_serialize() {
        for policy in [
            ConcurrencyPolicy::Allow,
            ConcurrencyPolicy::Forbid,
            ConcurrencyPolicy::Replace,
        ] {
            let yaml = serde_yaml::to_string(&policy).unwrap();
            let parsed: ConcurrencyPolicy = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(policy, parsed);
        }
    }

    // ---------------------------------------------------------------
    // Burstable QoS (cpu_request / memory_request)
    // ---------------------------------------------------------------

    #[test]
    fn test_resource_requests_valid() {
        let mut w = make_valid_workload();
        w.requirements.cpu_request = Some("500m".to_string());
        w.requirements.memory_request = Some("1Gi".to_string());
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_resource_requests_none_backward_compat() {
        let w = make_valid_workload();
        assert!(w.requirements.cpu_request.is_none());
        assert!(w.requirements.memory_request.is_none());
        assert!(w.validate().is_ok());
    }

    #[test]
    fn test_cpu_request_invalid_rejected() {
        let mut w = make_valid_workload();
        w.requirements.cpu_request = Some("abc".to_string());
        assert!(w.validate().is_err());
    }

    #[test]
    fn test_memory_request_invalid_rejected() {
        let mut w = make_valid_workload();
        w.requirements.memory_request = Some("not-memory".to_string());
        assert!(w.validate().is_err());
    }
}
