//! Kubernetes runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::{AccessMode, Workload};
use async_trait::async_trait;
use k8s_openapi::api::autoscaling::v2::{
    HorizontalPodAutoscaler, HorizontalPodAutoscalerSpec, MetricIdentifier, MetricSpec,
    MetricTarget, PodsMetricSource, ResourceMetricSource,
};
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec};
use k8s_openapi::api::batch::v1::{
    CronJob, CronJobSpec, JobSpec, JobTemplateSpec,
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
    NetworkPolicy, NetworkPolicyEgressRule, NetworkPolicyIngressRule, NetworkPolicyPeer,
    NetworkPolicySpec,
};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
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
    /// Clean up resources created during a failed deployment.
    /// Best-effort: logs errors but doesn't propagate them.
    async fn cleanup_resources(&self, resources: &[(&str, String)]) {
        for (kind, name) in resources {
            let result = match *kind {
                "configmap" => {
                    let api: Api<ConfigMap> = Api::namespaced(self.client.clone(), &self.namespace);
                    api.delete(name, &DeleteParams::default()).await.map(|_| ())
                }
                "secret" => {
                    let api: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);
                    api.delete(name, &DeleteParams::default()).await.map(|_| ())
                }
                "pvc" => {
                    let api: Api<PersistentVolumeClaim> = Api::namespaced(self.client.clone(), &self.namespace);
                    api.delete(name, &DeleteParams::default()).await.map(|_| ())
                }
                "service" => {
                    let api: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);
                    api.delete(name, &DeleteParams::default()).await.map(|_| ())
                }
                "ingress" => {
                    let api: Api<Ingress> = Api::namespaced(self.client.clone(), &self.namespace);
                    api.delete(name, &DeleteParams::default()).await.map(|_| ())
                }
                "deployment" => {
                    let api: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);
                    api.delete(name, &DeleteParams::default()).await.map(|_| ())
                }
                "cronjob" => {
                    let api: Api<CronJob> = Api::namespaced(self.client.clone(), &self.namespace);
                    api.delete(name, &DeleteParams::default()).await.map(|_| ())
                }
                "networkpolicy" => {
                    let api: Api<NetworkPolicy> = Api::namespaced(self.client.clone(), &self.namespace);
                    api.delete(name, &DeleteParams::default()).await.map(|_| ())
                }
                _ => Ok(()),
            };
            match result {
                Ok(()) => tracing::info!("Cleaned up {} '{}'", kind, name),
                Err(e) => tracing::warn!("Failed to clean up {} '{}': {}", kind, name, e),
            }
        }
    }

    /// Get pod status by looking up pods via label selectors.
    /// Works with Deployments where pod names include generated hashes.
    async fn get_pod_status(&self, name: &str) -> anyhow::Result<Status> {
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams::default().labels(&format!("app={},managed-by=aether", name));

        match pods.list(&lp).await {
            Ok(pod_list) => {
                if let Some(pod) = pod_list.items.first() {
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
                } else {
                    Ok(Status {
                        state: InstanceState::Unknown,
                        ready: false,
                        message: Some("No pods found for workload".to_string()),
                        restart_count: 0,
                    })
                }
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

/// Default timeout for Kubernetes API calls (5 minutes).
const KUBE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(300);

/// Default graceful termination period for workloads (30 seconds).
const KUBE_GRACE_PERIOD: u32 = 30;

/// Create DeleteParams with graceful termination period.
fn graceful_delete_params() -> DeleteParams {
    DeleteParams {
        grace_period_seconds: Some(KUBE_GRACE_PERIOD),
        ..Default::default()
    }
}

/// Execute a Kubernetes API future with a timeout.
async fn kube_with_timeout<F, T>(op: &str, fut: F) -> crate::Result<T>
where
    F: std::future::Future<Output = Result<T, kube::Error>>,
{
    tokio::time::timeout(KUBE_TIMEOUT, fut)
        .await
        .map_err(|_| anyhow::anyhow!("Kubernetes {} timed out after {:?}", op, KUBE_TIMEOUT))?
        .map_err(Into::into)
}

/// Check if a kube error is a 409 Conflict (resource already exists)
fn is_already_exists(err: &kube::Error) -> bool {
    matches!(err, kube::Error::Api(resp) if resp.code == 409)
}

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

/// Build Kubernetes resource requirements from workload spec.
///
/// Supports burstable QoS: when `cpu_request`/`memory_request` are set and
/// differ from the limits, Kubernetes assigns the Burstable QoS class.
fn build_resource_requirements(spec: &Workload) -> K8sResourceRequirements {
    let mut limits = BTreeMap::new();
    let mut requests = BTreeMap::new();

    limits.insert("cpu".to_string(), Quantity(spec.requirements.cpu.clone()));
    limits.insert("memory".to_string(), Quantity(spec.requirements.memory.clone()));

    let cpu_req = spec.requirements.cpu_request.as_deref()
        .unwrap_or(&spec.requirements.cpu);
    let mem_req = spec.requirements.memory_request.as_deref()
        .unwrap_or(&spec.requirements.memory);
    requests.insert("cpu".to_string(), Quantity(cpu_req.to_string()));
    requests.insert("memory".to_string(), Quantity(mem_req.to_string()));

    // GPU resources
    if let Some(ref gpu) = spec.requirements.gpu {
        let gpu_key = format!("{}.com/gpu", gpu.vendor);
        limits.insert(gpu_key.clone(), Quantity(gpu.count.to_string()));
        requests.insert(gpu_key, Quantity(gpu.count.to_string()));
    }

    K8sResourceRequirements {
        limits: Some(limits),
        requests: Some(requests),
        ..Default::default()
    }
}

/// Build a Deployment manifest from workload spec.
fn build_deployment_manifest(namespace: &str, image: &Image, spec: &Workload) -> Deployment {
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "aether".to_string());

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

    let resources = build_resource_requirements(spec);

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

    // Determine replica count: use scaling min_replicas if configured, otherwise 1
    let replicas = spec
        .scaling
        .as_ref()
        .filter(|s| s.enabled)
        .map(|s| s.min_replicas as i32)
        .unwrap_or(1);

    let mut match_labels = BTreeMap::new();
    match_labels.insert("app".to_string(), spec.metadata.name.clone());
    match_labels.insert("managed-by".to_string(), "aether".to_string());

    // Build pod template annotations from workload metadata + mesh config
    let mut pod_annotations: BTreeMap<String, String> = spec.metadata.annotations.clone().into_iter().collect();

    if let Some(mesh) = &spec.mesh {
        match mesh.provider.as_str() {
            "istio" => {
                pod_annotations.insert("sidecar.istio.io/inject".to_string(), mesh.inject.to_string());
            }
            "linkerd" => {
                pod_annotations.insert("linkerd.io/inject".to_string(), if mesh.inject { "enabled" } else { "disabled" }.to_string());
            }
            "consul" => {
                pod_annotations.insert("consul.hashicorp.com/connect-inject".to_string(), mesh.inject.to_string());
            }
            _ => {
                tracing::warn!("Unknown mesh provider '{}', adding custom annotations only", mesh.provider);
            }
        }
        // Add any custom mesh annotations
        for (k, v) in &mesh.annotations {
            pod_annotations.insert(k.clone(), v.clone());
        }
    }

    Deployment {
        metadata: ObjectMeta {
            name: Some(spec.metadata.name.clone()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels.clone()),
            annotations: Some(spec.metadata.annotations.clone().into_iter().collect()),
            ..Default::default()
        },
        spec: Some(DeploymentSpec {
            replicas: Some(replicas),
            selector: LabelSelector {
                match_labels: Some(match_labels),
                ..Default::default()
            },
            template: k8s_openapi::api::core::v1::PodTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(labels),
                    annotations: Some(pod_annotations),
                    ..Default::default()
                }),
                spec: Some(PodSpec {
                    containers: vec![container],
                    volumes: if volumes.is_empty() { None } else { Some(volumes) },
                    ..Default::default()
                }),
            },
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
    labels.insert("managed-by".to_string(), "aether".to_string());

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

/// Parse "key=value" label strings into NetworkPolicyPeer selectors.
fn parse_label_peers(labels: &[String], direction: &str) -> Vec<NetworkPolicyPeer> {
    labels
        .iter()
        .filter_map(|label| {
            let parts: Vec<&str> = label.splitn(2, '=').collect();
            if parts.len() == 2 {
                let mut match_labels = BTreeMap::new();
                match_labels.insert(parts[0].to_string(), parts[1].to_string());
                Some(NetworkPolicyPeer {
                    pod_selector: Some(LabelSelector {
                        match_labels: Some(match_labels),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
            } else {
                tracing::warn!("Invalid {} label '{}', expected key=value", direction, label);
                None
            }
        })
        .collect()
}

/// Build a NetworkPolicy manifest from workload spec.
fn build_networkpolicy_manifest(namespace: &str, spec: &Workload) -> Option<NetworkPolicy> {
    let np_config = spec.network.network_policy.as_ref()?;

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "aether".to_string());

    let pod_selector = LabelSelector {
        match_labels: Some(labels.clone()),
        ..Default::default()
    };

    // Build ingress rules from allow_from labels
    let ingress = if np_config.deny_all_ingress || !np_config.allow_from.is_empty() {
        let peers = parse_label_peers(&np_config.allow_from, "allow_from");
        if peers.is_empty() {
            // deny_all with no allow_from → empty rules = deny all
            Some(vec![])
        } else {
            Some(vec![NetworkPolicyIngressRule {
                from: Some(peers),
                ..Default::default()
            }])
        }
    } else {
        None
    };

    // Build egress rules from allow_to labels
    let egress = if np_config.deny_all_egress || !np_config.allow_to.is_empty() {
        let peers = parse_label_peers(&np_config.allow_to, "allow_to");
        if peers.is_empty() {
            Some(vec![])
        } else {
            Some(vec![NetworkPolicyEgressRule {
                to: Some(peers),
                ..Default::default()
            }])
        }
    } else {
        None
    };

    // Determine policy types
    let mut policy_types = Vec::new();
    if ingress.is_some() {
        policy_types.push("Ingress".to_string());
    }
    if egress.is_some() {
        policy_types.push("Egress".to_string());
    }

    Some(NetworkPolicy {
        metadata: ObjectMeta {
            name: Some(format!("{}-netpol", spec.metadata.name)),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        spec: Some(NetworkPolicySpec {
            pod_selector,
            ingress,
            egress,
            policy_types: Some(policy_types),
        }),
    })
}

/// Build a PersistentVolumeClaim manifest from workload spec.
fn build_pvc_manifest(namespace: &str, spec: &Workload) -> Option<PersistentVolumeClaim> {
    if !spec.persistence.enabled {
        return None;
    }

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "aether".to_string());

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
    labels.insert("managed-by".to_string(), "aether".to_string());

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
    labels.insert("managed-by".to_string(), "aether".to_string());

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
    labels.insert("managed-by".to_string(), "aether".to_string());

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
    labels.insert("managed-by".to_string(), "aether".to_string());

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
            crate::spec::MetricType::Custom => {
                let metric_name = match &m.metric_name {
                    Some(name) if !name.is_empty() => name.clone(),
                    _ => {
                        tracing::warn!(
                            "Custom metric missing metric_name for '{}', skipping",
                            spec.metadata.name
                        );
                        return None;
                    }
                };

                let target_value: i32 = match m.target_value.trim_end_matches('%').parse() {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!(
                            value = %m.target_value,
                            "invalid custom metric HPA target value, skipping"
                        );
                        return None;
                    }
                };

                Some(MetricSpec {
                    type_: "Pods".to_string(),
                    pods: Some(PodsMetricSource {
                        metric: MetricIdentifier {
                            name: metric_name,
                            selector: None,
                        },
                        target: MetricTarget {
                            type_: "AverageValue".to_string(),
                            average_value: Some(Quantity(target_value.to_string())),
                            ..Default::default()
                        },
                    }),
                    ..Default::default()
                })
            }
        })
        .collect();

    // Don't create an HPA with no valid metrics
    if metrics.is_empty() {
        tracing::warn!("No valid HPA metrics for '{}', skipping HPA creation", spec.metadata.name);
        return None;
    }

    // Validate min/max replicas
    if scaling_spec.min_replicas > scaling_spec.max_replicas {
        tracing::error!(
            "Invalid scaling spec for '{}': min_replicas ({}) > max_replicas ({}). \
             Fix the workload spec. Skipping HPA creation.",
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

/// Build a CronJob manifest from workload spec.
/// Used when `spec.schedule` is set, creating a Kubernetes CronJob instead of a Deployment.
fn build_cronjob_manifest(namespace: &str, image: &Image, spec: &Workload) -> CronJob {
    let schedule_spec = spec.schedule.as_ref().expect("schedule must be set for CronJob");

    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "aether".to_string());

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

    let resources = build_resource_requirements(spec);

    // Environment variables from ConfigMaps
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

    let restart_policy = match schedule_spec.restart_policy {
        crate::spec::JobRestartPolicy::Never => "Never",
        crate::spec::JobRestartPolicy::OnFailure => "OnFailure",
    };

    let container = Container {
        name: spec.metadata.name.clone(),
        image: Some(image.full_name()),
        ports: if ports.is_empty() { None } else { Some(ports) },
        resources: Some(resources),
        env: if inline_env.is_empty() { None } else { Some(inline_env) },
        ..Default::default()
    };

    let concurrency_policy = match schedule_spec.concurrency_policy {
        crate::spec::ConcurrencyPolicy::Allow => "Allow",
        crate::spec::ConcurrencyPolicy::Forbid => "Forbid",
        crate::spec::ConcurrencyPolicy::Replace => "Replace",
    };

    CronJob {
        metadata: ObjectMeta {
            name: Some(spec.metadata.name.clone()),
            namespace: Some(namespace.to_string()),
            labels: Some(labels.clone()),
            annotations: Some(spec.metadata.annotations.clone().into_iter().collect()),
            ..Default::default()
        },
        spec: Some(CronJobSpec {
            schedule: schedule_spec.cron.clone(),
            concurrency_policy: Some(concurrency_policy.to_string()),
            job_template: JobTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(labels.clone()),
                    ..Default::default()
                }),
                spec: Some(JobSpec {
                    backoff_limit: Some(schedule_spec.backoff_limit as i32),
                    active_deadline_seconds: schedule_spec.active_deadline_seconds,
                    template: k8s_openapi::api::core::v1::PodTemplateSpec {
                        metadata: Some(ObjectMeta {
                            labels: Some(labels),
                            ..Default::default()
                        }),
                        spec: Some(PodSpec {
                            containers: vec![container],
                            restart_policy: Some(restart_policy.to_string()),
                            ..Default::default()
                        }),
                    },
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    }
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
        // Validate workload name and namespace are valid Kubernetes DNS labels
        validate_kube_name(&spec.metadata.name)?;
        validate_kube_name(&self.namespace)?;

        tracing::info!(
            "Deploying to Kubernetes namespace '{}': {}",
            self.namespace,
            spec.metadata.name
        );

        // Track created resources for logging
        let mut _created_configmaps: Vec<String> = Vec::new();
        let mut _created_secrets: Vec<String> = Vec::new();

        // Track resources created in this deployment for cleanup on failure
        let mut new_resources: Vec<(&str, String)> = Vec::new();

        // Create ConfigMaps
        for configmap in build_configmap_manifests(&self.namespace, spec) {
            let configmaps: Api<ConfigMap> =
                Api::namespaced(self.client.clone(), &self.namespace);
            let cm_name = configmap.metadata.name.clone().unwrap_or_default();

            match kube_with_timeout("ConfigMap create", configmaps.create(&PostParams::default(), &configmap)).await {
                Ok(_) => {
                    tracing::info!("Created ConfigMap: {}", cm_name);
                    new_resources.push(("configmap", cm_name));
                }
                Err(e) if e.downcast_ref::<kube::Error>().is_some_and(is_already_exists) => {
                    tracing::info!("ConfigMap already exists: {}", cm_name);
                }
                Err(e) => {
                    tracing::error!("ConfigMap creation failed: {}", e);
                    self.cleanup_resources(&new_resources).await;
                    return Err(e);
                }
            }
        }

        // Create Secrets
        for secret in build_secret_manifests(&self.namespace, spec) {
            let secrets: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);
            let secret_name = secret.metadata.name.clone().unwrap_or_default();

            match kube_with_timeout("Secret create", secrets.create(&PostParams::default(), &secret)).await {
                Ok(_) => {
                    tracing::info!("Created Secret: {}", secret_name);
                    new_resources.push(("secret", secret_name));
                }
                Err(e) if e.downcast_ref::<kube::Error>().is_some_and(is_already_exists) => {
                    tracing::info!("Secret already exists: {}", secret_name);
                }
                Err(e) => {
                    tracing::error!("Secret creation failed: {}", e);
                    self.cleanup_resources(&new_resources).await;
                    return Err(e);
                }
            }
        }

        // Create PVC if needed
        if let Some(pvc) = build_pvc_manifest(&self.namespace, spec) {
            let pvcs: Api<PersistentVolumeClaim> =
                Api::namespaced(self.client.clone(), &self.namespace);

            match kube_with_timeout("PVC create", pvcs.create(&PostParams::default(), &pvc)).await {
                Ok(_) => { tracing::info!("Created PVC: {}-pvc", spec.metadata.name); new_resources.push(("pvc", format!("{}-pvc", spec.metadata.name))); }
                Err(e) if e.downcast_ref::<kube::Error>().is_some_and(is_already_exists) => tracing::info!("PVC already exists: {}-pvc", spec.metadata.name),
                Err(e) => { tracing::error!("PVC creation failed: {}", e); self.cleanup_resources(&new_resources).await; return Err(e); }
            }
        }

        // Create Service if needed
        if let Some(service) = build_service_manifest(&self.namespace, spec) {
            let services: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);

            match kube_with_timeout("Service create", services.create(&PostParams::default(), &service)).await {
                Ok(_) => { tracing::info!("Created Service: {}-service", spec.metadata.name); new_resources.push(("service", format!("{}-service", spec.metadata.name))); }
                Err(e) if e.downcast_ref::<kube::Error>().is_some_and(is_already_exists) => tracing::info!("Service already exists: {}-service", spec.metadata.name),
                Err(e) => { tracing::error!("Service creation failed: {}", e); self.cleanup_resources(&new_resources).await; return Err(e); }
            }
        }

        // Create Ingress if needed
        if let Some(ingress) = build_ingress_manifest(&self.namespace, spec) {
            let ingresses: Api<Ingress> = Api::namespaced(self.client.clone(), &self.namespace);

            match kube_with_timeout("Ingress create", ingresses.create(&PostParams::default(), &ingress)).await {
                Ok(_) => { tracing::info!("Created Ingress: {}-ingress", spec.metadata.name); new_resources.push(("ingress", format!("{}-ingress", spec.metadata.name))); }
                Err(e) if e.downcast_ref::<kube::Error>().is_some_and(is_already_exists) => tracing::info!("Ingress already exists: {}-ingress", spec.metadata.name),
                Err(e) => { tracing::error!("Ingress creation failed: {}", e); self.cleanup_resources(&new_resources).await; return Err(e); }
            }
        }

        // Create NetworkPolicy if needed
        if let Some(netpol) = build_networkpolicy_manifest(&self.namespace, spec) {
            let netpols: Api<NetworkPolicy> = Api::namespaced(self.client.clone(), &self.namespace);
            let netpol_name = format!("{}-netpol", spec.metadata.name);

            match kube_with_timeout("NetworkPolicy create", netpols.create(&PostParams::default(), &netpol)).await {
                Ok(_) => { tracing::info!("Created NetworkPolicy: {}", netpol_name); new_resources.push(("networkpolicy", netpol_name)); }
                Err(e) if e.downcast_ref::<kube::Error>().is_some_and(is_already_exists) => tracing::info!("NetworkPolicy already exists: {}", netpol_name),
                Err(e) => { tracing::error!("NetworkPolicy creation failed: {}", e); self.cleanup_resources(&new_resources).await; return Err(e); }
            }
        }

        // Create CronJob or Deployment depending on schedule
        let (resource_name, resource_uid) = if spec.schedule.is_some() {
            // Scheduled workload → CronJob
            let cronjob = build_cronjob_manifest(&self.namespace, image, spec);
            let cronjobs: Api<CronJob> = Api::namespaced(self.client.clone(), &self.namespace);

            let created = match kube_with_timeout("CronJob create", cronjobs.create(&PostParams::default(), &cronjob)).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("CronJob creation failed, cleaning up {} resources", new_resources.len());
                    self.cleanup_resources(&new_resources).await;
                    return Err(e);
                }
            };

            let name = created.metadata.name.unwrap_or_else(|| spec.metadata.name.clone());
            let uid = created.metadata.uid.unwrap_or_else(|| "unknown".to_string());
            new_resources.push(("cronjob", name.clone()));
            tracing::info!("Created CronJob: {}", name);
            (name, uid)
        } else {
            // Long-running workload → Deployment
            let deployment = build_deployment_manifest(&self.namespace, image, spec);
            let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);

            let created = match kube_with_timeout("Deployment create", deployments.create(&PostParams::default(), &deployment)).await {
                Ok(d) => d,
                Err(e) => {
                    tracing::error!("Deployment creation failed, cleaning up {} resources", new_resources.len());
                    self.cleanup_resources(&new_resources).await;
                    return Err(e);
                }
            };

            let name = created.metadata.name.unwrap_or_else(|| spec.metadata.name.clone());
            let uid = created.metadata.uid.unwrap_or_else(|| "unknown".to_string());
            new_resources.push(("deployment", name.clone()));
            tracing::info!("Created Deployment: {}", name);

            // Create HPA if needed (non-critical, don't clean up on failure)
            // HPA does not apply to CronJobs, so only create for Deployments
            if let Some(hpa) = build_hpa_manifest(&self.namespace, spec) {
                let hpas: Api<HorizontalPodAutoscaler> =
                    Api::namespaced(self.client.clone(), &self.namespace);

                match kube_with_timeout("HPA create", hpas.create(&PostParams::default(), &hpa)).await {
                    Ok(_) => tracing::info!("Created HPA: {}-hpa", spec.metadata.name),
                    Err(e) if e.downcast_ref::<kube::Error>().is_some_and(is_already_exists) => tracing::info!("HPA already exists: {}-hpa", spec.metadata.name),
                    Err(e) => tracing::warn!("HPA creation failed: {}", e),
                }
            }

            (name, uid)
        };

        Ok(Instance::new(resource_uid, resource_name, RuntimeKind::Kubernetes, image.full_name()))
    }

    async fn stop(&self, instance: &Instance) -> crate::Result<()> {
        validate_kube_name(&instance.name)?;
        tracing::info!("Stopping (deleting) Deployment: {}", instance.name);

        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);
        let dp = DeleteParams {
            grace_period_seconds: Some(30),
            ..Default::default()
        };
        kube_with_timeout("Deployment delete", deployments.delete(&instance.name, &dp)).await?;

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

        // Look up pods by label since Deployment pods have generated names
        let lp = ListParams::default().labels(&format!("app={},managed-by=aether", instance.name));
        let pod_list = pods.list(&lp).await?;
        let pod_name = pod_list.items.first()
            .and_then(|p| p.metadata.name.clone())
            .ok_or_else(|| anyhow::anyhow!("No pods found for workload '{}'", instance.name))?;

        let logs = kube_with_timeout("Pod logs", pods.logs(&pod_name, &log_params)).await?;
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

        // Delete Deployment (cascades to ReplicaSet and Pods)
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);
        match deployments.delete(&instance.name, &graceful_delete_params()).await {
            Ok(_) => tracing::info!("Deleted Deployment: {}", instance.name),
            Err(e) => tracing::debug!("Deployment deletion failed (may not exist): {}", e),
        }

        // Delete CronJob (may exist if workload was scheduled)
        let cronjobs: Api<CronJob> = Api::namespaced(self.client.clone(), &self.namespace);
        match cronjobs.delete(&instance.name, &DeleteParams::default()).await {
            Ok(_) => tracing::info!("Deleted CronJob: {}", instance.name),
            Err(e) => tracing::debug!("CronJob deletion failed (may not exist): {}", e),
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

        // Delete NetworkPolicy
        let netpols: Api<NetworkPolicy> = Api::namespaced(self.client.clone(), &self.namespace);
        let netpol_name = format!("{}-netpol", instance.name);
        match netpols.delete(&netpol_name, &DeleteParams::default()).await {
            Ok(_) => tracing::info!("Deleted NetworkPolicy: {}", netpol_name),
            Err(e) => tracing::debug!("NetworkPolicy deletion failed (may not exist): {}", e),
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

        // Delete ConfigMaps and Secrets managed by aether
        // We'll use label selectors to find and delete them
        validate_kube_name(&instance.name)?;
        let lp = ListParams::default().labels(&format!(
            "app={},managed-by=aether",
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
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);

        // List only deployments managed by aether
        let lp = ListParams::default().labels("managed-by=aether");
        let deploy_list = deployments.list(&lp).await?;

        let instances: Vec<Instance> = deploy_list
            .items
            .iter()
            .map(|deploy| {
                let name = deploy
                    .metadata
                    .name
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());

                let uid = deploy
                    .metadata
                    .uid
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());

                let image = deploy
                    .spec
                    .as_ref()
                    .and_then(|s| s.template.spec.as_ref())
                    .and_then(|ps| ps.containers.first())
                    .and_then(|c| c.image.clone())
                    .unwrap_or_else(|| "unknown".to_string());

                let created_at = deploy
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

    async fn update(&self, instance: &Instance, image: &Image, spec: &Workload) -> crate::Result<Instance> {
        validate_kube_name(&spec.metadata.name)?;
        tracing::info!("Updating Deployment: {}", instance.name);

        let deployment = build_deployment_manifest(&self.namespace, image, spec);
        let deployments: Api<Deployment> = Api::namespaced(self.client.clone(), &self.namespace);

        let patch = kube::api::Patch::Apply(deployment);
        let pp = kube::api::PatchParams::apply("aether").force();
        let patched = kube_with_timeout("Deployment patch", deployments.patch(&instance.name, &pp, &patch)).await?;

        let uid = patched.metadata.uid.unwrap_or_else(|| "unknown".to_string());
        let name = patched.metadata.name.unwrap_or_else(|| instance.name.clone());

        tracing::info!("Updated Deployment: {} (rolling update triggered)", name);
        Ok(Instance::new(uid, name, RuntimeKind::Kubernetes, image.full_name()))
    }

    async fn capacity(&self) -> crate::Result<Option<crate::runtime::Capacity>> {
        use k8s_openapi::api::core::v1::Node;

        let nodes: Api<Node> = Api::all(self.client.clone());
        match nodes.list(&ListParams::default()).await {
            Ok(node_list) => {
                let mut total_cpu = 0.0_f64;
                let mut total_memory_mb = 0_u64;
                let mut allocatable_cpu = 0.0_f64;
                let mut allocatable_memory_mb = 0_u64;

                for node in &node_list.items {
                    if let Some(status) = &node.status {
                        if let Some(allocatable) = &status.allocatable {
                            if let Some(cpu) = allocatable.get("cpu") {
                                allocatable_cpu += crate::resources::parse_cpu(&cpu.0);
                            }
                            if let Some(mem) = allocatable.get("memory") {
                                let gi = crate::resources::parse_memory_gi(&mem.0);
                                allocatable_memory_mb += (gi * 1024.0) as u64;
                            }
                        }
                        if let Some(cap) = &status.capacity {
                            if let Some(cpu) = cap.get("cpu") {
                                total_cpu += crate::resources::parse_cpu(&cpu.0);
                            }
                            if let Some(mem) = cap.get("memory") {
                                let gi = crate::resources::parse_memory_gi(&mem.0);
                                total_memory_mb += (gi * 1024.0) as u64;
                            }
                        }
                    }
                }

                Ok(Some(crate::runtime::Capacity {
                    total_cpu,
                    available_cpu: allocatable_cpu,
                    total_memory_mb,
                    available_memory_mb: allocatable_memory_mb,
                }))
            }
            Err(e) => {
                tracing::warn!("Failed to probe Kubernetes node capacity: {}", e);
                Ok(None)
            }
        }
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
                network_policy: None,
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
            mesh: None,
            intent: None,
            schedule: None,
        }
    }

    // -----------------------------------------------------------------------
    // Deployment generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_deployment_basic() {
        let spec = create_test_workload();
        let image = create_test_image();

        let deploy = build_deployment_manifest("default", &image, &spec);

        assert_eq!(deploy.metadata.name, Some("test-app".to_string()));
        assert_eq!(deploy.metadata.namespace, Some("default".to_string()));

        let deploy_spec = deploy.spec.as_ref().unwrap();
        assert_eq!(deploy_spec.replicas, Some(1));

        let pod_spec = deploy_spec.template.spec.as_ref().unwrap();
        assert_eq!(pod_spec.containers.len(), 1);

        let container = &pod_spec.containers[0];
        assert_eq!(container.name, "test-app");
        assert_eq!(container.image, Some("test-app:latest".to_string()));

        // Verify selector
        let match_labels = deploy_spec.selector.match_labels.as_ref().unwrap();
        assert_eq!(match_labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(match_labels.get("managed-by"), Some(&"aether".to_string()));
    }

    #[test]
    fn test_generate_deployment_namespace() {
        let spec = create_test_workload();
        let image = create_test_image();

        let deploy = build_deployment_manifest("production", &image, &spec);
        assert_eq!(deploy.metadata.namespace, Some("production".to_string()));
    }

    #[test]
    fn test_generate_deployment_default_labels() {
        let spec = create_test_workload();
        let image = create_test_image();

        let deploy = build_deployment_manifest("default", &image, &spec);
        let labels = deploy.metadata.labels.as_ref().unwrap();

        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"aether".to_string()));
    }

    #[test]
    fn test_generate_deployment_user_labels_merged() {

        let mut spec = create_test_workload();
        spec.metadata.labels.insert("env".to_string(), "staging".to_string());
        spec.metadata.labels.insert("team".to_string(), "backend".to_string());
        let image = create_test_image();

        let deploy = build_deployment_manifest("default", &image, &spec);
        let labels = deploy.metadata.labels.as_ref().unwrap();

        // Default labels still present
        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"aether".to_string()));
        // User labels merged in
        assert_eq!(labels.get("env"), Some(&"staging".to_string()));
        assert_eq!(labels.get("team"), Some(&"backend".to_string()));
    }

    #[test]
    fn test_generate_deployment_annotations() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let annotations = deploy.metadata.annotations.as_ref().unwrap();

        assert_eq!(annotations.get("prometheus.io/scrape"), Some(&"true".to_string()));
        assert_eq!(annotations.get("prometheus.io/port"), Some(&"9090".to_string()));
    }

    #[test]
    fn test_generate_deployment_container_ports() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];
        let ports = container.ports.as_ref().unwrap();

        assert_eq!(ports.len(), 3);
        assert_eq!(ports[0].container_port, 80);
        assert_eq!(ports[0].protocol, Some("TCP".to_string()));
        assert_eq!(ports[1].container_port, 443);
        assert_eq!(ports[2].container_port, 5353);
        assert_eq!(ports[2].protocol, Some("UDP".to_string()));
    }

    #[test]
    fn test_generate_deployment_no_ports() {

        let mut spec = create_test_workload();
        spec.network.ports = vec![];
        let image = create_test_image();

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];
        let ports = container.ports.as_ref().unwrap();

        assert!(ports.is_empty());
    }

    #[test]
    fn test_generate_deployment_resource_limits() {

        let mut spec = create_test_workload();
        spec.requirements.cpu = "500m".to_string();
        spec.requirements.memory = "256Mi".to_string();
        let image = create_test_image();

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];
        let resources = container.resources.as_ref().unwrap();

        let limits = resources.limits.as_ref().unwrap();
        assert_eq!(limits.get("cpu").unwrap().0, "500m");
        assert_eq!(limits.get("memory").unwrap().0, "256Mi");

        let requests = resources.requests.as_ref().unwrap();
        assert_eq!(requests.get("cpu").unwrap().0, "500m");
        assert_eq!(requests.get("memory").unwrap().0, "256Mi");
    }

    #[test]
    fn test_generate_deployment_resource_limits_whole_cpu() {

        let mut spec = create_test_workload();
        spec.requirements.cpu = "4".to_string();
        spec.requirements.memory = "8Gi".to_string();
        let image = create_test_image();

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];
        let resources = container.resources.as_ref().unwrap();

        let limits = resources.limits.as_ref().unwrap();
        assert_eq!(limits.get("cpu").unwrap().0, "4");
        assert_eq!(limits.get("memory").unwrap().0, "8Gi");
    }

    #[test]
    fn test_generate_deployment_image_with_digest() {

        let spec = create_test_workload();
        let image = Image {
            name: "myregistry.io/myapp".to_string(),
            tag: "v1.2.3".to_string(),
            digest: Some("sha256:abc123".to_string()),
            runtime: RuntimeKind::Kubernetes,
        };

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];
        // full_name() returns "name:tag"
        assert_eq!(container.image, Some("myregistry.io/myapp:v1.2.3".to_string()));
    }

    #[test]
    fn test_generate_deployment_no_health_probes() {
        let spec = create_test_workload();
        let image = create_test_image();

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];

        assert!(container.liveness_probe.is_none());
        assert!(container.readiness_probe.is_none());
    }

    #[test]
    fn test_generate_deployment_no_env_from() {
        let spec = create_test_workload();
        let image = create_test_image();

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];

        assert!(container.env_from.is_none());
    }

    // -----------------------------------------------------------------------
    // Health probe tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_deployment_liveness_http_probe() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];

        let probe = container.liveness_probe.as_ref().unwrap();
        assert_eq!(probe.initial_delay_seconds, Some(15));
        assert_eq!(probe.period_seconds, Some(20));

        let http_get = probe.http_get.as_ref().unwrap();
        assert_eq!(http_get.path, Some("/healthz".to_string()));
        assert_eq!(http_get.port, IntOrString::Int(8080));
    }

    #[test]
    fn test_generate_deployment_readiness_http_probe() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];

        assert!(container.liveness_probe.is_none());

        let probe = container.readiness_probe.as_ref().unwrap();
        assert_eq!(probe.initial_delay_seconds, Some(5));
        assert_eq!(probe.period_seconds, Some(10));

        let http_get = probe.http_get.as_ref().unwrap();
        assert_eq!(http_get.path, Some("/ready".to_string()));
        assert_eq!(http_get.port, IntOrString::Int(3000));
    }

    #[test]
    fn test_generate_deployment_both_probes() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];

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
    fn test_generate_deployment_tcp_probe_produces_no_http_get() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];

        let probe = container.liveness_probe.as_ref().unwrap();
        // TcpSocket probe type falls into the _ => None arm for http_get
        assert!(probe.http_get.is_none());
        assert_eq!(probe.initial_delay_seconds, Some(10));
        assert_eq!(probe.period_seconds, Some(10));
    }

    #[test]
    fn test_generate_deployment_exec_probe_produces_no_http_get() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];

        let probe = container.liveness_probe.as_ref().unwrap();
        assert!(probe.http_get.is_none());
        assert_eq!(probe.initial_delay_seconds, Some(5));
    }

    // -----------------------------------------------------------------------
    // Environment variable tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_deployment_env_from_configmap() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];
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
    fn test_generate_deployment_env_from_secret() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];
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
    fn test_generate_deployment_env_from_multiple_sources() {

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];
        let env_from = container.env_from.as_ref().unwrap();

        assert_eq!(env_from.len(), 3);
        assert!(env_from[0].config_map_ref.is_some());
        assert!(env_from[1].secret_ref.is_some());
        assert!(env_from[2].config_map_ref.is_some());
    }

    #[test]
    fn test_generate_deployment_env_from_empty_list() {

        let mut spec = create_test_workload();
        spec.config = Some(ConfigSpec {
            config_maps: vec![],
            secrets: vec![],
            env_from: vec![],
        });
        let image = create_test_image();

        let deploy = build_deployment_manifest("default", &image, &spec);
        let container = &deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap().containers[0];
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
        assert_eq!(labels.get("managed-by"), Some(&"aether".to_string()));
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
        assert_eq!(labels.get("managed-by"), Some(&"aether".to_string()));
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
        assert_eq!(labels.get("managed-by"), Some(&"aether".to_string()));
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
        assert_eq!(labels.get("managed-by"), Some(&"aether".to_string()));
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
                metric_name: None,
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
                metric_name: None,
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
                    metric_name: None,
                },
                ScalingMetric {
                    metric_type: MetricType::Memory,
                    target_value: "85".to_string(),
                    metric_name: None,
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
                    metric_name: None,
                },
                ScalingMetric {
                    metric_type: MetricType::Custom,
                    target_value: "100".to_string(),
                    metric_name: None,
                },
            ],
        });

        let hpa = build_hpa_manifest("default", &spec).unwrap();
        let metrics = hpa.spec.as_ref().unwrap().metrics.as_ref().unwrap();
        // Custom metrics without metric_name are filtered out
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
                metric_name: None,
            }],
        });

        let hpa = build_hpa_manifest("staging", &spec).unwrap();
        let labels = hpa.metadata.labels.as_ref().unwrap();
        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"aether".to_string()));
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
                metric_name: None,
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
    fn test_full_workload_deployment_with_all_features() {

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

        let deploy = build_deployment_manifest("production", &image, &spec);

        // Verify metadata
        assert_eq!(deploy.metadata.namespace, Some("production".to_string()));
        let labels = deploy.metadata.labels.as_ref().unwrap();
        assert_eq!(labels.get("version"), Some(&"v2".to_string()));

        // Verify deployment spec
        let deploy_spec = deploy.spec.as_ref().unwrap();
        assert_eq!(deploy_spec.replicas, Some(1));

        // Verify container
        let container = &deploy_spec.template.spec.as_ref().unwrap().containers[0];
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
                metric_name: None,
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
        let deploy = build_deployment_manifest("default", &image, &spec);
        assert!(deploy.metadata.name.is_some());
        assert_eq!(deploy.spec.as_ref().unwrap().replicas, Some(2));

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

        let deploy = build_deployment_manifest("default", &image, &spec);
        assert!(deploy.metadata.name.is_some());
        assert_eq!(deploy.spec.as_ref().unwrap().replicas, Some(1));

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
    fn test_deployment_manifest_with_configmap_volume_mount() {
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
        let deploy = build_deployment_manifest("default", &image, &spec);

        let pod_spec = deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap();
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
    fn test_deployment_manifest_with_pvc_volume_mount() {
        let mut spec = create_test_workload();
        spec.persistence = PersistenceSpec {
            enabled: true,
            size: "10Gi".to_string(),
            access_mode: AccessMode::ReadWriteOnce,
            storage_class: Some("standard".to_string()),
        };

        let image = create_test_image();
        let deploy = build_deployment_manifest("default", &image, &spec);

        let pod_spec = deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap();
        let volumes = pod_spec.volumes.as_ref().unwrap();
        assert_eq!(volumes.len(), 1);
        assert!(volumes[0].persistent_volume_claim.is_some());

        let container = &pod_spec.containers[0];
        let mounts = container.volume_mounts.as_ref().unwrap();
        assert_eq!(mounts.len(), 1);
        assert_eq!(mounts[0].mount_path, "/data");
    }

    #[test]
    fn test_deployment_manifest_no_volumes_when_no_mount_path() {
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
        let deploy = build_deployment_manifest("default", &image, &spec);

        let pod_spec = deploy.spec.as_ref().unwrap().template.spec.as_ref().unwrap();
        assert!(pod_spec.volumes.is_none());
    }

    // -----------------------------------------------------------------------
    // Custom HPA metric tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_hpa_custom_metric_with_name() {
        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 10,
            metrics: vec![ScalingMetric {
                metric_type: MetricType::Custom,
                target_value: "30".to_string(),
                metric_name: Some("http_requests_per_second".to_string()),
            }],
        });

        let hpa = build_hpa_manifest("default", &spec).unwrap();
        let metrics = hpa.spec.as_ref().unwrap().metrics.as_ref().unwrap();
        assert_eq!(metrics.len(), 1);
        assert_eq!(metrics[0].type_, "Pods");

        let pods = metrics[0].pods.as_ref().unwrap();
        assert_eq!(pods.metric.name, "http_requests_per_second");
        assert_eq!(pods.target.type_, "AverageValue");
        assert_eq!(pods.target.average_value.as_ref().unwrap().0, "30");
    }

    #[test]
    fn test_generate_hpa_custom_metric_missing_name_skipped() {
        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 10,
            metrics: vec![ScalingMetric {
                metric_type: MetricType::Custom,
                target_value: "50".to_string(),
                metric_name: None,
            }],
        });

        // No valid metrics → no HPA generated
        let hpa = build_hpa_manifest("default", &spec);
        assert!(hpa.is_none());
    }

    #[test]
    fn test_generate_hpa_custom_metric_empty_name_skipped() {
        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 10,
            metrics: vec![ScalingMetric {
                metric_type: MetricType::Custom,
                target_value: "50".to_string(),
                metric_name: Some(String::new()),
            }],
        });

        let hpa = build_hpa_manifest("default", &spec);
        assert!(hpa.is_none());
    }

    #[test]
    fn test_generate_hpa_mixed_cpu_and_custom() {
        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 2,
            max_replicas: 20,
            metrics: vec![
                ScalingMetric {
                    metric_type: MetricType::CPU,
                    target_value: "70".to_string(),
                    metric_name: None,
                },
                ScalingMetric {
                    metric_type: MetricType::Custom,
                    target_value: "100".to_string(),
                    metric_name: Some("queue_depth".to_string()),
                },
            ],
        });

        let hpa = build_hpa_manifest("default", &spec).unwrap();
        let metrics = hpa.spec.as_ref().unwrap().metrics.as_ref().unwrap();
        assert_eq!(metrics.len(), 2);

        // First is CPU Resource metric
        assert_eq!(metrics[0].type_, "Resource");
        assert_eq!(metrics[0].resource.as_ref().unwrap().name, "cpu");

        // Second is Custom Pods metric
        assert_eq!(metrics[1].type_, "Pods");
        assert_eq!(metrics[1].pods.as_ref().unwrap().metric.name, "queue_depth");
    }

    #[test]
    fn test_generate_hpa_custom_metric_invalid_value() {
        let mut spec = create_test_workload();
        spec.scaling = Some(ScalingSpec {
            enabled: true,
            min_replicas: 1,
            max_replicas: 10,
            metrics: vec![ScalingMetric {
                metric_type: MetricType::Custom,
                target_value: "abc".to_string(),
                metric_name: Some("requests".to_string()),
            }],
        });

        // Invalid target value → metric skipped → no valid metrics → no HPA
        let hpa = build_hpa_manifest("default", &spec);
        assert!(hpa.is_none());
    }

    #[test]
    fn test_scaling_metric_with_name_serde_roundtrip() {
        let metric = ScalingMetric {
            metric_type: MetricType::Custom,
            target_value: "42".to_string(),
            metric_name: Some("rps".to_string()),
        };
        let yaml = serde_yaml::to_string(&metric).unwrap();
        let parsed: ScalingMetric = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed.metric_name, Some("rps".to_string()));
        assert_eq!(parsed.metric_type, MetricType::Custom);
    }

    // -----------------------------------------------------------------------
    // NetworkPolicy generation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_generate_networkpolicy_none_when_absent() {
        let spec = create_test_workload();
        assert!(spec.network.network_policy.is_none());
        let result = build_networkpolicy_manifest("default", &spec);
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_networkpolicy_deny_all_ingress() {
        let mut spec = create_test_workload();
        spec.network.network_policy = Some(crate::spec::NetworkPolicyConfig {
            allow_from: vec![],
            allow_to: vec![],
            deny_all_ingress: true,
            deny_all_egress: false,
        });

        let netpol = build_networkpolicy_manifest("default", &spec).unwrap();
        assert_eq!(netpol.metadata.name, Some("test-app-netpol".to_string()));
        assert_eq!(netpol.metadata.namespace, Some("default".to_string()));

        let np_spec = netpol.spec.as_ref().unwrap();
        // Empty ingress vec = deny all ingress
        assert!(np_spec.ingress.as_ref().unwrap().is_empty());
        assert!(np_spec.egress.is_none());
        assert!(np_spec.policy_types.as_ref().unwrap().contains(&"Ingress".to_string()));
    }

    #[test]
    fn test_generate_networkpolicy_allow_from_labels() {
        let mut spec = create_test_workload();
        spec.network.network_policy = Some(crate::spec::NetworkPolicyConfig {
            allow_from: vec!["app=frontend".to_string(), "role=api".to_string()],
            allow_to: vec![],
            deny_all_ingress: false,
            deny_all_egress: false,
        });

        let netpol = build_networkpolicy_manifest("default", &spec).unwrap();
        let np_spec = netpol.spec.as_ref().unwrap();

        let ingress_rules = np_spec.ingress.as_ref().unwrap();
        assert_eq!(ingress_rules.len(), 1);
        let peers = ingress_rules[0].from.as_ref().unwrap();
        assert_eq!(peers.len(), 2);

        let first_labels = peers[0].pod_selector.as_ref().unwrap().match_labels.as_ref().unwrap();
        assert_eq!(first_labels.get("app"), Some(&"frontend".to_string()));
    }

    #[test]
    fn test_generate_networkpolicy_deny_all_egress() {
        let mut spec = create_test_workload();
        spec.network.network_policy = Some(crate::spec::NetworkPolicyConfig {
            allow_from: vec![],
            allow_to: vec![],
            deny_all_ingress: false,
            deny_all_egress: true,
        });

        let netpol = build_networkpolicy_manifest("default", &spec).unwrap();
        let np_spec = netpol.spec.as_ref().unwrap();

        assert!(np_spec.ingress.is_none());
        assert!(np_spec.egress.as_ref().unwrap().is_empty());
        assert!(np_spec.policy_types.as_ref().unwrap().contains(&"Egress".to_string()));
    }

    #[test]
    fn test_generate_networkpolicy_allow_to_labels() {
        let mut spec = create_test_workload();
        spec.network.network_policy = Some(crate::spec::NetworkPolicyConfig {
            allow_from: vec![],
            allow_to: vec!["app=database".to_string()],
            deny_all_ingress: false,
            deny_all_egress: false,
        });

        let netpol = build_networkpolicy_manifest("default", &spec).unwrap();
        let np_spec = netpol.spec.as_ref().unwrap();

        let egress_rules = np_spec.egress.as_ref().unwrap();
        assert_eq!(egress_rules.len(), 1);
        let peers = egress_rules[0].to.as_ref().unwrap();
        assert_eq!(peers.len(), 1);

        let labels = peers[0].pod_selector.as_ref().unwrap().match_labels.as_ref().unwrap();
        assert_eq!(labels.get("app"), Some(&"database".to_string()));
    }

    #[test]
    fn test_generate_networkpolicy_labels_and_namespace() {
        let mut spec = create_test_workload();
        spec.network.network_policy = Some(crate::spec::NetworkPolicyConfig {
            allow_from: vec![],
            allow_to: vec![],
            deny_all_ingress: true,
            deny_all_egress: false,
        });

        let netpol = build_networkpolicy_manifest("production", &spec).unwrap();
        assert_eq!(netpol.metadata.namespace, Some("production".to_string()));

        let labels = netpol.metadata.labels.as_ref().unwrap();
        assert_eq!(labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(labels.get("managed-by"), Some(&"aether".to_string()));
    }

    #[test]
    fn test_generate_networkpolicy_combined_ingress_egress() {
        let mut spec = create_test_workload();
        spec.network.network_policy = Some(crate::spec::NetworkPolicyConfig {
            allow_from: vec!["app=frontend".to_string()],
            allow_to: vec!["app=database".to_string()],
            deny_all_ingress: false,
            deny_all_egress: false,
        });

        let netpol = build_networkpolicy_manifest("default", &spec).unwrap();
        let np_spec = netpol.spec.as_ref().unwrap();

        assert!(np_spec.ingress.is_some());
        assert!(np_spec.egress.is_some());

        let policy_types = np_spec.policy_types.as_ref().unwrap();
        assert!(policy_types.contains(&"Ingress".to_string()));
        assert!(policy_types.contains(&"Egress".to_string()));
    }

    #[test]
    fn test_generate_networkpolicy_pod_selector() {
        let mut spec = create_test_workload();
        spec.network.network_policy = Some(crate::spec::NetworkPolicyConfig {
            allow_from: vec![],
            allow_to: vec![],
            deny_all_ingress: true,
            deny_all_egress: true,
        });

        let netpol = build_networkpolicy_manifest("default", &spec).unwrap();
        let np_spec = netpol.spec.as_ref().unwrap();

        let selector_labels = np_spec.pod_selector.match_labels.as_ref().unwrap();
        assert_eq!(selector_labels.get("app"), Some(&"test-app".to_string()));
        assert_eq!(selector_labels.get("managed-by"), Some(&"aether".to_string()));
    }
}
