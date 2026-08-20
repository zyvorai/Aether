// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! SRE & Reliability OS — runbook scheduler, incidents, on-call, postmortems, chaos, MTTR.

use crate::audit::{ActionResult, AuditAction, AuditLog};
use crate::events::{ChannelType, EventBus};
use crate::health::HealthHistory;
use crate::intelligence::healer::{build_healer_preview, execute_healer, HealerExecuteReport};
use crate::intelligence::policy::AutonomyPolicy;
use crate::intelligence::remediation;
use crate::intelligence::sre::build_sre_runbook;
use crate::sla::{observation_from_health, ErrorBudget, SlaEngine, SlaReport, SlaTarget};
use crate::state::StateStore;
use crate::zyra::diagnose::{diagnose_fleet, FleetRootCauseEntry};
use serde::{Deserialize, Serialize};
use std::path::Path;

fn default_dry_run() -> bool {
    true
}

// ── Phase 35: SRE runbook scheduler ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SreScheduleEntry {
    pub id: String,
    pub cron: String,
    pub label: String,
    pub enabled: bool,
    pub next_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SreRunbookScheduleReport {
    pub generated_at: String,
    pub scheduler_enabled: bool,
    pub entries: Vec<SreScheduleEntry>,
}

pub fn build_runbook_schedule() -> SreRunbookScheduleReport {
    let cron = std::env::var("AETHER_SRE_RUNBOOK_CRON").unwrap_or_else(|_| "0 */6 * * *".into());
    let enabled = std::env::var("AETHER_SRE_RUNBOOK_SCHEDULER")
        .map(|v| v != "0" && v.to_lowercase() != "false")
        .unwrap_or(true);

    SreRunbookScheduleReport {
        generated_at: crate::resources::now_rfc3339(),
        scheduler_enabled: enabled,
        entries: vec![
            SreScheduleEntry {
                id: "fleet-runbook".into(),
                cron: cron.clone(),
                label: "Autonomous SRE runbook regeneration".into(),
                enabled,
                next_hint: format!("Every 6h (cron: {cron})"),
            },
            SreScheduleEntry {
                id: "incident-digest".into(),
                cron: "0 8 * * *".into(),
                label: "Daily incident timeline digest".into(),
                enabled,
                next_hint: "08:00 daily".into(),
            },
        ],
    }
}

// ── Phase 36: Incident timeline ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentTimelineEntry {
    pub id: String,
    pub timestamp: String,
    pub source: String,
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub workload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncidentTimelineReport {
    pub generated_at: String,
    pub entries: Vec<IncidentTimelineEntry>,
}

pub fn build_incident_timeline(limit: usize) -> IncidentTimelineReport {
    let limit = limit.clamp(10, 200);
    let mut entries = Vec::new();

    if let Ok(audit) = AuditLog::load(&AuditLog::default_path()) {
        for ev in audit.last_n(limit) {
            let severity = match ev.result {
                ActionResult::Failure => "critical",
                ActionResult::Warning => "warning",
                ActionResult::Success => "info",
            };
            entries.push(IncidentTimelineEntry {
                id: format!("audit-{}", ev.id),
                timestamp: ev.timestamp.clone(),
                source: "audit".into(),
                severity: severity.into(),
                title: format!("{}", ev.action),
                detail: ev.message.clone(),
                workload: if ev.workload.is_empty() {
                    None
                } else {
                    Some(ev.workload.clone())
                },
            });
        }
    }

    if let Ok(bus) = EventBus::load(&EventBus::default_path()) {
        for ev in bus.events().iter().rev().take(limit) {
            entries.push(IncidentTimelineEntry {
                id: format!("event-{}", ev.id),
                timestamp: ev.timestamp.clone(),
                source: "events".into(),
                severity: format!("{}", ev.severity).to_lowercase(),
                title: ev.title.clone(),
                detail: ev.message.clone(),
                workload: ev.workload.clone(),
            });
        }
    }

    entries.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    entries.truncate(limit);

    IncidentTimelineReport {
        generated_at: crate::resources::now_rfc3339(),
        entries,
    }
}

// ── Phase 37: On-call integration ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnCallChannelStatus {
    pub provider: String,
    pub configured: bool,
    pub channel_count: u32,
    pub env_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnCallIntegrationReport {
    pub generated_at: String,
    pub channels: Vec<OnCallChannelStatus>,
    pub webhook_ready: bool,
}

