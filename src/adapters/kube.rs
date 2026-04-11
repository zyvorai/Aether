//! Kubernetes runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::{AccessMode, Workload};
use async_trait::async_trait;
use k8s_openapi::api::autoscaling::v2::{
    HorizontalPodAutoscaler, HorizontalPodAutoscalerSpec, MetricSpec, MetricTarget,
    ResourceMetricSource,
};
use k8s_openapi::api::core::v1::{
    ConfigMap, Container, ContainerPort, EnvFromSource as K8sEnvFromSource, HTTPGetAction,
    PersistentVolumeClaim, PersistentVolumeClaimSpec, Pod, PodSpec, Probe,
    ResourceRequirements as K8sResourceRequirements, Secret, Service, ServicePort, ServiceSpec,
    VolumeResourceRequirements, ConfigMapEnvSource, SecretEnvSource,
};
use k8s_openapi::api::networking::v1::{
    HTTPIngressPath, HTTPIngressRuleValue, Ingress, IngressBackend, IngressRule,
    IngressServiceBackend, IngressSpec, IngressTLS, ServiceBackendPort,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use kube::{
    api::{Api, DeleteParams, ListParams, LogParams, PostParams},
    Client,
};
use std::collections::BTreeMap;

/// Kubernetes runtime implementation
pub struct KubernetesRuntime {
    client: Client,
    namespace: String,
}

super::impl_kube_adapter_new!(KubernetesRuntime, "default");

impl KubernetesRuntime {
    /// Get pod status
    async fn get_pod_status(&self, name: &str) -> anyhow::Result<Status> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        match pods.get(name).await {
            Ok(pod) => {
                let pod_status = pod.status.as_ref();
                let phase = pod_status
                    .and_then(|s| s.phase.as_ref())
                    .map(|p| p.as_str())
                    .unwrap_or("Unknown");

                let state = match phase {
                    "Pending" => InstanceState::Pending,
                    "Running" => InstanceState::Running,
                    "Succeeded" => InstanceState::Stopped,
                    "Failed" => InstanceState::Failed,
                    _ => InstanceState::Unknown,
                };

                let ready = pod_status
                    .and_then(|s| s.conditions.as_ref())
                    .and_then(|conditions| {
                        conditions.iter().find(|c| c.type_ == "Ready")
                    })
                    .map(|c| c.status == "True")
                    .unwrap_or(false);

                let message = pod_status
                    .and_then(|s| s.message.clone())
                    .or_else(|| pod_status.and_then(|s| s.reason.clone()));

                let restart_count = pod_status
                    .and_then(|s| s.container_statuses.as_ref())
                    .and_then(|statuses| statuses.first())
                    .map(|s| s.restart_count as u32)
                    .unwrap_or(0);

                Ok(Status {
                    state,
                    ready,
                    message,
                    restart_count,
                })
            }
            Err(_) => Ok(Status {
                state: InstanceState::Unknown,
                ready: false,
                message: Some("Pod not found".to_string()),
                restart_count: 0,
            }),
        }
    }
}

use super::common::validate_kube_name;

// ---------------------------------------------------------------------------
// Standalone manifest-generation functions (testable without a kube::Client)
// ---------------------------------------------------------------------------

/// Sanitize a string into a valid Kubernetes DNS-1123 label for use as a
/// volume name.  Replaces invalid characters with hyphens, lowercases,
/// trims leading/trailing hyphens, and truncates to 63 characters.
fn sanitize_volume_name(name: &str) -> String {
    let sanitized: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "vol".to_string()
    } else {
        trimmed.chars().take(63).collect()
    }
}

