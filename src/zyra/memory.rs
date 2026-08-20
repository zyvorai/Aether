// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Zyra scoped memory system.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryScope {
    #[default]
    Global,
    Project,
    Team,
    Session,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Inventory,
    Preference,
    Conversation,
    Runbook,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZyraMemoryEntry {
    pub id: String,
    pub scope: MemoryScope,
    pub scope_id: Option<String>,
    pub kind: MemoryKind,
    pub content: String,
    pub session_id: Option<String>,
    pub summary: String,
    pub fleet_context: serde_json::Value,
    pub updated_at: String,
    pub ttl: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZyraMemorySettings {
    pub enabled: bool,
    pub project_scoped: bool,
    pub team_scoped: bool,
    pub retention_days: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ZyraMemoryReport {
    pub entries: Vec<ZyraMemoryEntry>,
    pub settings: ZyraMemorySettings,
    pub persisted: bool,
}

fn memory_path() -> PathBuf {
    crate::resources::aether_path("zyra-memory.json")
}

fn settings_path() -> PathBuf {
    crate::resources::aether_path("zyra-memory-settings.json")
}

pub fn read_memory_settings() -> ZyraMemorySettings {
    let path = settings_path();
    if !path.exists() {
        return ZyraMemorySettings {
            enabled: true,
            project_scoped: true,
            team_scoped: false,
            retention_days: 90,
        };
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

pub fn save_memory_settings(settings: &ZyraMemorySettings) -> anyhow::Result<()> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, serde_json::to_string_pretty(settings)?)?;
    Ok(())
}

pub fn read_zyra_memory() -> ZyraMemoryReport {
    let settings = read_memory_settings();
    let path = memory_path();
    if !path.exists() {
        return ZyraMemoryReport {
            entries: Vec::new(),
            settings,
            persisted: false,
        };
    }
    match std::fs::read_to_string(&path) {
        Ok(raw) => {
            let mut report: ZyraMemoryReport = serde_json::from_str(&raw).unwrap_or_default();
            report.settings = settings;
            report.persisted = true;
            report
        }
        Err(_) => ZyraMemoryReport {
            entries: Vec::new(),
            settings,
            persisted: false,
        },
    }
}

pub fn write_zyra_memory_entry(entry: ZyraMemoryEntry) -> anyhow::Result<ZyraMemoryReport> {
    let settings = read_memory_settings();
    if !settings.enabled {
        return Ok(read_zyra_memory());
    }
    let mut report = read_zyra_memory();
    if let Some(existing) = report.entries.iter_mut().find(|e| e.id == entry.id) {
        *existing = entry;
    } else {
        report.entries.push(entry);
    }
    while report.entries.len() > 64 {
        report.entries.remove(0);
    }
    report.persisted = true;
    report.settings = settings;
    if let Some(parent) = memory_path().parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(memory_path(), serde_json::to_string_pretty(&report)?)?;
    Ok(report)
}

pub fn write_session_memory(
    session_id: &str,
    summary: &str,
    fleet_context: serde_json::Value,
) -> anyhow::Result<ZyraMemoryReport> {
    write_zyra_memory_entry(ZyraMemoryEntry {
        id: format!("session-{session_id}"),
        scope: MemoryScope::Session,
        scope_id: Some(session_id.to_string()),
        kind: MemoryKind::Conversation,
        content: summary.to_string(),
        session_id: Some(session_id.to_string()),
        summary: summary.chars().take(240).collect(),
        fleet_context,
        updated_at: crate::resources::now_rfc3339(),
        ttl: None,
    })
}

pub fn memory_for_context(scope_id: Option<&str>, team_id: Option<&str>) -> Vec<ZyraMemoryEntry> {
    let report = read_zyra_memory();
    if !report.settings.enabled {
        return Vec::new();
    }
    report
        .entries
        .into_iter()
        .filter(|e| {
            e.scope == MemoryScope::Global
                || (report.settings.project_scoped
                    && e.scope == MemoryScope::Project
                    && scope_id.is_some_and(|s| e.scope_id.as_deref() == Some(s)))
                || (report.settings.team_scoped
                    && e.scope == MemoryScope::Team
                    && team_id.is_some_and(|t| e.scope_id.as_deref() == Some(t)))
        })
        .collect()
}

pub fn purge_memory() -> anyhow::Result<()> {
    let path = memory_path();
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}
