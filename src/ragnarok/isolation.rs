//! Tenant isolation policy engine for confidential workloads.

use crate::spec::Workload;
use serde::{Deserialize, Serialize};
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
        violations.push("encrypted state required but confidential.isolation.encryptedState is false".into());
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

pub fn apply_to_kubevirt_annotations(spec: &Workload) -> HashMap<String, String> {
    let mut ann = HashMap::new();
    let Some(conf) = spec.confidential.as_ref() else {
        return ann;
    };
    if !conf.enabled {
        return ann;
    }
    let policy = IsolationPolicy::default();
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
    ann
}
