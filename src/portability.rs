// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Portability matrix: which spec fields each runtime adapter honours.
//!
//! One Aether spec targets three runtimes, but the adapters do not honour the
//! same fields (for example Podman ignores `ingress`, `scaling` and
//! `kubernetes.*`; KubeVirt ignores probes). The matrix below is derived from
//! what `src/adapters/{podman,kube*,kubevirt}.rs` actually read. It powers
//! `aether portability`, the `W040` lint rule and `aether spec-plan`.
//!
//! Paths are YAML key paths (camelCase, as written in a spec). A rule matches a
//! spec path when it equals the rule path or is nested below it.

use serde::Serialize;
use serde_yaml::Value;

/// Runtime family a spec can be rendered for. Mirrors `runtime.allow` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Target {
    /// Podman / Docker.
    Container,
    /// Kubernetes.
    Kube,
    /// KubeVirt virtual machines.
    Kubevirt,
}

impl Target {
    pub const ALL: [Target; 3] = [Target::Container, Target::Kube, Target::Kubevirt];

    pub fn as_str(self) -> &'static str {
        match self {
            Target::Container => "container",
            Target::Kube => "kube",
            Target::Kubevirt => "kubevirt",
        }
    }
}

/// How well a runtime honours a field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Support {
    /// Rendered faithfully.
    Honoured,
    /// Rendered with reduced fidelity (see the note).
    Partial,
    /// Silently dropped by the adapter.
    Ignored,
    /// Evaluated by the Aether control plane, not by a runtime.
    ControlPlane,
}

/// How a change to a field is applied to an already-running workload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Mutability {
    /// Part of the pod/VM template: changing it restarts the workload.
    Template,
    /// A separate API object that can be patched in place (Service, HPA, ...).
    Object,
    /// Immutable once created: the object must be replaced.
    Immutable,
}

#[derive(Debug, Clone, Copy)]
pub struct Rule {
    pub path: &'static str,
    pub container: Support,
    pub kube: Support,
    pub kubevirt: Support,
    pub mutability: Mutability,
    pub note: &'static str,
}

impl Rule {
    pub fn support(&self, t: Target) -> Support {
        match t {
            Target::Container => self.container,
            Target::Kube => self.kube,
            Target::Kubevirt => self.kubevirt,
        }
    }
}

use Mutability::{Immutable, Object, Template};
use Support::{ControlPlane as Cp, Honoured as H, Ignored as I, Partial as P};

const fn r(
    path: &'static str,
    container: Support,
    kube: Support,
    kubevirt: Support,
    mutability: Mutability,
    note: &'static str,
) -> Rule {
    Rule {
        path,
        container,
        kube,
        kubevirt,
        mutability,
        note,
    }
}

