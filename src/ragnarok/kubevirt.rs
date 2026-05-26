// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! KubeVirt confidential VM manifest extensions.

use crate::spec::{ConfidentialSpec, Workload};
use crate::ragnarok::inject::{annotate_vm_for_attest_secrets, attest_gated_secret_names};
use crate::ragnarok::isolation::{apply_to_kubevirt_annotations, kubevirt_anti_affinity};
use serde_json::{json, Value};

/// Apply launchSecurity, firmware, TPM, and node selectors for confidential VMs.
pub fn apply_confidential_to_vm(vm_spec: &mut Value, spec: &Workload) {
    let Some(conf) = spec.confidential.as_ref() else {
        return;
    };
    if !conf.enabled {
        return;
    }

    let domain = &mut vm_spec["spec"]["template"]["spec"]["domain"];

    let launch_security = build_launch_security(conf);
    if let Some(launch) = launch_security.clone() {
        domain["launchSecurity"] = launch;
    }

    if conf.isolation.vtpm {
        domain["devices"]["tpm"] = json!({});
    }

    // TEE launch (SEV/TDX) requires UEFI/OVMF; strict policy also mandates EFI.
    if launch_security.is_some()
        || conf.attestation.policy == crate::spec::AttestationPolicy::Strict
        || !conf.isolation.debug_allowed
    {
        domain["firmware"] = json!({
            "bootloader": {
                "efi": {
                    "secureBoot": false
                }
            },
            "uuid": spec.metadata.name
        });
    }

    if let Some(ref k8s) = spec.kubernetes {
        let mut node_selector = k8s.node_selector.clone();
        if let Some(sel) = super::tee::tee_node_selector(&conf.tee) {
            node_selector.extend(sel);
        }
        if !node_selector.is_empty() {
            vm_spec["spec"]["template"]["spec"]["nodeSelector"] = serde_json::to_value(node_selector)
                .unwrap_or(json!({}));
        }
    } else if let Some(sel) = super::tee::tee_node_selector(&conf.tee) {
        vm_spec["spec"]["template"]["spec"]["nodeSelector"] = serde_json::to_value(sel)
            .unwrap_or(json!({}));
    }

    if let Some(digest) = &conf.image_digest {
        let annotations = vm_spec["metadata"]["annotations"]
            .as_object_mut()
            .map(|m| m.to_owned())
            .unwrap_or_default();
        let mut ann = annotations;
        ann.insert(
            "ragnarok.zyvor.dev/launch-digest".into(),
            json!(digest),
        );
        vm_spec["metadata"]["annotations"] = json!(ann);
    }

    if conf.secrets.release_policy == crate::spec::SecretReleasePolicy::AttestGated {
        let names = attest_gated_secret_names(spec);
        annotate_vm_for_attest_secrets(vm_spec, &names);
    }

    for (k, v) in apply_to_kubevirt_annotations(spec) {
        let obj = vm_spec
            .pointer_mut("/metadata/annotations")
            .and_then(|a| a.as_object_mut());
        if let Some(ann) = obj {
            ann.insert(k, json!(v));
        } else {
            vm_spec["metadata"]["annotations"] = json!({ k: v });
        }
    }

    if let Some(affinity) = kubevirt_anti_affinity(spec) {
        vm_spec["spec"]["template"]["spec"]["affinity"] = affinity;
    }
}

fn build_launch_security(conf: &ConfidentialSpec) -> Option<Value> {
    use crate::spec::ConfidentialTee;
    match conf.tee {
        ConfidentialTee::Sev | ConfidentialTee::SevEs | ConfidentialTee::SevSnp => Some(json!({
            "sev": {
                "policy": {
                    "encryptedState": conf.isolation.encrypted_state,
                    "debug": conf.isolation.debug_allowed,
                    "keySharing": false
                }
            }
        })),
        ConfidentialTee::Tdx => Some(json!({
            "tdx": {}
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn confidential_workload() -> Workload {
        let mut w = Workload {
            api_version: "aether/v1".into(),
            kind: "Workload".into(),
            metadata: Metadata {
                name: "conf-vm".into(),
                owner: "test".into(),
                project: "demo".into(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/test".into(),
                build_args: HashMap::new(),
                tag: None,
                push: false,
            },
            requirements: ResourceRequirements {
                cpu: "2".into(),
                memory: "4Gi".into(),
                storage: "20Gi".into(),
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
            confidential: None,
            schedule: None,
            kubernetes: None,
        };
        w.confidential = Some(ConfidentialSpec {
            enabled: true,
            tee: ConfidentialTee::SevSnp,
            attestation: ConfidentialAttestationSpec {
                required: true,
                policy: AttestationPolicy::Strict,
            },
            isolation: ConfidentialIsolationSpec {
                vtpm: true,
                encrypted_state: true,
                debug_allowed: false,
            },
            secrets: ConfidentialSecretsSpec {
                release_policy: SecretReleasePolicy::AttestGated,
                provider: SecretProvider::Vault,
                names: vec!["db-password".into()],
            },
            image_digest: None,
            region_lock: None,
            kata_runtime_class: None,
            security_profile: None,
        });
        w
    }

    #[test]
    fn test_apply_confidential_launch_security() {
        let spec = confidential_workload();
        let mut vm = json!({
            "spec": { "template": { "spec": { "domain": { "devices": {} } } } },
            "metadata": { "annotations": {} }
        });
        apply_confidential_to_vm(&mut vm, &spec);
        assert!(vm["spec"]["template"]["spec"]["domain"]["launchSecurity"]["sev"].is_object());
        assert!(vm["spec"]["template"]["spec"]["domain"]["devices"]["tpm"].is_object());
    }
}
