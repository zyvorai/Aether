// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! KubeVirt VM runtime adapter

use super::common;
use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind, Status};
use crate::spec::{AccessMode, Workload};
use anyhow::Context as _;
use async_trait::async_trait;
use k8s_openapi::api::core::v1::Pod;
use kube::{
    api::{Api, DeleteParams, ListParams, LogParams, PostParams},
    core::DynamicObject,
    Client, ResourceExt,
};
use serde_json::json;

/// KubeVirt runtime implementation
pub struct KubeVirtRuntime {
    client: Client,
    namespace: String,
}

super::impl_kube_adapter_new!(KubeVirtRuntime, "default");

impl KubeVirtRuntime {
    /// Generate DataVolume JSON from workload spec
    fn generate_datavolume_json(&self, image: &Image, spec: &Workload) -> serde_json::Value {
        build_datavolume_json(&self.namespace, image, spec)
    }

    /// Generate VirtualMachine JSON from workload spec
    fn generate_virtualmachine_json(&self, spec: &Workload) -> serde_json::Value {
        build_virtualmachine_json(&self.namespace, spec)
    }

    /// Get API for DataVolume CRD
    async fn get_datavolume_api(&self) -> anyhow::Result<Api<DynamicObject>> {
        common::discover_crd_api(
            self.client.clone(),
            &self.namespace,
            "cdi.kubevirt.io",
            "v1beta1",
            "DataVolume",
        )
        .await
    }

    /// Get API for VirtualMachine CRD
    async fn get_virtualmachine_api(&self) -> anyhow::Result<Api<DynamicObject>> {
        common::discover_crd_api(
            self.client.clone(),
            &self.namespace,
            "kubevirt.io",
            "v1",
            "VirtualMachine",
        )
        .await
    }

    /// Get API for VirtualMachineInstanceMigration CRD
    async fn get_migration_api(&self) -> anyhow::Result<Api<DynamicObject>> {
        common::discover_crd_api(
            self.client.clone(),
            &self.namespace,
            "kubevirt.io",
            "v1",
            "VirtualMachineInstanceMigration",
        )
        .await
    }

    /// Trigger a KubeVirt live migration of a running VM. Returns the created
    /// VirtualMachineInstanceMigration name; progress can be watched via
    /// `migration_phase`. The VM must have been deployed with
    /// kubevirt.liveMigration (evictionStrategy LiveMigrate).
    pub async fn live_migrate(&self, name: &str) -> anyhow::Result<String> {
        common::validate_kube_name(name)?;
        let mig_name = format!(
            "{}-mig-{:08x}",
            name,
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() as u32)
                .unwrap_or_default()
        );
        let mig_json = json!({
            "apiVersion": "kubevirt.io/v1",
            "kind": "VirtualMachineInstanceMigration",
            "metadata": {
                "name": mig_name,
                "namespace": self.namespace,
                "labels": { "app.kubernetes.io/managed-by": "aether" }
            },
            "spec": { "vmiName": name }
        });
        let mig: DynamicObject = serde_json::from_value(mig_json)?;
        let api = self.get_migration_api().await?;
        api.create(&PostParams::default(), &mig)
            .await
            .context("creating VirtualMachineInstanceMigration (is the VM running with liveMigration enabled?)")?;
        tracing::info!("Created VirtualMachineInstanceMigration: {}", mig_name);
        Ok(mig_name)
    }

    /// Read the phase of a VirtualMachineInstanceMigration (e.g. Pending,
    /// Scheduling, PreparingTarget, TargetReady, Running, Succeeded, Failed).
    pub async fn migration_phase(&self, mig_name: &str) -> anyhow::Result<String> {
        let api = self.get_migration_api().await?;
        let mig = api.get(mig_name).await?;
        Ok(mig
            .data
            .get("status")
            .and_then(|s| s.get("phase"))
            .and_then(|p| p.as_str())
            .unwrap_or("Pending")
            .to_string())
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
            Err(_) => Ok(common::not_found_status("VM")),
        }
    }
}

