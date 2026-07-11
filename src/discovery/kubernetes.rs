// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Metadata-preserving Kubernetes discovery.
//!
//! Unlike `kubecluster.rs` (which returns flat rows for browsing), this pass keeps
//! the labels, annotations, owner references, and pod-spec details that application
//! grouping and migration-class classification depend on. Secret **values** are
//! never read — only references (names).

use anyhow::Result;
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    Namespace, PersistentVolumeClaim, Pod, PodSpec, Service, ServiceAccount,
};
use k8s_openapi::api::networking::v1::Ingress;
use k8s_openapi::api::policy::v1::PodDisruptionBudget;
use k8s_openapi::api::storage::v1::StorageClass;
use kube::api::ListParams;
use kube::{Api, Client};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

// ── Discovered model ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwnerRef {
    pub kind: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    /// Whether the container declares CPU/memory requests.
    pub has_requests: bool,
    /// Declared CPU/memory request quantities (when present).
    #[serde(default)]
    pub cpu_request: Option<String>,
    #[serde(default)]
    pub memory_request: Option<String>,
    /// Whether the container requests a GPU (e.g. nvidia.com/gpu).
    pub wants_gpu: bool,
    /// Whether the container's securityContext is privileged.
    pub privileged: bool,
}

/// Pod-level portability signals used for classification.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PodSignals {
    pub host_network: bool,
    pub host_pid: bool,
    pub host_path_volumes: Vec<String>,
    pub node_name: Option<String>,
    pub service_account: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredWorkload {
    pub namespace: String,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub annotations: BTreeMap<String, String>,
    #[serde(default)]
    pub owner_refs: Vec<OwnerRef>,
    pub replicas: i32,
    #[serde(default)]
    pub containers: Vec<ContainerInfo>,
    #[serde(default)]
    pub signals: PodSignals,
    #[serde(default)]
    pub config_map_refs: Vec<String>,
    #[serde(default)]
    pub secret_refs: Vec<String>,
    #[serde(default)]
    pub pvc_refs: Vec<String>,
    /// Env values that look like external endpoints (host:port or URL).
    #[serde(default)]
    pub env_endpoints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub namespace: String,
    pub name: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub selector: BTreeMap<String, String>,
    pub type_: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngressInfo {
    pub namespace: String,
    pub name: String,
    #[serde(default)]
    pub hosts: Vec<String>,
    #[serde(default)]
    pub backend_services: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvcInfo {
    pub namespace: String,
    pub name: String,
    pub storage_class: Option<String>,
    #[serde(default)]
    pub access_modes: Vec<String>,
    pub size: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageClassInfo {
    pub name: String,
    pub provisioner: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RawInventory {
    pub connection: String,
    pub discovered_at: String,
    #[serde(default)]
    pub namespaces: Vec<String>,
    #[serde(default)]
    pub workloads: Vec<DiscoveredWorkload>,
    #[serde(default)]
    pub services: Vec<ServiceInfo>,
    #[serde(default)]
    pub ingresses: Vec<IngressInfo>,
    #[serde(default)]
    pub pvcs: Vec<PvcInfo>,
    #[serde(default)]
    pub storage_classes: Vec<StorageClassInfo>,
    #[serde(default)]
    pub pdbs: Vec<String>,
    #[serde(default)]
    pub service_accounts: Vec<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
}

// ── Discovery ───────────────────────────────────────────────────────────────

/// Discover a cluster into a metadata-rich [`RawInventory`]. When `namespaces`
/// is empty, every namespace is scanned. `connection` labels the snapshot.
pub async fn discover(
    client: &Client,
    connection: &str,
    namespaces: &[String],
) -> Result<RawInventory> {
    let mut inv = RawInventory {
        connection: connection.to_string(),
        discovered_at: crate::resources::now_rfc3339(),
        ..Default::default()
    };

    // Namespaces to scan.
    let ns_list: Vec<String> = if namespaces.is_empty() {
        let api: Api<Namespace> = Api::all(client.clone());
        match api.list(&ListParams::default()).await {
            Ok(list) => list.items.into_iter().filter_map(|n| n.metadata.name).collect(),
            Err(e) => {
                inv.warnings.push(format!("list namespaces failed: {e}"));
                Vec::new()
            }
        }
    } else {
        namespaces.to_vec()
    };
    inv.namespaces = ns_list.clone();

    // Cluster-scoped: storage classes.
    let sc: Api<StorageClass> = Api::all(client.clone());
    if let Ok(list) = sc.list(&ListParams::default()).await {
        for c in list.items {
            let is_default = c
                .metadata
                .annotations
                .as_ref()
                .and_then(|a| a.get("storageclass.kubernetes.io/is-default-class"))
                .map(|v| v == "true")
                .unwrap_or(false);
            inv.storage_classes.push(StorageClassInfo {
                name: c.metadata.name.unwrap_or_default(),
                provisioner: c.provisioner,
                is_default,
            });
        }
    }

    for ns in &ns_list {
        discover_namespace(client, ns, &mut inv).await;
    }

    Ok(inv)
}

async fn discover_namespace(client: &Client, ns: &str, inv: &mut RawInventory) {
    // Controllers with pod templates.
    let deploys: Api<Deployment> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = deploys.list(&ListParams::default()).await {
        for d in list.items {
            let replicas = d.spec.as_ref().and_then(|s| s.replicas).unwrap_or(1);
            let podspec = d.spec.as_ref().and_then(|s| s.template.spec.as_ref());
            inv.workloads.push(build_workload(
                ns, "Deployment", &d.metadata, replicas, podspec,
            ));
        }
    }
    let sts: Api<StatefulSet> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = sts.list(&ListParams::default()).await {
        for s in list.items {
            let replicas = s.spec.as_ref().and_then(|sp| sp.replicas).unwrap_or(1);
            let podspec = s.spec.as_ref().and_then(|sp| sp.template.spec.as_ref());
            let mut w = build_workload(ns, "StatefulSet", &s.metadata, replicas, podspec);
            // StatefulSet volumeClaimTemplates count as PVC usage.
            if let Some(vcts) = s.spec.as_ref().and_then(|sp| sp.volume_claim_templates.as_ref()) {
                for vct in vcts {
                    if let Some(n) = &vct.metadata.name {
                        w.pvc_refs.push(n.clone());
                    }
                }
            }
            inv.workloads.push(w);
        }
    }
    let ds: Api<DaemonSet> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = ds.list(&ListParams::default()).await {
        for d in list.items {
            let podspec = d.spec.as_ref().and_then(|s| s.template.spec.as_ref());
            inv.workloads
                .push(build_workload(ns, "DaemonSet", &d.metadata, 1, podspec));
        }
    }
    let jobs: Api<Job> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = jobs.list(&ListParams::default()).await {
        for j in list.items {
            // Skip Jobs owned by a CronJob (avoid duplicate churn).
            if owner_kinds(&j.metadata).iter().any(|k| k == "CronJob") {
                continue;
            }
            let podspec = j.spec.as_ref().and_then(|s| s.template.spec.as_ref());
            inv.workloads
                .push(build_workload(ns, "Job", &j.metadata, 1, podspec));
        }
    }
    let cronjobs: Api<CronJob> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = cronjobs.list(&ListParams::default()).await {
        for c in list.items {
            let podspec = c
                .spec
                .as_ref()
                .and_then(|s| s.job_template.spec.as_ref())
                .and_then(|jt| jt.template.spec.as_ref());
            inv.workloads
                .push(build_workload(ns, "CronJob", &c.metadata, 1, podspec));
        }
    }
    // Naked pods (no controller owner).
    let pods: Api<Pod> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = pods.list(&ListParams::default()).await {
        for p in list.items {
            if !owner_kinds(&p.metadata).is_empty() {
                continue; // owned by a controller → represented above
            }
            inv.workloads
                .push(build_workload(ns, "Pod", &p.metadata, 1, p.spec.as_ref()));
        }
    }

    // Services.
    let svcs: Api<Service> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = svcs.list(&ListParams::default()).await {
        for s in list.items {
            inv.services.push(ServiceInfo {
                namespace: ns.to_string(),
                name: s.metadata.name.unwrap_or_default(),
                labels: s.metadata.labels.unwrap_or_default().into_iter().collect(),
                selector: s
                    .spec
                    .as_ref()
                    .and_then(|sp| sp.selector.clone())
                    .unwrap_or_default()
                    .into_iter()
                    .collect(),
                type_: s
                    .spec
                    .as_ref()
                    .and_then(|sp| sp.type_.clone())
                    .unwrap_or_else(|| "ClusterIP".to_string()),
            });
        }
    }

    // Ingresses.
    let ings: Api<Ingress> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = ings.list(&ListParams::default()).await {
        for i in list.items {
            let mut hosts = Vec::new();
            let mut backends = Vec::new();
            if let Some(spec) = &i.spec {
                for rule in spec.rules.iter().flatten() {
                    if let Some(h) = &rule.host {
                        hosts.push(h.clone());
                    }
                    if let Some(http) = &rule.http {
                        for path in &http.paths {
                            if let Some(svc) = &path.backend.service {
                                backends.push(svc.name.clone());
                            }
                        }
                    }
                }
            }
            inv.ingresses.push(IngressInfo {
                namespace: ns.to_string(),
                name: i.metadata.name.unwrap_or_default(),
                hosts,
                backend_services: backends,
            });
        }
    }

    // PVCs.
    let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = pvcs.list(&ListParams::default()).await {
        for p in list.items {
            let spec = p.spec.as_ref();
            inv.pvcs.push(PvcInfo {
                namespace: ns.to_string(),
                name: p.metadata.name.unwrap_or_default(),
                storage_class: spec.and_then(|s| s.storage_class_name.clone()),
                access_modes: spec.and_then(|s| s.access_modes.clone()).unwrap_or_default(),
                size: spec
                    .and_then(|s| s.resources.as_ref())
                    .and_then(|r| r.requests.as_ref())
                    .and_then(|r| r.get("storage"))
                    .map(|q| q.0.clone()),
            });
        }
    }

    // PDBs and ServiceAccounts (names only).
    let pdbs: Api<PodDisruptionBudget> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = pdbs.list(&ListParams::default()).await {
        for p in list.items {
            inv.pdbs
                .push(format!("{}/{}", ns, p.metadata.name.unwrap_or_default()));
        }
    }
    let sas: Api<ServiceAccount> = Api::namespaced(client.clone(), ns);
    if let Ok(list) = sas.list(&ListParams::default()).await {
        for s in list.items {
            inv.service_accounts
                .push(format!("{}/{}", ns, s.metadata.name.unwrap_or_default()));
        }
    }
}

// ── Extraction helpers ──────────────────────────────────────────────────────

fn owner_kinds(meta: &k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta) -> Vec<String> {
    meta.owner_references
        .iter()
        .flatten()
        .map(|o| o.kind.clone())
        .collect()
}

fn build_workload(
    ns: &str,
    kind: &str,
    meta: &k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta,
    replicas: i32,
    podspec: Option<&PodSpec>,
) -> DiscoveredWorkload {
    let mut w = DiscoveredWorkload {
        namespace: ns.to_string(),
        kind: kind.to_string(),
        name: meta.name.clone().unwrap_or_default(),
        labels: meta.labels.clone().unwrap_or_default().into_iter().collect(),
        annotations: meta
            .annotations
            .clone()
            .unwrap_or_default()
            .into_iter()
            .collect(),
        owner_refs: meta
            .owner_references
            .iter()
            .flatten()
            .map(|o| OwnerRef {
                kind: o.kind.clone(),
                name: o.name.clone(),
            })
            .collect(),
        replicas,
        containers: Vec::new(),
        signals: PodSignals::default(),
        config_map_refs: Vec::new(),
        secret_refs: Vec::new(),
        pvc_refs: Vec::new(),
        env_endpoints: Vec::new(),
    };
    if let Some(ps) = podspec {
        extract_podspec(ps, &mut w);
    }
    w
}

fn extract_podspec(ps: &PodSpec, w: &mut DiscoveredWorkload) {
    w.signals.host_network = ps.host_network.unwrap_or(false);
    w.signals.host_pid = ps.host_pid.unwrap_or(false);
    w.signals.node_name = ps.node_name.clone();
    w.signals.service_account = ps.service_account_name.clone();

    for v in ps.volumes.iter().flatten() {
        if let Some(pvc) = &v.persistent_volume_claim {
            w.pvc_refs.push(pvc.claim_name.clone());
        }
        if let Some(cm) = &v.config_map {
            if !cm.name.is_empty() {
                w.config_map_refs.push(cm.name.clone());
            }
        }
        if let Some(s) = &v.secret {
            if let Some(n) = &s.secret_name {
                w.secret_refs.push(n.clone());
            }
        }
        if let Some(hp) = &v.host_path {
            w.signals.host_path_volumes.push(hp.path.clone());
        }
    }

    let all_containers = ps
        .containers
        .iter()
        .chain(ps.init_containers.iter().flatten());
    for c in all_containers {
        let requests = c.resources.as_ref().and_then(|r| r.requests.as_ref());
        let has_requests = requests.map(|r| !r.is_empty()).unwrap_or(false);
        let cpu_request = requests.and_then(|r| r.get("cpu")).map(|q| q.0.clone());
        let memory_request = requests.and_then(|r| r.get("memory")).map(|q| q.0.clone());
        let wants_gpu = c
            .resources
            .as_ref()
            .and_then(|r| r.limits.as_ref())
            .map(|l| l.keys().any(|k| k.contains("gpu")))
            .unwrap_or(false);
        let privileged = c
            .security_context
            .as_ref()
            .and_then(|s| s.privileged)
            .unwrap_or(false);
        w.containers.push(ContainerInfo {
            name: c.name.clone(),
            image: c.image.clone().unwrap_or_default(),
            has_requests,
            cpu_request,
            memory_request,
            wants_gpu,
            privileged,
        });

        // env references + endpoint-looking values.
        for e in c.env.iter().flatten() {
            if let Some(src) = &e.value_from {
                if let Some(cmr) = &src.config_map_key_ref {
                    if !cmr.name.is_empty() {
                        w.config_map_refs.push(cmr.name.clone());
                    }
                }
                if let Some(sr) = &src.secret_key_ref {
                    if !sr.name.is_empty() {
                        w.secret_refs.push(sr.name.clone());
                    }
                }
            }
            if let Some(val) = &e.value {
                if looks_like_endpoint(val) {
                    w.env_endpoints.push(val.clone());
                }
            }
        }
        for ef in c.env_from.iter().flatten() {
            if let Some(cmr) = &ef.config_map_ref {
                if !cmr.name.is_empty() {
                    w.config_map_refs.push(cmr.name.clone());
                }
            }
            if let Some(sr) = &ef.secret_ref {
                if !sr.name.is_empty() {
                    w.secret_refs.push(sr.name.clone());
                }
            }
        }
    }

    w.config_map_refs.sort();
    w.config_map_refs.dedup();
    w.secret_refs.sort();
    w.secret_refs.dedup();
    w.pvc_refs.sort();
    w.pvc_refs.dedup();
}

/// Heuristic: does an env value look like an external endpoint (URL or host:port)?
fn looks_like_endpoint(v: &str) -> bool {
    let v = v.trim();
    if v.len() < 4 || v.len() > 512 {
        return false;
    }
    v.starts_with("http://")
        || v.starts_with("https://")
        || v.starts_with("postgres://")
        || v.starts_with("mysql://")
        || v.starts_with("mongodb://")
        || v.starts_with("redis://")
        || v.starts_with("amqp://")
        || (v.contains('.') && v.contains(':') && !v.contains(' '))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_looks_like_endpoint() {
        assert!(looks_like_endpoint("https://api.stripe.com/v1"));
        assert!(looks_like_endpoint("mydb.abc123.us-east-1.rds.amazonaws.com:5432"));
        assert!(looks_like_endpoint("postgres://host:5432/db"));
        assert!(!looks_like_endpoint("true"));
        assert!(!looks_like_endpoint("8080"));
        assert!(!looks_like_endpoint("hello world"));
    }
}
