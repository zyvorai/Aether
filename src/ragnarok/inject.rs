// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! KubeVirt / Kubernetes secret injection after attestation-gated release.

use crate::spec::Workload;
use serde_json::{json, Value};

pub fn kubevirt_attest_secret_name(vm_name: &str, secret_name: &str) -> String {
    format!("{vm_name}-ragnarok-ag-{secret_name}")
}

pub fn attest_gated_secret_names(spec: &Workload) -> Vec<String> {
    if let Some(conf) = spec.confidential.as_ref() {
        if !conf.secrets.names.is_empty() {
            return conf.secrets.names.clone();
        }
    }
    spec.config
        .as_ref()
        .map(|c| c.secrets.iter().map(|s| s.name.clone()).collect())
        .unwrap_or_default()
}

pub fn build_k8s_secret_json(namespace: &str, name: &str, data: &str) -> Value {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    let encoded = STANDARD.encode(data.as_bytes());
    json!({
        "apiVersion": "v1",
        "kind": "Secret",
        "metadata": {
            "name": name,
            "namespace": namespace,
            "labels": {
                "app.kubernetes.io/managed-by": "aether",
                "ragnarok.zyvor.dev/attest-gated": "true"
            }
        },
        "type": "Opaque",
        "data": {
            "value": encoded
        }
    })
}

pub fn annotate_vm_for_attest_secrets(vm_spec: &mut Value, secret_names: &[String]) {
    if secret_names.is_empty() {
        return;
    }
    let ann = vm_spec
        .pointer_mut("/metadata/annotations")
        .and_then(|v| v.as_object_mut());
    if let Some(obj) = ann {
        obj.insert(
            "ragnarok.zyvor.dev/attest-gated-secrets".into(),
            json!(secret_names.join(",")),
        );
    } else {
        vm_spec["metadata"]["annotations"] = json!({
            "ragnarok.zyvor.dev/attest-gated-secrets": secret_names.join(",")
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stable_k8s_secret_name() {
        assert_eq!(
            kubevirt_attest_secret_name("vm1", "db"),
            "vm1-ragnarok-ag-db"
        );
    }
}
