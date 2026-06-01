// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Zeus agent loop.

use crate::intelligence::zeus_os::{append_zeus_audit, ZeusAuditEntry};
use crate::zeus::memory::write_session_memory;
use crate::zeus::policy::{role_allows_execute, tool_risk};
use crate::zeus::provider::{ChatMessage, LlmProvider};
use crate::zeus::providers::{load_registry, provider_from_registry};
use crate::zeus::routing::{resolve_provider_for_routing, route_message, RoutingDecision};
use crate::zeus::agents::agent_system_prompt;
use crate::zeus::session::{PendingAction, ZeusSession, ZeusSessionStore};
use crate::zeus::tools::{execute_tool, tools_openai_schema, ToolContext};
use crate::rbac::Role;
use crate::state::StateStore;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeusChatResponse {
    pub session_id: String,
    pub reply: String,
    pub tool_results: Vec<ToolResultSummary>,
    pub pending_actions: Vec<PendingAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultSummary {
    pub tool: String,
    pub summary: String,
}

pub struct ZeusAgent {
    sessions: ZeusSessionStore,
}

impl ZeusAgent {
    pub fn new() -> Self {
        Self {
            sessions: ZeusSessionStore::default(),
        }
    }

    fn resolve_provider(&self, routing: &RoutingDecision) -> Box<dyn LlmProvider> {
        let reg = load_registry();
        let kind = resolve_provider_for_routing(routing, &reg);
        provider_from_registry(Some(kind))
    }

    pub async fn chat(
        &self,
        message: &str,
        session_id: Option<&str>,
        confirm_action_id: Option<&str>,
        agent_focus: Option<&str>,
        ctx: &ToolContext,
    ) -> anyhow::Result<ZeusChatResponse> {
        let routing = route_message(message, agent_focus);
        let provider = self.resolve_provider(&routing);

        let mut session = self.sessions.get_or_create(session_id);
        if session.messages.is_empty() {
            session.messages.push(ChatMessage {
                role: "system".into(),
                content: format!(
                    "{}\n\nActive specialist: {} ({})",
                    agent_system_prompt(&routing.agent_id),
                    routing.agent_label,
                    routing.rationale
                ),
            });
        }
        session.messages.push(ChatMessage {
            role: "user".into(),
            content: message.into(),
        });

        if let Some(action_id) = confirm_action_id {
            if let Some(idx) = session.pending_actions.iter().position(|a| a.id == action_id) {
                let action = session.pending_actions.remove(idx);
                if role_allows_execute(&ctx.role, true) {
                    let result = execute_tool(ctx, &action.tool, &action.arguments).await?;
                    session.messages.push(ChatMessage {
                        role: "assistant".into(),
                        content: format!("Executed {}: {}", action.tool, result),
                    });
                    let _ = append_zeus_audit(ZeusAuditEntry {
                        timestamp: crate::resources::now_rfc3339(),
                        session_id: session.id.clone(),
                        action: "confirm".into(),
                        tool: Some(action.tool.clone()),
                        role: format!("{:?}", ctx.role),
                        detail: action.description.clone(),
                        agent: None,
                        provider: None,
                        model: None,
                    });
                }
            }
        }

        let tools = tools_openai_schema();
        let llm_resp = provider.chat(&session.messages, &tools).await?;

        let mut tool_results = Vec::new();
        let mut pending_actions = Vec::new();

        for tc in &llm_resp.tool_calls {
            let risk = tool_risk(&tc.name);
            if risk == crate::zeus::policy::ToolRisk::Mutate {
                let pa = PendingAction {
                    id: tc.id.clone(),
                    tool: tc.name.clone(),
                    arguments: tc.arguments.clone(),
                    description: format!("Confirm {} with {:?}", tc.name, tc.arguments),
                };
                pending_actions.push(pa);
                continue;
            }
            match execute_tool(ctx, &tc.name, &tc.arguments).await {
                Ok(val) => {
                    let summary = summarize_tool_result(&tc.name, &val);
                    tool_results.push(ToolResultSummary {
                        tool: tc.name.clone(),
                        summary: summary.clone(),
                    });
                    session.messages.push(ChatMessage {
                        role: "assistant".into(),
                        content: summary,
                    });
                }
                Err(e) => {
                    tool_results.push(ToolResultSummary {
                        tool: tc.name.clone(),
                        summary: format!("Error: {e}"),
                    });
                }
            }
        }

        session.pending_actions.extend(pending_actions.clone());
        let reply = if llm_resp.content.is_empty() && !tool_results.is_empty() {
            tool_results
                .iter()
                .map(|t| t.summary.as_str())
                .collect::<Vec<_>>()
                .join("\n\n")
        } else if llm_resp.content.is_empty() && !pending_actions.is_empty() {
            "Action requires confirmation. Call confirm with action_id.".into()
        } else {
            llm_resp.content
        };

        session.messages.push(ChatMessage {
            role: "assistant".into(),
            content: reply.clone(),
        });
        session.updated_at = crate::resources::now_rfc3339();
        self.sessions.save(session.clone());

        let _ = write_session_memory(
            &session.id,
            &reply.chars().take(240).collect::<String>(),
            serde_json::json!({
                "message_count": session.messages.len(),
                "agent": routing.agent_id,
                "provider_kind": format!("{:?}", routing.provider_kind),
            }),
        );
        let _ = append_zeus_audit(ZeusAuditEntry {
            timestamp: crate::resources::now_rfc3339(),
            session_id: session.id.clone(),
            action: "chat".into(),
            tool: tool_results.first().map(|t| t.tool.clone()),
            role: format!("{:?}", ctx.role),
            detail: message.chars().take(120).collect(),
            agent: Some(routing.agent_id.clone()),
            provider: Some(format!("{:?}", routing.provider_kind)),
            model: Some(routing.model_hint.clone()),
        });

        Ok(ZeusChatResponse {
            session_id: session.id,
            reply,
            tool_results,
            pending_actions,
        })
    }

    pub fn get_session(&self, id: &str) -> Option<ZeusSession> {
        self.sessions.get(id)
    }

    pub async fn confirm_batch(
        &self,
        session_id: &str,
        action_ids: &[String],
        ctx: &ToolContext,
    ) -> anyhow::Result<crate::intelligence::zeus_os::BatchConfirmReport> {
        let mut session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| anyhow::anyhow!("session not found"))?;

        let mut confirmed = Vec::new();
        let mut skipped = Vec::new();
        let mut errors = Vec::new();

        for action_id in action_ids {
            if let Some(idx) = session.pending_actions.iter().position(|a| a.id == *action_id) {
                let action = session.pending_actions.remove(idx);
                if role_allows_execute(&ctx.role, true) {
                    match execute_tool(ctx, &action.tool, &action.arguments).await {
                        Ok(result) => {
                            session.messages.push(ChatMessage {
                                role: "assistant".into(),
                                content: format!("Executed {}: {}", action.tool, result),
                            });
                            confirmed.push(action_id.clone());
                            let _ = append_zeus_audit(ZeusAuditEntry {
                                timestamp: crate::resources::now_rfc3339(),
                                session_id: session.id.clone(),
                                action: "batch_confirm".into(),
                                tool: Some(action.tool),
                                role: format!("{:?}", ctx.role),
                                detail: action.description,
                                agent: None,
                                provider: None,
                                model: None,
                            });
                        }
                        Err(e) => errors.push(format!("{action_id}: {e}")),
                    }
                } else {
                    skipped.push(action_id.clone());
                }
            } else {
                skipped.push(action_id.clone());
            }
        }

        session.updated_at = crate::resources::now_rfc3339();
        self.sessions.save(session);

        Ok(crate::intelligence::zeus_os::BatchConfirmReport {
            session_id: session_id.to_string(),
            confirmed,
            skipped,
            errors,
        })
    }
}

