// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Metal3 bare metal runtime adapter

use super::common;
use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::Workload;
use async_trait::async_trait;
use k8s_openapi::api::core::v1::Secret;
use kube::{
    api::{Api, DeleteParams, Patch, PatchParams, PostParams},
    core::DynamicObject,
    Client, ResourceExt,
};
use serde_json::json;
use std::collections::BTreeMap;

/// Metal3 runtime implementation
pub struct Metal3Runtime {
    client: Client,
    namespace: String,
}

super::impl_kube_adapter_new!(Metal3Runtime, "metal3-system");

impl Metal3Runtime {
    /// Generate BareMetalHost JSON from workload spec
    fn generate_baremetalhost_json(&self, spec: &Workload) -> serde_json::Value {
        build_baremetalhost_json(&self.namespace, spec)
    }
}

/// Parse memory string (e.g., "64Gi", "512Mi") to megabytes.
/// Delegates to the shared `resources::parse_memory_gi` and converts GiB -> MiB.
fn parse_memory_to_mb(memory: &str) -> i64 {
    let gi = crate::resources::parse_memory_gi(memory);
    if gi <= 0.0 {
        tracing::warn!("Could not parse memory '{}', defaulting to 1024 MB", memory);
        1024
    } else {
        // Guard against overflow: i64::MAX MB ≈ 8.8 exabytes
        let mb = gi * 1024.0;
        if mb > i64::MAX as f64 {
            i64::MAX
        } else {
            mb as i64
        }
    }
}

/// Parse storage string (e.g., "500Gi", "10240Mi") to gigabytes.
/// Delegates to the shared `resources::parse_memory_gi` (same suffix rules).
fn parse_storage_to_gb(storage: &str) -> i64 {
    let gi = crate::resources::parse_memory_gi(storage);
    if gi == 0.0 { 10 } else { (gi as i64).max(1) }
}