/// The matrix. Longest matching path wins.
pub const RULES: &[Rule] = &[
    r(
        "metadata.name",
        H,
        H,
        H,
        Immutable,
        "Object/container/VM name; changing it replaces the workload.",
    ),
    r(
        "metadata.labels",
        I,
        H,
        H,
        Object,
        "Podman only sets its own app/aether-managed labels.",
    ),
    r(
        "metadata.annotations",
        I,
        H,
        H,
        Object,
        "Not passed to `podman run`.",
    ),
    r(
        "build",
        H,
        H,
        P,
        Template,
        "KubeVirt only uses the resulting image reference.",
    ),
    r(
        "requirements.cpu",
        P,
        H,
        H,
        Template,
        "Podman applies --cpus only when rootful; KubeVirt maps it to vCPU cores.",
    ),
    r(
        "requirements.memory",
        P,
        H,
        H,
        Template,
        "Podman applies --memory only when rootful.",
    ),
    r(
        "requirements.storage",
        I,
        I,
        H,
        Immutable,
        "Only KubeVirt sizes a root disk from it; use persistence.size on Kubernetes.",
    ),
    r(
        "requirements.gpu",
        I,
        P,
        H,
        Template,
        "Kubernetes needs a device plugin; Podman has no GPU path.",
    ),
    r(
        "requirements.cpuRequest",
        I,
        H,
        I,
        Template,
        "Requests/limits split exists only on Kubernetes.",
    ),
    r(
        "requirements.memoryRequest",
        I,
        H,
        I,
        Template,
        "Requests/limits split exists only on Kubernetes.",
    ),
    r(
        "network.service",
        I,
        H,
        H,
        Object,
        "KubeVirt uses it to enable masquerade pod networking.",
    ),
    r(
        "network.ports",
        H,
        H,
        I,
        Template,
        "KubeVirt VMs get no port mapping from the spec.",
    ),
    r(
        "network.serviceType",
        I,
        H,
        I,
        Object,
        "Kubernetes Service type.",
    ),
    r(
        "network.headless",
        I,
        H,
        I,
        Immutable,
        "clusterIP is immutable on a Service.",
    ),
    r("network.externalName", I, H, I, Object, ""),
    r("network.sessionAffinity", I, H, I, Object, ""),
    r("network.externalTrafficPolicy", I, H, I, Object, ""),
    r(
        "network.networkPolicy",
        I,
        H,
        I,
        Object,
        "Needs a NetworkPolicy-enforcing CNI.",
    ),
    r(
        "network.ciliumNetworkPolicy",
        I,
        H,
        I,
        Object,
        "Needs the Cilium CNI.",
    ),
    r(
        "network.calicoNetworkPolicy",
        I,
        H,
        I,
        Object,
        "Needs the Calico CNI.",
    ),
    r(
        "persistence",
        I,
        H,
        P,
        Immutable,
        "KubeVirt honours only storageClass and accessMode.",
    ),
    r(
        "health.liveness",
        P,
        H,
        I,
        Template,
        "Podman builds a curl/bash health-cmd that must exist in the image.",
    ),
    r("health.readiness", I, H, I, Template, ""),
    r("health.startup", I, H, I, Template, ""),
    r(
        "config.configMaps",
        P,
        H,
        I,
        Template,
        "Podman injects data as env vars only; mountPath is ignored.",
    ),
    r(
        "config.secrets",
        I,
        H,
        I,
        Template,
        "Podman does not inject secrets.",
    ),
    r("config.envFrom", I, H, I, Template, ""),
    r("ingress", I, H, I, Object, ""),
    r(
        "scaling",
        I,
        H,
        I,
        Object,
        "HorizontalPodAutoscaler / KEDA objects.",
    ),
    r("mesh", I, H, I, Template, "Sidecar injection annotations."),
    r(
        "schedule",
        I,
        H,
        I,
        Immutable,
        "Switches the controller to a CronJob.",
    ),
    r(
        "kubernetes.workloadKind",
        I,
        H,
        I,
        Immutable,
        "Controller kind cannot change in place.",
    ),
    r("kubernetes", I, H, I, Template, "Kubernetes-only options."),
    r("kubevirt", I, I, H, Template, "KubeVirt-only options."),
    r(
        "confidential",
        I,
        P,
        I,
        Template,
        "Kubernetes only labels the pod; attestation lives in Ragnarok.",
    ),
    r(
        "intent",
        Cp,
        Cp,
        Cp,
        Object,
        "Placement/scoring policy evaluated by Aether.",
    ),
    r(
        "autonomy",
        Cp,
        Cp,
        Cp,
        Object,
        "Healing/migration policy evaluated by Aether.",
    ),
    r(
        "runtime",
        Cp,
        Cp,
        Cp,
        Object,
        "Runtime selection is decided by Aether.",
    ),
    r("apiVersion", Cp, Cp, Cp, Object, ""),
    r("kind", Cp, Cp, Cp, Object, ""),
    r("metadata.owner", Cp, Cp, Cp, Object, "Bookkeeping only."),
    r("metadata.project", Cp, Cp, Cp, Object, "Bookkeeping only."),
];

/// Find the rule governing `path` (indices like `[0]` are ignored).
pub fn rule_for(path: &str) -> Option<&'static Rule> {
    let norm = normalise_path(path);
    RULES
        .iter()
        .filter(|rule| {
            norm == rule.path
                || (norm.starts_with(rule.path)
                    && norm.as_bytes().get(rule.path.len()) == Some(&b'.'))
        })
        .max_by_key(|rule| rule.path.len())
}

/// Drop `[n]` index segments: `network.ports[0].containerPort` becomes
/// `network.ports.containerPort`.
pub fn normalise_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    let mut skipping = false;
    for c in path.chars() {
        match c {
            '[' => skipping = true,
            ']' => skipping = false,
            _ if !skipping => out.push(c),
            _ => {}
        }
    }
    out
}

