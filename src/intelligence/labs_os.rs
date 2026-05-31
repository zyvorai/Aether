// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Lab Graduation — Era K (phases 105–114).

use crate::intelligence::copilot_os::build_voice_copilot_lab;
use crate::intelligence::finops_os::build_carbon_footprint;
use crate::intelligence::graph_os::export_graph;
use crate::intelligence::intent_os::{list_intent_templates, IntentTemplateLibrary};
use crate::intelligence::pipeline::{build_intent_pipeline, IntentPipelineRequest};
use crate::intelligence::platform_os::{
    build_community_intent_library, build_ide_extension_manifest, build_mobile_companion_manifest,
    build_pulumi_bridge, build_terraform_export, CommunityIntentEntry,
};
use crate::intelligence::security_os::build_compliance_report;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

fn community_intents_path() -> PathBuf {
    crate::resources::aether_path("community-intents.json")
}

// ── Phase 105–114 overview ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsGraduatedFeature {
    pub phase: u32,
    pub name: String,
    pub status: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsGraduationOverview {
    pub generated_at: String,
    pub era: String,
    pub graduated_count: u32,
    pub features: Vec<LabsGraduatedFeature>,
}

pub fn build_labs_graduation_overview() -> LabsGraduationOverview {
    LabsGraduationOverview {
        generated_at: crate::resources::now_rfc3339(),
        era: "K".into(),
        graduated_count: 10,
        features: vec![
            feature(105, "Terraform export v2", "/api/intelligence/labs/terraform-export"),
            feature(106, "Pulumi bridge v2", "/api/intelligence/labs/pulumi-bridge"),
            feature(107, "Mobile companion", "/api/intelligence/labs/mobile-companion"),
            feature(108, "IDE extensions", "/api/intelligence/labs/ide-extensions"),
            feature(109, "Community intents", "/api/intelligence/labs/community-intents"),
            feature(110, "Carbon footprint", "/api/intelligence/labs/carbon"),
            feature(111, "Compliance report", "/api/intelligence/labs/compliance-report"),
            feature(112, "Voice copilot", "/api/intelligence/labs/voice-copilot"),
            feature(113, "Graph export", "/api/intelligence/labs/graph-export"),
            feature(114, "Lab graduation hub", "/api/intelligence/labs/overview"),
        ],
    }
}

fn feature(phase: u32, name: &str, endpoint: &str) -> LabsGraduatedFeature {
    LabsGraduatedFeature {
        phase,
        name: name.into(),
        status: "ship".into(),
        endpoint: endpoint.into(),
    }
}

// ── Phase 105: Terraform export v2 ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsTerraformRequest {
    pub workload_name: Option<String>,
    pub goals: Option<Vec<String>>,
    pub yaml: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsTerraformReport {
    pub status: String,
    pub generated_at: String,
    pub workload: String,
    pub hcl: String,
    pub modules: Vec<String>,
    pub hint: String,
}

pub async fn build_labs_terraform_export(
    workloads: &[(Workload, WorkloadState)],
    req: &LabsTerraformRequest,
) -> anyhow::Result<LabsTerraformReport> {
    let spec = resolve_spec(workloads, req.workload_name.as_deref(), req).await?;
    let base = build_terraform_export(&spec);
    let variables = format!(
        r#"
variable "region" {{
  default = "{}"
}}

output "workload_name" {{
  value = aether_workload.{name}.name
}}
"#,
        crate::cost::pricing_region(),
        name = spec.metadata.name.replace('-', "_"),
    );
    Ok(LabsTerraformReport {
        status: "ship".into(),
        generated_at: base.generated_at,
        workload: base.workload,
        hcl: format!("{}\n{}", base.hcl, variables),
        modules: vec![
            "aether_workload".into(),
            "aether_intent".into(),
            "aether_runtime_binding".into(),
        ],
        hint: "OpenTofu/Terraform Cloud ready — wire the aether provider or import as module.".into(),
    })
}

// ── Phase 106: Pulumi bridge v2 ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsPulumiRequest {
    pub workload_name: Option<String>,
    pub goals: Option<Vec<String>>,
    pub yaml: Option<String>,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsPulumiReport {
    pub status: String,
    pub generated_at: String,
    pub language: String,
    pub program: String,
    pub package_name: String,
    pub hint: String,
}

