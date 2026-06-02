// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Confidential live migration helpers (hyper2kvm integration surface).

use crate::migration::MigrationStrategy;
use crate::ragnarok::{
    attestation::AttestationService,
    image::ImageCatalog,
    tee::{probe_host_tee, HostTeeStatus},
};
use crate::spec::Workload;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum MigrationPhase {
    Plan,
    DeployTarget,
    ReAttest,
    Cutover,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialMigrationPlan {
    pub workload: String,
    pub source_tee_capable: bool,
    pub target_tee_capable: bool,
    pub source_tee: HostTeeStatus,
    pub target_tee: HostTeeStatus,
    pub encrypted_channel_required: bool,
    pub re_attestation_on_target: bool,
    pub launch_digest: Option<String>,
    pub launch_digest_preserved: bool,
    pub migration_uri: String,
    pub encrypted_migration_uri: String,
    pub blockers: Vec<String>,
    pub phases: Vec<String>,
    pub hyper2kvm_hints: Vec<String>,
    pub recommended_strategy: MigrationStrategy,
    pub ready_for_cutover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct MigrationStore {
    records: HashMap<String, ConfidentialMigrationRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialMigrationRecord {
    pub workload: String,
    pub phase: MigrationPhase,
    pub target_runtime: String,
    pub migration_uri: String,
    pub started_at: String,
    pub attestation_required: bool,
    pub cutover_ready: bool,
    pub error: Option<String>,
}

#[derive(Clone)]
pub struct ConfidentialMigrationStore {
    path: PathBuf,
    inner: Arc<Mutex<MigrationStore>>,
}

impl ConfidentialMigrationStore {
    pub fn new(state_dir: &Path) -> Self {
        let path = state_dir.join("ragnarok-migration.json");
        let inner = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            MigrationStore::default()
        };
        Self {
            path,
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn upsert(&self, record: ConfidentialMigrationRecord) -> Result<()> {
        self.inner
            .lock()
            .unwrap()
            .records
            .insert(record.workload.clone(), record);
        self.persist()
    }

    pub fn get(&self, workload: &str) -> Option<ConfidentialMigrationRecord> {
        self.inner.lock().unwrap().records.get(workload).cloned()
    }

    fn persist(&self) -> Result<()> {
        let store = self.inner.lock().unwrap();
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, serde_json::to_string_pretty(&*store)?)?;
        Ok(())
    }
}

pub fn plan_confidential_migration(
    spec: &Workload,
    source_snp: bool,
    target_snp: bool,
) -> ConfidentialMigrationPlan {
    plan_confidential_migration_tee(spec, source_snp, target_snp, false, false)
}

pub fn plan_confidential_migration_tee(
    spec: &Workload,
    source_snp: bool,
    target_snp: bool,
    source_tdx: bool,
    target_tdx: bool,
) -> ConfidentialMigrationPlan {
    let confidential = spec
        .confidential
        .as_ref()
        .map(|c| c.enabled)
        .unwrap_or(false);
    let digest = spec
        .confidential
        .as_ref()
        .and_then(|c| c.image_digest.clone());

    let source_tee = HostTeeStatus {
        sev_snp: source_snp,
        tdx: source_tdx,
        ..probe_host_tee()
    };
    let target_tee = HostTeeStatus {
        sev_snp: target_snp,
        tdx: target_tdx,
        ..probe_host_tee()
    };

    let mut blockers = Vec::new();
    if confidential && !source_snp && !source_tdx {
        blockers.push("Source host lacks SEV-SNP/TDX for confidential migration".into());
    }
    if confidential && !target_snp && !target_tdx {
        blockers.push("Target host lacks SEV-SNP/TDX for confidential migration".into());
    }
    if confidential {
        if let Some(ref d) = digest {
            let catalog = ImageCatalog::load(
                &crate::ragnarok::client::RagnarokClient::attestation_data_dir(),
            );
            if !catalog.verify_digest(d) {
                blockers.push(format!(
                    "Launch digest {d} not in measured catalog — re-sign before migrate"
                ));
            }
        } else if spec
            .confidential
            .as_ref()
            .is_some_and(|c| c.attestation.policy == crate::spec::AttestationPolicy::Strict)
        {
            blockers.push("Strict confidential policy requires imageDigest for migration".into());
        }
    }

    let base_uri = format!(
        "qemu+tcp://migrate/{}/{}",
        spec.metadata.name, spec.metadata.name
    );
    let migration_uri = migration_uri_wrapper(&base_uri, false);
    let encrypted_migration_uri = migration_uri_wrapper(&base_uri, confidential);

    let phases = if confidential {
        vec![
            "deploy-target-on-tee-node".into(),
            "establish-tls+sev-migration-channel".into(),
            "re-attest-target-launch-digest".into(),
            "cutover-traffic".into(),
            "retire-source".into(),
        ]
    } else {
        vec!["standard-blue-green".into()]
    };

    let hyper2kvm_hints = hyper2kvm_command_hints(spec, &encrypted_migration_uri);

    let recommended_strategy = if confidential && blockers.is_empty() {
        MigrationStrategy::ConfidentialBlueGreen
    } else if confidential {
        MigrationStrategy::Immediate
    } else {
        MigrationStrategy::BlueGreen
    };

    ConfidentialMigrationPlan {
        workload: spec.metadata.name.clone(),
        source_tee_capable: source_snp || source_tdx,
        target_tee_capable: target_snp || target_tdx,
        source_tee,
        target_tee,
        encrypted_channel_required: confidential,
        re_attestation_on_target: confidential
            && spec
                .confidential
                .as_ref()
                .is_some_and(|c| c.attestation.required),
        launch_digest: digest.clone(),
        launch_digest_preserved: digest.is_some(),
        migration_uri,
        encrypted_migration_uri,
        blockers: blockers.clone(),
        phases,
        hyper2kvm_hints,
        recommended_strategy,
        ready_for_cutover: blockers.is_empty(),
    }
}

/// Whether the hyper2kvm operator tool is on PATH or `HYPER2KVM_BIN`.
pub fn hyper2kvm_available() -> bool {
    if let Ok(bin) = std::env::var("HYPER2KVM_BIN") {
        if !bin.trim().is_empty() && std::path::Path::new(&bin).exists() {
            return true;
        }
    }
    std::process::Command::new("hyper2kvm")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn hyper2kvm_command_hints(spec: &Workload, encrypted_uri: &str) -> Vec<String> {
    let mut hints = vec![
        "hyper2kvm migrate --source <vm> --dest <target> --bandwidth-limit 1G".into(),
        format!("Use encrypted channel URI: {encrypted_uri}"),
        "Preserve launch digest in guest firmware config during stream".into(),
    ];
    if hyper2kvm_available() {
        hints.push("hyper2kvm detected on host — run migrate with encrypted URI above".into());
    } else {
        hints.push(
            "Install hyper2kvm or set HYPER2KVM_BIN for live encrypted memory migration".into(),
        );
    }
    if let Some(ref d) = spec
        .confidential
        .as_ref()
        .and_then(|c| c.image_digest.as_ref())
    {
        hints.push(format!(
            "Verify target attestation reports digest {d} before cutover"
        ));
    }
    hints.push("Set RAGNAROK_MIGRATION_ENCRYPTED=1 on both hosts".into());
    hints
}

pub fn migration_uri_wrapper(base_uri: &str, encrypted: bool) -> String {
    if encrypted {
        format!("tls+sev://{base_uri}")
    } else {
        base_uri.to_string()
    }
}

pub fn pre_migrate_gate(spec: &Workload, plan: &ConfidentialMigrationPlan) -> Result<()> {
    if !plan.blockers.is_empty() {
        bail!(
            "confidential migration blocked: {}",
            plan.blockers.join("; ")
        );
    }
    if plan.encrypted_channel_required && !plan.target_tee_capable {
        bail!("target host is not TEE-capable for encrypted migration");
    }
    if spec.confidential.as_ref().is_some_and(|c| c.enabled) {
        crate::ragnarok::sovereign::deploy_sovereign_gate(spec)?;
    }
    Ok(())
}

pub fn attestation_ready_for_cutover(
    workload: &str,
    attestation: &AttestationService,
    required: bool,
) -> bool {
    if !required {
        return true;
    }
    attestation.passed(workload)
}

pub async fn wait_for_re_attestation(
    workload: &str,
    attestation: &AttestationService,
    timeout_secs: u64,
) -> bool {
    if timeout_secs == 0 {
        return attestation.passed(workload);
    }
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    while tokio::time::Instant::now() < deadline {
        if attestation.passed(workload) {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    attestation.passed(workload)
}

pub fn record_migration_start(
    store: &ConfidentialMigrationStore,
    spec: &Workload,
    plan: &ConfidentialMigrationPlan,
    target_runtime: &str,
) -> Result<()> {
    store.upsert(ConfidentialMigrationRecord {
        workload: spec.metadata.name.clone(),
        phase: MigrationPhase::DeployTarget,
        target_runtime: target_runtime.to_string(),
        migration_uri: plan.encrypted_migration_uri.clone(),
        started_at: chrono::Utc::now().to_rfc3339(),
        attestation_required: plan.re_attestation_on_target,
        cutover_ready: false,
        error: None,
    })
}

pub fn record_migration_complete(
    store: &ConfidentialMigrationStore,
    workload: &str,
    success: bool,
    error: Option<String>,
) -> Result<()> {
    let mut rec = store
        .get(workload)
        .context("no confidential migration record")?;
    rec.phase = if success {
        MigrationPhase::Complete
    } else {
        MigrationPhase::Failed
    };
    rec.cutover_ready = success;
    rec.error = error;
    store.upsert(rec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{
        BuildSpec, ConfidentialAttestationSpec, ConfidentialIsolationSpec, ConfidentialSecretsSpec,
        ConfidentialSpec, ConfidentialTee, Metadata, NetworkSpec, PersistenceSpec,
        ResourceRequirements, RuntimePreference, RuntimeSpec, RuntimeType, Workload,
    };

    fn conf_spec() -> Workload {
        Workload {
            api_version: "aether/v1".into(),
            kind: "Workload".into(),
            metadata: Metadata {
                name: "mig-vm".into(),
                owner: "t".into(),
                project: "t".into(),
                labels: Default::default(),
                annotations: Default::default(),
            },
            build: BuildSpec::default(),
            requirements: ResourceRequirements {
                cpu: "1".into(),
                memory: "1Gi".into(),
                storage: "10Gi".into(),
                gpu: None,
                cpu_request: None,
                memory_request: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Kubevirt,
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
                attestation: ConfidentialAttestationSpec {
                    required: true,
                    policy: crate::spec::AttestationPolicy::Standard,
                },
                isolation: ConfidentialIsolationSpec::default(),
                secrets: ConfidentialSecretsSpec::default(),
                image_digest: None,
                region_lock: None,
                kata_runtime_class: None,
                security_profile: None,
            }),
            schedule: None,
            kubernetes: None,
        }
    }

    #[test]
    fn encrypted_uri_wrapper_prefix() {
        let u = migration_uri_wrapper("qemu+tcp://host/vm", true);
        assert!(u.starts_with("tls+sev://"));
    }

    #[test]
    fn plan_recommends_confidential_blue_green() {
        let spec = conf_spec();
        let plan = plan_confidential_migration_tee(&spec, true, true, false, false);
        assert_eq!(
            plan.recommended_strategy,
            MigrationStrategy::ConfidentialBlueGreen
        );
        assert!(plan.encrypted_channel_required);
    }

    #[test]
    fn migration_store_roundtrip() {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let store = ConfidentialMigrationStore::new(dir.path());
        store
            .upsert(ConfidentialMigrationRecord {
                workload: "vm1".into(),
                phase: MigrationPhase::ReAttest,
                target_runtime: "kubevirt".into(),
                migration_uri: "tls+sev://test".into(),
                started_at: chrono::Utc::now().to_rfc3339(),
                attestation_required: true,
                cutover_ready: false,
                error: None,
            })
            .unwrap();
        assert_eq!(store.get("vm1").unwrap().phase, MigrationPhase::ReAttest);
    }
}
