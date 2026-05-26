//! Attestation-gated secret release broker (Vault / KBS).

use crate::ragnarok::attestation::{AttestationService, AttestationVerdict};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
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

pub struct SecretBroker {
    attestation: Arc<AttestationService>,
    issued: Arc<Mutex<HashMap<String, SecretReleaseToken>>>,
}

impl SecretBroker {
    pub fn new(attestation: Arc<AttestationService>) -> Self {
        Self {
            attestation,
            issued: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn request_release(&self, req: &BrokerRequest) -> Result<SecretReleaseToken> {
        if !self.attestation.passed(&req.vm_id) {
            bail!(
                "secret '{}' blocked: VM '{}' has not passed attestation",
                req.secret_name,
                req.vm_id
            );
        }

        let provider = match req.provider {
            SecretBrokerProvider::Vault => "vault",
            SecretBrokerProvider::Kbs => "kbs",
            SecretBrokerProvider::AwsKms => "aws-kms",
            SecretBrokerProvider::AzureKv => "azure-kv",
        };

        let token = SecretReleaseToken {
            vm_id: req.vm_id.clone(),
            secret_name: req.secret_name.clone(),
            token: format!("ragnarok-{}-{}", req.vm_id, uuid_simple()),
            expires_at: (chrono::Utc::now() + chrono::Duration::minutes(15)).to_rfc3339(),
            provider: provider.into(),
        };

        self.issued
            .lock()
            .unwrap()
            .insert(format!("{}:{}", req.vm_id, req.secret_name), token.clone());

        Ok(token)
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

    pub fn on_attestation_failure(&self, vm_id: &str, verdict: AttestationVerdict) {
        if verdict == AttestationVerdict::Fail {
            self.revoke_for_vm(vm_id);
        }
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

    #[test]
    fn test_broker_requires_attestation() {
        let dir = tempdir().unwrap();
        let att = Arc::new(AttestationService::new(dir.path().to_path_buf()));
        let broker = SecretBroker::new(att.clone());
        let err = broker
            .request_release(&BrokerRequest {
                vm_id: "vm-x".into(),
                secret_name: "db-password".into(),
                provider: SecretBrokerProvider::Vault,
            })
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
            .unwrap();
        assert!(tok.token.starts_with("ragnarok-vm-x"));
    }
}
