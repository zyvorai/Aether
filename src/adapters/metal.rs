//! Metal3 bare metal runtime adapter

use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::Workload;
use async_trait::async_trait;
use kube::{
    api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams},
    core::{DynamicObject, GroupVersionKind},
    discovery, Client, ResourceExt,
};
use serde_json::json;
use std::collections::BTreeMap;

/// Metal3 runtime implementation
pub struct Metal3Runtime {
    client: Client,
    namespace: String,
}

impl Metal3Runtime {
    /// Create new Metal3 runtime
    pub async fn new() -> anyhow::Result<Self> {
        let client = Client::try_default().await?;
        let namespace =
            std::env::var("ORCHESTR8_NAMESPACE").unwrap_or_else(|_| "metal3-system".to_string());

        Ok(Self { client, namespace })
    }

    /// Create new Metal3 runtime with specific namespace
    pub async fn with_namespace(namespace: String) -> anyhow::Result<Self> {
        let client = Client::try_default().await?;
        Ok(Self { client, namespace })
    }

    /// Generate BareMetalHost JSON from workload spec
    fn generate_baremetalhost_json(&self, spec: &Workload) -> serde_json::Value {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), spec.metadata.name.clone());
        labels.insert("managed-by".to_string(), "orchestr8".to_string());

        // Add user labels
        for (k, v) in &spec.metadata.labels {
            labels.insert(k.clone(), v.clone());
        }

        // Parse CPU cores
        let cpu_cores = if spec.requirements.cpu.ends_with('m') {
            let milli = spec
                .requirements
                .cpu
                .trim_end_matches('m')
                .parse::<i32>()
                .unwrap_or(1000);
            (milli / 1000).max(1)
        } else {
            spec.requirements.cpu.parse::<i32>().unwrap_or(1)
        };

        // Parse memory (convert to MB)
        let memory_mb = self.parse_memory_to_mb(&spec.requirements.memory);

        // Parse storage (convert to GB)
        let storage_gb = self.parse_storage_to_gb(&spec.requirements.storage);

        let mut bmh = json!({
            "apiVersion": "metal3.io/v1alpha1",
            "kind": "BareMetalHost",
            "metadata": {
                "name": spec.metadata.name,
                "namespace": self.namespace,
                "labels": labels,
                "annotations": spec.metadata.annotations,
            },
            "spec": {
                "online": true,
                "bootMACAddress": "00:00:00:00:00:00", // Placeholder - should be discovered
                "bootMode": "UEFI",
                "image": {
                    "url": format!("http://image-server/{}.img", spec.image_name()),
                    "checksum": "http://image-server/{}.img.sha256sum".to_string(),
                },
                "userData": {
                    "name": format!("{}-userdata", spec.metadata.name),
                    "namespace": self.namespace,
                },
                "networkData": {
                    "name": format!("{}-networkdata", spec.metadata.name),
                    "namespace": self.namespace,
                },
                "customDeploy": {
                    "method": "install_coreos"
                },
                "rootDeviceHints": {
                    "deviceName": "/dev/sda",
                    "minSizeGigabytes": storage_gb,
                },
                "hardwareProfile": "unknown",
            }
        });

        // Add hardware requirements as annotations for matching
        bmh["metadata"]["annotations"]
            .as_object_mut()
            .unwrap()
            .insert(
                "orchestr8.io/cpu-cores".to_string(),
                json!(cpu_cores.to_string()),
            );
        bmh["metadata"]["annotations"]
            .as_object_mut()
            .unwrap()
            .insert(
                "orchestr8.io/memory-mb".to_string(),
                json!(memory_mb.to_string()),
            );

        // Add GPU requirements if specified
        if let Some(ref gpu_req) = spec.requirements.gpu {
            bmh["metadata"]["annotations"]
                .as_object_mut()
                .unwrap()
                .insert(
                    "orchestr8.io/gpu-vendor".to_string(),
                    json!(gpu_req.vendor.clone()),
                );
            bmh["metadata"]["annotations"]
                .as_object_mut()
                .unwrap()
                .insert(
                    "orchestr8.io/gpu-count".to_string(),
                    json!(gpu_req.count.to_string()),
                );
        }

        bmh
    }

    /// Parse memory string to MB
    fn parse_memory_to_mb(&self, memory: &str) -> i64 {
        if memory.ends_with("Gi") {
            let val = memory.trim_end_matches("Gi").parse::<i64>().unwrap_or(1);
            val * 1024
        } else if memory.ends_with("Mi") {
            memory.trim_end_matches("Mi").parse::<i64>().unwrap_or(1024)
        } else if memory.ends_with("G") {
            let val = memory.trim_end_matches('G').parse::<i64>().unwrap_or(1);
            val * 1024
        } else if memory.ends_with('M') {
            memory.trim_end_matches('M').parse::<i64>().unwrap_or(1024)
        } else {
            1024
        }
    }

    /// Parse storage string to GB
    fn parse_storage_to_gb(&self, storage: &str) -> i64 {
        if storage.ends_with("Gi") {
            storage.trim_end_matches("Gi").parse::<i64>().unwrap_or(10)
        } else if storage.ends_with("Mi") {
            let val = storage.trim_end_matches("Mi").parse::<i64>().unwrap_or(10240);
            (val / 1024).max(1)
        } else if storage.ends_with('G') {
            storage.trim_end_matches('G').parse::<i64>().unwrap_or(10)
        } else if storage.ends_with('M') {
            let val = storage.trim_end_matches('M').parse::<i64>().unwrap_or(10240);
            (val / 1024).max(1)
        } else {
            10
        }
    }

    /// Get API for BareMetalHost CRD
    async fn get_baremetalhost_api(&self) -> anyhow::Result<Api<DynamicObject>> {
        let gvk = GroupVersionKind::gvk("metal3.io", "v1alpha1", "BareMetalHost");
        let discovery = discovery::Discovery::new(self.client.clone()).run().await?;

        let apigroup = discovery
            .groups()
            .find(|g| g.name() == gvk.group)
            .ok_or_else(|| {
                anyhow::anyhow!("Cannot find Metal3 API group (Metal3 not installed?)")
            })?;

        let (ar, _caps) = apigroup
            .recommended_kind(&gvk.kind)
            .ok_or_else(|| anyhow::anyhow!("Cannot find BareMetalHost resource"))?;

        let api = Api::namespaced_with(self.client.clone(), &self.namespace, &ar);
        Ok(api)
    }

    /// Get host status
    async fn get_host_status(&self, name: &str) -> anyhow::Result<Status> {
        let api = self.get_baremetalhost_api().await?;

        match api.get(name).await {
            Ok(host) => {
                let spec = host.data.get("spec");
                let status = host.data.get("status");

                // Check online field
                let _online = spec
                    .and_then(|s| s.get("online"))
                    .and_then(|o| o.as_bool())
                    .unwrap_or(false);

                // Check provisioning state
                let provisioning_state = status
                    .and_then(|s| s.get("provisioning"))
                    .and_then(|p| p.get("state"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");

                // Check operational status
                let operational_status = status
                    .and_then(|s| s.get("operationalStatus"))
                    .and_then(|o| o.as_str())
                    .unwrap_or("unknown");

                // Determine state based on provisioning state
                let state = match provisioning_state {
                    "provisioned" => InstanceState::Running,
                    "provisioning" | "inspecting" | "preparing" | "registering" => {
                        InstanceState::Pending
                    }
                    "deprovisioning" => InstanceState::Stopped,
                    "available" | "ready" => InstanceState::Stopped,
                    _ => InstanceState::Unknown,
                };

                let ready = provisioning_state == "provisioned" && operational_status == "OK";

                // Get error message if any
                let message = status
                    .and_then(|s| s.get("errorMessage"))
                    .and_then(|m| m.as_str())
                    .map(|s| s.to_string());

                Ok(Status {
                    state,
                    ready,
                    message,
                    restart_count: 0,
                })
            }
            Err(_) => Ok(Status {
                state: InstanceState::Unknown,
                ready: false,
                message: Some("Host not found".to_string()),
                restart_count: 0,
            }),
        }
    }
}

impl Default for Metal3Runtime {
    fn default() -> Self {
        futures::executor::block_on(Self::new()).expect("Failed to initialize Metal3 runtime")
    }
}

#[async_trait]
impl Runtime for Metal3Runtime {
    async fn build(&self, spec: &Workload) -> crate::Result<Image> {
        // For Metal3, we assume the bare metal image is already built and available
        // The image should be a bootable disk image (e.g., CoreOS, Ubuntu, etc.)
        tracing::info!(
            "Metal3: Using pre-built image {} (ensure it's available on image server)",
            spec.image_name()
        );

        Ok(Image {
            name: spec.metadata.name.clone(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::Metal3,
        })
    }

    async fn run(&self, image: &Image, spec: &Workload) -> crate::Result<Instance> {
        tracing::info!(
            "Provisioning BareMetalHost to namespace '{}': {}",
            self.namespace,
            spec.metadata.name
        );

        // Generate BareMetalHost manifest
        let bmh_json = self.generate_baremetalhost_json(spec);
        let bmh: DynamicObject = serde_json::from_value(bmh_json)?;

        // Get API
        let api = self.get_baremetalhost_api().await?;

        // Create BareMetalHost
        let created_host = api.create(&PostParams::default(), &bmh).await?;

        let host_name = created_host.name_any();
        let uid = created_host
            .metadata
            .uid
            .unwrap_or_else(|| "unknown".to_string());

        tracing::info!("Created BareMetalHost: {}", host_name);
        tracing::info!(
            "Provisioning will begin automatically. Monitor with: kubectl get bmh -n {}",
            self.namespace
        );

        Ok(Instance {
            id: uid,
            name: host_name,
            runtime: RuntimeKind::Metal3,
            image: image.full_name(),
            created_at: chrono::Utc::now().to_rfc3339(),
        })
    }

    async fn stop(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Powering off BareMetalHost: {}", instance.name);

        let api = self.get_baremetalhost_api().await?;

        // Patch the host to set online=false
        let patch = json!({
            "spec": {
                "online": false
            }
        });

        api.patch(
            &instance.name,
            &PatchParams::default(),
            &Patch::Merge(patch),
        )
        .await?;

        tracing::info!("Host will power off gracefully");

        Ok(())
    }

    async fn status(&self, instance: &Instance) -> crate::Result<Status> {
        self.get_host_status(&instance.name).await
    }

    async fn logs(&self, instance: &Instance, _follow: bool) -> crate::Result<String> {
        // Bare metal hosts don't have logs in the traditional sense
        // You would typically access via BMC (IPMI, Redfish) or serial console
        tracing::info!(
            "Bare metal host console access available via BMC for: {}",
            instance.name
        );

        // Get host details to show access info
        let api = self.get_baremetalhost_api().await?;
        let host = api.get(&instance.name).await?;

        let bmc_address = host
            .data
            .get("spec")
            .and_then(|s| s.get("bmc"))
            .and_then(|b| b.get("address"))
            .and_then(|a| a.as_str())
            .unwrap_or("unknown");

        let provisioning_state = host
            .data
            .get("status")
            .and_then(|s| s.get("provisioning"))
            .and_then(|p| p.get("state"))
            .and_then(|s| s.as_str())
            .unwrap_or("unknown");

        let hardware_details = host
            .data
            .get("status")
            .and_then(|s| s.get("hardware"))
            .map(|h| serde_json::to_string_pretty(h).unwrap_or_else(|_| "N/A".to_string()))
            .unwrap_or_else(|| "N/A".to_string());

        Ok(format!(
            "BareMetalHost Information:\\n\\n\\\
            Host: {}\\n\\\
            BMC Address: {}\\n\\\
            Provisioning State: {}\\n\\n\\\
            Hardware Details:\\n{}\\n\\n\\\
            Console Access:\\n  \\\
            For IPMI: ipmitool -I lanplus -H <BMC-IP> -U <user> -P <pass> sol activate\\n  \\\
            For Redfish: Check BMC web interface at {}\\n\\n\\\
            Note: BMC credentials should be stored in Secret referenced by BareMetalHost\\n\\\
            Get full status: kubectl get bmh {} -n {} -o yaml",
            instance.name,
            bmc_address,
            provisioning_state,
            hardware_details,
            bmc_address,
            instance.name,
            self.namespace
        ))
    }

    async fn delete(&self, instance: &Instance) -> crate::Result<()> {
        tracing::info!("Deprovisioning BareMetalHost: {}", instance.name);

        let api = self.get_baremetalhost_api().await?;

        // Delete BareMetalHost
        match api.delete(&instance.name, &DeleteParams::default()).await {
            Ok(_) => {
                tracing::info!("Deleted BareMetalHost: {}", instance.name);
                tracing::info!("Host will be deprovisioned and returned to available pool");
            }
            Err(e) => tracing::warn!("Host deletion failed: {}", e),
        }

        Ok(())
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        let api = self.get_baremetalhost_api().await?;

        // List only hosts managed by orchestr8
        let lp = ListParams::default().labels("managed-by=orchestr8");
        let host_list = api.list(&lp).await?;

        let instances: Vec<Instance> = host_list
            .items
            .iter()
            .map(|host| {
                let name = host.name_any();
                let uid = host
                    .metadata
                    .uid
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());

                // Get image URL from spec
                let image = host
                    .data
                    .get("spec")
                    .and_then(|s| s.get("image"))
                    .and_then(|i| i.get("url"))
                    .and_then(|u| u.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let created_at = host
                    .metadata
                    .creation_timestamp
                    .as_ref()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());

                Instance {
                    id: uid,
                    name,
                    runtime: RuntimeKind::Metal3,
                    image,
                    created_at,
                }
            })
            .collect();

        Ok(instances)
    }
}
