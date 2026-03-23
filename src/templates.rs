//! Workload template library
//!
//! Pre-built templates for common workload patterns with customizable
//! parameters. Generates production-ready workload specs.

use crate::spec::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Template identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TemplateKind {
    WebApp,
    RestApi,
    Database,
    Cache,
    Worker,
    CronJob,
    MlTraining,
    Microservice,
}

impl std::fmt::Display for TemplateKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TemplateKind::WebApp => write!(f, "Web Application"),
            TemplateKind::RestApi => write!(f, "REST API"),
            TemplateKind::Database => write!(f, "Database"),
            TemplateKind::Cache => write!(f, "Cache"),
            TemplateKind::Worker => write!(f, "Worker"),
            TemplateKind::CronJob => write!(f, "Cron Job"),
            TemplateKind::MlTraining => write!(f, "ML Training"),
            TemplateKind::Microservice => write!(f, "Microservice"),
        }
    }
}

impl std::str::FromStr for TemplateKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "web-app" | "webapp" => Ok(TemplateKind::WebApp),
            "rest-api" | "restapi" | "api" => Ok(TemplateKind::RestApi),
            "database" | "db" => Ok(TemplateKind::Database),
            "cache" => Ok(TemplateKind::Cache),
            "worker" => Ok(TemplateKind::Worker),
            "cron-job" | "cronjob" | "cron" => Ok(TemplateKind::CronJob),
            "ml-training" | "ml" => Ok(TemplateKind::MlTraining),
            "microservice" => Ok(TemplateKind::Microservice),
            _ => Err(anyhow::anyhow!(
                "Unknown template: '{}'. Valid: web-app, rest-api, database, cache, worker, cron-job, ml-training, microservice",
                s
            )),
        }
    }
}

/// Template metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateInfo {
    pub kind: TemplateKind,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub default_runtime: RuntimePreference,
}

/// Template parameters for customization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateParams {
    pub name: String,
    pub owner: String,
    pub project: String,
    pub registry: String,
    pub cpu: Option<String>,
    pub memory: Option<String>,
    pub storage: Option<String>,
    pub port: Option<u16>,
    pub replicas: Option<u32>,
    pub host: Option<String>,
}

impl Default for TemplateParams {
    fn default() -> Self {
        Self {
            name: "my-app".to_string(),
            owner: "team".to_string(),
            project: "default".to_string(),
            registry: "ghcr.io/org".to_string(),
            cpu: None,
            memory: None,
            storage: None,
            port: None,
            replicas: None,
            host: None,
        }
    }
}

/// List all available templates
pub fn list_templates() -> Vec<TemplateInfo> {
    vec![
        TemplateInfo {
            kind: TemplateKind::WebApp,
            name: "web-app".to_string(),
            description: "Production web application with ingress, TLS, auto-scaling, and health probes".to_string(),
            tags: vec!["web".to_string(), "http".to_string(), "production".to_string()],
            default_runtime: RuntimePreference::Kube,
        },
        TemplateInfo {
            kind: TemplateKind::RestApi,
            name: "rest-api".to_string(),
            description: "REST API service with health checks and horizontal scaling".to_string(),
            tags: vec!["api".to_string(), "rest".to_string(), "microservice".to_string()],
            default_runtime: RuntimePreference::Kube,
        },
        TemplateInfo {
            kind: TemplateKind::Database,
            name: "database".to_string(),
            description: "Stateful database with persistent storage and backup configuration".to_string(),
            tags: vec!["database".to_string(), "stateful".to_string(), "persistence".to_string()],
            default_runtime: RuntimePreference::Kube,
        },
        TemplateInfo {
            kind: TemplateKind::Cache,
            name: "cache".to_string(),
            description: "In-memory cache (Redis-style) with eviction and monitoring".to_string(),
            tags: vec!["cache".to_string(), "redis".to_string(), "memory".to_string()],
            default_runtime: RuntimePreference::Kube,
        },
        TemplateInfo {
            kind: TemplateKind::Worker,
            name: "worker".to_string(),
            description: "Background worker for queue processing with retry logic".to_string(),
            tags: vec!["worker".to_string(), "queue".to_string(), "async".to_string()],
            default_runtime: RuntimePreference::Kube,
        },
        TemplateInfo {
            kind: TemplateKind::CronJob,
            name: "cron-job".to_string(),
            description: "Scheduled job with configurable timing and retry".to_string(),
            tags: vec!["cron".to_string(), "batch".to_string(), "scheduled".to_string()],
            default_runtime: RuntimePreference::Container,
        },
        TemplateInfo {
            kind: TemplateKind::MlTraining,
            name: "ml-training".to_string(),
            description: "GPU-accelerated ML training job with large resource allocation".to_string(),
            tags: vec!["ml".to_string(), "gpu".to_string(), "training".to_string()],
            default_runtime: RuntimePreference::Kubevirt,
        },
        TemplateInfo {
            kind: TemplateKind::Microservice,
            name: "microservice".to_string(),
            description: "Lightweight microservice with service discovery and health probes".to_string(),
            tags: vec!["microservice".to_string(), "lightweight".to_string(), "service-mesh".to_string()],
            default_runtime: RuntimePreference::Kube,
        },
    ]
}

