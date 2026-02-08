//! KubeVirt VM runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::{AccessMode, Workload};
use async_trait::async_trait;
use kube::{
    api::{Api, DeleteParams, ListParams, PostParams},
    core::{DynamicObject, GroupVersionKind},
    discovery, Client, ResourceExt,
};
use serde_json::json;
use std::collections::BTreeMap;

/// KubeVirt runtime implementation
pub struct KubeVirtRuntime {
    client: Client,
    namespace: String,
}

impl KubeVirtRuntime {
    /// Create new KubeVirt runtime
    pub async fn new() -> anyhow::Result<Self> {
        let client = Client::try_default().await?;
        let namespace =
            std::env::var("ORCHESTR8_NAMESPACE").unwrap_or_else(|_| "default".to_string());

        Ok(Self { client, namespace })
    }

    /// Create new KubeVirt runtime with specific namespace
    pub async fn with_namespace(namespace: String) -> anyhow::Result<Self> {
        let client = Client::try_default().await?;
        Ok(Self { client, namespace })
    }

    /// Generate DataVolume JSON from workload spec
    fn generate_datavolume_json(&self, image: &Image, spec: &Workload) -> serde_json::Value {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), spec.metadata.name.clone());
        labels.insert("managed-by".to_string(), "orchestr8".to_string());

        let access_mode = match spec.persistence.access_mode {
            AccessMode::ReadWriteOnce => "ReadWriteOnce",
            AccessMode::ReadOnlyMany => "ReadOnlyMany",
            AccessMode::ReadWriteMany => "ReadWriteMany",
        };

        json!({
            "apiVersion": "cdi.kubevirt.io/v1beta1",
            "kind": "DataVolume",
            "metadata": {
                "name": format!("{}-disk", spec.metadata.name),
                "namespace": self.namespace,
                "labels": labels,
            },
            "spec": {
                "source": {
                    "registry": {
                        "url": format!("docker://{}", image.full_name())
                    }
                },
                "storage": {
                    "accessModes": [access_mode],
                    "resources": {
                        "requests": {
                            "storage": spec.requirements.storage
                        }
                    },
                    "storageClassName": spec.persistence.storage_class
                }
            }
        })
    }

    /// Generate VirtualMachine JSON from workload spec
    fn generate_virtualmachine_json(&self, spec: &Workload) -> serde_json::Value {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), spec.metadata.name.clone());
        labels.insert("managed-by".to_string(), "orchestr8".to_string());

        // Add user labels
        for (k, v) in &spec.metadata.labels {
            labels.insert(k.clone(), v.clone());
        }

        let cpu_cores = parse_cpu_cores(&spec.requirements.cpu);

        let mut vm_spec = json!({
            "apiVersion": "kubevirt.io/v1",
            "kind": "VirtualMachine",
            "metadata": {
                "name": spec.metadata.name,
                "namespace": self.namespace,
                "labels": labels,
                "annotations": spec.metadata.annotations,
            },
            "spec": {
                "running": true,
                "template": {
                    "metadata": {
                        "labels": labels
                    },
                    "spec": {
                        "domain": {
                            "cpu": {
                                "cores": cpu_cores
                            },
                            "resources": {
                                "requests": {
                                    "memory": spec.requirements.memory
                                }
                            },
                            "devices": {
                                "disks": [{
                                    "name": "rootdisk",
                                    "disk": {
                                        "bus": "virtio"
                                    }
                                }],
                                "interfaces": if spec.network.service {
                                    Some(vec![json!({
                                        "name": "default",
                                        "masquerade": {}
                                    })])
                                } else {
                                    None
                                }
                            }
                        },
                        "networks": if spec.network.service {
                            Some(vec![json!({
                                "name": "default",
                                "pod": {}
                            })])
                        } else {
                            None
                        },
                        "volumes": [{
                            "name": "rootdisk",
                            "dataVolume": {
                                "name": format!("{}-disk", spec.metadata.name)
                            }
                        }]
                    }
                }
            }
        });

        // Add GPU if requested
        if let Some(ref gpu_req) = spec.requirements.gpu {
            let gpus: Vec<serde_json::Value> = (0..gpu_req.count)
                .map(|i| {
                    json!({
                        "name": format!("gpu{}", i),
                        "deviceName": format!("{}.com/{}", gpu_req.vendor, gpu_req.vendor)
                    })
                })
                .collect();

            vm_spec["spec"]["template"]["spec"]["domain"]["devices"]["gpus"] = json!(gpus);
        }

        vm_spec
    }

    /// Get API for DataVolume CRD
    async fn get_datavolume_api(&self) -> anyhow::Result<Api<DynamicObject>> {
        let gvk = GroupVersionKind::gvk("cdi.kubevirt.io", "v1beta1", "DataVolume");
        let discovery = discovery::Discovery::new(self.client.clone()).run().await?;

        let apigroup = discovery
            .groups()
            .find(|g| g.name() == gvk.group)
            .ok_or_else(|| anyhow::anyhow!("Cannot find CDI API group (KubeVirt not installed?)"))?;

        let (ar, _caps) = apigroup
            .recommended_kind(&gvk.kind)
            .ok_or_else(|| anyhow::anyhow!("Cannot find DataVolume resource"))?;

        let api = Api::namespaced_with(self.client.clone(), &self.namespace, &ar);
        Ok(api)
    }

    /// Get API for VirtualMachine CRD
    async fn get_virtualmachine_api(&self) -> anyhow::Result<Api<DynamicObject>> {
        let gvk = GroupVersionKind::gvk("kubevirt.io", "v1", "VirtualMachine");
        let discovery = discovery::Discovery::new(self.client.clone()).run().await?;

        let apigroup = discovery
            .groups()
            .find(|g| g.name() == gvk.group)
            .ok_or_else(|| anyhow::anyhow!("Cannot find KubeVirt API group (KubeVirt not installed?)"))?;

        let (ar, _caps) = apigroup
            .recommended_kind(&gvk.kind)
            .ok_or_else(|| anyhow::anyhow!("Cannot find VirtualMachine resource"))?;

        let api = Api::namespaced_with(self.client.clone(), &self.namespace, &ar);
        Ok(api)
    }

    /// Get VM status
    async fn get_vm_status(&self, name: &str) -> anyhow::Result<Status> {
        let api = self.get_virtualmachine_api().await?;

        match api.get(name).await {
            Ok(vm) => {
                let spec = vm.data.get("spec");
                let status = vm.data.get("status");

                // Check running field
                let running = spec
                    .and_then(|s| s.get("running"))
                    .and_then(|r| r.as_bool())
                    .unwrap_or(false);

                // Check ready condition
                let ready = status
                    .and_then(|s| s.get("ready"))
                    .and_then(|r| r.as_bool())
                    .unwrap_or(false);

                let state = if running && ready {
                    InstanceState::Running
                } else if running {
                    InstanceState::Pending
                } else {
                    InstanceState::Stopped
                };

                Ok(Status {
                    state,
                    ready,
                    message: None,
                    restart_count: 0,
                })
            }
            Err(_) => Ok(Status {
                state: InstanceState::Unknown,
                ready: false,
                message: Some("VM not found".to_string()),
                restart_count: 0,
            }),
        }
    }
}

