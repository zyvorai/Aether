// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Sovereign cloud mode — BYOK, offline attestation, region lock.

use crate::spec::Workload;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SovereignConfig {
    pub byok_signing_key: Option<String>,
    pub offline_attestation: bool,
    pub embedded_cert_bundle: Option<String>,
    pub region_lock: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SovereignVerdict {
    pub compliant: bool,
    pub violations: Vec<String>,
    pub hints: Vec<String>,
    pub config: SovereignConfig,
}

impl SovereignConfig {
    pub fn from_env() -> Self {
        Self {
            byok_signing_key: std::env::var("RAGNAROK_BYOK_SIGNING_KEY").ok(),
            offline_attestation: std::env::var("RAGNAROK_OFFLINE_ATTESTATION")
                .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
                .unwrap_or(false),
            embedded_cert_bundle: std::env::var("RAGNAROK_CERT_BUNDLE").ok(),
            region_lock: std::env::var("RAGNAROK_REGION_LOCK").ok(),
        }
    }

    pub fn signing_key_id(&self, default: &str) -> String {
        self.byok_signing_key
            .clone()
            .filter(|k| !k.trim().is_empty())
            .unwrap_or_else(|| default.to_string())
    }
}

pub fn evaluate(spec: &Workload, config: &SovereignConfig) -> SovereignVerdict {
    let mut violations = Vec::new();
    let mut hints = Vec::new();

    if config.offline_attestation {
        hints.push("Offline attestation mode — CRL checks use embedded cert bundle".into());
        if config.embedded_cert_bundle.is_none() {
            violations
                .push("RAGNAROK_OFFLINE_ATTESTATION=1 requires RAGNAROK_CERT_BUNDLE path".into());
        }
    }

    if let Some(ref byok) = config.byok_signing_key {
        hints.push(format!("BYOK signing key active: {byok}"));
    }

    if let Err(e) = enforce_region_lock(spec, config) {
        violations.push(e.to_string());
    }

    if spec.confidential.as_ref().is_some_and(|c| c.enabled)
        && config.region_lock.is_some()
        && spec.confidential.as_ref().unwrap().region_lock.is_none()
    {
        hints.push("Set confidential.regionLock to pin workload to sovereign region".into());
    }

    SovereignVerdict {
        compliant: violations.is_empty(),
        violations,
        hints,
        config: config.clone(),
    }
}

pub fn deploy_sovereign_gate(spec: &Workload) -> anyhow::Result<()> {
    let config = SovereignConfig::from_env();
    let verdict = evaluate(spec, &config);
    if verdict.compliant {
        Ok(())
    } else {
        anyhow::bail!(
            "sovereign policy violation: {}",
            verdict.violations.join("; ")
        )
    }
}

pub fn enforce_region_lock(spec: &Workload, config: &SovereignConfig) -> anyhow::Result<()> {
    let Some(lock) = config.region_lock.as_ref() else {
        return Ok(());
    };
    if let Some(ref conf) = spec.confidential {
        if conf.enabled {
            if let Some(ref region) = conf.region_lock {
                if region != lock {
                    anyhow::bail!(
                        "sovereign region lock '{lock}' violated — workload pinned to '{region}'"
                    );
                }
            }
        }
    }
    Ok(())
}

pub fn offline_mode_active() -> bool {
    SovereignConfig::from_env().offline_attestation
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_lock_violation() {
        use crate::spec::{
            BuildSpec, ConfidentialAttestationSpec, ConfidentialIsolationSpec,
            ConfidentialSecretsSpec, ConfidentialSpec, ConfidentialTee, Metadata, NetworkSpec,
            PersistenceSpec, ResourceRequirements, RuntimePreference, RuntimeSpec, RuntimeType,
            Workload,
        };
        let spec = Workload {
            api_version: "aether/v1".into(),
            kind: "Workload".into(),
            metadata: Metadata {
                name: "sov".into(),
                owner: "o".into(),
                project: "p".into(),
                labels: Default::default(),
                annotations: Default::default(),
            },
            build: BuildSpec::default(),
            requirements: ResourceRequirements {
                cpu: "100m".into(),
                memory: "128Mi".into(),
                storage: "1Gi".into(),
                gpu: None,
                cpu_request: None,
                memory_request: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Auto,
                allow: vec![RuntimeType::Kubevirt],
            },
            network: NetworkSpec::default(),
            persistence: PersistenceSpec::default(),
            health: None,
            config: None,
            ingress: None,
            scaling: None,
            mesh: None,
            intent: None,
            autonomy: None,
            confidential: Some(ConfidentialSpec {
                enabled: true,
                tee: ConfidentialTee::SevSnp,
                attestation: ConfidentialAttestationSpec::default(),
                isolation: ConfidentialIsolationSpec::default(),
                secrets: ConfidentialSecretsSpec::default(),
                image_digest: None,
                region_lock: Some("eu-west".into()),
                kata_runtime_class: None,
                security_profile: None,
            }),
            schedule: None,
            kubernetes: None,
        };
        let config = SovereignConfig {
            region_lock: Some("us-sovereign".into()),
            ..Default::default()
        };
        assert!(enforce_region_lock(&spec, &config).is_err());
    }
}
