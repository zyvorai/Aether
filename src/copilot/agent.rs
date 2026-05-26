//! Copilot agent loop.

use crate::copilot::policy::{role_allows_execute, tool_risk};
use crate::copilot::provider::{provider_from_env, ChatMessage, LlmProvider};
use crate::copilot::session::{CopilotSession, CopilotSessionStore, PendingAction};
use crate::copilot::tools::{execute_tool, tools_openai_schema, ToolContext};
use crate::rbac::Role;
use crate::state::StateStore;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotChatResponse {
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

pub struct CopilotAgent {
    sessions: CopilotSessionStore,
    provider: Box<dyn LlmProvider>,
}

impl CopilotAgent {
    pub fn new() -> Self {
        Self {
            sessions: CopilotSessionStore::default(),
            provider: provider_from_env(),
        }
    }

    pub async fn chat(
        &self,
        message: &str,
        session_id: Option<&str>,
        confirm_action_id: Option<&str>,
        ctx: &ToolContext,
    ) -> anyhow::Result<CopilotChatResponse> {
        let mut session = self.sessions.get_or_create(session_id);
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
                }
            }
        }

        let tools = tools_openai_schema();
        let llm_resp = self.provider.chat(&session.messages, &tools).await?;

        let mut tool_results = Vec::new();
        let mut pending_actions = Vec::new();

        for tc in &llm_resp.tool_calls {
            let risk = tool_risk(&tc.name);
            if risk == crate::copilot::policy::ToolRisk::Mutate {
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

        Ok(CopilotChatResponse {
            session_id: session.id,
            reply,
            tool_results,
            pending_actions,
        })
    }

    pub fn get_session(&self, id: &str) -> Option<CopilotSession> {
        self.sessions.get(id)
    }
}

impl Default for CopilotAgent {
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
