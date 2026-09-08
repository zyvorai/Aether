// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Zyra agent marketplace — installable specialist agents.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraMarketplaceAgent {
    pub id: String,
    pub name: String,
    pub description: String,
    pub category: String,
    pub required_provider: Option<String>,
    pub installed: bool,
    pub system_prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraMarketplaceReport {
    pub generated_at: String,
    pub agents: Vec<ZyraMarketplaceAgent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZyraInstalledAgents {
    pub agent_ids: Vec<String>,
}

fn installed_path() -> PathBuf {
    crate::resources::aether_path("zyra-agents.json")
}

pub fn load_installed() -> ZyraInstalledAgents {
    let path = installed_path();
    if !path.exists() {
        return ZyraInstalledAgents::default();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

fn save_installed(inst: &ZyraInstalledAgents) -> anyhow::Result<()> {
    let path = installed_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(inst)?)?;
    Ok(())
}

fn catalog() -> Vec<ZyraMarketplaceAgent> {
    vec![
        mp(
            "aws-expert",
            "AWS Expert",
            "AWS architecture, EKS, and cost optimization.",
            "cloud",
        ),
        mp(
            "azure-expert",
            "Azure Expert",
            "Azure AKS, networking, and compliance.",
            "cloud",
        ),
        mp(
            "gcp-expert",
            "GCP Expert",
            "GKE, Anthos, and Google Cloud FinOps.",
            "cloud",
        ),
        mp(
            "terraform-expert",
            "Terraform Expert",
            "Generate and review Terraform modules.",
            "iac",
        ),
        mp(
            "linux-expert",
            "Linux Expert",
            "Host tuning, systemd, and bare-metal ops.",
            "platform",
        ),
        mp(
            "finops-expert",
            "FinOps Expert",
            "Deep cloud cost analysis and chargeback.",
            "cost",
        ),
    ]
}

fn mp(id: &str, name: &str, description: &str, category: &str) -> ZyraMarketplaceAgent {
    ZyraMarketplaceAgent {
        id: id.into(),
        name: name.into(),
        description: description.into(),
        category: category.into(),
        required_provider: None,
        installed: false,
        system_prompt: format!("You are Zyra {name}. {description}"),
    }
}

pub fn build_marketplace() -> ZyraMarketplaceReport {
    let installed = load_installed();
    let mut agents = catalog();
    for a in &mut agents {
        a.installed = installed.agent_ids.iter().any(|id| id == &a.id);
    }
    ZyraMarketplaceReport {
        generated_at: crate::resources::now_rfc3339(),
        agents,
    }
}

pub fn install_agent(id: &str) -> anyhow::Result<ZyraMarketplaceReport> {
    let mut installed = load_installed();
    if !installed.agent_ids.iter().any(|x| x == id) {
        installed.agent_ids.push(id.to_string());
        save_installed(&installed)?;
    }
    Ok(build_marketplace())
}

pub fn uninstall_agent(id: &str) -> anyhow::Result<ZyraMarketplaceReport> {
    let mut installed = load_installed();
    installed.agent_ids.retain(|x| x != id);
    save_installed(&installed)?;
    Ok(build_marketplace())
}
