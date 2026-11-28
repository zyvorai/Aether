// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Health observations, incidents and availability.
//!
//! Every status Aether reads from a runtime is an [`Observation`]. Consecutive
//! not-ready observations open an incident; the next ready one resolves it.
//! Downtime the operator asked for (a stop or delete made through Aether) is
//! recorded but neither opens an incident nor counts against availability.
//!
//! Times are observation times: an outage shorter than the observation
//! interval can be missed, and start and end are accurate to one interval.

use crate::events::{EventCategory, EventSeverity};
use crate::health::{HealthHistory, HealthRecord};
use crate::ops::{self, changes, journal, store};
use crate::runtime::{InstanceState, RuntimeKind};
use crate::state::WorkloadState;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Consecutive not-ready observations that open an incident, unless
/// `AETHER_INCIDENT_THRESHOLD` says otherwise.
const DEFAULT_THRESHOLD: u32 = 2;
/// Raw health records kept per workload.
const RECORDS_PER_WORKLOAD: usize = 1_440;
const MAX_INCIDENTS: usize = 1_000;
const MAX_NOTES: usize = 200;
const MAX_NOTE_CHARS: usize = 2_000;
/// Days of hourly availability counters kept.
const AVAILABILITY_DAYS: i64 = 31;
/// A ready observation this soon after a stop is the runtime still winding
/// down, not somebody starting the workload again.
pub const STOP_GRACE_SECS: i64 = 120;
/// How far before an incident a change is listed as a possible cause.
pub const SUSPECT_WINDOW_SECS: i64 = 1_800;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub workload: String,
    pub runtime: RuntimeKind,
    pub state: InstanceState,
    pub ready: bool,
    pub restart_count: u32,
    pub message: Option<String>,
    pub at: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    /// A later observation found the workload ready.
    Recovered,
    /// The operator stopped it while the incident was open.
    Stopped,
    /// The operator deleted it while the incident was open.
    Deleted,
}