pub async fn build_labs_pulumi_bridge(
    workloads: &[(Workload, WorkloadState)],
    req: &LabsPulumiRequest,
) -> anyhow::Result<LabsPulumiReport> {
    let spec = resolve_spec(
        workloads,
        req.workload_name.as_deref(),
        &LabsTerraformRequest {
            workload_name: req.workload_name.clone(),
            goals: req.goals.clone(),
            yaml: req.yaml.clone(),
        },
    )
    .await?;
    let base = build_pulumi_bridge(&spec);
    let lang = req
        .language
        .as_deref()
        .unwrap_or("typescript")
        .to_lowercase();
    let program = if lang == "python" {
        format!(
            r#"import pulumi
import pulumi_aether as aether

workload = aether.Workload("{name}",
    cpu="{cpu}",
    memory="{mem}",
    runtime="{runtime}",
)
pulumi.export("workloadName", workload.name)
"#,
            name = spec.metadata.name,
            cpu = spec.requirements.cpu,
            mem = spec.requirements.memory,
            runtime = format!("{:?}", spec.runtime.preferred),
        )
    } else {
        base.program
    };
    Ok(LabsPulumiReport {
        status: "ship".into(),
        generated_at: base.generated_at,
        language: lang,
        program,
        package_name: "@aether/pulumi".into(),
        hint: "Use Pulumi Automation API or `pulumi preview` with the generated program.".into(),
    })
}

async fn resolve_spec(
    workloads: &[(Workload, WorkloadState)],
    workload_name: Option<&str>,
    req: &LabsTerraformRequest,
) -> anyhow::Result<Workload> {
    if let Some(yaml) = req.yaml.as_deref().filter(|s| !s.trim().is_empty()) {
        return parse_workload_yaml(yaml);
    }
    if let Some(name) = workload_name.filter(|s| !s.is_empty()) {
        if let Some((spec, _)) = workloads.iter().find(|(s, _)| s.metadata.name == name) {
            return Ok(spec.clone());
        }
    }
    if let Some((spec, _)) = workloads.first() {
        return Ok(spec.clone());
    }
    let pipeline = build_intent_pipeline(&IntentPipelineRequest {
        yaml: None,
        goals: req.goals.clone().unwrap_or_else(|| vec!["cost-optimized".into()]),
        workload_name: workload_name.map(String::from),
    })
    .await?;
    parse_workload_yaml(&pipeline.spec_yaml)
}

fn parse_workload_yaml(yaml: &str) -> anyhow::Result<Workload> {
    let workload: Workload = serde_yaml::from_str(yaml)?;
    workload.validate()?;
    Ok(workload)
}

// ── Phase 107: Mobile companion (Ship) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsMobileCompanionReport {
    pub status: String,
    pub generated_at: String,
    pub deep_link_scheme: String,
    pub pwa_manifest: serde_json::Value,
    pub read_only_endpoints: Vec<String>,
    pub hint: String,
}

pub fn build_labs_mobile_companion() -> LabsMobileCompanionReport {
    let base = build_mobile_companion_manifest();
    LabsMobileCompanionReport {
        status: "ship".into(),
        generated_at: base.generated_at,
        deep_link_scheme: base.deep_link_scheme,
        pwa_manifest: serde_json::json!({
            "name": "Aether Fleet",
            "short_name": "Aether",
            "start_url": "/",
            "display": "standalone",
            "theme_color": "#3B82F6",
            "background_color": "#0B0E14",
        }),
        read_only_endpoints: base.read_only_endpoints,
        hint: base.hint,
    }
}

// ── Phase 108: IDE extensions (Ship) ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsIdeExtensionReport {
    pub status: String,
    pub generated_at: String,
    pub extensions: Vec<crate::intelligence::platform_os::IdeExtensionEntry>,
    pub openapi_url: String,
    pub install_commands: Vec<String>,
}

pub fn build_labs_ide_extensions() -> LabsIdeExtensionReport {
    let base = build_ide_extension_manifest();
    LabsIdeExtensionReport {
        status: "ship".into(),
        generated_at: base.generated_at,
        extensions: base.extensions,
        openapi_url: base.openapi_url,
        install_commands: vec![
            "code --install-extension aether.vscode-workload-designer".into(),
            "cursor --install-extension aether.cursor-workload-designer".into(),
        ],
    }
}

// ── Phase 109: Community intent library (Ship) ────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsCommunityIntentImportRequest {
    pub id: String,
    pub title: String,
    pub author: String,
    pub goal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsCommunityIntentReport {
    pub status: String,
    pub generated_at: String,
    pub built_in_count: usize,
    pub community: Vec<CommunityIntentEntry>,
    pub imported_count: usize,
}

pub fn build_labs_community_intents() -> LabsCommunityIntentReport {
    let base = build_community_intent_library();
    let imported = read_imported_community_intents();
    let mut community = base.community;
    community.extend(imported);
    LabsCommunityIntentReport {
        status: "ship".into(),
        generated_at: base.generated_at,
        built_in_count: base.built_in_count,
        community,
        imported_count: read_imported_community_intents().len(),
    }
}

pub fn import_community_intent(req: &LabsCommunityIntentImportRequest) -> anyhow::Result<LabsCommunityIntentReport> {
    if req.id.trim().is_empty() || req.title.trim().is_empty() {
        anyhow::bail!("id and title are required");
    }
    let mut entries = read_imported_community_intents();
    let entry = CommunityIntentEntry {
        id: req.id.clone(),
        title: req.title.clone(),
        author: req.author.clone(),
        stars: 0,
        goal: req.goal.clone(),
    };
    if let Some(existing) = entries.iter_mut().find(|e| e.id == req.id) {
        *existing = entry;
    } else {
        entries.push(entry);
    }
    if let Some(parent) = community_intents_path().parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(community_intents_path(), serde_json::to_string_pretty(&entries)?)?;
    Ok(build_labs_community_intents())
}

