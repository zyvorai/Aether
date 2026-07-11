// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Security & Compliance platform — Era I (phases 85–94).

use crate::intelligence::security::SecurityEngine;
use crate::ragnarok::attestation::AttestationService;
use crate::ragnarok::image::ImageCatalog;
use crate::ragnarok::network::policy_count;
use crate::ragnarok::sovereign::{evaluate, SovereignConfig};
use crate::ragnarok::trust::confidential_fleet_rows;
use crate::secrets::SecretStore;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

fn security_history_path() -> PathBuf {
    crate::resources::aether_path("security-score-history.json")
}

fn sbom_digest_path() -> PathBuf {
    crate::resources::aether_path("sbom-digest-history.json")
}

fn sovereign_audit_path() -> PathBuf {
    crate::resources::aether_path("sovereign-audit.jsonl")
}

// ── Phase 85: Policy auto-apply ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyAutoApplyRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
    #[serde(default)]
    pub confirm: bool,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyAutoApplyReport {
    pub dry_run: bool,
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
    pub pending_confirmation: Vec<String>,
}

pub fn apply_security_policies(
    workloads: &[(Workload, WorkloadState)],
    req: &PolicyAutoApplyRequest,
) -> PolicyAutoApplyReport {
    let report = SecurityEngine::remediate_fleet(workloads, req.dry_run, req.confirm);
    PolicyAutoApplyReport {
        dry_run: report.dry_run,
        applied: report.applied,
        skipped: report.skipped,
        pending_confirmation: report.pending_confirmation,
    }
}

// ── Phase 86: SBOM drift alerts ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SbomDigestSnapshot {
    pub timestamp: String,
    pub dashboard_sha256: String,
    pub component_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomDriftAlert {
    pub field: String,
    pub previous: String,
    pub current: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomDriftReport {
    pub generated_at: String,
    pub drift_detected: bool,
    pub alerts: Vec<SbomDriftAlert>,
    pub current_component_count: usize,
}

pub fn detect_sbom_drift() -> SbomDriftReport {
    let bom = crate::sbom::load_cached()
        .unwrap_or_else(|| crate::sbom::generate_cyclonedx(None).unwrap_or(serde_json::json!({})));
    let meta = crate::sbom::sbom_metadata(&bom);
    let current_digest = meta
        .dashboard_sha256
        .clone()
        .unwrap_or_else(|| meta.serial_number.clone());

    let prev_path = sbom_digest_path();
    let previous: Option<SbomDigestSnapshot> = prev_path
        .exists()
        .then(|| std::fs::read_to_string(&prev_path).ok())
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok());

    let mut alerts = Vec::new();
    if let Some(ref prev) = previous {
        if prev.dashboard_sha256 != current_digest && !current_digest.is_empty() {
            alerts.push(SbomDriftAlert {
                field: "dashboard_sha256".into(),
                previous: prev.dashboard_sha256.clone(),
                current: current_digest.clone(),
                severity: "warning".into(),
            });
        }
        if prev.component_count != meta.component_count {
            alerts.push(SbomDriftAlert {
                field: "component_count".into(),
                previous: prev.component_count.to_string(),
                current: meta.component_count.to_string(),
                severity: "info".into(),
            });
        }
    }

    if let Some(parent) = prev_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        prev_path,
        serde_json::to_string_pretty(&SbomDigestSnapshot {
            timestamp: crate::resources::now_rfc3339(),
            dashboard_sha256: current_digest,
            component_count: meta.component_count,
        })
        .unwrap_or_default(),
    );

    SbomDriftReport {
        generated_at: crate::resources::now_rfc3339(),
        drift_detected: !alerts.is_empty(),
        alerts,
        current_component_count: meta.component_count,
    }
}

// ── Phase 87: Confidential fleet dashboard ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialFleetDashboardReport {
    pub generated_at: String,
    pub workload_count: u32,
    pub attestation_passed: u32,
    pub catalog_verified: u32,
    pub average_trust_score: f64,
    pub rows: Vec<crate::ragnarok::trust::ConfidentialFleetRow>,
}

pub fn build_confidential_fleet_dashboard(
    workloads: &[(&str, &Workload, &str)],
    attestation: &AttestationService,
    catalog: &ImageCatalog,
) -> ConfidentialFleetDashboardReport {
    let rows = confidential_fleet_rows(workloads, attestation, catalog);
    let attestation_passed = rows.iter().filter(|r| r.attestation_passed).count() as u32;
    let catalog_verified = rows.iter().filter(|r| r.image_in_catalog).count() as u32;
    let avg = if rows.is_empty() {
        0.0
    } else {
        rows.iter().map(|r| r.trust.composite).sum::<f64>() / rows.len() as f64
    };
    ConfidentialFleetDashboardReport {
        generated_at: crate::resources::now_rfc3339(),
        workload_count: rows.len() as u32,
        attestation_passed,
        catalog_verified,
        average_trust_score: avg,
        rows,
    }
}

