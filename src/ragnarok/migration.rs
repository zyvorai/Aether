//! Confidential live migration helpers (hyper2kvm integration surface).

use crate::migration::MigrationStrategy;
use crate::spec::Workload;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialMigrationPlan {
    pub source_tee_capable: bool,
    pub target_tee_capable: bool,
    pub encrypted_channel_required: bool,
    pub re_attestation_on_target: bool,
    pub blockers: Vec<String>,
    pub recommended_strategy: MigrationStrategy,
}

pub fn plan_confidential_migration(
    spec: &Workload,
    source_snp: bool,
    target_snp: bool,
) -> ConfidentialMigrationPlan {
    let confidential = spec
        .confidential
        .as_ref()
        .map(|c| c.enabled)
        .unwrap_or(false);

    let mut blockers = Vec::new();
    if confidential && !source_snp {
        blockers.push("Source host lacks SEV-SNP capability".into());
    }
    if confidential && !target_snp {
        blockers.push("Target host lacks SEV-SNP capability".into());
    }

    ConfidentialMigrationPlan {
        source_tee_capable: source_snp,
        target_tee_capable: target_snp,
        encrypted_channel_required: confidential,
        re_attestation_on_target: confidential,
        blockers: blockers.clone(),
        recommended_strategy: if confidential && blockers.is_empty() {
            MigrationStrategy::ConfidentialBlueGreen
        } else if confidential {
            MigrationStrategy::Immediate
        } else {
            MigrationStrategy::BlueGreen
        },
    }
}

pub fn migration_uri_wrapper(base_uri: &str, encrypted: bool) -> String {
    if encrypted {
        format!("tls+sev://{base_uri}")
    } else {
        base_uri.to_string()
    }
}
