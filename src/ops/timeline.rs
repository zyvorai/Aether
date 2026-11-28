// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! One chronological view of a workload: what was applied and done to it, when
//! its health changed, its incidents, notes and holds.
//!
//! It merges the change feed with transitions derived from the recorded health
//! checks. A stretch without checks is its own entry, so silence is never read
//! as "nothing went wrong".

use crate::health::HealthRecord;
use crate::ops::changes::{self, Level};
use crate::ops::{self, board, incidents, journal, planned, revisions, store};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Entries read from the change feed before filtering.
const FEED_LIMIT: usize = 5_000;

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// Deploys, updates, other operations and warnings.
    Change,
    /// Transitions and gaps in the health checks.
    Health,
    Incident,
    Note,
    Hold,
}

impl Category {
    pub const ALL: [Category; 5] = [
        Category::Change,
        Category::Health,
        Category::Incident,
        Category::Note,
        Category::Hold,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Category::Change => "change",
            Category::Health => "health",
            Category::Incident => "incident",
            Category::Note => "note",
            Category::Hold => "hold",
        }
    }
}

/// Parse a comma separated filter such as `health,incidents`. Empty means all.
pub fn parse_categories(value: &str) -> Result<Vec<Category>> {
    let mut out = Vec::new();
    for part in value.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let singular = part.strip_suffix('s').unwrap_or(part);
        let found = Category::ALL
            .into_iter()
            .find(|c| c.name() == singular)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "unknown timeline filter '{}': use changes, health, incidents, notes or holds",
                    part.escape_default()
                )
            })?;
        if !out.contains(&found) {
            out.push(found);
        }
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    pub at: String,
    pub category: Category,
    /// Stable identifier such as `deploy`, `became-not-ready` or `checks-gap`.
    pub kind: &'static str,
    pub level: Level,
    pub title: String,
    pub detail: String,
    pub actor: Option<String>,
    pub revision: Option<u64>,
    /// The revision this one replaced, to compare against.
    pub previous_revision: Option<u64>,
    pub incident: Option<u64>,
    /// Spec fields that differ from the previous revision.
    pub fields: Vec<String>,
}

impl Entry {
    fn new(
        at: impl Into<String>,
        category: Category,
        kind: &'static str,
        level: Level,
        title: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            at: at.into(),
            category,
            kind,
            level,
            title: title.into(),
            detail: detail.into(),
            actor: None,
            revision: None,
            previous_revision: None,
            incident: None,
            fields: Vec::new(),
        }
    }
}

/// How much health checking stands behind the window.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Coverage {
    /// Checks recorded inside the window.
    pub checks: usize,
    pub ready: usize,
    /// Oldest check still kept; transitions before it are not known.
    pub retained_since: Option<String>,
    pub last_check_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Timeline {
    pub workload: String,
    pub managed: bool,
    pub since: String,
    pub until: String,
    /// Entries per category in the window, before filtering.
    pub counts: BTreeMap<&'static str, usize>,
    /// Entries that match the filter, before the limit.
    pub total: usize,
    pub truncated: bool,
    pub health: Coverage,
    /// Newest first.
    pub entries: Vec<Entry>,
    /// Records that could not be read; the timeline is incomplete when non-empty.
    pub unavailable: Vec<String>,
    /// Whether anything at all is recorded about this name.
    pub known: bool,
}

#[derive(Debug, Clone)]
pub struct Query {
    pub workload: String,
    /// Runtime of the managed workload; `None` when it is not managed.
    pub runtime: Option<crate::runtime::RuntimeKind>,
    pub since: DateTime<Utc>,
    pub now: DateTime<Utc>,
    /// Empty means every category.
    pub only: Vec<Category>,
    pub limit: usize,
    /// A longer silence between two checks is a gap.
    pub gap_secs: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Health {
    Ready,
    NotReady,
    Stopped,
}

impl Health {
    fn words(self) -> &'static str {
        match self {
            Health::Ready => "ready",
            Health::NotReady => "not ready",
            Health::Stopped => "stopped on purpose",
        }
    }