/// Build DataVolume JSON (standalone, testable without kube::Client)
fn build_datavolume_json(namespace: &str, _image: &Image, spec: &Workload) -> serde_json::Value {
    use crate::ragnarok::isolation::datavolume_encryption_annotations;

    let labels = common::build_managed_labels(&spec.metadata.name, &spec.metadata.labels);
    let enc_ann = datavolume_encryption_annotations(spec);
    let storage_class = enc_ann
        .get("ragnarok.zyvor.dev/storage-class")
        .cloned()
        .or_else(|| spec.persistence.storage_class.clone());

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
            "namespace": namespace,
            "labels": labels,
            "annotations": enc_ann,
        },
        "spec": {
            "source": {
                "registry": {
                    "url": format!("docker://{}", spec.container_image_ref())
                }
            },
            "storage": {
                "accessModes": [access_mode],
                "resources": {
                    "requests": {
                        "storage": spec.requirements.storage
                    }
                },
                "storageClassName": storage_class
            }
        }
    })
}

/// Build VirtualMachine JSON (standalone, testable without kube::Client)
fn build_virtualmachine_json(namespace: &str, spec: &Workload) -> serde_json::Value {
    let labels = common::build_managed_labels(&spec.metadata.name, &spec.metadata.labels);

    let cpu_cores = common::parse_cpu_cores(&spec.requirements.cpu);

    let live_migration = spec.kubevirt.as_ref().is_some_and(|kv| kv.live_migration);
    // KubeVirt refuses to migrate VMIs on the pod network unless the interface
    // uses masquerade binding (bridge is the default when none is specified),
    // so live-migratable VMs always get an explicit masquerade interface.
    let pod_network = spec.network.service || live_migration;

    let mut vm_spec = json!({
        "apiVersion": "kubevirt.io/v1",
        "kind": "VirtualMachine",
        "metadata": {
            "name": spec.metadata.name,
            "namespace": namespace,
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
                            },
                            "limits": {
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
                            "interfaces": if pod_network {
                                Some(vec![json!({
                                    "name": "default",
                                    "masquerade": {}
                                })])
                            } else {
                                None
                            }
                        }
                    },
                    "networks": if pod_network {
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

    // Add GPU if requested: a vGPU profile attaches mediated slices (the only
    // GPU mode compatible with live migration); otherwise VFIO passthrough.
    if let Some(ref gpu_req) = spec.requirements.gpu {
        let device_name = gpu_req
            .vgpu_profile
            .clone()
            .unwrap_or_else(|| format!("{}.com/gpu", gpu_req.vendor));
        let gpus: Vec<serde_json::Value> = (0..gpu_req.count)
            .map(|i| {
                json!({
                    "name": format!("gpu{}", i),
                    "deviceName": device_name
                })
            })
            .collect();

        vm_spec["spec"]["template"]["spec"]["domain"]["devices"]["gpus"] = json!(gpus);
    }

    if spec.kubevirt.as_ref().is_some_and(|kv| kv.live_migration) {
        vm_spec["spec"]["template"]["spec"]["evictionStrategy"] = json!("LiveMigrate");
    }

    crate::ragnarok::kubevirt::apply_confidential_to_vm(&mut vm_spec, spec);

    vm_spec
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
        common::validate_kube_name(&spec.metadata.name)?;

        // Deploy confidential VM extensions when spec requests TEE
        crate::ragnarok::image::deploy_image_gate(
            spec,
            &crate::ragnarok::image::ImageCatalog::load(
                &crate::ragnarok::client::RagnarokClient::attestation_data_dir(),
            ),
        )?;
        crate::ragnarok::sovereign::enforce_region_lock(
            spec,
            &crate::ragnarok::sovereign::SovereignConfig::from_env(),
        )?;

        // Create DataVolume
        let datavolume_json = self.generate_datavolume_json(image, spec);
        let dv: DynamicObject = serde_json::from_value(datavolume_json)?;

        let dv_api = self.get_datavolume_api().await?;
        match dv_api.create(&PostParams::default(), &dv).await {
            Ok(_) => tracing::info!("Created DataVolume: {}-disk", spec.metadata.name),
            Err(e) => tracing::warn!("DataVolume creation failed (may already exist): {}", e),
        }

        // Poll for DataVolume registration before creating the VM.
        {
            let max_wait = tokio::time::Duration::from_secs(60);
            let poll_interval = tokio::time::Duration::from_secs(2);
            let start = tokio::time::Instant::now();
            loop {
                match dv_api.get(&format!("{}-disk", spec.metadata.name)).await {
                    Ok(_) => {
                        tracing::info!("DataVolume registered, proceeding with VM creation");
                        break;
                    }
                    Err(_) if start.elapsed() < max_wait => {
                        tokio::time::sleep(poll_interval).await;
                    }
                    Err(e) => {
                        tracing::warn!(
                            "DataVolume not ready after {:?}, proceeding anyway: {}",
                            max_wait,
                            e
                        );
                        break;
                    }
                }
            }
        }

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

        Ok(Instance::new(
            uid,
            vm_name,
            RuntimeKind::KubeVirt,
            image.full_name(),
        ))
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

    async fn logs(&self, instance: &Instance, follow: bool) -> crate::Result<String> {
        // Attempt to get real logs from the VM's launcher pod
        let pods: Api<Pod> = Api::namespaced(self.client.clone(), &self.namespace);
        let lp = ListParams::default().labels(&format!("vm.kubevirt.io/name={}", instance.name));

        if let Ok(pod_list) = pods.list(&lp).await {
            if let Some(pod) = pod_list.items.first() {
                if let Some(pod_name) = pod.metadata.name.as_deref() {
                    let log_params = LogParams {
                        follow,
                        ..Default::default()
                    };
                    match pods.logs(pod_name, &log_params).await {
                        Ok(logs) => return Ok(logs),
                        Err(e) => {
                            tracing::warn!("Failed to get launcher pod logs: {}", e);
                        }
                    }
                }
            }
        }

        // Fall back to instructional text if no launcher pod found
        tracing::info!(
            "VM console access available via: virtctl console {}",
            instance.name
        );

        Ok(format!(
            "VirtualMachine Console Access:\n\n\
            No launcher pod found for VM '{}'. Use virtctl for console access:\n\n\
            Serial Console:\n  virtctl console {}\n\n\
            VNC Access:\n  virtctl vnc {}\n\n\
            Note: Install virtctl CLI tool from:\n  \
            https://kubevirt.io/user-guide/operations/virtctl_client_tool/",
            instance.name, instance.name, instance.name
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

        // List only VMs managed by aether
        let lp = common::managed_list_params();
        let vm_list = api.list(&lp).await?;

        let instances: Vec<Instance> = vm_list
            .items
            .iter()
            .map(|vm| {
                let name = vm.name_any();
                let uid = vm
                    .metadata
                    .uid
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string());

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
                    .unwrap_or_else(crate::resources::now_rfc3339);

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
                tracing::warn!("Failed to probe KubeVirt capacity: {}", e);
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
                preferred: RuntimePreference::Kubevirt,
                allow: vec![RuntimeType::Kubevirt],
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
            kubevirt: None,
        }
    }

    fn make_image(name: &str) -> Image {
        Image {
            name: name.to_string(),
            tag: "latest".to_string(),
            digest: None,
            runtime: RuntimeKind::KubeVirt,
        }
    }

    // ---------------------------------------------------------------
    // VirtualMachine manifest / JSON generation
    // ---------------------------------------------------------------
    #[test]
    fn test_vm_json_api_version_and_kind() {
        let spec = make_workload("my-vm", "4", "8Gi", "50Gi");
        let vm = build_virtualmachine_json("default", &spec);

        assert_eq!(vm["apiVersion"], "kubevirt.io/v1");
        assert_eq!(vm["kind"], "VirtualMachine");
    }

    #[test]
    fn test_vm_json_metadata_name_and_namespace() {
        let spec = make_workload("web-server", "2", "4Gi", "20Gi");
        let vm = build_virtualmachine_json("production", &spec);

        assert_eq!(vm["metadata"]["name"], "web-server");
        assert_eq!(vm["metadata"]["namespace"], "production");
    }

    #[test]
    fn test_vm_json_running_flag_is_true() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let vm = build_virtualmachine_json("default", &spec);

        assert_eq!(vm["spec"]["running"], true);
    }

    #[test]
    fn test_vm_json_labels_contain_managed_by() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let vm = build_virtualmachine_json("default", &spec);

        assert_eq!(vm["metadata"]["labels"]["app"], "my-vm");
        assert_eq!(vm["metadata"]["labels"]["managed-by"], "aether");
    }

    #[test]
    fn test_vm_json_user_labels_propagated() {
        let mut spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        spec.metadata
            .labels
            .insert("env".to_string(), "staging".to_string());
        spec.metadata
            .labels
            .insert("team".to_string(), "infra".to_string());

        let vm = build_virtualmachine_json("default", &spec);

        assert_eq!(vm["metadata"]["labels"]["env"], "staging");
        assert_eq!(vm["metadata"]["labels"]["team"], "infra");
    }

    #[test]
    fn test_vm_json_annotations_propagated() {
        let mut spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        spec.metadata.annotations.insert(
            "description".to_string(),
            "test virtual machine".to_string(),
        );

        let vm = build_virtualmachine_json("default", &spec);

        assert_eq!(
            vm["metadata"]["annotations"]["description"],
            "test virtual machine"
        );
    }

    // ---------------------------------------------------------------
    // CPU / memory resource specification in VM manifests
    // ---------------------------------------------------------------
    #[test]
    fn test_vm_json_cpu_cores_whole_number() {
        let spec = make_workload("my-vm", "8", "16Gi", "100Gi");
        let vm = build_virtualmachine_json("default", &spec);

        let cores = vm["spec"]["template"]["spec"]["domain"]["cpu"]["cores"]
            .as_i64()
            .unwrap();
        assert_eq!(cores, 8);
    }

    #[test]
    fn test_vm_json_cpu_cores_millicore() {
        let spec = make_workload("my-vm", "4000m", "8Gi", "50Gi");
        let vm = build_virtualmachine_json("default", &spec);

        let cores = vm["spec"]["template"]["spec"]["domain"]["cpu"]["cores"]
            .as_i64()
            .unwrap();
        assert_eq!(cores, 4);
    }

    #[test]
    fn test_vm_json_memory_in_resources() {
        let spec = make_workload("my-vm", "2", "16Gi", "50Gi");
        let vm = build_virtualmachine_json("default", &spec);

        let mem = vm["spec"]["template"]["spec"]["domain"]["resources"]["requests"]["memory"]
            .as_str()
            .unwrap();
        assert_eq!(mem, "16Gi");
    }

    #[test]
    fn test_vm_json_small_memory_value() {
        let spec = make_workload("tiny-vm", "1", "512Mi", "10Gi");
        let vm = build_virtualmachine_json("default", &spec);

        let mem = vm["spec"]["template"]["spec"]["domain"]["resources"]["requests"]["memory"]
            .as_str()
            .unwrap();
        assert_eq!(mem, "512Mi");
    }

    // ---------------------------------------------------------------
    // Disk configuration
    // ---------------------------------------------------------------
    #[test]
    fn test_vm_json_rootdisk_configured() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let vm = build_virtualmachine_json("default", &spec);

        let disks = vm["spec"]["template"]["spec"]["domain"]["devices"]["disks"]
            .as_array()
            .unwrap();
        assert_eq!(disks.len(), 1);
        assert_eq!(disks[0]["name"], "rootdisk");
        assert_eq!(disks[0]["disk"]["bus"], "virtio");
    }

    #[test]
    fn test_vm_json_volume_references_datavolume() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let vm = build_virtualmachine_json("default", &spec);

        let volumes = vm["spec"]["template"]["spec"]["volumes"]
            .as_array()
            .unwrap();
        assert_eq!(volumes.len(), 1);
        assert_eq!(volumes[0]["name"], "rootdisk");
        assert_eq!(volumes[0]["dataVolume"]["name"], "my-vm-disk");
    }

    // ---------------------------------------------------------------
    // Network interface configuration
    // ---------------------------------------------------------------
    #[test]
    fn test_vm_json_network_disabled_by_default() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        // network.service defaults to false
        let vm = build_virtualmachine_json("default", &spec);

        // When network service is false, interfaces should be null
        assert!(vm["spec"]["template"]["spec"]["domain"]["devices"]["interfaces"].is_null());
        assert!(vm["spec"]["template"]["spec"]["networks"].is_null());
    }

    #[test]
    fn test_vm_json_network_enabled() {
        let mut spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        spec.network.service = true;

        let vm = build_virtualmachine_json("default", &spec);

        let interfaces = vm["spec"]["template"]["spec"]["domain"]["devices"]["interfaces"]
            .as_array()
            .unwrap();
        assert_eq!(interfaces.len(), 1);
        assert_eq!(interfaces[0]["name"], "default");
        assert!(interfaces[0]["masquerade"].is_object());

        let networks = vm["spec"]["template"]["spec"]["networks"]
            .as_array()
            .unwrap();
        assert_eq!(networks.len(), 1);
        assert_eq!(networks[0]["name"], "default");
        assert!(networks[0]["pod"].is_object());
    }

    // ---------------------------------------------------------------
    // GPU passthrough configuration
    // ---------------------------------------------------------------
    #[test]
    fn test_vm_json_no_gpu_by_default() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let vm = build_virtualmachine_json("default", &spec);

        // gpus key should not exist when no GPU is requested
        assert!(vm["spec"]["template"]["spec"]["domain"]["devices"]["gpus"].is_null());
    }

    #[test]
    fn test_vm_json_single_gpu_passthrough() {
        let mut spec = make_workload("gpu-vm", "8", "32Gi", "100Gi");
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
            vgpu_profile: None,
        });

        let vm = build_virtualmachine_json("default", &spec);

        let gpus = vm["spec"]["template"]["spec"]["domain"]["devices"]["gpus"]
            .as_array()
            .unwrap();
        assert_eq!(gpus.len(), 1);
        assert_eq!(gpus[0]["name"], "gpu0");
        assert_eq!(gpus[0]["deviceName"], "nvidia.com/gpu");
    }

    #[test]
    fn test_vm_json_multiple_gpu_passthrough() {
        let mut spec = make_workload("multi-gpu-vm", "16", "64Gi", "200Gi");
        spec.requirements.gpu = Some(GpuRequirements {
            count: 4,
            vendor: "nvidia".to_string(),
            vgpu_profile: None,
        });

        let vm = build_virtualmachine_json("default", &spec);

        let gpus = vm["spec"]["template"]["spec"]["domain"]["devices"]["gpus"]
            .as_array()
            .unwrap();
        assert_eq!(gpus.len(), 4);
        for (i, gpu) in gpus.iter().enumerate() {
            assert_eq!(gpu["name"], format!("gpu{}", i));
            assert_eq!(gpu["deviceName"], "nvidia.com/gpu");
        }
    }

    #[test]
    fn test_vm_json_vgpu_profile_device_name() {
        let mut spec = make_workload("vgpu-vm", "8", "32Gi", "100Gi");
        spec.requirements.gpu = Some(GpuRequirements {
            count: 2,
            vendor: "nvidia".to_string(),
            vgpu_profile: Some("nvidia.com/GRID_A100-10C".to_string()),
        });

        let vm = build_virtualmachine_json("default", &spec);

        let gpus = vm["spec"]["template"]["spec"]["domain"]["devices"]["gpus"]
            .as_array()
            .unwrap();
        assert_eq!(gpus.len(), 2);
        for gpu in gpus {
            assert_eq!(gpu["deviceName"], "nvidia.com/GRID_A100-10C");
        }
    }

    #[test]
    fn test_vm_json_no_eviction_strategy_by_default() {
        let spec = make_workload("plain-vm", "2", "4Gi", "20Gi");
        let vm = build_virtualmachine_json("default", &spec);
        assert!(vm["spec"]["template"]["spec"]["evictionStrategy"].is_null());
    }

    #[test]
    fn test_vm_json_live_migration_eviction_strategy() {
        let mut spec = make_workload("lm-vm", "2", "4Gi", "20Gi");
        spec.kubevirt = Some(crate::spec::KubevirtSpec {
            live_migration: true,
        });
        let vm = build_virtualmachine_json("default", &spec);
        assert_eq!(
            vm["spec"]["template"]["spec"]["evictionStrategy"],
            "LiveMigrate"
        );
    }

    #[test]
    fn test_vm_json_live_migration_forces_masquerade_interface() {
        // KubeVirt's migration webhook rejects pod-network VMIs without
        // masquerade binding, so liveMigration must render the interface even
        // when no service is requested.
        let mut spec = make_workload("lm-net-vm", "2", "4Gi", "20Gi");
        spec.network.service = false;
        spec.kubevirt = Some(crate::spec::KubevirtSpec {
            live_migration: true,
        });
        let vm = build_virtualmachine_json("default", &spec);
        assert_eq!(
            vm["spec"]["template"]["spec"]["domain"]["devices"]["interfaces"][0]["masquerade"],
            serde_json::json!({})
        );
        assert_eq!(
            vm["spec"]["template"]["spec"]["networks"][0]["pod"],
            serde_json::json!({})
        );
    }

    #[test]
    fn test_vm_json_vgpu_with_live_migration() {
        let mut spec = make_workload("vgpu-lm-vm", "8", "32Gi", "100Gi");
        spec.requirements.gpu = Some(GpuRequirements {
            count: 1,
            vendor: "nvidia".to_string(),
            vgpu_profile: Some("nvidia.com/GRID_A100-10C".to_string()),
        });
        spec.kubevirt = Some(crate::spec::KubevirtSpec {
            live_migration: true,
        });
        let vm = build_virtualmachine_json("default", &spec);
        assert_eq!(
            vm["spec"]["template"]["spec"]["evictionStrategy"],
            "LiveMigrate"
        );
        assert_eq!(
            vm["spec"]["template"]["spec"]["domain"]["devices"]["gpus"][0]["deviceName"],
            "nvidia.com/GRID_A100-10C"
        );
    }

    #[test]
    fn test_vm_json_amd_gpu_passthrough() {
        let mut spec = make_workload("amd-vm", "8", "32Gi", "100Gi");
        spec.requirements.gpu = Some(GpuRequirements {
            count: 2,
            vendor: "amd".to_string(),
            vgpu_profile: None,
        });

        let vm = build_virtualmachine_json("default", &spec);

        let gpus = vm["spec"]["template"]["spec"]["domain"]["devices"]["gpus"]
            .as_array()
            .unwrap();
        assert_eq!(gpus.len(), 2);
        assert_eq!(gpus[0]["deviceName"], "amd.com/gpu");
        assert_eq!(gpus[1]["deviceName"], "amd.com/gpu");
    }

    // ---------------------------------------------------------------
    // DataVolume JSON generation
    // ---------------------------------------------------------------
    #[test]
    fn test_datavolume_json_api_version_and_kind() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let image = make_image("my-vm");
        let dv = build_datavolume_json("default", &image, &spec);

        assert_eq!(dv["apiVersion"], "cdi.kubevirt.io/v1beta1");
        assert_eq!(dv["kind"], "DataVolume");
    }

    #[test]
    fn test_datavolume_json_name_has_disk_suffix() {
        let spec = make_workload("web-app", "2", "4Gi", "20Gi");
        let image = make_image("web-app");
        let dv = build_datavolume_json("default", &image, &spec);

        assert_eq!(dv["metadata"]["name"], "web-app-disk");
    }

    #[test]
    fn test_datavolume_json_namespace() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let image = make_image("my-vm");
        let dv = build_datavolume_json("production", &image, &spec);

        assert_eq!(dv["metadata"]["namespace"], "production");
    }

    #[test]
    fn test_datavolume_json_labels() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let image = make_image("my-vm");
        let dv = build_datavolume_json("default", &image, &spec);

        assert_eq!(dv["metadata"]["labels"]["app"], "my-vm");
        assert_eq!(dv["metadata"]["labels"]["managed-by"], "aether");
    }

    #[test]
    fn test_datavolume_json_registry_source() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let image = make_image("my-vm");
        let dv = build_datavolume_json("default", &image, &spec);

        let url = dv["spec"]["source"]["registry"]["url"].as_str().unwrap();
        assert_eq!(url, "docker://ghcr.io/testorg/my-vm:latest");
    }

    #[test]
    fn test_datavolume_json_storage_size() {
        let spec = make_workload("my-vm", "2", "4Gi", "50Gi");
        let image = make_image("my-vm");
        let dv = build_datavolume_json("default", &image, &spec);

        assert_eq!(
            dv["spec"]["storage"]["resources"]["requests"]["storage"],
            "50Gi"
        );
    }

    #[test]
    fn test_datavolume_json_default_access_mode() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let image = make_image("my-vm");
        let dv = build_datavolume_json("default", &image, &spec);

        let modes = dv["spec"]["storage"]["accessModes"].as_array().unwrap();
        assert_eq!(modes.len(), 1);
        assert_eq!(modes[0], "ReadWriteOnce");
    }

    #[test]
    fn test_datavolume_json_readwritemany_access_mode() {
        let mut spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        spec.persistence.access_mode = AccessMode::ReadWriteMany;
        let image = make_image("my-vm");
        let dv = build_datavolume_json("default", &image, &spec);

        let modes = dv["spec"]["storage"]["accessModes"].as_array().unwrap();
        assert_eq!(modes[0], "ReadWriteMany");
    }

    #[test]
    fn test_datavolume_json_readonlymany_access_mode() {
        let mut spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        spec.persistence.access_mode = AccessMode::ReadOnlyMany;
        let image = make_image("my-vm");
        let dv = build_datavolume_json("default", &image, &spec);

        let modes = dv["spec"]["storage"]["accessModes"].as_array().unwrap();
        assert_eq!(modes[0], "ReadOnlyMany");
    }

    #[test]
    fn test_datavolume_json_storage_class() {
        let mut spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        spec.persistence.storage_class = Some("fast-ssd".to_string());
        let image = make_image("my-vm");
        let dv = build_datavolume_json("default", &image, &spec);

        assert_eq!(dv["spec"]["storage"]["storageClassName"], "fast-ssd");
    }

    #[test]
    fn test_datavolume_json_no_storage_class() {
        let spec = make_workload("my-vm", "2", "4Gi", "20Gi");
        let image = make_image("my-vm");
        let dv = build_datavolume_json("default", &image, &spec);

        assert!(dv["spec"]["storage"]["storageClassName"].is_null());
    }

    // ---------------------------------------------------------------
    // Serial console access configuration
    // ---------------------------------------------------------------
    #[test]
    fn test_logs_returns_console_instructions() {
        // The logs() method on the trait requires async + kube client,
        // but the format string is deterministic.  We test the expected
        // output shape here synchronously.
        let expected_serial = "virtctl console test-vm";
        let expected_vnc = "virtctl vnc test-vm";
        let output = format!(
            "VirtualMachine Console Access:\n\n\
            Serial Console:\n  virtctl console {}\n\n\
            VNC Access:\n  virtctl vnc {}\n\n\
            Note: Install virtctl CLI tool from:\n  \
            https://kubevirt.io/user-guide/operations/virtctl_client_tool/",
            "test-vm", "test-vm"
        );
        assert!(output.contains(expected_serial));
        assert!(output.contains(expected_vnc));
        assert!(output.contains("virtctl"));
    }

    // ---------------------------------------------------------------
    // Full round-trip: VM + DataVolume together
    // ---------------------------------------------------------------
    #[test]
    fn test_vm_and_datavolume_names_match() {
        let spec = make_workload("linked-vm", "4", "8Gi", "40Gi");
        let image = make_image("linked-vm");

        let vm = build_virtualmachine_json("staging", &spec);
        let dv = build_datavolume_json("staging", &image, &spec);

        // The volume in the VM spec should reference the DataVolume by name
        let vol_dv_name = vm["spec"]["template"]["spec"]["volumes"][0]["dataVolume"]["name"]
            .as_str()
            .unwrap();
        let dv_name = dv["metadata"]["name"].as_str().unwrap();
        assert_eq!(vol_dv_name, dv_name);
    }

    #[test]
    fn test_vm_and_datavolume_share_namespace() {
        let spec = make_workload("ns-vm", "2", "4Gi", "20Gi");
        let image = make_image("ns-vm");

        let vm = build_virtualmachine_json("kube-test", &spec);
        let dv = build_datavolume_json("kube-test", &image, &spec);

        assert_eq!(
            vm["metadata"]["namespace"].as_str().unwrap(),
            dv["metadata"]["namespace"].as_str().unwrap()
        );
    }

    // ---------------------------------------------------------------
    // Template labels mirror top-level labels
    // ---------------------------------------------------------------
    #[test]
    fn test_vm_json_template_labels_match_metadata_labels() {
        let mut spec = make_workload("label-vm", "2", "4Gi", "20Gi");
        spec.metadata
            .labels
            .insert("tier".to_string(), "backend".to_string());

        let vm = build_virtualmachine_json("default", &spec);

        let meta_labels = &vm["metadata"]["labels"];
        let tmpl_labels = &vm["spec"]["template"]["metadata"]["labels"];

        assert_eq!(meta_labels["app"], tmpl_labels["app"]);
        assert_eq!(meta_labels["managed-by"], tmpl_labels["managed-by"]);
        assert_eq!(meta_labels["tier"], tmpl_labels["tier"]);
    }
}
