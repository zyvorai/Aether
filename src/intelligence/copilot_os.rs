// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Copilot & LLM platform — Era G (phases 65–74).

use crate::copilot::policy::{role_allows_tool, tool_risk, ToolRisk};
use crate::copilot::tools::tools_openai_schema;
use crate::intelligence::intent_os::{scan_intent_violations, IntentViolationsReport};
use crate::rbac::Role;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn copilot_audit_path() -> PathBuf {
    crate::resources::aether_path("copilot-audit.jsonl")
}

fn copilot_memory_path() -> PathBuf {
    crate::resources::aether_path("copilot-memory.json")
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
pub struct VoiceCopilotLabReport {
    pub status: String,
    pub hint: String,
    pub supported: bool,
    pub sample_transcript: String,
}

pub fn build_voice_copilot_lab() -> VoiceCopilotLabReport {
    VoiceCopilotLabReport {
        status: "lab".into(),
        hint: "Use browser SpeechRecognition in the dashboard or pipe audio transcripts to POST /api/copilot/chat.".into(),
        supported: true,
        sample_transcript: "Why is the web workload unhealthy?".into(),
    }
}

// ── Phase 67: Copilot memory ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CopilotMemoryEntry {
    pub session_id: String,
    pub summary: String,
    pub fleet_context: serde_json::Value,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CopilotMemoryReport {
    pub entries: Vec<CopilotMemoryEntry>,
    pub persisted: bool,
}

pub fn read_copilot_memory() -> CopilotMemoryReport {
    let path = copilot_memory_path();
    if !path.exists() {
        return CopilotMemoryReport {
            entries: Vec::new(),
            persisted: false,
        };
    }
    match std::fs::read_to_string(&path) {
        Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
        Err(_) => CopilotMemoryReport::default(),
    }
}

pub fn write_copilot_memory_entry(entry: CopilotMemoryEntry) -> anyhow::Result<CopilotMemoryReport> {
    let mut report = read_copilot_memory();
    if let Some(existing) = report.entries.iter_mut().find(|e| e.session_id == entry.session_id) {
        *existing = entry;
    } else {
        report.entries.push(entry);
    }
    while report.entries.len() > 32 {
        report.entries.remove(0);
    }
    report.persisted = true;
    if let Some(parent) = copilot_memory_path().parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(copilot_memory_path(), serde_json::to_string_pretty(&report)?)?;
    Ok(report)
}

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

pub fn route_copilot_agent(message: &str) -> MultiAgentRouteReport {
    let lower = message.to_lowercase();
    if lower.contains("cost") || lower.contains("finops") || lower.contains("savings") {
        return MultiAgentRouteReport {
            agent: "cost".into(),
            label: "FinOps Agent".into(),
            confidence: 0.92,
            suggested_prompts: vec![
                "Find workloads wasting resources".into(),
                "Predict cost next month".into(),
            ],
        };
    }
    if lower.contains("security") || lower.contains("threat") || lower.contains("policy") {
        return MultiAgentRouteReport {
            agent: "security".into(),
            label: "Security Agent".into(),
            confidence: 0.9,
            suggested_prompts: vec![
                "Scan fleet for security threats".into(),
                "Explain policy violations".into(),
            ],
        };
    }
    if lower.contains("migrate") || lower.contains("placement") || lower.contains("runtime") {
        return MultiAgentRouteReport {
            agent: "migration".into(),
            label: "Migration Agent".into(),
            confidence: 0.88,
            suggested_prompts: vec![
                "Show migration opportunities".into(),
                "Recommend runtime placement".into(),
            ],
        };
    }
    if lower.contains("capacity") || lower.contains("scale") || lower.contains("saturation") {
        return MultiAgentRouteReport {
            agent: "capacity".into(),
            label: "Capacity Agent".into(),
            confidence: 0.86,
            suggested_prompts: vec![
                "Predict capacity risks this week".into(),
                "Recommend scaling actions".into(),
            ],
        };
    }
    if lower.contains("gitops") || lower.contains("drift") || lower.contains("sync") {
        return MultiAgentRouteReport {
            agent: "gitops".into(),
            label: "GitOps Agent".into(),
            confidence: 0.87,
            suggested_prompts: vec![
                "Check GitOps drift across fleet".into(),
                "Summarize reconciliation status".into(),
            ],
        };
    }
    MultiAgentRouteReport {
        agent: "sre".into(),
        label: "SRE Agent".into(),
        confidence: 0.75,
        suggested_prompts: vec![
            "Run fleet health diagnosis".into(),
            "Which workloads need healing?".into(),
        ],
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
    let openai = std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some();
    let anthropic = std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some();
    let ollama = std::env::var("AETHER_OLLAMA_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
        || std::env::var("AETHER_LLM_PROVIDER")
            .map(|s| s.eq_ignore_ascii_case("ollama"))
            .unwrap_or(false);
    let model = std::env::var("AETHER_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into());
    let active = if anthropic
        && std::env::var("AETHER_LLM_PROVIDER")
            .map(|s| s.eq_ignore_ascii_case("anthropic"))
            .unwrap_or(false)
    {
        "anthropic"
    } else if openai {
        "openai"
    } else if ollama {
        "ollama"
    } else {
        "rule-based"
    };
    LlmProviderStatusReport {
        active_provider: active.into(),
        openai_configured: openai,
        anthropic_configured: anthropic,
        ollama_configured: ollama,
        model,
        fallback_rule_based: !openai && !anthropic && !ollama,
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

// ── Phase 73: Copilot audit trail ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotAuditEntry {
    pub timestamp: String,
    pub session_id: String,
    pub action: String,
    pub tool: Option<String>,
    pub role: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotAuditReport {
    pub entries: Vec<CopilotAuditEntry>,
}

pub fn append_copilot_audit(entry: CopilotAuditEntry) -> anyhow::Result<()> {
    if let Some(parent) = copilot_audit_path().parent() {
        std::fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(copilot_audit_path())?;
    writeln!(file, "{}", serde_json::to_string(&entry)?)?;
    Ok(())
}

pub fn read_copilot_audit(limit: usize) -> CopilotAuditReport {
    let path = copilot_audit_path();
    if !path.exists() {
        return CopilotAuditReport {
            entries: Vec::new(),
        };
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut entries: Vec<CopilotAuditEntry> = raw
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }
    CopilotAuditReport { entries }
}

// ── Phase 74: RBAC scopes ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotToolScope {
    pub name: String,
    pub risk: String,
    pub allowed_roles: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotRbacScopesReport {
    pub role: String,
    pub can_execute_mutations: bool,
    pub tools: Vec<CopilotToolScope>,
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

pub fn build_copilot_rbac_scopes(role: Role) -> CopilotRbacScopesReport {
    let schema = tools_openai_schema();
    let mut tools = Vec::new();
    if let Some(arr) = schema.as_array() {
        for item in arr {
            if let Some(name) = item.pointer("/function/name").and_then(|v| v.as_str()) {
                let risk = tool_risk(name);
                tools.push(CopilotToolScope {
                    name: name.to_string(),
                    risk: format!("{risk:?}"),
                    allowed_roles: roles_for_risk(risk),
                });
            }
        }
    }
    tools.sort_by(|a, b| a.name.cmp(&b.name));
    CopilotRbacScopesReport {
        role: format!("{role:?}"),
        can_execute_mutations: role_allows_tool(&role, ToolRisk::Mutate),
        tools,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_cost_agent() {
        let r = route_copilot_agent("Find cost savings across the fleet");
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
