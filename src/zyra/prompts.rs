// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Zyra prompt library.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PromptCategory {
    Infrastructure,
    Security,
    Kubernetes,
    Runbooks,
    Sops,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraPrompt {
    pub id: String,
    pub title: String,
    pub category: PromptCategory,
    pub body: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZyraPromptLibrary {
    pub prompts: Vec<ZyraPrompt>,
}

fn prompts_path() -> PathBuf {
    crate::resources::aether_path("zyra-prompts.json")
}

pub fn load_prompts() -> ZyraPromptLibrary {
    let path = prompts_path();
    if !path.exists() {
        return ZyraPromptLibrary {
            prompts: default_prompts(),
        };
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_else(|| ZyraPromptLibrary {
            prompts: default_prompts(),
        })
}

fn save_prompts(lib: &ZyraPromptLibrary) -> anyhow::Result<()> {
    let path = prompts_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(lib)?)?;
    Ok(())
}

fn default_prompts() -> Vec<ZyraPrompt> {
    let now = crate::resources::now_rfc3339();
    vec![
        ZyraPrompt {
            id: "infra-health".into(),
            title: "Fleet health summary".into(),
            category: PromptCategory::Infrastructure,
            body: "Summarize fleet health, highlight unhealthy workloads, and recommend top 3 actions.".into(),
            tags: vec!["health".into(), "fleet".into()],
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        ZyraPrompt {
            id: "sec-scan".into(),
            title: "Security threat scan".into(),
            category: PromptCategory::Security,
            body: "Scan the fleet for security threats, policy violations, and exposed secrets.".into(),
            tags: vec!["security".into()],
            created_at: now.clone(),
            updated_at: now.clone(),
        },
        ZyraPrompt {
            id: "k8s-diagnose".into(),
            title: "Diagnose crashing pods".into(),
            category: PromptCategory::Kubernetes,
            body: "Diagnose why pods are crashing, include events, logs evidence, and a fix plan.".into(),
            tags: vec!["kubernetes".into(), "pods".into()],
            created_at: now.clone(),
            updated_at: now,
        },
    ]
}

pub fn list_prompts() -> ZyraPromptLibrary {
    load_prompts()
}

pub fn upsert_prompt(mut prompt: ZyraPrompt) -> anyhow::Result<ZyraPrompt> {
    let mut lib = load_prompts();
    prompt.updated_at = crate::resources::now_rfc3339();
    if prompt.created_at.is_empty() {
        prompt.created_at = prompt.updated_at.clone();
    }
    if let Some(existing) = lib.prompts.iter_mut().find(|p| p.id == prompt.id) {
        *existing = prompt.clone();
    } else {
        lib.prompts.push(prompt.clone());
    }
    save_prompts(&lib)?;
    Ok(prompt)
}

pub fn delete_prompt(id: &str) -> anyhow::Result<bool> {
    let mut lib = load_prompts();
    let before = lib.prompts.len();
    lib.prompts.retain(|p| p.id != id);
    save_prompts(&lib)?;
    Ok(lib.prompts.len() < before)
}

pub fn export_yaml() -> anyhow::Result<String> {
    let lib = load_prompts();
    Ok(serde_yaml::to_string(&lib.prompts)?)
}

pub fn import_yaml(raw: &str) -> anyhow::Result<ZyraPromptLibrary> {
    let imported: Vec<ZyraPrompt> = serde_yaml::from_str(raw)?;
    let mut lib = load_prompts();
    for p in imported {
        if let Some(existing) = lib.prompts.iter_mut().find(|e| e.id == p.id) {
            *existing = p;
        } else {
            lib.prompts.push(p);
        }
    }
    save_prompts(&lib)?;
    Ok(lib)
}
