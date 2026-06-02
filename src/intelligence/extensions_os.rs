// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Extensions & Native Ship — Era L (phases 115–124).

use crate::intelligence::macos_os::{
    build_live_activity, build_menu_extras, build_shortcuts_manifest, build_spotlight_index,
    LiveActivityReport, MenuExtrasReport, ShortcutsManifestReport, SpotlightIndexReport,
};
use crate::intelligence::sre_os::{
    build_chaos_catalog, build_game_day_plan, run_chaos_experiment, ChaosExperimentCatalog,
    ChaosRunReport, GameDayPlanReport,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

fn chaos_live_enabled() -> bool {
    std::env::var("AETHER_CHAOS_ENABLED")
        .ok()
        .filter(|s| !s.is_empty())
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

// ── Phase 115–124 overview ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionsGraduatedFeature {
    pub phase: u32,
    pub name: String,
    pub status: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionsGraduationOverview {
    pub generated_at: String,
    pub era: String,
    pub graduated_count: u32,
    pub chaos_live_enabled: bool,
    pub features: Vec<ExtensionsGraduatedFeature>,
}

pub fn build_extensions_graduation_overview() -> ExtensionsGraduationOverview {
    ExtensionsGraduationOverview {
        generated_at: crate::resources::now_rfc3339(),
        era: "L".into(),
        graduated_count: 10,
        chaos_live_enabled: chaos_live_enabled(),
        features: vec![
            ext(
                115,
                "Chaos experiments",
                "/api/intelligence/extensions/chaos/experiments",
            ),
            ext(
                116,
                "Game days planner",
                "/api/intelligence/extensions/game-days",
            ),
            ext(
                117,
                "Live Activity migrations",
                "/api/intelligence/extensions/live-activity",
            ),
            ext(
                118,
                "Spotlight index",
                "/api/intelligence/extensions/spotlight",
            ),
            ext(
                119,
                "Shortcuts manifest",
                "/api/intelligence/extensions/shortcuts",
            ),
            ext(
                120,
                "Menu extras toggles",
                "/api/intelligence/extensions/menu-extras",
            ),
            ext(
                121,
                "Chaos live execute",
                "/api/intelligence/extensions/chaos/run",
            ),
            ext(
                122,
                "Game day execute",
                "/api/intelligence/extensions/game-days/execute",
            ),
            ext(
                123,
                "Native extensions bundle",
                "/api/intelligence/extensions/native-bundle",
            ),
            ext(
                124,
                "SRE extensions bundle",
                "/api/intelligence/extensions/sre-bundle",
            ),
        ],
    }
}

fn ext(phase: u32, name: &str, endpoint: &str) -> ExtensionsGraduatedFeature {
    ExtensionsGraduatedFeature {
        phase,
        name: name.into(),
        status: "ship".into(),
        endpoint: endpoint.into(),
    }
}

// ── Phase 115: Chaos experiments (Ship) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipChaosCatalogReport {
    pub status: String,
    pub live_enabled: bool,
    pub safety_hint: String,
    pub catalog: ChaosExperimentCatalog,
}

pub fn build_ship_chaos_catalog(state_path: &Path) -> anyhow::Result<ShipChaosCatalogReport> {
    let catalog = build_chaos_catalog(state_path)?;
    Ok(ShipChaosCatalogReport {
        status: "ship".into(),
        live_enabled: chaos_live_enabled(),
        safety_hint: if chaos_live_enabled() {
            "Live chaos enabled via AETHER_CHAOS_ENABLED=1 — use dry_run unless in staging.".into()
        } else {
            "Dry-run only until AETHER_CHAOS_ENABLED=1 is set on the API server.".into()
        },
        catalog,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipChaosRunReport {
    pub status: String,
    pub live_enabled: bool,
    pub report: ChaosRunReport,
}

pub fn run_ship_chaos(dry_run: bool, experiment_id: &str) -> ShipChaosRunReport {
    let live = chaos_live_enabled();
    let effective_dry = dry_run || !live;
    ShipChaosRunReport {
        status: "ship".into(),
        live_enabled: live,
        report: run_chaos_experiment(effective_dry, experiment_id),
    }
}

// ── Phase 116: Game days (Ship) ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipGameDayReport {
    pub status: String,
    pub plan: GameDayPlanReport,
    pub hint: String,
}

pub async fn build_ship_game_days(state_path: &Path) -> anyhow::Result<ShipGameDayReport> {
    let plan = build_game_day_plan(state_path).await?;
    Ok(ShipGameDayReport {
        status: "ship".into(),
        hint: "Run scenarios with POST /api/intelligence/extensions/game-days/execute (dry_run default).".into(),
        plan,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDayExecuteRequest {
    pub scenario_id: String,
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameDayExecuteReport {
    pub status: String,
    pub dry_run: bool,
    pub scenario_id: String,
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

pub async fn execute_game_day_scenario(
    state_path: &Path,
    req: &GameDayExecuteRequest,
) -> anyhow::Result<GameDayExecuteReport> {
    let plan = build_game_day_plan(state_path).await?;
    let scenario = plan
        .scenarios
        .iter()
        .find(|s| s.id == req.scenario_id)
        .ok_or_else(|| anyhow::anyhow!("scenario not found: {}", req.scenario_id))?;

    let mut executed = Vec::new();
    let mut skipped = Vec::new();
    if req.dry_run {
        for step in &scenario.steps {
            executed.push(format!("dry-run: {step}"));
        }
    } else {
        skipped.push("Live game-day requires operator confirmation in dashboard or CLI".into());
        executed.push(format!(
            "scheduled: {} ({} min)",
            scenario.title, scenario.duration_minutes
        ));
    }

    Ok(GameDayExecuteReport {
        status: "ship".into(),
        dry_run: req.dry_run,
        scenario_id: req.scenario_id.clone(),
        executed,
        skipped,
    })
}

// ── Phases 117–120: macOS native (Ship) ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipLiveActivityReport {
    pub status: String,
    pub report: LiveActivityReport,
}

pub fn build_ship_live_activity(state_path: &Path) -> anyhow::Result<ShipLiveActivityReport> {
    Ok(ShipLiveActivityReport {
        status: "ship".into(),
        report: build_live_activity(state_path)?,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipSpotlightReport {
    pub status: String,
    pub report: SpotlightIndexReport,
    pub indexable_count: u32,
}

pub fn build_ship_spotlight(state_path: &Path) -> anyhow::Result<ShipSpotlightReport> {
    let report = build_spotlight_index(state_path)?;
    let indexable_count = report.items.len() as u32;
    Ok(ShipSpotlightReport {
        status: "ship".into(),
        indexable_count,
        report,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipShortcutsReport {
    pub status: String,
    pub report: ShortcutsManifestReport,
}

pub fn build_ship_shortcuts() -> ShipShortcutsReport {
    ShipShortcutsReport {
        status: "ship".into(),
        report: build_shortcuts_manifest(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShipMenuExtrasReport {
    pub status: String,
    pub report: MenuExtrasReport,
}

pub fn build_ship_menu_extras(state_path: &Path) -> anyhow::Result<ShipMenuExtrasReport> {
    Ok(ShipMenuExtrasReport {
        status: "ship".into(),
        report: build_menu_extras(state_path)?,
    })
}

// ── Phase 123: Native bundle ──────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeExtensionsBundle {
    pub status: String,
    pub generated_at: String,
    pub live_activity_count: u32,
    pub spotlight_items: u32,
    pub shortcuts_count: u32,
    pub menu_toggles: u32,
}

pub fn build_native_extensions_bundle(state_path: &Path) -> anyhow::Result<NativeExtensionsBundle> {
    let live = build_live_activity(state_path)?;
    let spotlight = build_spotlight_index(state_path)?;
    let shortcuts = build_shortcuts_manifest();
    let menu = build_menu_extras(state_path)?;
    Ok(NativeExtensionsBundle {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        live_activity_count: live.active.len() as u32,
        spotlight_items: spotlight.items.len() as u32,
        shortcuts_count: shortcuts.shortcuts.len() as u32,
        menu_toggles: menu.toggles.len() as u32,
    })
}

// ── Phase 124: SRE bundle ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SreExtensionsBundle {
    pub status: String,
    pub generated_at: String,
    pub chaos_experiments: u32,
    pub game_day_scenarios: u32,
    pub chaos_live_enabled: bool,
}

pub async fn build_sre_extensions_bundle(state_path: &Path) -> anyhow::Result<SreExtensionsBundle> {
    let chaos = build_chaos_catalog(state_path)?;
    let game = build_game_day_plan(state_path).await?;
    Ok(SreExtensionsBundle {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        chaos_experiments: chaos.experiments.len() as u32,
        game_day_scenarios: game.scenarios.len() as u32,
        chaos_live_enabled: chaos_live_enabled(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_has_ten_features() {
        let o = build_extensions_graduation_overview();
        assert_eq!(o.graduated_count, 10);
    }

    #[test]
    fn ship_chaos_dry_run() {
        let r = run_ship_chaos(true, "pod-restart");
        assert_eq!(r.status, "ship");
        assert!(!r.report.executed.is_empty());
    }

    #[tokio::test]
    async fn game_day_execute_dry_run() {
        let dir = tempfile::tempdir().unwrap();
        let state_path = dir.path().join("state.json");
        let store = crate::state::StateStore::default();
        store.save(&state_path).unwrap();
        let plan = build_game_day_plan(&state_path).await.unwrap();
        let id = plan.scenarios[0].id.clone();
        let r = execute_game_day_scenario(
            &state_path,
            &GameDayExecuteRequest {
                scenario_id: id,
                dry_run: true,
            },
        )
        .await
        .unwrap();
        assert_eq!(r.status, "ship");
        assert!(!r.executed.is_empty());
    }
}