/// Build BareMetalHost JSON (standalone, testable without kube::Client)
fn build_baremetalhost_json(namespace: &str, spec: &Workload) -> serde_json::Value {
    let labels = common::build_managed_labels(&spec.metadata.name, &spec.metadata.labels);

    // Parse CPU cores
    let cpu_cores = common::parse_cpu_cores(&spec.requirements.cpu);

    // Parse memory (convert to MB)
    let memory_mb = parse_memory_to_mb(&spec.requirements.memory);

    // Parse storage (convert to GB)
    let storage_gb = parse_storage_to_gb(&spec.requirements.storage);

    let mut bmh = json!({
        "apiVersion": "metal3.io/v1alpha1",
        "kind": "BareMetalHost",
        "metadata": {
            "name": spec.metadata.name,
            "namespace": namespace,
            "labels": labels,
            "annotations": spec.metadata.annotations,
        },
        "spec": {
            "online": true,
            "bootMACAddress": spec.metadata.annotations
                .get("aether.io/boot-mac-address")
                .cloned()
                .unwrap_or_default(),
            "bootMode": spec.metadata.annotations
                .get("aether.io/boot-mode")
                .cloned()
                .unwrap_or_else(|| "UEFI".to_string()),
            "image": {
                "url": spec.metadata.annotations
                    .get("aether.io/image-url")
                    .cloned()
                    .unwrap_or_default(),
                "checksum": spec.metadata.annotations
                    .get("aether.io/image-checksum-url")
                    .cloned()
                    .unwrap_or_default(),
            },
            "userData": {
                "name": format!("{}-userdata", spec.metadata.name),
                "namespace": namespace,
            },
            "networkData": {
                "name": format!("{}-networkdata", spec.metadata.name),
                "namespace": namespace,
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

    // Add BMC configuration if annotations are present
    let bmc_address = spec.metadata.annotations.get("aether.io/bmc-address");
    if let Some(address) = bmc_address {
        bmh["spec"]["bmc"] = json!({
            "address": address,
            "credentialsName": format!("{}-bmc-creds", spec.metadata.name)
        });
    }

    // Add hardware requirements as annotations for matching
    if let Some(annotations) = bmh["metadata"]["annotations"].as_object_mut() {
        annotations.insert(
            "aether.io/cpu-cores".to_string(),
            json!(cpu_cores.to_string()),
        );
        annotations.insert(
            "aether.io/memory-mb".to_string(),
            json!(memory_mb.to_string()),
        );

        // Add GPU requirements if specified
        if let Some(ref gpu_req) = spec.requirements.gpu {
            annotations.insert(
                "aether.io/gpu-vendor".to_string(),
                json!(gpu_req.vendor.clone()),
            );
            annotations.insert(
                "aether.io/gpu-count".to_string(),
                json!(gpu_req.count.to_string()),
            );
        }
    }

    bmh
}

impl Metal3Runtime {
    /// Get API for BareMetalHost CRD
    async fn get_baremetalhost_api(&self) -> anyhow::Result<Api<DynamicObject>> {
        common::discover_crd_api(
            self.client.clone(),
            &self.namespace,
            "metal3.io",
            "v1alpha1",
            "BareMetalHost",
        )
        .await
    }

    /// Get host status
    async fn get_host_status(&self, name: &str) -> anyhow::Result<Status> {
        let api = self.get_baremetalhost_api().await?;

        match api.get(name).await {
            Ok(host) => {
                let status = host.data.get("status");

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
            Err(_) => Ok(common::not_found_status("Host")),
        }
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
        common::validate_kube_name(&spec.metadata.name)?;

        // Validate required Metal3 annotations before proceeding
        if !spec.metadata.annotations.contains_key("aether.io/boot-mac-address") {
            anyhow::bail!(
                "Required annotation 'aether.io/boot-mac-address' not set for '{}'. \
                 Metal3 provisioning requires a valid MAC address matching real hardware. \
                 Set it in metadata.annotations.",
                spec.metadata.name
            );
        }
        if !spec.metadata.annotations.contains_key("aether.io/image-url") {
            anyhow::bail!(
                "Required annotation 'aether.io/image-url' not set for '{}'. \
                 Metal3 provisioning requires an explicit bootable disk image URL. \
                 Set it in metadata.annotations.",
                spec.metadata.name
            );
        }

        tracing::info!(
            "Provisioning BareMetalHost to namespace '{}': {}",
            self.namespace,
            spec.metadata.name
        );

        // Create BMC credentials Secret if all three BMC annotations are present
        let bmc_address = spec.metadata.annotations.get("aether.io/bmc-address");
        let bmc_username = spec.metadata.annotations.get("aether.io/bmc-username");
        let bmc_password = spec.metadata.annotations.get("aether.io/bmc-password");

        if let (Some(_), Some(username), Some(password)) = (bmc_address, bmc_username, bmc_password)
        {
            let secret_name = format!("{}-bmc-creds", spec.metadata.name);
            let mut string_data = BTreeMap::new();
            string_data.insert("username".to_string(), username.clone());
            string_data.insert("password".to_string(), password.clone());

            let secret = Secret {
                metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                    name: Some(secret_name.clone()),
                    namespace: Some(self.namespace.clone()),
                    ..Default::default()
                },
                string_data: Some(string_data),
                type_: Some("Opaque".to_string()),
                ..Default::default()
            };

            let secrets: Api<Secret> = Api::namespaced(self.client.clone(), &self.namespace);
            match secrets.create(&PostParams::default(), &secret).await {
                Ok(_) => tracing::info!("Created BMC credentials Secret: {}", secret_name),
                Err(e) => tracing::warn!(
                    "BMC Secret creation failed (may already exist): {}",
                    e
                ),
            }
        }

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

        Ok(Instance::new(uid, host_name, RuntimeKind::Metal3, image.full_name()))
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
            "BareMetalHost Information:\n\n\
            Host: {}\n\
            BMC Address: {}\n\
            Provisioning State: {}\n\n\
            Hardware Details:\n{}\n\n\
            Console Access:\n  \
            For IPMI: ipmitool -I lanplus -H <BMC-IP> -U <user> -P <pass> sol activate\n  \
            For Redfish: Check BMC web interface at {}\n\n\
            Note: BMC credentials should be stored in Secret referenced by BareMetalHost\n\
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

        // List only hosts managed by aether
        let lp = common::managed_list_params();
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
                    .unwrap_or_else(crate::resources::now_rfc3339);

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

    async fn capacity(&self) -> crate::Result<Option<crate::runtime::Capacity>> {
        use k8s_openapi::api::core::v1::Node;
        use kube::api::{Api, ListParams};

        let nodes: Api<Node> = Api::all(self.client.clone());
        match nodes.list(&ListParams::default()).await {
            Ok(node_list) => {
                let mut total_cpu = 0.0_f64;
                let mut total_memory_mb = 0_u64;

                for node in &node_list.items {
                    if let Some(status) = &node.status {
                        if let Some(alloc) = &status.allocatable {
                            if let Some(cpu) = alloc.get("cpu") {
                                total_cpu += crate::resources::parse_cpu(&cpu.0);
                            }
                            if let Some(mem) = alloc.get("memory") {
                                let gi = crate::resources::parse_memory_gi(&mem.0);
                                total_memory_mb += (gi * 1024.0) as u64;
                            }
                        }
                    }
                }

                Ok(Some(crate::runtime::Capacity {
                    total_cpu,
                    available_cpu: total_cpu,
                    total_memory_mb,
                    available_memory_mb: total_memory_mb,
                }))
            }
            Err(e) => {
                tracing::warn!("Failed to probe Metal3 node capacity: {}", e);
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    // ---------------------------------------------------------------
    // Helper: build a minimal Workload for test purposes
    // ---------------------------------------------------------------
    fn make_workload(name: &str, cpu: &str, memory: &str, storage: &str) -> Workload {
        Workload {
            api_version: "aether/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: name.to_string(),
                owner: "test-owner".to_string(),
                project: "test-project".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/testorg".to_string(),
                build_args: HashMap::new(),
            ..Default::default()
            },
            requirements: ResourceRequirements {
                cpu: cpu.to_string(),
                memory: memory.to_string(),
                storage: storage.to_string(),
                gpu: None,
                cpu_request: None,
                memory_request: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Metal,
                allow: vec![RuntimeType::Metal],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
            intent: None,
            autonomy: None,
            confidential: None,
            schedule: None,
        kubernetes: None,
        }
    }

    // ===============================================================
    // parse_memory_to_mb tests (original tests kept, new ones added)
    // ===============================================================
    #[test]
    fn test_parse_memory_to_mb_gi() {
        assert_eq!(parse_memory_to_mb("64Gi"), 65536);
        assert_eq!(parse_memory_to_mb("1Gi"), 1024);
    }

    #[test]
    fn test_parse_memory_to_mb_mi() {
        assert_eq!(parse_memory_to_mb("512Mi"), 512);
        assert_eq!(parse_memory_to_mb("2048Mi"), 2048);
    }

    #[test]
    fn test_parse_memory_to_mb_g() {
        // 4 decimal GB ≈ 3.725 GiB ≈ 3814 MB
        assert_eq!(parse_memory_to_mb("4G"), 3814);
    }

    #[test]
    fn test_parse_memory_to_mb_m() {
        // 256 decimal MB ≈ 0.238 GiB ≈ 244 MB
        assert_eq!(parse_memory_to_mb("256M"), 244);
    }

    #[test]
    fn test_parse_memory_to_mb_default() {
        assert_eq!(parse_memory_to_mb("unknown"), 1024);
    }

    #[test]
    fn test_parse_memory_to_mb_large_gi() {
        assert_eq!(parse_memory_to_mb("128Gi"), 131072);
        assert_eq!(parse_memory_to_mb("256Gi"), 262144);
    }

    #[test]
    fn test_parse_memory_to_mb_small_mi() {
        assert_eq!(parse_memory_to_mb("64Mi"), 64);
        assert_eq!(parse_memory_to_mb("128Mi"), 128);
    }

    #[test]
    fn test_parse_memory_to_mb_large_g() {
        // 16 decimal GB ≈ 14.901 GiB ≈ 15258 MB
        assert_eq!(parse_memory_to_mb("16G"), 15258);
    }

    #[test]
    fn test_parse_memory_to_mb_large_m() {
        // 4096 decimal MB ≈ 3.815 GiB ≈ 3906 MB
        assert_eq!(parse_memory_to_mb("4096M"), 3906);
    }

    #[test]
    fn test_parse_memory_to_mb_invalid_gi_value() {
        // Non-numeric value before Gi defaults to 1
        assert_eq!(parse_memory_to_mb("xGi"), 1024); // 1 * 1024
    }

    #[test]
    fn test_parse_memory_to_mb_invalid_mi_value() {
        // Non-numeric value before Mi defaults to 1024
        assert_eq!(parse_memory_to_mb("abcMi"), 1024);
    }

    #[test]
    fn test_parse_memory_to_mb_empty_string() {
        assert_eq!(parse_memory_to_mb(""), 1024);
    }

    #[test]
    fn test_parse_memory_to_mb_plain_number() {
        // A plain number without suffix falls to default
        assert_eq!(parse_memory_to_mb("8192"), 1024);
    }

    // ===============================================================
    // parse_storage_to_gb tests (original tests kept, new ones added)
    // ===============================================================
    #[test]
    fn test_parse_storage_to_gb_gi() {
        assert_eq!(parse_storage_to_gb("500Gi"), 500);
        assert_eq!(parse_storage_to_gb("1Gi"), 1);
    }

    #[test]
    fn test_parse_storage_to_gb_mi() {
        assert_eq!(parse_storage_to_gb("10240Mi"), 10);
        assert_eq!(parse_storage_to_gb("512Mi"), 1); // min 1
    }

    #[test]
    fn test_parse_storage_to_gb_default() {
        assert_eq!(parse_storage_to_gb("unknown"), 10);
    }

    #[test]
    fn test_parse_storage_to_gb_large_gi() {
        assert_eq!(parse_storage_to_gb("2000Gi"), 2000);
    }

    #[test]
    fn test_parse_storage_to_gb_g_suffix() {
        // 100 decimal GB ≈ 93.13 GiB → 93
        assert_eq!(parse_storage_to_gb("100G"), 93);
        // 1 decimal GB ≈ 0.93 GiB → max(1) = 1
        assert_eq!(parse_storage_to_gb("1G"), 1);
    }

    #[test]
    fn test_parse_storage_to_gb_m_suffix() {
        // 10240 decimal MB ≈ 9.54 GiB → 9
        assert_eq!(parse_storage_to_gb("10240M"), 9);
        // 2048 decimal MB ≈ 1.91 GiB → 1
        assert_eq!(parse_storage_to_gb("2048M"), 1);
    }

    #[test]
    fn test_parse_storage_to_gb_small_m_clamps_to_one() {
        assert_eq!(parse_storage_to_gb("100M"), 1);
        assert_eq!(parse_storage_to_gb("512M"), 1);
    }

    #[test]
    fn test_parse_storage_to_gb_small_mi_clamps_to_one() {
        assert_eq!(parse_storage_to_gb("100Mi"), 1);
    }

    #[test]
    fn test_parse_storage_to_gb_invalid_gi_value() {
        assert_eq!(parse_storage_to_gb("abcGi"), 10);
    }

    #[test]
    fn test_parse_storage_to_gb_invalid_mi_value() {
        // Non-numeric before Mi defaults to 10240, then 10240 / 1024 = 10
        assert_eq!(parse_storage_to_gb("xyzMi"), 10);
    }

    #[test]
    fn test_parse_storage_to_gb_empty_string() {
        assert_eq!(parse_storage_to_gb(""), 10);
    }

    #[test]
    fn test_parse_storage_to_gb_plain_number() {
        // No recognized suffix -> default
        assert_eq!(parse_storage_to_gb("500"), 10);
    }

    // ===============================================================
    // BareMetalHost CRD JSON generation
    // ===============================================================
    #[test]
    fn test_bmh_json_api_version_and_kind() {
        let spec = make_workload("bare-host", "8", "64Gi", "500Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["apiVersion"], "metal3.io/v1alpha1");
        assert_eq!(bmh["kind"], "BareMetalHost");
    }

    #[test]
    fn test_bmh_json_metadata_name_and_namespace() {
        let spec = make_workload("worker-01", "16", "128Gi", "1000Gi");
        let bmh = build_baremetalhost_json("infra-ns", &spec);

        assert_eq!(bmh["metadata"]["name"], "worker-01");
        assert_eq!(bmh["metadata"]["namespace"], "infra-ns");
    }

    #[test]
    fn test_bmh_json_labels_contain_managed_by() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["metadata"]["labels"]["app"], "my-host");
        assert_eq!(bmh["metadata"]["labels"]["managed-by"], "aether");
    }

    #[test]
    fn test_bmh_json_user_labels_propagated() {
        let mut spec = make_workload("my-host", "4", "32Gi", "200Gi");
        spec.metadata.labels.insert("rack".to_string(), "rack-a".to_string());
        spec.metadata.labels.insert("dc".to_string(), "us-east-1".to_string());

        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["metadata"]["labels"]["rack"], "rack-a");
        assert_eq!(bmh["metadata"]["labels"]["dc"], "us-east-1");
    }

    #[test]
    fn test_bmh_json_online_true() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["online"], true);
    }

    #[test]
    fn test_bmh_json_boot_mode_default_uefi() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["bootMode"], "UEFI");
    }

    #[test]
    fn test_bmh_json_boot_mode_from_annotation() {
        let mut spec = make_workload("my-host", "4", "32Gi", "200Gi");
        spec.metadata.annotations.insert(
            "aether.io/boot-mode".to_string(),
            "BIOS".to_string(),
        );
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["bootMode"], "BIOS");
    }

    #[test]
    fn test_bmh_json_empty_mac_address_without_annotation() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        // Without the annotation, bootMACAddress defaults to empty string
        assert_eq!(bmh["spec"]["bootMACAddress"], "");
    }

    #[test]
    fn test_bmh_json_mac_address_from_annotation() {
        let mut spec = make_workload("my-host", "4", "32Gi", "200Gi");
        spec.metadata.annotations.insert(
            "aether.io/boot-mac-address".to_string(),
            "AA:BB:CC:DD:EE:FF".to_string(),
        );
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["bootMACAddress"], "AA:BB:CC:DD:EE:FF");
    }

    // ---------------------------------------------------------------
    // BMC configuration in manifest
    // ---------------------------------------------------------------
    #[test]
    fn test_bmh_json_image_url_empty_without_annotation() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        // Without the annotation, image URL defaults to empty string
        assert_eq!(bmh["spec"]["image"]["url"], "");
    }

    #[test]
    fn test_bmh_json_image_url_from_annotation() {
        let mut spec = make_workload("my-host", "4", "32Gi", "200Gi");
        spec.metadata.annotations.insert(
            "aether.io/image-url".to_string(),
            "https://images.example.com/coreos.img".to_string(),
        );
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["image"]["url"], "https://images.example.com/coreos.img");
    }

    #[test]
    fn test_bmh_json_userdata_reference() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["userData"]["name"], "my-host-userdata");
        assert_eq!(bmh["spec"]["userData"]["namespace"], "metal3-system");
    }

    #[test]
    fn test_bmh_json_networkdata_reference() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["networkData"]["name"], "my-host-networkdata");
        assert_eq!(bmh["spec"]["networkData"]["namespace"], "metal3-system");
    }

    #[test]
    fn test_bmh_json_custom_deploy_method() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["customDeploy"]["method"], "install_coreos");
    }

    #[test]
    fn test_bmh_json_root_device_hints() {
        let spec = make_workload("my-host", "4", "32Gi", "500Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["rootDeviceHints"]["deviceName"], "/dev/sda");
        assert_eq!(bmh["spec"]["rootDeviceHints"]["minSizeGigabytes"], 500);
    }

    #[test]
    fn test_bmh_json_root_device_hints_storage_conversion() {
        let spec = make_workload("my-host", "4", "32Gi", "10240Mi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        // 10240Mi = 10 GB
        assert_eq!(bmh["spec"]["rootDeviceHints"]["minSizeGigabytes"], 10);
    }

    #[test]
    fn test_bmh_json_hardware_profile() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["spec"]["hardwareProfile"], "unknown");
    }

    // ---------------------------------------------------------------
    // Hardware matching via annotations
    // ---------------------------------------------------------------
    #[test]
    fn test_bmh_json_cpu_annotation() {
        let spec = make_workload("my-host", "8", "64Gi", "500Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["metadata"]["annotations"]["aether.io/cpu-cores"], "8");
    }

    #[test]
    fn test_bmh_json_cpu_annotation_millicore() {
        let spec = make_workload("my-host", "4000m", "64Gi", "500Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(bmh["metadata"]["annotations"]["aether.io/cpu-cores"], "4");
    }

    #[test]
    fn test_bmh_json_cpu_annotation_small_millicore_clamps() {
        let spec = make_workload("my-host", "500m", "64Gi", "500Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        // 500m / 1000 = 0, clamped to 1
        assert_eq!(bmh["metadata"]["annotations"]["aether.io/cpu-cores"], "1");
    }

    #[test]
    fn test_bmh_json_memory_annotation() {
        let spec = make_workload("my-host", "8", "64Gi", "500Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        // 64 Gi = 65536 MB
        assert_eq!(
            bmh["metadata"]["annotations"]["aether.io/memory-mb"],
            "65536"
        );
    }

    #[test]
    fn test_bmh_json_memory_annotation_mi() {
        let spec = make_workload("my-host", "8", "2048Mi", "500Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(
            bmh["metadata"]["annotations"]["aether.io/memory-mb"],
            "2048"
        );
    }

    // ---------------------------------------------------------------
    // GPU annotation insertion
    // ---------------------------------------------------------------
    #[test]
    fn test_bmh_json_no_gpu_annotations_by_default() {
        let spec = make_workload("my-host", "8", "64Gi", "500Gi");
        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert!(bmh["metadata"]["annotations"]["aether.io/gpu-vendor"].is_null());
        assert!(bmh["metadata"]["annotations"]["aether.io/gpu-count"].is_null());
    }

    #[test]
    fn test_bmh_json_gpu_vendor_annotation() {
        let mut spec = make_workload("gpu-host", "16", "128Gi", "1000Gi");
        spec.requirements.gpu = Some(GpuRequirements {
            count: 2,
            vendor: "nvidia".to_string(),
        });

        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(
            bmh["metadata"]["annotations"]["aether.io/gpu-vendor"],
            "nvidia"
        );
    }

    #[test]
    fn test_bmh_json_gpu_count_annotation() {
        let mut spec = make_workload("gpu-host", "16", "128Gi", "1000Gi");
        spec.requirements.gpu = Some(GpuRequirements {
            count: 4,
            vendor: "nvidia".to_string(),
        });

        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(
            bmh["metadata"]["annotations"]["aether.io/gpu-count"],
            "4"
        );
    }

    #[test]
    fn test_bmh_json_amd_gpu_annotation() {
        let mut spec = make_workload("amd-host", "8", "64Gi", "500Gi");
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "amd".to_string(),
        });

        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(
            bmh["metadata"]["annotations"]["aether.io/gpu-vendor"],
            "amd"
        );
        assert_eq!(
            bmh["metadata"]["annotations"]["aether.io/gpu-count"],
            "1"
        );
    }

    #[test]
    fn test_bmh_json_intel_gpu_annotation() {
        let mut spec = make_workload("intel-host", "8", "64Gi", "500Gi");
        spec.requirements.gpu = Some(GpuRequirements {
            count: 3,
            vendor: "intel".to_string(),
        });

        let bmh = build_baremetalhost_json("metal3-system", &spec);

        assert_eq!(
            bmh["metadata"]["annotations"]["aether.io/gpu-vendor"],
            "intel"
        );
        assert_eq!(
            bmh["metadata"]["annotations"]["aether.io/gpu-count"],
            "3"
        );
    }

    // ---------------------------------------------------------------
    // User annotations co-exist with hardware annotations
    // ---------------------------------------------------------------
    #[test]
    fn test_bmh_json_user_annotations_preserved_with_hardware() {
        let mut spec = make_workload("annotated-host", "4", "32Gi", "200Gi");
        spec.metadata.annotations.insert(
            "description".to_string(),
            "production bare metal worker".to_string(),
        );

        let bmh = build_baremetalhost_json("metal3-system", &spec);

        // User annotation should be present
        assert_eq!(
            bmh["metadata"]["annotations"]["description"],
            "production bare metal worker"
        );
        // Hardware annotations should also be present
        assert_eq!(bmh["metadata"]["annotations"]["aether.io/cpu-cores"], "4");
        assert_eq!(
            bmh["metadata"]["annotations"]["aether.io/memory-mb"],
            "32768"
        );
    }

    // ---------------------------------------------------------------
    // Namespace propagation in sub-resources
    // ---------------------------------------------------------------
    #[test]
    fn test_bmh_json_namespace_propagates_to_subreferences() {
        let spec = make_workload("my-host", "4", "32Gi", "200Gi");
        let bmh = build_baremetalhost_json("custom-ns", &spec);

        assert_eq!(bmh["metadata"]["namespace"], "custom-ns");
        assert_eq!(bmh["spec"]["userData"]["namespace"], "custom-ns");
        assert_eq!(bmh["spec"]["networkData"]["namespace"], "custom-ns");
    }
}
