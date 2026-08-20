// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! LLM provider adapters for Zyra.

use crate::zyra::provider::{ChatMessage, LlmProvider, LlmResponse, ToolCallRequest};
use anyhow::{Context, Result};
use async_trait::async_trait;

pub struct OpenAiCompatibleProvider {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
    pub organization_id: Option<String>,
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools_json: &serde_json::Value,
    ) -> Result<LlmResponse> {
        let client = reqwest::Client::new();
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "tools": tools_json,
        });
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let mut req = client.post(&url).json(&body);
        if !self.api_key.is_empty() {
            req = req.bearer_auth(&self.api_key);
        }
        if let Some(org) = &self.organization_id {
            req = req.header("OpenAI-Organization", org.as_str());
        }
        let resp: serde_json::Value = req.send().await?.error_for_status()?.json().await?;
        parse_openai_response(resp)
    }
}

pub struct AnthropicProvider {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools_json: &serde_json::Value,
    ) -> Result<LlmResponse> {
        let client = reqwest::Client::new();
        let system = messages
            .iter()
            .find(|m| m.role == "system")
            .map(|m| m.content.clone())
            .unwrap_or_default();
        let msgs: Vec<_> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": if m.role == "assistant" { "assistant" } else { "user" },
                    "content": m.content,
                })
            })
            .collect();
        let mut body = serde_json::json!({
            "model": self.model,
            "max_tokens": 4096,
            "messages": msgs,
        });
        if !system.is_empty() {
            body["system"] = serde_json::json!(system);
        }
        if tools_json.as_array().is_some_and(|a| !a.is_empty()) {
            body["tools"] = tools_json.clone();
        }
        let url = format!("{}/v1/messages", self.base_url.trim_end_matches('/'));
        let resp: serde_json::Value = client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .json(&body)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let content = resp
            .pointer("/content/0/text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let tool_calls = resp
            .get("content")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|block| {
                        if block.get("type")?.as_str()? != "tool_use" {
                            return None;
                        }
                        Some(ToolCallRequest {
                            id: block.get("id")?.as_str()?.to_string(),
                            name: block.get("name")?.as_str()?.to_string(),
                            arguments: block.get("input").cloned().unwrap_or(serde_json::json!({})),
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

pub struct GeminiProvider {
    pub api_key: String,
    pub model: String,
    pub base_url: String,
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        _tools_json: &serde_json::Value,
    ) -> Result<LlmResponse> {
        let client = reqwest::Client::new();
        let contents: Vec<_> = messages
            .iter()
            .filter(|m| m.role != "system")
            .map(|m| {
                serde_json::json!({
                    "role": if m.role == "assistant" { "model" } else { "user" },
                    "parts": [{"text": m.content}],
                })
            })
            .collect();
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            self.model,
            self.api_key
        );
        let resp: serde_json::Value = client
            .post(&url)
            .json(&serde_json::json!({ "contents": contents }))
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let content = resp
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        Ok(LlmResponse {
            content,
            tool_calls: Vec::new(),
        })
    }
}

pub struct OllamaProvider {
    pub base_url: String,
    pub model: String,
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    async fn chat(
        &self,
        messages: &[ChatMessage],
        tools_json: &serde_json::Value,
    ) -> Result<LlmResponse> {
        let client = reqwest::Client::new();
        let url = format!("{}/api/chat", self.base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": self.model,
            "messages": messages,
            "stream": false,
            "tools": tools_json,
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

fn parse_openai_response(resp: serde_json::Value) -> Result<LlmResponse> {
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
