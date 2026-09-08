// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Cross-cluster **Move** executor for stateless (Class-A) applications.
//!
//! Reads the app's objects from the persisted discovery snapshot, transforms them
//! for the target, mirrors images, applies to the target cluster (shadow, no
//! external traffic), and records a resumable `MoveRun` for cutover/rollback.

use crate::discovery::ConnectionStore;
use crate::inventory::InventorySnapshot;
use crate::migration::image_mirror::{mirror_images, ImageMap};
use crate::migration::transform::{transform, TransformNote, TransformOptions};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// Inputs for a Move.
#[derive(Debug, Clone)]
pub struct MoveRequest {
    pub app: String,
    pub source_conn: String,
    pub target_conn: String,
    pub target_namespace: String,
    pub registry: Option<String>,
    pub strategy: String,
}

/// One transformed object to apply.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePlanItem {
    pub kind: String,
    pub name: String,
    pub manifest: Value,
}

/// The dry-run result: what would be applied and how it was transformed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovePlan {
    pub app: String,
    pub target_namespace: String,
    pub items: Vec<MovePlanItem>,
    pub images: Vec<ImageMap>,
    pub notes: Vec<TransformNote>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MovePhase {
    Deployed,
    CutOver,
    RolledBack,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRef {
    pub api_version: String,
    pub kind: String,
    pub name: String,
}

/// Persisted state of a Move so cutover/rollback/status are resumable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveRun {
    pub app: String,
    pub source_conn: String,
    pub target_conn: String,
    pub target_namespace: String,
    pub phase: MovePhase,
    pub applied: Vec<ResourceRef>,
    #[serde(default)]
    pub notes: Vec<TransformNote>,
    pub started_at: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MoveStore {
    #[serde(default)]
    pub runs: BTreeMap<String, MoveRun>,
}

crate::impl_json_store!(MoveStore, "moves.json");

// ── Planning ────────────────────────────────────────────────────────────────

fn obj_kind(v: &Value) -> String {
    v.get("kind")
        .and_then(|k| k.as_str())
        .unwrap_or("")
        .to_string()
}
fn obj_api_version(v: &Value) -> String {
    v.get("apiVersion")
        .and_then(|k| k.as_str())
        .unwrap_or("v1")
        .to_string()
}

/// `apiVersion` + `kind` → GroupVersionKind (core group is empty).
fn gvk(api_version: &str, kind: &str) -> kube::core::GroupVersionKind {
    let (group, version) = match api_version.split_once('/') {
        Some((g, v)) => (g.to_string(), v.to_string()),
        None => (String::new(), api_version.to_string()),
    };
    kube::core::GroupVersionKind::gvk(&group, &version, kind)
}

/// Server-side apply an object to the target cluster (create-or-update), resolving
/// the resource dynamically so any kind works — the `kubectl apply` equivalent.
async fn apply_object(client: &kube::Client, namespace: &str, obj: &Value) -> Result<()> {
    use kube::api::{Api, Patch, PatchParams};
    use kube::core::DynamicObject;

    let gvk = gvk(&obj_api_version(obj), &obj_kind(obj));
    let (ar, caps) = kube::discovery::pinned_kind(client, &gvk)
        .await
        .with_context(|| format!("resolving {}/{}", gvk.group, gvk.kind))?;
    let dobj: DynamicObject = serde_json::from_value(obj.clone())?;
    let name = dobj
        .metadata
        .name
        .clone()
        .ok_or_else(|| anyhow::anyhow!("manifest has no metadata.name"))?;
    let api: Api<DynamicObject> = if caps.scope == kube::discovery::Scope::Cluster {
        Api::all_with(client.clone(), &ar)
    } else {
        Api::namespaced_with(client.clone(), namespace, &ar)
    };
    api.patch(
        &name,
        &PatchParams::apply("aether-move").force(),
        &Patch::Apply(&dobj),
    )
    .await?;
    Ok(())
}

/// Delete an object from the target cluster (best-effort).
async fn delete_object(client: &kube::Client, namespace: &str, r: &ResourceRef) -> Result<()> {
    use kube::api::{Api, DeleteParams};
    use kube::core::DynamicObject;

    let gvk = gvk(&r.api_version, &r.kind);
    let (ar, caps) = kube::discovery::pinned_kind(client, &gvk).await?;
    let api: Api<DynamicObject> = if caps.scope == kube::discovery::Scope::Cluster {
        Api::all_with(client.clone(), &ar)
    } else {
        Api::namespaced_with(client.clone(), namespace, &ar)
    };
    api.delete(&r.name, &DeleteParams::default()).await?;
    Ok(())
}
fn obj_name(v: &Value) -> String {
    v.get("metadata")
        .and_then(|m| m.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string()
}
fn cm_matches(v: &Value, ns: &str, name: &str) -> bool {
    let m = v.get("metadata");
    m.and_then(|m| m.get("name")).and_then(|n| n.as_str()) == Some(name)
        && m.and_then(|m| m.get("namespace"))
            .and_then(|n| n.as_str())
            .unwrap_or(ns)
            == ns
}

/// Build the Move plan (dry-run): resolve the app's objects from the snapshot,
/// mirror-plan images, and transform each object for the target.
pub async fn plan_move(snapshot: &InventorySnapshot, req: &MoveRequest) -> Result<MovePlan> {
    let app = snapshot
        .find_application(&req.app)
        .ok_or_else(|| anyhow::anyhow!("application '{}' not found", req.app))?;

    // Collect source manifests: workloads, services, referenced configmaps.
    let mut raw_objects: Vec<Value> = Vec::new();
    let mut images: BTreeSet<String> = BTreeSet::new();

    for wref in &app.workloads {
        let (kind, name) = wref.split_once('/').unwrap_or(("", wref.as_str()));
        if let Some(w) = snapshot
            .raw
            .workloads
            .iter()
            .find(|w| w.namespace == app.namespace && w.kind == kind && w.name == name)
        {
            for c in &w.containers {
                if !c.image.is_empty() {
                    images.insert(c.image.clone());
                }
            }
            if let Some(m) = &w.manifest {
                raw_objects.push(m.clone());
            }
        }
    }
    for sname in &app.services {
        if let Some(s) = snapshot
            .raw
            .services
            .iter()
            .find(|s| s.namespace == app.namespace && &s.name == sname)
        {
            if let Some(m) = &s.manifest {
                raw_objects.push(m.clone());
            }
        }
    }
    for cm in &app.config_map_refs {
        if let Some(m) = snapshot
            .raw
            .config_maps
            .iter()
            .find(|c| cm_matches(c, &app.namespace, cm))
        {
            raw_objects.push(m.clone());
        }
    }

    // Image mirror plan (dry — the plan never copies).
    let image_list: Vec<String> = images.into_iter().collect();
    let image_maps = match &req.registry {
        Some(reg) => mirror_images(&image_list, reg, true).await?,
        None => Vec::new(),
    };
    let image_map: BTreeMap<String, String> = image_maps
        .iter()
        .map(|m| (m.source.clone(), m.target.clone()))
        .collect();

    let opts = TransformOptions {
        target_namespace: Some(req.target_namespace.clone()),
        image_map,
        endpoint_map: BTreeMap::new(),
    };

    let mut items = Vec::new();
    let mut notes = Vec::new();
    for mut obj in raw_objects {
        notes.extend(transform(&mut obj, &opts));
        items.push(MovePlanItem {
            kind: obj_kind(&obj),
            name: obj_name(&obj),
            manifest: obj,
        });
    }

    Ok(MovePlan {
        app: app.name.clone(),
        target_namespace: req.target_namespace.clone(),
        items,
        images: image_maps,
        notes,
    })
}

/// Apply order: ConfigMaps → Services → everything else (workloads last).
fn apply_rank(kind: &str) -> u8 {
    match kind {
        "ConfigMap" => 0,
        "Service" => 1,
        _ => 2,
    }
}

// ── Execution ───────────────────────────────────────────────────────────────

/// Execute the Move: mirror images (if a registry is set), then shadow-deploy the
/// transformed objects into `target_namespace` on the target cluster.
pub async fn execute_move(
    snapshot: &InventorySnapshot,
    req: &MoveRequest,
    connections: &ConnectionStore,
) -> Result<MoveRun> {
    let plan = plan_move(snapshot, req).await?;

    // Real image copy when a target registry is configured.
    if req.registry.is_some() && !plan.images.is_empty() {
        let srcs: Vec<String> = plan.images.iter().map(|m| m.source.clone()).collect();
        mirror_images(&srcs, req.registry.as_deref().unwrap(), false)
            .await
            .context("mirroring images")?;
    }

    let target = connections
        .get(&req.target_conn)
        .ok_or_else(|| anyhow::anyhow!("target connection '{}' not found", req.target_conn))?;
    let (client, _version) = target.connect().await?;

    // Ensure the target namespace exists.
    let ns_manifest = serde_json::json!({
        "apiVersion": "v1", "kind": "Namespace",
        "metadata": {"name": req.target_namespace}
    });
    apply_object(&client, "", &ns_manifest)
        .await
        .context("creating target namespace")?;

    let mut ordered = plan.items.clone();
    ordered.sort_by_key(|i| apply_rank(&i.kind));

    let mut applied = Vec::new();
    for item in &ordered {
        apply_object(&client, &req.target_namespace, &item.manifest)
            .await
            .with_context(|| format!("applying {} {}", item.kind, item.name))?;
        applied.push(ResourceRef {
            api_version: obj_api_version(&item.manifest),
            kind: item.kind.clone(),
            name: item.name.clone(),
        });
    }

    let run = MoveRun {
        app: req.app.clone(),
        source_conn: req.source_conn.clone(),
        target_conn: req.target_conn.clone(),
        target_namespace: req.target_namespace.clone(),
        phase: MovePhase::Deployed,
        applied,
        notes: plan.notes,
        started_at: crate::resources::now_rfc3339(),
    };
    save_run(&run)?;
    Ok(run)
}

/// Roll back a Move: delete every applied resource from the target.
pub async fn rollback(app: &str, connections: &ConnectionStore) -> Result<usize> {
    let mut store = MoveStore::load(&MoveStore::default_path())?;
    let run = store
        .runs
        .get(app)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("no Move run for '{}'", app))?;
    let target = connections
        .get(&run.target_conn)
        .ok_or_else(|| anyhow::anyhow!("target connection '{}' not found", run.target_conn))?;
    let (client, _version) = target.connect().await?;

    let mut deleted = 0;
    for r in &run.applied {
        if delete_object(&client, &run.target_namespace, r)
            .await
            .is_ok()
        {
            deleted += 1;
        }
    }

    let mut updated = run;
    updated.phase = MovePhase::RolledBack;
    store.runs.insert(app.to_string(), updated);
    store.save(&MoveStore::default_path())?;
    Ok(deleted)
}

/// Mark a Move cut over (traffic is DNS-level across clusters; caller performs the
/// DNS/ingress change). Returns the retained-source note.
pub fn cutover(app: &str) -> Result<()> {
    let mut store = MoveStore::load(&MoveStore::default_path())?;
    let run = store
        .runs
        .get_mut(app)
        .ok_or_else(|| anyhow::anyhow!("no Move run for '{}'", app))?;
    run.phase = MovePhase::CutOver;
    store.save(&MoveStore::default_path())?;
    Ok(())
}

pub fn get_run(app: &str) -> Result<Option<MoveRun>> {
    let store = MoveStore::load(&MoveStore::default_path())?;
    Ok(store.runs.get(app).cloned())
}

fn save_run(run: &MoveRun) -> Result<()> {
    let mut store = MoveStore::load(&MoveStore::default_path())?;
    store.runs.insert(run.app.clone(), run.clone());
    store.save(&MoveStore::default_path())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::kubernetes::{DiscoveredWorkload, PodSignals, RawInventory};

    fn snapshot_with_app() -> InventorySnapshot {
        let manifest = serde_json::json!({
            "apiVersion": "apps/v1", "kind": "Deployment",
            "metadata": {"name": "api", "namespace": "prod", "uid": "x"},
            "spec": {"template": {"spec": {"containers": [
                {"name": "api", "image": "1.dkr.ecr.us-east-1.amazonaws.com/api:1"}]}}},
            "status": {"replicas": 2}
        });
        let mut labels = std::collections::BTreeMap::new();
        labels.insert("app.kubernetes.io/part-of".to_string(), "shop".to_string());
        let w = DiscoveredWorkload {
            namespace: "prod".into(),
            kind: "Deployment".into(),
            name: "api".into(),
            labels,
            annotations: Default::default(),
            owner_refs: vec![],
            replicas: 2,
            containers: vec![crate::discovery::kubernetes::ContainerInfo {
                name: "api".into(),
                image: "1.dkr.ecr.us-east-1.amazonaws.com/api:1".into(),
                has_requests: true,
                cpu_request: Some("100m".into()),
                memory_request: Some("128Mi".into()),
                wants_gpu: false,
                privileged: false,
            }],
            signals: PodSignals::default(),
            config_map_refs: vec![],
            secret_refs: vec![],
            pvc_refs: vec![],
            env_endpoints: vec![],
            manifest: Some(manifest),
        };
        InventorySnapshot::from_raw(RawInventory {
            connection: "src".into(),
            workloads: vec![w],
            ..Default::default()
        })
    }

    #[tokio::test]
    async fn test_plan_move_transforms_from_snapshot() {
        let snap = snapshot_with_app();
        let req = MoveRequest {
            app: "shop".into(),
            source_conn: "src".into(),
            target_conn: "dst".into(),
            target_namespace: "shop-move".into(),
            registry: Some("reg.local".into()),
            strategy: "blue-green".into(),
        };
        let plan = plan_move(&snap, &req).await.unwrap();
        assert_eq!(plan.items.len(), 1);
        let m = &plan.items[0].manifest;
        // namespace retargeted, status/uid stripped, image rewritten to target registry.
        assert_eq!(m["metadata"]["namespace"], "shop-move");
        assert!(m.get("status").is_none());
        assert!(m["metadata"].get("uid").is_none());
        assert_eq!(
            m["spec"]["template"]["spec"]["containers"][0]["image"],
            "reg.local/api:1"
        );
        assert_eq!(plan.images[0].target, "reg.local/api:1");
    }
}
