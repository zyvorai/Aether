// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Behavioral security intelligence.

use crate::audit::AuditLog;
use crate::health::HealthHistory;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatReport {
    pub generated_at: String,
    pub threats: Vec<ThreatEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatEntry {
    pub workload: String,
    pub severity: String,
    pub category: String,
    pub score: f64,
    pub reason: String,
    pub detected_at: String,
}

pub struct SecurityEngine;

impl SecurityEngine {
    pub fn scan_fleet(workloads: &[(Workload, WorkloadState)]) -> ThreatReport {
        let health = HealthHistory::load(&HealthHistory::default_path()).unwrap_or_default();
        let audit = AuditLog::load(&AuditLog::default_path()).unwrap_or_default();
        let mut threats = Vec::new();

        for (spec, ws) in workloads {
            if let Some(t) = Self::scan_workload(spec, ws, &health, &audit) {
                threats.push(t);
            }
        }

        threats.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        ThreatReport {
            generated_at: crate::resources::now_rfc3339(),
            threats,
        }
    }

    fn scan_workload(
        spec: &Workload,
        ws: &WorkloadState,
        health: &HealthHistory,
        audit: &AuditLog,
    ) -> Option<ThreatEntry> {
        let restarts = health.restart_count(&spec.metadata.name);
        let recent_audit: Vec<_> = audit
            .events()
            .iter()
            .filter(|e| e.workload == spec.metadata.name)
            .collect();

        if restarts > 10 {
            return Some(ThreatEntry {
                workload: spec.metadata.name.clone(),
                severity: "high".into(),
                category: "instability".into(),
                score: 0.75,
                reason: format!("Excessive restarts ({restarts}) may indicate compromise or miner"),
                detected_at: crate::resources::now_rfc3339(),
            });
        }

        if spec.requirements.gpu.is_some() && ws.runtime == crate::runtime::RuntimeKind::Kubernetes {
            return Some(ThreatEntry {
                workload: spec.metadata.name.clone(),
                severity: "medium".into(),
                category: "gpu_exposure".into(),
                score: 0.55,
                reason: "GPU workload on shared Kubernetes — consider KubeVirt/Metal3 isolation".into(),
                detected_at: crate::resources::now_rfc3339(),
            });
        }

        if spec.confidential.as_ref().is_some_and(|c| c.enabled && c.attestation.required) {
            let dir = crate::ragnarok::client::RagnarokClient::attestation_data_dir();
            let att = crate::ragnarok::AttestationService::new(dir);
            if !att.passed(&spec.metadata.name) {
                return Some(ThreatEntry {
                    workload: spec.metadata.name.clone(),
                    severity: "critical".into(),
                    category: "attestation".into(),
                    score: 0.9,
                    reason: "Confidential workload has not passed Ragnarok attestation".into(),
                    detected_at: crate::resources::now_rfc3339(),
                });
            }
        }

        if spec.confidential.as_ref().is_some_and(|c| c.enabled)
            && crate::ragnarok::network::policy_count(spec) < 2
        {
            return Some(ThreatEntry {
                workload: spec.metadata.name.clone(),
                severity: "medium".into(),
                category: "network_exposure".into(),
                score: 0.6,
                reason: "Confidential workload lacks zero-trust network policies (Cilium/NetworkPolicy)".into(),
                detected_at: crate::resources::now_rfc3339(),
            });
        }

        if spec.confidential.as_ref().is_some_and(|c| c.enabled) {
            let sovereign = crate::ragnarok::sovereign::evaluate(
                spec,
                &crate::ragnarok::sovereign::SovereignConfig::from_env(),
            );
            if !sovereign.compliant {
                return Some(ThreatEntry {
                    workload: spec.metadata.name.clone(),
                    severity: "high".into(),
                    category: "sovereign".into(),
                    score: 0.85,
                    reason: format!(
                        "Sovereign policy violation: {}",
                        sovereign.violations.join("; ")
                    ),
                    detected_at: crate::resources::now_rfc3339(),
                });
            }
        }

        if recent_audit.len() > 20 {
            return Some(ThreatEntry {
                workload: spec.metadata.name.clone(),
                severity: "low".into(),
                category: "audit_noise".into(),
                score: 0.35,
                reason: "High audit activity — review recent changes".into(),
                detected_at: crate::resources::now_rfc3339(),
            });
        }

        None
    }
}
