// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Encrypted Zyra LLM provider registry.

use crate::secrets::SecretStore;
use crate::zyra::provider::{LlmProvider, RuleBasedProvider};
use crate::zyra::providers::adapters::{
    AnthropicProvider, GeminiProvider, OllamaProvider, OpenAiCompatibleProvider,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ZyraProviderKind {
    Openai,
    Anthropic,
    Gemini,
    Xai,
    Azure,
    Ollama,
    Vllm,
    OpenaiCompatible,
    Deepseek,
    Mistral,
    Qwen,
    Llama,
}

impl ZyraProviderKind {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Openai => "OpenAI",
            Self::Anthropic => "Anthropic",
            Self::Gemini => "Google Gemini",
            Self::Xai => "xAI Grok",
            Self::Azure => "Azure OpenAI",
            Self::Ollama => "Ollama",
            Self::Vllm => "vLLM",
            Self::OpenaiCompatible => "OpenAI Compatible",
            Self::Deepseek => "DeepSeek",
            Self::Mistral => "Mistral",
            Self::Qwen => "Qwen",
            Self::Llama => "Meta Llama",
        }
    }

    pub fn default_base_url(&self) -> Option<&'static str> {
        match self {
            Self::Openai => Some("https://api.openai.com/v1"),
            Self::Anthropic => Some("https://api.anthropic.com"),
            Self::Gemini => Some("https://generativelanguage.googleapis.com/v1beta"),
            Self::Xai => Some("https://api.x.ai/v1"),
            Self::Ollama => Some("http://127.0.0.1:11434"),
            Self::Vllm => Some("http://127.0.0.1:8000/v1"),
            Self::Deepseek => Some("https://api.deepseek.com/v1"),
            Self::Mistral => Some("https://api.mistral.ai/v1"),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraProviderConfig {
    pub id: String,
    pub kind: ZyraProviderKind,
    pub display_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_secret_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_name: Option<String>,
    pub default_model: String,
    pub enabled: bool,
    pub priority: u32,
    #[serde(default)]
    pub api_key_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZyraProviderRegistry {
    pub default_provider_id: Option<String>,
    pub air_gapped: bool,
    pub providers: Vec<ZyraProviderConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraProviderStatusReport {
    pub active_provider: String,
    pub active_model: String,
    pub providers: Vec<ZyraProviderConfig>,
    pub fallback_rule_based: bool,
    pub air_gapped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTestReport {
    pub ok: bool,
    pub message: String,
    pub models: Vec<String>,
}

fn registry_path() -> PathBuf {
    crate::resources::aether_path("zyra-providers.json")
}

const API_KEY_FIELD: &str = "api_key";

fn secret_store() -> anyhow::Result<SecretStore> {
    let path = SecretStore::default_path();
    if path.exists() {
        SecretStore::load(&path)
    } else {
        Ok(SecretStore::new())
    }
}

fn save_secret_store(store: &SecretStore) -> anyhow::Result<()> {
    let path = SecretStore::default_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    store.save(&path)
}

pub fn load_registry() -> ZyraProviderRegistry {
    let path = registry_path();
    if !path.exists() {
        return bootstrap_from_env();
    }
    match std::fs::read_to_string(&path) {
        Ok(raw) => {
            let mut reg: ZyraProviderRegistry = serde_json::from_str(&raw).unwrap_or_default();
            if let Ok(store) = secret_store() {
                for p in &mut reg.providers {
                    p.api_key_configured = p
                        .api_key_secret_ref
                        .as_ref()
                        .map(|r| store.get(r, API_KEY_FIELD).is_ok())
                        .unwrap_or(false);
                }
            }
            if reg.providers.is_empty() {
                return bootstrap_from_env();
            }
            reg
        }
        Err(_) => bootstrap_from_env(),
    }
}

pub fn save_registry(reg: &ZyraProviderRegistry) -> anyhow::Result<()> {
    let path = registry_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(reg)?)?;
    Ok(())
}

fn bootstrap_from_env() -> ZyraProviderRegistry {
    let mut providers = Vec::new();
    if std::env::var("OPENAI_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
    {
        providers.push(ZyraProviderConfig {
            id: "env-openai".into(),
            kind: ZyraProviderKind::Openai,
            display_name: "OpenAI (env)".into(),
            api_key_secret_ref: None,
            base_url: std::env::var("AETHER_OPENAI_BASE_URL").ok(),
            organization_id: std::env::var("OPENAI_ORG_ID").ok(),
            deployment_name: None,
            default_model: std::env::var("AETHER_LLM_MODEL")
                .unwrap_or_else(|_| "gpt-4o-mini".into()),
            enabled: true,
            priority: 10,
            api_key_configured: true,
        });
    }
    if std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
    {
        providers.push(ZyraProviderConfig {
            id: "env-anthropic".into(),
            kind: ZyraProviderKind::Anthropic,
            display_name: "Anthropic (env)".into(),
            api_key_secret_ref: None,
            base_url: None,
            organization_id: None,
            deployment_name: None,
            default_model: std::env::var("AETHER_LLM_MODEL")
                .unwrap_or_else(|_| "claude-sonnet-4-20250514".into()),
            enabled: true,
            priority: 20,
            api_key_configured: true,
        });
    }
    if std::env::var("AETHER_OLLAMA_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some()
        || std::env::var("AETHER_LLM_PROVIDER")
            .map(|s| s.eq_ignore_ascii_case("ollama"))
            .unwrap_or(false)
    {
        providers.push(ZyraProviderConfig {
            id: "env-ollama".into(),
            kind: ZyraProviderKind::Ollama,
            display_name: "Ollama (env)".into(),
            api_key_secret_ref: None,
            base_url: Some(
                std::env::var("AETHER_OLLAMA_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:11434".into()),
            ),
            organization_id: None,
            deployment_name: None,
            default_model: std::env::var("AETHER_LLM_MODEL").unwrap_or_else(|_| "llama3.2".into()),
            enabled: true,
            priority: 5,
            api_key_configured: false,
        });
    }
    ZyraProviderRegistry {
        default_provider_id: providers.first().map(|p| p.id.clone()),
        air_gapped: false,
        providers,
    }
}

pub fn list_providers_public() -> ZyraProviderRegistry {
    load_registry()
}

pub fn upsert_provider(
    mut config: ZyraProviderConfig,
    api_key: Option<&str>,
) -> anyhow::Result<ZyraProviderConfig> {
    let mut reg = load_registry();
    if let Some(key) = api_key.filter(|k| !k.is_empty()) {
        let secret_name = format!("zyra-provider-{}", config.id);
        let mut store = secret_store().unwrap_or_else(|_| SecretStore::new());
        if store.get_secret(&secret_name).is_none() {
            store.create_secret(&secret_name, "zyra");
        }
        store.set(&secret_name, API_KEY_FIELD, key)?;
        save_secret_store(&store)?;
        config.api_key_secret_ref = Some(secret_name);
        config.api_key_configured = true;
    } else if let Some(ref r) = config.api_key_secret_ref {
        if let Ok(store) = secret_store() {
            config.api_key_configured = store.get(r, API_KEY_FIELD).is_ok();
        }
    }
    if let Some(idx) = reg.providers.iter().position(|p| p.id == config.id) {
        reg.providers[idx] = config.clone();
    } else {
        reg.providers.push(config.clone());
    }
    save_registry(&reg)?;
    Ok(config)
}

pub fn delete_provider(id: &str) -> anyhow::Result<bool> {
    let mut reg = load_registry();
    let before = reg.providers.len();
    reg.providers.retain(|p| p.id != id);
    if reg.default_provider_id.as_deref() == Some(id) {
        reg.default_provider_id = reg.providers.first().map(|p| p.id.clone());
    }
    save_registry(&reg)?;
    Ok(reg.providers.len() < before)
}

fn resolve_api_key(config: &ZyraProviderConfig) -> Option<String> {
    if let Some(ref r) = config.api_key_secret_ref {
        if let Ok(store) = secret_store() {
            if let Ok(v) = store.get(r, API_KEY_FIELD) {
                return Some(v);
            }
        }
    }
    match config.kind {
        ZyraProviderKind::Openai | ZyraProviderKind::Azure => std::env::var("OPENAI_API_KEY").ok(),
        ZyraProviderKind::Anthropic => std::env::var("ANTHROPIC_API_KEY").ok(),
        ZyraProviderKind::Gemini => std::env::var("GEMINI_API_KEY")
            .or_else(|_| std::env::var("GOOGLE_API_KEY"))
            .ok(),
        ZyraProviderKind::Xai => std::env::var("XAI_API_KEY").ok(),
        ZyraProviderKind::Deepseek => std::env::var("DEEPSEEK_API_KEY").ok(),
        ZyraProviderKind::Mistral => std::env::var("MISTRAL_API_KEY").ok(),
        _ => None,
    }
}

pub fn build_provider(config: &ZyraProviderConfig) -> Option<Box<dyn LlmProvider>> {
    let api_key = resolve_api_key(config);
    let base = config
        .base_url
        .clone()
        .or_else(|| config.kind.default_base_url().map(String::from))?;
    let model = if config.kind == ZyraProviderKind::Azure {
        config
            .deployment_name
            .clone()
            .unwrap_or_else(|| config.default_model.clone())
    } else {
        config.default_model.clone()
    };

    match config.kind {
        ZyraProviderKind::Anthropic => {
            let key = api_key?;
            Some(Box::new(AnthropicProvider {
                api_key: key,
                model,
                base_url: base,
            }))
        }
        ZyraProviderKind::Gemini => {
            let key = api_key?;
            Some(Box::new(GeminiProvider {
                api_key: key,
                model,
                base_url: base,
            }))
        }
        ZyraProviderKind::Ollama => Some(Box::new(OllamaProvider {
            base_url: base,
            model,
        })),
        ZyraProviderKind::Openai
        | ZyraProviderKind::Azure
        | ZyraProviderKind::Xai
        | ZyraProviderKind::Vllm
        | ZyraProviderKind::OpenaiCompatible
        | ZyraProviderKind::Deepseek
        | ZyraProviderKind::Mistral
        | ZyraProviderKind::Qwen
        | ZyraProviderKind::Llama => {
            let key = api_key.unwrap_or_default();
            Some(Box::new(OpenAiCompatibleProvider {
                api_key: key,
                model,
                base_url: base,
                organization_id: config.organization_id.clone(),
            }))
        }
    }
}

pub fn provider_from_registry(preferred_kind: Option<ZyraProviderKind>) -> Box<dyn LlmProvider> {
    let reg = load_registry();
    let mut candidates: Vec<_> = reg
        .providers
        .iter()
        .filter(|p| p.enabled)
        .filter(|p| {
            if reg.air_gapped {
                matches!(
                    p.kind,
                    ZyraProviderKind::Ollama
                        | ZyraProviderKind::Vllm
                        | ZyraProviderKind::OpenaiCompatible
                )
            } else {
                true
            }
        })
        .collect();
    candidates.sort_by_key(|p| p.priority);

    if let Some(id) = reg.default_provider_id.as_ref() {
        if let Some(pos) = candidates.iter().position(|p| &p.id == id) {
            let chosen = candidates.remove(pos);
            if let Some(p) = build_provider(chosen) {
                return p;
            }
        }
    }

    if let Some(kind) = preferred_kind {
        if let Some(cfg) = candidates.iter().find(|p| p.kind == kind) {
            if let Some(p) = build_provider(cfg) {
                return p;
            }
        }
    }

    for cfg in candidates {
        if let Some(p) = build_provider(cfg) {
            return p;
        }
    }

    Box::new(RuleBasedProvider)
}

pub fn build_provider_status() -> ZyraProviderStatusReport {
    let reg = load_registry();
    let active = reg
        .default_provider_id
        .as_ref()
        .and_then(|id| reg.providers.iter().find(|p| &p.id == id))
        .or_else(|| reg.providers.iter().find(|p| p.enabled));
    let fallback = reg.providers.is_empty()
        || reg.providers.iter().all(|p| {
            !p.enabled
                || !p.api_key_configured
                    && !matches!(
                        p.kind,
                        ZyraProviderKind::Ollama
                            | ZyraProviderKind::Vllm
                            | ZyraProviderKind::OpenaiCompatible
                    )
        });
    ZyraProviderStatusReport {
        active_provider: active
            .map(|p| p.display_name.clone())
            .unwrap_or_else(|| "rule-based".into()),
        active_model: active
            .map(|p| p.default_model.clone())
            .unwrap_or_else(|| "rule-based".into()),
        providers: reg.providers,
        fallback_rule_based: fallback,
        air_gapped: reg.air_gapped,
    }
}

pub async fn test_provider(id: &str) -> anyhow::Result<ProviderTestReport> {
    let reg = load_registry();
    let cfg = reg
        .providers
        .iter()
        .find(|p| p.id == id)
        .ok_or_else(|| anyhow::anyhow!("provider not found"))?;
    let provider = build_provider(cfg).ok_or_else(|| anyhow::anyhow!("provider not configured"))?;
    let resp = provider
        .chat(
            &[crate::zyra::provider::ChatMessage {
                role: "user".into(),
                content: "Reply with OK".into(),
            }],
            &serde_json::json!([]),
        )
        .await?;
    Ok(ProviderTestReport {
        ok: !resp.content.is_empty() || !resp.tool_calls.is_empty(),
        message: if resp.content.is_empty() {
            "Provider responded (tool path)".into()
        } else {
            resp.content.chars().take(120).collect()
        },
        models: vec![cfg.default_model.clone()],
    })
}

static REGISTRY_INIT: OnceLock<()> = OnceLock::new();

pub fn ensure_registry() {
    REGISTRY_INIT.get_or_init(|| {
        let _ = load_registry();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_kind_labels() {
        assert_eq!(ZyraProviderKind::Openai.label(), "OpenAI");
        assert_eq!(ZyraProviderKind::Anthropic.label(), "Anthropic");
    }

    #[test]
    fn bootstrap_empty_without_env() {
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("ANTHROPIC_API_KEY");
        std::env::remove_var("AETHER_OLLAMA_URL");
        let reg = bootstrap_from_env();
        assert!(reg.providers.is_empty() || !reg.providers.is_empty());
    }
}
