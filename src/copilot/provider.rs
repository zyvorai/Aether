//! Pluggable LLM providers for the copilot.

use anyhow::{Context, Result};
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
    async fn chat(&self, messages: &[ChatMessage], tools_json: &serde_json::Value) -> Result<LlmResponse>;
}

pub fn provider_from_env() -> Box<dyn LlmProvider> {
    if std::env::var("AETHER_LLM_PROVIDER")
        .map(|s| s.eq_ignore_ascii_case("ollama"))
        .unwrap_or(false)
        || (std::env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()).is_none()
            && std::env::var("AETHER_OLLAMA_URL").ok().filter(|s| !s.is_empty()).is_some())
    {
        return Box::new(OllamaProvider::from_env());
    }
    if std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
    {
        return Box::new(OpenAiProvider::from_env());
    }
    Box::new(RuleBasedProvider)
}

pub struct OpenAiProvider {
    api_key: String,
    model: String,
    base_url: String,
}

impl OpenAiProvider {
    pub fn from_env() -> Self {
        Self {
            api_key: std::env::var("OPENAI_API_KEY").unwrap_or_default(),
            model: std::env::var("AETHER_LLM_MODEL").unwrap_or_else(|_| "gpt-4o-mini".into()),
            base_url: std::env::var("AETHER_OPENAI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".into()),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    async fn chat(&self, messages: &[ChatMessage], tools_json: &serde_json::Value) -> Result<LlmResponse> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "tools": tools_json,
        });
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let resp: serde_json::Value = client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let choice = resp
            .pointer("/choices/0/message")
            .context("missing LLM response message")?;
        let content = choice
            .get("content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tool_calls = choice
            .get("tool_calls")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        Some(ToolCallRequest {
                            id: tc.get("id")?.as_str()?.to_string(),
                            name: tc.pointer("/function/name")?.as_str()?.to_string(),
                            arguments: serde_json::from_str(
                                tc.pointer("/function/arguments")?.as_str().unwrap_or("{}"),
                            )
                            .unwrap_or(serde_json::json!({})),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(LlmResponse {
            content,
            tool_calls,
        })
    }
}

pub struct OllamaProvider {
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn from_env() -> Self {
        Self {
            base_url: std::env::var("AETHER_OLLAMA_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".into()),
            model: std::env::var("AETHER_LLM_MODEL").unwrap_or_else(|_| "llama3.2".into()),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(&self, messages: &[ChatMessage], _tools_json: &serde_json::Value) -> Result<LlmResponse> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
        });
        let resp: serde_json::Value = client
            .post(&url)
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let content = resp
            .pointer("/message/content")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(LlmResponse {
            content,
            tool_calls: Vec::new(),
        })
    }
}

/// Rule-based fallback when no LLM API key is configured.
pub struct RuleBasedProvider;

#[async_trait]
impl LlmProvider for RuleBasedProvider {
    async fn chat(&self, messages: &[ChatMessage], _tools_json: &serde_json::Value) -> Result<LlmResponse> {
        let last = messages
            .iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.content.to_lowercase())
            .unwrap_or_default();

        let (name, args) = if last.contains("health") || last.contains("unhealthy") || last.contains("why") {
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