impl Default for ZeusAgent {
    fn default() -> Self {
        Self::new()
    }
}

fn summarize_tool_result(name: &str, val: &serde_json::Value) -> String {
    match name {
        "list_workloads" => {
            let n = val.pointer("/workloads").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
            format!("Found {n} workload(s).")
        }
        "context_snapshot" => {
            let n = val.get("workload_count").and_then(|v| v.as_u64()).unwrap_or(0);
            format!("Platform snapshot: {n} workloads.")
        }
        "explain_health" => val.to_string(),
        "predictions" => {
            let risk = val.get("fleet_risk_score").and_then(|v| v.as_f64()).unwrap_or(0.0);
            format!("Fleet risk score: {:.2}", risk)
        }
        "cost_summary" => {
            let pct = val
                .get("total_potential_savings_pct")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            format!("Potential savings: {:.1}%", pct)
        }
        "diagnose_workload" => {
            let level = val.get("health_level").and_then(|v| v.as_str()).unwrap_or("unknown");
            let summary = val.get("summary").and_then(|v| v.as_str()).unwrap_or("");
            format!("Diagnosis: {level} — {summary}")
        }
        "ai_insights" => "Combined AI insights ready (predictions, threats, cost).".into(),
        "policy_violations" => {
            let n = val.pointer("/violations").and_then(|v| v.as_array()).map(|a| a.len()).unwrap_or(0);
            format!("Found {n} workload(s) with policy violations.")
        }
        "gitops_status" => val.to_string(),
        _ => val.to_string(),
    }
}

pub fn tool_context(
    state: Arc<RwLock<StateStore>>,
    state_path: PathBuf,
    role: Role,
) -> ToolContext {
    ToolContext {
        state,
        state_path,
        role,
    }
}
