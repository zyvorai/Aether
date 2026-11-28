// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! The status board: what every managed workload was last observed doing, and
//! how it has done over a window.
//!
//! Everything comes from health checks, incidents, holds, revisions and the
//! dependency graph Aether recorded. A period without a check is reported as
//! having no data; it is never counted or drawn as healthy. A last check that
//! is too old no longer describes the present and is reported as stale.

use crate::audit::{AuditEvent, AuditLog};
use crate::health::{HealthHistory, HealthRecord};
use crate::ops::{self, holds, incidents, journal, planned, revisions, store};
use crate::state::{StateStore, WorkloadState};
use anyhow::Result;
use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

/// A health record older than this is stale when the interval is unknown.
const DEFAULT_STALE_SECS: i64 = 900;
/// The shortest age at which a check is called stale.
const MIN_STALE_SECS: i64 = 180;

/// How long after a check it stops describing the present, given how often the
/// server observes (`0`: observation is off and the interval is unknown).
pub fn stale_after_secs(observe_secs: u64) -> i64 {
    if observe_secs == 0 {
        return DEFAULT_STALE_SECS;
    }
    i64::try_from(observe_secs.saturating_mul(5))
        .unwrap_or(i64::MAX)
        .clamp(MIN_STALE_SECS, 7 * 86_400)
}

/// Where the cells of an uptime strip come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Individual checks (the last 1440 per workload are kept).
    Checks,
    /// Hourly counters (31 days are kept).
    Hourly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub key: &'static str,
    pub secs: i64,
    pub bucket_secs: i64,
    pub source: Source,
}

pub const WINDOWS: [Window; 5] = [
    Window {
        key: "1h",
        secs: 3_600,
        bucket_secs: 120,
        source: Source::Checks,
    },
    Window {
        key: "6h",
        secs: 21_600,
        bucket_secs: 600,
        source: Source::Checks,
    },
    Window {
        key: "24h",
        secs: 86_400,
        bucket_secs: 3_600,
        source: Source::Hourly,
    },
    Window {
        key: "7d",
        secs: 604_800,
        bucket_secs: 10_800,
        source: Source::Hourly,
    },
    Window {
        key: "30d",
        secs: 2_592_000,
        bucket_secs: 86_400,
        source: Source::Hourly,
    },
];

pub fn parse_window(key: &str) -> Result<Window> {
    let key = key.trim();
    WINDOWS
        .into_iter()
        .find(|w| w.key == key)
        .ok_or_else(|| anyhow::anyhow!("unknown window: use 1h, 6h, 24h, 7d or 30d"))
}

/// One cell of an uptime strip.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CellState {
    /// Every check found it ready.
    Up,
    /// Some checks found it ready and some did not.
    Partial,
    /// No check found it ready.
    Down,
    /// Stopped or deleted on purpose.
    Planned,
    /// No check was recorded.
    NoData,
}

#[derive(Debug, Clone, Serialize)]
pub struct Cell {
    pub start: String,
    pub state: CellState,
    /// Checks counted (planned downtime excluded).
    pub checks: u32,
    pub ready: u32,
    /// Share of the elapsed part of the cell that checks or planned downtime
    /// account for. Below 1 there are gaps.
    pub covered: f64,
}