/// Generate a workload spec from a template
pub fn generate(kind: &TemplateKind, params: &TemplateParams) -> Workload {
    match kind {
        TemplateKind::WebApp => generate_web_app(params),
        TemplateKind::RestApi => generate_rest_api(params),
        TemplateKind::Database => generate_database(params),
        TemplateKind::Cache => generate_cache(params),
        TemplateKind::Worker => generate_worker(params),
        TemplateKind::CronJob => generate_cron_job(params),
        TemplateKind::MlTraining => generate_ml_training(params),
        TemplateKind::Microservice => generate_microservice(params),
    }
}

fn base_workload(params: &TemplateParams, pref: RuntimePreference, allow: Vec<RuntimeType>) -> Workload {
    Workload {
        api_version: "orchestr8/v1".to_string(),
        kind: "Workload".to_string(),
        metadata: Metadata {
            name: params.name.clone(),
            owner: params.owner.clone(),
            project: params.project.clone(),
            labels: HashMap::new(),
            annotations: HashMap::new(),
        },
        build: BuildSpec {
            context: PathBuf::from("."),
            dockerfile: PathBuf::from("Dockerfile"),
            registry: params.registry.clone(),
            build_args: HashMap::new(),
        },
        requirements: ResourceRequirements {
            cpu: params.cpu.clone().unwrap_or_else(|| "1".to_string()),
            memory: params.memory.clone().unwrap_or_else(|| "1Gi".to_string()),
            storage: params.storage.clone().unwrap_or_else(|| "10Gi".to_string()),
            gpu: None,
        },
        runtime: RuntimeSpec {
            preferred: pref,
            allow,
        },
        network: NetworkSpec::default(),
        persistence: PersistenceSpec::default(),
        health: None,
        config: None,
        ingress: None,
        scaling: None,
    }
}