/// Parse CPU string (e.g., "4" or "2000m") to core count
fn parse_cpu_cores(cpu: &str) -> i32 {
    if cpu.ends_with('m') {
        let milli = cpu.trim_end_matches('m').parse::<i32>().unwrap_or(1000);
        (milli / 1000).max(1)
    } else {
        cpu.parse::<i32>().unwrap_or(1)
    }
}


#[async_trait]
impl Runtime for KubeVirtRuntime {
    async fn build(&self, spec: &Workload) -> crate::Result<Image> {
        // For KubeVirt, we assume the image is already built and pushed to registry
        // The DataVolume will pull it from the registry
        tracing::info!(
            "KubeVirt: Using pre-built image {} (ensure it's pushed to registry)",
            spec.image_name()
        );

        Ok(Image {
            name: spec.metadata.name.clone(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::KubeVirt,
        })
    }

    async fn run(&self, image: &Image, spec: &Workload) -> crate::Result<Instance> {
        tracing::info!(
            "Deploying VirtualMachine to namespace '{}': {}",
            self.namespace,
            spec.metadata.name
        );

        // Create DataVolume
        let datavolume_json = self.generate_datavolume_json(image, spec);
        let dv: DynamicObject = serde_json::from_value(datavolume_json)?;

        let dv_api = self.get_datavolume_api().await?;
        match dv_api.create(&PostParams::default(), &dv).await {
            Ok(_) => tracing::info!("Created DataVolume: {}-disk", spec.metadata.name),
            Err(e) => tracing::warn!("DataVolume creation failed (may already exist): {}", e),
        }

        // Wait a moment for DataVolume to be created
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Create VirtualMachine
        let vm_json = self.generate_virtualmachine_json(spec);
        let vm: DynamicObject = serde_json::from_value(vm_json)?;

        let vm_api = self.get_virtualmachine_api().await?;
        let created_vm = vm_api.create(&PostParams::default(), &vm).await?;

        let vm_name = created_vm.name_any();
        let uid = created_vm
            .metadata
            .uid
            .unwrap_or_else(|| "unknown".to_string());

        tracing::info!("Created VirtualMachine: {}", vm_name);

        Ok(Instance {
            id: uid,
            name: vm_name,
            runtime: RuntimeKind::KubeVirt,
            image: image.full_name(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn stop(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Stopping VirtualMachine: {}", instance.name);

        let api = self.get_virtualmachine_api().await?;

        // Patch the VM to set running=false
        let patch = json!({
            "spec": {
                "running": false
            }
        });

        api.patch(
            &instance.name,
            &kube::api::PatchParams::default(),
            &kube::api::Patch::Merge(patch),
        )
        .await?;

        Ok(())
    }

    async fn status(&self, instance: &Instance) -> crate::Result<Status> {
        self.get_vm_status(&instance.name).await
    }

    async fn logs(&self, instance: &Instance, _follow: bool) -> crate::Result<String> {
        // KubeVirt VMs don't have logs in the traditional sense
        // You would typically access the serial console
        tracing::info!(
            "VM console access available via: virtctl console {}",
            instance.name
        );

        Ok(format!(
            "VirtualMachine Console Access:\n\n\
            Serial Console:\n  virtctl console {}\n\n\
            VNC Access:\n  virtctl vnc {}\n\n\
            Note: Install virtctl CLI tool from:\n  \
            https://kubevirt.io/user-guide/operations/virtctl_client_tool/",
            instance.name, instance.name
        ))
    }

    async fn delete(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Deleting KubeVirt resources for: {}", instance.name);

        // Delete VirtualMachine
        let vm_api = self.get_virtualmachine_api().await?;
        match vm_api
            .delete(&instance.name, &DeleteParams::default())
            .await
        {
            Ok(_) => tracing::info!("Deleted VirtualMachine: {}", instance.name),
            Err(e) => tracing::warn!("VM deletion failed: {}", e),
        }

        // Delete DataVolume
        let dv_api = self.get_datavolume_api().await?;
        let dv_name = format!("{}-disk", instance.name);
        match dv_api.delete(&dv_name, &DeleteParams::default()).await {
            Ok(_) => tracing::info!("Deleted DataVolume: {}", dv_name),
            Err(e) => tracing::debug!("DataVolume deletion failed (may not exist): {}", e),
        }

        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        let api = self.get_virtualmachine_api().await?;

        // List only VMs managed by orchestr8
        let lp = ListParams::default().labels("managed-by=orchestr8");
        let vm_list = api.list(&lp).await?;

        let instances: Vec<Instance> = vm_list
            .items
            .iter()
            .map(|vm| {
                let name = vm.name_any();
                let uid = vm.metadata.uid.clone().unwrap_or_else(|| "unknown".to_string());

                // Get image from spec if available
                let image = vm
                    .data
                    .get("spec")
                    .and_then(|s| s.get("template"))
                    .and_then(|t| t.get("spec"))
                    .and_then(|s| s.get("volumes"))
                    .and_then(|v| v.as_array())
                    .and_then(|arr| arr.first())
                    .and_then(|vol| vol.get("dataVolume"))
                    .and_then(|dv| dv.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let created_at = vm
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

                Instance {
                    id: uid,
                    name,
                    runtime: RuntimeKind::KubeVirt,
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

    #[test]
    fn test_parse_cpu_cores_whole() {
        assert_eq!(parse_cpu_cores("4"), 4);
        assert_eq!(parse_cpu_cores("1"), 1);
        assert_eq!(parse_cpu_cores("16"), 16);
    }

    #[test]
    fn test_parse_cpu_cores_millicore() {
        assert_eq!(parse_cpu_cores("2000m"), 2);
        assert_eq!(parse_cpu_cores("4000m"), 4);
        assert_eq!(parse_cpu_cores("500m"), 1); // min 1 core
    }

    #[test]
    fn test_parse_cpu_cores_invalid() {
        assert_eq!(parse_cpu_cores("invalid"), 1); // defaults to 1
        assert_eq!(parse_cpu_cores("xm"), 1); // invalid millicore defaults
    }
}