impl std::fmt::Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Resolution::Recovered => "recovered",
            Resolution::Stopped => "stopped",
            Resolution::Deleted => "deleted",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Note {
    pub at: String,
    pub actor: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Acknowledgement {
    pub at: String,
    pub actor: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Incident {
    pub id: u64,
    pub workload: String,
    pub runtime: RuntimeKind,
    /// First failed observation.
    pub opened_at: String,
    /// Observation that crossed the threshold.
    pub detected_at: String,
    pub last_failed_at: String,
    #[serde(default)]
    pub resolved_at: Option<String>,
    #[serde(default)]
    pub resolution: Option<Resolution>,
    pub failed_checks: u32,
    pub last_state: InstanceState,
    #[serde(default)]
    pub last_message: Option<String>,
    /// Restarts reported by the runtime since the incident opened.
    #[serde(default)]
    pub restarts: u32,
    restarts_at_open: u32,
    #[serde(default)]
    pub acknowledged: Option<Acknowledgement>,
    #[serde(default)]
    pub notes: Vec<Note>,
}

impl Incident {
    pub fn is_open(&self) -> bool {
        self.resolved_at.is_none()
    }

    /// Seconds from the first failed observation to resolution (or `now`).
    pub fn duration_secs(&self, now: chrono::DateTime<chrono::Utc>) -> i64 {
        let Some(start) = ops::parse_time(&self.opened_at) else {
            return 0;
        };
        let end = self
            .resolved_at
            .as_deref()
            .and_then(ops::parse_time)
            .unwrap_or(now);
        (end - start).num_seconds().max(0)
    }
}

/// Run of not-ready observations that has not yet opened an incident.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Streak {
    failures: u32,
    first_failed_at: String,
    restarts_at_start: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpectedDown {
    pub runtime: RuntimeKind,
    pub since: String,
    pub why: String,
}

/// A workload whose status could not be read.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObserveError {
    pub at: String,
    pub message: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct IncidentStore {
    next_id: u64,
    pub incidents: Vec<Incident>,
    streaks: BTreeMap<String, Streak>,
    pub expected_down: BTreeMap<String, ExpectedDown>,
    pub observe_errors: BTreeMap<String, ObserveError>,
}

/// What an observation changed.
#[derive(Debug, Clone)]
pub enum Transition {
    None,
    /// Counted as downtime the operator asked for.
    Planned,
    Opened(Incident),
    Resolved(Incident),
}

impl IncidentStore {
    fn open_index(&self, workload: &str) -> Option<usize> {
        self.incidents
            .iter()
            .rposition(|i| i.workload == workload && i.is_open())
    }

    /// Fold one observation into the store.
    pub fn apply(&mut self, o: &Observation, threshold: u32) -> Transition {
        self.observe_errors.remove(&o.workload);
        if let Some(down) = self.expected_down.get(&o.workload) {
            if down.runtime == o.runtime {
                if !o.ready {
                    return Transition::Planned;
                }
                let settled = match (ops::parse_time(&down.since), ops::parse_time(&o.at)) {
                    (Some(since), Some(at)) => (at - since).num_seconds() >= STOP_GRACE_SECS,
                    _ => true,
                };
                if !settled {
                    return Transition::Planned;
                }
                // Somebody started it again.
                self.expected_down.remove(&o.workload);
            }
        }
        if o.ready {
            self.streaks.remove(&o.workload);
            return match self.open_index(&o.workload) {
                Some(i) => {
                    let inc = &mut self.incidents[i];
                    inc.resolved_at = Some(o.at.clone());
                    inc.resolution = Some(Resolution::Recovered);
                    inc.restarts = o.restart_count.saturating_sub(inc.restarts_at_open);
                    Transition::Resolved(inc.clone())
                }
                None => Transition::None,
            };
        }
        if let Some(i) = self.open_index(&o.workload) {
            let inc = &mut self.incidents[i];
            inc.failed_checks += 1;
            inc.last_failed_at = o.at.clone();
            inc.last_state = o.state.clone();
            inc.last_message = o.message.clone();
            inc.runtime = o.runtime;
            inc.restarts = o.restart_count.saturating_sub(inc.restarts_at_open);
            return Transition::None;
        }
        let streak = self
            .streaks
            .entry(o.workload.clone())
            .or_insert_with(|| Streak {
                failures: 0,
                first_failed_at: o.at.clone(),
                restarts_at_start: o.restart_count,
            });
        streak.failures += 1;
        if streak.failures < threshold.max(1) {
            return Transition::None;
        }
        let streak = self.streaks.remove(&o.workload).expect("streak just seen");
        self.next_id += 1;
        let incident = Incident {
            id: self.next_id,
            workload: o.workload.clone(),
            runtime: o.runtime,
            opened_at: streak.first_failed_at,
            detected_at: o.at.clone(),
            last_failed_at: o.at.clone(),
            resolved_at: None,
            resolution: None,
            failed_checks: streak.failures,
            last_state: o.state.clone(),
            last_message: o.message.clone(),
            restarts: o.restart_count.saturating_sub(streak.restarts_at_start),
            restarts_at_open: streak.restarts_at_start,
            acknowledged: None,
            notes: Vec::new(),
        };
        self.incidents.push(incident.clone());
        self.trim();
        Transition::Opened(incident)
    }

    /// Drop the oldest resolved incidents beyond the cap.
    fn trim(&mut self) {
        while self.incidents.len() > MAX_INCIDENTS {
            let Some(i) = self.incidents.iter().position(|i| !i.is_open()) else {
                break;
            };
            self.incidents.remove(i);
        }
    }

    fn expect_down(&mut self, workload: &str, runtime: RuntimeKind, why: &str, now: &str) {
        self.streaks.remove(workload);
        self.expected_down.insert(
            workload.to_string(),
            ExpectedDown {
                runtime,
                since: now.to_string(),
                why: why.to_string(),
            },
        );
        if let Some(i) = self.open_index(workload) {
            let inc = &mut self.incidents[i];
            if inc.runtime == runtime {
                inc.resolved_at = Some(now.to_string());
                inc.resolution = Some(if why == "deleted" {
                    Resolution::Deleted
                } else {
                    Resolution::Stopped
                });
            }
        }
        if why == "deleted" {
            self.observe_errors.remove(workload);
        }
    }

    pub fn get(&self, id: u64) -> Option<&Incident> {
        self.incidents.iter().find(|i| i.id == id)
    }
}

/// Hourly ready/total counters, so availability can be reported for longer
/// than the raw records are kept.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Availability {
    /// workload -> hour (`YYYY-MM-DDTHH`, UTC) -> counters
    pub hours: BTreeMap<String, BTreeMap<String, Counter>>,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Counter {
    pub checks: u32,
    pub ready: u32,
}

/// The key of the hourly counter containing `t`.
pub fn hour_key(t: chrono::DateTime<chrono::Utc>) -> String {
    t.format("%Y-%m-%dT%H").to_string()
}

impl Availability {
    fn add(&mut self, workload: &str, at: chrono::DateTime<chrono::Utc>, ready: bool) {
        let hours = self.hours.entry(workload.to_string()).or_default();
        let c = hours.entry(hour_key(at)).or_default();
        c.checks += 1;
        c.ready += u32::from(ready);
        let oldest = hour_key(at - chrono::Duration::days(AVAILABILITY_DAYS));
        hours.retain(|k, _| *k >= oldest);
    }

    /// Totals for `workload` from the hour containing `since`.
    pub fn since(&self, workload: &str, since: chrono::DateTime<chrono::Utc>) -> Counter {
        let from = hour_key(since);
        self.hours
            .get(workload)
            .into_iter()
            .flat_map(|h| h.range(from.clone()..))
            .fold(Counter::default(), |a, (_, c)| Counter {
                checks: a.checks + c.checks,
                ready: a.ready + c.ready,
            })
    }
}

pub fn path(dir: &Path) -> PathBuf {
    dir.join("incidents.json")
}

pub fn availability_path(dir: &Path) -> PathBuf {
    dir.join("availability.json")
}

pub fn health_path(dir: &Path) -> PathBuf {
    dir.join("health.json")
}

pub fn load(dir: &Path) -> Result<IncidentStore> {
    store::load(&path(dir))
}

pub fn threshold() -> u32 {
    std::env::var("AETHER_INCIDENT_THRESHOLD")
        .ok()
        .and_then(|v| v.parse().ok())
        .filter(|v| (1..=100).contains(v))
        .unwrap_or(DEFAULT_THRESHOLD)
}

/// The workload was just stopped or deleted through Aether.
pub fn expect_down(dir: &Path, workload: &str, runtime: RuntimeKind, why: &str) -> Result<()> {
    let now = crate::resources::now_rfc3339();
    store::update(&path(dir), |s: &mut IncidentStore| {
        s.expect_down(workload, runtime, why, &now);
    })
}

/// The workload was just deployed: downtime is no longer expected.
pub fn expect_up(dir: &Path, workload: &str) -> Result<()> {
    store::update(&path(dir), |s: &mut IncidentStore| {
        s.expected_down.remove(workload);
        s.streaks.remove(workload);
    })
}

/// Record that a workload's status could not be read. This is not downtime.
pub fn record_error(dir: &Path, workload: &str, message: &str) -> Result<()> {
    let error = ObserveError {
        at: crate::resources::now_rfc3339(),
        message: message.to_string(),
    };
    store::update(&path(dir), |s: &mut IncidentStore| {
        s.observe_errors.insert(workload.to_string(), error);
    })
}

/// Add an observation to the health history.
fn append_history(dir: &Path, o: &Observation) -> Result<()> {
    store::update(&health_path(dir), |h: &mut HealthHistory| {
        h.records.push(HealthRecord {
            timestamp: o.at.clone(),
            workload: o.workload.clone(),
            runtime: o.runtime,
            state: o.state.clone(),
            ready: o.ready,
            restart_count: o.restart_count,
            latency_ms: None,
        });
        let mine = h
            .records
            .iter()
            .filter(|r| r.workload == o.workload)
            .count();
        let mut excess = mine.saturating_sub(RECORDS_PER_WORKLOAD);
        h.records.retain(|r| {
            let drop = excess > 0 && r.workload == o.workload;
            excess -= usize::from(drop);
            !drop
        });
        // Retention is per workload here; keep `HealthHistory::record`, which
        // trims globally, from discarding what was just kept.
        h.max_records = h.max_records.max(h.records.len() + RECORDS_PER_WORKLOAD);
    })?;
    Ok(())
}

/// Look at a workload once, right after it was deployed or started, so that its
/// status does not read `unknown` until the next observation round. Only the
/// health history is written: one early look does not count toward incidents or
/// availability.
pub async fn first_look(dir: &Path, ws: &WorkloadState) {
    const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
    let status = tokio::time::timeout(TIMEOUT, async {
        let rt = crate::runtime::create_runtime_ns(&ws.runtime, ws.namespace.as_deref()).await?;
        rt.status(&ws.instance).await
    })
    .await;
    // No answer leaves the status unknown; the observation loop reports why.
    let Ok(Ok(status)) = status else { return };
    let o = Observation {
        workload: ws.name.clone(),
        runtime: ws.runtime,
        state: status.state,
        ready: status.ready,
        restart_count: status.restart_count,
        message: status.message,
        at: crate::resources::now_rfc3339(),
    };
    if let Err(e) = append_history(dir, &o) {
        tracing::warn!("could not record the status of {}: {e:#}", ws.name);
    }
}

/// Store an observation and update incidents and availability.
pub fn record(dir: &Path, o: &Observation) -> Result<Transition> {
    append_history(dir, o)?;
    let threshold = threshold();
    let transition = store::update(&path(dir), |s: &mut IncidentStore| s.apply(o, threshold))?;
    if !matches!(transition, Transition::Planned) {
        if let Some(at) = ops::parse_time(&o.at) {
            store::update(&availability_path(dir), |a: &mut Availability| {
                a.add(&o.workload, at, o.ready);
            })?;
        }
    }
    match &transition {
        Transition::Opened(i) => journal::event(
            dir,
            EventSeverity::Critical,
            EventCategory::HealthCheck,
            Some(&i.workload),
            "Incident opened",
            &format!(
                "incident {}: {} is not ready ({}) after {} consecutive checks",
                i.id, i.workload, i.last_state, i.failed_checks
            ),
        )?,
        Transition::Resolved(i) => journal::event(
            dir,
            EventSeverity::Warning,
            EventCategory::HealthCheck,
            Some(&i.workload),
            "Incident resolved",
            &format!(
                "incident {}: {} recovered after {}",
                i.id,
                i.workload,
                ops::format_duration(i.duration_secs(chrono::Utc::now()))
            ),
        )?,
        Transition::None | Transition::Planned => {}
    }
    Ok(transition)
}

/// Result of one observation round.
#[derive(Debug, Clone, Default, Serialize)]
pub struct CollectReport {
    pub observed: usize,
    pub opened: Vec<u64>,
    pub resolved: Vec<u64>,
    /// `workload: why its status could not be read`.
    pub errors: Vec<String>,
}

/// Ask each workload's runtime for its status and record the answer.
pub async fn collect(
    dir: &Path,
    workloads: &[WorkloadState],
    namespace: Option<&str>,
) -> Result<CollectReport> {
    const STATUS_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
    let mut report = CollectReport::default();
    for ws in workloads {
        let ns = namespace.or(ws.namespace.as_deref());
        let status = tokio::time::timeout(STATUS_TIMEOUT, async {
            let rt = crate::runtime::create_runtime_ns(&ws.runtime, ns).await?;
            rt.status(&ws.instance).await
        })
        .await
        .unwrap_or_else(|_| Err(anyhow::anyhow!("status query timed out")));
        match status {
            Ok(status) => {
                let transition = record(
                    dir,
                    &Observation {
                        workload: ws.name.clone(),
                        runtime: ws.runtime,
                        state: status.state,
                        ready: status.ready,
                        restart_count: status.restart_count,
                        message: status.message,
                        at: crate::resources::now_rfc3339(),
                    },
                )?;
                report.observed += 1;
                match transition {
                    Transition::Opened(i) => report.opened.push(i.id),
                    Transition::Resolved(i) => report.resolved.push(i.id),
                    Transition::None | Transition::Planned => {}
                }
            }
            Err(e) => {
                let message = format!("{e:#}");
                record_error(dir, &ws.name, &message)?;
                report.errors.push(format!("{}: {message}", ws.name));
            }
        }
    }
    Ok(report)
}

pub fn acknowledge(dir: &Path, id: u64) -> Result<Incident> {
    let ack = Acknowledgement {
        at: crate::resources::now_rfc3339(),
        actor: ops::current_actor(),
    };
    store::update(&path(dir), |s: &mut IncidentStore| {
        s.incidents.iter_mut().find(|i| i.id == id).map(|i| {
            i.acknowledged.get_or_insert(ack);
            i.clone()
        })
    })?
    .ok_or_else(|| anyhow::anyhow!("incident {id} not found"))
}

pub fn add_note(dir: &Path, id: u64, text: &str) -> Result<Incident> {
    let text = crate::speclint::sanitize(text.trim());
    if text.is_empty() {
        anyhow::bail!("a note cannot be empty");
    }
    if text.chars().count() > MAX_NOTE_CHARS {
        anyhow::bail!("a note is limited to {MAX_NOTE_CHARS} characters");
    }
    let note = Note {
        at: crate::resources::now_rfc3339(),
        actor: ops::current_actor(),
        text,
    };
    store::update(&path(dir), |s: &mut IncidentStore| {
        match s.incidents.iter_mut().find(|i| i.id == id) {
            None => Err(anyhow::anyhow!("incident {id} not found")),
            Some(i) if i.notes.len() >= MAX_NOTES => Err(anyhow::anyhow!(
                "incident {id} already has {MAX_NOTES} notes"
            )),
            Some(i) => {
                i.notes.push(note);
                Ok(i.clone())
            }
        }
    })?
}

/// Mean time to recovery over incidents that ended with a ready observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recovery {
    pub incidents: u32,
    pub recovered: u32,
    /// `None` when no incident has recovered yet.
    pub mean_minutes: Option<f64>,
    pub last_opened_at: Option<String>,
}

pub fn recovery<'a>(incidents: impl IntoIterator<Item = &'a Incident>) -> Recovery {
    let now = chrono::Utc::now();
    let (mut total, mut recovered, mut secs) = (0u32, 0u32, 0i64);
    let mut last: Option<&str> = None;
    for i in incidents {
        total += 1;
        if i.resolution == Some(Resolution::Recovered) {
            recovered += 1;
            secs += i.duration_secs(now);
        }
        if last.is_none_or(|l| ops::parse_time(l) < ops::parse_time(&i.opened_at)) {
            last = Some(&i.opened_at);
        }
    }
    Recovery {
        incidents: total,
        recovered,
        mean_minutes: (recovered > 0).then(|| secs as f64 / 60.0 / f64::from(recovered)),
        last_opened_at: last.map(str::to_string),
    }
}

