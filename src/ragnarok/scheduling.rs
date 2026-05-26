//! Trust-aware placement and scheduling hints for confidential workloads.

use crate::ragnarok::{
    image::ImageCatalog,
    isolation::{self, IsolationPolicy},
    kata::KataHypervisor,
    sovereign::{self, SovereignConfig},
    tee::{probe_host_tee, HostTeeStatus},
};
use crate::runtime::RuntimeKind;
use crate::spec::{ConfidentialTee, RuntimePreference, RuntimeType, Workload};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfidentialPlacementAdvice {
    pub workload: String,
    pub confidential_enabled: bool,
    pub tee: Option<String>,
    pub recommended_runtime: String,
    pub kata_runtime_class: Option<String>,
    pub schedule_constraints: Vec<String>,
    pub scheduler_hints: std::collections::HashMap<String, String>,
    pub host_tee_ready: bool,
    pub placement_score_bonus: f64,
    pub blockers: Vec<String>,
    pub gitops_issues: Vec<String>,
    pub isolation_compliant: bool,
}

pub fn tee_hardware_label(tee: ConfidentialTee) -> &'static str {
    match tee {
        ConfidentialTee::Sev | ConfidentialTee::SevEs => "sev",
        ConfidentialTee::SevSnp => "sev-snp",
        ConfidentialTee::Tdx => "tdx",
    }
}

pub fn tee_kind_matches(tee: ConfidentialTee, host: &HostTeeStatus) -> bool {
    match tee {
        ConfidentialTee::Tdx => host.tdx,
        ConfidentialTee::SevSnp => host.sev_snp,
        ConfidentialTee::Sev | ConfidentialTee::SevEs => host.sev_device,
    }
}

/// Human-readable scheduling constraints for confidential specs.
pub fn schedule_constraint_labels(spec: &Workload) -> Vec<String> {
    let Some(conf) = spec.confidential.as_ref().filter(|c| c.enabled) else {
        return Vec::new();
    };

    let mut labels = vec![
        "require:trusted".into(),
        format!("require:{}", tee_hardware_label(conf.tee)),
    ];
    if conf.isolation.encrypted_state {
        labels.push("require:nvme".into());
    }
    if conf.attestation.required {
        labels.push("require:attestation-pass".into());
    }
    labels
}

pub fn preferred_runtime_kind(pref: RuntimePreference) -> RuntimeKind {
    match pref {
        RuntimePreference::Kube => RuntimeKind::Kubernetes,
        RuntimePreference::Kubevirt => RuntimeKind::KubeVirt,
        RuntimePreference::Container => RuntimeKind::Podman,
        RuntimePreference::Metal => RuntimeKind::Metal3,
        RuntimePreference::Auto => RuntimeKind::Kubernetes,
    }
}

pub fn recommended_runtime(spec: &Workload) -> RuntimeKind {
    let Some(conf) = spec.confidential.as_ref().filter(|c| c.enabled) else {
        return preferred_runtime_kind(spec.runtime.preferred.clone());
    };

    if conf.kata_runtime_class.is_some() {
        return RuntimeKind::Kubernetes;
    }
    if matches!(spec.runtime.preferred, RuntimePreference::Kube) {
        return RuntimeKind::Kubernetes;
    }
    if spec.runtime.allow.contains(&RuntimeType::Kubevirt) {
        return RuntimeKind::KubeVirt;
    }
    if spec.runtime.allow.contains(&RuntimeType::Kube) {
        return RuntimeKind::Kubernetes;
    }
    RuntimeKind::KubeVirt
}

pub fn gitops_policy_issues(spec: &Workload) -> Vec<String> {
    let mut issues = Vec::new();
    let Some(conf) = spec.confidential.as_ref().filter(|c| c.enabled) else {
        return issues;
    };

    let isolation = isolation::evaluate(spec, &IsolationPolicy::from_env());
    if !isolation.compliant {
        issues.extend(isolation.violations);
    }

    let sovereign = sovereign::evaluate(spec, &SovereignConfig::from_env());
    if !sovereign.compliant {
        issues.extend(sovereign.violations);
    }

    if let Some(ref digest) = conf.image_digest {
        let catalog = ImageCatalog::load(&crate::ragnarok::client::RagnarokClient::attestation_data_dir());
        if !catalog.verify_digest(digest) {
            issues.push(format!(
                "launch digest {digest} not in verified image catalog"
            ));
        }
    } else if conf.attestation.required
        && matches!(
            conf.attestation.policy,
            crate::spec::AttestationPolicy::Strict
        )
    {
        issues.push("strict attestation requires confidential.imageDigest".into());
    }

    issues
}

/// Placement score bonus (0.0–0.25) and reasons for multi-cluster placement.
pub fn placement_bonus(spec: &Workload, host: &HostTeeStatus) -> (f64, Vec<String>) {
    let Some(conf) = spec.confidential.as_ref().filter(|c| c.enabled) else {
        return (0.0, Vec::new());
    };

    let mut bonus: f64 = 0.05;
    let mut reasons = vec!["Confidential workload — TEE placement preferred".to_string()];

    if tee_kind_matches(conf.tee, host) {
        bonus += 0.12;
        reasons.push(format!(
            "Host supports {}",
            tee_hardware_label(conf.tee)
        ));
    } else {
        reasons.push(format!(
            "Host missing {} capability",
            tee_hardware_label(conf.tee)
        ));
    }

    let isolation = isolation::evaluate(spec, &IsolationPolicy::from_env());
    if isolation.compliant {
        bonus += 0.05;
        reasons.push("Tenant isolation policy compliant".into());
    }

    if conf.attestation.required {
        bonus += 0.03;
        reasons.push("Attestation-gated deploy path".into());
    }

    let capped = ((bonus * 100.0).round() / 100.0).min(0.25_f64);
    (capped, reasons)
}