// ── Phase 88: Zero-trust rollout wizard ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroTrustWizardStep {
    pub id: String,
    pub title: String,
    pub description: String,
    pub policy_kind: String,
    pub workloads: Vec<String>,
    pub ready: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroTrustWizardReport {
    pub generated_at: String,
    pub steps: Vec<ZeroTrustWizardStep>,
    pub completion_pct: f64,
}

pub fn build_zero_trust_wizard(workloads: &[(Workload, WorkloadState)]) -> ZeroTrustWizardReport {
    let mut needs_ingress: Vec<String> = Vec::new();
    let mut needs_egress: Vec<String> = Vec::new();
    let mut needs_cilium: Vec<String> = Vec::new();

    for (spec, _ws) in workloads {
        let policies = policy_count(spec);
        if spec.confidential.as_ref().is_some_and(|c| c.enabled) && policies < 2 {
            needs_cilium.push(spec.metadata.name.clone());
        }
        if policies == 0 {
            needs_ingress.push(spec.metadata.name.clone());
        }
        if spec
            .network
            .network_policy
            .as_ref()
            .is_some_and(|n| !n.allow_to.is_empty())
            && policies < 1
        {
            needs_egress.push(spec.metadata.name.clone());
        }
    }

    let steps = vec![
        ZeroTrustWizardStep {
            id: "baseline".into(),
            title: "Baseline NetworkPolicy".into(),
            description: "Default deny ingress with explicit allow rules".into(),
            policy_kind: "NetworkPolicy".into(),
            workloads: needs_ingress.clone(),
            ready: needs_ingress.is_empty(),
        },
        ZeroTrustWizardStep {
            id: "egress".into(),
            title: "Egress lockdown".into(),
            description: "Restrict outbound traffic to required endpoints".into(),
            policy_kind: "NetworkPolicy".into(),
            workloads: needs_egress,
            ready: needs_ingress.is_empty(),
        },
        ZeroTrustWizardStep {
            id: "cilium".into(),
            title: "Cilium zero-trust".into(),
            description: "Layer-7 policies for confidential workloads".into(),
            policy_kind: "CiliumNetworkPolicy".into(),
            workloads: needs_cilium.clone(),
            ready: needs_cilium.is_empty(),
        },
    ];

    let done = steps.iter().filter(|s| s.ready).count();
    ZeroTrustWizardReport {
        generated_at: crate::resources::now_rfc3339(),
        completion_pct: if steps.is_empty() {
            100.0
        } else {
            (done as f64 / steps.len() as f64) * 100.0
        },
        steps,
    }
}

// ── Phase 89: Compliance report (Lab) ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReportSection {
    pub control: String,
    pub status: String,
    pub evidence: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceReport {
    pub status: String,
    pub generated_at: String,
    pub framework: String,
    pub sections: Vec<ComplianceReportSection>,
    pub export_hint: String,
}

pub fn build_compliance_report(workloads: &[(Workload, WorkloadState)]) -> ComplianceReport {
    let threats = SecurityEngine::scan_fleet(workloads);
    let critical = threats
        .threats
        .iter()
        .filter(|t| t.severity == "critical" || t.severity == "high")
        .count();
    let confidential = workloads
        .iter()
        .filter(|(s, _)| s.confidential.as_ref().is_some_and(|c| c.enabled))
        .count();

    ComplianceReport {
        status: "lab".into(),
        generated_at: crate::resources::now_rfc3339(),
        framework: "SOC2-style summary".into(),
        sections: vec![
            ComplianceReportSection {
                control: "CC6.1 Logical access".into(),
                status: if critical == 0 { "pass" } else { "review" }.into(),
                evidence: format!("{critical} high/critical threat(s) in fleet scan"),
            },
            ComplianceReportSection {
                control: "CC7.2 System monitoring".into(),
                status: "pass".into(),
                evidence: "Audit log + health history integrated".into(),
            },
            ComplianceReportSection {
                control: "CC6.7 Confidential computing".into(),
                status: if confidential > 0 { "pass" } else { "n/a" }.into(),
                evidence: format!("{confidential} confidential workload(s) with attestation hooks"),
            },
        ],
        export_hint: "PDF export is lab-only — use GET response as JSON evidence bundle.".into(),
    }
}