/// Something that happened shortly before an incident. A correlation in time,
/// not a proven cause.
#[derive(Debug, Clone, Serialize)]
pub struct Suspect {
    pub change: changes::Change,
    pub seconds_before: i64,
    /// `this workload` or `dependency`.
    pub relation: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct IncidentDetail {
    #[serde(flatten)]
    pub incident: Incident,
    pub duration_secs: i64,
    /// Newest first within the suspect window; own changes before
    /// dependencies'.
    pub suspects: Vec<Suspect>,
    /// Oldest first, from the suspect window to resolution.
    pub timeline: Vec<changes::Change>,
    pub window_secs: i64,
}

/// An incident with the changes around it.
pub fn detail(dir: &Path, id: u64) -> Result<IncidentDetail> {
    let all = load(dir)?;
    let incident = all
        .get(id)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("incident {id} not found"))?;
    let now = chrono::Utc::now();
    let opened = ops::parse_time(&incident.opened_at)
        .ok_or_else(|| anyhow::anyhow!("incident {id} has an unreadable start time"))?;
    let start = opened - chrono::Duration::seconds(SUSPECT_WINDOW_SECS);
    let end = incident
        .resolved_at
        .as_deref()
        .and_then(ops::parse_time)
        .map_or(now, |t| t + chrono::Duration::seconds(60));
    let dependencies = crate::dependencies::DependencyGraph::load(&dir.join("dependencies.json"))?
        .dependencies_of(&incident.workload);

