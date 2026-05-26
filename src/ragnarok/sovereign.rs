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
