// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! When was a workload down on purpose?
//!
//! The health records say whether a workload was ready; they do not say whether
//! somebody asked for it to be stopped. That is reconstructed here from what
//! Aether recorded: a successful stop or delete starts planned downtime, and
//! the next deploy, update or start (successful or not) ends it, as does a
//! ready check made after the runtime had time to wind down. These are the same
//! rules the incident tracker applies while it observes.

use crate::audit::{ActionResult, AuditAction, AuditEvent};
use crate::health::HealthRecord;
use crate::ops::{self, incidents::ExpectedDown, incidents::STOP_GRACE_SECS};
use chrono::{DateTime, Utc};

/// A period during which a workload was expected to be down.
#[derive(Debug, Clone, PartialEq)]
pub struct Interval {
    pub from: DateTime<Utc>,
    /// `None` while it is still expected to be down.
    pub to: Option<DateTime<Utc>>,
    /// The runtime that was stopped; `None` when the record does not say.
    pub runtime: Option<String>,
}

impl Interval {
    fn on(&self, runtime: &str) -> bool {
        self.runtime.as_deref().is_none_or(|r| r == runtime)
    }

    /// Whether a check of `runtime` made at `at` falls in planned downtime.
    pub fn covers(&self, at: DateTime<Utc>, runtime: &str) -> bool {
        self.on(runtime) && at >= self.from && self.to.is_none_or(|to| at < to)
    }

    /// Whether any part of `from..to` was planned downtime of `runtime`.
    pub fn overlaps(&self, from: DateTime<Utc>, to: DateTime<Utc>, runtime: &str) -> bool {
        self.on(runtime) && self.from < to && self.to.is_none_or(|end| end > from)
    }
}

/// A hold is audited as a configuration change but changes nothing.
pub fn is_hold_record(event: &AuditEvent) -> bool {
    event.action == AuditAction::ConfigChange
        && (event.message.starts_with("hold placed:")
            || event.message.starts_with("hold released:"))
}

