//! Kata / Confidential Containers RuntimeClass path.

use crate::spec::Workload;
use serde_json::{json, Value};

pub const RUNTIME_CLASS_SNP: &str = "kata-qemu-snp";
pub const RUNTIME_CLASS_TDX: &str = "kata-qemu-tdx";

pub fn confidential_pod_patch(spec: &Workload) -> Option<Value> {
    let conf = spec.confidential.as_ref()?;
    if !conf.enabled {
        return None;
    }
    let runtime_class = match conf.tee {
        crate::spec::ConfidentialTee::Tdx => RUNTIME_CLASS_TDX,
        _ => RUNTIME_CLASS_SNP,
    };
    Some(json!({
        "runtimeClassName": runtime_class,
        "metadata": {
            "labels": {
                "ragnarok.zyvor.dev/confidential": "true",
                "ragnarok.zyvor.dev/tee": serde_json::to_value(&conf.tee).ok()
            }
        }
    }))
}

pub fn operator_requirements() -> Vec<&'static str> {
    vec![
        "confidential-containers-operator",
        "trustee",
        "kbs",
    ]
}