    fn kind(self) -> &'static str {
        match self {
            Health::Ready => "became-ready",
            Health::NotReady => "became-not-ready",
            Health::Stopped => "became-stopped",
        }
    }

    fn level(self) -> Level {
        match self {
            Health::NotReady => Level::Critical,
            _ => Level::Info,
        }
    }
}

fn capital(s: &str) -> String {
    let mut c = s.chars();
    c.next()
        .map(|f| f.to_uppercase().chain(c).collect())
        .unwrap_or_default()
}

/// Transitions, restarts and gaps in checks, oldest first. `records` must be
/// one workload's, sorted by time.
fn health_entries(
    records: &[(DateTime<Utc>, &HealthRecord)],
    downtime: &[planned::Interval],
    gap_secs: i64,
    now: Option<DateTime<Utc>>,
) -> Vec<Entry> {
    struct Prev<'a> {
        at: DateTime<Utc>,
        record: &'a HealthRecord,
        status: Health,
        /// First check of the current unbroken run of this status.
        run_from: DateTime<Utc>,
    }
    let mut out = Vec::new();
    let mut prev: Option<Prev> = None;
    for (at, r) in records {
        let runtime = r.runtime.to_string();
        let status = if r.ready {
            Health::Ready
        } else if downtime.iter().any(|d| d.covers(*at, &runtime)) {
            Health::Stopped
        } else {
            Health::NotReady
        };
        let entry = |kind, level, title: String, detail: String| {
            Entry::new(
                r.timestamp.clone(),
                Category::Health,
                kind,
                level,
                title,
                detail,
            )
        };
        let mut run_from = *at;
        match &prev {
            None => out.push(entry(
                "first-check",
                status.level(),
                format!("First check on record: {}", status.words()),
                format!("state {} on {runtime}", r.state),
            )),
            Some(p) => {
                let silence = (*at - p.at).num_seconds();
                if silence > gap_secs {
                    out.push(entry(
                        "checks-gap",
                        Level::Warning,
                        format!("No health checks for {}", ops::format_duration(silence)),
                        format!(
                            "none recorded between {} and {}; what happened in between is not known",
                            short(&p.record.timestamp),
                            short(&r.timestamp)
                        ),
                    ));
                    if status != p.status {
                        out.push(entry(
                            status.kind(),
                            status.level(),
                            format!("{} when checks resumed", capital(status.words())),
                            format!("state {} on {runtime}", r.state),
                        ));
                    }
                } else if status != p.status {
                    out.push(entry(
                        status.kind(),
                        status.level(),
                        match status {
                            Health::Stopped => "Stopped on purpose".to_string(),
                            _ => format!("Became {}", status.words()),
                        },
                        format!(
                            "state {}; checks had found it {} for {}",
                            r.state,
                            p.status.words(),
                            ops::format_duration((p.at - p.run_from).num_seconds())
                        ),
                    ));
                } else {
                    run_from = p.run_from;
                }
                if r.runtime == p.record.runtime && r.restart_count > p.record.restart_count {
                    out.push(entry(
                        "restarted",
                        Level::Warning,
                        "Restarted by the runtime".to_string(),
                        format!(
                            "restart count went from {} to {}",
                            p.record.restart_count, r.restart_count
                        ),
                    ));
                }
            }
        }
        prev = Some(Prev {
            at: *at,
            record: r,
            status,
            run_from,
        });
    }
    // `now` is given for managed workloads, which should still be checked.
    if let (Some(now), Some(p)) = (now, &prev) {
        let silence = (now - p.at).num_seconds();
        if silence > gap_secs {
            out.push(Entry::new(
                now.to_rfc3339(),
                Category::Health,
                "checks-gap",
                Level::Warning,
                format!("No health checks for {}", ops::format_duration(silence)),
                format!(
                    "the last one was at {}; the current state is not known",
                    short(&p.record.timestamp)
                ),
            ));
        }
    }
    out
}

