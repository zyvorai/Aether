// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Critical issue notifications derived from Command Center briefing.

use crate::intelligence::briefing::build_command_center_briefing;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalNotification {
    pub id: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub workload: Option<String>,
    pub route: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CriticalNotificationsReport {
    pub generated_at: String,
    pub notifications: Vec<CriticalNotification>,
    pub notify_tray: bool,
}

pub fn build_critical_notifications(
    state_path: &Path,
) -> anyhow::Result<CriticalNotificationsReport> {
    let briefing = build_command_center_briefing(state_path)?;
    let mut notifications = Vec::new();

    for (idx, issue) in briefing.issues.iter().enumerate() {
        let sev = issue.severity.to_lowercase();
        if sev != "critical" && sev != "high" {
            continue;
        }
        notifications.push(CriticalNotification {
            id: format!("crit-{idx}"),
            severity: issue.severity.clone(),
            title: issue.title.clone(),
            detail: issue.detail.clone(),
            workload: issue.workload.clone(),
            route: "observability".into(),
        });
    }

    for (idx, risk) in briefing.capacity_risks.iter().enumerate() {
        if risk.days_remaining > 7 {
            continue;
        }
        notifications.push(CriticalNotification {
            id: format!("cap-{idx}"),
            severity: if risk.days_remaining <= 3 {
                "critical".into()
            } else {
                "high".into()
            },
            title: format!("Capacity risk — {}", risk.resource),
            detail: risk.summary.clone(),
            workload: None,
            route: "observability".into(),
        });
    }

    notifications.sort_by_key(|b| std::cmp::Reverse(severity_rank(&b.severity)));
    notifications.truncate(6);

    Ok(CriticalNotificationsReport {
        generated_at: crate::resources::now_rfc3339(),
        notify_tray: !notifications.is_empty(),
        notifications,
    })
}

fn severity_rank(severity: &str) -> u8 {
    match severity.to_lowercase().as_str() {
        "critical" => 3,
        "high" => 2,
        "medium" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::StateStore;

    #[test]
    fn empty_fleet_notifications() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let report = build_critical_notifications(&path).unwrap();
        assert!(!report.notify_tray);
    }
}
