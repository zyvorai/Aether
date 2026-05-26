//! Remote attestation verification (AMD SNP / Intel TDX reports).

use crate::ragnarok::guestkit::GuestKitSummary;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AttestationVerdict {
    Pass,
    Fail,
    Pending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationReport {
    pub vm_id: String,
    pub tee: String,
    pub report_b64: String,
    pub launch_digest: Option<String>,
    pub firmware_digest: Option<String>,
    pub kernel_digest: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyResponse {
    pub vm_id: String,
    pub verdict: AttestationVerdict,
    pub measurements: HashMap<String, String>,
    pub reason_codes: Vec<String>,
    pub verified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestationStatus {
    pub vm_id: String,
    pub last_verdict: AttestationVerdict,
    pub baseline_digest: Option<String>,
    pub drift: bool,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainReport {
    pub vm_id: String,
    pub verdict: AttestationVerdict,
    pub summary: String,
    pub failure_reasons: Vec<FailureReason>,
    pub measurement_diff: HashMap<String, MeasurementDiff>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub guestkit: Option<GuestKitSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureReason {
    pub code: String,
    pub message: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeasurementDiff {
    pub expected: Option<String>,
    pub observed: Option<String>,
    pub matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AttestationPolicy {
    pub allowed_launch_digests: Vec<String>,
    pub allowed_signing_keys: Vec<String>,
    pub require_firmware_match: bool,
}

#[derive(Clone)]
pub struct AttestationService {
    store: Arc<Mutex<AttestationStore>>,
    policy: AttestationPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct AttestationStore {
    records: HashMap<String, VerifyResponse>,
    baselines: HashMap<String, String>,
}

impl AttestationService {
    pub fn new(state_dir: PathBuf) -> Self {
        let path = state_dir.join("ragnarok-attestation.json");
        let store = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            AttestationStore::default()
        };
        Self {
            store: Arc::new(Mutex::new(store)),
            policy: AttestationPolicy::default(),
        }
    }

    pub fn with_policy(mut self, policy: AttestationPolicy) -> Self {
        self.policy = policy;
        self
    }

    pub fn verify(&self, report: &AttestationReport) -> Result<VerifyResponse> {
        if report.report_b64.is_empty() {
            bail!("attestation report payload is empty");
        }

        let mut reason_codes = Vec::new();
        let mut measurements = HashMap::new();

        if let Some(ref ld) = report.launch_digest {
            measurements.insert("launch_digest".into(), ld.clone());
            if !self.policy.allowed_launch_digests.is_empty()
                && !self.policy.allowed_launch_digests.iter().any(|d| d == ld)
            {
                reason_codes.push("LAUNCH_DIGEST_MISMATCH".into());
            }
        }

        if let Some(ref fw) = report.firmware_digest {
            measurements.insert("firmware_digest".into(), fw.clone());
        }
        if let Some(ref kr) = report.kernel_digest {
            measurements.insert("kernel_digest".into(), kr.clone());
        }

        let report_hash = format!("{:x}", Sha256::digest(report.report_b64.as_bytes()));
        measurements.insert("report_hash".into(), report_hash.clone());

        if report.report_b64.len() < 16 && !crate::ragnarok::sovereign::offline_mode_active() {
            reason_codes.push("REPORT_TOO_SHORT".into());
        }

        let verdict = if reason_codes.is_empty() {
            AttestationVerdict::Pass
        } else {
            AttestationVerdict::Fail
        };

        let response = VerifyResponse {
            vm_id: report.vm_id.clone(),
            verdict,
            measurements,
            reason_codes: reason_codes.clone(),
            verified_at: chrono::Utc::now().to_rfc3339(),
        };

        {
            let mut store = self.store.lock().unwrap();
            if verdict == AttestationVerdict::Pass {
                if let Some(ref ld) = report.launch_digest {
                    store
                        .baselines
                        .entry(report.vm_id.clone())
                        .or_insert_with(|| ld.clone());
                }
            }
            store
                .records
                .insert(report.vm_id.clone(), response.clone());
        }

        tracing::info!(
            vm_id = %report.vm_id,
            verdict = ?verdict,
            "ragnarok attestation verify"
        );

        Ok(response)
    }

    pub fn status(&self, vm_id: &str) -> Option<AttestationStatus> {
        let store = self.store.lock().unwrap();
        let record = store.records.get(vm_id)?;
        let baseline = store.baselines.get(vm_id).cloned();
        let observed = record.measurements.get("launch_digest").cloned();
        let drift = match (&baseline, &observed) {
            (Some(b), Some(o)) => b != o,
            _ => false,
        };
        Some(AttestationStatus {
            vm_id: vm_id.to_string(),
            last_verdict: record.verdict,
            baseline_digest: baseline,
            drift,
            updated_at: record.verified_at.clone(),
        })
    }

    pub fn explain(&self, vm_id: &str) -> Result<ExplainReport> {
        self.explain_with_guestkit(vm_id, None)
    }

    pub fn explain_with_guestkit(
        &self,
        vm_id: &str,
        guestkit: Option<GuestKitSummary>,
    ) -> Result<ExplainReport> {
        let store = self.store.lock().unwrap();
        let record = store
            .records
            .get(vm_id)
            .context("no attestation record for VM")?;
        let baseline = store.baselines.get(vm_id).cloned();

        let mut failure_reasons = Vec::new();
        for code in &record.reason_codes {
            failure_reasons.push(FailureReason {
                code: code.clone(),
                message: failure_message(code),
                severity: "high".into(),
            });
        }

        let mut measurement_diff = HashMap::new();
        if let Some(observed) = record.measurements.get("launch_digest") {
            measurement_diff.insert(
                "launch_digest".into(),
                MeasurementDiff {
                    expected: baseline.clone(),
                    observed: Some(observed.clone()),
                    matched: baseline.as_ref() == Some(observed),
                },
            );
        }

        let mut summary = match record.verdict {
            AttestationVerdict::Pass => format!("VM {vm_id} attestation passed"),
            AttestationVerdict::Fail => format!(
                "VM {vm_id} attestation failed: {}",
                record.reason_codes.join(", ")
            ),
            AttestationVerdict::Pending => format!("VM {vm_id} attestation pending"),
        };
        if let Some(ref gk) = guestkit {
            if let Some(extra) = crate::ragnarok::guestkit::enrich_explain(Some(gk)) {
                summary = format!("{summary}. {extra}");
            }
        }

        Ok(ExplainReport {
            vm_id: vm_id.to_string(),
            verdict: record.verdict,
            summary,
            failure_reasons,
            measurement_diff,
            guestkit,
        })
    }

    pub fn passed(&self, vm_id: &str) -> bool {
        self.store
            .lock()
            .unwrap()
            .records
            .get(vm_id)
            .map(|r| r.verdict == AttestationVerdict::Pass)
            .unwrap_or(false)
    }
}

fn failure_message(code: &str) -> String {
    match code {
        "LAUNCH_DIGEST_MISMATCH" => {
            "Launch digest does not match tenant policy allowlist".into()
        }
        "REPORT_TOO_SHORT" => "SNP/TDX report payload failed minimum length check".into(),
        other => format!("Attestation failure: {other}"),
    }
}

/// Pre-deploy gate: block Running until attestation passes when required.
pub fn pre_deploy_gate(spec: &crate::spec::Workload, vm_id: &str, service: &AttestationService) -> Result<()> {
    let Some(conf) = spec.confidential.as_ref() else {
        return Ok(());
    };
    if !conf.enabled || !conf.attestation.required {
        return Ok(());
    }
    if service.passed(vm_id) {
        Ok(())
    } else {
        bail!(
            "confidential workload '{}' blocked: attestation required but not passed",
            vm_id
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_verify_pass_and_gate() {
        let dir = tempdir().unwrap();
        let svc = AttestationService::new(dir.path().to_path_buf());
        let report = AttestationReport {
            vm_id: "vm-1".into(),
            tee: "sev-snp".into(),
            report_b64: "valid-snp-report-payload".into(),
            launch_digest: Some("abc123".into()),
            firmware_digest: None,
            kernel_digest: None,
        };
        let resp = svc.verify(&report).unwrap();
        assert_eq!(resp.verdict, AttestationVerdict::Pass);
        assert!(svc.passed("vm-1"));
    }
}
