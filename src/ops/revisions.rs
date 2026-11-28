// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Applied spec revisions: what was actually deployed, when, by whom and why.
//!
//! A revision is recorded only after a runtime accepted the deploy or update,
//! and holds the full spec that was applied (not a path to a file that may
//! change later). That is what makes diffing and rollback truthful.
//!
//! Specs can contain secret values, so the files are written owner-only and
//! every view that leaves this module redacts secret-looking values.

use crate::engine::{DecisionTrace, Engine};
use crate::ops::{self, store};
use crate::runtime::{Instance, RuntimeKind};
use crate::spec::Workload;
use crate::specplan::{self, PlanReport};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Revisions kept per workload.
const MAX_REVISIONS: usize = 100;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum RevisionOp {
    Deploy,
    Update,
}

impl std::fmt::Display for RevisionOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            RevisionOp::Deploy => "deploy",
            RevisionOp::Update => "update",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub id: u64,
    pub at: String,
    pub op: RevisionOp,
    pub runtime: RuntimeKind,
    pub image: String,
    pub instance_id: String,
    pub actor: String,
    #[serde(default)]
    pub reason: Option<String>,
    /// `sha256:<hex>` of the canonical spec.
    pub spec_hash: String,
    /// The applied spec, unredacted.
    pub spec_yaml: String,
    /// What the engine decides for this spec. The runtime actually used is
    /// [`Revision::runtime`]; they differ when the operator chose another.
    pub decision: DecisionTrace,
}

/// A revision without the spec body, safe to show.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RevisionSummary {
    pub id: u64,
    pub at: String,
    pub op: RevisionOp,
    pub runtime: RuntimeKind,
    pub image: String,
    pub actor: String,
    pub reason: Option<String>,
    pub spec_hash: String,
    /// Fields that differ from the previous revision; `None` for the first.
    pub changed_fields: Option<usize>,
}

/// A revision prepared for display: the spec is redacted.
#[derive(Debug, Clone, Serialize)]
pub struct RevisionView {
    #[serde(flatten)]
    pub summary: RevisionSummary,
    pub decision: DecisionTrace,
    pub spec_yaml: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RevisionLog {
    pub next_id: u64,
    pub revisions: Vec<Revision>,
    /// Last deploy or update that failed, if it is newer than the last success.
    #[serde(default)]
    pub failed_at: Option<String>,
}

impl RevisionLog {
    pub fn latest(&self) -> Option<&Revision> {
        self.revisions.last()
    }

    pub fn get(&self, id: u64) -> Option<&Revision> {
        self.revisions.iter().find(|r| r.id == id)
    }

    /// The revision a plain "roll back" should apply: after a failed change
    /// that is the last good revision, otherwise the newest one whose spec
    /// differs from the current spec on the same runtime.
    pub fn rollback_target(&self) -> Result<&Revision> {
        let current = self
            .latest()
            .ok_or_else(|| anyhow::anyhow!("no applied revision is recorded"))?;
        if self.failed_at.is_some() {
            return Ok(current);
        }
        self.revisions
            .iter()
            .rev()
            .skip(1)
            .find(|r| r.runtime == current.runtime && r.spec_hash != current.spec_hash)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "no earlier revision with a different spec on {} is recorded",
                    current.runtime
                )
            })
    }

    pub fn summaries(&self) -> Vec<RevisionSummary> {
        (0..self.revisions.len())
            .map(|i| self.summary_at(i))
            .collect()
    }

    fn summary_at(&self, i: usize) -> RevisionSummary {
        let r = &self.revisions[i];
        let changed_fields = i.checked_sub(1).map(|p| {
            diff_yaml(&self.revisions[p].spec_yaml, &r.spec_yaml)
                .map_or(0, |plan| plan.changes.len())
        });
        RevisionSummary {
            id: r.id,
            at: r.at.clone(),
            op: r.op,
            runtime: r.runtime,
            image: r.image.clone(),
            actor: r.actor.clone(),
            reason: r.reason.clone(),
            spec_hash: r.spec_hash.clone(),
            changed_fields,
        }
    }

    pub fn view(&self, id: u64) -> Result<RevisionView> {
        let i = self
            .revisions
            .iter()
            .position(|r| r.id == id)
            .ok_or_else(|| anyhow::anyhow!("revision {id} not found"))?;
        let r = &self.revisions[i];
        let redacted: serde_yaml::Value =
            serde_yaml::from_str(&redacted_yaml(&r.spec_yaml)?).context("failed to parse spec")?;
        Ok(RevisionView {
            summary: self.summary_at(i),
            decision: r.decision.clone(),
            spec_yaml: crate::specrender::yaml_without_nulls(&redacted)
                .context("failed to serialise spec")?,
        })
    }

    /// Field changes from revision `from` to revision `to`, with the effect on
    /// each runtime. Secret-looking values are redacted.
    pub fn diff(&self, from: u64, to: u64) -> Result<PlanReport> {
        let find = |id| {
            self.get(id)
                .ok_or_else(|| anyhow::anyhow!("revision {id} not found"))
        };
        diff_yaml(&find(from)?.spec_yaml, &find(to)?.spec_yaml)
    }
}

