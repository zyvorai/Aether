// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! "Explain this workload": why it runs where it does, what shape it is in and
//! what changed, assembled from recorded facts. No runtime is queried.

use crate::dependencies::DependencyGraph;
use crate::engine::{DecisionTrace, Engine};
use crate::health::HealthHistory;
use crate::ops::{self, attention, changes, holds, incidents, revisions, store};
use crate::portability::{self, Support, Target};
use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use crate::specplan::PlanReport;
use crate::state::WorkloadState;
use anyhow::Result;
use serde::Serialize;
use std::path::Path;

/// The portability family a runtime belongs to.
pub fn target_of(runtime: RuntimeKind) -> Target {
    match runtime {
        RuntimeKind::Podman | RuntimeKind::Docker => Target::Container,
        RuntimeKind::Kubernetes => Target::Kube,
        RuntimeKind::KubeVirt => Target::Kubevirt,
    }
}

/// The spec a workload is running and the file it came from.
pub struct SpecSources {
    /// The latest applied revision.
    pub applied: Option<revisions::Revision>,
    /// The spec file as Aether serialises it, if it loads.
    pub file: Option<Workload>,
    pub file_error: Option<String>,
    /// The file was edited after the last apply and now differs.
    pub pending: Option<PlanReport>,
}

impl SpecSources {
    pub fn load(dir: &Path, ws: &WorkloadState) -> Result<Self> {
        let applied = revisions::load(dir, &ws.name)?.revisions.pop();
        let (file, file_error) = match Workload::from_file(&ws.spec_path) {
            Ok(w) => (Some(w), None),
            Err(e) => (None, Some(format!("{}: {e:#}", ws.spec_path.display()))),
        };
        let pending = match (&applied, &file) {
            (Some(rev), Some(file)) if edited_after(&ws.spec_path, &rev.at) => {
                let plan = revisions::diff_yaml(&rev.spec_yaml, &revisions::canonical_yaml(file)?)?;
                plan.has_changes().then_some(plan)
            }
            _ => None,
        };
        Ok(Self {
            applied,
            file,
            file_error,
            pending,
        })
    }

    /// The spec in effect: the applied revision, else the spec file.
    pub fn current(&self) -> Option<(Workload, String)> {
        if let Some(rev) = &self.applied {
            if let Ok(w) = serde_yaml::from_str::<Workload>(&rev.spec_yaml) {
                return Some((w, format!("applied revision {}", rev.id)));
            }
        }
        self.file
            .clone()
            .map(|w| (w, "spec file (no applied revision recorded)".to_string()))
    }
}

