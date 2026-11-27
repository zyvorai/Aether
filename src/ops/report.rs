// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! The operations report: availability, incidents, changes and open items for
//! a time window, as Markdown, HTML or JSON.

use crate::audit::{ActionResult, AuditLog};
use crate::ops::incidents::md_escape;
use crate::ops::{self, attention, changes, incidents, journal, store};
use crate::runtime::RuntimeKind;
use crate::state::StateStore;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::Path;

/// Changes listed in a report.
const MAX_CHANGES: usize = 200;

#[derive(Debug, Clone, Serialize)]
pub struct WorkloadRow {
    pub workload: String,
    pub runtime: RuntimeKind,
    pub checks: u32,
    pub ready_checks: u32,
    /// `None` when no check was recorded in the window.
    pub ready_pct: Option<f64>,
    pub incidents: u32,
    /// Time inside the window covered by incidents.
    pub incident_secs: i64,
    pub deploys: u32,
    pub failed_operations: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditCheck {
    pub events_in_window: usize,
    pub events_total: usize,
    pub failed_integrity: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct IncidentRow {
    pub id: u64,
    pub workload: String,
    pub opened_at: String,
    pub resolved_at: Option<String>,
    pub status: String,
    pub duration_secs: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub generated_at: String,
    pub since: String,
    pub workloads: Vec<WorkloadRow>,
    pub incidents: Vec<IncidentRow>,
    pub recovery: incidents::Recovery,
    pub changes: changes::Feed,
    pub attention: Vec<attention::Item>,
    pub audit: AuditCheck,
    /// What limits how far the numbers can be trusted.
    pub caveats: Vec<String>,
}

pub fn build(dir: &Path, state: &StateStore, since: DateTime<Utc>) -> Result<Report> {
    let now = Utc::now();
    let all = incidents::load(dir)?;
    let availability: incidents::Availability = store::load(&incidents::availability_path(dir))?;
    let audit: AuditLog = store::load(&journal::audit_path(dir))?;
    let in_window = |ts: &str| ops::parse_time(ts).is_some_and(|t| t >= since);
    let overlapping: Vec<&incidents::Incident> = all
        .incidents
        .iter()
        .filter(|i| i.resolved_at.as_deref().is_none_or(in_window))
        .collect();

    let feed = changes::collect(dir, &changes::Query::since(since, None, MAX_CHANGES))?;

    let mut names: Vec<_> = state.list();
    names.sort_by(|a, b| a.name.cmp(&b.name));
    let mut workloads = Vec::new();
    for ws in names {
        let counter = availability.since(&ws.name, since);
        let mine: Vec<_> = overlapping
            .iter()
            .filter(|i| i.workload == ws.name)
            .collect();
        let incident_secs = mine
            .iter()
            .map(|i| {
                let start = ops::parse_time(&i.opened_at).map_or(since, |t| t.max(since));
                let end = i
                    .resolved_at
                    .as_deref()
                    .and_then(ops::parse_time)
                    .unwrap_or(now);
                (end - start).num_seconds().max(0)
            })
            .sum();
        let events = audit
            .events()
            .iter()
            .filter(|e| e.workload == ws.name && in_window(&e.timestamp));
        let failed_operations = events
            .clone()
            .filter(|e| e.result == ActionResult::Failure)
            .count() as u32;
        let deploys = events
            .filter(|e| {
                e.result == ActionResult::Success
                    && e.details
                        .as_deref()
                        .is_some_and(|d| d.contains(journal::REVISION_MARKER))
            })
            .count() as u32;
        workloads.push(WorkloadRow {
            workload: ws.name.clone(),
            runtime: ws.runtime,
            checks: counter.checks,
            ready_checks: counter.ready,
            ready_pct: (counter.checks > 0)
                .then(|| f64::from(counter.ready) * 100.0 / f64::from(counter.checks)),
            incidents: mine.len() as u32,
            incident_secs,
            deploys,
            failed_operations,
        });
    }

    let incident_rows = overlapping
        .iter()
        .rev()
        .map(|i| IncidentRow {
            id: i.id,
            workload: i.workload.clone(),
            opened_at: i.opened_at.clone(),
            resolved_at: i.resolved_at.clone(),
            status: i
                .resolution
                .map_or_else(|| "open".to_string(), |r| r.to_string()),
            duration_secs: i.duration_secs(now),
        })
        .collect();

    let mut caveats = vec![
        "Availability is the share of recorded health checks that found a workload ready; it is not a measure of user-facing uptime. Planned stops are left out.".to_string(),
        "Only changes made through Aether are recorded.".to_string(),
    ];
    let unobserved: Vec<_> = workloads
        .iter()
        .filter(|w| w.checks == 0)
        .map(|w| w.workload.as_str())
        .collect();
    if !unobserved.is_empty() {
        caveats.push(format!(
            "No health data in this window for: {}.",
            unobserved.join(", ")
        ));
    }
    if (now - since).num_days() > 31 {
        caveats.push("Hourly availability counters are kept for 31 days.".to_string());
    }
    if feed.truncated {
        caveats.push(format!(
            "Only the newest {} of {} changes are listed.",
            feed.changes.len(),
            feed.total
        ));
    }
    for u in &feed.unavailable {
        caveats.push(format!("Could not read {u}."));
    }

    Ok(Report {
        generated_at: now.to_rfc3339(),
        since: since.to_rfc3339(),
        recovery: incidents::recovery(overlapping.iter().copied()),
        workloads,
        incidents: incident_rows,
        changes: feed,
        attention: attention::build(dir, state)?.items,
        audit: AuditCheck {
            events_in_window: audit
                .events()
                .iter()
                .filter(|e| in_window(&e.timestamp))
                .count(),
            events_total: audit.events().len(),
            failed_integrity: audit
                .events()
                .iter()
                .filter(|e| !AuditLog::verify_event_integrity(e))
                .count(),
        },
        caveats,
    })
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn pct(p: Option<f64>) -> String {
    p.map_or_else(|| "no data".to_string(), |p| format!("{p:.2}%"))
}

/// A table or list of a report, independent of the output format.
struct Section {
    title: &'static str,
    /// Empty for a plain list.
    columns: Vec<&'static str>,
    rows: Vec<Vec<String>>,
    empty: &'static str,
}

fn summary_lines(r: &Report) -> Vec<String> {
    let open = r.incidents.iter().filter(|i| i.status == "open").count();
    vec![
        format!("Workloads managed: {}", r.workloads.len()),
        format!(
            "Incidents in the window: {} ({open} open)",
            r.incidents.len()
        ),
        r.recovery.mean_minutes.map_or_else(
            || "Mean time to recovery: no recovered incident".to_string(),
            |m| {
                format!(
                    "Mean time to recovery: {m:.1} minutes over {} recovered incident(s)",
                    r.recovery.recovered
                )
            },
        ),
        format!("Changes recorded: {}", r.changes.total),
        format!(
            "Audit trail: {} event(s) in the window, {} of {} fail the integrity check",
            r.audit.events_in_window, r.audit.failed_integrity, r.audit.events_total
        ),
    ]
}

fn sections(r: &Report) -> Vec<Section> {
    vec![
        Section {
            title: "Availability",
            columns: vec![
                "Workload",
                "Runtime",
                "Ready",
                "Checks",
                "Incidents",
                "Incident time",
                "Deploys",
                "Failed operations",
            ],
            rows: r
                .workloads
                .iter()
                .map(|w| {
                    vec![
                        w.workload.clone(),
                        w.runtime.to_string(),
                        pct(w.ready_pct),
                        w.checks.to_string(),
                        w.incidents.to_string(),
                        ops::format_duration(w.incident_secs),
                        w.deploys.to_string(),
                        w.failed_operations.to_string(),
                    ]
                })
                .collect(),
            empty: "No workloads are managed.",
        },
        Section {
            title: "Incidents",
            columns: vec![
                "ID", "Workload", "Started", "Resolved", "Status", "Duration",
            ],
            rows: r
                .incidents
                .iter()
                .map(|i| {
                    vec![
                        i.id.to_string(),
                        i.workload.clone(),
                        i.opened_at.clone(),
                        i.resolved_at.clone().unwrap_or_else(|| "-".to_string()),
                        i.status.clone(),
                        ops::format_duration(i.duration_secs),
                    ]
                })
                .collect(),
            empty: "No incident was recorded.",
        },
        Section {
            title: "Needs attention now",
            columns: vec!["Level", "Workload", "Item", "Detail"],
            rows: r
                .attention
                .iter()
                .map(|a| {
                    vec![
                        format!("{:?}", a.level).to_lowercase(),
                        a.workload.clone().unwrap_or_else(|| "-".to_string()),
                        a.title.clone(),
                        a.detail.clone(),
                    ]
                })
                .collect(),
            empty: "Nothing, based on recorded data.",
        },
        Section {
            title: "Changes",
            columns: vec!["Time", "Workload", "What", "Detail", "By"],
            rows: r
                .changes
                .changes
                .iter()
                .map(|c| {
                    vec![
                        c.at.clone(),
                        c.workload.clone().unwrap_or_else(|| "-".to_string()),
                        c.title.clone(),
                        c.detail.clone(),
                        c.actor.clone().unwrap_or_else(|| "-".to_string()),
                    ]
                })
                .collect(),
            empty: "No change was recorded.",
        },
    ]
}

pub fn markdown(r: &Report) -> String {
    let mut out = format!(
        "# Aether operations report\n\n{} to {}\n\n",
        r.since, r.generated_at
    );
    for line in summary_lines(r) {
        out.push_str(&format!("- {line}\n"));
    }
    for s in sections(r) {
        out.push_str(&format!("\n## {}\n\n", s.title));
        if s.rows.is_empty() {
            out.push_str(&format!("{}\n", s.empty));
            continue;
        }
        out.push_str(&format!("| {} |\n", s.columns.join(" | ")));
        out.push_str(&format!("|{}\n", " --- |".repeat(s.columns.len())));
        for row in s.rows {
            let cells: Vec<_> = row.iter().map(|c| md_escape(c)).collect();
            out.push_str(&format!("| {} |\n", cells.join(" | ")));
        }
    }
    out.push_str("\n## How to read this\n\n");
    for c in &r.caveats {
        out.push_str(&format!("- {}\n", md_escape(c)));
    }
    out
}

/// A self-contained HTML document with no scripts or external requests.
pub fn html(r: &Report) -> String {
    let mut out = String::from(
        "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\">\n<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<meta http-equiv=\"Content-Security-Policy\" content=\"default-src 'none'; style-src 'unsafe-inline'\">\n<title>Aether operations report</title>\n<style>body{font:14px/1.5 system-ui,sans-serif;margin:2rem;max-width:70rem}table{border-collapse:collapse;width:100%;display:block;overflow-x:auto}th,td{border:1px solid #ccc;padding:.3rem .6rem;text-align:left;vertical-align:top}@media (prefers-color-scheme:dark){body{background:#111;color:#eee}th,td{border-color:#444}}</style>\n</head><body>\n",
    );
    out.push_str(&format!(
        "<h1>Aether operations report</h1>\n<p>{} to {}</p>\n<ul>\n",
        html_escape(&r.since),
        html_escape(&r.generated_at)
    ));
    for line in summary_lines(r) {
        out.push_str(&format!("<li>{}</li>\n", html_escape(&line)));
    }
    out.push_str("</ul>\n");
    for s in sections(r) {
        out.push_str(&format!("<h2>{}</h2>\n", s.title));
        if s.rows.is_empty() {
            out.push_str(&format!("<p>{}</p>\n", s.empty));
            continue;
        }
        out.push_str("<table><thead><tr>");
        for c in &s.columns {
            out.push_str(&format!("<th>{c}</th>"));
        }
        out.push_str("</tr></thead><tbody>\n");
        for row in s.rows {
            out.push_str("<tr>");
            for cell in row {
                out.push_str(&format!("<td>{}</td>", html_escape(&cell)));
            }
            out.push_str("</tr>\n");
        }
        out.push_str("</tbody></table>\n");
    }
    out.push_str("<h2>How to read this</h2>\n<ul>\n");
    for c in &r.caveats {
        out.push_str(&format!("<li>{}</li>\n", html_escape(c)));
    }
    out.push_str("</ul>\n</body></html>\n");
    out
}

/// Render in `markdown` (or `md`), `html` or `json`.
pub fn render(r: &Report, format: &str) -> Result<String> {
    match format {
        "markdown" | "md" => Ok(markdown(r)),
        "html" => Ok(html(r)),
        "json" => Ok(serde_json::to_string_pretty(r)?),
        other => anyhow::bail!("unknown report format '{other}': use markdown, html or json"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::revisions::tests::instance;
    use crate::state::WorkloadState;

    fn fleet() -> StateStore {
        let mut state = StateStore::new();
        state.upsert(
            "web".into(),
            WorkloadState::new(
                "web".into(),
                RuntimeKind::Podman,
                instance("web", RuntimeKind::Podman),
                "gone.yaml".into(),
            ),
        );
        state
    }

    #[test]
    fn reports_missing_data_as_missing() {
        let dir = tempfile::tempdir().unwrap();
        let r = build(dir.path(), &fleet(), Utc::now() - chrono::Duration::days(1)).unwrap();
        assert_eq!(r.workloads[0].ready_pct, None);
        let md = markdown(&r);
        assert!(md.contains("no data"), "{md}");
        assert!(md.contains("No incident was recorded"));
        assert!(md.contains("No health data in this window for: web"));
        assert!(md.contains("no recovered incident"));
    }

    #[test]
    fn counts_checks_incidents_and_escapes_text() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let now = Utc::now().to_rfc3339();
        incidents::record(d, &incidents::tests::obs("web", true, &now)).unwrap();
        for _ in 0..2 {
            incidents::record(d, &incidents::tests::obs("web", false, &now)).unwrap();
        }
        journal::audit(
            d,
            crate::audit::AuditAction::Stop,
            "web",
            None,
            ActionResult::Failure,
            "<script>alert(1)</script> | pipe",
        )
        .unwrap();
        let r = build(d, &fleet(), Utc::now() - chrono::Duration::days(1)).unwrap();
        let w = &r.workloads[0];
        assert_eq!((w.checks, w.ready_checks, w.incidents), (3, 1, 1));
        assert_eq!(w.failed_operations, 1);
        assert!(!html(&r).contains("<script>"));
        assert!(markdown(&r).contains("\\<script\\>alert(1)"));
        assert!(render(&r, "pdf").is_err());
        serde_json::from_str::<serde_json::Value>(&render(&r, "json").unwrap()).unwrap();
    }
}