fn short(ts: &str) -> String {
    ts.get(..19).unwrap_or(ts).replace('T', " ") + " UTC"
}

/// Readable titles for audited operations (`STOP OK` -> `Stopped`).
fn operation_title(title: &str) -> String {
    let Some((action, result)) = title.split_once(' ') else {
        return title.to_string();
    };
    let done = match action {
        "BUILD" => "Built",
        "DEPLOY" => "Deployed",
        "START" => "Started",
        "STOP" => "Stopped",
        "DELETE" => "Deleted",
        "MIGRATE" => "Migrated",
        "SCALE" => "Scaled",
        "CONFIG" => "Configuration changed",
        "BACKUP" => "Backed up",
        "RESTORE" => "Restored",
        _ => return title.to_string(),
    };
    let name = match action {
        "CONFIG" => "Configuration change".to_string(),
        other => capital(&other.to_lowercase()),
    };
    match result {
        "OK" => done.to_string(),
        "FAIL" => format!("{name} failed"),
        "WARN" => format!("{done} with a warning"),
        _ => title.to_string(),
    }
}

fn from_change(c: changes::Change) -> Entry {
    use changes::Kind;
    let hold = c.kind == Kind::Operation
        && (c.detail.starts_with("hold placed: ") || c.detail.starts_with("hold released: "));
    let (category, kind, title, detail) = if hold {
        let placed = c.detail.starts_with("hold placed: ");
        let reason = c
            .detail
            .split_once(": ")
            .map_or("", |(_, reason)| reason)
            .to_string();
        (
            Category::Hold,
            if placed {
                "hold-placed"
            } else {
                "hold-released"
            },
            if placed {
                "Hold placed"
            } else {
                "Hold released"
            }
            .to_string(),
            reason,
        )
    } else {
        match c.kind {
            Kind::Deploy => (Category::Change, "deploy", c.title, c.detail),
            Kind::Operation => (
                Category::Change,
                "operation",
                operation_title(&c.title),
                c.detail,
            ),
            Kind::Event => (Category::Change, "event", c.title, c.detail),
            Kind::IncidentOpened => (Category::Incident, "incident-opened", c.title, c.detail),
            Kind::IncidentResolved => (Category::Incident, "incident-resolved", c.title, c.detail),
            Kind::Note => (Category::Note, "note", c.title, c.detail),
        }
    };
    Entry {
        actor: c.actor,
        revision: c.revision,
        incident: c.incident,
        fields: c.fields,
        ..Entry::new(c.at, category, kind, c.level, capital(&title), detail)
    }
}

