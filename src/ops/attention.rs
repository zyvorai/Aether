// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! The attention queue: what an operator should look at first, with the next
//! command to run.

use crate::audit::{ActionResult, AuditLog};
use crate::health::HealthHistory;
use crate::ops::{
    self, changes::Level, explain::SpecSources, holds, incidents, journal, observed, store,
};
use crate::state::{StateStore, WorkloadState};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct Item {
    pub level: Level,
    pub workload: Option<String>,
    /// Stable identifier of the check, such as `open-incident`.
    pub kind: &'static str,
    pub title: String,
    pub detail: String,
    /// A command that shows more or deals with it.
    pub next: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Queue {
    pub generated_at: String,
    pub workloads_checked: usize,
    pub items: Vec<Item>,
}

/// Checks that concern one workload.
pub fn for_workload(dir: &Path, ws: &WorkloadState, now: DateTime<Utc>) -> Result<Vec<Item>> {
    let name = ws.name.as_str();
    let mut items = Vec::new();
    let mut push = |level, kind, title: String, detail: String, next: Option<String>| {
        items.push(Item {
            level,
            workload: Some(name.to_string()),
            kind,
            title,
            detail,
            next,
        });
    };

    let all = incidents::load(dir)?;
    for i in all
        .incidents
        .iter()
        .filter(|i| i.workload == name && i.is_open())
    {
        push(
            Level::Critical,
            "open-incident",
            format!("Incident {} is open", i.id),
            format!(
                "{} for {}, {} failed checks{}",
                i.last_state,
                ops::format_duration(i.duration_secs(now)),
                i.failed_checks,
                if i.acknowledged.is_some() {
                    ""
                } else {
                    ", not acknowledged"
                }
            ),
            Some(format!("aether incidents show {}", i.id)),
        );
    }

    let audit: AuditLog = store::load(&journal::audit_path(dir))?;
    let day = now - chrono::Duration::hours(24);
    let failures: Vec<_> = audit
        .events()
        .iter()
        .filter(|e| {
            e.workload == name
                && e.result == ActionResult::Failure
                && ops::parse_time(&e.timestamp).is_some_and(|t| t >= day)
        })
        .collect();
    if let Some(last) = failures.last() {
        push(
            Level::Warning,
            "failed-operation",
            format!("{} failed operation(s) in 24h", failures.len()),
            format!("latest: {}", crate::speclint::sanitize(&last.message)),
            Some(format!("aether changes --workload {name} --since 24h")),
        );
    }

    if let Some(e) = all.observe_errors.get(name) {
        push(
            Level::Warning,
            "unobservable",
            "Status cannot be read".to_string(),
            format!("{} at {}", e.message, e.at),
            Some(format!("aether status {name}")),
        );
    } else {
        let history: HealthHistory = store::load(&incidents::health_path(dir))?;
        let last = history.records.iter().rev().find(|r| r.workload == name);
        match last.and_then(|r| ops::parse_time(&r.timestamp)) {
            None => push(
                Level::Info,
                "never-observed",
                "No health data".to_string(),
                "no status check has been recorded; `aether serve` records them".to_string(),
                Some("aether health collect".to_string()),
            ),
            Some(t) if (now - t).num_seconds() > observed::stale_secs() => push(
                Level::Info,
                "stale-health",
                "Health data is stale".to_string(),
                format!(
                    "last check was {} ago",
                    ops::format_duration((now - t).num_seconds())
                ),
                Some("aether health collect".to_string()),
            ),
            Some(_) => {}
        }
    }

    let sources = SpecSources::load(dir, ws)?;
    if let Some(plan) = &sources.pending {
        push(
            Level::Warning,
            "unapplied-spec",
            "Spec file has unapplied changes".to_string(),
            format!(
                "{} field(s) differ from the applied revision: {}",
                plan.changes.len(),
                plan.changes
                    .iter()
                    .take(5)
                    .map(|c| c.path.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Some(format!(
                "aether preview {name} --update {}",
                ws.spec_path.display()
            )),
        );
    }
    if let Some(error) = &sources.file_error {
        push(
            Level::Warning,
            "spec-unreadable",
            "Spec file cannot be loaded".to_string(),
            crate::speclint::sanitize(error),
            sources
                .applied
                .as_ref()
                .map(|_| format!("aether history {name}")),
        );
    }

    if let Some(h) = holds::get(dir, name)? {
        push(
            Level::Info,
            "on-hold",
            "On hold".to_string(),
            format!(
                "{} (by {}, until {})",
                h.reason,
                h.actor,
                h.expires_at.as_deref().unwrap_or("released")
            ),
            Some(format!("aether hold release {name}")),
        );
    }
    Ok(items)
}

/// Most urgent first, then by workload name.
fn sort(items: &mut [Item]) {
    items.sort_by(|a, b| {
        b.level
            .cmp(&a.level)
            .then_with(|| a.workload.cmp(&b.workload))
            .then_with(|| a.kind.cmp(b.kind))
    });
}

/// The queue for the whole fleet.
pub fn build(dir: &Path, state: &StateStore) -> Result<Queue> {
    let now = Utc::now();
    let mut items = Vec::new();
    for ws in state.list() {
        items.extend(for_workload(dir, ws, now)?);
    }
    let audit: AuditLog = store::load(&journal::audit_path(dir))?;
    let tampered = audit
        .events()
        .iter()
        .filter(|e| !AuditLog::verify_event_integrity(e))
        .count();
    if tampered > 0 {
        items.push(Item {
            level: Level::Critical,
            workload: None,
            kind: "audit-integrity",
            title: format!("{tampered} audit event(s) fail the integrity check"),
            detail: "their recorded fields no longer match their hash".to_string(),
            next: Some("aether audit".to_string()),
        });
    }
    sort(&mut items);
    Ok(Queue {
        generated_at: now.to_rfc3339(),
        workloads_checked: state.workloads.len(),
        items,
    })
}

pub fn format_items(items: &[Item]) -> String {
    let mut out = String::new();
    for i in items {
        let mark = match i.level {
            Level::Critical => "CRIT",
            Level::Warning => "WARN",
            Level::Info => "INFO",
        };
        out.push_str(&format!(
            "  {mark}  {:<20} {}\n        {}\n",
            i.workload.as_deref().unwrap_or("-"),
            i.title,
            i.detail
        ));
        if let Some(next) = &i.next {
            out.push_str(&format!("        next: {next}\n"));
        }
    }
    out
}

pub fn format_queue(q: &Queue) -> String {
    if q.items.is_empty() {
        return format!(
            "Nothing needs attention across {} workload(s), based on recorded data.\n",
            q.workloads_checked
        );
    }
    let mut out = format!(
        "{} item(s) across {} workload(s)\n\n",
        q.items.len(),
        q.workloads_checked
    );
    out.push_str(&format_items(&q.items));
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::revisions::tests::{instance, spec};
    use crate::runtime::RuntimeKind;

    #[test]
    fn ranks_incidents_above_housekeeping() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let mut state = StateStore::new();
        for name in ["quiet", "web"] {
            let spec_path = d.join(format!("{name}.yaml"));
            std::fs::write(
                &spec_path,
                crate::ops::revisions::canonical_yaml(&spec(name, "256Mi")).unwrap(),
            )
            .unwrap();
            state.upsert(
                name.into(),
                WorkloadState::new(
                    name.into(),
                    RuntimeKind::Podman,
                    instance(name, RuntimeKind::Podman),
                    spec_path,
                ),
            );
        }
        let now = Utc::now().to_rfc3339();
        for _ in 0..2 {
            incidents::record(d, &incidents::tests::obs("web", false, &now)).unwrap();
        }
        holds::set(d, "quiet", "freeze", None).unwrap();
        let q = build(d, &state).unwrap();
        assert_eq!(q.items[0].kind, "open-incident");
        let kinds: Vec<_> = q.items.iter().map(|i| i.kind).collect();
        assert!(kinds.contains(&"never-observed"), "{kinds:?}");
        assert!(kinds.contains(&"on-hold"), "{kinds:?}");
        assert!(format_queue(&q).contains("aether incidents show 1"));
    }

    #[test]
    fn an_empty_fleet_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let q = build(dir.path(), &StateStore::new()).unwrap();
        assert!(format_queue(&q).contains("Nothing needs attention across 0"));
    }
}
