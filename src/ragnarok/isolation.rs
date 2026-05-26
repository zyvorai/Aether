//! Tenant isolation policy engine for confidential workloads.

use crate::spec::Workload;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationPolicy {
    pub one_confidential_vm_per_numa: bool,
    pub require_encrypted_pvc: bool,
    pub require_vtpm: bool,
    pub anti_coresharing: bool,
    pub allowed_passthrough_devices: Vec<String>,
}

impl Default for IsolationPolicy {
    fn default() -> Self {
        Self {
            one_confidential_vm_per_numa: true,
            require_encrypted_pvc: true,
            require_vtpm: true,
            anti_coresharing: true,
            allowed_passthrough_devices: vec![],
        }
    }
}

impl IsolationPolicy {
    pub fn from_env() -> Self {
        let raw = std::env::var("RAGNAROK_ISOLATION_POLICY").unwrap_or_default();
        if raw.trim().is_empty() {
            return Self::default();
        }
        serde_json::from_str(&raw).unwrap_or_default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationVerdict {
    pub compliant: bool,
    pub violations: Vec<String>,
    pub node_taints: Vec<String>,
    pub scheduler_hints: HashMap<String, String>,
}

pub fn evaluate(spec: &Workload, policy: &IsolationPolicy) -> IsolationVerdict {
    let mut violations = Vec::new();
    let mut hints = HashMap::new();
    let mut taints = Vec::new();

    let Some(conf) = spec.confidential.as_ref() else {
        return IsolationVerdict {
            compliant: true,
            violations,
            node_taints: taints,
            scheduler_hints: hints,
        };
    };

    if !conf.enabled {
        return IsolationVerdict {
            compliant: true,
            violations,
            node_taints: taints,
            scheduler_hints: hints,
        };
    }

    if policy.require_vtpm && !conf.isolation.vtpm {
        violations.push("vTPM required by tenant policy but disabled in spec".into());
    }
    if policy.require_encrypted_pvc && !conf.isolation.encrypted_state {
        violations.push(
            "encrypted state required but confidential.isolation.encryptedState is false".into(),
        );
    }
    if policy.one_confidential_vm_per_numa {
        hints.insert(
            "scheduler.alpha.kubernetes.io/confidential-numa".into(),
            "exclusive".into(),
        );
        taints.push("ragnarok.zyvor.dev/confidential=exclusive:NoSchedule".into());
    }
    if policy.anti_coresharing {
        hints.insert("cpu-manager-policy".into(), "static".into());
    }

    IsolationVerdict {
        compliant: violations.is_empty(),
        violations,
        node_taints: taints,
        scheduler_hints: hints,
    }
}

/// Block deploy when tenant isolation policy is violated.
pub fn deploy_isolation_gate(spec: &Workload) -> Result<()> {
    let policy = IsolationPolicy::from_env();
    let verdict = evaluate(spec, &policy);
    if !verdict.compliant {
        bail!(
            "confidential isolation policy violated: {}",
            verdict.violations.join("; ")
        );
    }
    Ok(())
}

pub fn apply_to_kubevirt_annotations(spec: &Workload) -> HashMap<String, String> {
    let mut ann = HashMap::new();
    let Some(conf) = spec.confidential.as_ref() else {
        return ann;
    };
    if !conf.enabled {
        return ann;
    }
    let policy = IsolationPolicy::from_env();
    let verdict = evaluate(spec, &policy);
    for (k, v) in verdict.scheduler_hints {
        ann.insert(k, v);
    }
    if conf.isolation.encrypted_state {
        ann.insert(
            "ragnarok.zyvor.dev/encrypted-pvc".into(),
            "required".into(),
        );
    }
    ann.insert(
        "ragnarok.zyvor.dev/confidential".into(),
        "true".into(),
    );
    ann
}

/// Pod anti-affinity: one confidential VM per host when anti-coresharing is enabled.
pub fn kubevirt_anti_affinity(spec: &Workload) -> Option<Value> {
    let conf = spec.confidential.as_ref()?;
    if !conf.enabled {
        return None;
    }
    let policy = IsolationPolicy::from_env();
    if !policy.anti_coresharing {
        return None;
    }
    Some(json!({
        "podAntiAffinity": {
            "requiredDuringSchedulingIgnoredDuringExecution": [{
                "labelSelector": {
                    "matchLabels": {
                        "ragnarok.zyvor.dev/confidential": "true"
                    }
                },
                "topologyKey": "kubernetes.io/hostname"
            }]
        }
    }))
}

/// Encrypted PVC hints for CDI DataVolume when encrypted state is required.
pub fn datavolume_encryption_annotations(spec: &Workload) -> HashMap<String, String> {
    let mut ann = HashMap::new();
    let Some(conf) = spec.confidential.as_ref() else {
        return ann;
    };
    if conf.enabled && conf.isolation.encrypted_state {
        ann.insert(
            "ragnarok.zyvor.dev/encrypted-pvc".into(),
            "required".into(),
        );
        if let Ok(sc) = std::env::var("RAGNAROK_ENCRYPTED_STORAGE_CLASS") {
            if !sc.trim().is_empty() {
                ann.insert(
                    "ragnarok.zyvor.dev/storage-class".into(),
                    sc.trim().to_string(),
                );
            }
        }
    }
    ann
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::*;

    fn conf_spec(vtpm: bool, encrypted: bool) -> Workload {
        use std::collections::HashMap;
        Workload {
            api_version: "aether/v1".into(),
            kind: "Workload".into(),
            metadata: Metadata {
                name: "iso".into(),
                owner: "t".into(),
                project: "t".into(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
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
                attestation: ConfidentialAttestationSpec::default(),
                isolation: ConfidentialIsolationSpec {
                    vtpm,
                    encrypted_state: encrypted,
                    debug_allowed: false,
                },
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
    fn violations_when_vtpm_disabled() {
        let w = conf_spec(false, true);
        let v = evaluate(&w, &IsolationPolicy::default());
        assert!(!v.compliant);
    }

    #[test]
    fn deploy_gate_blocks_bad_spec() {
        let w = conf_spec(false, true);
        assert!(deploy_isolation_gate(&w).is_err());
    }
}