pub fn collect(dir: &Path, q: &Query) -> Result<Timeline> {
    ops::check_name(&q.workload)?;
    let name = q.workload.as_str();
    // Tolerate small clock differences between writers.
    let until = q.now + chrono::Duration::minutes(5);
    let feed = changes::collect(
        dir,
        &changes::Query {
            since: q.since,
            until,
            workloads: Some(vec![name.to_string()]),
            limit: FEED_LIMIT,
        },
    )?;
    let mut unavailable = feed.unavailable;
    let mut feed_truncated = feed.truncated;
    let mut entries: Vec<Entry> = feed.changes.into_iter().map(from_change).collect();
    let mut read = |what: &str, e: anyhow::Error| {
        let line = format!("{what}: {e:#}");
        if !unavailable.contains(&line) {
            unavailable.push(line);
        }
    };

    let log = revisions::load(dir, name).unwrap_or_else(|e| {
        read(&format!("revisions of {name}"), e);
        Default::default()
    });
    for e in &mut entries {
        let Some(id) = e.revision else { continue };
        e.previous_revision = log
            .revisions
            .iter()
            .position(|r| r.id == id)
            .and_then(|i| i.checked_sub(1))
            .map(|i| log.revisions[i].id);
    }

    let all_incidents = incidents::load(dir).unwrap_or_else(|e| {
        read("incidents", e);
        Default::default()
    });
    let in_window = |ts: &str| ops::parse_time(ts).is_some_and(|t| t >= q.since && t <= until);
    for i in all_incidents
        .incidents
        .iter()
        .filter(|i| i.workload == name)
    {
        if let Some(a) = i.acknowledged.as_ref().filter(|a| in_window(&a.at)) {
            entries.push(Entry {
                actor: Some(a.actor.clone()),
                incident: Some(i.id),
                ..Entry::new(
                    a.at.clone(),
                    Category::Incident,
                    "incident-acknowledged",
                    Level::Info,
                    format!("Incident {} acknowledged", i.id),
                    "",
                )
            });
        }
    }
    if let Some(error) = all_incidents
        .observe_errors
        .get(name)
        .filter(|e| in_window(&e.at))
    {
        entries.push(Entry::new(
            error.at.clone(),
            Category::Health,
            "unreadable",
            Level::Warning,
            "Status could not be read",
            crate::speclint::sanitize(&error.message),
        ));
    }

    let health = board::health_by_workload(dir).unwrap_or_else(|e| {
        read("health checks", e);
        Default::default()
    });
    let empty = Vec::new();
    let mine = health.get(name).unwrap_or(&empty);
    let records = board::timed(mine);
    let audit: crate::audit::AuditLog = store::load(&journal::audit_path(dir)).unwrap_or_default();
    let downtime = planned::intervals(
        audit.events().iter().filter(|e| e.workload == name),
        mine,
        all_incidents.expected_down.get(name),
    );
    let stopped = q.runtime.is_some_and(|r| {
        all_incidents
            .expected_down
            .get(name)
            .is_some_and(|d| d.runtime == r)
    });
    // Silence is expected of nothing: only a workload that is gone or stopped
    // on purpose has nothing to report.
    let watch_until = (q.runtime.is_some() && !stopped).then_some(q.now);
    // Added newest first, which the stable sort keeps for entries of one instant.
    entries.extend(
        health_entries(&records, &downtime, q.gap_secs, watch_until)
            .into_iter()
            .rev()
            .filter(|e| in_window(&e.at)),
    );
    let window: Vec<_> = records
        .iter()
        .filter(|(at, _)| *at >= q.since && *at <= until)
        .collect();
    let coverage = Coverage {
        checks: window.len(),
        ready: window.iter().filter(|(_, r)| r.ready).count(),
        retained_since: records.first().map(|(_, r)| r.timestamp.clone()),
        last_check_at: records.last().map(|(_, r)| r.timestamp.clone()),
    };

    let known = q.runtime.is_some()
        || !records.is_empty()
        || !log.revisions.is_empty()
        || audit.events().iter().any(|e| e.workload == name)
        || all_incidents.incidents.iter().any(|i| i.workload == name);

    let counts: BTreeMap<&'static str, usize> = Category::ALL
        .into_iter()
        .map(|c| (c.name(), entries.iter().filter(|e| e.category == c).count()))
        .collect();
    if !q.only.is_empty() {
        entries.retain(|e| q.only.contains(&e.category));
    }
    // Newest first; entries of the same instant keep a stable order.
    entries.sort_by_key(|e| std::cmp::Reverse((ops::parse_time(&e.at), e.category)));
    let total = entries.len();
    feed_truncated |= total > q.limit;
    entries.truncate(q.limit);

    Ok(Timeline {
        workload: name.to_string(),
        managed: q.runtime.is_some(),
        since: q.since.to_rfc3339(),
        until: q.now.to_rfc3339(),
        counts,
        total,
        truncated: feed_truncated,
        health: coverage,
        entries,
        unavailable,
        known,
    })
}