fn read_imported_community_intents() -> Vec<CommunityIntentEntry> {
    let path = community_intents_path();
    if !path.exists() {
        return Vec::new();
    }
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}

// ── Phase 110: Carbon footprint (Ship) ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsCarbonWorkloadLine {
    pub workload: String,
    pub carbon_kg_monthly: f64,
    pub region: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsCarbonReport {
    pub status: String,
    pub generated_at: String,
    pub fleet_carbon_kg_monthly: f64,
    pub workloads: Vec<LabsCarbonWorkloadLine>,
    pub hint: String,
}

pub fn build_labs_carbon_report(workloads: &[(Workload, WorkloadState)]) -> LabsCarbonReport {
    let base = build_carbon_footprint(workloads);
    let fleet_kg = base.entries.first().map(|e| e.carbon_kg_monthly).unwrap_or(0.0);
    let region = base.entries.first().map(|e| e.region.clone()).unwrap_or_default();
    let per = if workloads.is_empty() {
        0.0
    } else {
        fleet_kg / workloads.len() as f64
    };
    let lines: Vec<LabsCarbonWorkloadLine> = workloads
        .iter()
        .map(|(spec, _)| LabsCarbonWorkloadLine {
            workload: spec.metadata.name.clone(),
            carbon_kg_monthly: per,
            region: region.clone(),
        })
        .collect();
    LabsCarbonReport {
        status: "ship".into(),
        generated_at: base.generated_at,
        fleet_carbon_kg_monthly: fleet_kg,
        workloads: lines,
        hint: base.hint,
    }
}

// ── Phase 111: Compliance report (Ship) ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsComplianceReport {
    pub status: String,
    pub generated_at: String,
    pub framework: String,
    pub sections: Vec<crate::intelligence::security_os::ComplianceReportSection>,
    pub export_format: String,
    pub export_hint: String,
}

pub fn build_labs_compliance_report(workloads: &[(Workload, WorkloadState)]) -> LabsComplianceReport {
    let base = build_compliance_report(workloads);
    LabsComplianceReport {
        status: "ship".into(),
        generated_at: base.generated_at,
        framework: base.framework,
        sections: base.sections,
        export_format: "json".into(),
        export_hint: "Use as SOC2-style evidence bundle — PDF renderer is optional.".into(),
    }
}

// ── Phase 112: Voice copilot (Ship) ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsVoiceCopilotReport {
    pub status: String,
    pub supported: bool,
    pub browser_apis: Vec<String>,
    pub sample_transcript: String,
    pub hint: String,
}

pub fn build_labs_voice_copilot() -> LabsVoiceCopilotReport {
    let base = build_voice_copilot_lab();
    LabsVoiceCopilotReport {
        status: "ship".into(),
        supported: base.supported,
        browser_apis: vec![
            "SpeechRecognition".into(),
            "speechSynthesis".into(),
            "MediaRecorder".into(),
        ],
        sample_transcript: base.sample_transcript,
        hint: base.hint,
    }
}

// ── Phase 113: Graph export (Ship) ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LabsGraphExportReport {
    pub status: String,
    pub generated_at: String,
    pub format: String,
    pub node_count: u32,
    pub edge_count: u32,
    pub payload: String,
    pub download_filename: String,
}

pub fn build_labs_graph_export(state_path: &Path, format: &str) -> anyhow::Result<LabsGraphExportReport> {
    let base = export_graph(state_path, format)?;
    let filename = if base.format == "jsonld" {
        "aether-graph.jsonld"
    } else {
        "aether-graph.cypher"
    };
    Ok(LabsGraphExportReport {
        status: "ship".into(),
        generated_at: base.generated_at,
        format: base.format,
        node_count: base.node_count,
        edge_count: base.edge_count,
        payload: base.payload,
        download_filename: filename.into(),
    })
}

// ── Phase 114: Built-in template bridge ───────────────────────────────────────

pub fn built_in_intent_library() -> IntentTemplateLibrary {
    list_intent_templates()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_lists_ten_features() {
        let o = build_labs_graduation_overview();
        assert_eq!(o.graduated_count, 10);
        assert_eq!(o.features.len(), 10);
    }

    #[test]
    fn mobile_companion_ship_status() {
        let m = build_labs_mobile_companion();
        assert_eq!(m.status, "ship");
        assert!(m.pwa_manifest.get("name").is_some());
    }

    #[test]
    fn community_intents_ship_status() {
        let c = build_labs_community_intents();
        assert_eq!(c.status, "ship");
        assert!(!c.community.is_empty());
    }
}
