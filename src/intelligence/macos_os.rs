// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! macOS Native OS — tray sparkline, dock badge, Spotlight, offline cache, deep links.

use crate::audit::{AuditAction, AuditLog};
use crate::health::HealthHistory;
use crate::intelligence::briefing::build_command_center_briefing;
use crate::intelligence::briefing::CommandCenterBriefing;
use crate::spec::Workload;
use crate::state::StateStore;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn offline_cache_path() -> PathBuf {
    crate::resources::aether_path("offline-briefing.json")
}

// ── Phase 55: Menu bar fleet sparkline ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraySparklineReport {
    pub generated_at: String,
    pub samples: Vec<f64>,
    pub sparkline: String,
    pub fleet_health_pct: f64,
}

pub fn build_tray_sparkline(state_path: &Path) -> anyhow::Result<TraySparklineReport> {
    let briefing = build_command_center_briefing(state_path)?;
    let history = HealthHistory::load(&HealthHistory::default_path()).unwrap_or_default();
    let store = StateStore::load(state_path)?;

    let mut samples = Vec::new();
    for ws in store.list().iter().take(12) {
        samples.push(history.uptime_percent(&ws.name));
    }
    if samples.is_empty() {
        samples.push(briefing.fleet_health_pct);
    }
    while samples.len() < 8 {
        samples.push(briefing.fleet_health_pct);
    }
    samples.truncate(12);

    Ok(TraySparklineReport {
        generated_at: crate::resources::now_rfc3339(),
        sparkline: render_sparkline(&samples),
        fleet_health_pct: briefing.fleet_health_pct,
        samples,
    })
}

fn render_sparkline(samples: &[f64]) -> String {
    const BARS: &[char] = &['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
    samples
        .iter()
        .map(|v| {
            let idx = ((v.clamp(0.0, 100.0) / 100.0) * (BARS.len() as f64 - 1.0)).round() as usize;
            BARS[idx]
        })
        .collect()
}

// ── Phase 56: Live Activity migrations (Lab) ───────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveActivityEntry {
    pub workload: String,
    pub phase: String,
    pub progress_pct: u32,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveActivityReport {
    pub generated_at: String,
    pub active: Vec<LiveActivityEntry>,
}

pub fn build_live_activity(state_path: &Path) -> anyhow::Result<LiveActivityReport> {
    let audit = AuditLog::load(&AuditLog::default_path()).unwrap_or_default();
    let store = StateStore::load(state_path)?;
    let mut active = Vec::new();

    for ev in audit.events().iter().rev().take(30) {
        if ev.action != AuditAction::Migrate {
            continue;
        }
        active.push(LiveActivityEntry {
            workload: ev.workload.clone(),
            phase: format!("{}", ev.result),
            progress_pct: if ev.message.contains("complete") { 100 } else { 55 },
            detail: ev.message.clone(),
        });
        if active.len() >= 3 {
            break;
        }
    }

    if active.is_empty() {
        for ws in store.list().iter().take(1) {
            active.push(LiveActivityEntry {
                workload: ws.name.clone(),
                phase: "idle".into(),
                progress_pct: 0,
                detail: "No active migrations".into(),
            });
        }
    }

    Ok(LiveActivityReport {
        generated_at: crate::resources::now_rfc3339(),
        active,
    })
}

// ── Phase 57: Dock badge issue count ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockBadgeReport {
    pub generated_at: String,
    pub issue_count: u32,
    pub badge_label: String,
}

pub fn build_dock_badge(state_path: &Path) -> anyhow::Result<DockBadgeReport> {
    let briefing = build_command_center_briefing(state_path)?;
    let count = briefing
        .issues
        .iter()
        .filter(|i| i.severity != "info")
        .count() as u32;
    Ok(DockBadgeReport {
        generated_at: crate::resources::now_rfc3339(),
        issue_count: count,
        badge_label: if count > 0 {
            format!("{count}")
        } else {
            String::new()
        },
    })
}

// ── Phase 58: Native notifications queue ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeNotificationEntry {
    pub title: String,
    pub body: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeNotificationsReport {
    pub generated_at: String,
    pub pending: Vec<NativeNotificationEntry>,
}

