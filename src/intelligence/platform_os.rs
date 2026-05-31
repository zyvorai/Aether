// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Platform & Ecosystem — Era J (phases 95–104).

use crate::hosted::billing::usage_summary;
use crate::hosted::tenant::TenantStore;
use crate::intelligence::autonomy::build_autonomy_status;
use crate::intelligence::finops::FinOpsEngine;
use crate::intelligence::gitops_agent::execute_gitops_agent;
use crate::intelligence::healer::{build_healer_preview, execute_healer};
use crate::intelligence::intent_os::list_intent_templates;
use crate::intelligence::pipeline::{build_intent_pipeline, IntentPipelineRequest};
use crate::intelligence::policy::AutonomyPolicy;
use crate::plugin::PluginRegistry;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Phase 95: SaaS multi-tenant UI ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasTenantLine {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub plan: String,
    pub active: bool,
    pub workload_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaasTenantDashboardReport {
    pub generated_at: String,
    pub billing_period: String,
    pub tenants: Vec<SaasTenantLine>,
    pub total_workloads: usize,
}

pub fn build_saas_tenant_dashboard(state_path: &Path) -> anyhow::Result<SaasTenantDashboardReport> {
    let store = StateStore::load(state_path)?;
    let tenants = TenantStore::load();
    let billing = usage_summary(&store, &tenants);
    let lines: Vec<SaasTenantLine> = billing
        .tenants
        .iter()
        .map(|t| {
            let meta = tenants.get(&t.tenant_id);
            SaasTenantLine {
                id: t.tenant_id.clone(),
                slug: t.tenant_slug.clone(),
                name: meta.as_ref().map(|m| m.name.clone()).unwrap_or_else(|| t.tenant_slug.clone()),
                plan: t.plan.clone(),
                active: meta.as_ref().map(|m| m.active).unwrap_or(true),
                workload_count: t.workload_count,
            }
        })
        .collect();
    Ok(SaasTenantDashboardReport {
        generated_at: crate::resources::now_rfc3339(),
        billing_period: billing.period,
        tenants: lines,
        total_workloads: billing.total_workloads,
    })
}

// ── Phase 96: Plugin marketplace ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketplaceEntry {
    pub name: String,
    pub version: String,
    pub runtime_kind: String,
    pub capabilities: Vec<String>,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginMarketplaceReport {
    pub generated_at: String,
    pub entries: Vec<PluginMarketplaceEntry>,
    pub discover_path: String,
}

pub fn build_plugin_marketplace() -> anyhow::Result<PluginMarketplaceReport> {
    let mut reg = PluginRegistry::load(&PluginRegistry::default_path())?;
    let _ = reg.discover();
    reg.save(&PluginRegistry::default_path())?;
    let mut entries: Vec<PluginMarketplaceEntry> = reg
        .plugins
        .values()
        .map(|p| PluginMarketplaceEntry {
            name: p.name.clone(),
            version: p.version.clone(),
            runtime_kind: p.runtime_kind.clone(),
            capabilities: p.capabilities.clone(),
            installed: true,
        })
        .collect();
    entries.push(PluginMarketplaceEntry {
        name: "packetwolf-bridge".into(),
        version: "0.1.0".into(),
        runtime_kind: "observability".into(),
        capabilities: vec!["anomalies".into(), "egress-verify".into()],
        installed: crate::ecosystem::packetwolf::config().configured,
    });
    entries.push(PluginMarketplaceEntry {
        name: "metal3-baremetal".into(),
        version: "1.0.0".into(),
        runtime_kind: "metal3".into(),
        capabilities: vec!["provision".into(), "capacity".into()],
        installed: false,
    });
    Ok(PluginMarketplaceReport {
        generated_at: crate::resources::now_rfc3339(),
        entries,
        discover_path: crate::resources::aether_dir().join("plugins").display().to_string(),
    })
}

// ── Phase 97: Helm AI generator v2 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmAiV2Request {
    pub goals: Vec<String>,
    pub workload_name: Option<String>,
    pub yaml: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HelmAiV2Report {
    pub generated_at: String,
    pub workload: String,
    pub recommended_runtime: String,
    pub chart_yaml: String,
    pub values_yaml: String,
    pub intent_yaml: String,
}

pub async fn build_helm_ai_v2(req: &HelmAiV2Request) -> anyhow::Result<HelmAiV2Report> {
    let pipeline_req = IntentPipelineRequest {
        yaml: req.yaml.clone(),
        goals: req.goals.clone(),
        workload_name: req.workload_name.clone(),
    };
    let pipeline = build_intent_pipeline(&pipeline_req).await?;
    let spec: Workload = serde_yaml::from_str(&pipeline.spec_yaml)?;
    let bundle = crate::helm::preview_helm_bundle(&spec, Some("0.2.0"));
    Ok(HelmAiV2Report {
        generated_at: crate::resources::now_rfc3339(),
        workload: bundle.workload,
        recommended_runtime: pipeline.recommended_runtime,
        chart_yaml: bundle.chart_yaml,
        values_yaml: bundle.values_yaml,
        intent_yaml: pipeline.intent_yaml,
    })
}

// ── Phase 98: Terraform export (Lab) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerraformExportReport {
    pub status: String,
    pub generated_at: String,
    pub workload: String,
    pub hcl: String,
    pub hint: String,
}