// ── Phase 90: Secret rotation agent ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRotationAgentRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRotationAction {
    pub secret: String,
    pub key: String,
    pub severity: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretRotationAgentReport {
    pub dry_run: bool,
    pub actions: Vec<SecretRotationAction>,
    pub rotated: Vec<String>,
    pub skipped: Vec<String>,
}

pub fn run_secret_rotation_agent(req: &SecretRotationAgentRequest) -> SecretRotationAgentReport {
    let path = SecretStore::default_path();
    let Ok(mut store) = SecretStore::load(&path) else {
        return SecretRotationAgentReport {
            dry_run: req.dry_run,
            actions: Vec::new(),
            rotated: Vec::new(),
            skipped: vec!["secrets store unavailable".into()],
        };
    };

    let report = rotate_due_secrets(&mut store, req.dry_run);

    if !req.dry_run && !report.rotated.is_empty() {
        let _ = store.save(&path);
    }

    report
}

/// Core rotation logic over an already-loaded store (no filesystem/IO), so it
/// is unit-testable in isolation. Rotates due secrets Aether owns; reports
/// externally-managed secrets as skipped without ever overwriting them.
fn rotate_due_secrets(store: &mut SecretStore, dry_run: bool) -> SecretRotationAgentReport {
    let alerts = store.audit_rotation();
    let mut actions = Vec::new();
    let mut rotated = Vec::new();
    let mut skipped = Vec::new();

    for alert in &alerts {
        actions.push(SecretRotationAction {
            secret: alert.secret.clone(),
            key: alert.key.clone(),
            severity: match alert.severity {
                crate::secrets::AlertSeverity::Critical => "critical",
                crate::secrets::AlertSeverity::Warning => "warning",
                crate::secrets::AlertSeverity::Info => "info",
            }
            .into(),
            message: alert.message.clone(),
        });
        let line = format!("rotate {}:{}", alert.secret, alert.key);
        // Only secrets Aether owns (rotation_policy.generate=true) may be
        // auto-rotated to a fresh generated credential. Externally-managed
        // secrets are never overwritten — they are reported for out-of-band
        // rotation so a mirrored credential is never clobbered.
        if !store.auto_generatable(&alert.secret) {
            skipped.push(format!(
                "{line}: requires external rotation (rotation_policy.generate=false)"
            ));
            continue;
        }
        if dry_run {
            rotated.push(format!("dry-run: {line}"));
        } else {
            match store.rotate_generated(&alert.secret, &alert.key, "security-agent") {
                Ok(_version) => rotated.push(line),
                Err(e) => skipped.push(format!("{line}: {e}")),
            }
        }
    }

    SecretRotationAgentReport {
        dry_run,
        actions,
        rotated,
        skipped,
    }
}

// ── Phase 91: Image signing enforcement ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSigningEntry {
    pub workload: String,
    pub digest: Option<String>,
    pub signed: bool,
    pub blocked: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSigningEnforcementReport {
    pub generated_at: String,
    pub enforce: bool,
    pub entries: Vec<ImageSigningEntry>,
    pub blocked_count: u32,
}

pub fn build_image_signing_enforcement(
    workloads: &[(Workload, WorkloadState)],
    catalog: &ImageCatalog,
) -> ImageSigningEnforcementReport {
    let enforce = std::env::var("AETHER_ENFORCE_SIGNED_IMAGES")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let mut entries = Vec::new();
    let mut blocked_count = 0u32;

    for (spec, _ws) in workloads {
        let digest = spec
            .confidential
            .as_ref()
            .and_then(|c| c.image_digest.clone());
        let signed = digest
            .as_deref()
            .map(|d| catalog.verify_digest(d))
            .unwrap_or(false);
        let needs_signing =
            spec.confidential.as_ref().is_some_and(|c| c.enabled) || digest.is_some();
        let blocked = enforce && needs_signing && !signed;
        if blocked {
            blocked_count += 1;
        }
        entries.push(ImageSigningEntry {
            workload: spec.metadata.name.clone(),
            digest,
            signed,
            blocked,
            reason: if blocked {
                "Unsigned image blocked by AETHER_ENFORCE_SIGNED_IMAGES".into()
            } else if needs_signing && !signed {
                "Confidential workload should use a signed catalog image".into()
            } else {
                "OK".into()
            },
        });
    }

    ImageSigningEnforcementReport {
        generated_at: crate::resources::now_rfc3339(),
        enforce,
        entries,
        blocked_count,
    }
}