/// A file touched after `applied_at`. Files that were not touched can differ
/// from the applied spec for legitimate reasons (compose env overrides).
fn edited_after(path: &Path, applied_at: &str) -> bool {
    let Some(applied) = ops::parse_time(applied_at) else {
        return false;
    };
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .map(|m| chrono::DateTime::<chrono::Utc>::from(m) > applied)
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize)]
pub struct HealthView {
    pub observed_at: Option<String>,
    pub state: Option<String>,
    pub ready: Option<bool>,
    pub restarts: Option<u32>,
    pub checks_24h: u32,
    /// `None` when there were no checks.
    pub ready_pct_24h: Option<f64>,
    /// Why the status could not be read, if the last attempt failed.
    pub error: Option<String>,
    /// Downtime the operator asked for is currently expected.
    pub expected_down: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldNote {
    pub path: String,
    /// `ignored` or `partial`.
    pub support: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Explanation {
    pub workload: String,
    pub generated_at: String,
    /// One paragraph an operator can read first.
    pub summary: String,
    pub runtime: RuntimeKind,
    pub image: String,
    pub created_at: String,
    pub updated_at: String,
    pub spec_source: Option<String>,
    /// How the engine decides for the current spec.
    pub decision: Option<DecisionTrace>,
    /// Why the workload runs on this runtime.
    pub placement: String,
    pub health: HealthView,
    pub open_incident: Option<u64>,
    pub recovery: incidents::Recovery,
    pub dependencies: Vec<String>,
    pub dependents: Vec<String>,
    pub hold: Option<holds::Hold>,
    /// Spec fields this runtime ignores or only partly honours.
    pub ignored_fields: Vec<FieldNote>,
    pub recent_changes: Vec<changes::Change>,
    pub attention: Vec<attention::Item>,
}

/// Spec fields `runtime` does not fully honour.
pub fn field_notes(spec: &Workload, runtime: RuntimeKind) -> Vec<FieldNote> {
    let Ok(doc) = serde_yaml::to_value(spec) else {
        return Vec::new();
    };
    let target = target_of(runtime);
    portability::report(&doc)
        .rows
        .iter()
        .filter_map(|row| {
            let support = match portability::support_of(row, target) {
                Support::Ignored => "ignored",
                Support::Partial => "partial",
                Support::Honoured | Support::ControlPlane => return None,
            };
            Some(FieldNote {
                path: row.path.clone(),
                support: support.to_string(),
                note: row.note.clone(),
            })
        })
        .collect()
}

fn placement(
    runtime: RuntimeKind,
    applied: Option<&revisions::Revision>,
    decision: Option<&DecisionTrace>,
) -> String {
    let Some(decision) = decision else {
        return format!(
            "Runs on {runtime}. The spec could not be read, so the decision cannot be explained."
        );
    };
    let applied_rule = decision.rules.iter().find(|r| r.applied);
    match decision.selected {
        Some(selected) if selected == runtime => match applied_rule {
            Some(r) => format!(
                "Runs on {runtime}, which is what the engine picks for this spec (rule: {}; {}).",
                r.rule, r.detail
            ),
            None => format!("Runs on {runtime}, which is what the engine picks for this spec."),
        },
        Some(selected) => {
            let how = applied
                .and_then(|r| r.reason.as_deref())
                .map(|r| format!(" The last change was recorded as: {r}."))
                .unwrap_or_default();
            format!(
                "Runs on {runtime}, but the engine would pick {selected} for this spec. The runtime was chosen another way (an explicit runtime, intent scoring, an interactive choice or a migration).{how}"
            )
        }
        None => format!(
            "Runs on {runtime}, although the spec currently allows no runtime the engine can pick."
        ),
    }
}

pub fn explain(dir: &Path, ws: &WorkloadState) -> Result<Explanation> {
    let now = chrono::Utc::now();
    let sources = SpecSources::load(dir, ws)?;
    let current = sources.current();
    let decision = current
        .as_ref()
        .map(|(spec, _)| Engine::new().decide_trace(spec));
    let placement = placement(ws.runtime, sources.applied.as_ref(), decision.as_ref());

    let history: HealthHistory = store::load(&incidents::health_path(dir))?;
    let last = history.records.iter().rev().find(|r| r.workload == ws.name);
    let counter = store::load::<incidents::Availability>(&incidents::availability_path(dir))?
        .since(&ws.name, now - chrono::Duration::hours(24));
    let all = incidents::load(dir)?;
    let mine: Vec<_> = all
        .incidents
        .iter()
        .filter(|i| i.workload == ws.name)
        .collect();
    let health = HealthView {
        observed_at: last.map(|r| r.timestamp.clone()),
        state: last.map(|r| r.state.to_string()),
        ready: last.map(|r| r.ready),
        restarts: last.map(|r| r.restart_count),
        checks_24h: counter.checks,
        ready_pct_24h: (counter.checks > 0)
            .then(|| f64::from(counter.ready) * 100.0 / f64::from(counter.checks)),
        error: all.observe_errors.get(&ws.name).map(|e| e.message.clone()),
        expected_down: all
            .expected_down
            .get(&ws.name)
            .filter(|d| d.runtime == ws.runtime)
            .map(|d| format!("{} since {}", d.why, d.since)),
    };
    let open_incident = mine.iter().find(|i| i.is_open()).map(|i| i.id);

    let graph = DependencyGraph::load(&dir.join("dependencies.json"))?;
    let mut dependencies = graph.dependencies_of(&ws.name);
    let mut dependents = graph.dependents_of(&ws.name);
    dependencies.sort();
    dependents.sort();

    let recent = changes::collect(
        dir,
        &changes::Query::since(now - chrono::Duration::days(30), Some(ws.name.clone()), 10),
    )?;
    let items = attention::for_workload(dir, ws, now)?;
    let hold = holds::get(dir, &ws.name)?;
    let ignored_fields = current
        .as_ref()
        .map(|(spec, _)| field_notes(spec, ws.runtime))
        .unwrap_or_default();

    let state_words = match (&health.expected_down, open_incident, health.ready) {
        (Some(why), _, _) => format!("It is expected to be down ({why})."),
        (None, Some(id), _) => format!("Incident {id} is open."),
        (None, None, Some(true)) => "The last health check found it ready.".to_string(),
        (None, None, Some(false)) => "The last health check found it not ready.".to_string(),
        (None, None, None) => "No health check has been recorded for it.".to_string(),
    };
    let mut summary = format!(
        "{} {} {state_words}",
        ws.name,
        placement.replacen("Runs", "runs", 1)
    );
    if sources.pending.is_some() {
        summary.push_str(" Its spec file has changes that are not applied.");
    }
    if let Some(h) = &hold {
        summary.push_str(&format!(" It is on hold: {}.", h.reason));
    }

    Ok(Explanation {
        workload: ws.name.clone(),
        generated_at: now.to_rfc3339(),
        summary,
        runtime: ws.runtime,
        image: ws.instance.image.clone(),
        created_at: ws.created_at.clone(),
        updated_at: ws.updated_at.clone(),
        spec_source: current.map(|(_, s)| s),
        decision,
        placement,
        health,
        open_incident,
        recovery: incidents::recovery(mine),
        dependencies,
        dependents,
        hold,
        ignored_fields,
        recent_changes: recent.changes,
        attention: items,
    })
}

pub fn format(e: &Explanation) -> String {
    let mut out = format!("{}\n\n", e.summary);
    out.push_str(&format!(
        "Runtime   {}\nImage     {}\nCreated   {}\nUpdated   {}\nSpec      {}\n",
        e.runtime,
        e.image,
        e.created_at,
        e.updated_at,
        e.spec_source.as_deref().unwrap_or("unavailable")
    ));
    if let Some(d) = &e.decision {
        out.push_str("\nDecision rules, in order\n");
        for r in &d.rules {
            let mark = if r.applied {
                "=>"
            } else if r.matched {
                " ~"
            } else {
                " -"
            };
            out.push_str(&format!("  {mark} {:<18} {}\n", r.rule, r.detail));
        }
    }
    out.push_str("\nHealth\n");
    match (&e.health.observed_at, &e.health.state, e.health.ready) {
        (Some(at), Some(state), Some(ready)) => out.push_str(&format!(
            "  last check {at}: {state}, ready={ready}, restarts={}\n",
            e.health.restarts.unwrap_or(0)
        )),
        _ => out.push_str("  no checks recorded\n"),
    }
    match e.health.ready_pct_24h {
        Some(p) => out.push_str(&format!(
            "  last 24h: {p:.2}% ready over {} checks\n",
            e.health.checks_24h
        )),
        None => out.push_str("  last 24h: no data\n"),
    }
    if let Some(err) = &e.health.error {
        out.push_str(&format!("  status could not be read: {err}\n"));
    }
    match e.recovery.mean_minutes {
        Some(m) => out.push_str(&format!(
            "  incidents: {} ({} recovered, mean {m:.1} min)\n",
            e.recovery.incidents, e.recovery.recovered
        )),
        None => out.push_str(&format!("  incidents: {}\n", e.recovery.incidents)),
    }
    if !e.dependencies.is_empty() {
        out.push_str(&format!("\nDepends on    {}\n", e.dependencies.join(", ")));
    }
    if !e.dependents.is_empty() {
        out.push_str(&format!("\nNeeded by     {}\n", e.dependents.join(", ")));
    }
    if !e.ignored_fields.is_empty() {
        out.push_str(&format!(
            "\nSpec fields {} does not fully honour\n",
            e.runtime
        ));
        for f in &e.ignored_fields {
            out.push_str(&format!("  {:<8} {:<28} {}\n", f.support, f.path, f.note));
        }
    }
    if !e.attention.is_empty() {
        out.push_str("\nNeeds attention\n");
        out.push_str(&attention::format_items(&e.attention));
    }
    if !e.recent_changes.is_empty() {
        out.push_str("\nRecent changes\n");
        for c in &e.recent_changes {
            out.push_str(&format!(
                "  {}  {} - {}\n",
                c.at.get(..19).unwrap_or(&c.at),
                c.title,
                crate::speclint::sanitize(&c.detail)
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::revisions::tests::{instance, spec};

    fn state(dir: &Path, name: &str) -> WorkloadState {
        let spec_path = dir.join(format!("{name}.yaml"));
        std::fs::write(
            &spec_path,
            revisions::canonical_yaml(&spec(name, "256Mi")).unwrap(),
        )
        .unwrap();
        WorkloadState::new(
            name.into(),
            RuntimeKind::Podman,
            instance(name, RuntimeKind::Podman),
            spec_path,
        )
    }

    #[test]
    fn explains_placement_health_and_pending_edits() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let ws = state(d, "web");
        let e = explain(d, &ws).unwrap();
        assert!(e.placement.contains("engine picks"), "{}", e.placement);
        assert!(e.summary.contains("No health check"), "{}", e.summary);
        assert_eq!(e.health.ready_pct_24h, None);

        revisions::record(
            d,
            revisions::RevisionOp::Deploy,
            &spec("web", "256Mi"),
            &instance("web", RuntimeKind::Podman),
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(50));
        std::fs::write(
            &ws.spec_path,
            revisions::canonical_yaml(&spec("web", "1Gi")).unwrap(),
        )
        .unwrap();
        let e = explain(d, &ws).unwrap();
        assert_eq!(e.spec_source.as_deref(), Some("applied revision 1"));
        assert!(e.summary.contains("not applied"), "{}", e.summary);
        let text = format(&e);
        assert!(!text.contains("hunter2"));
    }

    #[test]
    fn says_when_the_runtime_is_not_the_engines_choice() {
        let dir = tempfile::tempdir().unwrap();
        let mut ws = state(dir.path(), "web");
        ws.runtime = RuntimeKind::Kubernetes;
        let e = explain(dir.path(), &ws).unwrap();
        assert!(e.placement.contains("would pick podman"), "{}", e.placement);
    }

    #[test]
    fn unreadable_spec_is_stated() {
        let dir = tempfile::tempdir().unwrap();
        let mut ws = state(dir.path(), "web");
        ws.spec_path = dir.path().join("gone.yaml");
        let e = explain(dir.path(), &ws).unwrap();
        assert!(e.decision.is_none());
        assert!(e.placement.contains("cannot be explained"));
    }
}