enum Step<'a> {
    Stop(Option<&'a str>),
    Change(Option<&'a str>),
    Ready(String),
}

/// Planned downtime of one workload. `events` and `records` must all belong to
/// it; `expected` is the incident tracker's current expectation.
pub fn intervals<'a>(
    events: impl IntoIterator<Item = &'a AuditEvent>,
    records: impl IntoIterator<Item = &'a HealthRecord>,
    expected: Option<&ExpectedDown>,
) -> Vec<Interval> {
    let mut steps: Vec<(DateTime<Utc>, u8, Step)> = Vec::new();
    for e in events {
        let Some(at) = ops::parse_time(&e.timestamp) else {
            continue;
        };
        let runtime = e.runtime.as_deref();
        match e.action {
            AuditAction::Stop | AuditAction::Delete if e.result == ActionResult::Success => {
                steps.push((at, 0, Step::Stop(runtime)));
            }
            AuditAction::Deploy | AuditAction::ConfigChange | AuditAction::Start
                if !is_hold_record(e) =>
            {
                steps.push((at, 0, Step::Change(runtime)));
            }
            _ => {}
        }
    }
    for r in records {
        if !r.ready {
            continue;
        }
        if let Some(at) = ops::parse_time(&r.timestamp) {
            steps.push((at, 1, Step::Ready(r.runtime.to_string())));
        }
    }
    steps.sort_by_key(|(at, order, _)| (*at, *order));

    let mut out = Vec::new();
    let mut open: Option<Interval> = None;
    for (at, _, step) in steps {
        match step {
            Step::Stop(runtime) => {
                let same = open
                    .as_ref()
                    .is_some_and(|o| o.runtime.as_deref() == runtime);
                if !same {
                    if let Some(mut o) = open.take() {
                        o.to = Some(at);
                        out.push(o);
                    }
                    open = Some(Interval {
                        from: at,
                        to: None,
                        runtime: runtime.map(str::to_string),
                    });
                }
            }
            Step::Change(runtime) => {
                if open
                    .as_ref()
                    .is_some_and(|o| runtime.is_none_or(|r| o.on(r)))
                {
                    if let Some(mut o) = open.take() {
                        o.to = Some(at);
                        out.push(o);
                    }
                }
            }
            Step::Ready(runtime) => {
                let started_again = open.as_ref().is_some_and(|o| {
                    o.on(&runtime) && (at - o.from).num_seconds() >= STOP_GRACE_SECS
                });
                if started_again {
                    if let Some(mut o) = open.take() {
                        o.to = Some(at);
                        out.push(o);
                    }
                }
            }
        }
    }
    out.extend(open);

    // The tracker's expectation survives a trimmed audit trail.
    if let Some(down) = expected {
        let runtime = down.runtime.to_string();
        let known = out.iter().any(|i| i.to.is_none() && i.on(&runtime));
        if let (false, Some(from)) = (known, ops::parse_time(&down.since)) {
            out.push(Interval {
                from,
                to: None,
                runtime: Some(runtime),
            });
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{InstanceState, RuntimeKind};

    fn t(minute: u32) -> DateTime<Utc> {
        ops::parse_time(&format!("2026-03-01T12:{minute:02}:00Z")).unwrap()
    }

    fn event(minute: u32, action: AuditAction, result: ActionResult, message: &str) -> AuditEvent {
        AuditEvent {
            id: u64::from(minute),
            timestamp: t(minute).to_rfc3339(),
            action,
            workload: "web".into(),
            runtime: Some("podman".into()),
            result,
            message: message.into(),
            details: None,
            integrity_hash: None,
        }
    }

    fn ready(minute: u32) -> HealthRecord {
        HealthRecord {
            timestamp: t(minute).to_rfc3339(),
            workload: "web".into(),
            runtime: RuntimeKind::Podman,
            state: InstanceState::Running,
            ready: true,
            restart_count: 0,
            latency_ms: None,
        }
    }

    #[test]
    fn a_stop_lasts_until_the_next_deploy() {
        let events = [
            event(1, AuditAction::Stop, ActionResult::Success, "stopped"),
            event(9, AuditAction::Deploy, ActionResult::Success, "deployed"),
        ];
        let got = intervals(&events, [], None);
        assert_eq!(got.len(), 1);
        assert!(got[0].covers(t(5), "podman"));
        assert!(!got[0].covers(t(9), "podman"));
        assert!(!got[0].covers(t(0), "podman"));
        assert!(!got[0].covers(t(5), "kubernetes"));
        assert!(got[0].overlaps(t(0), t(2), "podman"));
        assert!(!got[0].overlaps(t(9), t(12), "podman"));
    }

    #[test]
    fn a_failed_stop_and_a_hold_change_nothing() {
        let failed = [event(1, AuditAction::Stop, ActionResult::Failure, "nope")];
        assert!(intervals(&failed, [], None).is_empty());
        let held = [
            event(1, AuditAction::Stop, ActionResult::Success, "stopped"),
            event(
                2,
                AuditAction::ConfigChange,
                ActionResult::Success,
                "hold placed: freeze",
            ),
        ];
        assert_eq!(intervals(&held, [], None)[0].to, None);
    }

    #[test]
    fn a_ready_check_ends_it_only_after_the_grace_period() {
        let events = [event(
            1,
            AuditAction::Stop,
            ActionResult::Success,
            "stopped",
        )];
        // One minute later the runtime is still winding down.
        assert_eq!(intervals(&events, &[ready(2)], None)[0].to, None);
        assert_eq!(
            intervals(&events, &[ready(2), ready(6)], None)[0].to,
            Some(t(6))
        );
    }

    #[test]
    fn a_failed_deploy_is_not_planned_downtime() {
        let events = [
            event(1, AuditAction::Delete, ActionResult::Success, "deleted"),
            event(
                4,
                AuditAction::Deploy,
                ActionResult::Failure,
                "deploy failed",
            ),
        ];
        assert_eq!(intervals(&events, [], None)[0].to, Some(t(4)));
    }

    #[test]
    fn the_trackers_expectation_is_used_when_the_audit_trail_is_silent() {
        let down = ExpectedDown {
            runtime: RuntimeKind::Podman,
            since: t(3).to_rfc3339(),
            why: "stopped".into(),
        };
        let got = intervals([], [], Some(&down));
        assert_eq!(got.len(), 1);
        assert!(got[0].covers(t(30), "podman"));
    }
}
