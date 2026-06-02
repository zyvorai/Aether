// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Kata / Confidential Containers RuntimeClass path.

use crate::ragnarok::tee::{probe_host_tee, HostTeeStatus};
use crate::spec::{ConfidentialTee, RuntimePreference, RuntimeType, Workload};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Cloud Hypervisor microVM runtime (preferred).
pub const RUNTIME_CLASS_CLH_SNP: &str = "kata-clh-snp";
pub const RUNTIME_CLASS_CLH_TDX: &str = "kata-clh-tdx";
pub const RUNTIME_CLASS_CLH_GPU_SNP: &str = "kata-clh-gpu-snp";

/// Legacy QEMU runtime (still supported).
pub const RUNTIME_CLASS_QEMU_SNP: &str = "kata-qemu-snp";
pub const RUNTIME_CLASS_QEMU_TDX: &str = "kata-qemu-tdx";

pub const RUNTIME_CLASS_SNP: &str = RUNTIME_CLASS_CLH_SNP;
pub const RUNTIME_CLASS_TDX: &str = RUNTIME_CLASS_CLH_TDX;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum KataHypervisor {
    #[default]
    CloudHypervisor,
    Qemu,
}

impl KataHypervisor {
    pub fn from_env() -> Self {
        match std::env::var("AETHER_KATA_HYPERVISOR")
            .unwrap_or_else(|_| "clh".into())
            .to_lowercase()
            .as_str()
        {
            "qemu" => Self::Qemu,
            _ => Self::CloudHypervisor,
        }
    }

