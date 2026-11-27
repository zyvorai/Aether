// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! "What changed": one ordered feed over applied revisions, the audit trail,
//! warning events and incidents.

use crate::audit::{ActionResult, AuditAction, AuditLog};
use crate::events::{EventBus, EventCategory, EventSeverity};
use crate::ops::{self, incidents, journal, revisions, store};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashSet;
use std::path::Path;

/// Changed field names listed per deploy.
const MAX_FIELDS: usize = 20;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    /// A spec was applied.
    Deploy,
    /// Any other audited operation (stop, delete, scale, hold, ...).
    Operation,
    /// A warning or worse from the event bus.
    Event,
    IncidentOpened,
    IncidentResolved,
    Note,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Level {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize)]
pub struct Change {
    pub at: String,
    pub workload: Option<String>,
    pub kind: Kind,
    pub level: Level,
    pub title: String,
    pub detail: String,
    pub actor: Option<String>,
    pub revision: Option<u64>,
    /// Set on incident entries and notes.
    pub incident: Option<u64>,
    /// Spec fields that differ from the previous revision.
    pub fields: Vec<String>,
}

impl Change {
    /// Whether this could explain a later incident: something was changed, or
    /// something else broke. Incident bookkeeping of the incident itself is not
    /// a cause; another workload's incident opening can be.
    pub fn is_possible_cause(&self) -> bool {
        matches!(
            self.kind,
            Kind::Deploy | Kind::Operation | Kind::Event | Kind::IncidentOpened
        )
    }
}

#[derive(Debug, Clone)]
pub struct Query {
    pub since: DateTime<Utc>,
    pub until: DateTime<Utc>,
    /// Restrict to these workloads (entries without a workload are dropped).
    pub workloads: Option<Vec<String>>,
    pub limit: usize,
}

impl Query {
    /// Everything since `since` up to now.
    pub fn since(since: DateTime<Utc>, workload: Option<String>, limit: usize) -> Self {
        Self {
            since,
            // Tolerate small clock differences between writers.
            until: Utc::now() + chrono::Duration::minutes(5),
            workloads: workload.map(|w| vec![w]),
            limit,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Feed {
    pub since: String,
    pub until: String,
    /// Entries matched before the limit was applied.
    pub total: usize,
    pub truncated: bool,
    /// Newest first.
    pub changes: Vec<Change>,
    /// Sources that could not be read; the feed is incomplete when non-empty.
    pub unavailable: Vec<String>,
}

fn detail_field<'a>(details: Option<&'a str>, key: &str) -> Option<&'a str> {
    details?
        .split("; ")
        .find_map(|part| part.strip_prefix(key)?.strip_prefix('='))
}

fn read<T>(result: Result<T>, what: &str, unavailable: &mut Vec<String>) -> Option<T> {
    match result {
        Ok(v) => Some(v),
        Err(e) => {
            unavailable.push(format!("{what}: {e:#}"));
            None
        }
    }
}

fn revision_changes(
    dir: &Path,
    q: &Query,
    out: &mut Vec<(DateTime<Utc>, Change)>,
    unavailable: &mut Vec<String>,
) -> HashSet<(String, u64)> {
    let mut seen = HashSet::new();
    let Ok(entries) = std::fs::read_dir(dir.join("revisions")) else {
        return seen;
    };
    let names: Vec<String> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().into_string().ok()?;
            name.strip_suffix(".json").map(str::to_string)
        })
        .filter(|n| q.workloads.as_ref().is_none_or(|w| w.contains(n)))
        .collect();
    for name in names {
        let Some(log) = read(
            revisions::load(dir, &name),
            &format!("revisions of {name}"),
            unavailable,
        ) else {
            continue;
        };
        for (i, r) in log.revisions.iter().enumerate() {
            seen.insert((name.clone(), r.id));
            let Some(at) = ops::parse_time(&r.at) else {
                continue;
            };
            if at < q.since || at > q.until {
                continue;
            }
            let fields: Vec<String> = i
                .checked_sub(1)
                .and_then(|p| revisions::diff_yaml(&log.revisions[p].spec_yaml, &r.spec_yaml).ok())
                .map(|plan| plan.changes.into_iter().map(|c| c.path).collect())
                .unwrap_or_default();
            let what = if i == 0 {
                "first recorded revision".to_string()
            } else if fields.is_empty() {
                "same spec as before".to_string()
            } else {
                format!("{} spec field(s) changed", fields.len())
            };
            let reason = r
                .reason
                .as_deref()
                .map(|r| format!("; {r}"))
                .unwrap_or_default();
            out.push((
                at,
                Change {
                    at: r.at.clone(),
                    workload: Some(name.clone()),
                    kind: Kind::Deploy,
                    level: Level::Info,
                    title: format!("{} revision {} on {}", r.op, r.id, r.runtime),
                    detail: format!("{what}{reason}"),
                    actor: Some(r.actor.clone()),
                    revision: Some(r.id),
                    incident: None,
                    fields: fields.into_iter().take(MAX_FIELDS).collect(),
                },
            ));
        }
    }
    seen
}

