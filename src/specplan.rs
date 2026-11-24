// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Offline plan for a spec change (`aether spec-plan`).
//!
//! `aether diff` compares a spec with stored and live state and needs a running
//! workload. This module answers the earlier question, in the spirit of
//! `terraform plan`: given the previous spec and the edited spec, which fields
//! changed and what would that do on each runtime (nothing, an in-place patch,
//! a rolling restart, or a replace)? It reads two files and touches no runtime.
//!
//! Impact comes from the portability matrix: a field a runtime ignores has no
//! effect there, and a field's [`Mutability`](crate::portability::Mutability)
//! decides between patch, restart and replace.

use crate::portability::{self, Mutability, Support, Target};
use crate::speclint;
use serde::Serialize;
use serde_yaml::Value;
use std::collections::BTreeMap;

/// Longest value shown in a plan before it is truncated.
const MAX_VALUE_CHARS: usize = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Impact {
    /// The runtime does not read the field.
    None,
    /// Patched on the live object.
    InPlace,
    /// Pod template change: rolling restart (Kubernetes).
    Rolling,
    /// The workload (or object) must be replaced / the VM restarted.
    Recreate,
}

impl Impact {
    pub fn label(self) -> &'static str {
        match self {
            Impact::None => "no effect",
            Impact::InPlace => "update in place",
            Impact::Rolling => "rolling restart",
            Impact::Recreate => "replace",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Added,
    Removed,
    Changed,
}

impl ChangeKind {
    pub fn symbol(self) -> char {
        match self {
            ChangeKind::Added => '+',
            ChangeKind::Removed => '-',
            ChangeKind::Changed => '~',
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetImpact {
    pub target: Target,
    pub impact: Impact,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Change {
    pub path: String,
    pub kind: ChangeKind,
    pub old: Option<String>,
    pub new: Option<String>,
    pub impacts: Vec<TargetImpact>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TargetSummary {
    pub target: Target,
    pub worst: Impact,
    pub affected_fields: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlanReport {
    pub changes: Vec<Change>,
    pub summary: Vec<TargetSummary>,
}

impl PlanReport {
    pub fn has_changes(&self) -> bool {
        !self.changes.is_empty()
    }

    pub fn render_text(&self) -> String {
        if self.changes.is_empty() {
            return "No changes. The two specs are equivalent.\n".to_string();
        }
        let mut out = String::new();
        for c in &self.changes {
            let delta = match (&c.old, &c.new) {
                (Some(o), Some(n)) => format!("{o} -> {n}"),
                (None, Some(n)) => n.clone(),
                (Some(o), None) => o.clone(),
                (None, None) => String::new(),
            };
            out.push_str(&format!("{} {}: {}\n", c.kind.symbol(), c.path, delta));
            for t in &c.impacts {
                out.push_str(&format!(
                    "    {:<9} {} ({})\n",
                    t.target.as_str(),
                    t.impact.label(),
                    t.reason
                ));
            }
        }
        out.push_str(&format!("\nPlan: {} change(s).\n", self.changes.len()));
        for s in &self.summary {
            out.push_str(&format!(
                "  {:<9} {} ({} field(s) affected)\n",
                s.target.as_str(),
                s.worst.label(),
                s.affected_fields
            ));
        }
        out
    }
}

fn truncate(s: &str) -> String {
    let s = &speclint::sanitize(s);
    if s.chars().count() <= MAX_VALUE_CHARS {
        s.to_string()
    } else {
        let head: String = s.chars().take(MAX_VALUE_CHARS).collect();
        format!("{head}...")
    }
}

pub(crate) fn is_sensitive(path: &str) -> bool {
    let p = path.to_lowercase();
    [
        "secret",
        "password",
        "passwd",
        "token",
        "apikey",
        "api_key",
        "credential",
        "privatekey",
    ]
    .iter()
    .any(|k| p.contains(k))
}

/// Flatten a document to `path -> display value` leaves.
/// Leaves keep the raw (sanitised) value for comparison; redaction happens
/// only when a change is displayed, so rotating a secret still shows up.
fn flatten(v: &Value, path: &str, out: &mut BTreeMap<String, String>) {
    match v {
        Value::Mapping(m) if !m.is_empty() => {
            for (k, c) in m {
                let key = match k {
                    Value::String(s) => s.clone(),
                    other => serde_yaml::to_string(other)
                        .unwrap_or_default()
                        .trim()
                        .to_string(),
                };
                let key = speclint::sanitize(&key);
                let p = if path.is_empty() {
                    key
                } else {
                    format!("{path}.{key}")
                };
                flatten(c, &p, out);
            }
        }
        Value::Sequence(s) if !s.is_empty() => {
            for (i, c) in s.iter().enumerate() {
                flatten(c, &format!("{path}[{i}]"), out);
            }
        }
        leaf => {
            let text = match leaf {
                Value::Null => "null".to_string(),
                Value::Mapping(_) => "{}".to_string(),
                Value::Sequence(_) => "[]".to_string(),
                other => serde_yaml::to_string(other)
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            };
            out.insert(path.to_string(), speclint::sanitize(&text));
        }
    }
}

fn impact_for(path: &str) -> Vec<TargetImpact> {
    let rule = portability::rule_for(path);
    Target::ALL
        .iter()
        .map(|&t| {
            let Some(rule) = rule else {
                return TargetImpact {
                    target: t,
                    impact: Impact::None,
                    reason: "field not read by any adapter".to_string(),
                };
            };
            match rule.support(t) {
                Support::Ignored => TargetImpact {
                    target: t,
                    impact: Impact::None,
                    reason: format!("{} ignores it", t.as_str()),
                },
                Support::ControlPlane => TargetImpact {
                    target: t,
                    impact: Impact::None,
                    reason: "control-plane policy, no runtime object changes".to_string(),
                },
                Support::Honoured | Support::Partial => {
                    let (impact, reason) = match (t, rule.mutability) {
                        (_, Mutability::Immutable) => (Impact::Recreate, "immutable once created"),
                        (Target::Container, _) => {
                            (Impact::Recreate, "container flags are fixed at `run`")
                        }
                        (Target::Kube, Mutability::Template) => {
                            (Impact::Rolling, "pod template change")
                        }
                        (Target::Kubevirt, Mutability::Template) => {
                            (Impact::Recreate, "VM must restart")
                        }
                        (_, Mutability::Object) => (Impact::InPlace, "patched on the live object"),
                    };
                    TargetImpact {
                        target: t,
                        impact,
                        reason: reason.to_string(),
                    }
                }
            }
        })
        .collect()
}

/// Compute the plan from two parsed documents.
pub fn plan_values(old: &Value, new: &Value) -> PlanReport {
    let (mut a, mut b) = (BTreeMap::new(), BTreeMap::new());
    flatten(old, "", &mut a);
    flatten(new, "", &mut b);
    let mut changes = Vec::new();
    let paths: std::collections::BTreeSet<&String> = a.keys().chain(b.keys()).collect();
    for path in paths {
        let (o, n) = (a.get(path), b.get(path));
        let kind = match (o, n) {
            (Some(x), Some(y)) if x == y => continue,
            (Some(_), Some(_)) => ChangeKind::Changed,
            (None, Some(_)) => ChangeKind::Added,
            (Some(_), None) => ChangeKind::Removed,
            (None, None) => continue,
        };
        let show = |v: Option<&String>| {
            v.map(|x| {
                if is_sensitive(path) {
                    "<redacted>".to_string()
                } else {
                    truncate(x)
                }
            })
        };
        changes.push(Change {
            path: path.clone(),
            kind,
            old: show(o),
            new: show(n),
            impacts: impact_for(path),
        });
    }
    let summary = Target::ALL
        .iter()
        .map(|&t| {
            let mut worst = Impact::None;
            let mut affected = 0;
            for c in &changes {
                if let Some(ti) = c.impacts.iter().find(|i| i.target == t) {
                    if ti.impact != Impact::None {
                        affected += 1;
                    }
                    worst = worst.max(ti.impact);
                }
            }
            TargetSummary {
                target: t,
                worst,
                affected_fields: affected,
            }
        })
        .collect();
    PlanReport { changes, summary }
}

/// Plan from two YAML documents, with the input guards of [`speclint::guarded_parse`].
pub fn plan_str(old: &str, new: &str) -> Result<PlanReport, String> {
    let o = speclint::guarded_parse(old).map_err(|d| format!("old spec: {}", d.message))?;
    let n = speclint::guarded_parse(new).map_err(|d| format!("new spec: {}", d.message))?;
    for (label, v) in [("old", &o), ("new", &n)] {
        if !matches!(v, Value::Mapping(_)) {
            return Err(format!("{label} spec must be a YAML mapping"));
        }
    }
    Ok(plan_values(&o, &n))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(a: &str, b: &str) -> PlanReport {
        plan_str(a, b).unwrap()
    }

    fn impact(r: &PlanReport, path: &str, t: Target) -> Impact {
        r.changes
            .iter()
            .find(|c| c.path == path)
            .unwrap_or_else(|| panic!("no change for {path}: {:?}", r.changes))
            .impacts
            .iter()
            .find(|i| i.target == t)
            .unwrap()
            .impact
    }

    #[test]
    fn identical_specs_have_no_changes() {
        let r = plan("a: 1\nb: [x, y]\n", "b: [x, y]\na: 1\n");
        assert!(!r.has_changes());
        assert!(r.render_text().starts_with("No changes"));
    }

    #[test]
    fn memory_change_is_runtime_specific() {
        let r = plan(
            "requirements:\n  memory: 1Gi\n",
            "requirements:\n  memory: 2Gi\n",
        );
        assert_eq!(
            impact(&r, "requirements.memory", Target::Kube),
            Impact::Rolling
        );
        assert_eq!(
            impact(&r, "requirements.memory", Target::Container),
            Impact::Recreate
        );
        assert_eq!(
            impact(&r, "requirements.memory", Target::Kubevirt),
            Impact::Recreate
        );
        let kube = r.summary.iter().find(|s| s.target == Target::Kube).unwrap();
        assert_eq!(kube.worst, Impact::Rolling);
    }

    #[test]
    fn kubernetes_only_field_does_not_touch_podman_or_kubevirt() {
        let r = plan("{}\n", "ingress:\n  host: a.example\n");
        assert_eq!(impact(&r, "ingress.host", Target::Container), Impact::None);
        assert_eq!(impact(&r, "ingress.host", Target::Kubevirt), Impact::None);
        assert_eq!(impact(&r, "ingress.host", Target::Kube), Impact::InPlace);
    }

    #[test]
    fn immutable_fields_force_replace_everywhere_they_apply() {
        let r = plan("metadata:\n  name: a\n", "metadata:\n  name: b\n");
        for t in Target::ALL {
            assert_eq!(impact(&r, "metadata.name", t), Impact::Recreate);
        }
        let r = plan(
            "network:\n  headless: false\n",
            "network:\n  headless: true\n",
        );
        assert_eq!(
            impact(&r, "network.headless", Target::Kube),
            Impact::Recreate
        );
        assert_eq!(
            impact(&r, "network.headless", Target::Container),
            Impact::None
        );
    }

    #[test]
    fn added_and_removed_list_items() {
        let r = plan(
            "network:\n  ports:\n    - containerPort: 80\n",
            "network:\n  ports:\n    - containerPort: 80\n    - containerPort: 81\n",
        );
        assert_eq!(r.changes.len(), 1);
        assert_eq!(r.changes[0].kind, ChangeKind::Added);
        assert_eq!(r.changes[0].path, "network.ports[1].containerPort");
        let r = plan("a: 1\nb: 2\n", "a: 1\n");
        assert_eq!(r.changes[0].kind, ChangeKind::Removed);
    }

    #[test]
    fn unknown_and_control_plane_fields_have_no_runtime_effect() {
        let r = plan("intent:\n  x: 1\nzzz: 1\n", "intent:\n  x: 2\nzzz: 2\n");
        for c in &r.changes {
            assert!(c.impacts.iter().all(|i| i.impact == Impact::None), "{c:?}");
        }
    }

    #[test]
    fn secret_values_are_redacted_in_the_plan() {
        let r = plan(
            "config:\n  secrets:\n    - name: s\n      data:\n        password: hunter2\n",
            "config:\n  secrets:\n    - name: s\n      data:\n        password: hunter3\n",
        );
        let text = r.render_text();
        assert!(!text.contains("hunter"), "{text}");
        assert!(text.contains("<redacted>"));
        let json = serde_json::to_string(&r).unwrap();
        assert!(!json.contains("hunter"));
    }

    #[test]
    fn long_values_are_truncated() {
        let long = "x".repeat(500);
        let r = plan("a: 1\n", &format!("a: {long}\n"));
        assert!(r.changes[0].new.as_ref().unwrap().len() < 100);
    }

    #[test]
    fn hostile_inputs_are_rejected_not_expanded() {
        assert!(plan_str("a: [", "a: 1\n")
            .unwrap_err()
            .starts_with("old spec"));
        assert!(plan_str("a: 1\n", "- x\n").unwrap_err().contains("mapping"));
        let mut bomb = String::from("l0: &l0 [a, a]\n");
        for i in 1..40 {
            bomb.push_str(&format!("l{i}: &l{i} [*l{}, *l{}]\n", i - 1, i - 1));
        }
        assert!(plan_str("a: 1\n", &bomb).unwrap_err().contains("alias"));
        let huge = format!("a: {}\n", "y".repeat(speclint::MAX_SPEC_BYTES + 1));
        assert!(plan_str(&huge, "a: 1\n").is_err());
    }

    #[test]
    fn injection_in_values_stays_data() {
        let r = plan(
            "metadata:\n  name: a\n",
            "metadata:\n  name: \"a\\n+ evil: injected\"\n",
        );
        // The newline is part of the YAML scalar: one change, rendered on one line.
        assert_eq!(r.changes.len(), 1);
        assert!(!r.render_text().contains("\n+ evil"), "{}", r.render_text());
    }

    #[test]
    fn json_shape_is_stable() {
        let r = plan("a: 1\n", "a: 2\n");
        let j = serde_json::to_value(&r).unwrap();
        assert_eq!(j["changes"][0]["kind"], "changed");
        assert_eq!(j["summary"].as_array().unwrap().len(), 3);
        assert_eq!(j["summary"][0]["target"], "container");
    }
}