/// What a workload was last seen doing.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TileState {
    Ready,
    NotReady,
    /// Stopped or deleted on purpose.
    Stopped,
    /// The last check is too old to describe the present.
    Stale,
    /// The runtime did not answer, so the status is not known.
    Unreadable,
    /// No check has been recorded.
    NoData,
    /// The records could not be read.
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastCheck {
    pub at: String,
    pub state: String,
    pub ready: bool,
    pub restart_count: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpenIncident {
    pub id: u64,
    pub opened_at: String,
    pub failed_checks: u32,
    pub acknowledged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LastChange {
    pub revision: u64,
    pub previous_revision: Option<u64>,
    pub at: String,
    pub actor: String,
    pub op: revisions::RevisionOp,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tile {
    pub name: String,
    pub runtime: String,
    pub image: String,
    pub state: TileState,
    pub last_check: Option<LastCheck>,
    /// When it was stopped on purpose, if it is stopped.
    pub stopped_since: Option<String>,
    /// Why the status could not be read.
    pub error: Option<String>,
    /// Oldest first.
    pub strip: Vec<Cell>,
    /// Share of counted checks in the window that found it ready; `None`
    /// without checks.
    pub ready_pct: Option<f64>,
    pub checks: u32,
    /// `None` when incidents could not be read.
    pub open_incidents: Option<Vec<OpenIncident>>,
    pub hold: Option<holds::Hold>,
    pub last_change: Option<LastChange>,
    pub depends_on: Vec<String>,
    pub dependents: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Board {
    pub generated_at: String,
    pub window: &'static str,
    pub bucket_secs: i64,
    /// Seconds between the server's checks; `0` when it does not check.
    pub observe_secs: u64,
    pub stale_after_secs: i64,
    pub incident_threshold: u32,
    pub workloads: Vec<Tile>,
    /// Records that could not be read. The board is incomplete when non-empty.
    pub unavailable: Vec<String>,
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

/// Start of the cell grid: `cells` aligned cells, the last containing `now`.
fn grid(window: Window, now: DateTime<Utc>) -> (DateTime<Utc>, i64) {
    let cells = window.secs / window.bucket_secs;
    let current = now.timestamp().div_euclid(window.bucket_secs);
    let start = (current - cells + 1) * window.bucket_secs;
    (Utc.timestamp_opt(start, 0).single().unwrap_or(now), cells)
}

#[derive(Default, Clone, Copy)]
struct Tally {
    checks: u32,
    ready: u32,
    planned: u32,
}

/// Decide a cell from its counts and the slots (equal parts of the cell) that
/// have checks or are accounted for by planned downtime.
fn cell(start: DateTime<Utc>, tally: Tally, observed: u32, planned: u32, elapsed: u32) -> Cell {
    let state = if tally.checks > 0 {
        if tally.ready == tally.checks {
            CellState::Up
        } else if tally.ready == 0 {
            CellState::Down
        } else {
            CellState::Partial
        }
    } else if tally.planned > 0 || (elapsed > 0 && planned >= elapsed) {
        CellState::Planned
    } else {
        CellState::NoData
    };
    let covered = if elapsed == 0 {
        0.0
    } else {
        f64::from((observed + planned).min(elapsed)) / f64::from(elapsed)
    };
    Cell {
        start: start.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        state,
        checks: tally.checks,
        ready: tally.ready,
        covered,
    }
}

/// Cells from individual checks.
fn strip_from_checks(
    window: Window,
    now: DateTime<Utc>,
    stale_after: i64,
    records: &[(DateTime<Utc>, &HealthRecord)],
    downtime: &[planned::Interval],
    runtime: &str,
) -> Vec<Cell> {
    let (start, cells) = grid(window, now);
    // A cell longer than the stale age is split so a partly observed cell shows
    // that it has gaps.
    let slots = (window.bucket_secs / stale_after.max(1)).max(1);
    let slot_secs = window.bucket_secs / slots;
    (0..cells)
        .map(|i| {
            let from = start + Duration::seconds(i * window.bucket_secs);
            let to = from + Duration::seconds(window.bucket_secs);
            let mut tally = Tally::default();
            let first = records.partition_point(|(at, _)| *at < from);
            let inside: Vec<_> = records[first..]
                .iter()
                .take_while(|(at, _)| *at < to)
                .filter(|(_, r)| r.runtime.to_string() == runtime)
                .collect();
            for (at, r) in &inside {
                if !r.ready && downtime.iter().any(|d| d.covers(*at, runtime)) {
                    tally.planned += 1;
                } else {
                    tally.checks += 1;
                    tally.ready += u32::from(r.ready);
                }
            }
            let (mut observed, mut stopped, mut elapsed) = (0, 0, 0);
            for s in 0..slots {
                let slot_from = from + Duration::seconds(s * slot_secs);
                let slot_to = slot_from + Duration::seconds(slot_secs);
                if slot_from > now {
                    break;
                }
                let seen = inside
                    .iter()
                    .any(|(at, _)| *at >= slot_from && *at < slot_to);
                let down = downtime
                    .iter()
                    .any(|d| d.overlaps(slot_from, slot_to.min(now), runtime));
                // The slot in progress counts only once something is known.
                if slot_to > now && !seen && !down {
                    break;
                }
                elapsed += 1;
                if seen {
                    observed += 1;
                } else if down {
                    stopped += 1;
                }
            }
            cell(from, tally, observed, stopped, elapsed)
        })
        .collect()
}

/// Cells from the hourly availability counters, which already leave planned
/// downtime out.
fn strip_from_hours(
    window: Window,
    now: DateTime<Utc>,
    hours: Option<&BTreeMap<String, incidents::Counter>>,
    downtime: &[planned::Interval],
    runtime: &str,
) -> Vec<Cell> {
    let (start, cells) = grid(window, now);
    let per_cell = window.bucket_secs / 3_600;
    (0..cells)
        .map(|i| {
            let from = start + Duration::seconds(i * window.bucket_secs);
            let mut tally = Tally::default();
            let (mut observed, mut stopped, mut elapsed) = (0, 0, 0);
            for h in 0..per_cell {
                let hour = from + Duration::hours(h);
                if hour > now {
                    break;
                }
                let next = hour + Duration::hours(1);
                let counter = hours.and_then(|m| m.get(&incidents::hour_key(hour)));
                let down = downtime
                    .iter()
                    .any(|d| d.overlaps(hour, next.min(now), runtime));
                if next > now && counter.is_none() && !down {
                    break;
                }
                elapsed += 1;
                if let Some(c) = counter {
                    tally.checks += c.checks;
                    tally.ready += c.ready;
                    observed += 1;
                } else if down {
                    stopped += 1;
                }
            }
            cell(from, tally, observed, stopped, elapsed)
        })
        .collect()
}

/// Everything read once and shared by all tiles.
struct Records {
    /// Health checks per workload, oldest first.
    health: Option<BTreeMap<String, Vec<HealthRecord>>>,
    incidents: Option<incidents::IncidentStore>,
    availability: Option<incidents::Availability>,
    audit: Option<AuditLog>,
    holds: Vec<holds::Hold>,
    dependencies: Option<crate::dependencies::DependencyGraph>,
}

/// Health records of every workload, oldest first.
pub(crate) fn health_by_workload(dir: &Path) -> Result<BTreeMap<String, Vec<HealthRecord>>> {
    let history: HealthHistory = store::load(&incidents::health_path(dir))?;
    let mut by: BTreeMap<String, Vec<HealthRecord>> = BTreeMap::new();
    for r in history.records {
        by.entry(r.workload.clone()).or_default().push(r);
    }
    Ok(by)
}

pub(crate) fn timed(records: &[HealthRecord]) -> Vec<(DateTime<Utc>, &HealthRecord)> {
    let mut out: Vec<_> = records
        .iter()
        .filter_map(|r| Some((ops::parse_time(&r.timestamp)?, r)))
        .collect();
    out.sort_by_key(|(at, _)| *at);
    out
}

fn sorted(mut names: Vec<String>) -> Vec<String> {
    names.sort();
    names
}

fn tile(
    dir: &Path,
    ws: &WorkloadState,
    window: Window,
    stale_after: i64,
    now: DateTime<Utc>,
    r: &Records,
    unavailable: &mut Vec<String>,
) -> Tile {
    let name = ws.name.as_str();
    let runtime = ws.runtime.to_string();
    let empty = Vec::new();
    let mine = r
        .health
        .as_ref()
        .and_then(|h| h.get(name))
        .unwrap_or(&empty);
    let records = timed(mine);
    let expected = r
        .incidents
        .as_ref()
        .and_then(|s| s.expected_down.get(name))
        .filter(|d| d.runtime == ws.runtime);
    let events: Vec<&AuditEvent> = r
        .audit
        .iter()
        .flat_map(|a| a.events())
        .filter(|e| e.workload == name)
        .collect();
    let downtime = planned::intervals(events, mine, expected);

    let strip = match window.source {
        Source::Checks => {
            strip_from_checks(window, now, stale_after, &records, &downtime, &runtime)
        }
        Source::Hourly => strip_from_hours(
            window,
            now,
            r.availability.as_ref().and_then(|a| a.hours.get(name)),
            &downtime,
            &runtime,
        ),
    };
    let (checks, ready) = strip.iter().fold((0u32, 0u32), |(c, k), cell| {
        (c + cell.checks, k + cell.ready)
    });

    // Checks of an earlier runtime (before a migration) say nothing about now.
    let last = records
        .iter()
        .rev()
        .find(|(_, rec)| rec.runtime == ws.runtime);
    let error = r
        .incidents
        .as_ref()
        .and_then(|s| s.observe_errors.get(name))
        .filter(|e| last.is_none_or(|(at, _)| ops::parse_time(&e.at).is_none_or(|t| t >= *at)));
    let state = if r.health.is_none() || r.incidents.is_none() {
        TileState::Unknown
    } else if expected.is_some() {
        TileState::Stopped
    } else if error.is_some() {
        TileState::Unreadable
    } else {
        match last {
            None => TileState::NoData,
            Some((at, _)) if (now - *at).num_seconds() > stale_after => TileState::Stale,
            Some((_, rec)) if rec.ready => TileState::Ready,
            Some(_) => TileState::NotReady,
        }
    };

    let open_incidents = r.incidents.as_ref().map(|s| {
        s.incidents
            .iter()
            .filter(|i| i.workload == name && i.is_open())
            .map(|i| OpenIncident {
                id: i.id,
                opened_at: i.opened_at.clone(),
                failed_checks: i.failed_checks,
                acknowledged: i.acknowledged.is_some(),
            })
            .collect()
    });

    let last_change = read(
        revisions::load(dir, name),
        &format!("revisions of {}", name.escape_default()),
        unavailable,
    )
    .and_then(|log| {
        let latest = log.revisions.last()?;
        Some(LastChange {
            revision: latest.id,
            previous_revision: log.revisions.iter().rev().nth(1).map(|p| p.id),
            at: latest.at.clone(),
            actor: latest.actor.clone(),
            op: latest.op,
        })
    });

    Tile {
        name: ws.name.clone(),
        runtime,
        image: ws.instance.image.clone(),
        state,
        last_check: last.map(|(_, rec)| LastCheck {
            at: rec.timestamp.clone(),
            state: rec.state.to_string(),
            ready: rec.ready,
            restart_count: rec.restart_count,
        }),
        stopped_since: expected.map(|d| d.since.clone()),
        error: error.map(|e| crate::speclint::sanitize(&e.message)),
        strip,
        ready_pct: (checks > 0).then(|| f64::from(ready) * 100.0 / f64::from(checks)),
        checks,
        open_incidents,
        hold: r.holds.iter().find(|h| h.workload == name).cloned(),
        last_change,
        depends_on: r
            .dependencies
            .as_ref()
            .map(|g| sorted(g.dependencies_of(name)))
            .unwrap_or_default(),
        dependents: r
            .dependencies
            .as_ref()
            .map(|g| sorted(g.dependents_of(name)))
            .unwrap_or_default(),
    }
}

/// The board for every managed workload, sorted by name.
pub fn build(
    dir: &Path,
    state: &StateStore,
    window: Window,
    observe_secs: u64,
    now: DateTime<Utc>,
) -> Result<Board> {
    let mut unavailable = Vec::new();
    let records = Records {
        health: read(health_by_workload(dir), "health checks", &mut unavailable),
        incidents: read(incidents::load(dir), "incidents", &mut unavailable),
        availability: read(
            store::load(&incidents::availability_path(dir)),
            "availability counters",
            &mut unavailable,
        ),
        audit: read(
            store::load(&journal::audit_path(dir)),
            "audit trail",
            &mut unavailable,
        ),
        holds: read(holds::active(dir), "holds", &mut unavailable).unwrap_or_default(),
        dependencies: read(
            crate::dependencies::DependencyGraph::load(&dir.join("dependencies.json")),
            "dependencies",
            &mut unavailable,
        ),
    };
    let stale_after = stale_after_secs(observe_secs);
    let mut managed = state.list();
    managed.sort_by(|a, b| a.name.cmp(&b.name));
    let workloads = managed
        .into_iter()
        .map(|ws| {
            tile(
                dir,
                ws,
                window,
                stale_after,
                now,
                &records,
                &mut unavailable,
            )
        })
        .collect();
    Ok(Board {
        generated_at: now.to_rfc3339(),
        window: window.key,
        bucket_secs: window.bucket_secs,
        observe_secs,
        stale_after_secs: stale_after,
        incident_threshold: incidents::threshold(),
        workloads,
        unavailable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::incidents::Observation;
    use crate::ops::revisions::tests::instance;
    use crate::runtime::{InstanceState, RuntimeKind};

    fn at(s: &str) -> DateTime<Utc> {
        ops::parse_time(s).unwrap()
    }

    fn state_with(names: &[&str]) -> StateStore {
        let mut s = StateStore::default();
        for name in names {
            s.upsert(
                name.to_string(),
                WorkloadState {
                    name: name.to_string(),
                    runtime: RuntimeKind::Podman,
                    instance: instance(name, RuntimeKind::Podman),
                    spec_path: "spec.yaml".into(),
                    created_at: "2026-03-01T00:00:00Z".into(),
                    updated_at: "2026-03-01T00:00:00Z".into(),
                    os_version: None,
                    node_labels: Vec::new(),
                    atlas_volume_ids: Vec::new(),
                    namespace: None,
                    cluster_context: None,
                    k8s_kind: None,
                },
            );
        }
        s
    }

    fn observe(dir: &Path, name: &str, ready: bool, when: &str) {
        incidents::record(
            dir,
            &Observation {
                workload: name.into(),
                runtime: RuntimeKind::Podman,
                state: if ready {
                    InstanceState::Running
                } else {
                    InstanceState::Failed
                },
                ready,
                restart_count: 0,
                message: None,
                at: when.into(),
            },
        )
        .unwrap();
    }

    fn board(dir: &Path, state: &StateStore, window: &str, now: &str) -> Board {
        build(dir, state, parse_window(window).unwrap(), 60, at(now)).unwrap()
    }

    #[test]
    fn every_window_has_whole_cells() {
        for w in WINDOWS {
            assert_eq!(w.secs % w.bucket_secs, 0, "{}", w.key);
            if w.source == Source::Hourly {
                assert_eq!(w.bucket_secs % 3_600, 0, "{}", w.key);
            }
        }
        assert!(parse_window("2h").is_err());
        assert_eq!(parse_window(" 24h ").unwrap().secs, 86_400);
    }

    #[test]
    fn staleness_follows_the_check_interval() {
        assert_eq!(stale_after_secs(0), 900);
        assert_eq!(stale_after_secs(15), 180);
        assert_eq!(stale_after_secs(60), 300);
        assert_eq!(stale_after_secs(u64::MAX), 7 * 86_400);
    }

    #[test]
    fn a_workload_without_checks_has_no_data_and_no_percentage() {
        let dir = tempfile::tempdir().unwrap();
        let b = board(
            dir.path(),
            &state_with(&["web"]),
            "1h",
            "2026-03-01T12:00:30Z",
        );
        let t = &b.workloads[0];
        assert_eq!(t.state, TileState::NoData);
        assert_eq!(t.ready_pct, None);
        assert_eq!(t.strip.len(), 30);
        assert!(t.strip.iter().all(|c| c.state == CellState::NoData));
        assert!(b.unavailable.is_empty(), "{:?}", b.unavailable);
    }

    #[test]
    fn gaps_between_checks_stay_no_data() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        observe(d, "web", true, "2026-03-01T11:30:10Z");
        observe(d, "web", false, "2026-03-01T11:50:10Z");
        observe(d, "web", true, "2026-03-01T11:50:50Z");
        observe(d, "web", true, "2026-03-01T11:59:10Z");
        let b = board(d, &state_with(&["web"]), "1h", "2026-03-01T11:59:30Z");
        let t = &b.workloads[0];
        let states: Vec<_> = t.strip.iter().map(|c| c.state).collect();
        // Cells start at 11:00 and are two minutes long.
        assert_eq!(states[15], CellState::Up);
        assert_eq!(states[16], CellState::NoData);
        assert_eq!(states[25], CellState::Partial);
        assert_eq!(states[29], CellState::Up);
        assert_eq!(
            states.iter().filter(|s| **s == CellState::NoData).count(),
            27
        );
        assert_eq!(t.state, TileState::Ready);
        assert_eq!(t.checks, 4);
        assert_eq!(t.ready_pct, Some(75.0));
    }

    #[test]
    fn an_old_ready_check_is_stale_not_ready() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        observe(d, "web", true, "2026-03-01T11:00:00Z");
        let state = state_with(&["web"]);
        assert_eq!(
            board(d, &state, "1h", "2026-03-01T11:04:00Z").workloads[0].state,
            TileState::Ready
        );
        assert_eq!(
            board(d, &state, "1h", "2026-03-01T11:06:00Z").workloads[0].state,
            TileState::Stale
        );
    }

    #[test]
    fn a_long_cell_with_one_check_reports_its_gaps() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        observe(d, "web", true, "2026-03-01T11:31:00Z");
        let b = board(d, &state_with(&["web"]), "6h", "2026-03-01T11:59:59Z");
        let cell = &b.workloads[0].strip[33];
        assert_eq!(cell.state, CellState::Up);
        assert!(cell.covered > 0.0 && cell.covered < 0.6, "{}", cell.covered);
    }

    #[test]
    fn hourly_windows_use_the_counters() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        observe(d, "web", true, "2026-03-01T09:10:00Z");
        observe(d, "web", false, "2026-03-01T11:10:00Z");
        observe(d, "web", false, "2026-03-01T11:11:00Z");
        let b = board(d, &state_with(&["web"]), "24h", "2026-03-01T11:12:00Z");
        let t = &b.workloads[0];
        assert_eq!(t.strip.len(), 24);
        assert_eq!(t.strip[23].state, CellState::Down);
        assert_eq!(t.strip[22].state, CellState::NoData);
        assert_eq!(t.strip[21].state, CellState::Up);
        assert_eq!(t.state, TileState::NotReady);
        assert_eq!(t.open_incidents.as_ref().unwrap().len(), 1);
        // A day-long cell observed in two of its hours is not fully covered.
        let month = board(d, &state_with(&["web"]), "30d", "2026-03-01T11:12:00Z");
        let today = month.workloads[0].strip.last().unwrap();
        assert_eq!(today.state, CellState::Partial);
        assert!(today.covered < 0.5, "{}", today.covered);
    }

    #[test]
    fn planned_downtime_is_neither_up_nor_down() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        observe(d, "web", true, "2026-03-01T11:40:00Z");
        incidents::expect_down(d, "web", RuntimeKind::Podman, "stopped").unwrap();
        // The tracker stamps the stop with the real clock, so observe after it.
        let stopped = Utc::now() + Duration::seconds(5);
        observe(d, "web", false, &stopped.to_rfc3339());
        let b = build(
            d,
            &state_with(&["web"]),
            parse_window("1h").unwrap(),
            60,
            stopped + Duration::seconds(1),
        )
        .unwrap();
        let t = &b.workloads[0];
        assert_eq!(t.state, TileState::Stopped);
        assert_eq!(t.strip.last().unwrap().state, CellState::Planned);
        assert_eq!(t.checks, 0);
        assert_eq!(t.ready_pct, None);
    }

    #[test]
    fn an_unreadable_status_is_not_reported_as_healthy() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        observe(d, "web", true, &Utc::now().to_rfc3339());
        incidents::record_error(d, "web", "podman: not found").unwrap();
        let t = build(
            d,
            &state_with(&["web"]),
            parse_window("1h").unwrap(),
            60,
            Utc::now(),
        )
        .unwrap()
        .workloads
        .remove(0);
        assert_eq!(t.state, TileState::Unreadable);
        assert_eq!(t.error.as_deref(), Some("podman: not found"));
    }

    #[test]
    fn unreadable_records_are_reported_and_make_the_state_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        observe(d, "web", true, &Utc::now().to_rfc3339());
        std::fs::write(incidents::path(d), "{").unwrap();
        let b = build(
            d,
            &state_with(&["web"]),
            parse_window("1h").unwrap(),
            60,
            Utc::now(),
        )
        .unwrap();
        assert_eq!(b.workloads[0].state, TileState::Unknown);
        assert!(b.workloads[0].open_incidents.is_none());
        assert!(b.unavailable.iter().any(|u| u.starts_with("incidents")));
    }

    #[test]
    fn tiles_carry_holds_dependencies_and_the_last_change_without_the_spec() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        for memory in ["256Mi", "512Mi"] {
            revisions::record(
                d,
                revisions::RevisionOp::Deploy,
                &crate::ops::revisions::tests::spec("api", memory),
                &instance("api", RuntimeKind::Podman),
            )
            .unwrap();
        }
        holds::set(d, "api", "freeze", None).unwrap();
        let mut graph = crate::dependencies::DependencyGraph::new();
        graph.add_dependency("web", "api");
        graph.add_dependency("api", "db");
        graph.save(&d.join("dependencies.json")).unwrap();
        let b = board(
            d,
            &state_with(&["web", "api"]),
            "1h",
            "2026-03-01T12:00:00Z",
        );
        let api = &b.workloads[0];
        assert_eq!(api.name, "api");
        assert_eq!(api.dependents, ["web"]);
        assert_eq!(api.depends_on, ["db"]);
        assert_eq!(api.hold.as_ref().unwrap().reason, "freeze");
        let change = api.last_change.as_ref().unwrap();
        assert_eq!((change.revision, change.previous_revision), (2, Some(1)));
        let json = serde_json::to_string(&b).unwrap();
        assert!(!json.contains("hunter2"), "{json}");
    }
}