fn generate_web_app(params: &TemplateParams) -> Workload {
    let port = params.port.unwrap_or(80);
    let mut w = base_workload(params, RuntimePreference::Kube, vec![RuntimeType::Kube, RuntimeType::Container]);

    w.requirements.cpu = params.cpu.clone().unwrap_or_else(|| "2".to_string());
    w.requirements.memory = params.memory.clone().unwrap_or_else(|| "2Gi".to_string());

    w.network = NetworkSpec {
        service: true,
        service_type: ServiceType::ClusterIP,
        ports: vec![PortMapping {
            container_port: port,
            service_port: port,
            protocol: "TCP".to_string(),
        }],
    };

    w.health = Some(HealthSpec {
        liveness: Some(HealthProbe {
            probe_type: ProbeType::HttpGet {
                path: "/health".to_string(),
                port,
            },
            initial_delay_seconds: 15,
            period_seconds: 10,
        }),
        readiness: Some(HealthProbe {
            probe_type: ProbeType::HttpGet {
                path: "/ready".to_string(),
                port,
            },
            initial_delay_seconds: 5,
            period_seconds: 5,
        }),
    });

    let host = params.host.clone().unwrap_or_else(|| {
        let fallback = format!("{}.example.com", params.name);
        tracing::warn!(
            "No host specified for web-app template '{}'; using fallback '{}'.  \
             Set a proper hostname via the --host flag or template params.",
            params.name, fallback
        );
        fallback
    });
    w.ingress = Some(IngressSpec {
        enabled: true,
        host,
        paths: vec![IngressPath {
            path: "/".to_string(),
            path_type: "Prefix".to_string(),
            port,
        }],
        tls: true,
        annotations: HashMap::new(),
    });

    let replicas = params.replicas.unwrap_or(2);
    w.scaling = Some(ScalingSpec {
        enabled: true,
        min_replicas: replicas,
        max_replicas: replicas * 5,
        metrics: vec![ScalingMetric {
            metric_type: MetricType::CPU,
            target_value: "70".to_string(),
        }],
    });

    w
}

fn generate_rest_api(params: &TemplateParams) -> Workload {
    let port = params.port.unwrap_or(8080);
    let mut w = base_workload(params, RuntimePreference::Kube, vec![RuntimeType::Kube, RuntimeType::Container]);

    w.requirements.cpu = params.cpu.clone().unwrap_or_else(|| "1".to_string());
    w.requirements.memory = params.memory.clone().unwrap_or_else(|| "1Gi".to_string());

    w.network = NetworkSpec {
        service: true,
        service_type: ServiceType::ClusterIP,
        ports: vec![PortMapping {
            container_port: port,
            service_port: port,
            protocol: "TCP".to_string(),
        }],
    };

    w.health = Some(HealthSpec {
        liveness: Some(HealthProbe {
            probe_type: ProbeType::HttpGet {
                path: "/health".to_string(),
                port,
            },
            initial_delay_seconds: 10,
            period_seconds: 10,
        }),
        readiness: Some(HealthProbe {
            probe_type: ProbeType::HttpGet {
                path: "/health".to_string(),
                port,
            },
            initial_delay_seconds: 5,
            period_seconds: 5,
        }),
    });

    let replicas = params.replicas.unwrap_or(2);
    w.scaling = Some(ScalingSpec {
        enabled: true,
        min_replicas: replicas,
        max_replicas: replicas * 4,
        metrics: vec![ScalingMetric {
            metric_type: MetricType::CPU,
            target_value: "75".to_string(),
        }],
    });

    w
}

fn generate_database(params: &TemplateParams) -> Workload {
    let port = params.port.unwrap_or(5432);
    let mut w = base_workload(params, RuntimePreference::Kube, vec![RuntimeType::Kube]);

    w.requirements.cpu = params.cpu.clone().unwrap_or_else(|| "2".to_string());
    w.requirements.memory = params.memory.clone().unwrap_or_else(|| "4Gi".to_string());
    w.requirements.storage = params.storage.clone().unwrap_or_else(|| "50Gi".to_string());

    w.network = NetworkSpec {
        service: true,
        service_type: ServiceType::ClusterIP,
        ports: vec![PortMapping {
            container_port: port,
            service_port: port,
            protocol: "TCP".to_string(),
        }],
    };

    w.persistence = PersistenceSpec {
        enabled: true,
        size: w.requirements.storage.clone(),
        access_mode: AccessMode::ReadWriteOnce,
        storage_class: Some("standard".to_string()),
    };

    w.health = Some(HealthSpec {
        liveness: Some(HealthProbe {
            probe_type: ProbeType::TcpSocket { port },
            initial_delay_seconds: 30,
            period_seconds: 10,
        }),
        readiness: Some(HealthProbe {
            probe_type: ProbeType::TcpSocket { port },
            initial_delay_seconds: 15,
            period_seconds: 5,
        }),
    });

    w
}