    let mut involved = vec![incident.workload.clone()];
    involved.extend(dependencies.iter().cloned());
    let around = changes::collect(
        dir,
        &changes::Query {
            since: start,
            until: end,
            workloads: Some(involved),
            limit: 500,
        },
    )?;

    let mut suspects: Vec<Suspect> = around
        .changes
        .iter()
        .filter(|c| c.is_possible_cause() && c.incident != Some(incident.id))
        .filter_map(|c| {
            let at = ops::parse_time(&c.at)?;
            // A deploy recorded in the same second as the first failure still
            // counts as "before".
            let before = (opened - at).num_seconds();
            if before < 0 {
                return None;
            }
            let own = c.workload.as_deref() == Some(incident.workload.as_str());
            Some(Suspect {
                change: c.clone(),
                seconds_before: before,
                relation: if own { "this workload" } else { "dependency" }.to_string(),
            })
        })
        .collect();
    suspects.sort_by_key(|s| (s.relation != "this workload", s.seconds_before));

    let mut timeline: Vec<_> = around
        .changes
        .into_iter()
        .filter(|c| {
            c.workload.as_deref() == Some(incident.workload.as_str()) || c.is_possible_cause()
        })
        .collect();
    timeline.reverse();

    Ok(IncidentDetail {
        duration_secs: incident.duration_secs(now),
        incident,
        suspects,
        timeline,
        window_secs: SUSPECT_WINDOW_SECS,
    })
}

