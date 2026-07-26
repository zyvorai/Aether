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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityPolicySuggestion {
    pub workload: String,
    pub severity: String,
    pub title: String,
    pub policy_yaml: String,
    pub rationale: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityCopilotReport {
    pub generated_at: String,
    pub suggestions: Vec<SecurityPolicySuggestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRemediateRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
    #[serde(default)]
    pub confirm: bool,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityRemediateReport {
    pub dry_run: bool,
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub pending_confirmation: Vec<String>,
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

        if spec.requirements.gpu.is_some() && ws.runtime == crate::runtime::RuntimeKind::Kubernetes
        {
            return Some(ThreatEntry {
                workload: spec.metadata.name.clone(),
                severity: "medium".into(),
                category: "gpu_exposure".into(),
                score: 0.55,
                reason: "GPU workload on shared Kubernetes — consider KubeVirt/Metal3 isolation"
                    .into(),
                detected_at: crate::resources::now_rfc3339(),
            });
        }

        if spec
            .confidential
            .as_ref()
            .is_some_and(|c| c.enabled && c.attestation.required)
        {
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
                reason:
                    "Confidential workload lacks zero-trust network policies (Cilium/NetworkPolicy)"
                        .into(),
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

    pub fn copilot_policies(threats: &ThreatReport) -> SecurityCopilotReport {
        let suggestions = threats
            .threats
            .iter()
            .filter_map(Self::policy_for_threat)
            .collect();
        SecurityCopilotReport {
            generated_at: crate::resources::now_rfc3339(),
            suggestions,
        }
    }

    fn policy_for_threat(threat: &ThreatEntry) -> Option<SecurityPolicySuggestion> {
        let (title, yaml) = match threat.category.as_str() {
            "gpu_exposure" => (
                "Isolate GPU workload ingress",
                format!(
                    r#"apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: {workload}-gpu-isolation
  namespace: default
spec:
  podSelector:
    matchLabels:
      app: {workload}
  policyTypes:
    - Ingress
  ingress:
    - from:
        - podSelector:
            matchLabels:
              role: gpu-gateway
"#,
                    workload = threat.workload
                ),
            ),
            "network_exposure" => (
                "Zero-trust network for confidential workload",
                format!(
                    r#"apiVersion: cilium.io/v2
kind: CiliumNetworkPolicy
metadata:
  name: {workload}-zero-trust
  namespace: default
spec:
  endpointSelector:
    matchLabels:
      app: {workload}
  ingress:
    - fromEndpoints:
        - matchLabels:
            io.kubernetes.pod.namespace: trusted
  egress:
    - toEntities:
        - kube-apiserver
"#,
                    workload = threat.workload
                ),
            ),
            "attestation" => (
                "Require attestation before scheduling",
                format!(
                    r#"apiVersion: v1
kind: Pod
metadata:
  name: {workload}
  annotations:
    aether.zyvor.dev/attestation: required
    aether.zyvor.dev/trust-policy: ragnarok-strict
spec:
  securityContext:
    runAsNonRoot: true
    seccompProfile:
      type: RuntimeDefault
"#,
                    workload = threat.workload
                ),
            ),
            "instability" => (
                "Cap restarts and enforce resource limits",
                format!(
                    r#"apiVersion: apps/v1
kind: Deployment
metadata:
  name: {workload}
spec:
  template:
    spec:
      containers:
        - name: {workload}
          resources:
            limits:
              cpu: "2"
              memory: 4Gi
            requests:
              cpu: 500m
              memory: 512Mi
"#,
                    workload = threat.workload
                ),
            ),
            "sovereign" => (
                "Sovereign compliance guardrails",
                format!(
                    r#"apiVersion: aether/v1
kind: WorkloadPolicy
metadata:
  name: {workload}-sovereign
spec:
  workload: {workload}
  require:
    - data_residency: in-region
    - encryption_at_rest: true
    - attestation: passed
"#,
                    workload = threat.workload
                ),
            ),
            _ => (
                "Least-privilege service account",
                format!(
                    r#"apiVersion: v1
kind: ServiceAccount
metadata:
  name: {workload}-restricted
  namespace: default
automountServiceAccountToken: false
---
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: {workload}-minimal
rules: []
"#,
                    workload = threat.workload
                ),
            ),
        };

        Some(SecurityPolicySuggestion {
            workload: threat.workload.clone(),
            severity: threat.severity.clone(),
            title: title.into(),
            policy_yaml: yaml,
            rationale: threat.reason.clone(),
        })
    }

    pub fn remediate_fleet(
        workloads: &[(Workload, WorkloadState)],
        dry_run: bool,
        confirm: bool,
    ) -> SecurityRemediateReport {
        let threats = Self::scan_fleet(workloads);
        let copilot = Self::copilot_policies(&threats);
        let mut applied = Vec::new();
        let mut skipped = Vec::new();
        let mut pending_confirmation = Vec::new();

        for suggestion in copilot.suggestions {
            let line = format!(
                "apply {} policy for {} ({})",
                suggestion.title, suggestion.workload, suggestion.severity
            );
            let needs_confirm = suggestion.severity == "critical" || suggestion.severity == "high";
            if needs_confirm && !confirm {
                pending_confirmation.push(line);
                skipped.push(format!(
                    "{}: requires confirm=true for {} severity",
                    suggestion.workload, suggestion.severity
                ));
                continue;
            }
            if dry_run {
                applied.push(format!("dry-run: {line}"));
            } else {
                // Policy YAML is suggested only — not applied to the cluster yet.
                skipped.push(format!(
                    "{}: not applied — cluster policy apply not implemented ({line})",
                    suggestion.workload
                ));
            }
        }

        SecurityRemediateReport {
            dry_run,
            applied,
            skipped,
            pending_confirmation,
        }
    }
}

#[cfg(test)]
mod copilot_tests {
    use super::*;

    #[test]
    fn copilot_generates_policy_per_threat() {
        let threats = ThreatReport {
            generated_at: "now".into(),
            threats: vec![ThreatEntry {
                workload: "api".into(),
                severity: "medium".into(),
                category: "gpu_exposure".into(),
                score: 0.5,
                reason: "GPU on shared cluster".into(),
                detected_at: "now".into(),
            }],
        };
        let report = SecurityEngine::copilot_policies(&threats);
        assert_eq!(report.suggestions.len(), 1);
        assert!(report.suggestions[0].policy_yaml.contains("NetworkPolicy"));
    }
}