pub fn build_terraform_export(spec: &Workload) -> TerraformExportReport {
    let cpu = spec.requirements.cpu.replace('m', "");
    let mem = spec.requirements.memory.clone();
    let hcl = format!(
        r#"# Generated by Aether — intent → Terraform (Lab)
resource "aether_workload" "{name}" {{
  name     = "{name}"
  cpu      = "{cpu}"
  memory   = "{mem}"
  runtime  = "{runtime}"
  owner    = "{owner}"
  project  = "{project}"
}}
"#,
        name = spec.metadata.name,
        cpu = cpu,
        mem = mem,
        runtime = format!("{:?}", spec.runtime.preferred),
        owner = spec.metadata.owner,
        project = spec.metadata.project,
    );
    TerraformExportReport {
        status: "lab".into(),
        generated_at: crate::resources::now_rfc3339(),
        workload: spec.metadata.name.clone(),
        hcl,
        hint: "Import into Terraform Cloud or OpenTofu — provider stub for lab use.".into(),
    }
}

// ── Phase 99: Pulumi bridge (Lab) ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PulumiBridgeReport {
    pub status: String,
    pub generated_at: String,
    pub language: String,
    pub program: String,
    pub hint: String,
}

pub fn build_pulumi_bridge(spec: &Workload) -> PulumiBridgeReport {
    let program = format!(
        r#"import * as aether from "@aether/pulumi";
const workload = new aether.Workload("{name}", {{
  cpu: "{cpu}",
  memory: "{mem}",
  runtime: "{runtime}",
}});
export const workloadName = workload.name;
"#,
        name = spec.metadata.name,
        cpu = spec.requirements.cpu,
        mem = spec.requirements.memory,
        runtime = format!("{:?}", spec.runtime.preferred),
    );
    PulumiBridgeReport {
        status: "lab".into(),
        generated_at: crate::resources::now_rfc3339(),
        language: "typescript".into(),
        program,
        hint: "Run with Pulumi Automation API — package is lab-only.".into(),
    }
}

// ── Phase 100: Public AI OS API manifest ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicApiRoute {
    pub method: String,
    pub path: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicApiManifest {
    pub version: String,
    pub generated_at: String,
    pub routes: Vec<PublicApiRoute>,
}

pub fn build_public_api_manifest() -> PublicApiManifest {
    PublicApiManifest {
        version: "v1".into(),
        generated_at: crate::resources::now_rfc3339(),
        routes: vec![
            PublicApiRoute {
                method: "GET".into(),
                path: "/v1/intelligence/briefing".into(),
                description: "Command center briefing".into(),
            },
            PublicApiRoute {
                method: "GET".into(),
                path: "/v1/intelligence/threats".into(),
                description: "Fleet threat scan".into(),
            },
            PublicApiRoute {
                method: "GET".into(),
                path: "/v1/intelligence/cost-optimize".into(),
                description: "FinOps optimization report".into(),
            },
            PublicApiRoute {
                method: "GET".into(),
                path: "/v1/intelligence/predictions".into(),
                description: "Failure predictions".into(),
            },
            PublicApiRoute {
                method: "GET".into(),
                path: "/v1/intelligence/autonomy".into(),
                description: "Autonomous SRE status".into(),
            },
        ],
    }
}

// ── Phase 101: Mobile companion (Lab) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MobileCompanionReport {
    pub status: String,
    pub generated_at: String,
    pub deep_link_scheme: String,
    pub read_only_endpoints: Vec<String>,
    pub hint: String,
}

pub fn build_mobile_companion_manifest() -> MobileCompanionReport {
    MobileCompanionReport {
        status: "lab".into(),
        generated_at: crate::resources::now_rfc3339(),
        deep_link_scheme: "aether://".into(),
        read_only_endpoints: vec![
            "/v1/intelligence/briefing".into(),
            "/api/command-center/briefing".into(),
            "/api/intelligence/threats".into(),
            "/api/health".into(),
        ],
        hint: "Read-only fleet status — pair with macOS universal links or PWA shell.".into(),
    }
}

// ── Phase 102: IDE extensions (Lab) ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeExtensionEntry {
    pub id: String,
    pub editor: String,
    pub features: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdeExtensionManifest {
    pub status: String,
    pub generated_at: String,
    pub extensions: Vec<IdeExtensionEntry>,
    pub openapi_url: String,
}

pub fn build_ide_extension_manifest() -> IdeExtensionManifest {
    IdeExtensionManifest {
        status: "lab".into(),
        generated_at: crate::resources::now_rfc3339(),
        extensions: vec![
            IdeExtensionEntry {
                id: "aether.vscode-workload-designer".into(),
                editor: "VS Code".into(),
                features: vec![
                    "Validate workload YAML".into(),
                    "Deploy from editor".into(),
                    "Intent template picker".into(),
                ],
            },
            IdeExtensionEntry {
                id: "aether.cursor-workload-designer".into(),
                editor: "Cursor".into(),
                features: vec![
                    "Copilot workload prompts".into(),
                    "Live drift badge".into(),
                    "Helm export command".into(),
                ],
            },
        ],
        openapi_url: "/api/openapi.json".into(),
    }
}