/// The spec as Aether serialises it, so that two documents differ only where
/// the workload differs (defaults filled in, unknown keys dropped).
pub fn canonical_yaml(spec: &Workload) -> Result<String> {
    serde_yaml::to_string(spec).context("failed to serialise spec")
}

fn spec_hash(yaml: &str) -> String {
    use sha2::{Digest, Sha256};
    // The canonical render sorts map keys; fall back to the bytes for a spec
    // that no longer validates under this version.
    crate::specrender::render_str(yaml)
        .map(|r| r.hash)
        .unwrap_or_else(|_| {
            format!(
                "sha256:{}",
                crate::hexfmt::hex_lower(Sha256::digest(yaml.as_bytes()))
            )
        })
}

/// Redact secret-looking values of a spec document.
pub fn redacted_yaml(yaml: &str) -> Result<String> {
    let mut doc: serde_yaml::Value = serde_yaml::from_str(yaml).context("failed to parse spec")?;
    redact(&mut doc, "");
    serde_yaml::to_string(&doc).context("failed to serialise spec")
}

fn redact(v: &mut serde_yaml::Value, path: &str) {
    use serde_yaml::Value;
    match v {
        Value::Mapping(m) => {
            for (k, c) in m.iter_mut() {
                let key = k.as_str().unwrap_or_default();
                let p = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                redact(c, &p);
            }
        }
        Value::Sequence(s) => {
            for (i, c) in s.iter_mut().enumerate() {
                redact(c, &format!("{path}[{i}]"));
            }
        }
        Value::String(s) if !s.is_empty() && specplan::is_sensitive(path) => {
            *s = "***".to_string();
        }
        _ => {}
    }
}

/// Plan between two canonical documents.
pub fn diff_yaml(old: &str, new: &str) -> Result<PlanReport> {
    specplan::plan_str(old, new).map_err(|e| anyhow::anyhow!(e))
}

pub fn path(dir: &Path, workload: &str) -> Result<PathBuf> {
    ops::check_name(workload)?;
    Ok(dir.join("revisions").join(format!("{workload}.json")))
}

/// The revision log of a workload (empty when nothing was recorded).
pub fn load(dir: &Path, workload: &str) -> Result<RevisionLog> {
    store::load(&path(dir, workload)?)
}

/// Record a spec that a runtime accepted. Returns the revision id.
pub fn record(dir: &Path, op: RevisionOp, spec: &Workload, instance: &Instance) -> Result<u64> {
    let spec_yaml = canonical_yaml(spec)?;
    let revision = Revision {
        id: 0,
        at: crate::resources::now_rfc3339(),
        op,
        runtime: instance.runtime,
        image: instance.image.clone(),
        instance_id: instance.id.clone(),
        actor: ops::current_actor(),
        reason: ops::current_reason(),
        spec_hash: spec_hash(&spec_yaml),
        spec_yaml,
        decision: Engine::new().decide_trace(spec),
    };
    store::update(&path(dir, &spec.metadata.name)?, |log: &mut RevisionLog| {
        log.next_id += 1;
        let mut revision = revision;
        revision.id = log.next_id;
        log.revisions.push(revision);
        let excess = log.revisions.len().saturating_sub(MAX_REVISIONS);
        log.revisions.drain(..excess);
        log.failed_at = None;
        log.next_id
    })
}

/// Remember that the latest deploy or update failed.
pub fn record_failure(dir: &Path, workload: &str) -> Result<()> {
    let path = path(dir, workload)?;
    if !path.exists() {
        return Ok(());
    }
    store::update(&path, |log: &mut RevisionLog| {
        log.failed_at = Some(crate::resources::now_rfc3339());
    })
}

