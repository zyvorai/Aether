//! AI confidential intelligence — attestation analyst, migration planner, fleet trust.

use crate::ragnarok::{
    attestation::AttestationService,
    guestkit::GuestKitService,
    image::ImageCatalog,
    isolation::{self, IsolationPolicy},
    kata,
    migration::plan_confidential_migration_tee,
    network::{self, policy_count},
    sovereign::{self, SovereignConfig},
    tee::probe_host_tee,
};
use crate::spec::Workload;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialAnalysis {
    pub workload: String,
    pub runtime: String,
    pub trust_composite: f64,
    pub risk_level: String,
    pub summary: String,
    pub findings: Vec<String>,
    pub recommendations: Vec<String>,
    pub attestation_passed: bool,
    pub image_in_catalog: bool,
    pub sovereign_compliant: bool,
    pub kata_placement_score: f64,
    pub migration_strategy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialFleetAnalysis {
    pub generated_at: String,
    pub workloads: Vec<ConfidentialAnalysis>,
    pub fleet_trust_avg: f64,
    pub critical_count: u32,
}

pub fn analyze_workload(
    spec: &Workload,
    runtime: &str,
    state_dir: &std::path::Path,
) -> ConfidentialAnalysis {
    let name = spec.metadata.name.clone();
    let att_svc = AttestationService::new(state_dir.to_path_buf());
    let gk_svc = GuestKitService::new(state_dir.to_path_buf());
    let catalog = ImageCatalog::load(state_dir);
    let host = probe_host_tee();

    let conf = spec.confidential.as_ref().filter(|c| c.enabled);
    let digest = conf.and_then(|c| c.image_digest.as_deref());
    let att_passed = att_svc.passed(&name);
    let image_in_catalog = digest
        .map(|d| catalog.verify_digest(d))
        .unwrap_or(false);
    let sovereign = sovereign::evaluate(spec, &SovereignConfig::from_env());
    let isolation = isolation::evaluate(spec, &IsolationPolicy::from_env());
    let net_count = policy_count(spec);
    let trust = network::compute_trust_score(
        &name,
        att_passed,
        net_count,
        conf.map(|c| c.isolation.debug_allowed).unwrap_or(false),
        digest,
    );

    let mut findings = Vec::new();
    let mut recommendations = Vec::new();

    if conf.is_none() {
        findings.push("Confidential block not enabled".into());
    }
    if !att_passed && conf.is_some_and(|c| c.attestation.required) {
        findings.push("Attestation required but not passed".into());
        recommendations.push("Submit guest attestation report or run GuestKit repair playbook".into());
    }
    if !image_in_catalog && digest.is_some() {
        findings.push("Launch digest not in measured catalog".into());
        recommendations.push("Run `aether confidential image sign` and update imageDigest".into());
    }
    if !isolation.compliant {
        findings.push(format!(
            "Isolation violations: {}",
            isolation.violations.join("; ")
        ));
    }
    if !sovereign.compliant {
        findings.push(format!(
            "Sovereign violations: {}",
            sovereign.violations.join("; ")
        ));
    }
    if let Some(gk) = gk_svc.summary(&name) {
        if !gk.passed {
            findings.push(format!("GuestKit {} inspection failed", gk.last_mode));
        }
    }
    if net_count == 0 {
        recommendations.push("Add CiliumNetworkPolicy or enable confidential auto-policy".into());
    }

    let kata_score = kata::placement_score(spec, &host);
    let migration_strategy = conf.map(|_| {
        let plan = plan_confidential_migration_tee(spec, host.sev_snp, host.sev_snp, host.tdx, host.tdx);
        plan.recommended_strategy.to_string()
    });

    let risk_level = if trust.composite >= 0.8 && findings.is_empty() {
        "low"
    } else if trust.composite >= 0.5 {
        "medium"
    } else {
        "high"
    }
    .to_string();

    let summary = if findings.is_empty() {
        format!("Workload {name} confidential posture is healthy ({risk_level} risk)")
    } else {
        format!(
            "Workload {name} has {} confidential finding(s) — {risk_level} risk",
            findings.len()
        )
    };

    ConfidentialAnalysis {
        workload: name,
        runtime: runtime.to_string(),
        trust_composite: trust.composite,
        risk_level,
        summary,
        findings,
        recommendations,
        attestation_passed: att_passed,
        image_in_catalog,
        sovereign_compliant: sovereign.compliant,
        kata_placement_score: kata_score,
        migration_strategy,
    }
}

pub fn analyze_fleet(
    workloads: &[(&str, &Workload, &str)],
    state_dir: &std::path::Path,
) -> ConfidentialFleetAnalysis {
    let analyses: Vec<_> = workloads
        .iter()
        .map(|(_name, spec, runtime)| analyze_workload(spec, runtime, state_dir))
        .collect();
    let fleet_trust_avg = if analyses.is_empty() {
        0.0
    } else {
        analyses.iter().map(|a| a.trust_composite).sum::<f64>() / analyses.len() as f64
    };
    let critical_count = analyses
        .iter()
        .filter(|a| a.risk_level == "high")
        .count() as u32;
    ConfidentialFleetAnalysis {
        generated_at: chrono::Utc::now().to_rfc3339(),
        workloads: analyses,
        fleet_trust_avg,
        critical_count,
    }
}