/// Make recorded text inert inside Markdown: one line, no markup, no tables.
pub fn md_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in crate::speclint::sanitize(s).chars() {
        match c {
            '\n' | '\r' | '\t' => out.push(' '),
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '|' | '#' => {
                out.push('\\');
                out.push(c);
            }
            c => out.push(c),
        }
    }
    out
}

/// A post-incident draft in Markdown, containing only recorded facts and
/// leaving the analysis to the operator.
pub fn markdown(d: &IncidentDetail) -> String {
    use md_escape as e;
    let i = &d.incident;
    let mut out = format!("# Incident {}: {}\n\n", i.id, e(&i.workload));
    let status = match (&i.resolved_at, i.resolution) {
        (Some(at), Some(r)) => format!("resolved ({r}) at {at}"),
        _ => "open".to_string(),
    };
    out.push_str(&format!(
        "- Status: {status}\n- Runtime: {}\n- First failed check: {}\n- Detected: {}\n- Duration: {}\n- Failed checks: {}\n- Last observed state: {}{}\n- Restarts during incident: {}\n",
        i.runtime,
        i.opened_at,
        i.detected_at,
        ops::format_duration(d.duration_secs),
        i.failed_checks,
        i.last_state,
        i.last_message
            .as_deref()
            .map(|m| format!(" ({})", e(m)))
            .unwrap_or_default(),
        i.restarts
    ));
    if let Some(a) = &i.acknowledged {
        out.push_str(&format!("- Acknowledged: {} by {}\n", a.at, e(&a.actor)));
    }
    out.push_str(&format!(
        "\n## Changes in the {} before it started\n\nCorrelation in time only; none of these is a confirmed cause.\n\n",
        ops::format_duration(d.window_secs)
    ));
    if d.suspects.is_empty() {
        out.push_str("No change was recorded in that window.\n");
    }
    for s in &d.suspects {
        out.push_str(&format!(
            "- {} before ({}): {} - {}\n",
            ops::format_duration(s.seconds_before),
            s.relation,
            e(&s.change.title),
            e(&s.change.detail)
        ));
    }
    out.push_str("\n## Timeline\n\n");
    for c in &d.timeline {
        out.push_str(&format!(
            "- {} {}: {} - {}\n",
            c.at,
            e(c.workload.as_deref().unwrap_or("-")),
            e(&c.title),
            e(&c.detail)
        ));
    }
    out.push_str("\n## Notes\n\n");
    if i.notes.is_empty() {
        out.push_str("None recorded.\n");
    }
    for n in &i.notes {
        out.push_str(&format!("- {} {}: {}\n", n.at, e(&n.actor), e(&n.text)));
    }
    out.push_str("\n## Analysis\n\n_To be written: cause, impact, follow-up actions._\n");
    out
}