/// Whether a spec node counts as "set" (a `false` flag or an empty container
/// is the same as leaving the field out).
pub fn is_set(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Sequence(s) => !s.is_empty(),
        Value::Mapping(m) => {
            if m.get(Value::String("enabled".into())) == Some(&Value::Bool(false)) {
                return false;
            }
            !m.is_empty()
        }
        _ => true,
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Row {
    pub path: String,
    pub container: Support,
    pub kube: Support,
    pub kubevirt: Support,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetSummary {
    pub target: Target,
    pub honoured: usize,
    pub partial: usize,
    pub ignored: usize,
    /// Share of runtime-relevant fields honoured, partial counting half; 100
    /// when the spec sets nothing the runtime could drop.
    pub score: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct PortabilityReport {
    pub rows: Vec<Row>,
    pub summary: Vec<TargetSummary>,
}

/// Fields the user actually set, as top-level-ish paths worth reporting. We
/// check every rule path (not every leaf) so the report stays readable.
pub fn report(spec: &Value) -> PortabilityReport {
    let mut rows = Vec::new();
    for rule in RULES {
        if let Some(node) = lookup(spec, rule.path) {
            if is_set(node) && !all_control_plane(rule) {
                rows.push(Row {
                    path: rule.path.to_string(),
                    container: rule.container,
                    kube: rule.kube,
                    kubevirt: rule.kubevirt,
                    note: rule.note.to_string(),
                });
            }
        }
    }
    // A parent rule (e.g. `kubernetes`) would double-report when a longer rule
    // (`kubernetes.workloadKind`) also matched; keep the parent only if it has
    // children outside the longer rules. For readability, keep both; the
    // parent note explains the family.
    let summary = Target::ALL
        .iter()
        .map(|&t| {
            let (mut h, mut p, mut i) = (0usize, 0usize, 0usize);
            for row in &rows {
                match support_of(row, t) {
                    Support::Honoured => h += 1,
                    Support::Partial => p += 1,
                    Support::Ignored => i += 1,
                    Support::ControlPlane => {}
                }
            }
            let total = h + p + i;
            let score = if total == 0 {
                100
            } else {
                (((h * 2 + p) * 100) / (total * 2)) as u32
            };
            TargetSummary {
                target: t,
                honoured: h,
                partial: p,
                ignored: i,
                score,
            }
        })
        .collect();
    PortabilityReport { rows, summary }
}

pub fn support_of(row: &Row, t: Target) -> Support {
    match t {
        Target::Container => row.container,
        Target::Kube => row.kube,
        Target::Kubevirt => row.kubevirt,
    }
}

fn all_control_plane(rule: &Rule) -> bool {
    rule.container == Support::ControlPlane
        && rule.kube == Support::ControlPlane
        && rule.kubevirt == Support::ControlPlane
}

/// Resolve a dotted key path inside a YAML mapping.
pub fn lookup<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let mut cur = root;
    for seg in path.split('.') {
        cur = cur.as_mapping()?.get(Value::String(seg.to_string()))?;
    }
    Some(cur)
}

/// Runtimes the spec can land on, from `runtime.allow` / `runtime.preferred`.
/// An empty or missing list means "any".
pub fn declared_targets(spec: &Value) -> Vec<Target> {
    let mut out = Vec::new();
    if let Some(Value::Sequence(allow)) = lookup(spec, "runtime.allow") {
        for v in allow {
            match v.as_str() {
                Some("container") => out.push(Target::Container),
                Some("kube") => out.push(Target::Kube),
                Some("kubevirt") => out.push(Target::Kubevirt),
                _ => {}
            }
        }
    }
    if out.is_empty() {
        out.extend(Target::ALL);
    }
    let mut uniq: Vec<Target> = Vec::new();
    for t in out {
        if !uniq.contains(&t) {
            uniq.push(t);
        }
    }
    uniq
}

#[cfg(test)]
mod tests {
    use super::*;

    fn y(s: &str) -> Value {
        serde_yaml::from_str(s).unwrap()
    }

    #[test]
    fn longest_rule_wins() {
        assert_eq!(
            rule_for("kubernetes.workloadKind").unwrap().path,
            "kubernetes.workloadKind"
        );
        assert_eq!(
            rule_for("kubernetes.nodeSelector.disk").unwrap().path,
            "kubernetes"
        );
        assert_eq!(
            rule_for("network.ports[0].containerPort").unwrap().path,
            "network.ports"
        );
        assert!(rule_for("totally.unknown").is_none());
    }

    #[test]
    fn prefix_must_end_on_segment_boundary() {
        // "network.service" must not capture "network.serviceType".
        assert_eq!(
            rule_for("network.serviceType").unwrap().path,
            "network.serviceType"
        );
        assert!(rule_for("kubernetesFoo").is_none());
    }

    #[test]
    fn disabled_sections_are_not_reported() {
        let spec = y(
            "ingress:\n  enabled: false\n  host: x\nscaling:\n  enabled: true\n  minReplicas: 1\n",
        );
        let rep = report(&spec);
        let paths: Vec<_> = rep.rows.iter().map(|r| r.path.as_str()).collect();
        assert!(!paths.contains(&"ingress"));
        assert!(paths.contains(&"scaling"));
    }

    #[test]
    fn podman_ignores_kubernetes_only_fields() {
        let spec = y("kubernetes:\n  serviceAccountName: x\nscaling:\n  enabled: true\nrequirements:\n  cpu: '1'\n");
        let rep = report(&spec);
        let c = rep
            .summary
            .iter()
            .find(|s| s.target == Target::Container)
            .unwrap();
        assert_eq!(c.ignored, 2);
        assert_eq!(c.partial, 1);
        let k = rep
            .summary
            .iter()
            .find(|s| s.target == Target::Kube)
            .unwrap();
        assert_eq!(k.score, 100);
    }

    #[test]
    fn empty_spec_scores_full() {
        let rep = report(&y("{}"));
        assert!(rep.rows.is_empty());
        assert!(rep.summary.iter().all(|s| s.score == 100));
    }

    #[test]
    fn declared_targets_defaults_to_all() {
        assert_eq!(declared_targets(&y("{}")).len(), 3);
        assert_eq!(
            declared_targets(&y("runtime:\n  allow: [kube]\n")),
            vec![Target::Kube]
        );
        // Unknown names are skipped rather than trusted.
        assert_eq!(
            declared_targets(&y("runtime:\n  allow: [bogus]\n")).len(),
            3
        );
    }

    #[test]
    fn normalise_strips_indices() {
        assert_eq!(normalise_path("a[0].b[12][3]"), "a.b");
    }
}