    pub fn runtime_class(&self, tee: &ConfidentialTee) -> &'static str {
        match (self, *tee) {
            (Self::CloudHypervisor, ConfidentialTee::Tdx) => RUNTIME_CLASS_CLH_TDX,
            (Self::CloudHypervisor, _) => RUNTIME_CLASS_CLH_SNP,
            (Self::Qemu, ConfidentialTee::Tdx) => RUNTIME_CLASS_QEMU_TDX,
            (Self::Qemu, _) => RUNTIME_CLASS_QEMU_SNP,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KataStatus {
    pub supported_runtime_classes: Vec<String>,
    pub operator_requirements: Vec<String>,
    pub hypervisor: String,
    pub host_tee: HostTeeStatus,
    pub placement_ready: bool,
    pub notes: Vec<String>,
}

pub fn supported_runtime_classes() -> Vec<&'static str> {
    vec![
        RUNTIME_CLASS_CLH_SNP,
        RUNTIME_CLASS_CLH_TDX,
        RUNTIME_CLASS_CLH_GPU_SNP,
        RUNTIME_CLASS_QEMU_SNP,
        RUNTIME_CLASS_QEMU_TDX,
    ]
}

pub fn is_kubernetes_confidential(spec: &Workload) -> bool {
    let Some(conf) = spec.confidential.as_ref() else {
        return false;
    };
    if !conf.enabled {
        return false;
    }
    spec.runtime.allow.contains(&RuntimeType::Kube)
        || matches!(spec.runtime.preferred, RuntimePreference::Kube)
}

fn runtime_class_for_security_profile(profile: &str, tee: &ConfidentialTee) -> Option<String> {
    match profile {
        "sandbox" => None,
        "sovereign-high" | "standard-confidential" => {
            Some(KataHypervisor::from_env().runtime_class(tee).to_string())
        }
        _ => None,
    }
}

pub fn resolve_runtime_class(spec: &Workload) -> Option<String> {
    let conf = spec.confidential.as_ref()?;
    if !conf.enabled || !is_kubernetes_confidential(spec) {
        return None;
    }
    if let Some(ref profile) = conf.security_profile {
        if let Some(rc) = runtime_class_for_security_profile(profile, &conf.tee) {
            return Some(rc);
        }
    }
    if let Some(ref rc) = conf.kata_runtime_class {
        return Some(rc.clone());
    }
    Some(
        KataHypervisor::from_env()
            .runtime_class(&conf.tee)
            .to_string(),
    )
}

pub fn placement_score(spec: &Workload, host: &HostTeeStatus) -> f64 {
    let Some(conf) = spec.confidential.as_ref() else {
        return 0.0;
    };
    if !conf.enabled || !is_kubernetes_confidential(spec) {
        return 0.0;
    }
    let mut score: f64 = 0.5;
    match conf.tee {
        ConfidentialTee::Tdx if host.tdx => score += 0.45_f64,
        ConfidentialTee::SevSnp if host.sev_snp => score += 0.45_f64,
        ConfidentialTee::Sev | ConfidentialTee::SevEs if host.sev_device => score += 0.35_f64,
        _ => score += 0.1_f64,
    }
    if conf.kata_runtime_class.is_some() {
        score += 0.05_f64;
    }
    score.clamp(0.0_f64, 1.0_f64)
}

pub fn kata_status() -> KataStatus {
    let host = probe_host_tee();
    let hypervisor = KataHypervisor::from_env();
    let placement_ready = host.sev_snp || host.tdx || host.sev_device;
    let mut notes = Vec::new();
    if !placement_ready {
        notes.push(
            "No host TEE detected — Kata confidential pods need SNP/TDX-capable nodes".into(),
        );
    }
    notes.push(
        "Install RuntimeClasses from deploy/confidential/kata-clh/runtime-classes.yaml".into(),
    );
    KataStatus {
        supported_runtime_classes: supported_runtime_classes()
            .into_iter()
            .map(String::from)
            .collect(),
        operator_requirements: operator_requirements()
            .into_iter()
            .map(String::from)
            .collect(),
        hypervisor: match hypervisor {
            KataHypervisor::CloudHypervisor => "cloud-hypervisor".into(),
            KataHypervisor::Qemu => "qemu".into(),
        },
        host_tee: host,
        placement_ready,
        notes,
    }
}

pub fn deploy_kata_gate(spec: &Workload) -> anyhow::Result<()> {
    if !is_kubernetes_confidential(spec) {
        return Ok(());
    }
    let _ = resolve_runtime_class(spec).ok_or_else(|| {
        anyhow::anyhow!("kubernetes confidential workload missing kata RuntimeClass")
    })?;
    Ok(())
}

pub fn confidential_pod_patch(spec: &Workload) -> Option<Value> {
    let conf = spec.confidential.as_ref()?;
    if !conf.enabled {
        return None;
    }
    let runtime_class = if is_kubernetes_confidential(spec) {
        resolve_runtime_class(spec)?
    } else {
        return None;
    };
    let hypervisor = KataHypervisor::from_env();
    Some(json!({
        "runtimeClassName": runtime_class,
        "metadata": {
            "labels": {
                "ragnarok.zyvor.dev/confidential": "true",
                "ragnarok.zyvor.dev/tee": serde_json::to_value(&conf.tee).ok(),
                "ragnarok.zyvor.dev/kata-hypervisor": match hypervisor {
                    KataHypervisor::CloudHypervisor => "clh",
                    KataHypervisor::Qemu => "qemu",
                },
                "ragnarok.zyvor.dev/runtime-class": runtime_class,
            }
        }
    }))
}

pub fn operator_requirements() -> Vec<&'static str> {
    vec![
        "confidential-containers-operator",
        "trustee",
        "kbs",
        "cloud-hypervisor (for kata-clh-* runtime classes)",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{
        BuildSpec, ConfidentialAttestationSpec, ConfidentialIsolationSpec, ConfidentialSecretsSpec,
        ConfidentialSpec, ConfidentialTee, Metadata, NetworkSpec, PersistenceSpec,
        ResourceRequirements, RuntimePreference, RuntimeSpec, RuntimeType, Workload,
    };

    fn kube_confidential_spec(class: Option<&str>) -> Workload {
        Workload {
            api_version: "aether/v1".into(),
            kind: "Workload".into(),
            metadata: Metadata {
                name: "kata-test".into(),
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
                preferred: RuntimePreference::Kube,
                allow: vec![RuntimeType::Kube],
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
                region_lock: None,
                kata_runtime_class: class.map(String::from),
                security_profile: None,
            }),
            schedule: None,
            kubernetes: None,
        }
    }

    #[test]
    fn resolve_explicit_runtime_class() {
        let spec = kube_confidential_spec(Some("kata-clh-tdx"));
        assert_eq!(
            resolve_runtime_class(&spec).as_deref(),
            Some("kata-clh-tdx")
        );
    }

    #[test]
    fn resolve_default_clh_snp() {
        let spec = kube_confidential_spec(None);
        assert_eq!(
            resolve_runtime_class(&spec).as_deref(),
            Some(RUNTIME_CLASS_CLH_SNP)
        );
    }
}