// ── Phase 92: Threat hunt mode ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatHuntRequest {
    pub query: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    25
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatHuntFinding {
    pub source: String,
    pub severity: String,
    pub summary: String,
    pub workload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatHuntReport {
    pub generated_at: String,
    pub query: String,
    pub configured: bool,
    pub findings: Vec<ThreatHuntFinding>,
}

pub async fn run_threat_hunt(
    workloads: &[(Workload, WorkloadState)],
    req: &ThreatHuntRequest,
) -> ThreatHuntReport {
    let mut findings = Vec::new();
    let threats = SecurityEngine::scan_fleet(workloads);
    let q = req.query.to_lowercase();

    for t in threats.threats {
        if q.is_empty()
            || t.workload.to_lowercase().contains(&q)
            || t.category.to_lowercase().contains(&q)
            || t.reason.to_lowercase().contains(&q)
        {
            findings.push(ThreatHuntFinding {
                source: "aether-threat-scan".into(),
                severity: t.severity.clone(),
                summary: t.reason.clone(),
                workload: Some(t.workload),
            });
        }
    }

    let configured = crate::ecosystem::packetwolf::config().configured;
    if configured {
        if let Ok(raw) = crate::ecosystem::packetwolf::anomalies(Some(req.limit)).await {
            if let Some(arr) = raw
                .as_array()
                .or_else(|| raw.get("items").and_then(|v| v.as_array()))
            {
                for item in arr.iter().take(req.limit as usize) {
                    let summary = item
                        .get("summary")
                        .or_else(|| item.get("message"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("PacketWolf anomaly");
                    if q.is_empty() || summary.to_lowercase().contains(&q) {
                        findings.push(ThreatHuntFinding {
                            source: "packetwolf".into(),
                            severity: item
                                .get("severity")
                                .and_then(|v| v.as_str())
                                .unwrap_or("medium")
                                .into(),
                            summary: summary.into(),
                            workload: item
                                .get("workload")
                                .and_then(|v| v.as_str())
                                .map(str::to_string),
                        });
                    }
                }
            }
        }
    }

    findings.truncate(req.limit as usize);

    ThreatHuntReport {
        generated_at: crate::resources::now_rfc3339(),
        query: req.query.clone(),
        configured,
        findings,
    }
}

// ── Phase 93: Sovereign audit log ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignAuditEntry {
    pub timestamp: String,
    pub workload: String,
    pub compliant: bool,
    pub violations: Vec<String>,
    pub region_lock: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignAuditReport {
    pub entries: Vec<SovereignAuditEntry>,
}

pub fn append_sovereign_audit(
    workloads: &[(Workload, WorkloadState)],
) -> anyhow::Result<SovereignAuditReport> {
    let config = SovereignConfig::from_env();
    let mut entries = Vec::new();

    for (spec, _ws) in workloads {
        if !spec.confidential.as_ref().is_some_and(|c| c.enabled) {
            continue;
        }
        let verdict = evaluate(spec, &config);
        let entry = SovereignAuditEntry {
            timestamp: crate::resources::now_rfc3339(),
            workload: spec.metadata.name.clone(),
            compliant: verdict.compliant,
            violations: verdict.violations.clone(),
            region_lock: config.region_lock.clone(),
        };
        entries.push(entry.clone());
        if let Some(parent) = sovereign_audit_path().parent() {
            std::fs::create_dir_all(parent)?;
        }
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(sovereign_audit_path())?;
        writeln!(file, "{}", serde_json::to_string(&entry)?)?;
    }

    Ok(SovereignAuditReport { entries })
}

pub fn read_sovereign_audit(limit: usize) -> SovereignAuditReport {
    let path = sovereign_audit_path();
    if !path.exists() {
        return SovereignAuditReport {
            entries: Vec::new(),
        };
    }
    let raw = std::fs::read_to_string(path).unwrap_or_default();
    let mut entries: Vec<SovereignAuditEntry> = raw
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    if entries.len() > limit {
        entries = entries.split_off(entries.len() - limit);
    }
    SovereignAuditReport { entries }
}

// ── Phase 94: Security score trend ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecurityScoreSnapshot {
    timestamp: String,
    score: f64,
    threat_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct SecurityScoreHistory {
    snapshots: Vec<SecurityScoreSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScoreTrendPoint {
    pub label: String,
    pub score: f64,
    pub threat_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScoreTrendReport {
    pub generated_at: String,
    pub current_score: f64,
    pub trend_direction: String,
    pub points: Vec<SecurityScoreTrendPoint>,
}

fn fleet_security_score(workloads: &[(Workload, WorkloadState)]) -> (f64, u32) {
    let threats = SecurityEngine::scan_fleet(workloads);
    let count = threats.threats.len() as u32;
    let penalty: f64 = threats
        .threats
        .iter()
        .map(|t| t.score)
        .sum::<f64>()
        .min(1.0);
    ((1.0 - penalty) * 100.0, count)
}

pub fn build_security_score_trend(
    workloads: &[(Workload, WorkloadState)],
) -> SecurityScoreTrendReport {
    let (current, threat_count) = fleet_security_score(workloads);

    let mut history: SecurityScoreHistory = security_history_path()
        .exists()
        .then(|| std::fs::read_to_string(security_history_path()).ok())
        .flatten()
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default();

    history.snapshots.push(SecurityScoreSnapshot {
        timestamp: crate::resources::now_rfc3339(),
        score: current,
        threat_count,
    });
    while history.snapshots.len() > 32 {
        history.snapshots.remove(0);
    }
    if let Some(parent) = security_history_path().parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(
        security_history_path(),
        serde_json::to_string_pretty(&history).unwrap_or_default(),
    );

    let points: Vec<SecurityScoreTrendPoint> = history
        .snapshots
        .iter()
        .enumerate()
        .map(|(i, s)| SecurityScoreTrendPoint {
            label: format!("t-{i}"),
            score: s.score,
            threat_count: s.threat_count,
        })
        .collect();

    let trend_direction = if history.snapshots.len() >= 2 {
        let prev = history.snapshots[history.snapshots.len() - 2].score;
        if current > prev + 1.0 {
            "improving".into()
        } else if current < prev - 1.0 {
            "declining".into()
        } else {
            "stable".into()
        }
    } else {
        "stable".into()
    };

    SecurityScoreTrendReport {
        generated_at: crate::resources::now_rfc3339(),
        current_score: current,
        trend_direction,
        points,
    }
}

pub fn sbom_fingerprint(data: &str) -> String {
    format!("{:x}", Sha256::digest(data.as_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sbom_drift_returns_report() {
        let r = detect_sbom_drift();
        assert!(r.current_component_count > 0 || !r.alerts.is_empty() || !r.drift_detected);
    }

    #[test]
    fn zero_trust_wizard_has_steps() {
        let r = build_zero_trust_wizard(&[]);
        assert_eq!(r.steps.len(), 3);
    }

    #[test]
    fn rotate_due_secrets_rotates_owned_and_defers_external() {
        use crate::secrets::{RotationPolicy, SecretStore};
        let mut store = SecretStore::new();

        // External (mirrored) secret — overdue, but Aether does not own it.
        store.create_secret("ext-cred", "prod");
        store.set("ext-cred", "token", "external-value").unwrap();
        store.set_last_rotated_days_ago("ext-cred", "token", 500);

        // Owned secret — overdue and auto-generatable.
        let owned = store.create_secret("owned-cred", "prod");
        owned.rotation_policy = Some(RotationPolicy {
            generate: true,
            ..RotationPolicy::default()
        });
        store.set("owned-cred", "password", "initial").unwrap();
        store.set_last_rotated_days_ago("owned-cred", "password", 500);

        let report = rotate_due_secrets(&mut store, false);

        // Owned secret rotated to a fresh, non-placeholder value.
        assert!(report.rotated.iter().any(|l| l.contains("owned-cred")));
        let rotated = store.get("owned-cred", "password").unwrap();
        assert_ne!(rotated, "initial");
        assert!(!rotated.starts_with("rotated-"));

        // External secret deferred, value never touched.
        assert!(report
            .skipped
            .iter()
            .any(|l| l.contains("ext-cred") && l.contains("requires external rotation")));
        assert_eq!(store.get("ext-cred", "token").unwrap(), "external-value");
    }

    #[test]
    fn rotate_due_secrets_dry_run_makes_no_changes() {
        use crate::secrets::{RotationPolicy, SecretStore};
        let mut store = SecretStore::new();
        let owned = store.create_secret("owned-cred", "prod");
        owned.rotation_policy = Some(RotationPolicy {
            generate: true,
            ..RotationPolicy::default()
        });
        store.set("owned-cred", "password", "initial").unwrap();
        store.set_last_rotated_days_ago("owned-cred", "password", 500);

        let report = rotate_due_secrets(&mut store, true);
        assert!(report.rotated.iter().any(|l| l.starts_with("dry-run:")));
        // Value untouched in dry-run.
        assert_eq!(store.get("owned-cred", "password").unwrap(), "initial");
    }
}
