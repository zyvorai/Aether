// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Zeus & LLM platform — Era G (phases 65–74).

use crate::intelligence::intent_os::{scan_intent_violations, IntentViolationsReport};
use crate::rbac::Role;
use crate::zeus::policy::{role_allows_tool, tool_risk, ToolRisk};
use crate::zeus::tools::tools_openai_schema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn zeus_audit_path() -> PathBuf {
    crate::resources::aether_path("zeus-audit.jsonl")
}

#[allow(dead_code)]
fn zeus_memory_path() -> PathBuf {
    crate::resources::aether_path("zeus-memory.json")
}

// ── Phase 65: Batch confirm ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfirmReport {
    pub session_id: String,
    pub confirmed: Vec<String>,
    pub skipped: Vec<String>,
    pub errors: Vec<String>,
}

// ── Phase 66: Voice copilot (Lab) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceZeusLabReport {
    pub status: String,
    pub hint: String,
    pub supported: bool,
    pub sample_transcript: String,
}

pub fn build_voice_zeus_lab() -> VoiceZeusLabReport {
    VoiceZeusLabReport {
        status: "lab".into(),
        hint: "Use browser SpeechRecognition in the dashboard or pipe audio transcripts to POST /api/zeus/chat.".into(),
        supported: true,
        sample_transcript: "Why is the web workload unhealthy?".into(),
    }
}

// ── Phase 67: Zeus memory (delegates to zeus::memory) ───────────────────────

pub use crate::zeus::memory::{
    read_zeus_memory, write_zeus_memory_entry, MemoryKind, MemoryScope, ZeusMemoryEntry,
    ZeusMemoryReport, ZeusMemorySettings,
};

pub fn snapshot_fleet_memory(_state_path: &Path) -> serde_json::Value {
    serde_json::json!({ "note": "Fleet context captured on chat via API handler" })
}

// ── Phase 68: Multi-agent routing ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiAgentRouteReport {
    pub agent: String,
    pub label: String,
    pub confidence: f64,
    pub suggested_prompts: Vec<String>,
}

pub fn route_zeus_agent(message: &str) -> MultiAgentRouteReport {
    let d = crate::zeus::routing::route_message(message, None);
    MultiAgentRouteReport {
        agent: d.agent_id,
        label: d.agent_label,
        confidence: d.confidence,
        suggested_prompts: d.suggested_prompts,
    }
}

// ── Phase 69: LLM provider status ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProviderStatusReport {
    pub active_provider: String,
    pub openai_configured: bool,
    pub anthropic_configured: bool,
    pub ollama_configured: bool,
    pub model: String,
    pub fallback_rule_based: bool,
}

pub fn build_llm_provider_status() -> LlmProviderStatusReport {
    let status = crate::zeus::providers::build_provider_status();
    LlmProviderStatusReport {
        active_provider: status.active_provider,
        openai_configured: status.providers.iter().any(|p| {
            p.enabled && matches!(p.kind, crate::zeus::providers::ZeusProviderKind::Openai)
        }),
        anthropic_configured: status.providers.iter().any(|p| {
            p.enabled && matches!(p.kind, crate::zeus::providers::ZeusProviderKind::Anthropic)
        }),
        ollama_configured: status.providers.iter().any(|p| {
            p.enabled && matches!(p.kind, crate::zeus::providers::ZeusProviderKind::Ollama)
        }),
        model: status.active_model,
        fallback_rule_based: status.fallback_rule_based,
    }
}

// ── Phase 70: Runbook author ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookAuthorRequest {
    pub prompt: String,
    pub workload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunbookAuthorReport {
    pub title: String,
    pub markdown: String,
}

pub fn author_runbook(req: &RunbookAuthorRequest) -> RunbookAuthorReport {
    let title = if let Some(w) = req.workload.as_deref().filter(|s| !s.is_empty()) {
        format!("Runbook: {w}")
    } else {
        "Operational Runbook".into()
    };
    let workload_line = req
        .workload
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|w| format!("**Workload:** `{w}`\n\n"))
        .unwrap_or_default();
    let markdown = format!(
        "# {title}\n\n{workload_line}## Objective\n{}\n\n## Preconditions\n- API reachable (`aether serve` or systemd unit active)\n- Operator or Admin RBAC role\n\n## Steps\n1. Inspect fleet health and recent events.\n2. Validate workload spec and runtime placement.\n3. Apply remediation (restart, scale, or migrate) with dry-run first.\n4. Confirm health checks pass within 5 minutes.\n\n## Rollback\n- Restore last snapshot or revert GitOps commit.\n\n## Verification\n- `aether status <workload>` shows running/healthy\n- Dashboard health tab green\n",
        req.prompt.trim()
    );
    RunbookAuthorReport { title, markdown }
}

// ── Phase 71: Policy explainer ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyExplainerRequest {
    pub workload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyExplainerReport {
    pub summary: String,
    pub violations: IntentViolationsReport,
    pub plain_english: Vec<String>,
}

pub fn explain_policy_violations(
    state_path: &Path,
    req: &PolicyExplainerRequest,
) -> anyhow::Result<PolicyExplainerReport> {
    let violations = scan_intent_violations(state_path)?;
    let filtered: Vec<_> = violations
        .violations
        .iter()
        .filter(|v| {
            req.workload
                .as_deref()
                .filter(|w| !w.is_empty())
                .map(|w| v.workload == w)
                .unwrap_or(true)
        })
        .cloned()
        .collect();
    let plain_english: Vec<String> = filtered
        .iter()
        .map(|v| {
            format!(
                "Workload `{}` violates {} ({}): {}",
                v.workload, v.violation_type, v.severity, v.detail
            )
        })
        .collect();
    let summary = if plain_english.is_empty() {
        "No intent or OPA policy violations detected in the current fleet snapshot.".into()
    } else {
        format!(
            "{} workload(s) have policy or intent violations that may block deploy or reconcile.",
            plain_english.len()
        )
    };
    Ok(PolicyExplainerReport {
        summary,
        violations: IntentViolationsReport {
            generated_at: violations.generated_at,
            violations: filtered,
            reconciliation_status: violations.reconciliation_status,
        },
        plain_english,
    })
}