pub fn build_native_notifications(state_path: &Path) -> anyhow::Result<NativeNotificationsReport> {
    let briefing = build_command_center_briefing(state_path)?;
    let pending: Vec<NativeNotificationEntry> = briefing
        .issues
        .iter()
        .filter(|i| i.severity == "critical" || i.severity == "high")
        .take(5)
        .map(|i| NativeNotificationEntry {
            title: i.title.clone(),
            body: i.detail.clone(),
            severity: i.severity.clone(),
        })
        .collect();

    Ok(NativeNotificationsReport {
        generated_at: crate::resources::now_rfc3339(),
        pending,
    })
}

// ── Phase 59: Spotlight index (Lab) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotlightIndexItem {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub deep_link: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpotlightIndexReport {
    pub generated_at: String,
    pub items: Vec<SpotlightIndexItem>,
}

pub fn build_spotlight_index(state_path: &Path) -> anyhow::Result<SpotlightIndexReport> {
    let store = StateStore::load(state_path)?;
    let mut items = vec![
        SpotlightIndexItem {
            id: "view-command".into(),
            title: "Command Center".into(),
            subtitle: "Fleet briefing and next actions".into(),
            deep_link: "aether://command".into(),
        },
        SpotlightIndexItem {
            id: "view-fleet".into(),
            title: "Fleet Intelligence".into(),
            subtitle: "Multi-cloud posture".into(),
            deep_link: "aether://fleet".into(),
        },
    ];

    for ws in store.list() {
        items.push(SpotlightIndexItem {
            id: format!("wl-{}", ws.name),
            title: ws.name.clone(),
            subtitle: format!("{} runtime", ws.runtime),
            deep_link: format!("aether://workloads/{}", ws.name),
        });
    }

    Ok(SpotlightIndexReport {
        generated_at: crate::resources::now_rfc3339(),
        items,
    })
}

// ── Phase 60: Shortcuts.app actions (Lab) ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutAction {
    pub name: String,
    pub phrase: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutsManifestReport {
    pub generated_at: String,
    pub shortcuts: Vec<ShortcutAction>,
}

pub fn build_shortcuts_manifest() -> ShortcutsManifestReport {
    ShortcutsManifestReport {
        generated_at: crate::resources::now_rfc3339(),
        shortcuts: vec![
            ShortcutAction {
                name: "Open Command Center".into(),
                phrase: "Open Aether".into(),
                url: "aether://command".into(),
            },
            ShortcutAction {
                name: "Fleet health".into(),
                phrase: "Aether fleet status".into(),
                url: "aether://fleet".into(),
            },
            ShortcutAction {
                name: "Run SRE runbook".into(),
                phrase: "Aether SRE runbook".into(),
                url: "aether://observability".into(),
            },
        ],
    }
}

// ── Phase 61: Menu extras / agent toggles (Lab) ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuExtraToggle {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MenuExtrasReport {
    pub generated_at: String,
    pub toggles: Vec<MenuExtraToggle>,
}

pub fn build_menu_extras(state_path: &Path) -> anyhow::Result<MenuExtrasReport> {
    let config = crate::config::Config::load();
    let _store = StateStore::load(state_path)?;
    Ok(MenuExtrasReport {
        generated_at: crate::resources::now_rfc3339(),
        toggles: vec![
            MenuExtraToggle {
                id: "healer".into(),
                label: "Self-healing agent".into(),
                enabled: config.reconciliation.auto_reconcile,
            },
            MenuExtraToggle {
                id: "notifications".into(),
                label: "Critical alerts".into(),
                enabled: true,
            },
            MenuExtraToggle {
                id: "autonomy".into(),
                label: "Autonomous mode".into(),
                enabled: config.reconciliation.auto_reconcile,
            },
        ],
    })
}

// ── Phase 62: Offline dashboard cache ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfflineCacheReport {
    pub generated_at: String,
    pub cached: bool,
    pub briefing: Option<CommandCenterBriefing>,
}

pub fn read_offline_cache() -> OfflineCacheReport {
    let path = offline_cache_path();
    if !path.exists() {
        return OfflineCacheReport {
            generated_at: crate::resources::now_rfc3339(),
            cached: false,
            briefing: None,
        };
    }
    let briefing = std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok());
    OfflineCacheReport {
        generated_at: crate::resources::now_rfc3339(),
        cached: briefing.is_some(),
        briefing,
    }
}

pub fn write_offline_cache(state_path: &Path) -> anyhow::Result<OfflineCacheReport> {
    let briefing = build_command_center_briefing(state_path)?;
    let path = offline_cache_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(&briefing)?)?;
    Ok(OfflineCacheReport {
        generated_at: crate::resources::now_rfc3339(),
        cached: true,
        briefing: Some(briefing),
    })
}