pub fn format_list(incidents: &[&Incident]) -> String {
    if incidents.is_empty() {
        return "No incidents recorded.\n".to_string();
    }
    let now = chrono::Utc::now();
    let mut out = format!(
        "{:<5} {:<24} {:<11} {:<19} {:<10} {}\n",
        "ID", "WORKLOAD", "STATUS", "STARTED (UTC)", "DURATION", "ACK"
    );
    for i in incidents {
        out.push_str(&format!(
            "{:<5} {:<24} {:<11} {:<19} {:<10} {}\n",
            i.id,
            i.workload,
            i.resolution
                .map_or_else(|| "OPEN".to_string(), |r| r.to_string()),
            i.opened_at.get(..19).unwrap_or(&i.opened_at),
            ops::format_duration(i.duration_secs(now)),
            i.acknowledged.as_ref().map_or("-", |a| a.actor.as_str())
        ));
    }
    out
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn obs(workload: &str, ready: bool, at: &str) -> Observation {
        Observation {
            workload: workload.into(),
            runtime: RuntimeKind::Podman,
            state: if ready {
                InstanceState::Running
            } else {
                InstanceState::Failed
            },
            ready,
            restart_count: 0,
            message: None,
            at: at.into(),
        }
    }

    fn at(min: u32) -> String {
        format!("2026-03-01T10:{min:02}:00+00:00")
    }

    #[test]
    fn opens_after_threshold_and_resolves_on_recovery() {
        let mut s = IncidentStore::default();
        assert!(matches!(
            s.apply(&obs("web", false, &at(0)), 2),
            Transition::None
        ));
        let Transition::Opened(i) = s.apply(&obs("web", false, &at(1)), 2) else {
            panic!("expected an incident");
        };
        assert_eq!(i.opened_at, at(0));
        assert_eq!(i.detected_at, at(1));
        assert!(matches!(
            s.apply(&obs("web", false, &at(2)), 2),
            Transition::None
        ));
        let Transition::Resolved(i) = s.apply(&obs("web", true, &at(5)), 2) else {
            panic!("expected resolution");
        };
        assert_eq!(i.failed_checks, 3);
        assert_eq!(i.duration_secs(chrono::Utc::now()), 300);
        let r = recovery(&s.incidents);
        assert_eq!((r.incidents, r.recovered), (1, 1));
        assert_eq!(r.mean_minutes, Some(5.0));
    }

    #[test]
    fn a_single_blip_is_not_an_incident() {
        let mut s = IncidentStore::default();
        s.apply(&obs("web", false, &at(0)), 2);
        s.apply(&obs("web", true, &at(1)), 2);
        s.apply(&obs("web", false, &at(2)), 2);
        assert!(s.incidents.is_empty());
        assert_eq!(recovery(&s.incidents).mean_minutes, None);
    }

    #[test]
    fn planned_downtime_is_not_an_incident() {
        let mut s = IncidentStore::default();
        s.expect_down("web", RuntimeKind::Podman, "stopped", &at(0));
        for m in 1..6 {
            assert!(matches!(
                s.apply(&obs("web", false, &at(m)), 2),
                Transition::Planned
            ));
        }
        assert!(s.incidents.is_empty());
        // Started again outside Aether, well after the stop.
        s.apply(&obs("web", true, &at(10)), 2);
        assert!(s.expected_down.is_empty());
        s.apply(&obs("web", false, &at(11)), 1);
        assert_eq!(s.incidents.len(), 1);
    }

    #[test]
    fn winding_down_does_not_cancel_a_stop() {
        let mut s = IncidentStore::default();
        s.expect_down("web", RuntimeKind::Podman, "stopped", &at(0));
        assert!(matches!(
            s.apply(&obs("web", true, &at(1)), 1),
            Transition::Planned
        ));
        assert!(matches!(
            s.apply(&obs("web", false, &at(2)), 1),
            Transition::Planned
        ));
    }

    #[test]
    fn stopping_closes_an_open_incident_without_counting_as_recovery() {
        let mut s = IncidentStore::default();
        s.apply(&obs("web", false, &at(0)), 1);
        s.expect_down("web", RuntimeKind::Podman, "stopped", &at(3));
        assert_eq!(s.incidents[0].resolution, Some(Resolution::Stopped));
        assert_eq!(recovery(&s.incidents).mean_minutes, None);
    }

    #[test]
    fn removing_the_old_runtime_after_a_migration_keeps_watching_the_new_one() {
        let mut s = IncidentStore::default();
        s.expect_down("web", RuntimeKind::Podman, "deleted", &at(0));
        let mut o = obs("web", false, &at(1));
        o.runtime = RuntimeKind::Kubernetes;
        assert!(matches!(s.apply(&o, 1), Transition::Opened(_)));
    }

    #[test]
    fn record_updates_health_availability_events_and_notes() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let now = chrono::Utc::now();
        let t = |m: i64| (now - chrono::Duration::minutes(m)).to_rfc3339();
        record(d, &obs("web", true, &t(4))).unwrap();
        record(d, &obs("web", false, &t(3))).unwrap();
        let tr = record(d, &obs("web", false, &t(2))).unwrap();
        let Transition::Opened(i) = tr else {
            panic!("expected incident")
        };
        let counter = store::load::<Availability>(&availability_path(d))
            .unwrap()
            .since("web", now - chrono::Duration::hours(2));
        assert_eq!((counter.checks, counter.ready), (3, 1));
        assert_eq!(
            store::load::<HealthHistory>(&health_path(d))
                .unwrap()
                .records
                .len(),
            3
        );
        add_note(d, i.id, "looking").unwrap();
        assert!(add_note(d, i.id, " ").is_err());
        assert!(add_note(d, 999, "x").is_err());
        assert!(acknowledge(d, i.id).unwrap().acknowledged.is_some());
        let detail = detail(d, i.id).unwrap();
        let md = markdown(&detail);
        assert!(md.contains("# Incident 1: web"), "{md}");
        assert!(md.contains("looking"));
        assert!(md.contains("No change was recorded"));
    }

    #[test]
    fn raw_history_is_capped_per_workload() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let mut h = HealthHistory::default();
        for i in 0..RECORDS_PER_WORKLOAD {
            h.records.push(HealthRecord {
                timestamp: format!("{i}"),
                workload: "busy".into(),
                runtime: RuntimeKind::Podman,
                state: InstanceState::Running,
                ready: true,
                restart_count: 0,
                latency_ms: None,
            });
        }
        h.records.insert(
            0,
            HealthRecord {
                workload: "quiet".into(),
                ..h.records[0].clone()
            },
        );
        h.save(&health_path(d)).unwrap();
        record(d, &obs("busy", true, &at(0))).unwrap();
        let h: HealthHistory = store::load(&health_path(d)).unwrap();
        assert_eq!(h.timeline("busy", usize::MAX).len(), RECORDS_PER_WORKLOAD);
        assert_eq!(h.timeline("quiet", usize::MAX).len(), 1);
    }
}