// ── Phase 103: Community intent library (Lab) ─────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityIntentEntry {
    pub id: String,
    pub title: String,
    pub author: String,
    pub stars: u32,
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityIntentLibraryReport {
    pub status: String,
    pub generated_at: String,
    pub built_in_count: usize,
    pub community: Vec<CommunityIntentEntry>,
}

pub fn build_community_intent_library() -> CommunityIntentLibraryReport {
    let built_in = list_intent_templates();
    CommunityIntentLibraryReport {
        status: "lab".into(),
        generated_at: crate::resources::now_rfc3339(),
        built_in_count: built_in.templates.len(),
        community: vec![
            CommunityIntentEntry {
                id: "community-finops-api".into(),
                title: "FinOps API (community)".into(),
                author: "aether-community".into(),
                stars: 128,
                goal: "cost-optimized".into(),
            },
            CommunityIntentEntry {
                id: "community-zero-trust".into(),
                title: "Zero-trust microservice".into(),
                author: "security-sig".into(),
                stars: 96,
                goal: "balanced".into(),
            },
            CommunityIntentEntry {
                id: "community-gpu-inference".into(),
                title: "GPU inference node".into(),
                author: "ml-platform".into(),
                stars: 74,
                goal: "high-throughput".into(),
            },
        ],
    }
}

// ── Phase 104: Full autonomous SRE ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousSreStatusReport {
    pub generated_at: String,
    pub closed_loop_ready: bool,
    pub autonomy_enabled: bool,
    pub human_gate_required: bool,
    pub agents_active: u32,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousSreExecuteRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutonomousSreExecuteReport {
    pub dry_run: bool,
    pub closed_loop: bool,
    pub healer: Vec<String>,
    pub finops: Vec<String>,
    pub gitops: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn build_autonomous_sre_status(state_path: &Path) -> anyhow::Result<AutonomousSreStatusReport> {
    let autonomy = build_autonomy_status(state_path)?;
    let config = crate::config::Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let human_gate = !policy.auto_restart
        || policy.auto_migrate == crate::intelligence::policy::AutonomyTier::Recommend
        || policy.auto_evolve == crate::intelligence::policy::AutonomyTier::Recommend;
    let agents_active = (if policy.auto_restart { 1 } else { 0 })
        + (if policy.auto_reconcile_drift { 1 } else { 0 })
        + (if policy.auto_migrate != crate::intelligence::policy::AutonomyTier::Recommend {
            1
        } else {
            0
        })
        + (if policy.auto_evolve != crate::intelligence::policy::AutonomyTier::Recommend {
            1
        } else {
            0
        });
    Ok(AutonomousSreStatusReport {
        generated_at: crate::resources::now_rfc3339(),
        closed_loop_ready: autonomy.autonomy_enabled && !human_gate,
        autonomy_enabled: autonomy.autonomy_enabled,
        human_gate_required: human_gate,
        agents_active,
        recommendations: autonomy.recommendations,
    })
}

pub async fn execute_autonomous_sre_loop(
    state_path: &Path,
    workloads: &[(Workload, WorkloadState)],
    req: &AutonomousSreExecuteRequest,
) -> anyhow::Result<AutonomousSreExecuteReport> {
    let config = crate::config::Config::load();
    let policy = AutonomyPolicy::from_config_and_workload(config.reconciliation.auto_reconcile, None);
    let status = build_autonomous_sre_status(state_path)?;
    let mut skipped = Vec::new();

    if status.human_gate_required && !req.dry_run {
        skipped.push("Human gate active — run with dry_run or enable full autonomy tiers".into());
    }

    let store = StateStore::load(state_path)?;
    let healer_preview = build_healer_preview(&store, &policy).await;
    let healer = if req.dry_run {
        healer_preview.would_execute.clone()
    } else if policy.allows_restart() || policy.allows_drift_reconcile() {
        execute_healer(&store, state_path, &policy, false)
            .await
            .executed
    } else {
        skipped.push("Healer skipped — autonomy policy".into());
        Vec::new()
    };

    let finops_patches = FinOpsEngine::build_cost_patches(workloads);
    let finops_report = FinOpsEngine::apply_cost_patches(&finops_patches, req.dry_run, &policy);
    let finops = finops_report.applied;

    let gitops_report = execute_gitops_agent(state_path, &policy, req.dry_run).await?;
    let gitops = gitops_report.executed;

    Ok(AutonomousSreExecuteReport {
        dry_run: req.dry_run,
        closed_loop: status.closed_loop_ready,
        healer,
        finops,
        gitops,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_api_manifest_has_v1_routes() {
        let m = build_public_api_manifest();
        assert_eq!(m.version, "v1");
        assert!(m.routes.len() >= 5);
    }

    #[test]
    fn plugin_marketplace_lists_entries() {
        let r = build_plugin_marketplace().unwrap();
        assert!(!r.entries.is_empty());
    }
}
