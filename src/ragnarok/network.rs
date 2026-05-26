//! Zero-trust networking — Cilium + PacketWolf attestation-bound identity.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTrustScore {
    pub workload: String,
    pub attestation_score: f64,
    pub network_policy_score: f64,
    pub firmware_exposure_score: f64,
    pub composite: f64,
    pub spiffe_id: Option<String>,
}

pub fn compute_trust_score(
    workload: &str,
    attestation_passed: bool,
    cilium_policy_count: u32,
    debug_allowed: bool,
    launch_digest: Option<&str>,
) -> NetworkTrustScore {
    let attestation_score: f64 = if attestation_passed { 1.0 } else { 0.0 };
    let network_policy_score: f64 = if cilium_policy_count > 0 {
        0.9
    } else {
        0.5
    };
    let firmware_exposure_score: f64 = if debug_allowed { 0.3 } else { 0.95 };

    let composite: f64 = attestation_score * 0.5_f64
        + network_policy_score * 0.3_f64
        + firmware_exposure_score * 0.2_f64;
    let composite = composite.clamp(0.0_f64, 1.0_f64);

    let spiffe_id = launch_digest.map(|d| format!("spiffe://ragnarok.zyvor.dev/workload/{workload}/digest/{d}"));

    NetworkTrustScore {
        workload: workload.to_string(),
        attestation_score,
        network_policy_score,
        firmware_exposure_score,
        composite,
        spiffe_id,
    }
}

pub fn packetwolf_policy_hints() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert(
        "packetwolf.zyvor.dev/verify-east-west".into(),
        "true".into(),
    );
    m.insert(
        "packetwolf.zyvor.dev/alert-crypto-miner".into(),
        "true".into(),
    );
    m
}