// ── Phase 73: Zeus audit trail ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusAuditEntry {
    pub timestamp: String,
    pub session_id: String,
    pub action: String,
    pub tool: Option<String>,
    pub role: String,
    pub detail: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusAuditReport {
    pub entries: Vec<ZeusAuditEntry>,
}

pub fn append_zeus_audit(entry: ZeusAuditEntry) -> anyhow::Result<()> {
    if let Some(parent) = zeus_audit_path().parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(zeus_audit_path())?;
    writeln!(file, "{}", serde_json::to_string(&entry)?)?;
    Ok(())
}

pub fn read_zeus_audit(limit: usize) -> ZeusAuditReport {
    let path = zeus_audit_path();
    if !path.exists() {
        return ZeusAuditReport {
            entries: Vec::new(),
        };
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut entries: Vec<ZeusAuditEntry> = raw
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }
    ZeusAuditReport { entries }
}

// ── Phase 74: RBAC scopes ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusToolScope {
    pub name: String,
    pub risk: String,
    pub allowed_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusRbacScopesReport {
    pub role: String,
    pub can_execute_mutations: bool,
    pub tools: Vec<ZeusToolScope>,
}

fn roles_for_risk(risk: ToolRisk) -> Vec<String> {
    match risk {
        ToolRisk::Read => ["Admin", "Operator", "Viewer"]
            .into_iter()
            .map(String::from)
            .collect(),
        ToolRisk::Mutate => ["Admin", "Operator"]
            .into_iter()
            .map(String::from)
            .collect(),
    }
}

pub fn build_zeus_rbac_scopes(role: Role) -> ZeusRbacScopesReport {
    let schema = tools_openai_schema();
    let mut tools = Vec::new();
    if let Some(arr) = schema.as_array() {
        for item in arr {
            if let Some(name) = item.pointer("/function/name").and_then(|v| v.as_str()) {
                let risk = tool_risk(name);
                tools.push(ZeusToolScope {
                    name: name.to_string(),
                    risk: format!("{risk:?}"),
                    allowed_roles: roles_for_risk(risk),
                });
            }
        }
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    ZeusRbacScopesReport {
        role: format!("{role:?}"),
        can_execute_mutations: role_allows_tool(&role, ToolRisk::Mutate),
        tools,
    }
}

// ── Zeus insights aggregator ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusInsightsReport {
    pub generated_at: String,
    pub summary: String,
    pub drift_workloads: usize,
    pub security_alerts: usize,
    pub cost_savings_usd: f64,
    pub pending_actions: u32,
    pub suggested_actions: Vec<String>,
}

pub async fn build_zeus_insights(state_path: &Path) -> anyhow::Result<ZeusInsightsReport> {
    let snap = crate::intelligence::context::build_context_snapshot(state_path).await?;
    let pairs: Vec<(crate::spec::Workload, crate::state::WorkloadState)> = {
        let store = crate::state::StateStore::load(state_path)?;
        store
            .list()
            .iter()
            .filter_map(|ws| {
                crate::spec::Workload::from_file(&ws.spec_path)
                    .ok()
                    .map(|s| (s, (*ws).clone()))
            })
            .collect()
    };
    let threats = crate::intelligence::security::SecurityEngine::scan_fleet(&pairs);
    let cost = crate::intelligence::finops::FinOpsEngine::optimize_fleet(&pairs);
    let savings: f64 = cost
        .recommendations
        .iter()
        .map(|r| r.savings_monthly_usd)
        .sum();
    let mut actions = Vec::new();
    if snap.drift_summary.workloads_with_drift > 0 {
        actions.push(format!(
            "Reconcile drift on {} workload(s)",
            snap.drift_summary.workloads_with_drift
        ));
    }
    if !threats.threats.is_empty() {
        actions.push(format!(
            "Review {} security threat(s)",
            threats.threats.len()
        ));
    }
    if savings > 0.0 {
        actions.push(format!("Save ${savings:.0}/mo via FinOps recommendations"));
    }
    let summary = if actions.is_empty() {
        "Fleet looks healthy — no urgent Zeus actions.".into()
    } else {
        actions.join(" · ")
    };
    Ok(ZeusInsightsReport {
        generated_at: crate::resources::now_rfc3339(),
        summary,
        drift_workloads: snap.drift_summary.workloads_with_drift,
        security_alerts: threats.threats.len(),
        cost_savings_usd: savings,
        pending_actions: 0,
        suggested_actions: actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_cost_agent() {
        let r = route_zeus_agent("Find cost savings across the fleet");
        assert_eq!(r.agent, "cost");
    }

    #[test]
    fn runbook_author_includes_prompt() {
        let r = author_runbook(&RunbookAuthorRequest {
            prompt: "Restart unhealthy pods".into(),
            workload: Some("web".into()),
        });
        assert!(r.markdown.contains("Restart unhealthy pods"));
        assert!(r.markdown.contains("web"));
    }

    #[test]
    fn llm_status_defaults_rule_based_without_keys() {
        let prev = std::env::var("OPENAI_API_KEY").ok();
        std::env::remove_var("OPENAI_API_KEY");
        let r = build_llm_provider_status();
        if prev.is_none() {
            assert!(r.fallback_rule_based || r.ollama_configured);
        }
        if let Some(k) = prev {
            std::env::set_var("OPENAI_API_KEY", k);
        }
    }
}
