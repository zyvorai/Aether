// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Security profile catalog — local defaults with optional Ragnarok remote fetch.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecurityProfileEntry {
    pub id: String,
    pub label: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kata_runtime_class: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CatalogEnvelope {
    data: Vec<SecurityProfileEntry>,
}

/// Built-in profiles when Ragnarok is unavailable.
pub fn default_profiles() -> Vec<SecurityProfileEntry> {
    vec![
        SecurityProfileEntry {
            id: "sandbox".into(),
            label: "sandbox".into(),
            description: "Non-confidential sandbox pool (no Kata TEE runtime class)".into(),
            kata_runtime_class: None,
        },
        SecurityProfileEntry {
            id: "standard-confidential".into(),
            label: "standard-confidential".into(),
            description: "Standard confidential Kata pool (SEV-SNP / TDX runtime class from env)".into(),
            kata_runtime_class: Some("kata-clh-snp".into()),
        },
        SecurityProfileEntry {
            id: "sovereign-high".into(),
            label: "sovereign-high".into(),
            description: "High-assurance sovereign pool with strict attestation expectations".into(),
            kata_runtime_class: Some("kata-clh-snp".into()),
        },
    ]
}

fn ragnarok_base_url() -> Option<String> {
    std::env::var("RAGNAROK_URL")
        .or_else(|_| std::env::var("RAGNAROK_API"))
        .ok()
        .filter(|s| !s.trim().is_empty())
}

async fn fetch_remote_catalog(base: &str) -> Result<Vec<SecurityProfileEntry>> {
    let url = format!(
        "{}/api/v1/confidential/security-profiles",
        base.trim_end_matches('/')
    );
    let env: CatalogEnvelope = reqwest::Client::new()
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await
        .context("ragnarok security profiles")?;
    Ok(env.data)
}

/// List profiles from Ragnarok when configured, otherwise local defaults.
pub async fn list_profiles() -> Vec<SecurityProfileEntry> {
    if let Some(base) = ragnarok_base_url() {
        if let Ok(remote) = fetch_remote_catalog(&base).await {
            if !remote.is_empty() {
                return remote;
            }
        }
        tracing::debug!("Ragnarok security profile fetch failed; using local catalog");
    }
    default_profiles()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_profiles_include_sandbox_and_sovereign() {
        let profiles = default_profiles();
        assert!(profiles.iter().any(|p| p.id == "sandbox"));
        assert!(profiles.iter().any(|p| p.id == "sovereign-high"));
    }
}
