//! HTTP client to standalone Ragnarok (`RAGNAROK_URL`) for composite deployments.

use crate::ragnarok::attestation::{AttestationReport, AttestationService, VerifyResponse};
use anyhow::{Context, Result};
use std::path::PathBuf;

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
    if !conf.enabled || !conf.attestation.required {
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
