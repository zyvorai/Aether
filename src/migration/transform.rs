// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Manifest transformation for cross-cluster Move.
//!
//! Pure functions over `serde_json::Value` that prepare a source object (from the
//! discovery snapshot) for a Kubernetes target: strip cluster-specific fields,
//! retarget the namespace, rewrite images (ECR → target registry), convert AWS
//! LoadBalancer Services, strip IRSA IAM annotations, and rewrite cloud endpoints
//! to in-cluster DNS. Each change is recorded as a [`TransformNote`].

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// A single transformation applied to a manifest (surfaced in the plan/report).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransformNote {
    pub field: String,
    pub action: String,
    pub reason: String,
}

impl TransformNote {
    fn new(field: &str, action: &str, reason: &str) -> Self {
        Self {
            field: field.to_string(),
            action: action.to_string(),
            reason: reason.to_string(),
        }
    }
}

/// Options for [`transform`].
#[derive(Debug, Clone, Default)]
pub struct TransformOptions {
    pub target_namespace: Option<String>,
    /// Map of source image ref → target image ref (from image mirroring).
    pub image_map: BTreeMap<String, String>,
    /// Map of AWS endpoint fragment → in-cluster replacement (host or URL).
    pub endpoint_map: BTreeMap<String, String>,
}

/// Apply the full Move transform to an object, returning the notes made.
pub fn transform(obj: &mut Value, opts: &TransformOptions) -> Vec<TransformNote> {
    let mut notes = Vec::new();
    notes.extend(sanitize(obj));
    if let Some(ns) = &opts.target_namespace {
        notes.extend(rewrite_namespace(obj, ns));
    }
    notes.extend(rewrite_loadbalancer(obj));
    notes.extend(rewrite_iam(obj));
    if !opts.image_map.is_empty() {
        notes.extend(rewrite_images(obj, &opts.image_map));
    }
    if !opts.endpoint_map.is_empty() {
        notes.extend(rewrite_endpoints(obj, &opts.endpoint_map));
    }
    notes
}

/// Drop cluster-specific / server-populated fields so the object can be applied
/// to a different cluster.
pub fn sanitize(obj: &mut Value) -> Vec<TransformNote> {
    let mut notes = Vec::new();
    if obj.get("status").is_some() {
        if let Some(m) = obj.as_object_mut() {
            m.remove("status");
            notes.push(TransformNote::new("status", "removed", "server-populated"));
        }
    }
    if let Some(meta) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) {
        for k in [
            "uid",
            "resourceVersion",
            "creationTimestamp",
            "generation",
            "managedFields",
            "ownerReferences",
            "selfLink",
        ] {
            if meta.remove(k).is_some() {
                notes.push(TransformNote::new(&format!("metadata.{k}"), "removed", "cluster-scoped"));
            }
        }
        if let Some(ann) = meta.get_mut("annotations").and_then(|a| a.as_object_mut()) {
            ann.remove("kubectl.kubernetes.io/last-applied-configuration");
        }
    }
    // Service ClusterIPs are assigned by the target cluster.
    if let Some(spec) = obj.get_mut("spec").and_then(|s| s.as_object_mut()) {
        for k in ["clusterIP", "clusterIPs"] {
            if spec.remove(k).is_some() {
                notes.push(TransformNote::new(&format!("spec.{k}"), "removed", "target assigns"));
            }
        }
    }
    if let Some(ps) = pod_spec_mut(obj) {
        if let Some(m) = ps.as_object_mut() {
            if m.remove("nodeName").is_some() {
                notes.push(TransformNote::new("spec…nodeName", "removed", "target schedules"));
            }
        }
    }
    notes
}

/// Point the object at `ns`.
pub fn rewrite_namespace(obj: &mut Value, ns: &str) -> Vec<TransformNote> {
    if let Some(meta) = obj.get_mut("metadata").and_then(|m| m.as_object_mut()) {
        meta.insert("namespace".to_string(), Value::String(ns.to_string()));
        return vec![TransformNote::new("metadata.namespace", "set", ns)];
    }
    Vec::new()
}

