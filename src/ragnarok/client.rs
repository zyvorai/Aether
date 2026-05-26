//! HTTP client to standalone Ragnarok (`RAGNAROK_URL`) for composite deployments.

use crate::ragnarok::attestation::{AttestationReport, AttestationService, VerifyResponse};
use anyhow::{Context, Result};
use std::path::PathBuf;

#[derive(Debug, Clone, serde::Deserialize, Default)]
struct TrustPolicy {
    #[serde(default)]
    require: Vec<String>,
}

impl TrustPolicy {
    fn from_env() -> Self {
        let raw = std::env::var("RAGNAROK_TRUST_POLICY").unwrap_or_default();
        if raw.trim().is_empty() {
            return Self::default();
        }
        serde_json::from_str(&raw).unwrap_or_default()
    }

    fn evaluate_workload(&self, spec: &crate::spec::Workload) -> Result<()> {
        let Some(conf) = spec.confidential.as_ref() else {
            return Ok(());
        };
        if self.require.is_empty() || !conf.enabled {
            return Ok(());
        }
        let mut violations = Vec::new();
        for req in &self.require {
            match req.as_str() {
                "attestation_required" if !conf.attestation.required => {
                    violations.push("attestation.required must be true");
                }
                "signed_image" if conf.image_digest.is_none() => {
                    violations.push("signed/measured image digest required");
                }
                "no_debug" | "secure_boot" if conf.isolation.debug_allowed => {
                    violations.push("debug not allowed by policy");
                }
                "confidential_compute" if !conf.enabled => {
                    violations.push("confidential_compute not enabled");
                }
                _ => {}
            }
        }
        if violations.is_empty() {
            Ok(())
        } else {
            anyhow::bail!(
                "Trust policy violation: {}",
                violations.join("; ")
            )
        }
    }
}

#[derive(Clone, Default)]
pub struct RagnarokClient {
    remote_base: Option<String>,
}

impl RagnarokClient {
    pub fn from_env() -> Self {
        Self {
            remote_base: std::env::var("RAGNAROK_URL")
                .ok()
                .filter(|s| !s.trim().is_empty()),
        }
    }

    pub fn is_remote(&self) -> bool {
        self.remote_base.is_some()
    }

    pub fn attestation_data_dir() -> PathBuf {
        std::env::var("RAGNAROK_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".aether")
            })
    }

    fn local(&self) -> AttestationService {
        AttestationService::new(Self::attestation_data_dir())
    }

    pub async fn verify(&self, report: &AttestationReport) -> Result<VerifyResponse> {
        if let Some(ref base) = self.remote_base {
            let url = format!(
                "{}/confidential/attestation/verify",
                base.trim_end_matches('/')
            );
            #[derive(serde::Deserialize)]
            struct Envelope {
                data: VerifyResponse,
            }
            let env: Envelope = reqwest::Client::new()
                .post(url)
                .json(report)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await
                .context("ragnarok attestation verify")?;
            return Ok(env.data);
        }
        self.local().verify(report)
    }

    pub fn passed(&self, vm_id: &str) -> bool {
        self.local().passed(vm_id)
    }
}

/// Gate confidential deploy: block only when local attestation store requires pass upfront.
pub async fn attestation_gate_for_workload(
    spec: &crate::spec::Workload,
    vm_id: &str,
) -> Result<()> {
    let Some(conf) = spec.confidential.as_ref() else {
        return Ok(());
    };
    if !conf.enabled {
        return Ok(());
    }

    TrustPolicy::from_env().evaluate_workload(spec)?;

    if !conf.attestation.required {
        return Ok(());
    }

    let client = RagnarokClient::from_env();
    if client.passed(vm_id) {
        return Ok(());
    }

    if client.is_remote() {
        tracing::info!(
            vm_id = %vm_id,
            "Confidential workload live; submit attestation to Ragnarok before secret release"
        );
        return Ok(());
    }

    crate::ragnarok::attestation::pre_deploy_gate(spec, vm_id, &client.local())
}
