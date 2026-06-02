// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Composite trust scoring for confidential workloads.

use crate::ragnarok::attestation::AttestationService;
use crate::ragnarok::image::ImageCatalog;
use crate::ragnarok::network::{compute_trust_score, policy_count, NetworkTrustScore};
use crate::spec::Workload;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialFleetRow {
    pub workload: String,
    pub runtime: String,
    pub tee: String,
    pub image_digest: Option<String>,
    pub image_in_catalog: bool,
    pub attestation_passed: bool,
    pub trust: NetworkTrustScore,
}

pub fn confidential_fleet_rows(
    workloads: &[(&str, &Workload, &str)],
    attestation: &AttestationService,
    catalog: &ImageCatalog,
) -> Vec<ConfidentialFleetRow> {
    workloads
        .iter()
        .map(|(name, spec, runtime)| {
            let conf = spec.confidential.as_ref().filter(|c| c.enabled);
            let debug = conf.map(|c| c.isolation.debug_allowed).unwrap_or(false);
            let digest = conf.and_then(|c| c.image_digest.clone());
            let image_in_catalog = digest
                .as_deref()
                .map(|d| catalog.verify_digest(d))
                .unwrap_or(false);
            let tee = conf
                .and_then(|c| serde_json::to_value(c.tee).ok())
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_else(|| "—".into());
            let trust = compute_trust_score(
                name,
                attestation.passed(name),
                policy_count(spec),
                debug,
                digest.as_deref(),
            );
            ConfidentialFleetRow {
                workload: name.to_string(),
                runtime: runtime.to_string(),
                tee,
                image_digest: digest,
                image_in_catalog,
                attestation_passed: attestation.passed(name),
                trust,
            }
        })
        .collect()
}

pub fn fleet_trust_scores(
    workloads: &[(&str, &Workload)],
    attestation: &AttestationService,
) -> Vec<NetworkTrustScore> {
    workloads
        .iter()
        .map(|(name, spec)| {
            let debug = spec
                .confidential
                .as_ref()
                .map(|c| c.isolation.debug_allowed)
                .unwrap_or(false);
            let digest = spec
                .confidential
                .as_ref()
                .and_then(|c| c.image_digest.as_deref());
            compute_trust_score(
                name,
                attestation.passed(name),
                policy_count(spec),
                debug,
                digest,
            )
        })
        .collect()
}

pub fn requires_attestation_for_strict_trust(spec: &Workload) -> bool {
    if let Some(ref conf) = spec.confidential {
        if conf.enabled && conf.attestation.required {
            return true;
        }
    }
    matches!(
        spec.intent.as_ref().and_then(|i| i.trust.as_ref()),
        Some(crate::spec::TrustLevel::Strict)
    )
}
