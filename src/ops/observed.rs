// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! The status of a managed workload, as last observed.
//!
//! An entry in the state store only says that Aether deployed something. What
//! the workload is doing now comes from the recorded observations, and where
//! there is no current one the answer is `unknown`, never `running`.

use crate::health::{HealthHistory, HealthRecord};
use crate::ops::{self, incidents, store};
use crate::runtime::InstanceState;
use crate::state::WorkloadState;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::path::Path;

/// An observation older than this no longer describes the present.
pub const STALE_SECS: i64 = 900;

/// [`STALE_SECS`], stretched when `AETHER_OBSERVE_SECS` makes observations
/// rarer than that, so a healthy slow schedule is not reported as stale.
pub fn stale_secs() -> i64 {
    let every = std::env::var("AETHER_OBSERVE_SECS")
        .ok()
        .and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(0);
    STALE_SECS.max(every.saturating_mul(3))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Condition {
    /// Running and ready.
    Running,
    /// Running, but failing or still starting its health check.
    Unhealthy,
    Pending,
    Failed,
    /// Found stopped, and nobody asked Aether to stop it.
    Down,
    /// Repeated checks found it not ready, without saying how.
    NotReady,
    /// Stopped (or deleted) through Aether.
    Stopped,
    /// Never checked, not checked recently, or the check could not tell.
    Unknown,
}

impl Condition {
    /// The status string the API, the dashboard and the CLI show.
    pub fn label(self) -> &'static str {
        match self {
            Condition::Running => "running",
            Condition::Unhealthy => "unhealthy",
            Condition::Pending => "pending",
            Condition::Failed => "failed",
            Condition::Down => "stopped unexpectedly",
            Condition::NotReady => "not ready",
            Condition::Stopped => "stopped",
            Condition::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservedStatus {
    pub condition: Condition,
    /// What the status is based on, in words.
    pub detail: String,
    /// Time of the observation the status comes from, if it comes from one.
    pub observed_at: Option<String>,
}

fn answer(condition: Condition, detail: String, observed_at: Option<String>) -> ObservedStatus {
    ObservedStatus {
        condition,
        detail,
        observed_at,
    }
}

fn of_record(record: &HealthRecord, confirmed_down: bool) -> Condition {
    match record.state {
        InstanceState::Running if record.ready => Condition::Running,
        InstanceState::Running => Condition::Unhealthy,
        InstanceState::Pending => Condition::Pending,
        InstanceState::Failed => Condition::Failed,
        InstanceState::Stopped => Condition::Down,
        InstanceState::Unknown if !record.ready && confirmed_down => Condition::NotReady,
        InstanceState::Unknown => Condition::Unknown,
    }
}

/// The recorded observations, loaded once and asked about many workloads.
#[derive(Debug, Default)]
pub struct Snapshot {
    history: HealthHistory,
    incidents: incidents::IncidentStore,
    /// Set when the records exist but cannot be read.
    load_error: Option<String>,
}

impl Snapshot {
    pub fn load(dir: &Path) -> Self {
        let history = store::load::<HealthHistory>(&incidents::health_path(dir));
        let incidents = incidents::load(dir);
        match (history, incidents) {
            (Ok(history), Ok(incidents)) => Self {
                history,
                incidents,
                load_error: None,
            },
            (Err(e), _) | (_, Err(e)) => Self {
                load_error: Some(format!("{e:#}")),
                ..Self::default()
            },
        }
    }

    pub fn status(&self, ws: &WorkloadState, now: DateTime<Utc>) -> ObservedStatus {
        let name = ws.name.as_str();
        if let Some(e) = &self.load_error {
            return answer(
                Condition::Unknown,
                format!("health records cannot be read: {e}"),
                None,
            );
        }
        let stale = stale_secs();
        let age = |t: DateTime<Utc>| (now - t).num_seconds();
        // Observations made before the last deploy or start describe the
        // instance that was replaced.
        let changed = ops::parse_time(&ws.updated_at);
        let since_change = |t: DateTime<Utc>| changed.is_none_or(|c| t >= c);
        let mine = || {
            self.history
                .records
                .iter()
                .rev()
                .filter(move |r| r.workload == name && r.runtime == ws.runtime)
        };
        let latest = mine().find_map(|r| {
            let at = ops::parse_time(&r.timestamp)?;
            since_change(at).then_some((r, at))
        });
        let requested = self
            .incidents
            .expected_down
            .get(name)
            .filter(|d| d.runtime == ws.runtime)
            .and_then(|d| Some((d, ops::parse_time(&d.since)?)));

        // A failed status query newer than anything else known.
        let unread = self.incidents.observe_errors.get(name).filter(|e| {
            ops::parse_time(&e.at).is_some_and(|at| {
                since_change(at)
                    && latest.is_none_or(|(_, seen)| at >= seen)
                    && requested.is_none_or(|(_, asked)| at >= asked)
            })
        });
        if let Some(e) = unread {
            return answer(
                Condition::Unknown,
                format!("status could not be read at {}: {}", e.at, e.message),
                None,
            );
        }

        if let Some((down, asked)) = requested {
            let confirmation = latest.filter(|(_, seen)| *seen >= asked);
            let evidence = confirmation.map_or(asked, |(_, seen)| seen);
            return if age(evidence) <= stale {
                answer(
                    Condition::Stopped,
                    format!("{} on request at {}", down.why, down.since),
                    confirmation.map(|(r, _)| r.timestamp.clone()),
                )
            } else {
                answer(
                    Condition::Unknown,
                    format!(
                        "{} on request at {}; not checked for {}",
                        down.why,
                        down.since,
                        ops::format_duration(age(evidence))
                    ),
                    confirmation.map(|(r, _)| r.timestamp.clone()),
                )
            };
        }

        let incident = self
            .incidents
            .incidents
            .iter()
            .rev()
            .find(|i| i.workload == name && i.runtime == ws.runtime && i.is_open());
        let Some((record, seen)) = latest else {
            let detail = if mine().next().is_some() {
                "not checked since it was last deployed or started"
            } else {
                "not checked yet"
            };
            return answer(Condition::Unknown, detail.to_string(), None);
        };
        let condition = of_record(record, incident.is_some());
        if age(seen) > stale {
            let was = match condition {
                Condition::Unknown => String::new(),
                c => format!(", when it was {}", c.label()),
            };
            return answer(
                Condition::Unknown,
                format!("last checked {} ago{was}", ops::format_duration(age(seen))),
                Some(record.timestamp.clone()),
            );
        }
        let mut detail = format!(
            "checked {} ago: {}, {}",
            ops::format_duration(age(seen)),
            record.state,
            if record.ready { "ready" } else { "not ready" }
        );
        if let Some(i) = incident {
            detail.push_str(&format!("; incident {} is open", i.id));
        }
        answer(condition, detail, Some(record.timestamp.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::incidents::tests::obs;
    use crate::ops::revisions::tests::instance;
    use crate::runtime::RuntimeKind;
    use chrono::Duration;

    fn workload(name: &str, updated: DateTime<Utc>) -> WorkloadState {
        let mut ws = WorkloadState::new(
            name.into(),
            RuntimeKind::Podman,
            instance(name, RuntimeKind::Podman),
            "web.yaml".into(),
        );
        ws.updated_at = updated.to_rfc3339();
        ws
    }

    fn record(dir: &Path, state: InstanceState, ready: bool, at: DateTime<Utc>) {
        let mut o = obs("web", ready, &at.to_rfc3339());
        o.runtime = RuntimeKind::Podman;
        o.state = state;
        incidents::record(dir, &o).unwrap();
    }

    fn look(dir: &Path, ws: &WorkloadState) -> ObservedStatus {
        Snapshot::load(dir).status(ws, Utc::now())
    }

    #[test]
    fn a_workload_nobody_checked_is_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let ws = workload("web", Utc::now());
        let s = look(dir.path(), &ws);
        assert_eq!(s.condition, Condition::Unknown);
        assert_eq!(s.detail, "not checked yet");
    }

    #[test]
    fn a_stop_nobody_asked_for_is_not_running() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let now = Utc::now();
        let ws = workload("web", now - Duration::hours(1));
        record(d, InstanceState::Running, true, now - Duration::seconds(90));
        assert_eq!(look(d, &ws).condition, Condition::Running);
        // One failed check is already what was observed, before any incident.
        record(
            d,
            InstanceState::Stopped,
            false,
            now - Duration::seconds(60),
        );
        assert_eq!(look(d, &ws).condition, Condition::Down);
        record(
            d,
            InstanceState::Stopped,
            false,
            now - Duration::seconds(30),
        );
        let s = look(d, &ws);
        assert_eq!(s.condition, Condition::Down);
        assert_eq!(s.condition.label(), "stopped unexpectedly");
        assert!(s.detail.contains("incident 1 is open"), "{}", s.detail);
    }

    #[test]
    fn a_requested_stop_is_just_stopped() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let ws = workload("web", Utc::now() - Duration::hours(1));
        record(
            d,
            InstanceState::Running,
            true,
            Utc::now() - Duration::seconds(30),
        );
        incidents::expect_down(d, "web", RuntimeKind::Podman, "stopped").unwrap();
        let s = look(d, &ws);
        assert_eq!(s.condition, Condition::Stopped, "{}", s.detail);
        assert!(s.detail.starts_with("stopped on request"), "{}", s.detail);
        // Long after, with nothing watching, somebody may have started it.
        let later = Snapshot::load(d).status(&ws, Utc::now() + Duration::hours(2));
        assert_eq!(later.condition, Condition::Unknown, "{}", later.detail);
    }

    #[test]
    fn old_observations_are_not_current() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let now = Utc::now();
        let ws = workload("web", now - Duration::days(2));
        record(d, InstanceState::Running, true, now - Duration::hours(2));
        let s = look(d, &ws);
        assert_eq!(s.condition, Condition::Unknown);
        assert!(s.detail.contains("when it was running"), "{}", s.detail);
        assert!(s.observed_at.is_some());
    }

    #[test]
    fn checks_from_before_a_redeploy_do_not_count() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let now = Utc::now();
        record(
            d,
            InstanceState::Stopped,
            false,
            now - Duration::seconds(60),
        );
        let ws = workload("web", now - Duration::seconds(5));
        let s = look(d, &ws);
        assert_eq!(s.condition, Condition::Unknown);
        assert!(s.detail.contains("since it was last"), "{}", s.detail);
    }

    #[test]
    fn a_failed_status_query_is_unknown() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let now = Utc::now();
        let ws = workload("web", now - Duration::hours(1));
        record(d, InstanceState::Running, true, now - Duration::seconds(30));
        incidents::record_error(d, "web", "podman is not installed").unwrap();
        let s = look(d, &ws);
        assert_eq!(s.condition, Condition::Unknown);
        assert!(s.detail.contains("podman is not installed"), "{}", s.detail);
    }

    #[test]
    fn a_vague_not_ready_needs_confirming() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let now = Utc::now();
        let ws = workload("web", now - Duration::hours(1));
        record(
            d,
            InstanceState::Unknown,
            false,
            now - Duration::seconds(60),
        );
        assert_eq!(look(d, &ws).condition, Condition::Unknown);
        record(
            d,
            InstanceState::Unknown,
            false,
            now - Duration::seconds(30),
        );
        assert_eq!(look(d, &ws).condition, Condition::NotReady);
    }

    #[test]
    fn unreadable_records_are_said_to_be_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("health.json"), "{").unwrap();
        let s = look(dir.path(), &workload("web", Utc::now()));
        assert_eq!(s.condition, Condition::Unknown);
        assert!(s.detail.contains("cannot be read"), "{}", s.detail);
    }
}
