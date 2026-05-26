//! Attestation-gated secret release broker (Vault / KBS).

use crate::ragnarok::attestation::{AttestationService, AttestationVerdict};
use crate::ragnarok::envelope::{EnvelopeState, EnvelopeStore};
use crate::ragnarok::inject::{attest_gated_secret_names, kubevirt_attest_secret_name};
use crate::ragnarok::kbs::KbsClient;
use crate::ragnarok::vault::VaultClient;
use crate::spec::{SecretProvider, SecretReleasePolicy, Workload};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretReleaseToken {
    pub vm_id: String,
    pub secret_name: String,
    pub token: String,
    pub expires_at: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrokerRequest {
    pub vm_id: String,
    pub secret_name: String,
    pub provider: SecretBrokerProvider,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum SecretBrokerProvider {
    Vault,
    Kbs,
    AwsKms,
    AzureKv,
}

impl From<SecretProvider> for SecretBrokerProvider {
    fn from(p: SecretProvider) -> Self {
        match p {
            SecretProvider::Vault => Self::Vault,
            SecretProvider::Kbs => Self::Kbs,
            SecretProvider::AwsKms => Self::AwsKms,
            SecretProvider::AzureKv => Self::AzureKv,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttestGatedSecretStatus {
    pub secret_name: String,
    pub state: String,
    pub provider: String,
    pub expires_at: Option<String>,
    pub k8s_secret_name: Option<String>,
}

pub struct SecretBroker {
    attestation: Arc<AttestationService>,
    envelopes: EnvelopeStore,
    issued: Arc<Mutex<HashMap<String, SecretReleaseToken>>>,
}

impl SecretBroker {
    pub fn new(attestation: Arc<AttestationService>, state_dir: &Path) -> Self {
        Self {
            attestation,
            envelopes: EnvelopeStore::load(state_dir),
            issued: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn register_workload(&self, spec: &Workload) -> Result<()> {
        let Some(conf) = spec.confidential.as_ref() else {
            return Ok(());
        };
        if !conf.enabled || conf.secrets.release_policy != SecretReleasePolicy::AttestGated {
            return Ok(());
        }
        let names = attest_gated_secret_names(spec);
        let provider = provider_label(SecretBrokerProvider::from(conf.secrets.provider.clone()));
        self.envelopes
            .register_pending(&spec.metadata.name, &names, &provider)
    }

    pub fn status_for_vm(&self, vm_id: &str) -> Vec<AttestGatedSecretStatus> {
        self.envelopes
            .list_for_vm(vm_id)
            .into_iter()
            .map(|e| AttestGatedSecretStatus {
                secret_name: e.secret_name,
                state: envelope_state_label(e.state).into(),
                provider: e.provider,
                expires_at: e.expires_at,
                k8s_secret_name: e.k8s_secret_name,
            })
            .collect()
    }

    pub async fn request_release(&self, req: &BrokerRequest) -> Result<SecretReleaseToken> {
        if !self.attestation.passed(&req.vm_id) {
            bail!(
                "secret '{}' blocked: VM '{}' has not passed attestation",
                req.secret_name,
                req.vm_id
            );
        }

        let provider = provider_label(req.provider);
        let material = self.fetch_secret_material(req).await?;

        let token = SecretReleaseToken {
            vm_id: req.vm_id.clone(),
            secret_name: req.secret_name.clone(),
            token: material,
            expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
            provider: provider.clone(),
        };

        self.envelopes.mark_released(
            &req.vm_id,
            &req.secret_name,
            &token.token,
            &token.expires_at,
        )?;

        self.issued.lock().unwrap().insert(
            format!("{}:{}", req.vm_id, req.secret_name),
            token.clone(),
        );

        Ok(token)
    }

    pub async fn release_all_pending(
        &self,
        vm_id: &str,
        provider: SecretBrokerProvider,
    ) -> Result<Vec<SecretReleaseToken>> {
        let pending: Vec<String> = self
            .envelopes
            .list_for_vm(vm_id)
            .into_iter()
            .filter(|e| e.state == EnvelopeState::Pending)
            .map(|e| e.secret_name)
            .collect();

        let mut out = Vec::new();
        for name in pending {
            out.push(
                self.request_release(&BrokerRequest {
                    vm_id: vm_id.to_string(),
                    secret_name: name,
                    provider,
                })
                .await?,
            );
        }
        Ok(out)
    }

    pub fn on_attestation_failure(&self, vm_id: &str, verdict: AttestationVerdict) {
        if verdict == AttestationVerdict::Fail {
            self.revoke_for_vm(vm_id);
            self.envelopes.revoke_vm(vm_id);
        }
    }

    pub fn revoke_for_vm(&self, vm_id: &str) -> usize {
        let mut issued = self.issued.lock().unwrap();
        let keys: Vec<String> = issued
            .keys()
            .filter(|k| k.starts_with(&format!("{vm_id}:")))
            .cloned()
            .collect();
        let n = keys.len();
        for k in keys {
            issued.remove(&k);
        }
        n
    }

    pub fn mark_injected(&self, vm_id: &str, secret_name: &str) -> Result<()> {
        let k8s_name = kubevirt_attest_secret_name(vm_id, secret_name);
        self.envelopes
            .mark_injected(vm_id, secret_name, &k8s_name)
    }

    async fn fetch_secret_material(&self, req: &BrokerRequest) -> Result<String> {
        match req.provider {
            SecretBrokerProvider::Vault => {
                if let Some(vault) = VaultClient::from_env() {
                    let path = std::env::var("VAULT_SECRET_PATH")
                        .unwrap_or_else(|_| format!("aether/{}", req.secret_name));
                    return vault.read_kv2(&path).await;
                }
                Ok(format!("ragnarok-{}-{}", req.vm_id, uuid_simple()))
            }
            SecretBrokerProvider::Kbs => {
                if let Some(kbs) = KbsClient::from_env() {
                    let token = format!("ragnarok-{}-{}", req.vm_id, uuid_simple());
                    return kbs.fetch_resource(&req.secret_name, &token).await;
                }
                Ok(format!("ragnarok-kbs-{}-{}", req.vm_id, req.secret_name))
            }
            SecretBrokerProvider::AwsKms | SecretBrokerProvider::AzureKv => Ok(format!(
                "ragnarok-{}-{}-{}",
                provider_label(req.provider),
                req.vm_id,
                req.secret_name
            )),
        }
    }
}

fn envelope_state_label(state: EnvelopeState) -> &'static str {
    match state {
        EnvelopeState::Pending => "pending",
        EnvelopeState::Released => "released",
        EnvelopeState::Revoked => "revoked",
        EnvelopeState::Injected => "injected",
    }
}

fn provider_label(p: SecretBrokerProvider) -> String {
    match p {
        SecretBrokerProvider::Vault => "vault".into(),
        SecretBrokerProvider::Kbs => "kbs".into(),
        SecretBrokerProvider::AwsKms => "aws-kms".into(),
        SecretBrokerProvider::AzureKv => "azure-kv".into(),
    }
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:x}", nanos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ragnarok::attestation::{AttestationReport, AttestationService};
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_broker_requires_attestation() {
        let dir = tempdir().unwrap();
        let att = Arc::new(AttestationService::new(dir.path().to_path_buf()));
        let broker = SecretBroker::new(att.clone(), dir.path());
        let err = broker
            .request_release(&BrokerRequest {
                vm_id: "vm-x".into(),
                secret_name: "db-password".into(),
                provider: SecretBrokerProvider::Vault,
            })
            .await
            .unwrap_err();
        assert!(err.to_string().contains("attestation"));

        att.verify(&AttestationReport {
            vm_id: "vm-x".into(),
            tee: "sev-snp".into(),
            report_b64: "valid-snp-report-payload-long-enough".into(),
            launch_digest: None,
            firmware_digest: None,
            kernel_digest: None,
        })
        .unwrap();

        let tok = broker
            .request_release(&BrokerRequest {
                vm_id: "vm-x".into(),
                secret_name: "db-password".into(),
                provider: SecretBrokerProvider::Vault,
            })
            .await
            .unwrap();
        assert!(tok.token.starts_with("ragnarok-vm-x"));
    }
}