/// Rewrite container images found in `image_map` (source → target).
pub fn rewrite_images(obj: &mut Value, image_map: &BTreeMap<String, String>) -> Vec<TransformNote> {
    let mut notes = Vec::new();
    for_each_container(obj, |c| {
        let img = c.get("image").and_then(|i| i.as_str()).map(str::to_string);
        if let Some(img) = img {
            if let Some(target) = image_map.get(&img) {
                let target = target.clone();
                if let Some(m) = c.as_object_mut() {
                    m.insert("image".to_string(), Value::String(target.clone()));
                }
                notes.push(TransformNote::new("image", "rewritten", &target));
            }
        }
    });
    notes
}

/// Convert an AWS `LoadBalancer` Service to `ClusterIP` and strip AWS annotations.
pub fn rewrite_loadbalancer(obj: &mut Value) -> Vec<TransformNote> {
    let mut notes = Vec::new();
    if obj.get("kind").and_then(|k| k.as_str()) != Some("Service") {
        return notes;
    }
    if let Some(spec) = obj.get_mut("spec").and_then(|s| s.as_object_mut()) {
        if spec.get("type").and_then(|t| t.as_str()) == Some("LoadBalancer") {
            spec.insert("type".to_string(), Value::String("ClusterIP".to_string()));
            notes.push(TransformNote::new(
                "spec.type",
                "LoadBalancer→ClusterIP",
                "recreate external access as Ingress/Gateway on target",
            ));
        }
    }
    if let Some(ann) = obj
        .get_mut("metadata")
        .and_then(|m| m.get_mut("annotations"))
        .and_then(|a| a.as_object_mut())
    {
        let aws_keys: Vec<String> = ann
            .keys()
            .filter(|k| k.contains("aws") || k.starts_with("service.beta.kubernetes.io/"))
            .cloned()
            .collect();
        for k in aws_keys {
            ann.remove(&k);
            notes.push(TransformNote::new(&format!("annotations.{k}"), "removed", "AWS-specific"));
        }
    }
    notes
}

/// Strip IRSA (`eks.amazonaws.com/role-arn`) annotations; flag for target IAM mapping.
pub fn rewrite_iam(obj: &mut Value) -> Vec<TransformNote> {
    let mut notes = Vec::new();
    if let Some(ann) = obj
        .get_mut("metadata")
        .and_then(|m| m.get_mut("annotations"))
        .and_then(|a| a.as_object_mut())
    {
        if ann.remove("eks.amazonaws.com/role-arn").is_some() {
            notes.push(TransformNote::new(
                "annotations.eks.amazonaws.com/role-arn",
                "removed",
                "map IAM (IRSA) → target ServiceAccount / Vault",
            ));
        }
    }
    notes
}

/// Replace AWS endpoints in container env values with in-cluster targets.
pub fn rewrite_endpoints(obj: &mut Value, endpoint_map: &BTreeMap<String, String>) -> Vec<TransformNote> {
    let mut notes = Vec::new();
    for_each_container(obj, |c| {
        let Some(env) = c.get_mut("env").and_then(|e| e.as_array_mut()) else {
            return;
        };
        for var in env.iter_mut() {
            let orig = var.get("value").and_then(|v| v.as_str()).map(str::to_string);
            let Some(orig) = orig else { continue };
            let mut new_val = orig.clone();
            for (frag, repl) in endpoint_map {
                if new_val.contains(frag) {
                    new_val = new_val.replace(frag, repl);
                }
            }
            if new_val != orig {
                let name = var.get("name").and_then(|n| n.as_str()).unwrap_or("?").to_string();
                if let Some(m) = var.as_object_mut() {
                    m.insert("value".to_string(), Value::String(new_val));
                }
                notes.push(TransformNote::new(&format!("env.{name}"), "endpoint rewritten", "AWS→in-cluster"));
            }
        }
    });
    notes
}

// ── helpers ─────────────────────────────────────────────────────────────────

/// The pod spec of a controller (`spec.template.spec`) or a bare Pod (`spec`).
fn pod_spec_mut(obj: &mut Value) -> Option<&mut Value> {
    if obj.get("kind").and_then(|k| k.as_str()) == Some("Pod") {
        return obj.get_mut("spec");
    }
    // CronJob: spec.jobTemplate.spec.template.spec
    if obj.get("kind").and_then(|k| k.as_str()) == Some("CronJob") {
        return obj
            .get_mut("spec")?
            .get_mut("jobTemplate")?
            .get_mut("spec")?
            .get_mut("template")?
            .get_mut("spec");
    }
    obj.get_mut("spec")?.get_mut("template")?.get_mut("spec")
}