/// Build a Pod manifest from workload spec.
fn build_pod_manifest(namespace: &str, image: &Image, spec: &Workload) -> Pod {
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "orchestr8".to_string());

    // Add user labels
    for (k, v) in &spec.metadata.labels {
        labels.insert(k.clone(), v.clone());
    }

    // Container ports
    let ports: Vec<ContainerPort> = spec
        .network
        .ports
        .iter()
        .map(|p| ContainerPort {
            container_port: p.container_port as i32,
            protocol: Some(p.protocol.clone()),
            ..Default::default()
        })
        .collect();

    // Resource requirements
    let mut limits = BTreeMap::new();
    let mut requests = BTreeMap::new();

    limits.insert("cpu".to_string(), Quantity(spec.requirements.cpu.clone()));
    limits.insert("memory".to_string(), Quantity(spec.requirements.memory.clone()));

    requests.insert("cpu".to_string(), Quantity(spec.requirements.cpu.clone()));
    requests.insert("memory".to_string(), Quantity(spec.requirements.memory.clone()));

    let resources = K8sResourceRequirements {
        limits: Some(limits),
        requests: Some(requests),
        ..Default::default()
    };

    // Health probes
    let liveness_probe = spec.health.as_ref().and_then(|h| {
        h.liveness.as_ref().map(|probe| {
            let http_get = match &probe.probe_type {
                crate::spec::ProbeType::HttpGet { path, port } => Some(HTTPGetAction {
                    path: Some(path.clone()),
                    port: IntOrString::Int(*port as i32),
                    ..Default::default()
                }),
                _ => None,
            };

            Probe {
                http_get,
                initial_delay_seconds: Some(probe.initial_delay_seconds as i32),
                period_seconds: Some(probe.period_seconds as i32),
                ..Default::default()
            }
        })
    });

    let readiness_probe = spec.health.as_ref().and_then(|h| {
        h.readiness.as_ref().map(|probe| {
            let http_get = match &probe.probe_type {
                crate::spec::ProbeType::HttpGet { path, port } => Some(HTTPGetAction {
                    path: Some(path.clone()),
                    port: IntOrString::Int(*port as i32),
                    ..Default::default()
                }),
                _ => None,
            };

            Probe {
                http_get,
                initial_delay_seconds: Some(probe.initial_delay_seconds as i32),
                period_seconds: Some(probe.period_seconds as i32),
                ..Default::default()
            }
        })
    });

    // Environment variables from ConfigMaps and Secrets
    let env_from: Vec<K8sEnvFromSource> = spec
        .config
        .as_ref()
        .map(|c| {
            c.env_from
                .iter()
                .map(|e| match e.source_type {
                    crate::spec::EnvSourceType::ConfigMap => K8sEnvFromSource {
                        config_map_ref: Some(ConfigMapEnvSource {
                            name: e.name.clone(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    crate::spec::EnvSourceType::Secret => K8sEnvFromSource {
                        secret_ref: Some(SecretEnvSource {
                            name: e.name.clone(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                })
                .collect()
        })
        .unwrap_or_default();

    // Inline environment variables from config maps (e.g. compose env injection)
    let inline_env: Vec<k8s_openapi::api::core::v1::EnvVar> = spec
        .config
        .as_ref()
        .map(|c| {
            c.config_maps
                .iter()
                .flat_map(|cm| {
                    cm.data.iter().map(|(k, v)| k8s_openapi::api::core::v1::EnvVar {
                        name: k.clone(),
                        value: Some(v.clone()),
                        ..Default::default()
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Volume mounts for ConfigMaps, Secrets, and PVCs
    let mut volumes: Vec<k8s_openapi::api::core::v1::Volume> = Vec::new();
    let mut volume_mounts: Vec<k8s_openapi::api::core::v1::VolumeMount> = Vec::new();

    if let Some(config) = &spec.config {
        for cm in &config.config_maps {
            if let Some(mount_path) = &cm.mount_path {
                let vol_name = sanitize_volume_name(&format!("cm-{}", cm.name));
                volumes.push(k8s_openapi::api::core::v1::Volume {
                    name: vol_name.clone(),
                    config_map: Some(k8s_openapi::api::core::v1::ConfigMapVolumeSource {
                        name: cm.name.clone(),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
                volume_mounts.push(k8s_openapi::api::core::v1::VolumeMount {
                    name: vol_name,
                    mount_path: mount_path.clone(),
                    read_only: Some(true),
                    ..Default::default()
                });
            }
        }
        for secret in &config.secrets {
            if let Some(mount_path) = &secret.mount_path {
                let vol_name = sanitize_volume_name(&format!("secret-{}", secret.name));
                volumes.push(k8s_openapi::api::core::v1::Volume {
                    name: vol_name.clone(),
                    secret: Some(k8s_openapi::api::core::v1::SecretVolumeSource {
                        secret_name: Some(secret.name.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
                volume_mounts.push(k8s_openapi::api::core::v1::VolumeMount {
                    name: vol_name,
                    mount_path: mount_path.clone(),
                    read_only: Some(true),
                    ..Default::default()
                });
            }
        }
    }

    // Mount PVC when persistence is enabled
    if spec.persistence.enabled {
        let vol_name = format!("{}-storage", spec.metadata.name);
        volumes.push(k8s_openapi::api::core::v1::Volume {
            name: vol_name.clone(),
            persistent_volume_claim: Some(
                k8s_openapi::api::core::v1::PersistentVolumeClaimVolumeSource {
                    claim_name: format!("{}-pvc", spec.metadata.name),
                    read_only: Some(false),
                },
            ),
            ..Default::default()
        });
        volume_mounts.push(k8s_openapi::api::core::v1::VolumeMount {
            name: vol_name,
            mount_path: "/data".to_string(),
            ..Default::default()
        });
    }

    // Container spec
    let container = Container {
        name: spec.metadata.name.clone(),
        image: Some(image.full_name()),
        ports: Some(ports),
        resources: Some(resources),
        liveness_probe,
        readiness_probe,
        env: if inline_env.is_empty() { None } else { Some(inline_env) },
        env_from: if env_from.is_empty() {
            None
        } else {
            Some(env_from)
        },
        volume_mounts: if volume_mounts.is_empty() { None } else { Some(volume_mounts) },
        ..Default::default()
    };

    Pod {
        metadata: ObjectMeta {
            name: Some(spec.metadata.name.clone()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels.clone()),
            annotations: Some(spec.metadata.annotations.clone().into_iter().collect()),
            ..Default::default()
        },
        spec: Some(PodSpec {
            containers: vec![container],
            volumes: if volumes.is_empty() { None } else { Some(volumes) },
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Build a Service manifest from workload spec.
fn build_service_manifest(namespace: &str, spec: &Workload) -> Option<Service> {
    if !spec.network.service {
        return None;
    }

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "orchestr8".to_string());

    let ports: Vec<ServicePort> = spec
        .network
        .ports
        .iter()
        .map(|p| ServicePort {
            port: p.service_port as i32,
            target_port: Some(IntOrString::Int(p.container_port as i32)),
            protocol: Some(p.protocol.clone()),
            ..Default::default()
        })
        .collect();

    let service_type = match spec.network.service_type {
        crate::spec::ServiceType::ClusterIP => "ClusterIP",
        crate::spec::ServiceType::NodePort => "NodePort",
        crate::spec::ServiceType::LoadBalancer => "LoadBalancer",
    };

    let mut selector = BTreeMap::new();
    selector.insert("app".to_string(), spec.metadata.name.clone());

    Some(Service {
        metadata: ObjectMeta {
            name: Some(format!("{}-service", spec.metadata.name)),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            type_: Some(service_type.to_string()),
            ports: Some(ports),
            selector: Some(selector),
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// Build a PersistentVolumeClaim manifest from workload spec.
fn build_pvc_manifest(namespace: &str, spec: &Workload) -> Option<PersistentVolumeClaim> {
    if !spec.persistence.enabled {
        return None;
    }

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "orchestr8".to_string());

    let access_mode = match spec.persistence.access_mode {
        AccessMode::ReadWriteOnce => "ReadWriteOnce",
        AccessMode::ReadOnlyMany => "ReadOnlyMany",
        AccessMode::ReadWriteMany => "ReadWriteMany",
    };

    let mut requests = BTreeMap::new();
    requests.insert("storage".to_string(), Quantity(spec.persistence.size.clone()));

    Some(PersistentVolumeClaim {
        metadata: ObjectMeta {
            name: Some(format!("{}-pvc", spec.metadata.name)),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        spec: Some(PersistentVolumeClaimSpec {
            access_modes: Some(vec![access_mode.to_string()]),
            resources: Some(VolumeResourceRequirements {
                requests: Some(requests),
                ..Default::default()
            }),
            storage_class_name: spec.persistence.storage_class.clone(),
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// Build ConfigMap manifests from workload spec.
fn build_configmap_manifests(namespace: &str, spec: &Workload) -> Vec<ConfigMap> {
    let config = match &spec.config {
        Some(c) => c,
        None => return vec![],
    };

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "orchestr8".to_string());

    config
        .config_maps
        .iter()
        .map(|cm| ConfigMap {
            metadata: ObjectMeta {
                name: Some(cm.name.clone()),
                namespace: Some(namespace.to_string()),
                labels: Some(labels.clone()),
                ..Default::default()
            },
            data: Some(cm.data.clone().into_iter().collect()),
            ..Default::default()
        })
        .collect()
}

/// Build Secret manifests from workload spec.
fn build_secret_manifests(namespace: &str, spec: &Workload) -> Vec<Secret> {
    let config = match &spec.config {
        Some(c) => c,
        None => return vec![],
    };

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "orchestr8".to_string());

    config
        .secrets
        .iter()
        .map(|s| {
            // Convert string values to base64-encoded bytes for k8s secrets
            let string_data: BTreeMap<String, String> =
                s.data.clone().into_iter().collect();

            Secret {
                metadata: ObjectMeta {
                    name: Some(s.name.clone()),
                    namespace: Some(namespace.to_string()),
                    labels: Some(labels.clone()),
                    ..Default::default()
                },
                string_data: Some(string_data),
                type_: Some("Opaque".to_string()),
                ..Default::default()
            }
        })
        .collect()
}

/// Build an Ingress manifest from workload spec.
fn build_ingress_manifest(namespace: &str, spec: &Workload) -> Option<Ingress> {
    let ingress_spec = match &spec.ingress {
        Some(i) if i.enabled => i,
        _ => return None,
    };

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "orchestr8".to_string());

    let annotations: BTreeMap<String, String> =
        ingress_spec.annotations.clone().into_iter().collect();

    // Build HTTP paths
    let paths: Vec<HTTPIngressPath> = ingress_spec
        .paths
        .iter()
        .map(|p| HTTPIngressPath {
            path: Some(p.path.clone()),
            path_type: p.path_type.clone(),
            backend: IngressBackend {
                service: Some(IngressServiceBackend {
                    name: format!("{}-service", spec.metadata.name),
                    port: Some(ServiceBackendPort {
                        number: Some(p.port as i32),
                        ..Default::default()
                    }),
                }),
                ..Default::default()
            },
        })
        .collect();

    let rule = IngressRule {
        host: Some(ingress_spec.host.clone()),
        http: Some(HTTPIngressRuleValue { paths }),
    };

    // TLS configuration
    let tls = if ingress_spec.tls {
        Some(vec![IngressTLS {
            hosts: Some(vec![ingress_spec.host.clone()]),
            secret_name: Some(format!("{}-tls", spec.metadata.name)),
        }])
    } else {
        None
    };

    Some(Ingress {
        metadata: ObjectMeta {
            name: Some(format!("{}-ingress", spec.metadata.name)),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            annotations: Some(annotations),
            ..Default::default()
        },
        spec: Some(IngressSpec {
            rules: Some(vec![rule]),
            tls,
            ..Default::default()
        }),
        ..Default::default()
    })
}

/// Build a HorizontalPodAutoscaler manifest from workload spec.
fn build_hpa_manifest(namespace: &str, spec: &Workload) -> Option<HorizontalPodAutoscaler> {
    let scaling_spec = match &spec.scaling {
        Some(s) if s.enabled => s,
        _ => return None,
    };

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "orchestr8".to_string());

    // Build metrics
    let metrics: Vec<MetricSpec> = scaling_spec
        .metrics
        .iter()
        .filter_map(|m| match m.metric_type {
            crate::spec::MetricType::CPU => {
                // Parse target value (e.g., "80" for 80%)
                let target_value: i32 = match m.target_value.trim_end_matches('%').parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!(
                            value = %m.target_value,
                            "invalid CPU HPA target value, skipping metric"
                        );
                        return None;
                    }
                };

                Some(MetricSpec {
                    type_: "Resource".to_string(),
                    resource: Some(ResourceMetricSource {
                        name: "cpu".to_string(),
                        target: MetricTarget {
                            type_: "Utilization".to_string(),
                            average_utilization: Some(target_value),
                            ..Default::default()
                        },
                    }),
                    ..Default::default()
                })
            }
            crate::spec::MetricType::Memory => {
                let target_value: i32 = match m.target_value.trim_end_matches('%').parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!(
                            value = %m.target_value,
                            "invalid memory HPA target value, skipping metric"
                        );
                        return None;
                    }
                };

                Some(MetricSpec {
                    type_: "Resource".to_string(),
                    resource: Some(ResourceMetricSource {
                        name: "memory".to_string(),
                        target: MetricTarget {
                            type_: "Utilization".to_string(),
                            average_utilization: Some(target_value),
                            ..Default::default()
                        },
                    }),
                    ..Default::default()
                })
            }
            crate::spec::MetricType::Custom => None, // Custom metrics not implemented yet
        })
        .collect();

    // Don't create an HPA with no valid metrics
    if metrics.is_empty() {
        tracing::warn!("No valid HPA metrics for '{}', skipping HPA creation", spec.metadata.name);
        return None;
    }

    // Validate min/max replicas
    if scaling_spec.min_replicas > scaling_spec.max_replicas {
        tracing::warn!(
            "Invalid scaling spec for '{}': min_replicas ({}) > max_replicas ({}), skipping HPA",
            spec.metadata.name, scaling_spec.min_replicas, scaling_spec.max_replicas
        );
        return None;
    }

    let mut match_labels = BTreeMap::new();
    match_labels.insert("app".to_string(), spec.metadata.name.clone());

    Some(HorizontalPodAutoscaler {
        metadata: ObjectMeta {
            name: Some(format!("{}-hpa", spec.metadata.name)),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        spec: Some(HorizontalPodAutoscalerSpec {
            scale_target_ref: k8s_openapi::api::autoscaling::v2::CrossVersionObjectReference {
                api_version: Some("apps/v1".to_string()),
                kind: "Deployment".to_string(),
                name: spec.metadata.name.clone(),
            },
            min_replicas: Some(scaling_spec.min_replicas as i32),
            max_replicas: scaling_spec.max_replicas as i32,
            metrics: Some(metrics),
            ..Default::default()
        }),
        ..Default::default()
    })
}

#[async_trait]
impl Runtime for KubernetesRuntime {
    async fn build(&self, spec: &Workload) -> crate::Result<Image> {
        // For Kubernetes, we assume the image is already built and pushed to registry
        // In a real implementation, you might want to:
        // 1. Build with Podman locally
        // 2. Push to the registry specified in spec.build.registry

        tracing::info!(
            "Kubernetes: Using pre-built image {} (ensure it's pushed to registry)",
            spec.image_name()
        );

        Ok(Image {
            name: spec.metadata.name.clone(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::Kubernetes,
        })
    }

    async fn run(&self, image: &Image, spec: &Workload) -> crate::Result<Instance> {
        // Validate workload name is a valid Kubernetes DNS label before creating any resources
        validate_kube_name(&spec.metadata.name)?;

        tracing::info!(
            "Deploying to Kubernetes namespace '{}': {}",
            self.namespace,
            spec.metadata.name
        );

        // Create ConfigMaps
        for configmap in build_configmap_manifests(&self.namespace, spec) {
            let configmaps: Api<ConfigMap> =
                Api::namespaced(self.client.clone(), &self.namespace);

            match configmaps.create(&PostParams::default(), &configmap).await {
                Ok(_) => {
                    tracing::info!(
                        "Created ConfigMap: {}",
                        configmap.metadata.name.unwrap_or_default()
                    )
                }
                Err(e) => tracing::warn!("ConfigMap creation failed (may already exist): {}", e),
            }
        }

        // Create Secrets
        for secret in build_secret_manifests(&self.namespace, spec) {
            let secrets: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);

            match secrets.create(&PostParams::default(), &secret).await {
                Ok(_) => {
                    tracing::info!(
                        "Created Secret: {}",
                        secret.metadata.name.unwrap_or_default()
                    )
                }
                Err(e) => tracing::warn!("Secret creation failed (may already exist): {}", e),
            }
        }

        // Create PVC if needed
        if let Some(pvc) = build_pvc_manifest(&self.namespace, spec) {
            let pvcs: Api<PersistentVolumeClaim> =
                Api::namespaced(self.client.clone(), &self.namespace);

            match pvcs.create(&PostParams::default(), &pvc).await {
                Ok(_) => tracing::info!("Created PVC: {}-pvc", spec.metadata.name),
                Err(e) => tracing::warn!("PVC creation failed (may already exist): {}", e),
            }
        }

        // Create Service if needed
        if let Some(service) = build_service_manifest(&self.namespace, spec) {
            let services: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);

            match services.create(&PostParams::default(), &service).await {
                Ok(_) => tracing::info!("Created Service: {}-service", spec.metadata.name),
                Err(e) => tracing::warn!("Service creation failed (may already exist): {}", e),
            }
        }

        // Create Ingress if needed
        if let Some(ingress) = build_ingress_manifest(&self.namespace, spec) {
            let ingresses: Api<Ingress> = Api::namespaced(self.client.clone(), &self.namespace);

            match ingresses.create(&PostParams::default(), &ingress).await {
                Ok(_) => tracing::info!("Created Ingress: {}-ingress", spec.metadata.name),
                Err(e) => tracing::warn!("Ingress creation failed (may already exist): {}", e),
            }
        }

        // Create Pod
        let pod = build_pod_manifest(&self.namespace, image, spec);
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        let created_pod = pods.create(&PostParams::default(), &pod).await?;

        let pod_name = created_pod
            .metadata
            .name
            .unwrap_or_else(|| spec.metadata.name.clone());

        let uid = created_pod
            .metadata
            .uid
            .unwrap_or_else(|| "unknown".to_string());

        tracing::info!("Created Pod: {}", pod_name);

        // Create HPA if needed
        if let Some(hpa) = build_hpa_manifest(&self.namespace, spec) {
            let hpas: Api<HorizontalPodAutoscaler> =
                Api::namespaced(self.client.clone(), &self.namespace);

            match hpas.create(&PostParams::default(), &hpa).await {
                Ok(_) => tracing::info!("Created HPA: {}-hpa", spec.metadata.name),
                Err(e) => tracing::warn!("HPA creation failed (may already exist): {}", e),
            }
        }

        Ok(Instance::new(uid, pod_name, RuntimeKind::Kubernetes, image.full_name()))
    }

    async fn stop(&self, instance: &Instance) -> crate::Result<()> {
        validate_kube_name(&instance.name)?;
        tracing::info!("Stopping (deleting) Pod: {}", instance.name);

        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        pods.delete(&instance.name, &DeleteParams::default()).await?;

        Ok(())
    }

    async fn status(&self, instance: &Instance) -> crate::Result<Status> {
        self.get_pod_status(&instance.name).await
    }

    async fn logs(&self, instance: &Instance, follow: bool) -> crate::Result<String> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        let log_params = LogParams {
            follow,
            ..Default::default()
        };

        let logs = pods.logs(&instance.name, &log_params).await?;
        Ok(logs)
    }

    async fn delete(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Deleting Kubernetes resources for: {}", instance.name);

        // Delete HPA
        let hpas: Api<HorizontalPodAutoscaler> =
            Api::namespaced(self.client.clone(), &self.namespace);
        let hpa_name = format!("{}-hpa", instance.name);
        match hpas.delete(&hpa_name, &DeleteParams::default()).await {
            Ok(_) => tracing::info!("Deleted HPA: {}", hpa_name),
            Err(e) => tracing::debug!("HPA deletion failed (may not exist): {}", e),
        }

        // Delete Pod
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        match pods.delete(&instance.name, &DeleteParams::default()).await {
            Ok(_) => tracing::info!("Deleted Pod: {}", instance.name),
            Err(e) => tracing::warn!("Pod deletion failed: {}", e),
        }

        // Delete Ingress
        let ingresses: Api<Ingress> = Api::namespaced(self.client.clone(), &self.namespace);
        let ingress_name = format!("{}-ingress", instance.name);
        match ingresses
            .delete(&ingress_name, &DeleteParams::default())
            .await
        {
            Ok(_) => tracing::info!("Deleted Ingress: {}", ingress_name),
            Err(e) => tracing::debug!("Ingress deletion failed (may not exist): {}", e),
        }

        // Delete Service
        let services: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
        let service_name = format!("{}-service", instance.name);
        match services.delete(&service_name, &DeleteParams::default()).await {
            Ok(_) => tracing::info!("Deleted Service: {}", service_name),
            Err(e) => tracing::debug!("Service deletion failed (may not exist): {}", e),
        }

        // Delete PVC
        let pvcs: Api<PersistentVolumeClaim> =
            Api::namespaced(self.client.clone(), &self.namespace);
        let pvc_name = format!("{}-pvc", instance.name);
        match pvcs.delete(&pvc_name, &DeleteParams::default()).await {
            Ok(_) => tracing::info!("Deleted PVC: {}", pvc_name),
            Err(e) => tracing::debug!("PVC deletion failed (may not exist): {}", e),
        }

        // Delete ConfigMaps and Secrets managed by orchestr8
        // We'll use label selectors to find and delete them
        validate_kube_name(&instance.name)?;
        let lp = ListParams::default().labels(&format!(
            "app={},managed-by=orchestr8",
            instance.name
        ));

        let configmaps: Api<ConfigMap> = Api::namespaced(self.client.clone(), &self.namespace);
        match configmaps.list(&lp).await {
            Ok(cm_list) => {
                for cm in cm_list.items {
                    if let Some(name) = cm.metadata.name {
                        match configmaps.delete(&name, &DeleteParams::default()).await {
                            Ok(_) => tracing::info!("Deleted ConfigMap: {}", name),
                            Err(e) => tracing::debug!("ConfigMap deletion failed: {}", e),
                        }
                    }
                }
            }
            Err(e) => tracing::debug!("ConfigMap list failed: {}", e),
        }

        let secrets: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);
        match secrets.list(&lp).await {
            Ok(secret_list) => {
                for secret in secret_list.items {
                    if let Some(name) = secret.metadata.name {
                        match secrets.delete(&name, &DeleteParams::default()).await {
                            Ok(_) => tracing::info!("Deleted Secret: {}", name),
                            Err(e) => tracing::debug!("Secret deletion failed: {}", e),
                        }
                    }
                }
            }
            Err(e) => tracing::debug!("Secret list failed: {}", e),
        }

        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);

        // List only pods managed by orchestr8
        let lp = ListParams::default().labels("managed-by=orchestr8");
        let pod_list = pods.list(&lp).await?;

        let instances: Vec<Instance> = pod_list
            .items
            .iter()
            .map(|pod| {
                let name = pod
                    .metadata
                    .name
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());

                let uid = pod
                    .metadata
                    .uid
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());

                let image = pod
                    .spec
                    .as_ref()
                    .and_then(|s| s.containers.first())
                    .and_then(|c| c.image.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let created_at = pod
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_else(crate::resources::now_rfc3339);

                Instance {
                    id: uid,
                    name,
                    runtime: RuntimeKind::Kubernetes,
                    image,
                    created_at,
                }
            })
            .collect();

        Ok(instances)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{
        AccessMode, BuildSpec, ConfigMapSpec, ConfigSpec, EnvFromSource, EnvSourceType,
        HealthProbe, HealthSpec, Metadata, MetricType, NetworkSpec, PersistenceSpec, PortMapping,
        ProbeType, ResourceRequirements, RuntimePreference, RuntimeSpec, RuntimeType,
        ScalingMetric, ScalingSpec, SecretSpec, ServiceType, Workload,
    };
    // Alias the spec-level types that conflict with k8s_openapi names
    use crate::spec::IngressPath as SpecIngressPath;
    use crate::spec::IngressSpec as SpecIngressSpec;
    use std::collections::HashMap;
    use std::path::PathBuf;

    // Tests call the standalone build_*_manifest functions directly.
    // No kube::Client, Tokio runtime, or network access is needed.

    fn create_test_image() -> Image {
        Image {
            name: "test-app".to_string(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::Kubernetes,
        }
    }

    fn create_test_workload() -> Workload {
        Workload {
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
                preferred: RuntimePreference::Kube,
                allow: vec![RuntimeType::Kube],
            },
            network: NetworkSpec {
                service: true,
                service_type: ServiceType::ClusterIP,
                ports: vec![PortMapping {
                    container_port: 80,
                    service_port: 8080,
                    protocol: "TCP".to_string(),
                }],
            },
            persistence: PersistenceSpec {
                enabled: false,
                size: String::new(),
                access_mode: AccessMode::ReadWriteOnce,
                storage_class: None,
            },
            health: None,
            config: None,
            ingress: None,
            scaling: None,
        }
    }

    // -----------------------------------------------------------------------
    // Pod generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_pod_basic() {
        let spec = create_test_workload();
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);

        assert_eq!(pod.metadata.name, Some("test-app".to_string()));
        assert_eq!(pod.metadata.namespace, Some("default".to_string()));

        let pod_spec = pod.spec.as_ref().unwrap();
        assert_eq!(pod_spec.containers.len(), 1);

        let container = &pod_spec.containers[0];
        assert_eq!(container.name, "test-app");
        assert_eq!(container.image, Some("test-app:latest".to_string()));
    }

    #[test]
    fn test_generate_pod_namespace() {
        let spec = create_test_workload();
        let image = create_test_image();

        let pod = build_pod_manifest("production", &image, &spec);
        assert_eq!(pod.metadata.namespace, Some("production".to_string()));
    }

    #[test]
    fn test_generate_pod_default_labels() {
        let spec = create_test_workload();
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let labels = pod.metadata.labels.as_ref().unwrap();

        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"orchestr8".to_string()));
    }

    #[test]
    fn test_generate_pod_user_labels_merged() {

        let mut spec = create_test_workload();
        spec.metadata.labels.insert("env".to_string(), "staging".to_string());
        spec.metadata.labels.insert("team".to_string(), "backend".to_string());
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let labels = pod.metadata.labels.as_ref().unwrap();

        // Default labels still present
        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"orchestr8".to_string()));
        // User labels merged in
        assert_eq!(labels.get("env"), Some(&"staging".to_string()));
        assert_eq!(labels.get("team"), Some(&"backend".to_string()));
    }

    #[test]
    fn test_generate_pod_annotations() {

        let mut spec = create_test_workload();
        spec.metadata.annotations.insert(
            "prometheus.io/scrape".to_string(),
            "true".to_string(),
        );
        spec.metadata.annotations.insert(
            "prometheus.io/port".to_string(),
            "9090".to_string(),
        );
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let annotations = pod.metadata.annotations.as_ref().unwrap();

        assert_eq!(annotations.get("prometheus.io/scrape"), Some(&"true".to_string()));
        assert_eq!(annotations.get("prometheus.io/port"), Some(&"9090".to_string()));
    }

    #[test]
    fn test_generate_pod_container_ports() {

        let mut spec = create_test_workload();
        spec.network.ports = vec![
            PortMapping {
                container_port: 80,
                service_port: 8080,
                protocol: "TCP".to_string(),
            },
            PortMapping {
                container_port: 443,
                service_port: 8443,
                protocol: "TCP".to_string(),
            },
            PortMapping {
                container_port: 5353,
                service_port: 53,
                protocol: "UDP".to_string(),
            },
        ];
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];
        let ports = container.ports.as_ref().unwrap();

        assert_eq!(ports.len(), 3);
        assert_eq!(ports[0].container_port, 80);
        assert_eq!(ports[0].protocol, Some("TCP".to_string()));
        assert_eq!(ports[1].container_port, 443);
        assert_eq!(ports[2].container_port, 5353);
        assert_eq!(ports[2].protocol, Some("UDP".to_string()));
    }

    #[test]
    fn test_generate_pod_no_ports() {

        let mut spec = create_test_workload();
        spec.network.ports = vec![];
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];
        let ports = container.ports.as_ref().unwrap();

        assert!(ports.is_empty());
    }

    #[test]
    fn test_generate_pod_resource_limits() {

        let mut spec = create_test_workload();
        spec.requirements.cpu = "500m".to_string();
        spec.requirements.memory = "256Mi".to_string();
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];
        let resources = container.resources.as_ref().unwrap();

        let limits = resources.limits.as_ref().unwrap();
        assert_eq!(limits.get("cpu").unwrap().0, "500m");
        assert_eq!(limits.get("memory").unwrap().0, "256Mi");

        let requests = resources.requests.as_ref().unwrap();
        assert_eq!(requests.get("cpu").unwrap().0, "500m");
        assert_eq!(requests.get("memory").unwrap().0, "256Mi");
    }

    #[test]
    fn test_generate_pod_resource_limits_whole_cpu() {

        let mut spec = create_test_workload();
        spec.requirements.cpu = "4".to_string();
        spec.requirements.memory = "8Gi".to_string();
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];
        let resources = container.resources.as_ref().unwrap();

        let limits = resources.limits.as_ref().unwrap();
        assert_eq!(limits.get("cpu").unwrap().0, "4");
        assert_eq!(limits.get("memory").unwrap().0, "8Gi");
    }

    #[test]
    fn test_generate_pod_image_with_digest() {

        let spec = create_test_workload();
        let image = Image {
            name: "myregistry.io/myapp".to_string(),
            tag: "v1.2.3".to_string(),
            digest: Some("sha256:abc123".to_string()),
            runtime: RuntimeKind::Kubernetes,
        };

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];
        // full_name() returns "name:tag"
        assert_eq!(container.image, Some("myregistry.io/myapp:v1.2.3".to_string()));
    }

    #[test]
    fn test_generate_pod_no_health_probes() {
        let spec = create_test_workload();
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];

        assert!(container.liveness_probe.is_none());
        assert!(container.readiness_probe.is_none());
    }

    #[test]
    fn test_generate_pod_no_env_from() {
        let spec = create_test_workload();
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];

        assert!(container.env_from.is_none());
    }

    // -----------------------------------------------------------------------
    // Health probe tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_pod_liveness_http_probe() {

        let mut spec = create_test_workload();
        spec.health = Some(HealthSpec {
            liveness: Some(HealthProbe {
                probe_type: ProbeType::HttpGet {
                    path: "/healthz".to_string(),
                    port: 8080,
                },
                initial_delay_seconds: 15,
                period_seconds: 20,
            }),
            readiness: None,
        });
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];

        let probe = container.liveness_probe.as_ref().unwrap();
        assert_eq!(probe.initial_delay_seconds, Some(15));
        assert_eq!(probe.period_seconds, Some(20));

        let http_get = probe.http_get.as_ref().unwrap();
        assert_eq!(http_get.path, Some("/healthz".to_string()));
        assert_eq!(http_get.port, IntOrString::Int(8080));
    }

    #[test]
    fn test_generate_pod_readiness_http_probe() {

        let mut spec = create_test_workload();
        spec.health = Some(HealthSpec {
            liveness: None,
            readiness: Some(HealthProbe {
                probe_type: ProbeType::HttpGet {
                    path: "/ready".to_string(),
                    port: 3000,
                },
                initial_delay_seconds: 5,
                period_seconds: 10,
            }),
        });
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];

        assert!(container.liveness_probe.is_none());

        let probe = container.readiness_probe.as_ref().unwrap();
        assert_eq!(probe.initial_delay_seconds, Some(5));
        assert_eq!(probe.period_seconds, Some(10));

        let http_get = probe.http_get.as_ref().unwrap();
        assert_eq!(http_get.path, Some("/ready".to_string()));
        assert_eq!(http_get.port, IntOrString::Int(3000));
    }

    #[test]
    fn test_generate_pod_both_probes() {

        let mut spec = create_test_workload();
        spec.health = Some(HealthSpec {
            liveness: Some(HealthProbe {
                probe_type: ProbeType::HttpGet {
                    path: "/healthz".to_string(),
                    port: 8080,
                },
                initial_delay_seconds: 30,
                period_seconds: 15,
            }),
            readiness: Some(HealthProbe {
                probe_type: ProbeType::HttpGet {
                    path: "/ready".to_string(),
                    port: 8080,
                },
                initial_delay_seconds: 5,
                period_seconds: 5,
            }),
        });
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];

        assert!(container.liveness_probe.is_some());
        assert!(container.readiness_probe.is_some());

        let liveness = container.liveness_probe.as_ref().unwrap();
        assert_eq!(liveness.initial_delay_seconds, Some(30));
        assert_eq!(
            liveness.http_get.as_ref().unwrap().path,
            Some("/healthz".to_string())
        );

        let readiness = container.readiness_probe.as_ref().unwrap();
        assert_eq!(readiness.initial_delay_seconds, Some(5));
        assert_eq!(
            readiness.http_get.as_ref().unwrap().path,
            Some("/ready".to_string())
        );
    }

    #[test]
    fn test_generate_pod_tcp_probe_produces_no_http_get() {

        let mut spec = create_test_workload();
        spec.health = Some(HealthSpec {
            liveness: Some(HealthProbe {
                probe_type: ProbeType::TcpSocket { port: 3306 },
                initial_delay_seconds: 10,
                period_seconds: 10,
            }),
            readiness: None,
        });
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];

        let probe = container.liveness_probe.as_ref().unwrap();
        // TcpSocket probe type falls into the _ => None arm for http_get
        assert!(probe.http_get.is_none());
        assert_eq!(probe.initial_delay_seconds, Some(10));
        assert_eq!(probe.period_seconds, Some(10));
    }

    #[test]
    fn test_generate_pod_exec_probe_produces_no_http_get() {

        let mut spec = create_test_workload();
        spec.health = Some(HealthSpec {
            liveness: Some(HealthProbe {
                probe_type: ProbeType::Exec {
                    command: vec!["cat".to_string(), "/tmp/healthy".to_string()],
                },
                initial_delay_seconds: 5,
                period_seconds: 5,
            }),
            readiness: None,
        });
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];

        let probe = container.liveness_probe.as_ref().unwrap();
        assert!(probe.http_get.is_none());
        assert_eq!(probe.initial_delay_seconds, Some(5));
    }

    // -----------------------------------------------------------------------
    // Environment variable tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_pod_env_from_configmap() {

        let mut spec = create_test_workload();
        spec.config = Some(ConfigSpec {
            config_maps: vec![],
            secrets: vec![],
            env_from: vec![EnvFromSource {
                source_type: EnvSourceType::ConfigMap,
                name: "app-config".to_string(),
            }],
        });
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];
        let env_from = container.env_from.as_ref().unwrap();

        assert_eq!(env_from.len(), 1);
        assert!(env_from[0].config_map_ref.is_some());
        assert!(env_from[0].secret_ref.is_none());
        assert_eq!(
            env_from[0].config_map_ref.as_ref().unwrap().name,
            "app-config".to_string()
        );
    }

    #[test]
    fn test_generate_pod_env_from_secret() {

        let mut spec = create_test_workload();
        spec.config = Some(ConfigSpec {
            config_maps: vec![],
            secrets: vec![],
            env_from: vec![EnvFromSource {
                source_type: EnvSourceType::Secret,
                name: "db-credentials".to_string(),
            }],
        });
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];
        let env_from = container.env_from.as_ref().unwrap();

        assert_eq!(env_from.len(), 1);
        assert!(env_from[0].secret_ref.is_some());
        assert!(env_from[0].config_map_ref.is_none());
        assert_eq!(
            env_from[0].secret_ref.as_ref().unwrap().name,
            "db-credentials".to_string()
        );
    }

    #[test]
    fn test_generate_pod_env_from_multiple_sources() {

        let mut spec = create_test_workload();
        spec.config = Some(ConfigSpec {
            config_maps: vec![],
            secrets: vec![],
            env_from: vec![
                EnvFromSource {
                    source_type: EnvSourceType::ConfigMap,
                    name: "app-config".to_string(),
                },
                EnvFromSource {
                    source_type: EnvSourceType::Secret,
                    name: "app-secrets".to_string(),
                },
                EnvFromSource {
                    source_type: EnvSourceType::ConfigMap,
                    name: "shared-config".to_string(),
                },
            ],
        });
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];
        let env_from = container.env_from.as_ref().unwrap();

        assert_eq!(env_from.len(), 3);
        assert!(env_from[0].config_map_ref.is_some());
        assert!(env_from[1].secret_ref.is_some());
        assert!(env_from[2].config_map_ref.is_some());
    }

    #[test]
    fn test_generate_pod_env_from_empty_list() {

        let mut spec = create_test_workload();
        spec.config = Some(ConfigSpec {
            config_maps: vec![],
            secrets: vec![],
            env_from: vec![],
        });
        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        let container = &pod.spec.as_ref().unwrap().containers[0];
        // Empty env_from vec results in None
        assert!(container.env_from.is_none());
    }

    // -----------------------------------------------------------------------
    // Service generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_service_clusterip() {

        let spec = create_test_workload();

        let service = build_service_manifest("default", &spec).unwrap();

        assert_eq!(service.metadata.name, Some("test-app-service".to_string()));
        assert_eq!(service.metadata.namespace, Some("default".to_string()));

        let svc_spec = service.spec.as_ref().unwrap();
        assert_eq!(svc_spec.type_, Some("ClusterIP".to_string()));

        let selector = svc_spec.selector.as_ref().unwrap();
        assert_eq!(selector.get("app"), Some(&"test-app".to_string()));
    }

    #[test]
    fn test_generate_service_loadbalancer() {

        let mut spec = create_test_workload();
        spec.network.service_type = ServiceType::LoadBalancer;

        let service = build_service_manifest("default", &spec).unwrap();
        let svc_spec = service.spec.as_ref().unwrap();
        assert_eq!(svc_spec.type_, Some("LoadBalancer".to_string()));
    }

    #[test]
    fn test_generate_service_nodeport() {

        let mut spec = create_test_workload();
        spec.network.service_type = ServiceType::NodePort;

        let service = build_service_manifest("default", &spec).unwrap();
        let svc_spec = service.spec.as_ref().unwrap();
        assert_eq!(svc_spec.type_, Some("NodePort".to_string()));
    }

    #[test]
    fn test_generate_service_disabled() {

        let mut spec = create_test_workload();
        spec.network.service = false;

        let service = build_service_manifest("default", &spec);
        assert!(service.is_none());
    }

    #[test]
    fn test_generate_service_ports() {

        let mut spec = create_test_workload();
        spec.network.ports = vec![
            PortMapping {
                container_port: 80,
                service_port: 8080,
                protocol: "TCP".to_string(),
            },
            PortMapping {
                container_port: 443,
                service_port: 8443,
                protocol: "TCP".to_string(),
            },
        ];

        let service = build_service_manifest("default", &spec).unwrap();
        let ports = service.spec.as_ref().unwrap().ports.as_ref().unwrap();

        assert_eq!(ports.len(), 2);

        assert_eq!(ports[0].port, 8080);
        assert_eq!(ports[0].target_port, Some(IntOrString::Int(80)));
        assert_eq!(ports[0].protocol, Some("TCP".to_string()));

        assert_eq!(ports[1].port, 8443);
        assert_eq!(ports[1].target_port, Some(IntOrString::Int(443)));
    }

    #[test]
    fn test_generate_service_labels() {
        let spec = create_test_workload();

        let service = build_service_manifest("staging", &spec).unwrap();
        let labels = service.metadata.labels.as_ref().unwrap();

        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"orchestr8".to_string()));
        assert_eq!(service.metadata.namespace, Some("staging".to_string()));
    }

    #[test]
    fn test_generate_service_udp_protocol() {

        let mut spec = create_test_workload();
        spec.network.ports = vec![PortMapping {
            container_port: 5353,
            service_port: 53,
            protocol: "UDP".to_string(),
        }];

        let service = build_service_manifest("default", &spec).unwrap();
        let ports = service.spec.as_ref().unwrap().ports.as_ref().unwrap();
        assert_eq!(ports[0].protocol, Some("UDP".to_string()));
        assert_eq!(ports[0].port, 53);
        assert_eq!(ports[0].target_port, Some(IntOrString::Int(5353)));
    }

    // -----------------------------------------------------------------------
    // PVC generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_pvc_disabled() {

        let spec = create_test_workload();

        let pvc = build_pvc_manifest("default", &spec);
        assert!(pvc.is_none());
    }

    #[test]
    fn test_generate_pvc_readwriteonce() {

        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        spec.persistence.size = "10Gi".to_string();
        spec.persistence.access_mode = AccessMode::ReadWriteOnce;

        let pvc = build_pvc_manifest("default", &spec).unwrap();

        assert_eq!(pvc.metadata.name, Some("test-app-pvc".to_string()));
        assert_eq!(pvc.metadata.namespace, Some("default".to_string()));

        let pvc_spec = pvc.spec.as_ref().unwrap();
        assert_eq!(
            pvc_spec.access_modes.as_ref().unwrap(),
            &vec!["ReadWriteOnce".to_string()]
        );

        let requests = pvc_spec.resources.as_ref().unwrap().requests.as_ref().unwrap();
        assert_eq!(requests.get("storage").unwrap().0, "10Gi");
    }

    #[test]
    fn test_generate_pvc_readonlymany() {

        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        spec.persistence.size = "5Gi".to_string();
        spec.persistence.access_mode = AccessMode::ReadOnlyMany;

        let pvc = build_pvc_manifest("default", &spec).unwrap();
        let pvc_spec = pvc.spec.as_ref().unwrap();
        assert_eq!(
            pvc_spec.access_modes.as_ref().unwrap(),
            &vec!["ReadOnlyMany".to_string()]
        );
    }

    #[test]
    fn test_generate_pvc_readwritemany() {

        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        spec.persistence.size = "100Gi".to_string();
        spec.persistence.access_mode = AccessMode::ReadWriteMany;

        let pvc = build_pvc_manifest("default", &spec).unwrap();
        let pvc_spec = pvc.spec.as_ref().unwrap();
        assert_eq!(
            pvc_spec.access_modes.as_ref().unwrap(),
            &vec!["ReadWriteMany".to_string()]
        );
        let requests = pvc_spec.resources.as_ref().unwrap().requests.as_ref().unwrap();
        assert_eq!(requests.get("storage").unwrap().0, "100Gi");
    }

    #[test]
    fn test_generate_pvc_with_storage_class() {

        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        spec.persistence.size = "50Gi".to_string();
        spec.persistence.storage_class = Some("fast-ssd".to_string());

        let pvc = build_pvc_manifest("default", &spec).unwrap();
        let pvc_spec = pvc.spec.as_ref().unwrap();
        assert_eq!(pvc_spec.storage_class_name, Some("fast-ssd".to_string()));
    }

    #[test]
    fn test_generate_pvc_without_storage_class() {

        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        spec.persistence.size = "10Gi".to_string();
        spec.persistence.storage_class = None;

        let pvc = build_pvc_manifest("default", &spec).unwrap();
        let pvc_spec = pvc.spec.as_ref().unwrap();
        assert!(pvc_spec.storage_class_name.is_none());
    }

    #[test]
    fn test_generate_pvc_labels() {

        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        spec.persistence.size = "1Gi".to_string();

        let pvc = build_pvc_manifest("production", &spec).unwrap();
        let labels = pvc.metadata.labels.as_ref().unwrap();

        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"orchestr8".to_string()));
        assert_eq!(pvc.metadata.namespace, Some("production".to_string()));
    }

    // -----------------------------------------------------------------------
    // ConfigMap generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_configmaps_none() {

        let spec = create_test_workload();

        let cms = build_configmap_manifests("default", &spec);
        assert!(cms.is_empty());
    }

    #[test]
    fn test_generate_configmaps_empty_list() {

        let mut spec = create_test_workload();
        spec.config = Some(ConfigSpec {
            config_maps: vec![],
            secrets: vec![],
            env_from: vec![],
        });

        let cms = build_configmap_manifests("default", &spec);
        assert!(cms.is_empty());
    }

    #[test]
    fn test_generate_configmaps_single() {

        let mut spec = create_test_workload();
        let mut data = HashMap::new();
        data.insert("DATABASE_URL".to_string(), "postgres://localhost/db".to_string());
        data.insert("LOG_LEVEL".to_string(), "info".to_string());

        spec.config = Some(ConfigSpec {
            config_maps: vec![ConfigMapSpec {
                name: "app-config".to_string(),
                data,
                mount_path: None,
            }],
            secrets: vec![],
            env_from: vec![],
        });

        let cms = build_configmap_manifests("default", &spec);
        assert_eq!(cms.len(), 1);

        let cm = &cms[0];
        assert_eq!(cm.metadata.name, Some("app-config".to_string()));
        assert_eq!(cm.metadata.namespace, Some("default".to_string()));

        let cm_data = cm.data.as_ref().unwrap();
        assert_eq!(
            cm_data.get("DATABASE_URL"),
            Some(&"postgres://localhost/db".to_string())
        );
        assert_eq!(cm_data.get("LOG_LEVEL"), Some(&"info".to_string()));

        let labels = cm.metadata.labels.as_ref().unwrap();
        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"orchestr8".to_string()));
    }

    #[test]
    fn test_generate_configmaps_multiple() {

        let mut spec = create_test_workload();
        let mut data1 = HashMap::new();
        data1.insert("key1".to_string(), "val1".to_string());
        let mut data2 = HashMap::new();
        data2.insert("key2".to_string(), "val2".to_string());

        spec.config = Some(ConfigSpec {
            config_maps: vec![
                ConfigMapSpec {
                    name: "cm-one".to_string(),
                    data: data1,
                    mount_path: None,
                },
                ConfigMapSpec {
                    name: "cm-two".to_string(),
                    data: data2,
                    mount_path: Some("/etc/config".to_string()),
                },
            ],
            secrets: vec![],
            env_from: vec![],
        });

        let cms = build_configmap_manifests("default", &spec);
        assert_eq!(cms.len(), 2);
        assert_eq!(cms[0].metadata.name, Some("cm-one".to_string()));
        assert_eq!(cms[1].metadata.name, Some("cm-two".to_string()));
    }

    // -----------------------------------------------------------------------
    // Secret generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_secrets_none() {

        let spec = create_test_workload();

        let secrets = build_secret_manifests("default", &spec);
        assert!(secrets.is_empty());
    }

    #[test]
    fn test_generate_secrets_single() {

        let mut spec = create_test_workload();
        let mut data = HashMap::new();
        data.insert("password".to_string(), "s3cret".to_string());
        data.insert("api_key".to_string(), "abc123".to_string());

        spec.config = Some(ConfigSpec {
            config_maps: vec![],
            secrets: vec![SecretSpec {
                name: "db-creds".to_string(),
                data,
                mount_path: None,
            }],
            env_from: vec![],
        });

        let secrets = build_secret_manifests("default", &spec);
        assert_eq!(secrets.len(), 1);

        let secret = &secrets[0];
        assert_eq!(secret.metadata.name, Some("db-creds".to_string()));
        assert_eq!(secret.metadata.namespace, Some("default".to_string()));
        assert_eq!(secret.type_, Some("Opaque".to_string()));

        let string_data = secret.string_data.as_ref().unwrap();
        assert_eq!(string_data.get("password"), Some(&"s3cret".to_string()));
        assert_eq!(string_data.get("api_key"), Some(&"abc123".to_string()));

        let labels = secret.metadata.labels.as_ref().unwrap();
        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
    }

    #[test]
    fn test_generate_secrets_multiple() {

        let mut spec = create_test_workload();
        let mut data1 = HashMap::new();
        data1.insert("key".to_string(), "val".to_string());
        let mut data2 = HashMap::new();
        data2.insert("token".to_string(), "xyz".to_string());

        spec.config = Some(ConfigSpec {
            config_maps: vec![],
            secrets: vec![
                SecretSpec {
                    name: "secret-one".to_string(),
                    data: data1,
                    mount_path: None,
                },
                SecretSpec {
                    name: "secret-two".to_string(),
                    data: data2,
                    mount_path: Some("/etc/secrets".to_string()),
                },
            ],
            env_from: vec![],
        });

        let secrets = build_secret_manifests("default", &spec);
        assert_eq!(secrets.len(), 2);
        assert_eq!(secrets[0].metadata.name, Some("secret-one".to_string()));
        assert_eq!(secrets[1].metadata.name, Some("secret-two".to_string()));
    }

    // -----------------------------------------------------------------------
    // Ingress generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_ingress_disabled_none() {

        let spec = create_test_workload();

        let ingress = build_ingress_manifest("default", &spec);
        assert!(ingress.is_none());
    }

    #[test]
    fn test_generate_ingress_disabled_explicit() {

        let mut spec = create_test_workload();
        spec.ingress = Some(SpecIngressSpec {
            enabled: false,
            host: "example.com".to_string(),
            paths: vec![],
            tls: false,
            annotations: HashMap::new(),
        });

        let ingress = build_ingress_manifest("default", &spec);
        assert!(ingress.is_none());
    }

    #[test]
    fn test_generate_ingress_basic() {

        let mut spec = create_test_workload();
        spec.ingress = Some(SpecIngressSpec {
            enabled: true,
            host: "myapp.example.com".to_string(),
            paths: vec![SpecIngressPath {
                path: "/".to_string(),
                path_type: "Prefix".to_string(),
                port: 8080,
            }],
            tls: false,
            annotations: HashMap::new(),
        });

        let ingress = build_ingress_manifest("default", &spec).unwrap();

        assert_eq!(ingress.metadata.name, Some("test-app-ingress".to_string()));
        assert_eq!(ingress.metadata.namespace, Some("default".to_string()));

        let ingress_spec = ingress.spec.as_ref().unwrap();
        let rules = ingress_spec.rules.as_ref().unwrap();
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].host, Some("myapp.example.com".to_string()));

        let http_paths: &Vec<HTTPIngressPath> = &rules[0].http.as_ref().unwrap().paths;
        assert_eq!(http_paths.len(), 1);
        assert_eq!(http_paths[0].path, Some("/".to_string()));
        assert_eq!(http_paths[0].path_type, "Prefix");

        let backend_svc = http_paths[0].backend.service.as_ref().unwrap();
        assert_eq!(backend_svc.name, "test-app-service");
        assert_eq!(backend_svc.port.as_ref().unwrap().number, Some(8080));

        // No TLS
        assert!(ingress_spec.tls.is_none());
    }

    #[test]
    fn test_generate_ingress_with_tls() {

        let mut spec = create_test_workload();
        spec.ingress = Some(SpecIngressSpec {
            enabled: true,
            host: "secure.example.com".to_string(),
            paths: vec![SpecIngressPath {
                path: "/".to_string(),
                path_type: "Prefix".to_string(),
                port: 443,
            }],
            tls: true,
            annotations: HashMap::new(),
        });

        let ingress = build_ingress_manifest("default", &spec).unwrap();
        let ingress_spec = ingress.spec.as_ref().unwrap();

        let tls = ingress_spec.tls.as_ref().unwrap();
        assert_eq!(tls.len(), 1);
        assert_eq!(
            tls[0].hosts.as_ref().unwrap(),
            &vec!["secure.example.com".to_string()]
        );
        assert_eq!(tls[0].secret_name, Some("test-app-tls".to_string()));
    }

    #[test]
    fn test_generate_ingress_multiple_paths() {

        let mut spec = create_test_workload();
        spec.ingress = Some(SpecIngressSpec {
            enabled: true,
            host: "api.example.com".to_string(),
            paths: vec![
                SpecIngressPath {
                    path: "/api".to_string(),
                    path_type: "Prefix".to_string(),
                    port: 8080,
                },
                SpecIngressPath {
                    path: "/health".to_string(),
                    path_type: "Exact".to_string(),
                    port: 8081,
                },
                SpecIngressPath {
                    path: "/metrics".to_string(),
                    path_type: "Prefix".to_string(),
                    port: 9090,
                },
            ],
            tls: false,
            annotations: HashMap::new(),
        });

        let ingress = build_ingress_manifest("default", &spec).unwrap();
        let rules = ingress.spec.as_ref().unwrap().rules.as_ref().unwrap();
        let paths = &rules[0].http.as_ref().unwrap().paths;

        assert_eq!(paths.len(), 3);
        assert_eq!(paths[0].path, Some("/api".to_string()));
        assert_eq!(paths[0].path_type, "Prefix");
        assert_eq!(
            paths[0].backend.service.as_ref().unwrap().port.as_ref().unwrap().number,
            Some(8080)
        );
        assert_eq!(paths[1].path, Some("/health".to_string()));
        assert_eq!(paths[1].path_type, "Exact");
        assert_eq!(paths[2].path, Some("/metrics".to_string()));
    }

    #[test]
    fn test_generate_ingress_annotations() {

        let mut spec = create_test_workload();
        let mut annotations = HashMap::new();
        annotations.insert(
            "nginx.ingress.kubernetes.io/rewrite-target".to_string(),
            "/".to_string(),
        );
        annotations.insert(
            "cert-manager.io/cluster-issuer".to_string(),
            "letsencrypt-prod".to_string(),
        );

        spec.ingress = Some(SpecIngressSpec {
            enabled: true,
            host: "app.example.com".to_string(),
            paths: vec![SpecIngressPath {
                path: "/".to_string(),
                path_type: "Prefix".to_string(),
                port: 80,
            }],
            tls: true,
            annotations,
        });

        let ingress = build_ingress_manifest("default", &spec).unwrap();
        let ing_annotations = ingress.metadata.annotations.as_ref().unwrap();

        assert_eq!(
            ing_annotations.get("nginx.ingress.kubernetes.io/rewrite-target"),
            Some(&"/".to_string())
        );
        assert_eq!(
            ing_annotations.get("cert-manager.io/cluster-issuer"),
            Some(&"letsencrypt-prod".to_string())
        );
    }

    #[test]
    fn test_generate_ingress_labels() {

        let mut spec = create_test_workload();
        spec.ingress = Some(SpecIngressSpec {
            enabled: true,
            host: "app.example.com".to_string(),
            paths: vec![SpecIngressPath {
                path: "/".to_string(),
                path_type: "Prefix".to_string(),
                port: 80,
            }],
            tls: false,
            annotations: HashMap::new(),
        });

        let ingress = build_ingress_manifest("production", &spec).unwrap();
        let labels = ingress.metadata.labels.as_ref().unwrap();
        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"orchestr8".to_string()));
        assert_eq!(ingress.metadata.namespace, Some("production".to_string()));
    }

    // -----------------------------------------------------------------------
    // HPA generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_hpa_disabled_none() {

        let spec = create_test_workload();

        let hpa = build_hpa_manifest("default", &spec);
        assert!(hpa.is_none());
    }

    #[test]
    fn test_generate_hpa_disabled_explicit() {

        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: false,
            min_replicas: 1,
            max_replicas: 5,
            metrics: vec![],
        });

        let hpa = build_hpa_manifest("default", &spec);
        assert!(hpa.is_none());
    }

    #[test]
    fn test_generate_hpa_cpu_metric() {

        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 2,
            max_replicas: 10,
            metrics: vec![ScalingMetric {
                metric_type: MetricType::CPU,
                target_value: "80%".to_string(),
            }],
        });

        let hpa = build_hpa_manifest("default", &spec).unwrap();

        assert_eq!(hpa.metadata.name, Some("test-app-hpa".to_string()));
        assert_eq!(hpa.metadata.namespace, Some("default".to_string()));

        let hpa_spec = hpa.spec.as_ref().unwrap();
        assert_eq!(hpa_spec.min_replicas, Some(2));
        assert_eq!(hpa_spec.max_replicas, 10);

        // Verify scale target ref
        assert_eq!(hpa_spec.scale_target_ref.kind, "Deployment");
        assert_eq!(hpa_spec.scale_target_ref.name, "test-app");
        assert_eq!(hpa_spec.scale_target_ref.api_version, Some("apps/v1".to_string()));

        let metrics = hpa_spec.metrics.as_ref().unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].type_, "Resource");
        let resource = metrics[0].resource.as_ref().unwrap();
        assert_eq!(resource.name, "cpu");
        assert_eq!(resource.target.type_, "Utilization");
        assert_eq!(resource.target.average_utilization, Some(80));
    }

    #[test]
    fn test_generate_hpa_memory_metric() {

        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 5,
            metrics: vec![ScalingMetric {
                metric_type: MetricType::Memory,
                target_value: "70%".to_string(),
            }],
        });

        let hpa = build_hpa_manifest("default", &spec).unwrap();
        let hpa_spec = hpa.spec.as_ref().unwrap();
        let metrics = hpa_spec.metrics.as_ref().unwrap();

        assert_eq!(metrics.len(), 1);
        let resource = metrics[0].resource.as_ref().unwrap();
        assert_eq!(resource.name, "memory");
        assert_eq!(resource.target.average_utilization, Some(70));
    }

    #[test]
    fn test_generate_hpa_cpu_and_memory_metrics() {

        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 3,
            max_replicas: 20,
            metrics: vec![
                ScalingMetric {
                    metric_type: MetricType::CPU,
                    target_value: "75".to_string(),
                },
                ScalingMetric {
                    metric_type: MetricType::Memory,
                    target_value: "85".to_string(),
                },
            ],
        });

        let hpa = build_hpa_manifest("default", &spec).unwrap();
        let hpa_spec = hpa.spec.as_ref().unwrap();

        assert_eq!(hpa_spec.min_replicas, Some(3));
        assert_eq!(hpa_spec.max_replicas, 20);

        let metrics = hpa_spec.metrics.as_ref().unwrap();
        assert_eq!(metrics.len(), 2);

        let cpu = metrics[0].resource.as_ref().unwrap();
        assert_eq!(cpu.name, "cpu");
        assert_eq!(cpu.target.average_utilization, Some(75));

        let mem = metrics[1].resource.as_ref().unwrap();
        assert_eq!(mem.name, "memory");
        assert_eq!(mem.target.average_utilization, Some(85));
    }

    #[test]
    fn test_generate_hpa_custom_metric_filtered_out() {

        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 10,
            metrics: vec![
                ScalingMetric {
                    metric_type: MetricType::CPU,
                    target_value: "80".to_string(),
                },
                ScalingMetric {
                    metric_type: MetricType::Custom,
                    target_value: "100".to_string(),
                },
            ],
        });

        let hpa = build_hpa_manifest("default", &spec).unwrap();
        let metrics = hpa.spec.as_ref().unwrap().metrics.as_ref().unwrap();
        // Custom metrics are filtered out (not yet implemented)
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].resource.as_ref().unwrap().name, "cpu");
    }

    #[test]
    fn test_generate_hpa_labels() {

        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 3,
            metrics: vec![ScalingMetric {
                metric_type: MetricType::CPU,
                target_value: "50".to_string(),
            }],
        });

        let hpa = build_hpa_manifest("staging", &spec).unwrap();
        let labels = hpa.metadata.labels.as_ref().unwrap();
        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"orchestr8".to_string()));
        assert_eq!(hpa.metadata.namespace, Some("staging".to_string()));
    }

    #[test]
    fn test_generate_hpa_target_value_without_percent() {

        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 5,
            metrics: vec![ScalingMetric {
                metric_type: MetricType::CPU,
                target_value: "90".to_string(),
            }],
        });

        let hpa = build_hpa_manifest("default", &spec).unwrap();
        let metrics = hpa.spec.as_ref().unwrap().metrics.as_ref().unwrap();
        assert_eq!(
            metrics[0].resource.as_ref().unwrap().target.average_utilization,
            Some(90)
        );
    }

    // -----------------------------------------------------------------------
    // Combined / integration-style tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_full_workload_pod_with_all_features() {

        let mut spec = create_test_workload();

        // Set labels and annotations
        spec.metadata.labels.insert("version".to_string(), "v2".to_string());
        spec.metadata.annotations.insert(
            "description".to_string(),
            "Production workload".to_string(),
        );

        // Set resources
        spec.requirements.cpu = "1000m".to_string();
        spec.requirements.memory = "2Gi".to_string();

        // Set health probes
        spec.health = Some(HealthSpec {
            liveness: Some(HealthProbe {
                probe_type: ProbeType::HttpGet {
                    path: "/healthz".to_string(),
                    port: 8080,
                },
                initial_delay_seconds: 30,
                period_seconds: 10,
            }),
            readiness: Some(HealthProbe {
                probe_type: ProbeType::HttpGet {
                    path: "/ready".to_string(),
                    port: 8080,
                },
                initial_delay_seconds: 5,
                period_seconds: 5,
            }),
        });

        // Set env from
        spec.config = Some(ConfigSpec {
            config_maps: vec![ConfigMapSpec {
                name: "app-config".to_string(),
                data: {
                    let mut m = HashMap::new();
                    m.insert("KEY".to_string(), "value".to_string());
                    m
                },
                mount_path: None,
            }],
            secrets: vec![SecretSpec {
                name: "app-secret".to_string(),
                data: {
                    let mut m = HashMap::new();
                    m.insert("SECRET".to_string(), "hidden".to_string());
                    m
                },
                mount_path: None,
            }],
            env_from: vec![
                EnvFromSource {
                    source_type: EnvSourceType::ConfigMap,
                    name: "app-config".to_string(),
                },
                EnvFromSource {
                    source_type: EnvSourceType::Secret,
                    name: "app-secret".to_string(),
                },
            ],
        });

        // Set ports
        spec.network.ports = vec![
            PortMapping {
                container_port: 8080,
                service_port: 80,
                protocol: "TCP".to_string(),
            },
            PortMapping {
                container_port: 9090,
                service_port: 9090,
                protocol: "TCP".to_string(),
            },
        ];

        let image = Image {
            name: "registry.example.com/myapp".to_string(),
            tag: "v2.0.0".to_string(),
            digest: None,
            runtime: RuntimeKind::Kubernetes,
        };

        let pod = build_pod_manifest("production", &image, &spec);

        // Verify metadata
        assert_eq!(pod.metadata.namespace, Some("production".to_string()));
        let labels = pod.metadata.labels.as_ref().unwrap();
        assert_eq!(labels.get("version"), Some(&"v2".to_string()));

        // Verify container
        let container = &pod.spec.as_ref().unwrap().containers[0];
        assert_eq!(
            container.image,
            Some("registry.example.com/myapp:v2.0.0".to_string())
        );

        // Verify resources
        let resources = container.resources.as_ref().unwrap();
        assert_eq!(resources.limits.as_ref().unwrap().get("cpu").unwrap().0, "1000m");
        assert_eq!(resources.limits.as_ref().unwrap().get("memory").unwrap().0, "2Gi");

        // Verify probes
        assert!(container.liveness_probe.is_some());
        assert!(container.readiness_probe.is_some());

        // Verify env_from
        let env_from = container.env_from.as_ref().unwrap();
        assert_eq!(env_from.len(), 2);

        // Verify ports
        let ports = container.ports.as_ref().unwrap();
        assert_eq!(ports.len(), 2);
    }

    #[test]
    fn test_full_workload_all_resources_generated() {

        let mut spec = create_test_workload();

        // Enable persistence
        spec.persistence = PersistenceSpec {
            enabled: true,
            size: "50Gi".to_string(),
            access_mode: AccessMode::ReadWriteOnce,
            storage_class: Some("gp2".to_string()),
        };

        // Enable ingress
        spec.ingress = Some(SpecIngressSpec {
            enabled: true,
            host: "app.example.com".to_string(),
            paths: vec![SpecIngressPath {
                path: "/".to_string(),
                path_type: "Prefix".to_string(),
                port: 80,
            }],
            tls: true,
            annotations: HashMap::new(),
        });

        // Enable scaling
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 2,
            max_replicas: 8,
            metrics: vec![ScalingMetric {
                metric_type: MetricType::CPU,
                target_value: "70%".to_string(),
            }],
        });

        // Add config
        spec.config = Some(ConfigSpec {
            config_maps: vec![ConfigMapSpec {
                name: "my-cm".to_string(),
                data: {
                    let mut m = HashMap::new();
                    m.insert("k".to_string(), "v".to_string());
                    m
                },
                mount_path: None,
            }],
            secrets: vec![SecretSpec {
                name: "my-secret".to_string(),
                data: {
                    let mut m = HashMap::new();
                    m.insert("s".to_string(), "hidden".to_string());
                    m
                },
                mount_path: None,
            }],
            env_from: vec![],
        });

        let image = create_test_image();

        // All generation methods should succeed
        let pod = build_pod_manifest("default", &image, &spec);
        assert!(pod.metadata.name.is_some());

        let service = build_service_manifest("default", &spec);
        assert!(service.is_some());

        let pvc = build_pvc_manifest("default", &spec);
        assert!(pvc.is_some());
        assert_eq!(
            pvc.as_ref().unwrap().spec.as_ref().unwrap().storage_class_name,
            Some("gp2".to_string())
        );

        let ingress = build_ingress_manifest("default", &spec);
        assert!(ingress.is_some());
        assert!(ingress.as_ref().unwrap().spec.as_ref().unwrap().tls.is_some());

        let hpa = build_hpa_manifest("default", &spec);
        assert!(hpa.is_some());

        let cms = build_configmap_manifests("default", &spec);
        assert_eq!(cms.len(), 1);

        let secrets = build_secret_manifests("default", &spec);
        assert_eq!(secrets.len(), 1);
    }

    #[test]
    fn test_minimal_workload_no_optional_resources() {

        let mut spec = create_test_workload();
        spec.network.service = false;
        spec.persistence.enabled = false;
        spec.ingress = None;
        spec.scaling = None;
        spec.config = None;
        spec.health = None;

        let image = create_test_image();

        let pod = build_pod_manifest("default", &image, &spec);
        assert!(pod.metadata.name.is_some());

        assert!(build_service_manifest("default", &spec).is_none());
        assert!(build_pvc_manifest("default", &spec).is_none());
        assert!(build_ingress_manifest("default", &spec).is_none());
        assert!(build_hpa_manifest("default", &spec).is_none());
        assert!(build_configmap_manifests("default", &spec).is_empty());
        assert!(build_secret_manifests("default", &spec).is_empty());
    }

    // ── sanitize_volume_name ─────────────────────────────────────────

    #[test]
    fn test_sanitize_volume_name_valid_input() {
        assert_eq!(sanitize_volume_name("cm-app-config"), "cm-app-config");
    }

    #[test]
    fn test_sanitize_volume_name_uppercase() {
        assert_eq!(sanitize_volume_name("CM-MyConfig"), "cm-myconfig");
    }

    #[test]
    fn test_sanitize_volume_name_underscores() {
        assert_eq!(sanitize_volume_name("cm-my_config"), "cm-my-config");
    }

    #[test]
    fn test_sanitize_volume_name_dots_and_special() {
        assert_eq!(sanitize_volume_name("secret-tls.crt@v2"), "secret-tls-crt-v2");
    }

    #[test]
    fn test_sanitize_volume_name_leading_trailing_hyphens() {
        assert_eq!(sanitize_volume_name("--name--"), "name");
    }

    #[test]
    fn test_sanitize_volume_name_all_invalid_chars() {
        assert_eq!(sanitize_volume_name("___"), "vol");
    }

    #[test]
    fn test_sanitize_volume_name_truncates_to_63() {
        let long_name = "a".repeat(100);
        let result = sanitize_volume_name(&long_name);
        assert_eq!(result.len(), 63);
    }

    #[test]
    fn test_sanitize_volume_name_empty() {
        assert_eq!(sanitize_volume_name(""), "vol");
    }

    // ── volume mount generation ──────────────────────────────────────

    #[test]
    fn test_pod_manifest_with_configmap_volume_mount() {
        let mut spec = create_test_workload();
        spec.config = Some(ConfigSpec {
            config_maps: vec![ConfigMapSpec {
                name: "app-config".to_string(),
                data: std::collections::HashMap::new(),
                mount_path: Some("/etc/app".to_string()),
            }],
            secrets: vec![],
            env_from: vec![],
        });

        let image = create_test_image();
        let pod = build_pod_manifest("default", &image, &spec);

        let pod_spec = pod.spec.as_ref().unwrap();
        let volumes = pod_spec.volumes.as_ref().unwrap();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0].name, "cm-app-config");
        assert!(volumes[0].config_map.is_some());

        let container = &pod_spec.containers[0];
        let mounts = container.volume_mounts.as_ref().unwrap();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].name, "cm-app-config");
        assert_eq!(mounts[0].mount_path, "/etc/app");
        assert_eq!(mounts[0].read_only, Some(true));
    }

    #[test]
    fn test_pod_manifest_with_pvc_volume_mount() {
        let mut spec = create_test_workload();
        spec.persistence = PersistenceSpec {
            enabled: true,
            size: "10Gi".to_string(),
            access_mode: AccessMode::ReadWriteOnce,
            storage_class: Some("standard".to_string()),
        };

        let image = create_test_image();
        let pod = build_pod_manifest("default", &image, &spec);

        let pod_spec = pod.spec.as_ref().unwrap();
        let volumes = pod_spec.volumes.as_ref().unwrap();
        assert_eq!(volumes.len(), 1);
        assert!(volumes[0].persistent_volume_claim.is_some());

        let container = &pod_spec.containers[0];
        let mounts = container.volume_mounts.as_ref().unwrap();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].mount_path, "/data");
    }

    #[test]
    fn test_pod_manifest_no_volumes_when_no_mount_path() {
        let mut spec = create_test_workload();
        spec.config = Some(ConfigSpec {
            config_maps: vec![ConfigMapSpec {
                name: "app-config".to_string(),
                data: [("key".to_string(), "val".to_string())].into_iter().collect(),
                mount_path: None, // No mount_path means env var injection only
            }],
            secrets: vec![],
            env_from: vec![],
        });

        let image = create_test_image();
        let pod = build_pod_manifest("default", &image, &spec);

        let pod_spec = pod.spec.as_ref().unwrap();
        assert!(pod_spec.volumes.is_none());
    }
}