fn generate_cache(params: &TemplateParams) -> Workload {
    let port = params.port.unwrap_or(6379);
    let mut w = base_workload(params, RuntimePreference::Kube, vec![RuntimeType::Kube, RuntimeType::Container]);

    w.requirements.cpu = params.cpu.clone().unwrap_or_else(|| "1".to_string());
    w.requirements.memory = params.memory.clone().unwrap_or_else(|| "2Gi".to_string());
    w.requirements.storage = params.storage.clone().unwrap_or_else(|| "5Gi".to_string());

    w.network = NetworkSpec {
        service: true,
        service_type: ServiceType::ClusterIP,
        ports: vec![PortMapping {
            container_port: port,
            service_port: port,
            protocol: "TCP".to_string(),
        }],
    };

    w.health = Some(HealthSpec {
        liveness: Some(HealthProbe {
            probe_type: ProbeType::TcpSocket { port },
            initial_delay_seconds: 10,
            period_seconds: 10,
        }),
        readiness: Some(HealthProbe {
            probe_type: ProbeType::TcpSocket { port },
            initial_delay_seconds: 5,
            period_seconds: 5,
        }),
    });

    w
}

fn generate_worker(params: &TemplateParams) -> Workload {
    let mut w = base_workload(params, RuntimePreference::Kube, vec![RuntimeType::Kube, RuntimeType::Container]);

    w.requirements.cpu = params.cpu.clone().unwrap_or_else(|| "1".to_string());
    w.requirements.memory = params.memory.clone().unwrap_or_else(|| "1Gi".to_string());

    // Workers typically don't expose ports
    w.network = NetworkSpec::default();

    let replicas = params.replicas.unwrap_or(1);
    w.scaling = Some(ScalingSpec {
        enabled: true,
        min_replicas: replicas,
        max_replicas: replicas * 10,
        metrics: vec![ScalingMetric {
            metric_type: MetricType::Memory,
            target_value: "80".to_string(),
        }],
    });

    w
}

fn generate_cron_job(params: &TemplateParams) -> Workload {
    let mut w = base_workload(
        params,
        RuntimePreference::Container,
        vec![RuntimeType::Container, RuntimeType::Kube],
    );

    w.requirements.cpu = params.cpu.clone().unwrap_or_else(|| "500m".to_string());
    w.requirements.memory = params.memory.clone().unwrap_or_else(|| "512Mi".to_string());
    w.requirements.storage = params.storage.clone().unwrap_or_else(|| "1Gi".to_string());
    w.network = NetworkSpec::default();

    w
}

fn generate_ml_training(params: &TemplateParams) -> Workload {
    let mut w = base_workload(
        params,
        RuntimePreference::Kubevirt,
        vec![RuntimeType::Kubevirt, RuntimeType::Metal, RuntimeType::Kube],
    );

    w.requirements.cpu = params.cpu.clone().unwrap_or_else(|| "8".to_string());
    w.requirements.memory = params.memory.clone().unwrap_or_else(|| "32Gi".to_string());
    w.requirements.storage = params.storage.clone().unwrap_or_else(|| "200Gi".to_string());
    w.requirements.gpu = Some(GpuRequirements {
        count: 1,
        vendor: "nvidia".to_string(),
    });

    w.network = NetworkSpec::default();

    w
}

fn generate_microservice(params: &TemplateParams) -> Workload {
    let port = params.port.unwrap_or(8080);
    let mut w = base_workload(params, RuntimePreference::Kube, vec![RuntimeType::Kube, RuntimeType::Container]);

    w.requirements.cpu = params.cpu.clone().unwrap_or_else(|| "500m".to_string());
    w.requirements.memory = params.memory.clone().unwrap_or_else(|| "512Mi".to_string());
    w.requirements.storage = params.storage.clone().unwrap_or_else(|| "1Gi".to_string());

    w.network = NetworkSpec {
        service: true,
        service_type: ServiceType::ClusterIP,
        ports: vec![PortMapping {
            container_port: port,
            service_port: port,
            protocol: "TCP".to_string(),
        }],
    };

    w.health = Some(HealthSpec {
        liveness: Some(HealthProbe {
            probe_type: ProbeType::HttpGet {
                path: "/health".to_string(),
                port,
            },
            initial_delay_seconds: 5,
            period_seconds: 10,
        }),
        readiness: Some(HealthProbe {
            probe_type: ProbeType::HttpGet {
                path: "/ready".to_string(),
                port,
            },
            initial_delay_seconds: 3,
            period_seconds: 5,
        }),
    });

    w
}

