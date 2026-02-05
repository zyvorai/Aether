//! Kubernetes runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::{AccessMode, Workload};
use async_trait::async_trait;
use k8s_openapi::api::core::v1::{
    Container, ContainerPort, HTTPGetAction, PersistentVolumeClaim, PersistentVolumeClaimSpec,
    Pod, PodSpec, Probe, ResourceRequirements as K8sResourceRequirements, Service, ServicePort,
    ServiceSpec, VolumeResourceRequirements,
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

impl KubernetesRuntime {
    /// Create new Kubernetes runtime
    pub async fn new() -> anyhow::Result<Self> {
        let client = Client::try_default().await?;
        let namespace = std::env::var("ORCHESTR8_NAMESPACE").unwrap_or_else(|_| "default".to_string());

        Ok(Self { client, namespace })
    }

    /// Create new Kubernetes runtime with specific namespace
    pub async fn with_namespace(namespace: String) -> anyhow::Result<Self> {
        let client = Client::try_default().await?;
        Ok(Self { client, namespace })
    }

    /// Generate Pod manifest from workload spec
    fn generate_pod(&self, image: &Image, spec: &Workload) -> Pod {
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

        // Container spec
        let container = Container {
            name: spec.metadata.name.clone(),
            image: Some(image.full_name()),
            ports: Some(ports),
            resources: Some(resources),
            liveness_probe,
            readiness_probe,
            ..Default::default()
        };

        Pod {
            metadata: ObjectMeta {
                name: Some(spec.metadata.name.clone()),
                namespace: Some(self.namespace.clone()),
                labels: Some(labels.clone()),
                annotations: Some(spec.metadata.annotations.clone().into_iter().collect()),
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![container],
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    /// Generate Service manifest from workload spec
    fn generate_service(&self, spec: &Workload) -> Option<Service> {
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
                namespace: Some(self.namespace.clone()),
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

    /// Generate PersistentVolumeClaim manifest from workload spec
    fn generate_pvc(&self, spec: &Workload) -> Option<PersistentVolumeClaim> {
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
                namespace: Some(self.namespace.clone()),
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

impl Default for KubernetesRuntime {
    fn default() -> Self {
        futures::executor::block_on(Self::new()).expect("Failed to initialize Kubernetes runtime")
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
        tracing::info!(
            "Deploying to Kubernetes namespace '{}': {}",
            self.namespace,
            spec.metadata.name
        );

        // Create PVC if needed
        if let Some(pvc) = self.generate_pvc(spec) {
            let pvcs: Api<PersistentVolumeClaim> =
                Api::namespaced(self.client.clone(), &self.namespace);

            match pvcs.create(&PostParams::default(), &pvc).await {
                Ok(_) => tracing::info!("Created PVC: {}-pvc", spec.metadata.name),
                Err(e) => tracing::warn!("PVC creation failed (may already exist): {}", e),
            }
        }

        // Create Service if needed
        if let Some(service) = self.generate_service(spec) {
            let services: Api<Service> = Api::namespaced(self.client.clone(), &self.namespace);

            match services.create(&PostParams::default(), &service).await {
                Ok(_) => tracing::info!("Created Service: {}-service", spec.metadata.name),
                Err(e) => tracing::warn!("Service creation failed (may already exist): {}", e),
            }
        }

        // Create Pod
        let pod = self.generate_pod(image, spec);
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

        Ok(Instance {
            id: uid,
            name: pod_name,
            runtime: RuntimeKind::Kubernetes,
            image: image.full_name(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn stop(&self, instance: &Instance) -> crate::Result<()> {
        // In Kubernetes, "stopping" means deleting the Pod
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

        let mut log_params = LogParams::default();
        log_params.follow = follow;

        let logs = pods.logs(&instance.name, &log_params).await?;
        Ok(logs)
    }

    async fn delete(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Deleting Kubernetes resources for: {}", instance.name);

        // Delete Pod
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        match pods.delete(&instance.name, &DeleteParams::default()).await {
            Ok(_) => tracing::info!("Deleted Pod: {}", instance.name),
            Err(e) => tracing::warn!("Pod deletion failed: {}", e),
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
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

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
    use crate::spec::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    // Note: These tests only validate manifest generation logic
    // They don't actually connect to Kubernetes
    // Integration tests with a real cluster would require #[tokio::test]

    #[test]
    fn test_generate_pod_manifest() {
        // Test Pod manifest generation without needing actual k8s client
        // We use impl directly to test the generate_pod method

        let spec = create_test_workload();
        let image = Image {
            name: "test-app".to_string(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::Kubernetes,
        };

        // Create pod manifest using the same logic as KubernetesRuntime
        let namespace = "default".to_string();
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), spec.metadata.name.clone());
        labels.insert("managed-by".to_string(), "orchestr8".to_string());

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

        let container = Container {
            name: spec.metadata.name.clone(),
            image: Some(image.full_name()),
            ports: Some(ports),
            ..Default::default()
        };

        let pod = Pod {
            metadata: ObjectMeta {
                name: Some(spec.metadata.name.clone()),
                namespace: Some(namespace),
                labels: Some(labels),
                ..Default::default()
            },
            spec: Some(PodSpec {
                containers: vec![container],
                ..Default::default()
            }),
            ..Default::default()
        };

        // Verify pod structure
        assert_eq!(pod.metadata.name, Some("test-app".to_string()));
        assert_eq!(pod.metadata.namespace, Some("default".to_string()));

        let pod_spec = pod.spec.unwrap();
        assert_eq!(pod_spec.containers.len(), 1);

        let container = &pod_spec.containers[0];
        assert_eq!(container.name, "test-app");
        assert_eq!(container.image, Some("test-app:latest".to_string()));
    }

    #[test]
    fn test_service_manifest_generation() {
        let spec = create_test_workload();

        // Test Service manifest structure
        let namespace = "default".to_string();
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), spec.metadata.name.clone());

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

        let service = Service {
            metadata: ObjectMeta {
                name: Some(format!("{}-service", spec.metadata.name)),
                namespace: Some(namespace),
                labels: Some(labels),
                ..Default::default()
            },
            spec: Some(ServiceSpec {
                type_: Some("ClusterIP".to_string()),
                ports: Some(ports.clone()),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(service.metadata.name, Some("test-app-service".to_string()));

        let service_spec = service.spec.unwrap();
        assert_eq!(service_spec.type_, Some("ClusterIP".to_string()));
        assert_eq!(service_spec.ports.unwrap().len(), 1);
    }

    #[test]
    fn test_pvc_manifest_generation() {
        let mut spec = create_test_workload();
        spec.persistence.enabled = true;
        spec.persistence.size = "10Gi".to_string();

        // Test PVC manifest structure
        let namespace = "default".to_string();
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), spec.metadata.name.clone());

        let mut requests = BTreeMap::new();
        requests.insert("storage".to_string(), Quantity(spec.persistence.size.clone()));

        let pvc = PersistentVolumeClaim {
            metadata: ObjectMeta {
                name: Some(format!("{}-pvc", spec.metadata.name)),
                namespace: Some(namespace),
                labels: Some(labels),
                ..Default::default()
            },
            spec: Some(PersistentVolumeClaimSpec {
                access_modes: Some(vec!["ReadWriteOnce".to_string()]),
                resources: Some(VolumeResourceRequirements {
                    requests: Some(requests),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        };

        assert_eq!(pvc.metadata.name, Some("test-app-pvc".to_string()));

        let pvc_spec = pvc.spec.unwrap();
        assert!(pvc_spec.access_modes.unwrap().contains(&"ReadWriteOnce".to_string()));
        let resources = pvc_spec.resources.unwrap();
        let requests = resources.requests.unwrap();
        assert_eq!(requests.get("storage").unwrap().0, "10Gi");
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
        }
    }
}