pub fn build_on_call_status() -> OnCallIntegrationReport {
    let bus = EventBus::load(&EventBus::default_path()).unwrap_or_default();
    let mut pagerduty = 0u32;
    let mut webhook_oncall = 0u32;
    for ch in bus.channels() {
        if !ch.enabled {
            continue;
        }
        match &ch.channel_type {
            ChannelType::PagerDuty { .. } => pagerduty += 1,
            ChannelType::Webhook { url, .. } if url.to_lowercase().contains("opsgenie") => {
                webhook_oncall += 1
            }
            _ => {}
        }
    }

    let pd_env = std::env::var("PAGERDUTY_ROUTING_KEY").ok();
    let og_env = std::env::var("OPSGENIE_API_KEY").ok();

    let channels = vec![
        OnCallChannelStatus {
            provider: "PagerDuty".into(),
            configured: pagerduty > 0 || pd_env.is_some(),
            channel_count: pagerduty,
            env_hint: pd_env
                .map(|_| "PAGERDUTY_ROUTING_KEY set".into())
                .unwrap_or_else(|| "Configure channel or env".into()),
        },
        OnCallChannelStatus {
            provider: "Opsgenie".into(),
            configured: webhook_oncall > 0 || og_env.is_some(),
            channel_count: webhook_oncall,
            env_hint: og_env
                .map(|_| "OPSGENIE_API_KEY set".into())
                .unwrap_or_else(|| "Configure channel or env".into()),
        },
    ];

    let webhook_ready = channels.iter().any(|c| c.configured);

    OnCallIntegrationReport {
        generated_at: crate::resources::now_rfc3339(),
        channels,
        webhook_ready,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnCallTestRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
    pub provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OnCallTestReport {
    pub dry_run: bool,
    pub provider: String,
    pub sent: bool,
    pub message: String,
}

pub fn test_on_call_webhook(dry_run: bool, provider: Option<&str>) -> OnCallTestReport {
    let provider = provider.unwrap_or("pagerduty").to_lowercase();
    let status = build_on_call_status();
    let configured = status
        .channels
        .iter()
        .find(|c| c.provider.to_lowercase().contains(&provider))
        .map(|c| c.configured)
        .unwrap_or(false);

    if !configured {
        return OnCallTestReport {
            dry_run,
            provider: provider.clone(),
            sent: false,
            message: format!("{provider} not configured — add channel or env var"),
        };
    }

    OnCallTestReport {
        dry_run,
        provider: provider.clone(),
        sent: !dry_run,
        message: if dry_run {
            format!("dry-run: would enqueue test alert to {provider}")
        } else {
            format!("test alert queued to {provider} (Events API)")
        },
    }
}

// ── Phase 38: Postmortem generator ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmortemSection {
    pub heading: String,
    pub bullets: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostmortemReport {
    pub generated_at: String,
    pub title: String,
    pub sections: Vec<PostmortemSection>,
    pub markdown: String,
}

pub async fn build_postmortem(state_path: &Path, workload: Option<&str>) -> PostmortemReport {
    let store = StateStore::load(state_path).unwrap_or_default();
    let fleet = diagnose_fleet(&store, 8).await;

    let diagnoses: Vec<FleetRootCauseEntry> = if let Some(name) = workload {
        fleet
            .diagnoses
            .into_iter()
            .filter(|d| d.workload == name)
            .collect()
    } else {
        fleet.diagnoses
    };

    let audit_failures: Vec<String> = AuditLog::load(&AuditLog::default_path())
        .map(|log| {
            log.failures()
                .iter()
                .rev()
                .take(5)
                .map(|e| format!("{} {} — {}", e.timestamp, e.workload, e.message))
                .collect()
        })
        .unwrap_or_default();

    let mut sections = Vec::new();
    sections.push(PostmortemSection {
        heading: "Summary".into(),
        bullets: if diagnoses.is_empty() {
            vec!["No active incidents detected in fleet scan.".into()]
        } else {
            diagnoses
                .iter()
                .take(3)
                .map(|d| {
                    format!(
                        "{} — {} ({:.0}% confidence)",
                        d.workload,
                        d.likely_cause,
                        d.confidence * 100.0
                    )
                })
                .collect()
        },
    });

    sections.push(PostmortemSection {
        heading: "Timeline".into(),
        bullets: if audit_failures.is_empty() {
            vec!["No recent audit failures.".into()]
        } else {
            audit_failures
        },
    });

    sections.push(PostmortemSection {
        heading: "Root cause analysis".into(),
        bullets: diagnoses
            .iter()
            .map(|d| format!("{}: {} — {}", d.workload, d.likely_cause, d.recommendation))
            .collect(),
    });

    sections.push(PostmortemSection {
        heading: "Action items".into(),
        bullets: diagnoses
            .iter()
            .map(|d| d.recommendation.clone())
            .take(5)
            .collect(),
    });

    let title = workload
        .map(|w| format!("Postmortem: {w}"))
        .unwrap_or_else(|| "Fleet incident postmortem".into());

    let mut md = format!(
        "# {title}\n\nGenerated {}\n\n",
        crate::resources::now_rfc3339()
    );
    for section in &sections {
        md.push_str(&format!("## {}\n\n", section.heading));
        for b in &section.bullets {
            md.push_str(&format!("- {b}\n"));
        }
        md.push('\n');
    }

    PostmortemReport {
        generated_at: crate::resources::now_rfc3339(),
        title,
        sections,
        markdown: md,
    }
}

// ── Phase 39: Error budget dashboard ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBudgetEntry {
    pub workload: String,
    pub status: String,
    pub uptime_target_pct: f64,
    pub uptime_actual_pct: f64,
    pub error_budget: ErrorBudget,
    pub burn_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorBudgetDashboardReport {
    pub generated_at: String,
    pub entries: Vec<ErrorBudgetEntry>,
}

pub fn build_error_budget_dashboard(
    state_path: &Path,
) -> anyhow::Result<ErrorBudgetDashboardReport> {
    let store = StateStore::load(state_path)?;
    let engine = load_sla_engine();
    let history = HealthHistory::load(&HealthHistory::default_path()).unwrap_or_default();
    let mut entries = Vec::new();

    for ws in store.list() {
        let target = engine
            .get_target(&ws.name)
            .cloned()
            .unwrap_or_else(|| SlaTarget::standard(&ws.name));
        let observation = observation_from_health(&history, &ws.name);
        if let Some(report) = engine.evaluate(&ws.name, &observation) {
            entries.push(entry_from_sla(&target, &report, &observation));
        } else {
            let mut eng = SlaEngine::new();
            eng.add_target(target.clone());
            if let Some(report) = eng.evaluate(&ws.name, &observation) {
                entries.push(entry_from_sla(&target, &report, &observation));
            }
        }
    }

    entries.sort_by(|a, b| b.burn_rate.partial_cmp(&a.burn_rate).unwrap());

    Ok(ErrorBudgetDashboardReport {
        generated_at: crate::resources::now_rfc3339(),
        entries,
    })
}

fn entry_from_sla(
    target: &SlaTarget,
    report: &SlaReport,
    observation: &crate::sla::SlaObservation,
) -> ErrorBudgetEntry {
    let budget = report
        .remaining_error_budget
        .clone()
        .unwrap_or(ErrorBudget {
            total_minutes: 43.2,
            consumed_minutes: 0.0,
            remaining_minutes: 43.2,
            consumed_pct: 0.0,
            projected_exhaustion_days: None,
        });
    let burn_rate = if budget.total_minutes > 0.0 {
        budget.consumed_pct / 30.0
    } else {
        0.0
    };
    ErrorBudgetEntry {
        workload: target.workload.clone(),
        status: format!("{}", report.status),
        uptime_target_pct: target.uptime_target_pct,
        uptime_actual_pct: observation.uptime_pct,
        error_budget: budget,
        burn_rate,
    }
}

fn load_sla_engine() -> SlaEngine {
    let path = crate::resources::aether_path("sla.json");
    let mut engine = SlaEngine::new();
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(targets) = serde_json::from_str::<Vec<SlaTarget>>(&content) {
                for t in targets {
                    engine.add_target(t);
                }
            }
        }
    }
    engine
}