/// Format template list as a report
pub fn format_template_list() -> String {
    let mut output = String::new();
    output.push_str("Available Templates:\n\n");

    for tmpl in list_templates() {
        output.push_str(&format!("  {} - {}\n", tmpl.name, tmpl.description));
        output.push_str(&format!(
            "    Tags: {}  |  Runtime: {:?}\n\n",
            tmpl.tags.join(", "),
            tmpl.default_runtime
        ));
    }

    output.push_str("Usage: orchestr8 template <name> --name <workload-name>\n");
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_templates() {
        let templates = list_templates();
        assert_eq!(templates.len(), 8);
    }

    #[test]
    fn test_generate_web_app() {
        let params = TemplateParams {
            name: "my-web".to_string(),
            ..Default::default()
        };
        let spec = generate(&TemplateKind::WebApp, &params);
        assert_eq!(spec.metadata.name, "my-web");
        assert!(spec.ingress.is_some());
        assert!(spec.health.is_some());
        assert!(spec.scaling.is_some());
    }

    #[test]
    fn test_generate_database() {
        let params = TemplateParams::default();
        let spec = generate(&TemplateKind::Database, &params);
        assert!(spec.persistence.enabled);
        assert!(spec.health.is_some());
    }

    #[test]
    fn test_generate_ml_training() {
        let params = TemplateParams::default();
        let spec = generate(&TemplateKind::MlTraining, &params);
        assert!(spec.requirements.gpu.is_some());
    }

    #[test]
    fn test_generate_microservice() {
        let params = TemplateParams {
            name: "auth-svc".to_string(),
            port: Some(9090),
            ..Default::default()
        };
        let spec = generate(&TemplateKind::Microservice, &params);
        assert_eq!(spec.network.ports[0].container_port, 9090);
        assert!(spec.health.is_some());
    }

    #[test]
    fn test_custom_resources() {
        let params = TemplateParams {
            name: "heavy-api".to_string(),
            cpu: Some("8".to_string()),
            memory: Some("16Gi".to_string()),
            ..Default::default()
        };
        let spec = generate(&TemplateKind::RestApi, &params);
        assert_eq!(spec.requirements.cpu, "8");
        assert_eq!(spec.requirements.memory, "16Gi");
    }

    #[test]
    fn test_format_list() {
        let output = format_template_list();
        assert!(output.contains("web-app"));
        assert!(output.contains("database"));
    }

    #[test]
    fn test_template_kind_from_str() {
        assert_eq!("web-app".parse::<TemplateKind>().unwrap(), TemplateKind::WebApp);
        assert_eq!("rest-api".parse::<TemplateKind>().unwrap(), TemplateKind::RestApi);
        assert_eq!("api".parse::<TemplateKind>().unwrap(), TemplateKind::RestApi);
        assert_eq!("database".parse::<TemplateKind>().unwrap(), TemplateKind::Database);
        assert_eq!("db".parse::<TemplateKind>().unwrap(), TemplateKind::Database);
        assert_eq!("cache".parse::<TemplateKind>().unwrap(), TemplateKind::Cache);
        assert_eq!("worker".parse::<TemplateKind>().unwrap(), TemplateKind::Worker);
        assert_eq!("cron-job".parse::<TemplateKind>().unwrap(), TemplateKind::CronJob);
        assert_eq!("ml-training".parse::<TemplateKind>().unwrap(), TemplateKind::MlTraining);
        assert_eq!("microservice".parse::<TemplateKind>().unwrap(), TemplateKind::Microservice);
        assert!("nonexistent".parse::<TemplateKind>().is_err());
    }
}