/// Apply `f` to every container + initContainer in an object's pod spec.
fn for_each_container<F: FnMut(&mut Value)>(obj: &mut Value, mut f: F) {
    if let Some(ps) = pod_spec_mut(obj) {
        if let Some(ps) = ps.as_object_mut() {
            for key in ["containers", "initContainers"] {
                if let Some(arr) = ps.get_mut(key).and_then(|c| c.as_array_mut()) {
                    for c in arr.iter_mut() {
                        f(c);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn deploy() -> Value {
        json!({
            "apiVersion": "apps/v1", "kind": "Deployment",
            "metadata": {
                "name": "api", "namespace": "old", "uid": "123", "resourceVersion": "999",
                "annotations": {"eks.amazonaws.com/role-arn": "arn:aws:iam::1:role/x", "keep": "yes"}
            },
            "spec": {"template": {"spec": {
                "nodeName": "ip-10-0-0-1",
                "containers": [{"name": "api", "image": "1.dkr.ecr.us-east-1.amazonaws.com/api:1",
                    "env": [{"name": "DB", "value": "db.abc.rds.amazonaws.com:5432"}]}]
            }}},
            "status": {"replicas": 3}
        })
    }

    #[test]
    fn test_sanitize_strips_server_fields() {
        let mut d = deploy();
        let notes = sanitize(&mut d);
        assert!(d.get("status").is_none());
        assert!(d["metadata"].get("uid").is_none());
        assert!(d["metadata"].get("resourceVersion").is_none());
        assert!(d["spec"]["template"]["spec"].get("nodeName").is_none());
        assert!(notes.iter().any(|n| n.field == "status"));
    }

    #[test]
    fn test_rewrite_namespace_images_endpoints() {
        let mut d = deploy();
        let mut image_map = BTreeMap::new();
        image_map.insert(
            "1.dkr.ecr.us-east-1.amazonaws.com/api:1".to_string(),
            "registry.zyvor.internal/api:1".to_string(),
        );
        let mut endpoint_map = BTreeMap::new();
        endpoint_map.insert("db.abc.rds.amazonaws.com:5432".to_string(), "postgres.data.svc:5432".to_string());
        let opts = TransformOptions {
            target_namespace: Some("payments".into()),
            image_map,
            endpoint_map,
        };
        let notes = transform(&mut d, &opts);
        assert_eq!(d["metadata"]["namespace"], "payments");
        assert_eq!(d["spec"]["template"]["spec"]["containers"][0]["image"], "registry.zyvor.internal/api:1");
        assert_eq!(
            d["spec"]["template"]["spec"]["containers"][0]["env"][0]["value"],
            "postgres.data.svc:5432"
        );
        // IRSA annotation stripped, "keep" retained.
        assert!(d["metadata"]["annotations"].get("eks.amazonaws.com/role-arn").is_none());
        assert_eq!(d["metadata"]["annotations"]["keep"], "yes");
        assert!(notes.iter().any(|n| n.action == "rewritten"));
    }

    #[test]
    fn test_loadbalancer_to_clusterip() {
        let mut svc = json!({
            "apiVersion": "v1", "kind": "Service",
            "metadata": {"name": "web", "annotations": {"service.beta.kubernetes.io/aws-load-balancer-type": "nlb"}},
            "spec": {"type": "LoadBalancer", "clusterIP": "10.1.2.3", "clusterIPs": ["10.1.2.3"]}
        });
        let notes = sanitize(&mut svc)
            .into_iter()
            .chain(rewrite_loadbalancer(&mut svc))
            .collect::<Vec<_>>();
        assert_eq!(svc["spec"]["type"], "ClusterIP");
        assert!(svc["spec"].get("clusterIP").is_none());
        assert!(svc["metadata"]["annotations"].as_object().unwrap().is_empty());
        assert!(notes.iter().any(|n| n.action.contains("LoadBalancer")));
    }
}