/// Gather the feed for a time window.
pub fn collect(dir: &Path, q: &Query) -> Result<Feed> {
    let mut out: Vec<(DateTime<Utc>, Change)> = Vec::new();
    let mut unavailable = Vec::new();
    let wanted = |w: Option<&str>| match &q.workloads {
        None => true,
        Some(list) => w.is_some_and(|w| list.iter().any(|l| l == w)),
    };
    let in_window = |ts: &str| ops::parse_time(ts).filter(|t| *t >= q.since && *t <= q.until);

    let seen = revision_changes(dir, q, &mut out, &mut unavailable);

    if let Some(log) = read(
        store::load::<AuditLog>(&journal::audit_path(dir)),
        "audit trail",
        &mut unavailable,
    ) {
        for e in log.events() {
            let Some(at) = in_window(&e.timestamp) else {
                continue;
            };
            if !wanted(Some(&e.workload)) {
                continue;
            }
            let details = e.details.as_deref();
            let revision = detail_field(details, "revision").and_then(|v| v.parse().ok());
            // The revision entry already tells this story.
            if revision.is_some_and(|id| seen.contains(&(e.workload.clone(), id))) {
                continue;
            }
            let level = match e.result {
                ActionResult::Failure => Level::Critical,
                ActionResult::Warning => Level::Warning,
                ActionResult::Success if e.action == AuditAction::Delete => Level::Warning,
                ActionResult::Success => Level::Info,
            };
            out.push((
                at,
                Change {
                    at: e.timestamp.clone(),
                    workload: Some(e.workload.clone()).filter(|w| !w.is_empty()),
                    kind: Kind::Operation,
                    level,
                    title: format!("{} {}", e.action, e.result),
                    detail: e.message.clone(),
                    actor: detail_field(details, "actor").map(str::to_string),
                    revision,
                    incident: None,
                    fields: Vec::new(),
                },
            ));
        }
    }

    if let Some(bus) = read(
        store::load::<EventBus>(&journal::events_path(dir)),
        "events",
        &mut unavailable,
    ) {
        for e in bus.events() {
            let Some(at) = in_window(&e.timestamp) else {
                continue;
            };
            let duplicate = e.metadata.contains_key(journal::EVENT_MARKER)
                || (e.category == EventCategory::Deployment && e.source == "cli");
            if duplicate || e.severity < EventSeverity::Warning || !wanted(e.workload.as_deref()) {
                continue;
            }
            out.push((
                at,
                Change {
                    at: e.timestamp.clone(),
                    workload: e.workload.clone(),
                    kind: Kind::Event,
                    level: if e.severity >= EventSeverity::Error {
                        Level::Critical
                    } else {
                        Level::Warning
                    },
                    title: e.title.clone(),
                    detail: e.message.clone(),
                    actor: Some(e.source.clone()),
                    revision: None,
                    incident: None,
                    fields: Vec::new(),
                },
            ));
        }
    }

    if let Some(incidents) = read(incidents::load(dir), "incidents", &mut unavailable) {
        for i in incidents
            .incidents
            .iter()
            .filter(|i| wanted(Some(&i.workload)))
        {
            let mut push = |ts: &str, kind, level, title: String, detail: String, actor| {
                if let Some(at) = in_window(ts) {
                    out.push((
                        at,
                        Change {
                            at: ts.to_string(),
                            workload: Some(i.workload.clone()),
                            kind,
                            level,
                            title,
                            detail,
                            actor,
                            revision: None,
                            incident: Some(i.id),
                            fields: Vec::new(),
                        },
                    ));
                }
            };
            push(
                &i.opened_at,
                Kind::IncidentOpened,
                Level::Critical,
                format!("incident {} started", i.id),
                format!("{} not ready ({})", i.workload, i.last_state),
                None,
            );
            if let (Some(ts), Some(r)) = (&i.resolved_at, i.resolution) {
                push(
                    ts,
                    Kind::IncidentResolved,
                    Level::Info,
                    format!("incident {} {r}", i.id),
                    format!(
                        "after {}",
                        ops::format_duration(i.duration_secs(Utc::now()))
                    ),
                    None,
                );
            }
            for n in &i.notes {
                push(
                    &n.at,
                    Kind::Note,
                    Level::Info,
                    format!("note on incident {}", i.id),
                    n.text.clone(),
                    Some(n.actor.clone()),
                );
            }
        }
    }

    out.sort_by_key(|(at, _)| std::cmp::Reverse(*at));
    let total = out.len();
    Ok(Feed {
        since: q.since.to_rfc3339(),
        until: q.until.to_rfc3339(),
        total,
        truncated: total > q.limit,
        changes: out.into_iter().take(q.limit).map(|(_, c)| c).collect(),
        unavailable,
    })
}