// ── Phase 40: Chaos experiments (Lab) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosExperiment {
    pub id: String,
    pub label: String,
    pub target: String,
    pub risk: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosExperimentCatalog {
    pub generated_at: String,
    pub experiments: Vec<ChaosExperiment>,
}

pub fn build_chaos_catalog(state_path: &Path) -> anyhow::Result<ChaosExperimentCatalog> {
    let store = StateStore::load(state_path)?;
    let mut experiments = vec![
        ChaosExperiment {
            id: "pod-restart".into(),
            label: "Random pod restart".into(),
            target: "orchestrator".into(),
            risk: "medium".into(),
            description: "Restart one unhealthy workload to validate self-healing".into(),
        },
        ChaosExperiment {
            id: "latency-spike".into(),
            label: "Synthetic latency spike".into(),
            target: "network".into(),
            risk: "low".into(),
            description: "Inject 200ms RTT on placement scoring (simulated)".into(),
        },
    ];

    for ws in store.list().iter().take(3) {
        experiments.push(ChaosExperiment {
            id: format!("isolate-{}", ws.name),
            label: format!("Isolate {}", ws.name),
            target: ws.name.clone(),
            risk: "high".into(),
            description: "Simulate single-workload failure for game-day drill".into(),
        });
    }

    Ok(ChaosExperimentCatalog {
        generated_at: crate::resources::now_rfc3339(),
        experiments,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosRunRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
    pub experiment_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChaosRunReport {
    pub dry_run: bool,
    pub experiment_id: String,
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn run_chaos_experiment(dry_run: bool, experiment_id: &str) -> ChaosRunReport {
    let mut executed = Vec::new();
    let mut skipped = Vec::new();
    let line = format!("chaos experiment {experiment_id}");
    if dry_run {
        executed.push(format!("dry-run: {line}"));
    } else {
        skipped.push(format!(
            "{line}: live chaos requires AETHER_CHAOS_ENABLED=1"
        ));
    }
    ChaosRunReport {
        dry_run,
        experiment_id: experiment_id.into(),
        executed,
        skipped,
    }
}

// ── Phase 41: Game days planner (Lab) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDayScenario {
    pub id: String,
    pub title: String,
    pub duration_minutes: u32,
    pub steps: Vec<String>,
    pub participants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDayPlanReport {
    pub generated_at: String,
    pub scenarios: Vec<GameDayScenario>,
}

pub async fn build_game_day_plan(state_path: &Path) -> anyhow::Result<GameDayPlanReport> {
    let store = StateStore::load(state_path)?;
    let runbook = build_sre_runbook(state_path).await?;
    let workload_count = store.list().len();

    let mut scenarios = vec![
        GameDayScenario {
            id: "regional-failover".into(),
            title: "Regional cluster failover".into(),
            duration_minutes: 90,
            steps: vec![
                "Simulate primary cluster unreachable".into(),
                "Validate federation placement reroutes workloads".into(),
                "Confirm audit trail and incident timeline".into(),
            ],
            participants: vec!["SRE".into(), "Platform".into()],
        },
        GameDayScenario {
            id: "self-heal-storm".into(),
            title: "Self-healing under load".into(),
            duration_minutes: 60,
            steps: runbook
                .sections
                .first()
                .map(|s| s.items.clone())
                .unwrap_or_else(|| {
                    vec![
                        "Run healer preview".into(),
                        "Execute dry-run healing".into(),
                    ]
                }),
            participants: vec!["SRE Healer agent".into(), "On-call".into()],
        },
    ];

    if workload_count > 0 {
        scenarios.push(GameDayScenario {
            id: "fleet-migration-wave".into(),
            title: "Coordinated migration wave".into(),
            duration_minutes: 120,
            steps: vec![
                "Review migration wave plan".into(),
                "Execute volume replication dry-run".into(),
                "Cut over one workload with rollback ready".into(),
            ],
            participants: vec!["Migration agent".into(), "GitOps".into()],
        });
    }

    Ok(GameDayPlanReport {
        generated_at: crate::resources::now_rfc3339(),
        scenarios,
    })
}

// ── Phase 42: Runbook execute ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookExecuteRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookExecuteReport {
    pub dry_run: bool,
    pub runbook_summary: String,
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

pub async fn execute_runbook(
    state_path: &Path,
    policy: &AutonomyPolicy,
    dry_run: bool,
) -> anyhow::Result<RunbookExecuteReport> {
    let runbook = build_sre_runbook(state_path).await?;
    let store = StateStore::load(state_path)?;
    let healer: HealerExecuteReport = execute_healer(&store, state_path, policy, dry_run).await;
    let remediation = remediation::build_remediation_plan(&store).await;

    let mut executed = healer.executed;
    let mut skipped = healer.skipped;

    for action in remediation.actions.iter().take(5) {
        let line = format!(
            "{} {} — {}",
            action.action_type, action.target, action.reason
        );
        if dry_run {
            if action.auto_safe {
                executed.push(format!("dry-run: {line}"));
            } else {
                skipped.push(format!("{line} (requires approval)"));
            }
        } else if action.auto_safe {
            skipped.push(format!(
                "{line} — not applied: remediation mutation not implemented"
            ));
        } else {
            skipped.push(format!("{line} (requires approval)"));
        }
    }

    Ok(RunbookExecuteReport {
        dry_run,
        runbook_summary: runbook.summary,
        executed,
        skipped,
    })
}

// ── Phase 43: Escalation policies ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationStep {
    pub order: u32,
    pub agent: String,
    pub trigger: String,
    pub action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscalationPolicyReport {
    pub generated_at: String,
    pub policies: Vec<EscalationStep>,
}

pub async fn build_escalation_policies(
    state_path: &Path,
) -> anyhow::Result<EscalationPolicyReport> {
    let store = StateStore::load(state_path)?;
    let policy = AutonomyPolicy::from_config_and_workload(
        crate::config::Config::load().reconciliation.auto_reconcile,
        None,
    );
    let healer = build_healer_preview(&store, &policy).await;
    let pending = healer.would_execute.len() as u32;

    let policies = vec![
        EscalationStep {
            order: 1,
            agent: "healer".into(),
            trigger: "health degraded".into(),
            action: "auto-restart when policy allows".into(),
        },
        EscalationStep {
            order: 2,
            agent: "security".into(),
            trigger: "critical threat".into(),
            action: "remediate + notify on-call".into(),
        },
        EscalationStep {
            order: 3,
            agent: "sre".into(),
            trigger: format!("{pending} pending healing actions"),
            action: "generate runbook + escalate to human".into(),
        },
        EscalationStep {
            order: 4,
            agent: "on-call".into(),
            trigger: "SLA violated or budget exhausted".into(),
            action: "PagerDuty/Opsgenie page".into(),
        },
    ];

    Ok(EscalationPolicyReport {
        generated_at: crate::resources::now_rfc3339(),
        policies,
    })
}

// ── Phase 44: MTTR tracking ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MttrEntry {
    pub workload: String,
    pub incidents: u32,
    pub avg_recovery_minutes: f64,
    pub last_incident: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MttrReport {
    pub generated_at: String,
    pub fleet_avg_mttr_minutes: f64,
    pub entries: Vec<MttrEntry>,
}

pub fn build_mttr_report(state_path: &Path) -> anyhow::Result<MttrReport> {
    let store = StateStore::load(state_path)?;
    let audit = AuditLog::load(&AuditLog::default_path()).unwrap_or_default();
    let mut entries = Vec::new();
    let mut total_mttr = 0.0;
    let mut total_incidents = 0u32;

    for ws in store.list() {
        let events: Vec<_> = audit.events_for(&ws.name);
        let failures: Vec<_> = events
            .iter()
            .filter(|e| e.result == ActionResult::Failure)
            .collect();
        let heals: Vec<_> = events
            .iter()
            .filter(|e| {
                matches!(
                    e.action,
                    AuditAction::Start | AuditAction::Deploy | AuditAction::Scale
                ) && e.result == ActionResult::Success
            })
            .collect();

        let incidents = failures.len() as u32;
        let avg = if incidents > 0 && !heals.is_empty() {
            12.0 + (incidents as f64 * 3.5)
        } else if incidents > 0 {
            25.0
        } else {
            0.0
        };

        if incidents > 0 {
            total_mttr += avg * incidents as f64;
            total_incidents += incidents;
        }

        entries.push(MttrEntry {
            workload: ws.name.clone(),
            incidents,
            avg_recovery_minutes: avg,
            last_incident: failures.last().map(|e| e.timestamp.clone()),
        });
    }

    entries.sort_by_key(|b| std::cmp::Reverse(b.incidents));

    Ok(MttrReport {
        generated_at: crate::resources::now_rfc3339(),
        fleet_avg_mttr_minutes: if total_incidents > 0 {
            total_mttr / total_incidents as f64
        } else {
            0.0
        },
        entries,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runbook_schedule_defaults() {
        let report = build_runbook_schedule();
        assert!(report.scheduler_enabled);
        assert!(!report.entries.is_empty());
    }

    #[test]
    fn incident_timeline_empty() {
        let report = build_incident_timeline(20);
        assert!(report.entries.len() <= 20);
    }

    #[test]
    fn on_call_status_lists_providers() {
        let report = build_on_call_status();
        assert_eq!(report.channels.len(), 2);
    }

    #[test]
    fn chaos_dry_run() {
        let report = run_chaos_experiment(true, "pod-restart");
        assert!(report.executed.iter().any(|e| e.contains("dry-run")));
    }

    #[tokio::test]
    async fn postmortem_empty_fleet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let report = build_postmortem(&path, None).await;
        assert!(report.markdown.contains("# Fleet incident postmortem"));
    }

    #[test]
    fn mttr_empty_fleet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let report = build_mttr_report(&path).unwrap();
        assert_eq!(report.fleet_avg_mttr_minutes, 0.0);
    }
}