// ── Phase 63: Universal Links ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalLinkRoute {
    pub scheme: String,
    pub path: String,
    pub view: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalLinkRegistry {
    pub generated_at: String,
    pub routes: Vec<UniversalLinkRoute>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UniversalLinkResolveReport {
    pub url: String,
    pub view: String,
    pub query: std::collections::HashMap<String, String>,
}

pub fn link_registry() -> UniversalLinkRegistry {
    UniversalLinkRegistry {
        generated_at: crate::resources::now_rfc3339(),
        routes: vec![
            route("command", "overview"),
            route("fleet", "fleet"),
            route("observability", "observability"),
            route("workloads", "workloads"),
            route("settings", "settings"),
            route("labs", "labs"),
        ],
    }
}

fn route(path: &str, view: &str) -> UniversalLinkRoute {
    UniversalLinkRoute {
        scheme: "aether".into(),
        path: path.into(),
        view: view.into(),
    }
}

pub fn resolve_universal_link(url: &str) -> UniversalLinkResolveReport {
    let mut query = std::collections::HashMap::new();
    let trimmed = url.trim();
    let without_scheme = trimmed
        .strip_prefix("aether://")
        .or_else(|| trimmed.strip_prefix("aether:"))
        .unwrap_or(trimmed);
    let (path, qs) = without_scheme.split_once('?').unwrap_or((without_scheme, ""));
    for pair in qs.split('&').filter(|s| !s.is_empty()) {
        if let Some((k, v)) = pair.split_once('=') {
            query.insert(k.into(), v.into());
        }
    }
    let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    let view = match segments.first().copied().unwrap_or("command") {
        "fleet" => "fleet",
        "observability" | "sre" => "observability",
        "workloads" | "workload" => {
            if let Some(name) = segments.get(1) {
                query.insert("workload".into(), (*name).into());
            }
            "workloads"
        }
        "settings" => "settings",
        "labs" => "labs",
        "ai" => "ai",
        _ => "overview",
    };
    UniversalLinkResolveReport {
        url: url.into(),
        view: view.into(),
        query,
    }
}

// ── Phase 64: Notarized DMG CI status ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasePipelineStep {
    pub id: String,
    pub label: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleasePipelineReport {
    pub generated_at: String,
    pub notarization_ready: bool,
    pub signing_identity_configured: bool,
    pub steps: Vec<ReleasePipelineStep>,
}

pub fn build_release_pipeline_status() -> ReleasePipelineReport {
    let signing = std::env::var("APPLE_SIGNING_IDENTITY").is_ok()
        || std::env::var("CODESIGN_IDENTITY").is_ok();
    let notary = std::env::var("APPLE_NOTARIZATION_PROFILE").is_ok()
        || std::env::var("NOTARY_API_KEY").is_ok();

    let steps = vec![
        ReleasePipelineStep {
            id: "build-dmg".into(),
            label: "Build Tauri DMG".into(),
            status: "ready".into(),
        },
        ReleasePipelineStep {
            id: "codesign".into(),
            label: "Codesign app bundle".into(),
            status: if signing { "ready" } else { "needs-secret" }.into(),
        },
        ReleasePipelineStep {
            id: "notarize".into(),
            label: "Notarize with Apple".into(),
            status: if notary { "ready" } else { "needs-secret" }.into(),
        },
        ReleasePipelineStep {
            id: "staple".into(),
            label: "Staple notarization ticket".into(),
            status: if notary && signing {
                "ready"
            } else {
                "blocked"
            }
            .into(),
        },
    ];

    ReleasePipelineReport {
        generated_at: crate::resources::now_rfc3339(),
        signing_identity_configured: signing,
        notarization_ready: signing && notary,
        steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparkline_renders_bars() {
        let s = render_sparkline(&[0.0, 50.0, 100.0]);
        assert_eq!(s.chars().count(), 3);
    }

    #[test]
    fn resolve_workload_link() {
        let r = resolve_universal_link("aether://workloads/api");
        assert_eq!(r.view, "workloads");
        assert_eq!(r.query.get("workload"), Some(&"api".to_string()));
    }

    #[test]
    fn shortcuts_manifest_nonempty() {
        assert!(!build_shortcuts_manifest().shortcuts.is_empty());
    }

    #[test]
    fn release_pipeline_defaults() {
        let r = build_release_pipeline_status();
        assert_eq!(r.steps.len(), 4);
    }
}