pub fn format(t: &Timeline) -> String {
    let mut out = format!(
        "Timeline of {}{} since {} (newest first)\n",
        t.workload,
        if t.managed { "" } else { " (not managed now)" },
        short(&t.since)
    );
    for u in &t.unavailable {
        out.push_str(&format!("warning: could not read {u}\n"));
    }
    match (&t.health.retained_since, t.health.checks) {
        (None, _) => out.push_str("Health checks: none recorded.\n"),
        (Some(from), checks) => out.push_str(&format!(
            "Health checks: {checks} in this window, {} ready. Kept since {}.\n",
            t.health.ready,
            short(from)
        )),
    }
    out.push('\n');
    if t.entries.is_empty() {
        out.push_str("Nothing recorded in this window for this filter.\n");
        return out;
    }
    for e in &t.entries {
        let mark = match e.level {
            Level::Critical => "!!",
            Level::Warning => "! ",
            Level::Info => "  ",
        };
        let detail = crate::speclint::sanitize(&e.detail);
        out.push_str(&format!(
            "{mark} {}  {:<8} {}{}{}\n",
            e.at.get(..19).unwrap_or(&e.at),
            e.category.name(),
            crate::speclint::sanitize(&e.title),
            if detail.is_empty() {
                String::new()
            } else {
                format!(" - {detail}")
            },
            e.actor
                .as_deref()
                .map(|a| format!(" [{}]", crate::speclint::sanitize(a)))
                .unwrap_or_default()
        ));
        if !e.fields.is_empty() {
            out.push_str(&format!("       changed: {}\n", e.fields.join(", ")));
        }
        if let (Some(rev), Some(prev)) = (e.revision, e.previous_revision) {
            out.push_str(&format!(
                "       diff: aether history {} --diff {prev} {rev}\n",
                t.workload
            ));
        }
    }
    if t.truncated {
        out.push_str(&format!(
            "Showing {} of at least {} entries; raise --limit or shorten --since.\n",
            t.entries.len(),
            t.total
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::incidents::Observation;
    use crate::ops::revisions::tests::{instance, spec};
    use crate::runtime::{InstanceState, RuntimeKind};

    fn observe(dir: &Path, ready: bool, restarts: u32, when: DateTime<Utc>) {
        incidents::record(
            dir,
            &Observation {
                workload: "web".into(),
                runtime: RuntimeKind::Podman,
                state: if ready {
                    InstanceState::Running
                } else {
                    InstanceState::Failed
                },
                ready,
                restart_count: restarts,
                message: None,
                at: when.to_rfc3339(),
            },
        )
        .unwrap();
    }

    fn query(now: DateTime<Utc>) -> Query {
        Query {
            workload: "web".into(),
            runtime: Some(RuntimeKind::Podman),
            since: now - chrono::Duration::hours(2),
            now,
            only: Vec::new(),
            limit: 100,
            gap_secs: 300,
        }
    }

    fn kinds(t: &Timeline) -> Vec<&'static str> {
        t.entries.iter().rev().map(|e| e.kind).collect()
    }

    #[test]
    fn health_transitions_gaps_and_restarts_are_derived_from_checks() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let now = Utc::now();
        let ago = |m: i64| now - chrono::Duration::minutes(m);
        observe(d, true, 0, ago(40));
        observe(d, true, 0, ago(39));
        // Twenty minutes of silence, then failures and a restart.
        observe(d, false, 0, ago(19));
        observe(d, false, 1, ago(18));
        observe(d, true, 1, ago(17));
        let mut q = query(now);
        q.only = vec![Category::Health];
        let t = collect(d, &q).unwrap();
        assert_eq!(
            kinds(&t),
            [
                "first-check",
                "checks-gap",
                "became-not-ready",
                "restarted",
                "became-ready",
                "checks-gap",
            ],
            "{:#?}",
            t.entries
        );
        assert_eq!(t.health.checks, 5);
        assert_eq!(t.health.ready, 3);
        assert!(t.entries[0].detail.contains("current state is not known"));
        assert!(t.entries[3].title.contains("when checks resumed"));
        // The incident opened by the second failure is counted but filtered out.
        assert_eq!(t.counts["incident"], 2);
        assert_eq!(t.total, 6);
    }

    #[test]
    fn merges_revisions_holds_incidents_and_notes_in_order() {
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
                crate::audit::AuditAction::Deploy,
                "web",
                Some("podman"),
                crate::audit::ActionResult::Success,
                "deployed",
                Some(&format!("{}{id}", journal::REVISION_MARKER)),
            )
            .unwrap();
        }
        crate::ops::holds::set(d, "web", "freeze", None).unwrap();
        crate::ops::holds::release(d, "web").unwrap();
        let now = Utc::now();
        observe(d, false, 0, now - chrono::Duration::seconds(2));
        observe(d, false, 0, now - chrono::Duration::seconds(1));
        incidents::add_note(d, 1, "looking").unwrap();
        incidents::acknowledge(d, 1).unwrap();
        let t = collect(d, &query(Utc::now())).unwrap();
        assert_eq!(t.counts["change"], 2);
        assert_eq!(t.counts["hold"], 2);
        assert_eq!(t.counts["note"], 1);
        assert_eq!(t.counts["incident"], 2);
        assert_eq!(t.counts["health"], 1);
        let newest_deploy = t.entries.iter().find(|e| e.kind == "deploy").unwrap();
        assert_eq!(
            (newest_deploy.revision, newest_deploy.previous_revision),
            (Some(2), Some(1))
        );
        assert_eq!(newest_deploy.fields, vec!["requirements.memory"]);
        let released = t
            .entries
            .iter()
            .find(|e| e.kind == "hold-released")
            .unwrap();
        assert_eq!(released.detail, "freeze");
        let times: Vec<_> = t.entries.iter().map(|e| ops::parse_time(&e.at)).collect();
        assert!(times.windows(2).all(|w| w[0] >= w[1]));
        let text = format(&t);
        assert!(text.contains("aether history web --diff 1 2"), "{text}");
        assert!(!serde_json::to_string(&t).unwrap().contains("hunter2"));
    }

    #[test]
    fn a_stop_on_purpose_is_not_shown_as_a_failure_or_a_gap() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let start = Utc::now() - chrono::Duration::minutes(1);
        observe(d, true, 0, start);
        incidents::expect_down(d, "web", RuntimeKind::Podman, "stopped").unwrap();
        observe(d, false, 0, Utc::now() + chrono::Duration::seconds(1));
        // Long after the last check there is still nothing to warn about.
        let mut q = query(Utc::now() + chrono::Duration::minutes(30));
        q.only = vec![Category::Health];
        let t = collect(d, &q).unwrap();
        assert_eq!(kinds(&t), ["first-check", "became-stopped"]);
        assert!(t.entries.iter().all(|e| e.level == Level::Info));
    }

    #[test]
    fn unknown_names_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        let mut q = query(Utc::now());
        q.runtime = None;
        let t = collect(dir.path(), &q).unwrap();
        assert!(!t.known);
        assert!(format(&t).contains("Nothing recorded"));
        assert_eq!(
            parse_categories("health, incidents,notes,health").unwrap(),
            [Category::Health, Category::Incident, Category::Note]
        );
        assert!(parse_categories("").unwrap().is_empty());
        assert!(parse_categories("everything").is_err());
        q.workload = "../etc".into();
        assert!(collect(dir.path(), &q).is_err());
    }

    #[test]
    fn operation_titles_read_as_sentences() {
        assert_eq!(operation_title("STOP OK"), "Stopped");
        assert_eq!(
            operation_title("CONFIG FAIL"),
            "Configuration change failed"
        );
        assert_eq!(operation_title("DELETE FAIL"), "Delete failed");
        assert_eq!(operation_title("something else"), "something else");
    }
}