pub fn format_feed(feed: &Feed) -> String {
    let mut out = String::new();
    for u in &feed.unavailable {
        out.push_str(&format!("warning: could not read {u}\n"));
    }
    if feed.changes.is_empty() {
        out.push_str(&format!(
            "Nothing recorded since {}.\n",
            feed.since.get(..19).unwrap_or(&feed.since)
        ));
        return out;
    }
    for c in &feed.changes {
        let mark = match c.level {
            Level::Critical => "!!",
            Level::Warning => "! ",
            Level::Info => "  ",
        };
        out.push_str(&format!(
            "{mark} {}  {:<20} {} - {}{}\n",
            c.at.get(..19).unwrap_or(&c.at),
            c.workload.as_deref().unwrap_or("-"),
            c.title,
            crate::speclint::sanitize(&c.detail),
            c.actor
                .as_deref()
                .map(|a| format!(" [{a}]"))
                .unwrap_or_default()
        ));
        if !c.fields.is_empty() {
            out.push_str(&format!("       changed: {}\n", c.fields.join(", ")));
        }
    }
    if feed.truncated {
        out.push_str(&format!(
            "Showing {} of {} entries.\n",
            feed.changes.len(),
            feed.total
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::revisions::tests::{instance, spec};
    use crate::runtime::RuntimeKind;

    fn hour_ago() -> Query {
        Query::since(Utc::now() - chrono::Duration::hours(1), None, 100)
    }

    #[test]
    fn deploy_appears_once_with_changed_fields() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        for memory in ["256Mi", "512Mi"] {
            let id = revisions::record(
                d,
                revisions::RevisionOp::Deploy,
                &spec("web", memory),
                &instance("web", RuntimeKind::Podman),
            )
            .unwrap();
            journal::audit_with(
                d,
                AuditAction::Deploy,
                "web",
                Some("podman"),
                ActionResult::Success,
                "deployed",
                Some(&format!("{}{id}", journal::REVISION_MARKER)),
            )
            .unwrap();
        }
        journal::audit(
            d,
            AuditAction::Stop,
            "web",
            None,
            ActionResult::Failure,
            "stop failed",
        )
        .unwrap();
        let feed = collect(d, &hour_ago()).unwrap();
        assert_eq!(feed.total, 3, "{:#?}", feed.changes);
        assert_eq!(feed.changes[0].level, Level::Critical);
        assert_eq!(feed.changes[1].fields, vec!["requirements.memory"]);
        assert_eq!(feed.changes[2].fields, Vec::<String>::new());
    }

    #[test]
    fn filters_by_workload_window_and_limit() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        for name in ["a", "b", "b"] {
            journal::audit(
                d,
                AuditAction::Stop,
                name,
                None,
                ActionResult::Success,
                "stopped",
            )
            .unwrap();
        }
        let mut q = hour_ago();
        q.workloads = Some(vec!["b".into()]);
        assert_eq!(collect(d, &q).unwrap().total, 2);
        q.limit = 1;
        let feed = collect(d, &q).unwrap();
        assert!(feed.truncated);
        assert_eq!(feed.changes.len(), 1);
        q.since = Utc::now() + chrono::Duration::minutes(1);
        assert_eq!(collect(d, &q).unwrap().total, 0);
    }

    #[test]
    fn an_unreadable_source_is_reported_not_hidden() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(journal::audit_path(dir.path()), "{").unwrap();
        let feed = collect(dir.path(), &hour_ago()).unwrap();
        assert_eq!(feed.unavailable.len(), 1);
        assert!(format_feed(&feed).contains("could not read audit trail"));
    }
}
