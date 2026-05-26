// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Zero-trust networking — Cilium + PacketWolf attestation-bound identity.

use crate::spec::{CiliumNetworkPolicySpec, CiliumPolicyRuleSpec, Workload};
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialNetworkStatus {
    pub workload: String,
    pub policy_count: u32,
    pub cilium_auto_policy: bool,
    pub packetwolf_hints: HashMap<String, String>,
    pub spiffe_id: Option<String>,
    pub recommendations: Vec<String>,
}

pub fn compute_trust_score(
    workload: &str,
    attestation_passed: bool,
    cilium_policy_count: u32,
    debug_allowed: bool,
    launch_digest: Option<&str>,
) -> NetworkTrustScore {
    let attestation_score: f64 = if attestation_passed { 1.0 } else { 0.0 };
    let network_policy_score: f64 = match cilium_policy_count {
        0 => 0.4,
        1 => 0.75,
        _ => 0.95,
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

pub fn policy_count(spec: &Workload) -> u32 {
    let mut count = 0u32;
    if spec.network.network_policy.is_some() {
        count += 1;
    }
    if spec
        .network
        .cilium_network_policy
        .as_ref()
        .is_some_and(|c| c.enabled)
    {
        count += 1;
    }
    if spec
        .network
        .calico_network_policy
        .as_ref()
        .is_some_and(|c| c.enabled)
    {
        count += 1;
    }
    if spec.confidential.as_ref().is_some_and(|c| c.enabled) {
        count += 1;
    }
    count
}

pub fn default_confidential_cilium(spec: &Workload) -> Option<CiliumNetworkPolicySpec> {
    if !spec.confidential.as_ref().is_some_and(|c| c.enabled) {
        return None;
    }
    if spec
        .network
        .cilium_network_policy
        .as_ref()
        .is_some_and(|c| c.enabled)
    {
        return None;
    }
    Some(CiliumNetworkPolicySpec {
        enabled: true,
        ingress: vec![CiliumPolicyRuleSpec {
            from_endpoints: vec![{
                let mut m = HashMap::new();
                m.insert("managed-by".into(), "aether".into());
                m
            }],
            to_ports: spec
                .network
                .ports
                .iter()
                .map(|p| crate::spec::CiliumPortRuleSpec {
                    port: p.container_port,
                    protocol: p.protocol.clone(),
                })
                .collect(),
            ..Default::default()
        }],
        egress: vec![CiliumPolicyRuleSpec {
            to_cidr: vec!["0.0.0.0/0".into()],
            ..Default::default()
        }],
    })
}

pub fn network_status(spec: &Workload) -> ConfidentialNetworkStatus {
    let name = spec.metadata.name.clone();
    let digest = spec
        .confidential
        .as_ref()
        .and_then(|c| c.image_digest.as_deref());
    let auto = default_confidential_cilium(spec).is_some();
    let mut recommendations = Vec::new();
    if auto {
        recommendations.push(
            "Auto CiliumNetworkPolicy will be applied on deploy (east-west cluster-only ingress)".into(),
        );
    }
    if std::env::var("AETHER_PACKETWOLF_URL")
        .ok()
        .filter(|s| !s.is_empty())
        .is_none()
    {
        recommendations.push("Set AETHER_PACKETWOLF_URL for eBPF east-west verification".into());
    }
    ConfidentialNetworkStatus {
        workload: name.clone(),
        policy_count: policy_count(spec),
        cilium_auto_policy: auto,
        packetwolf_hints: packetwolf_policy_hints(),
        spiffe_id: digest.map(|d| format!("spiffe://ragnarok.zyvor.dev/workload/{name}/digest/{d}")),
        recommendations,
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
    m.insert(
        "packetwolf.zyvor.dev/spiffe-bound".into(),
        "true".into(),
    );
    m
}

pub fn confidential_pod_annotations(spec: &Workload) -> HashMap<String, String> {
    let mut ann = HashMap::new();
    if !spec.confidential.as_ref().is_some_and(|c| c.enabled) {
        return ann;
    }
    for (k, v) in packetwolf_policy_hints() {
        ann.insert(k, v);
    }
    if let Some(d) = spec.confidential.as_ref().and_then(|c| c.image_digest.as_ref()) {
        ann.insert(
            "ragnarok.zyvor.dev/attestation-digest".into(),
            d.clone(),
        );
    }
    ann
}
