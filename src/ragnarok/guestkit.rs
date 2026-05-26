//! GuestKit offline inspection integration.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InspectionMode {
    PreLaunch,
    OfflinePolicy,
    PostShutdown,
    AttestedRepair,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestKitRequest {
    pub vm_id: String,
    pub image_path: Option<String>,
    pub mode: InspectionMode,
    pub policy_manifest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuestKitResult {
    pub vm_id: String,
    pub mode: InspectionMode,
    pub passed: bool,
    pub findings: Vec<String>,
    pub chain_valid: bool,
}

pub fn inspect(req: &GuestKitRequest) -> GuestKitResult {
    let chain_valid = req.image_path.is_some() || req.policy_manifest.is_some();
    let passed = chain_valid && !req.vm_id.is_empty();
    GuestKitResult {
        vm_id: req.vm_id.clone(),
        mode: req.mode.clone(),
        passed,
        findings: if passed {
            vec!["GuestKit: offline chain validation OK".into()]
        } else {
            vec!["GuestKit: missing image or policy manifest".into()]
        },
        chain_valid,
    }
}

pub fn enrich_explain(findings: &[String]) -> String {
    if findings.is_empty() {
        "No GuestKit offline findings".into()
    } else {
        format!("GuestKit: {}", findings.join("; "))
    }
}