pub fn placement_advice(spec: &Workload) -> ConfidentialPlacementAdvice {
    let host = probe_host_tee();
    let isolation = isolation::evaluate(spec, &IsolationPolicy::from_env());
    let gitops_issues = gitops_policy_issues(spec);
    let mut blockers = gitops_issues.clone();

    let confidential_enabled = spec
        .confidential
        .as_ref()
        .is_some_and(|c| c.enabled);

    let tee = spec
        .confidential
        .as_ref()
        .filter(|c| c.enabled)
        .map(|c| serde_json::to_value(&c.tee).ok())
        .flatten()
        .and_then(|v| v.as_str().map(String::from));

    let recommended = recommended_runtime(spec);
    let kata_runtime_class = spec.confidential.as_ref().and_then(|c| {
        if !c.enabled {
            return None;
        }
        c.kata_runtime_class.clone().or_else(|| {
            if c.security_profile.is_some() {
                return crate::ragnarok::kata::resolve_runtime_class(spec);
            }
            if recommended == RuntimeKind::Kubernetes {
                Some(
                    KataHypervisor::from_env()
                        .runtime_class(&c.tee)
                        .to_string(),
                )
            } else {
                None
            }
        })
    });

    if confidential_enabled {
        if let Some(ref t) = tee {
            if !host_tee_ready_for_label(t, &host) {
                blockers.push(format!("host lacks TEE capability for {t}"));
            }
        }
    }

    let (placement_score_bonus, _) = placement_bonus(spec, &host);

    ConfidentialPlacementAdvice {
        workload: spec.metadata.name.clone(),
        confidential_enabled,
        tee,
        recommended_runtime: recommended.to_string(),
        kata_runtime_class,
        schedule_constraints: schedule_constraint_labels(spec),
        scheduler_hints: isolation.scheduler_hints,
        host_tee_ready: confidential_enabled
            && spec
                .confidential
                .as_ref()
                .map(|c| tee_kind_matches(c.tee, &host))
                .unwrap_or(true),
        placement_score_bonus,
        blockers,
        gitops_issues,
        isolation_compliant: isolation.compliant,
    }
}

fn host_tee_ready_for_label(tee_label: &str, host: &HostTeeStatus) -> bool {
    match tee_label {
        "tdx" => host.tdx,
        "sev-snp" => host.sev_snp,
        "sev" | "sev-es" => host.sev_device,
        _ => host.sev_snp || host.tdx,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{
        BuildSpec, ConfidentialAttestationSpec, ConfidentialIsolationSpec, ConfidentialSecretsSpec,
        ConfidentialSpec, ConfidentialTee, Metadata, NetworkSpec, PersistenceSpec,
        ResourceRequirements, RuntimePreference, RuntimeSpec, RuntimeType, Workload,
    };

    fn confidential_spec(name: &str, tee: ConfidentialTee) -> Workload {
        Workload {
            api_version: "aether/v1".into(),
            kind: "Workload".into(),
            metadata: Metadata {
                name: name.into(),
                owner: "t".into(),
                project: "t".into(),
                labels: Default::default(),
                annotations: Default::default(),
            },
            build: BuildSpec::default(),
            requirements: ResourceRequirements {
                cpu: "2".into(),
                memory: "4Gi".into(),
                storage: "10Gi".into(),
                gpu: None,
                cpu_request: None,
                memory_request: None,
            },
            runtime: RuntimeSpec {
                preferred: RuntimePreference::Kubevirt,
                allow: vec![RuntimeType::Kubevirt, RuntimeType::Kube],
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
                tee,
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
    fn placement_advice_includes_tee_constraints() {
        let spec = confidential_spec("vm1", ConfidentialTee::SevSnp);
        let advice = placement_advice(&spec);
        assert!(advice.confidential_enabled);
        assert!(advice
            .schedule_constraints
            .iter()
            .any(|c| c.contains("sev-snp")));
        assert_eq!(advice.recommended_runtime, "kubevirt");
    }

    #[test]
    fn gitops_flags_missing_digest_on_strict() {
        let mut spec = confidential_spec("vm2", ConfidentialTee::SevSnp);
        spec.confidential.as_mut().unwrap().attestation.policy =
            crate::spec::AttestationPolicy::Strict;
        let issues = gitops_policy_issues(&spec);
        assert!(issues.iter().any(|i| i.contains("imageDigest")));
    }

    #[test]
    fn placement_bonus_capped() {
        let spec = confidential_spec("vm3", ConfidentialTee::SevSnp);
        let host = HostTeeStatus {
            sev_snp: true,
            ..Default::default()
        };
        let (bonus, _) = placement_bonus(&spec, &host);
        assert!(bonus <= 0.25);
    }
}
