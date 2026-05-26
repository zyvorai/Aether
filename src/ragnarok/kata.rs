//! Kata / Confidential Containers RuntimeClass path.

use crate::spec::Workload;
use serde_json::{json, Value};

/// Cloud Hypervisor microVM runtime (preferred).
pub const RUNTIME_CLASS_CLH_SNP: &str = "kata-clh-snp";
pub const RUNTIME_CLASS_CLH_TDX: &str = "kata-clh-tdx";

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

    pub fn runtime_class(&self, tee: crate::spec::ConfidentialTee) -> &'static str {
        match (self, tee) {
            (Self::CloudHypervisor, crate::spec::ConfidentialTee::Tdx) => RUNTIME_CLASS_CLH_TDX,
            (Self::CloudHypervisor, _) => RUNTIME_CLASS_CLH_SNP,
            (Self::Qemu, crate::spec::ConfidentialTee::Tdx) => RUNTIME_CLASS_QEMU_TDX,
            (Self::Qemu, _) => RUNTIME_CLASS_QEMU_SNP,
        }
    }
}

pub fn supported_runtime_classes() -> Vec<&'static str> {
    vec![
        RUNTIME_CLASS_CLH_SNP,
        RUNTIME_CLASS_CLH_TDX,
        RUNTIME_CLASS_QEMU_SNP,
        RUNTIME_CLASS_QEMU_TDX,
    ]
}

pub fn confidential_pod_patch(spec: &Workload) -> Option<Value> {
    let conf = spec.confidential.as_ref()?;
    if !conf.enabled {
        return None;
    }
    let hypervisor = KataHypervisor::from_env();
    let runtime_class = hypervisor.runtime_class(conf.tee);
    Some(json!({
        "runtimeClassName": runtime_class,
        "metadata": {
            "labels": {
                "ragnarok.zyvor.dev/confidential": "true",
                "ragnarok.zyvor.dev/tee": serde_json::to_value(&conf.tee).ok(),
                "ragnarok.zyvor.dev/kata-hypervisor": match hypervisor {
                    KataHypervisor::CloudHypervisor => "clh",
                    KataHypervisor::Qemu => "qemu",
                }
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