pub fn format_history(workload: &str, summaries: &[RevisionSummary]) -> String {
    if summaries.is_empty() {
        return format!(
            "No applied revisions are recorded for '{workload}'. Revisions are recorded from the next deploy or update.\n"
        );
    }
    let mut out = format!(
        "{:<5} {:<20} {:<7} {:<11} {:<8} {:<16} {}\n",
        "REV", "APPLIED (UTC)", "OP", "RUNTIME", "CHANGED", "BY", "REASON"
    );
    for s in summaries.iter().rev() {
        out.push_str(&format!(
            "{:<5} {:<20} {:<7} {:<11} {:<8} {:<16} {}\n",
            s.id,
            s.at.get(..19).unwrap_or(&s.at),
            s.op.to_string(),
            s.runtime.to_string(),
            s.changed_fields
                .map_or_else(|| "-".to_string(), |n| n.to_string()),
            s.actor,
            s.reason.as_deref().unwrap_or("")
        ));
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn spec(name: &str, memory: &str) -> Workload {
        let yaml = format!(
            "apiVersion: aether/v1\nkind: Workload\nmetadata:\n  name: {name}\n  owner: me\n  project: p\nbuild:\n  context: .\n  dockerfile: Dockerfile\n  registry: localhost\nrequirements:\n  cpu: \"500m\"\n  memory: {memory}\n  storage: 1Gi\nruntime:\n  preferred: container\n  allow:\n    - container\nconfig:\n  secrets:\n    - name: s\n      data:\n        DB_PASSWORD: hunter2\n"
        );
        serde_yaml::from_str(&yaml).unwrap()
    }

    pub(crate) fn instance(name: &str, runtime: RuntimeKind) -> Instance {
        Instance::new(
            "id-1".into(),
            name.into(),
            runtime,
            format!("{name}:latest"),
        )
    }

    fn deploy(dir: &Path, memory: &str) -> u64 {
        record(
            dir,
            RevisionOp::Deploy,
            &spec("web", memory),
            &instance("web", RuntimeKind::Podman),
        )
        .unwrap()
    }

    #[test]
    fn revisions_keep_the_applied_spec_and_diff() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        assert_eq!(deploy(d, "256Mi"), 1);
        assert_eq!(deploy(d, "512Mi"), 2);
        let log = load(d, "web").unwrap();
        let s = log.summaries();
        assert_eq!(s[0].changed_fields, None);
        assert_eq!(s[1].changed_fields, Some(1));
        let plan = log.diff(1, 2).unwrap();
        assert_eq!(plan.changes[0].path, "requirements.memory");
        assert!(log.diff(1, 9).is_err());
    }

    #[test]
    fn secrets_are_stored_but_never_shown() {
        let dir = tempfile::tempdir().unwrap();
        deploy(dir.path(), "256Mi");
        let log = load(dir.path(), "web").unwrap();
        assert!(log.revisions[0].spec_yaml.contains("hunter2"));
        let view = serde_json::to_string(&log.view(1).unwrap()).unwrap();
        assert!(!view.contains("hunter2"), "{view}");
        assert!(view.contains("***"));
        let shown = log.view(1).unwrap().spec_yaml;
        assert!(!shown.contains("null"), "{shown}");
        let summaries = serde_json::to_string(&log.summaries()).unwrap();
        assert!(!summaries.contains("hunter2"));
    }

    #[test]
    fn rollback_target_skips_identical_specs() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        deploy(d, "256Mi");
        assert!(load(d, "web").unwrap().rollback_target().is_err());
        deploy(d, "512Mi");
        deploy(d, "512Mi");
        assert_eq!(load(d, "web").unwrap().rollback_target().unwrap().id, 1);
    }

    #[test]
    fn after_a_failed_change_the_last_good_revision_is_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        deploy(d, "256Mi");
        deploy(d, "512Mi");
        record_failure(d, "web").unwrap();
        assert_eq!(load(d, "web").unwrap().rollback_target().unwrap().id, 2);
        deploy(d, "1Gi");
        assert_eq!(load(d, "web").unwrap().rollback_target().unwrap().id, 2);
    }

    #[test]
    fn history_is_bounded_and_ids_keep_increasing() {
        let dir = tempfile::tempdir().unwrap();
        for _ in 0..MAX_REVISIONS + 3 {
            deploy(dir.path(), "256Mi");
        }
        let log = load(dir.path(), "web").unwrap();
        assert_eq!(log.revisions.len(), MAX_REVISIONS);
        assert_eq!(log.latest().unwrap().id, MAX_REVISIONS as u64 + 3);
    }

    #[test]
    fn stored_spec_loads_again() {
        let dir = tempfile::tempdir().unwrap();
        deploy(dir.path(), "256Mi");
        let log = load(dir.path(), "web").unwrap();
        let w: Workload = serde_yaml::from_str(&log.revisions[0].spec_yaml).unwrap();
        w.validate().unwrap();
        assert_eq!(w, spec("web", "256Mi"));
    }
}
