// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Zyra & LLM platform — Era G (phases 65–74).

use crate::intelligence::intent_os::{scan_intent_violations, IntentViolationsReport};
use crate::rbac::Role;
use crate::zyra::policy::{role_allows_tool, tool_risk, ToolRisk};
use crate::zyra::tools::tools_openai_schema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn zyra_audit_path() -> PathBuf {
    crate::resources::aether_path("zyra-audit.jsonl")
}

#[allow(dead_code)]
fn zyra_memory_path() -> PathBuf {
    crate::resources::aether_path("zyra-memory.json")
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
pub struct VoiceZyraLabReport {
    pub status: String,
    pub hint: String,
    pub supported: bool,
    pub sample_transcript: String,
}

pub fn build_voice_zyra_lab() -> VoiceZyraLabReport {
    VoiceZyraLabReport {
        status: "lab".into(),
        hint: "Use browser SpeechRecognition in the dashboard or pipe audio transcripts to POST /api/zyra/chat.".into(),
        supported: true,
        sample_transcript: "Why is the web workload unhealthy?".into(),
    }
}

// ── Phase 67: Zyra memory (delegates to zyra::memory) ───────────────────────

pub use crate::zyra::memory::{
    read_zyra_memory, write_zyra_memory_entry, MemoryKind, MemoryScope, ZyraMemoryEntry,
    ZyraMemoryReport, ZyraMemorySettings,
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

pub fn route_zyra_agent(message: &str) -> MultiAgentRouteReport {
    let d = crate::zyra::routing::route_message(message, None);
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
    let status = crate::zyra::providers::build_provider_status();
    LlmProviderStatusReport {
        active_provider: status.active_provider,
        openai_configured: status.providers.iter().any(|p| {
            p.enabled && matches!(p.kind, crate::zyra::providers::ZyraProviderKind::Openai)
        }),
        anthropic_configured: status.providers.iter().any(|p| {
            p.enabled && matches!(p.kind, crate::zyra::providers::ZyraProviderKind::Anthropic)
        }),
        ollama_configured: status.providers.iter().any(|p| {
            p.enabled && matches!(p.kind, crate::zyra::providers::ZyraProviderKind::Ollama)
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

// ── Phase 73: Zyra audit trail ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraAuditEntry {
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
pub struct ZyraAuditReport {
    pub entries: Vec<ZyraAuditEntry>,
}

pub fn append_zyra_audit(entry: ZyraAuditEntry) -> anyhow::Result<()> {
    if let Some(parent) = zyra_audit_path().parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(zyra_audit_path())?;
    writeln!(file, "{}", serde_json::to_string(&entry)?)?;
    Ok(())
}

pub fn read_zyra_audit(limit: usize) -> ZyraAuditReport {
    let path = zyra_audit_path();
    if !path.exists() {
        return ZyraAuditReport {
            entries: Vec::new(),
        };
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut entries: Vec<ZyraAuditEntry> = raw
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }
    ZyraAuditReport { entries }
}

// ── Phase 74: RBAC scopes ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraToolScope {
    pub name: String,
    pub risk: String,
    pub allowed_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraRbacScopesReport {
    pub role: String,
    pub can_execute_mutations: bool,
    pub tools: Vec<ZyraToolScope>,
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

pub fn build_zyra_rbac_scopes(role: Role) -> ZyraRbacScopesReport {
    let schema = tools_openai_schema();
    let mut tools = Vec::new();
    if let Some(arr) = schema.as_array() {
        for item in arr {
            if let Some(name) = item.pointer("/function/name").and_then(|v| v.as_str()) {
                let risk = tool_risk(name);
                tools.push(ZyraToolScope {
                    name: name.to_string(),
                    risk: format!("{risk:?}"),
                    allowed_roles: roles_for_risk(risk),
                });
            }
        }
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    ZyraRbacScopesReport {
        role: format!("{role:?}"),
        can_execute_mutations: role_allows_tool(&role, ToolRisk::Mutate),
        tools,
    }
}

// ── Zyra insights aggregator ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraInsightsReport {
    pub generated_at: String,
    pub summary: String,
    pub drift_workloads: usize,
    pub security_alerts: usize,
    pub cost_savings_usd: f64,
    pub pending_actions: u32,
    pub suggested_actions: Vec<String>,
}

pub async fn build_zyra_insights(state_path: &Path) -> anyhow::Result<ZyraInsightsReport> {
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
        "Fleet looks healthy — no urgent Zyra actions.".into()
    } else {
        actions.join(" · ")
    };
    Ok(ZyraInsightsReport {
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
        let r = route_zyra_agent("Find cost savings across the fleet");
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
