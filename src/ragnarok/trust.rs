//! Composite trust scoring for confidential workloads.

use crate::ragnarok::attestation::AttestationService;
use crate::ragnarok::network::{compute_trust_score, NetworkTrustScore};
use crate::spec::Workload;

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
                if spec.confidential.as_ref().is_some_and(|c| c.enabled) {
                    1
                } else {
                    0
                },
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
