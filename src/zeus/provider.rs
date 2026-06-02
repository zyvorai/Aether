// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Pluggable LLM providers for Zeus.

use crate::zeus::providers::provider_from_registry;
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub content: String,
    pub tool_calls: Vec<ToolCallRequest>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools_json: &serde_json::Value,
    ) -> Result<LlmResponse>;
}

pub fn provider_from_env() -> Box<dyn LlmProvider> {
    provider_from_registry(None)
}

/// Rule-based fallback when no LLM API key is configured.
pub struct RuleBasedProvider;

#[async_trait]
impl LlmProvider for RuleBasedProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        _tools_json: &serde_json::Value,
    ) -> Result<LlmResponse> {
        let last = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.to_lowercase())
            .unwrap_or_default();

        let (name, args) =
            if last.contains("health") || last.contains("unhealthy") || last.contains("why") {
                ("explain_health", serde_json::json!({}))
            } else if last.contains("workload") || last.contains("list") {
                ("list_workloads", serde_json::json!({}))
            } else if last.contains("drift") {
                ("check_drift", serde_json::json!({}))
            } else if last.contains("cluster") {
                ("cluster_summary", serde_json::json!({}))
            } else if last.contains("cost") {
                ("cost_summary", serde_json::json!({}))
            } else if last.contains("runtime") || last.contains("recommend") {
                ("recommend_runtime", serde_json::json!({}))
            } else {
                ("context_snapshot", serde_json::json!({}))
            };

        Ok(LlmResponse {
            content: String::new(),
            tool_calls: vec![ToolCallRequest {
                id: "rule-1".into(),
                name: name.into(),
                arguments: args,
            }],
        })
    }
}
