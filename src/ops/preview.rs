// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Preview an action before taking it: what blocks it, what to be careful
//! about and what it will do.
//!
//! A preview reads Aether's records and the spec only. It never contacts or
//! changes a runtime, so it cannot see changes made outside Aether.

use crate::config::PolicyConfig;
use crate::dependencies::DependencyGraph;
use crate::engine::Engine;
use crate::ops::explain::{field_notes, target_of, SpecSources};
use crate::ops::{holds, incidents, revisions};
use crate::policy::{self, QuotaStore};
use crate::runtime::RuntimeKind;
use crate::spec::{RuntimeType, Workload};
use crate::specplan::{Impact, PlanReport};
use crate::state::WorkloadState;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "lowercase")]
pub enum Action {
    /// Apply an edited spec.
    Update {
        yaml: String,
    },
    Delete,
    Stop,
    Restart,
    Migrate {
        target: String,
    },
    Rollback {
        #[serde(default)]
        revision: Option<u64>,
    },
}

impl Action {
    fn label(&self) -> &'static str {
        match self {
            Action::Update { .. } => "update",
            Action::Delete => "delete",
            Action::Stop => "stop",
            Action::Restart => "restart",
            Action::Migrate { .. } => "migrate",
            Action::Rollback { .. } => "rollback",
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Verdict {
    /// Aether would refuse it, or it cannot succeed.
    Blocked,
    /// It can run; read the warnings first.
    Caution,
    /// No blocker or warning was found in the recorded data.
    Clear,
}

#[derive(Debug, Clone, Serialize)]
pub struct Preview {
    pub workload: String,
    pub action: String,
    pub runtime: RuntimeKind,
    pub verdict: Verdict,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    /// What the action does.
    pub effects: Vec<String>,
    /// Spec changes with their effect per runtime.
    pub plan: Option<PlanReport>,
    /// Revision a rollback would apply.
    pub target_revision: Option<u64>,
    /// Workloads that depend on this one, directly or indirectly.
    pub affected: Vec<String>,
}

#[derive(Default)]
struct Findings {
    blockers: Vec<String>,
    warnings: Vec<String>,
    effects: Vec<String>,
    plan: Option<PlanReport>,
    target_revision: Option<u64>,
}

/// Preview `action` on a managed workload.
pub fn preview(
    dir: &Path,
    ws: &WorkloadState,
    action: &Action,
    policy_config: &PolicyConfig,
) -> Result<Preview> {
    let mut f = Findings::default();
    if let Some(h) = holds::get(dir, &ws.name)? {
        f.blockers.push(format!(
            "on hold until {}: {} (placed by {})",
            h.expires_at.as_deref().unwrap_or("released"),
            h.reason,
            h.actor
        ));
    }
    let open = incidents::load(dir)?
        .incidents
        .iter()
        .find(|i| i.workload == ws.name && i.is_open())
        .map(|i| i.id);
    if let Some(id) = open {
        f.warnings.push(format!(
            "incident {id} is open on this workload; check it before changing anything"
        ));
    }
    let graph = DependencyGraph::load(&dir.join("dependencies.json"))?;
    let mut affected: Vec<String> = graph.impact_analysis(&ws.name).affected_workloads;
    affected.sort();
    let sources = SpecSources::load(dir, ws)?;
    let log = revisions::load(dir, &ws.name)?;

    match action {
        Action::Update { yaml } => update(dir, ws, yaml, &sources, policy_config, &mut f)?,
        Action::Delete => delete(ws, &sources, &mut f),
        Action::Stop => {
            f.effects.push(format!(
                "stops the instance on {}; it stays in Aether's state and can be started again",
                ws.runtime
            ));
            f.effects
                .push("counts as planned downtime: no incident is opened for it".to_string());
        }
        Action::Restart => {
            if matches!(ws.runtime, RuntimeKind::Podman | RuntimeKind::Docker) {
                f.effects.push(
                    "stops the container, rebuilds the image from the spec file and runs it again"
                        .to_string(),
                );
                f.warnings
                    .push("the workload is down between the stop and the new start".to_string());
            } else {
                f.effects
                    .push(format!("restarts the workload on {}", ws.runtime));
                f.warnings.push(
                    "brief unavailability is possible, certainly with a single replica or a VM"
                        .to_string(),
                );
            }
            pending_warning(
                &sources,
                "a restart from the CLI or API reads the spec file",
                &mut f,
            );
        }
        Action::Migrate { target } => migrate(ws, target, &sources, &mut f),
        Action::Rollback { revision } => rollback(ws, *revision, &log, &mut f)?,
    }

    let disruptive = !matches!(action, Action::Update { .. } if f.plan.as_ref().is_some_and(|p| !p.has_changes()));
    if disruptive && !affected.is_empty() {
        f.warnings.push(format!(
            "{} workload(s) depend on this one: {}",
            affected.len(),
            affected.join(", ")
        ));
    }

    let verdict = if !f.blockers.is_empty() {
        Verdict::Blocked
    } else if !f.warnings.is_empty() {
        Verdict::Caution
    } else {
        Verdict::Clear
    };
    Ok(Preview {
        workload: ws.name.clone(),
        action: action.label().to_string(),
        runtime: ws.runtime,
        verdict,
        blockers: f.blockers,
        warnings: f.warnings,
        effects: f.effects,
        plan: f.plan,
        target_revision: f.target_revision,
        affected,
    })
}

fn pending_warning(sources: &SpecSources, why: &str, f: &mut Findings) {
    if let Some(plan) = &sources.pending {
        f.warnings.push(format!(
            "the spec file has {} unapplied change(s), and {why}",
            plan.changes.len()
        ));
    }
}

/// Describe what a spec plan does on one runtime.
fn plan_effects(ws: &WorkloadState, plan: &PlanReport, f: &mut Findings) {
    if !plan.has_changes() {
        f.effects
            .push("no spec field changes compared with the current spec".to_string());
        return;
    }
    let target = target_of(ws.runtime);
    if let Some(summary) = plan.summary.iter().find(|s| s.target == target) {
        f.effects.push(format!(
            "{} field(s) change; on {} the strongest effect is: {} ({} field(s) take effect)",
            plan.changes.len(),
            ws.runtime,
            summary.worst.label(),
            summary.affected_fields
        ));
        if matches!(summary.worst, Impact::Recreate) {
            f.warnings.push(format!(
                "at least one change makes {} replace the workload: expect downtime",
                ws.runtime
            ));
        }
    }
    let dead: Vec<&str> = plan
        .changes
        .iter()
        .filter(|c| {
            c.impacts
                .iter()
                .any(|i| i.target == target && i.impact == Impact::None)
        })
        .map(|c| c.path.as_str())
        .collect();
    if !dead.is_empty() {
        f.warnings
            .push(format!("no effect on {}: {}", ws.runtime, dead.join(", ")));
    }
}

fn update(
    dir: &Path,
    ws: &WorkloadState,
    yaml: &str,
    sources: &SpecSources,
    policy_config: &PolicyConfig,
    f: &mut Findings,
) -> Result<()> {
    let lint = crate::speclint::lint_str(yaml);
    for d in &lint.diagnostics {
        let line = format!("lint {} line {}: {} {}", d.code, d.line, d.path, d.message);
        match d.severity {
            crate::speclint::Severity::Error => f.blockers.push(line),
            crate::speclint::Severity::Warning => f.warnings.push(line),
            _ => {}
        }
    }
    let spec: Workload = match serde_yaml::from_str(yaml) {
        Ok(s) => s,
        Err(e) => {
            if lint.errors == 0 {
                f.blockers.push(format!("the spec does not parse: {e}"));
            }
            return Ok(());
        }
    };
    if let Err(e) = spec.validate() {
        if lint.errors == 0 {
            f.blockers.push(format!("the spec is invalid: {e:#}"));
        }
        return Ok(());
    }
    if spec.metadata.name != ws.name {
        f.blockers.push(format!(
            "metadata.name is '{}', not '{}': applying it would not update this workload",
            spec.metadata.name, ws.name
        ));
    }
    if policy_config.enforce_on_deploy {
        if let Err(e) = policy::gate_deploy(&spec, policy_config) {
            f.blockers.push(format!("policy: {e:#}"));
        }
    }
    let quotas = QuotaStore::load(&dir.join("quotas.json"))?;
    if let Err(e) = policy::check_quota_gate(&quotas, &spec) {
        f.blockers.push(format!("{e:#}"));
    }
    match sources.current() {
        Some((current, source)) => {
            let plan = revisions::diff_yaml(
                &revisions::canonical_yaml(&current)?,
                &revisions::canonical_yaml(&spec)?,
            )?;
            f.effects.push(format!("compared with the {source}"));
            plan_effects(ws, &plan, f);
            match ws.runtime {
                RuntimeKind::Podman | RuntimeKind::Docker => f.warnings.push(format!(
                    "`aether update` on {} stops and removes the container before running the new one, whatever changed",
                    ws.runtime
                )),
                RuntimeKind::KubeVirt => f.warnings.push(
                    "`aether update` on kubevirt deletes the VM and its disk (DataVolume) before creating it again; the dashboard and API update does not delete them"
                        .to_string(),
                ),
                RuntimeKind::Kubernetes => {}
            }
            f.plan = Some(plan);
        }
        None => f
            .warnings
            .push("no current spec is available, so the changes cannot be listed".to_string()),
    }
    let target = target_runtime_type(ws.runtime);
    if !spec.runtime.allow.contains(&target) {
        f.warnings.push(format!(
            "runtime.allow no longer includes {}, where the workload runs",
            ws.runtime
        ));
    }
    if let Some(selected) = Engine::new().decide_trace(&spec).selected {
        if selected != ws.runtime {
            f.warnings.push(format!(
                "the engine would place this spec on {selected}; an update stays on {} (use migrate to move it)",
                ws.runtime
            ));
        }
    }
    Ok(())
}

fn target_runtime_type(runtime: RuntimeKind) -> RuntimeType {
    match runtime {
        RuntimeKind::Podman | RuntimeKind::Docker => RuntimeType::Container,
        RuntimeKind::Kubernetes => RuntimeType::Kube,
        RuntimeKind::KubeVirt => RuntimeType::Kubevirt,
    }
}

fn delete(ws: &WorkloadState, sources: &SpecSources, f: &mut Findings) {
    f.effects.push(format!(
        "removes the workload's resources on {} and drops it from Aether's state",
        ws.runtime
    ));
    f.effects
        .push("applied revisions, incidents and the audit trail are kept".to_string());
    if ws.runtime == RuntimeKind::KubeVirt {
        f.warnings
            .push("the VM's disk (DataVolume) is deleted with it".to_string());
    }
    if let Some((spec, _)) = sources.current() {
        if spec.persistence.enabled {
            f.warnings.push(format!(
                "persistence is enabled ({}): deleting may destroy its data; back it up first",
                spec.requirements.storage
            ));
        }
    }
    if !ws.atlas_volume_ids.is_empty() {
        f.warnings.push(format!(
            "`aether delete` releases {} Atlas volume(s): {}",
            ws.atlas_volume_ids.len(),
            ws.atlas_volume_ids.join(", ")
        ));
    }
    if sources.applied.is_none() {
        f.warnings.push(
            "no applied revision is recorded, so Aether cannot redeploy it from history"
                .to_string(),
        );
    }
}

fn migrate(ws: &WorkloadState, target: &str, sources: &SpecSources, f: &mut Findings) {
    let target: RuntimeKind = match target.parse() {
        Ok(t) => t,
        Err(e) => {
            f.blockers.push(format!("{e:#}"));
            return;
        }
    };
    if target == ws.runtime {
        f.blockers
            .push(format!("the workload already runs on {target}"));
        return;
    }
    let Some(spec) = sources.file.clone() else {
        f.blockers.push(format!(
            "migration reads the spec file, which cannot be loaded: {}",
            sources.file_error.as_deref().unwrap_or("unknown error")
        ));
        return;
    };
    pending_warning(sources, "migration deploys the spec file", f);
    f.effects.push(format!(
        "deploys the spec on {target} and removes the instance on {}",
        ws.runtime
    ));
    if !spec.runtime.allow.contains(&target_runtime_type(target)) {
        f.warnings.push(format!(
            "runtime.allow in the spec does not include {target}"
        ));
    }
    let before: Vec<String> = field_notes(&spec, ws.runtime)
        .into_iter()
        .map(|n| format!("{} ({})", n.path, n.support))
        .collect();
    let lost: Vec<String> = field_notes(&spec, target)
        .into_iter()
        .map(|n| format!("{} ({})", n.path, n.support))
        .filter(|n| !before.contains(n))
        .collect();
    if !lost.is_empty() {
        f.warnings.push(format!(
            "{target} does not fully honour fields that matter to this spec: {}",
            lost.join(", ")
        ));
    }
    if spec.persistence.enabled {
        f.warnings
            .push("persistence is enabled: `aether migrate` does not copy volume data".to_string());
    }
    if !ws.atlas_volume_ids.is_empty() {
        f.warnings
            .push("Atlas volumes are not moved to the new runtime".to_string());
    }
    if ws.runtime == RuntimeKind::KubeVirt {
        f.warnings
            .push("removing the source VM deletes its disk (DataVolume)".to_string());
    }
    let advisor =
        crate::ai::migration::MigrationAdvisor::new(crate::config::MigrationConfig::default());
    let advice = advisor.advise(&spec, ws.runtime, target);
    f.effects.push(format!(
        "the migration advisor suggests the {} strategy (a heuristic estimate, not a measurement)",
        advice.recommended_strategy
    ));
    f.warnings.extend(advice.warnings);
}

fn rollback(
    ws: &WorkloadState,
    revision: Option<u64>,
    log: &revisions::RevisionLog,
    f: &mut Findings,
) -> Result<()> {
    let target = match revision {
        Some(id) => log
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("revision {id} not found")),
        None => log.rollback_target(),
    };
    let target = match target {
        Ok(t) => t,
        Err(e) => {
            f.blockers.push(format!("{e:#}"));
            return Ok(());
        }
    };
    f.target_revision = Some(target.id);
    if target.runtime != ws.runtime {
        f.blockers.push(format!(
            "revision {} ran on {}, the workload now runs on {}: migrate back first",
            target.id, target.runtime, ws.runtime
        ));
    }
    f.effects.push(format!(
        "applies revision {} (recorded {}) on {}",
        target.id, target.at, ws.runtime
    ));
    if let Some(current) = log.latest() {
        let plan = revisions::diff_yaml(&current.spec_yaml, &target.spec_yaml)?;
        plan_effects(ws, &plan, f);
        f.plan = Some(plan);
    }
    f.warnings.push(
        "only the spec is rolled back; images are rebuilt and volume data is not restored"
            .to_string(),
    );
    Ok(())
}

pub fn format(p: &Preview) -> String {
    let verdict = match p.verdict {
        Verdict::Blocked => "BLOCKED",
        Verdict::Caution => "CAUTION",
        Verdict::Clear => "CLEAR",
    };
    let mut out = format!(
        "Preview: {} '{}' on {}  [{verdict}]\n",
        p.action, p.workload, p.runtime
    );
    let mut section = |title: &str, lines: &[String]| {
        if !lines.is_empty() {
            out.push_str(&format!("\n{title}\n"));
            for l in lines {
                out.push_str(&format!("  - {}\n", crate::speclint::sanitize(l)));
            }
        }
    };
    section("Blockers", &p.blockers);
    section("Warnings", &p.warnings);
    section("What it does", &p.effects);
    if let Some(plan) = p.plan.as_ref().filter(|p| p.has_changes()) {
        out.push('\n');
        out.push_str(&plan.render_text());
    }
    out.push_str("\nNothing was changed. This preview did not contact the runtime.\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::revisions::tests::{instance, spec};

    fn setup(dir: &Path) -> WorkloadState {
        let spec_path = dir.join("web.yaml");
        let w = spec("web", "256Mi");
        std::fs::write(&spec_path, revisions::canonical_yaml(&w).unwrap()).unwrap();
        let inst = instance("web", RuntimeKind::Podman);
        revisions::record(dir, revisions::RevisionOp::Deploy, &w, &inst).unwrap();
        WorkloadState::new("web".into(), RuntimeKind::Podman, inst, spec_path)
    }

    fn run(dir: &Path, ws: &WorkloadState, action: Action) -> Preview {
        preview(dir, ws, &action, &PolicyConfig::default()).unwrap()
    }

    fn lenient() -> PolicyConfig {
        PolicyConfig {
            enforce_on_deploy: false,
            ..Default::default()
        }
    }

    #[test]
    fn update_lists_changes_and_redacts_secrets() {
        let dir = tempfile::tempdir().unwrap();
        let ws = setup(dir.path());
        let yaml = revisions::canonical_yaml(&spec("web", "1Gi"))
            .unwrap()
            .replace("hunter2", "s3cret-new");
        let p = preview(dir.path(), &ws, &Action::Update { yaml }, &lenient()).unwrap();
        assert_eq!(p.verdict, Verdict::Caution, "{:?}", p.blockers);
        let paths: Vec<_> = p
            .plan
            .as_ref()
            .unwrap()
            .changes
            .iter()
            .map(|c| c.path.as_str())
            .collect();
        assert!(paths.contains(&"requirements.memory"), "{paths:?}");
        let text = serde_json::to_string(&p).unwrap() + &format(&p);
        assert!(!text.contains("s3cret-new") && !text.contains("hunter2"));
    }

    #[test]
    fn update_of_the_wrong_or_broken_spec_is_blocked() {
        let dir = tempfile::tempdir().unwrap();
        let ws = setup(dir.path());
        let other = revisions::canonical_yaml(&spec("other", "256Mi")).unwrap();
        for yaml in [
            other,
            "not: [valid".to_string(),
            "kind: Workload".to_string(),
        ] {
            let p = preview(dir.path(), &ws, &Action::Update { yaml }, &lenient()).unwrap();
            assert_eq!(p.verdict, Verdict::Blocked, "{p:?}");
        }
    }

    #[test]
    fn a_hold_blocks_every_action_and_dependents_are_listed() {
        let dir = tempfile::tempdir().unwrap();
        let ws = setup(dir.path());
        let mut graph = DependencyGraph::new();
        graph.add_dependency("api", "web");
        graph.save(&dir.path().join("dependencies.json")).unwrap();
        holds::set(dir.path(), "web", "freeze", None).unwrap();
        for action in [Action::Delete, Action::Stop, Action::Restart] {
            let p = run(dir.path(), &ws, action);
            assert_eq!(p.verdict, Verdict::Blocked);
            assert_eq!(p.affected, vec!["api"]);
        }
    }

    #[test]
    fn migrate_names_what_the_target_ignores() {
        let dir = tempfile::tempdir().unwrap();
        let ws = setup(dir.path());
        let p = run(
            dir.path(),
            &ws,
            Action::Migrate {
                target: "podman".into(),
            },
        );
        assert_eq!(p.verdict, Verdict::Blocked);
        let p = run(
            dir.path(),
            &ws,
            Action::Migrate {
                target: "kubevirt".into(),
            },
        );
        assert_eq!(p.verdict, Verdict::Caution);
        assert!(
            p.warnings.iter().any(|w| w.contains("does not include")),
            "{:?}",
            p.warnings
        );
    }

    #[test]
    fn rollback_needs_a_different_revision() {
        let dir = tempfile::tempdir().unwrap();
        let ws = setup(dir.path());
        let p = run(dir.path(), &ws, Action::Rollback { revision: None });
        assert_eq!(p.verdict, Verdict::Blocked);
        revisions::record(
            dir.path(),
            revisions::RevisionOp::Update,
            &spec("web", "1Gi"),
            &ws.instance,
        )
        .unwrap();
        let p = run(dir.path(), &ws, Action::Rollback { revision: None });
        assert_eq!(p.target_revision, Some(1));
        assert_eq!(p.verdict, Verdict::Caution);
    }
}
