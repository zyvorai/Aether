// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Kubernetes manifest generation helpers (Deployment, StatefulSet, Job, PDB, Gateway, VPA, KEDA).

#![allow(clippy::needless_update)]

use crate::runtime::Image;
use crate::spec::{
    AccessMode, HealthProbe, K8sContainerSpec, K8sVolumeSource, K8sWorkloadKind, ProbeType,
    Workload,
};
use k8s_openapi::api::apps::v1::{
    DaemonSet, DaemonSetSpec, Deployment, DeploymentSpec, DeploymentStrategy,
    RollingUpdateDeployment, StatefulSet, StatefulSetSpec,
};
use k8s_openapi::api::batch::v1::{CronJob, CronJobSpec, Job, JobSpec, JobTemplateSpec};
use k8s_openapi::api::core::v1::{
    Affinity, CSIVolumeSource, ConfigMapEnvSource, ConfigMapProjection, ConfigMapVolumeSource,
    Container, ContainerPort, EmptyDirVolumeSource, EnvFromSource as K8sEnvFromSource, ExecAction,
    GRPCAction, HostPathVolumeSource, Lifecycle, LifecycleHandler, LocalObjectReference,
    NFSVolumeSource, NodeAffinity, NodeSelector, NodeSelectorRequirement, NodeSelectorTerm,
    PersistentVolumeClaim, PersistentVolumeClaimSpec, PodAffinity, PodAffinityTerm,
    PodAntiAffinity, PodSecurityContext, PodSpec, PodTemplateSpec, PreferredSchedulingTerm, Probe,
    ProjectedVolumeSource, SecretEnvSource, SecretProjection, SecretVolumeSource, SecurityContext,
    Service, ServiceAccountTokenProjection, ServicePort, ServiceSpec, TCPSocketAction, Toleration,
    TopologySpreadConstraint, Volume, VolumeMount, VolumeProjection, VolumeResourceRequirements,
};
use k8s_openapi::api::policy::v1::{PodDisruptionBudget, PodDisruptionBudgetSpec};
use k8s_openapi::apimachinery::pkg::api::resource::Quantity;
use k8s_openapi::apimachinery::pkg::apis::meta::v1::{LabelSelector, ObjectMeta};
use k8s_openapi::apimachinery::pkg::util::intstr::IntOrString;
use serde_json::{json, Value};
use std::collections::BTreeMap;

pub(crate) fn sanitize_volume_name(name: &str) -> String {
    let sanitized: String = name
        .to_ascii_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "vol".to_string()
    } else {
        trimmed.chars().take(63).collect()
    }
}

fn build_resource_requirements(
    spec: &Workload,
) -> k8s_openapi::api::core::v1::ResourceRequirements {
    let mut limits = BTreeMap::new();
    let mut requests = BTreeMap::new();
    limits.insert("cpu".to_string(), Quantity(spec.requirements.cpu.clone()));
    limits.insert(
        "memory".to_string(),
        Quantity(spec.requirements.memory.clone()),
    );
    let cpu_req = spec
        .requirements
        .cpu_request
        .as_deref()
        .unwrap_or(&spec.requirements.cpu);
    let mem_req = spec
        .requirements
        .memory_request
        .as_deref()
        .unwrap_or(&spec.requirements.memory);
    requests.insert("cpu".to_string(), Quantity(cpu_req.to_string()));
    requests.insert("memory".to_string(), Quantity(mem_req.to_string()));
    if let Some(ref gpu) = spec.requirements.gpu {
        let gpu_key = format!("{}.com/gpu", gpu.vendor);
        limits.insert(gpu_key.clone(), Quantity(gpu.count.to_string()));
        requests.insert(gpu_key, Quantity(gpu.count.to_string()));
    }
    k8s_openapi::api::core::v1::ResourceRequirements {
        limits: Some(limits),
        requests: Some(requests),
        ..Default::default()
    }
}

fn int_or_string_from_str(value: &str) -> IntOrString {
    if let Ok(num) = value.parse::<i32>() {
        IntOrString::Int(num)
    } else {
        IntOrString::String(value.to_string())
    }
}

/// Map Aether health probe to a Kubernetes probe (HTTP, TCP, Exec, or gRPC).
pub(crate) fn build_k8s_probe(probe: &HealthProbe) -> Probe {
    let (http_get, tcp_socket, exec, grpc) = match &probe.probe_type {
        ProbeType::HttpGet { path, port } => (
            Some(k8s_openapi::api::core::v1::HTTPGetAction {
                path: Some(path.clone()),
                port: IntOrString::Int(*port as i32),
                ..Default::default()
            }),
            None,
            None,
            None,
        ),
        ProbeType::TcpSocket { port } => (
            None,
            Some(TCPSocketAction {
                port: IntOrString::Int(*port as i32),
                ..Default::default()
            }),
            None,
            None,
        ),
        ProbeType::Exec { command } => (
            None,
            None,
            Some(ExecAction {
                command: Some(command.clone()),
            }),
            None,
        ),
        ProbeType::Grpc { port, service } => (
            None,
            None,
            None,
            Some(GRPCAction {
                port: *port as i32,
                service: Some(service.clone()),
            }),
        ),
    };

    Probe {
        http_get,
        tcp_socket,
        exec,
        grpc,
        initial_delay_seconds: Some(probe.initial_delay_seconds as i32),
        period_seconds: Some(probe.period_seconds as i32),
        ..Default::default()
    }
}

fn workload_labels(spec: &Workload) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "aether".to_string());
    for (k, v) in &spec.metadata.labels {
        labels.insert(k.clone(), v.clone());
    }
    if let Some(conf) = spec.confidential.as_ref() {
        if conf.enabled {
            labels.insert("ragnarok.zyvor.dev/confidential".into(), "true".into());
            if let Ok(v) = serde_json::to_value(conf.tee) {
                if let Some(tee) = v.as_str() {
                    labels.insert("ragnarok.zyvor.dev/tee".into(), tee.into());
                }
            }
            if let Some(rc) = crate::ragnarok::kata::resolve_runtime_class(spec) {
                labels.insert("ragnarok.zyvor.dev/runtime-class".into(), rc);
            }
        }
    }
    labels
}

fn pod_annotations(spec: &Workload) -> BTreeMap<String, String> {
    let mut pod_annotations: BTreeMap<String, String> =
        spec.metadata.annotations.clone().into_iter().collect();
    if let Some(mesh) = &spec.mesh {
        match mesh.provider.as_str() {
            "istio" => {
                pod_annotations.insert(
                    "sidecar.istio.io/inject".to_string(),
                    mesh.inject.to_string(),
                );
            }
            "linkerd" => {
                pod_annotations.insert(
                    "linkerd.io/inject".to_string(),
                    if mesh.inject { "enabled" } else { "disabled" }.to_string(),
                );
            }
            "consul" => {
                pod_annotations.insert(
                    "consul.hashicorp.com/connect-inject".to_string(),
                    mesh.inject.to_string(),
                );
            }
            _ => {}
        }
        for (k, v) in &mesh.annotations {
            pod_annotations.insert(k.clone(), v.clone());
        }
    }
    if let Some(wi) = spec
        .kubernetes
        .as_ref()
        .and_then(|k| k.workload_identity.as_ref())
    {
        match wi.provider.as_str() {
            "aws" => {
                if let Some(arn) = &wi.role_arn {
                    pod_annotations.insert("eks.amazonaws.com/role-arn".to_string(), arn.clone());
                }
            }
            "azure" => {
                if let Some(id) = &wi.client_id {
                    pod_annotations
                        .insert("azure.workload.identity/client-id".to_string(), id.clone());
                }
            }
            "gcp" => {
                if let Some(sa) = &wi.gcp_service_account {
                    pod_annotations
                        .insert("iam.gke.io/gcp-service-account".to_string(), sa.clone());
                }
            }
            _ => {}
        }
        for (k, v) in &wi.annotations {
            pod_annotations.insert(k.clone(), v.clone());
        }
    }
    for (k, v) in crate::ragnarok::network::confidential_pod_annotations(spec) {
        pod_annotations.insert(k, v);
    }
    pod_annotations
}

fn build_main_container(image: &Image, spec: &Workload) -> Container {
    let ports: Vec<ContainerPort> = spec
        .network
        .ports
        .iter()
        .map(|p| ContainerPort {
            container_port: p.container_port as i32,
            protocol: Some(p.protocol.clone()),
            ..Default::default()
        })
        .collect();

    let resources = build_resource_requirements(spec);

    let liveness_probe = spec
        .health
        .as_ref()
        .and_then(|h| h.liveness.as_ref())
        .map(build_k8s_probe);
    let readiness_probe = spec
        .health
        .as_ref()
        .and_then(|h| h.readiness.as_ref())
        .map(build_k8s_probe);
    let startup_probe = spec
        .health
        .as_ref()
        .and_then(|h| h.startup.as_ref())
        .map(build_k8s_probe);

    let env_from: Vec<K8sEnvFromSource> = spec
        .config
        .as_ref()
        .map(|c| {
            c.env_from
                .iter()
                .map(|e| match e.source_type {
                    crate::spec::EnvSourceType::ConfigMap => K8sEnvFromSource {
                        config_map_ref: Some(ConfigMapEnvSource {
                            name: e.name.clone(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    crate::spec::EnvSourceType::Secret => K8sEnvFromSource {
                        secret_ref: Some(SecretEnvSource {
                            name: e.name.clone(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                })
                .collect()
        })
        .unwrap_or_default();

    let inline_env: Vec<k8s_openapi::api::core::v1::EnvVar> = spec
        .config
        .as_ref()
        .map(|c| {
            c.config_maps
                .iter()
                .flat_map(|cm| {
                    cm.data
                        .iter()
                        .map(|(k, v)| k8s_openapi::api::core::v1::EnvVar {
                            name: k.clone(),
                            value: Some(v.clone()),
                            ..Default::default()
                        })
                })
                .collect()
        })
        .unwrap_or_default();

    let mut volume_mounts = build_config_volume_mounts(spec);
    if spec.persistence.enabled && spec.resolved_k8s_workload_kind() != K8sWorkloadKind::StatefulSet
    {
        volume_mounts.push(VolumeMount {
            name: format!("{}-storage", spec.metadata.name),
            mount_path: "/data".to_string(),
            ..Default::default()
        });
    }

    let security_context = spec.kubernetes.as_ref().and_then(|k| {
        k.container_security_context
            .as_ref()
            .map(|sc| SecurityContext {
                run_as_non_root: sc.run_as_non_root,
                run_as_user: sc.run_as_user,
                read_only_root_filesystem: sc.read_only_root_filesystem,
                allow_privilege_escalation: sc.allow_privilege_escalation,
                ..Default::default()
            })
    });

    let lifecycle = spec
        .kubernetes
        .as_ref()
        .and_then(|k| k.pod.as_ref())
        .and_then(|p| p.lifecycle.as_ref())
        .map(build_container_lifecycle);

    Container {
        name: spec.metadata.name.clone(),
        image: Some(image.reference()),
        ports: Some(ports),
        resources: Some(resources),
        liveness_probe,
        readiness_probe,
        startup_probe,
        lifecycle,
        env: if inline_env.is_empty() {
            None
        } else {
            Some(inline_env)
        },
        env_from: if env_from.is_empty() {
            None
        } else {
            Some(env_from)
        },
        volume_mounts: if volume_mounts.is_empty() {
            None
        } else {
            Some(volume_mounts)
        },
        security_context,
        ..Default::default()
    }
}

fn build_config_volume_mounts(spec: &Workload) -> Vec<VolumeMount> {
    let mut volume_mounts = Vec::new();
    if let Some(config) = &spec.config {
        for cm in &config.config_maps {
            if let Some(mount_path) = &cm.mount_path {
                volume_mounts.push(VolumeMount {
                    name: sanitize_volume_name(&format!("cm-{}", cm.name)),
                    mount_path: mount_path.clone(),
                    read_only: Some(true),
                    ..Default::default()
                });
            }
        }
        for secret in &config.secrets {
            if let Some(mount_path) = &secret.mount_path {
                volume_mounts.push(VolumeMount {
                    name: sanitize_volume_name(&format!("secret-{}", secret.name)),
                    mount_path: mount_path.clone(),
                    read_only: Some(true),
                    ..Default::default()
                });
            }
        }
    }
    if let Some(k8s) = &spec.kubernetes {
        for vol in &k8s.extra_volumes {
            volume_mounts.push(VolumeMount {
                name: vol.name.clone(),
                mount_path: vol.mount_path.clone(),
                read_only: Some(vol.read_only),
                sub_path: vol.sub_path.clone(),
                ..Default::default()
            });
        }
        if spec.persistence.enabled
            && spec.resolved_k8s_workload_kind() == K8sWorkloadKind::StatefulSet
        {
            volume_mounts.push(VolumeMount {
                name: format!("{}-storage", spec.metadata.name),
                mount_path: "/data".to_string(),
                ..Default::default()
            });
        }
    }
    volume_mounts
}

fn build_volumes(spec: &Workload) -> Vec<Volume> {
    let mut volumes = Vec::new();
    if let Some(config) = &spec.config {
        for cm in &config.config_maps {
            if cm.mount_path.is_some() {
                let vol_name = sanitize_volume_name(&format!("cm-{}", cm.name));
                volumes.push(Volume {
                    name: vol_name,
                    config_map: Some(k8s_openapi::api::core::v1::ConfigMapVolumeSource {
                        name: cm.name.clone(),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
        }
        for secret in &config.secrets {
            if secret.mount_path.is_some() {
                let vol_name = sanitize_volume_name(&format!("secret-{}", secret.name));
                volumes.push(Volume {
                    name: vol_name,
                    secret: Some(k8s_openapi::api::core::v1::SecretVolumeSource {
                        secret_name: Some(secret.name.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
        }
    }
    if spec.persistence.enabled && spec.resolved_k8s_workload_kind() != K8sWorkloadKind::StatefulSet
    {
        volumes.push(Volume {
            name: format!("{}-storage", spec.metadata.name),
            persistent_volume_claim: Some(
                k8s_openapi::api::core::v1::PersistentVolumeClaimVolumeSource {
                    claim_name: format!("{}-pvc", spec.metadata.name),
                    read_only: Some(false),
                },
            ),
            ..Default::default()
        });
    }
    if let Some(k8s) = &spec.kubernetes {
        for vol in &k8s.extra_volumes {
            volumes.push(match &vol.source {
                K8sVolumeSource::EmptyDir { medium } => Volume {
                    name: vol.name.clone(),
                    empty_dir: Some(EmptyDirVolumeSource {
                        medium: medium.clone(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                K8sVolumeSource::HostPath {
                    path,
                    host_path_type,
                } => Volume {
                    name: vol.name.clone(),
                    host_path: Some(HostPathVolumeSource {
                        path: path.clone(),
                        type_: Some(host_path_type.clone()),
                    }),
                    ..Default::default()
                },
                K8sVolumeSource::ConfigMap { name } => Volume {
                    name: vol.name.clone(),
                    config_map: Some(ConfigMapVolumeSource {
                        name: name.clone(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                K8sVolumeSource::Secret { name } => Volume {
                    name: vol.name.clone(),
                    secret: Some(SecretVolumeSource {
                        secret_name: Some(name.clone()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                K8sVolumeSource::Nfs {
                    server,
                    path,
                    read_only,
                } => Volume {
                    name: vol.name.clone(),
                    nfs: Some(NFSVolumeSource {
                        server: server.clone(),
                        path: path.clone(),
                        read_only: Some(*read_only),
                    }),
                    ..Default::default()
                },
                K8sVolumeSource::Csi {
                    driver,
                    volume_handle,
                    fs_type,
                } => {
                    let mut attrs = BTreeMap::new();
                    attrs.insert("volumeHandle".to_string(), volume_handle.clone());
                    if let Some(fs) = fs_type {
                        attrs.insert("fsType".to_string(), fs.clone());
                    }
                    Volume {
                        name: vol.name.clone(),
                        csi: Some(CSIVolumeSource {
                            driver: driver.clone(),
                            volume_attributes: Some(attrs),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }
                }
                K8sVolumeSource::Projected { sources } => Volume {
                    name: vol.name.clone(),
                    projected: Some(ProjectedVolumeSource {
                        sources: Some(
                            sources
                                .iter()
                                .filter_map(projected_source_from_spec)
                                .collect(),
                        ),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            });
        }
    }
    volumes
}

fn projected_source_from_spec(
    src: &crate::spec::K8sProjectedVolumeSourceSpec,
) -> Option<VolumeProjection> {
    if src.service_account_token {
        return Some(VolumeProjection {
            service_account_token: Some(ServiceAccountTokenProjection {
                path: "token".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        });
    }
    if let Some(name) = &src.config_map_name {
        return Some(VolumeProjection {
            config_map: Some(ConfigMapProjection {
                name: name.clone(),
                ..Default::default()
            }),
            ..Default::default()
        });
    }
    if let Some(name) = &src.secret_name {
        return Some(VolumeProjection {
            secret: Some(SecretProjection {
                name: name.clone(),
                ..Default::default()
            }),
            ..Default::default()
        });
    }
    None
}

fn container_from_spec(c: &K8sContainerSpec) -> Container {
    let env: Vec<k8s_openapi::api::core::v1::EnvVar> = c
        .env
        .iter()
        .map(|(k, v)| k8s_openapi::api::core::v1::EnvVar {
            name: k.clone(),
            value: Some(v.clone()),
            ..Default::default()
        })
        .collect();
    Container {
        name: c.name.clone(),
        image: Some(c.image.clone()),
        command: if c.command.is_empty() {
            None
        } else {
            Some(c.command.clone())
        },
        args: if c.args.is_empty() {
            None
        } else {
            Some(c.args.clone())
        },
        env: if env.is_empty() { None } else { Some(env) },
        ..Default::default()
    }
}

fn build_container_lifecycle(spec: &crate::spec::K8sLifecycleSpec) -> Lifecycle {
    Lifecycle {
        pre_stop: spec.pre_stop.as_ref().map(lifecycle_handler),
        post_start: spec.post_start.as_ref().map(lifecycle_handler),
    }
}

fn lifecycle_handler(handler: &crate::spec::K8sLifecycleHandler) -> LifecycleHandler {
    match handler {
        crate::spec::K8sLifecycleHandler::Exec { command } => LifecycleHandler {
            exec: Some(ExecAction {
                command: Some(command.clone()),
            }),
            ..Default::default()
        },
        crate::spec::K8sLifecycleHandler::HttpGet { path, port } => LifecycleHandler {
            http_get: Some(k8s_openapi::api::core::v1::HTTPGetAction {
                path: Some(path.clone()),
                port: IntOrString::Int(*port as i32),
                ..Default::default()
            }),
            ..Default::default()
        },
    }
}

fn build_topology_spread(spec: &Workload) -> Option<Vec<TopologySpreadConstraint>> {
    let pod = spec.kubernetes.as_ref()?.pod.as_ref()?;
    if pod.topology_spread_constraints.is_empty() {
        return None;
    }
    Some(
        pod.topology_spread_constraints
            .iter()
            .map(|t| {
                let mut match_labels = BTreeMap::new();
                for (k, v) in &t.label_selector {
                    match_labels.insert(k.clone(), v.clone());
                }
                TopologySpreadConstraint {
                    max_skew: t.max_skew,
                    topology_key: t.topology_key.clone(),
                    when_unsatisfiable: t.when_unsatisfiable.clone(),
                    label_selector: if match_labels.is_empty() {
                        None
                    } else {
                        Some(LabelSelector {
                            match_labels: Some(match_labels),
                            ..Default::default()
                        })
                    },
                    ..Default::default()
                }
            })
            .collect(),
    )
}

fn build_affinity(spec: &Workload) -> Option<Affinity> {
    let k8s = spec.kubernetes.as_ref()?;
    let aff = k8s.affinity.as_ref();
    let has_pod =
        aff.is_some_and(|a| !a.pod_affinity.is_empty() || !a.pod_anti_affinity.is_empty());
    let node_affinity = aff
        .and_then(|a| a.node_affinity.as_ref())
        .map(build_node_affinity);
    if !has_pod && node_affinity.is_none() {
        return None;
    }
    let to_terms = |terms: &[crate::spec::K8sPodAffinityTermSpec]| -> Vec<PodAffinityTerm> {
        terms
            .iter()
            .map(|t| {
                let mut match_labels = BTreeMap::new();
                for (k, v) in &t.label_selector {
                    match_labels.insert(k.clone(), v.clone());
                }
                PodAffinityTerm {
                    topology_key: t.topology_key.clone(),
                    label_selector: Some(LabelSelector {
                        match_labels: Some(match_labels),
                        ..Default::default()
                    }),
                    ..Default::default()
                }
            })
            .collect()
    };
    Some(Affinity {
        node_affinity,
        pod_affinity: aff
            .filter(|a| !a.pod_affinity.is_empty())
            .map(|a| PodAffinity {
                required_during_scheduling_ignored_during_execution: Some(to_terms(
                    &a.pod_affinity,
                )),
                ..Default::default()
            }),
        pod_anti_affinity: aff.filter(|a| !a.pod_anti_affinity.is_empty()).map(|a| {
            PodAntiAffinity {
                required_during_scheduling_ignored_during_execution: Some(to_terms(
                    &a.pod_anti_affinity,
                )),
                ..Default::default()
            }
        }),
        ..Default::default()
    })
}

fn build_node_affinity(spec: &crate::spec::K8sNodeAffinitySpec) -> NodeAffinity {
    let required: Vec<NodeSelectorTerm> = spec
        .required
        .iter()
        .map(|term| NodeSelectorTerm {
            match_expressions: Some(
                term.match_expressions
                    .iter()
                    .map(|e| NodeSelectorRequirement {
                        key: e.key.clone(),
                        operator: e.operator.clone(),
                        values: if e.values.is_empty() {
                            None
                        } else {
                            Some(e.values.clone())
                        },
                    })
                    .collect(),
            ),
            ..Default::default()
        })
        .collect();
    let preferred: Vec<PreferredSchedulingTerm> = spec
        .preferred
        .iter()
        .map(|p| PreferredSchedulingTerm {
            weight: p.weight,
            preference: NodeSelectorTerm {
                match_expressions: Some(
                    p.match_expressions
                        .iter()
                        .map(|e| NodeSelectorRequirement {
                            key: e.key.clone(),
                            operator: e.operator.clone(),
                            values: if e.values.is_empty() {
                                None
                            } else {
                                Some(e.values.clone())
                            },
                        })
                        .collect(),
                ),
                ..Default::default()
            },
        })
        .collect();
    NodeAffinity {
        required_during_scheduling_ignored_during_execution: if required.is_empty() {
            None
        } else {
            Some(NodeSelector {
                node_selector_terms: required,
            })
        },
        preferred_during_scheduling_ignored_during_execution: if preferred.is_empty() {
            None
        } else {
            Some(preferred)
        },
    }
}

fn build_tolerations(spec: &Workload) -> Option<Vec<Toleration>> {
    let k8s = spec.kubernetes.as_ref()?;
    if k8s.tolerations.is_empty() {
        return None;
    }
    Some(
        k8s.tolerations
            .iter()
            .map(|t| Toleration {
                key: Some(t.key.clone()),
                operator: Some(t.operator.clone()),
                value: t.value.clone(),
                effect: t.effect.clone(),
                ..Default::default()
            })
            .collect(),
    )
}

fn build_pod_security_context(spec: &Workload) -> Option<PodSecurityContext> {
    spec.kubernetes
        .as_ref()
        .and_then(|k| k.pod_security_context.as_ref())
        .map(|sc| PodSecurityContext {
            run_as_non_root: sc.run_as_non_root,
            run_as_user: sc.run_as_user,
            fs_group: sc.fs_group,
            ..Default::default()
        })
}

fn build_image_pull_secrets(spec: &Workload) -> Option<Vec<LocalObjectReference>> {
    let k8s = spec.kubernetes.as_ref()?;
    if k8s.image_pull_secrets.is_empty() {
        return None;
    }
    Some(
        k8s.image_pull_secrets
            .iter()
            .map(|name| LocalObjectReference { name: name.clone() })
            .collect(),
    )
}

fn volume_claim_template(spec: &Workload) -> Option<PersistentVolumeClaim> {
    if !spec.persistence.enabled {
        return None;
    }
    let access_mode = match spec.persistence.access_mode {
        AccessMode::ReadWriteOnce => "ReadWriteOnce",
        AccessMode::ReadOnlyMany => "ReadOnlyMany",
        AccessMode::ReadWriteMany => "ReadWriteMany",
    };
    let mut requests = BTreeMap::new();
    requests.insert(
        "storage".to_string(),
        Quantity(spec.persistence.size.clone()),
    );
    Some(PersistentVolumeClaim {
        metadata: ObjectMeta {
            name: Some(format!("{}-storage", spec.metadata.name)),
            ..Default::default()
        },
        spec: Some(PersistentVolumeClaimSpec {
            access_modes: Some(vec![access_mode.to_string()]),
            resources: Some(VolumeResourceRequirements {
                requests: Some(requests),
                ..Default::default()
            }),
            storage_class_name: spec.persistence.storage_class.clone(),
            ..Default::default()
        }),
        ..Default::default()
    })
}

pub(crate) fn build_pod_template_spec(
    image: &Image,
    spec: &Workload,
    restart_policy: Option<&str>,
) -> PodTemplateSpec {
    let labels = workload_labels(spec);
    let mut containers = vec![build_main_container(image, spec)];
    if let Some(k8s) = &spec.kubernetes {
        containers.extend(k8s.sidecars.iter().map(container_from_spec));
    }
    let init_containers: Option<Vec<Container>> = spec
        .kubernetes
        .as_ref()
        .filter(|k| !k.init_containers.is_empty())
        .map(|k| k.init_containers.iter().map(container_from_spec).collect());
    let volumes = build_volumes(spec);
    let pod_settings = spec.kubernetes.as_ref().and_then(|k| k.pod.as_ref());
    let mut image_pull_secrets = build_image_pull_secrets(spec).unwrap_or_default();
    if let Some(reg) = spec
        .kubernetes
        .as_ref()
        .and_then(|k| k.docker_registry_secret.as_ref())
    {
        if !image_pull_secrets.iter().any(|s| s.name == reg.name) {
            image_pull_secrets.push(LocalObjectReference {
                name: reg.name.clone(),
            });
        }
    }
    PodTemplateSpec {
        metadata: Some(ObjectMeta {
            labels: Some(labels),
            annotations: Some(pod_annotations(spec)),
            ..Default::default()
        }),
        spec: Some(PodSpec {
            containers,
            init_containers,
            volumes: if volumes.is_empty() {
                None
            } else {
                Some(volumes)
            },
            restart_policy: restart_policy.map(str::to_string),
            image_pull_secrets: if image_pull_secrets.is_empty() {
                None
            } else {
                Some(image_pull_secrets)
            },
            service_account_name: crate::adapters::kube_extras::service_account_name(spec).or_else(
                || {
                    spec.kubernetes
                        .as_ref()
                        .and_then(|k| k.service_account_name.clone())
                },
            ),
            priority_class_name: spec
                .kubernetes
                .as_ref()
                .and_then(|k| k.priority_class_name.clone()),
            node_selector: spec
                .kubernetes
                .as_ref()
                .filter(|k| !k.node_selector.is_empty())
                .map(|k| k.node_selector.clone().into_iter().collect()),
            tolerations: build_tolerations(spec),
            affinity: build_affinity(spec),
            security_context: build_pod_security_context(spec),
            termination_grace_period_seconds: pod_settings
                .and_then(|p| p.termination_grace_period_seconds),
            host_network: pod_settings.and_then(|p| p.host_network),
            dns_policy: pod_settings.and_then(|p| p.dns_policy.clone()),
            topology_spread_constraints: build_topology_spread(spec),
            runtime_class_name: confidential_runtime_class(spec),
            ..Default::default()
        }),
    }
}

fn confidential_runtime_class(spec: &Workload) -> Option<String> {
    crate::ragnarok::kata::resolve_runtime_class(spec)
}

fn object_meta(namespace: &str, spec: &Workload) -> ObjectMeta {
    ObjectMeta {
        name: Some(spec.metadata.name.clone()),
        namespace: Some(namespace.to_string()),
        labels: Some(workload_labels(spec)),
        annotations: Some(spec.metadata.annotations.clone().into_iter().collect()),
        ..Default::default()
    }
}

fn selector_labels(spec: &Workload) -> BTreeMap<String, String> {
    let mut match_labels = BTreeMap::new();
    match_labels.insert("app".to_string(), spec.metadata.name.clone());
    match_labels.insert("managed-by".to_string(), "aether".to_string());
    match_labels
}

pub(crate) fn build_statefulset_manifest(
    namespace: &str,
    image: &Image,
    spec: &Workload,
) -> StatefulSet {
    let replicas = spec
        .scaling
        .as_ref()
        .filter(|s| s.enabled)
        .map(|s| s.min_replicas as i32)
        .unwrap_or(1);
    let vct = volume_claim_template(spec).map(|pvc| vec![pvc]);
    StatefulSet {
        metadata: object_meta(namespace, spec),
        spec: Some(StatefulSetSpec {
            replicas: Some(replicas),
            selector: LabelSelector {
                match_labels: Some(selector_labels(spec)),
                ..Default::default()
            },
            service_name: format!("{}-service", spec.metadata.name),
            volume_claim_templates: vct,
            template: build_pod_template_spec(image, spec, None),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub(crate) fn build_daemonset_manifest(
    namespace: &str,
    image: &Image,
    spec: &Workload,
) -> DaemonSet {
    DaemonSet {
        metadata: object_meta(namespace, spec),
        spec: Some(DaemonSetSpec {
            selector: LabelSelector {
                match_labels: Some(selector_labels(spec)),
                ..Default::default()
            },
            template: build_pod_template_spec(image, spec, None),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub(crate) fn build_job_manifest(namespace: &str, image: &Image, spec: &Workload) -> Job {
    let job_cfg = spec.kubernetes.as_ref().and_then(|k| k.job.as_ref());
    let backoff = job_cfg.map(|j| j.backoff_limit as i32).unwrap_or(3);
    let completions = job_cfg.map(|j| j.completions as i32);
    let parallelism = job_cfg.map(|j| j.parallelism as i32);
    let active_deadline = job_cfg.and_then(|j| j.active_deadline_seconds);
    let ttl = job_cfg.and_then(|j| j.ttl_seconds_after_finished);
    let restart_policy = job_cfg
        .map(|j| match j.restart_policy {
            crate::spec::JobRestartPolicy::Never => "Never",
            crate::spec::JobRestartPolicy::OnFailure => "OnFailure",
        })
        .unwrap_or("Never");
    Job {
        metadata: object_meta(namespace, spec),
        spec: Some(JobSpec {
            backoff_limit: Some(backoff),
            completions,
            parallelism,
            active_deadline_seconds: active_deadline,
            ttl_seconds_after_finished: ttl,
            template: build_pod_template_spec(image, spec, Some(restart_policy)),
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub(crate) fn build_pdb_manifest(namespace: &str, spec: &Workload) -> Option<PodDisruptionBudget> {
    let pdb = spec.kubernetes.as_ref()?.pod_disruption_budget.as_ref()?;
    let mut labels = workload_labels(spec);
    labels.insert("component".to_string(), "pdb".to_string());
    Some(PodDisruptionBudget {
        metadata: ObjectMeta {
            name: Some(format!("{}-pdb", spec.metadata.name)),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        spec: Some(PodDisruptionBudgetSpec {
            min_available: pdb
                .min_available
                .as_ref()
                .map(|v| int_or_string_from_str(v)),
            max_unavailable: pdb
                .max_unavailable
                .as_ref()
                .map(|v| int_or_string_from_str(v)),
            selector: Some(LabelSelector {
                match_labels: Some(selector_labels(spec)),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    })
}

pub(crate) fn build_gateway_http_route_json(namespace: &str, spec: &Workload) -> Option<Value> {
    let gw = spec.kubernetes.as_ref()?.gateway.as_ref()?;
    if !gw.enabled {
        return None;
    }
    let paths = if gw.paths.is_empty() {
        vec![json!({
            "matches": [{ "path": { "type": "PathPrefix", "value": "/" } }],
            "backendRefs": [{
                "name": format!("{}-service", spec.metadata.name),
                "port": spec.network.ports.first().map(|p| p.service_port).unwrap_or(80)
            }]
        })]
    } else {
        gw.paths
            .iter()
            .map(|p| {
                json!({
                    "matches": [{ "path": { "type": p.path_type, "value": p.path } }],
                    "backendRefs": [{
                        "name": format!("{}-service", spec.metadata.name),
                        "port": p.port
                    }]
                })
            })
            .collect()
    };
    Some(json!({
        "apiVersion": "gateway.networking.k8s.io/v1",
        "kind": "HTTPRoute",
        "metadata": {
            "name": format!("{}-route", spec.metadata.name),
            "namespace": namespace,
            "labels": { "app": spec.metadata.name, "managed-by": "aether" }
        },
        "spec": {
            "parentRefs": [{
                "name": gw.gateway_name,
                "namespace": gw.gateway_namespace
            }],
            "hostnames": [gw.host],
            "rules": paths
        }
    }))
}

pub(crate) fn build_gateway_provision_json(
    namespace: &str,
    spec: &Workload,
) -> Option<serde_json::Value> {
    let gw = spec.kubernetes.as_ref()?.gateway.as_ref()?;
    if !gw.enabled || !gw.provision_gateway {
        return None;
    }
    let class_name = gw
        .gateway_class_name
        .clone()
        .unwrap_or_else(|| "aether".into());
    let gw_namespace = if gw.gateway_namespace == "default" {
        namespace.to_string()
    } else {
        gw.gateway_namespace.clone()
    };
    Some(serde_json::json!({
        "apiVersion": "gateway.networking.k8s.io/v1",
        "kind": "Gateway",
        "metadata": {
            "name": gw.gateway_name.clone(),
            "namespace": gw_namespace,
            "labels": { "app": spec.metadata.name, "managed-by": "aether" }
        },
        "spec": {
            "gatewayClassName": class_name,
            "listeners": [{
                "name": "http",
                "protocol": "HTTP",
                "port": 80,
                "hostname": gw.host.clone(),
                "allowedRoutes": {
                    "namespaces": { "from": "Same" }
                }
            }]
        }
    }))
}

pub(crate) fn build_vpa_json(namespace: &str, spec: &Workload, target_kind: &str) -> Option<Value> {
    let vpa = spec.kubernetes.as_ref()?.vertical_pod_autoscaler.as_ref()?;
    if !vpa.enabled {
        return None;
    }
    Some(json!({
        "apiVersion": "autoscaling.k8s.io/v1",
        "kind": "VerticalPodAutoscaler",
        "metadata": {
            "name": format!("{}-vpa", spec.metadata.name),
            "namespace": namespace,
            "labels": { "app": spec.metadata.name, "managed-by": "aether" }
        },
        "spec": {
            "targetRef": {
                "apiVersion": "apps/v1",
                "kind": target_kind,
                "name": spec.metadata.name
            },
            "updatePolicy": { "updateMode": vpa.update_mode }
        }
    }))
}

pub(crate) fn build_keda_json(namespace: &str, spec: &Workload) -> Option<Value> {
    let keda = spec.kubernetes.as_ref()?.keda.as_ref()?;
    if !keda.enabled {
        return None;
    }
    let triggers: Vec<Value> = keda
        .triggers
        .iter()
        .map(|t| {
            json!({
                "type": t.trigger_type,
                "metadata": t.metadata
            })
        })
        .collect();
    Some(json!({
        "apiVersion": "keda.sh/v1alpha1",
        "kind": "ScaledObject",
        "metadata": {
            "name": format!("{}-keda", spec.metadata.name),
            "namespace": namespace,
            "labels": { "app": spec.metadata.name, "managed-by": "aether" }
        },
        "spec": {
            "scaleTargetRef": { "name": spec.metadata.name },
            "minReplicaCount": keda.min_replica_count,
            "maxReplicaCount": keda.max_replica_count,
            "triggers": triggers
        }
    }))
}

/// Rebuild Deployment using shared pod template (used when migrating probe/pod logic).
pub(crate) fn build_deployment_manifest_v2(
    namespace: &str,
    image: &Image,
    spec: &Workload,
) -> Deployment {
    let replicas = spec
        .scaling
        .as_ref()
        .filter(|s| s.enabled)
        .map(|s| s.min_replicas as i32)
        .unwrap_or(1);
    Deployment {
        metadata: object_meta(namespace, spec),
        spec: Some(DeploymentSpec {
            replicas: Some(replicas),
            selector: LabelSelector {
                match_labels: Some(selector_labels(spec)),
                ..Default::default()
            },
            strategy: build_deployment_strategy(spec),
            template: build_pod_template_spec(image, spec, None),
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn build_deployment_strategy(spec: &Workload) -> Option<DeploymentStrategy> {
    let rs = spec.kubernetes.as_ref()?.rollout_strategy.as_ref()?;
    Some(DeploymentStrategy {
        type_: Some(rs.strategy_type.clone()),
        rolling_update: if rs.strategy_type.eq_ignore_ascii_case("RollingUpdate") {
            Some(RollingUpdateDeployment {
                max_surge: rs
                    .max_surge
                    .as_ref()
                    .map(|s| int_or_string_from_str(s.as_str())),
                max_unavailable: rs
                    .max_unavailable
                    .as_ref()
                    .map(|s| int_or_string_from_str(s.as_str())),
            })
        } else {
            None
        },
    })
}

/// Build a Service manifest (ClusterIP, NodePort, LoadBalancer, Headless, ExternalName).
pub(crate) fn build_service_manifest(namespace: &str, spec: &Workload) -> Option<Service> {
    if !spec.network.service {
        return None;
    }

    let labels = workload_labels(spec);

    let ports: Vec<ServicePort> = spec
        .network
        .ports
        .iter()
        .map(|p| ServicePort {
            port: p.service_port as i32,
            target_port: Some(IntOrString::Int(p.container_port as i32)),
            protocol: Some(p.protocol.clone()),
            ..Default::default()
        })
        .collect();

    let headless = spec.wants_headless_service();
    let service_type = if spec.network.external_name.is_some() {
        "ExternalName"
    } else if headless {
        "ClusterIP"
    } else {
        match spec.network.service_type {
            crate::spec::ServiceType::ClusterIP | crate::spec::ServiceType::Headless => "ClusterIP",
            crate::spec::ServiceType::NodePort => "NodePort",
            crate::spec::ServiceType::LoadBalancer => "LoadBalancer",
            crate::spec::ServiceType::ExternalName => "ExternalName",
        }
    };

    let mut selector = selector_labels(spec);
    if service_type == "ExternalName" {
        selector.clear();
    }

    Some(Service {
        metadata: ObjectMeta {
            name: Some(format!("{}-service", spec.metadata.name)),
            namespace: Some(namespace.to_string()),
            labels: Some(labels),
            ..Default::default()
        },
        spec: Some(ServiceSpec {
            type_: Some(service_type.to_string()),
            cluster_ip: if headless {
                Some("None".to_string())
            } else {
                None
            },
            external_name: spec.network.external_name.clone(),
            session_affinity: spec.network.session_affinity.clone(),
            external_traffic_policy: spec.network.external_traffic_policy.clone(),
            ports: if ports.is_empty() { None } else { Some(ports) },
            selector: if selector.is_empty() {
                None
            } else {
                Some(selector)
            },
            ..Default::default()
        }),
        ..Default::default()
    })
}

pub(crate) fn build_cronjob_manifest_v2(
    namespace: &str,
    image: &Image,
    spec: &Workload,
) -> CronJob {
    let schedule_spec = spec
        .schedule
        .as_ref()
        .expect("schedule required for CronJob");
    let restart_policy = match schedule_spec.restart_policy {
        crate::spec::JobRestartPolicy::Never => "Never",
        crate::spec::JobRestartPolicy::OnFailure => "OnFailure",
    };
    let concurrency_policy = match schedule_spec.concurrency_policy {
        crate::spec::ConcurrencyPolicy::Allow => "Allow",
        crate::spec::ConcurrencyPolicy::Forbid => "Forbid",
        crate::spec::ConcurrencyPolicy::Replace => "Replace",
    };
    CronJob {
        metadata: object_meta(namespace, spec),
        spec: Some(CronJobSpec {
            schedule: schedule_spec.cron.clone(),
            concurrency_policy: Some(concurrency_policy.to_string()),
            job_template: JobTemplateSpec {
                metadata: Some(ObjectMeta {
                    labels: Some(workload_labels(spec)),
                    ..Default::default()
                }),
                spec: Some(JobSpec {
                    backoff_limit: Some(schedule_spec.backoff_limit as i32),
                    active_deadline_seconds: schedule_spec.active_deadline_seconds,
                    template: build_pod_template_spec(image, spec, Some(restart_policy)),
                    ..Default::default()
                }),
            },
            ..Default::default()
        }),
        ..Default::default()
    }
}

/// Whether a standalone PVC should be created (Deployment/Job path).
pub(crate) fn needs_standalone_pvc(spec: &Workload) -> bool {
    spec.persistence.enabled && spec.resolved_k8s_workload_kind() != K8sWorkloadKind::StatefulSet
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{
        BuildSpec, HealthSpec, K8sWorkloadKind, Metadata, NetworkSpec, PersistenceSpec,
        ResourceRequirements, RuntimePreference, RuntimeSpec, RuntimeType,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;

    fn test_workload() -> Workload {
        Workload {
            api_version: "aether/v1".to_string(),
            kind: "Workload".to_string(),
            metadata: Metadata {
                name: "test-app".to_string(),
                owner: "test".to_string(),
                project: "demo".to_string(),
                labels: HashMap::new(),
                annotations: HashMap::new(),
            },
            build: BuildSpec {
                context: PathBuf::from("."),
                dockerfile: PathBuf::from("Dockerfile"),
                registry: "ghcr.io/test".to_string(),
                build_args: HashMap::new(),
                ..Default::default()
            },
            requirements: ResourceRequirements {
                cpu: "1".to_string(),
                memory: "512Mi".to_string(),
                storage: "1Gi".to_string(),
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
            confidential: None,
            schedule: None,
            kubernetes: None,
            kubevirt: None,
        }
    }

    fn test_image() -> Image {
        Image {
            name: "test-app".to_string(),
            tag: "latest".to_string(),
            digest: None,
            runtime: crate::runtime::RuntimeKind::Kubernetes,
        }
    }

    #[test]
    fn test_tcp_probe_maps_to_k8s() {
        let mut spec = test_workload();
        spec.health = Some(HealthSpec {
            liveness: Some(HealthProbe {
                probe_type: ProbeType::TcpSocket { port: 8080 },
                initial_delay_seconds: 5,
                period_seconds: 10,
            }),
            readiness: None,
            startup: None,
        });
        let deploy = build_deployment_manifest_v2("default", &test_image(), &spec);
        let container = &deploy
            .spec
            .as_ref()
            .unwrap()
            .template
            .spec
            .as_ref()
            .unwrap()
            .containers[0];
        let probe = container.liveness_probe.as_ref().unwrap();
        assert!(probe.tcp_socket.is_some());
        assert_eq!(
            probe.tcp_socket.as_ref().unwrap().port,
            IntOrString::Int(8080)
        );
    }

    #[test]
    fn test_statefulset_volume_claim_template() {
        let mut spec = test_workload();
        spec.persistence.enabled = true;
        spec.persistence.size = "10Gi".to_string();
        spec.kubernetes = Some(crate::spec::KubernetesSpec {
            workload_kind: Some(K8sWorkloadKind::StatefulSet),
            ..Default::default()
        });
        let sts = build_statefulset_manifest("default", &test_image(), &spec);
        assert!(
            sts.spec
                .as_ref()
                .unwrap()
                .volume_claim_templates
                .as_ref()
                .unwrap()
                .len()
                == 1
        );
        assert!(!needs_standalone_pvc(&spec));
    }

    #[test]
    fn test_grpc_probe_maps_to_k8s() {
        let mut spec = test_workload();
        spec.health = Some(HealthSpec {
            liveness: Some(HealthProbe {
                probe_type: ProbeType::Grpc {
                    port: 9090,
                    service: "my.Service".to_string(),
                },
                initial_delay_seconds: 5,
                period_seconds: 10,
            }),
            readiness: None,
            startup: None,
        });
        let deploy = build_deployment_manifest_v2("default", &test_image(), &spec);
        let probe = deploy
            .spec
            .as_ref()
            .unwrap()
            .template
            .spec
            .as_ref()
            .unwrap()
            .containers[0]
            .liveness_probe
            .as_ref()
            .unwrap();
        assert!(probe.grpc.is_some());
    }

    #[test]
    fn test_headless_service_for_statefulset() {
        let mut spec = test_workload();
        spec.network.service = true;
        spec.kubernetes = Some(crate::spec::KubernetesSpec {
            workload_kind: Some(K8sWorkloadKind::StatefulSet),
            ..Default::default()
        });
        let svc = build_service_manifest("default", &spec).unwrap();
        assert_eq!(
            svc.spec.as_ref().unwrap().cluster_ip.as_deref(),
            Some("None")
        );
    }

    #[test]
    fn test_job_manifest() {
        let mut spec = test_workload();
        spec.kubernetes = Some(crate::spec::KubernetesSpec {
            workload_kind: Some(K8sWorkloadKind::Job),
            ..Default::default()
        });
        let job = build_job_manifest("default", &test_image(), &spec);
        assert_eq!(job.metadata.name.as_deref(), Some("test-app"));
    }

    #[test]
    fn test_sanitize_volume_name() {
        assert_eq!(sanitize_volume_name("cm-app-config"), "cm-app-config");
        assert_eq!(sanitize_volume_name("CM-MyConfig"), "cm-myconfig");
        assert_eq!(
            sanitize_volume_name("secret-tls.crt@v2"),
            "secret-tls-crt-v2"
        );
        assert_eq!(sanitize_volume_name("--name--"), "name");
        assert_eq!(sanitize_volume_name(""), "vol");
        assert_eq!(sanitize_volume_name(&"a".repeat(100)).len(), 63);
    }

    #[test]
    fn gateway_provision_uses_workload_namespace_when_default() {
        use crate::spec::{K8sGatewaySpec, K8sWorkloadKind, KubernetesSpec};

        let mut spec = test_workload();
        spec.metadata.name = "web".into();
        spec.kubernetes = Some(KubernetesSpec {
            workload_kind: Some(K8sWorkloadKind::Deployment),
            gateway: Some(K8sGatewaySpec {
                enabled: true,
                gateway_name: "edge".into(),
                gateway_namespace: "default".into(),
                host: "app.example.com".into(),
                provision_gateway: true,
                paths: vec![],
                gateway_class_name: None,
            }),
            ..Default::default()
        });
        let gw = build_gateway_provision_json("staging", &spec).expect("gateway json");
        assert_eq!(gw["metadata"]["namespace"], "staging");
    }

    #[test]
    fn gateway_provision_keeps_explicit_gateway_namespace() {
        use crate::spec::{K8sGatewaySpec, K8sWorkloadKind, KubernetesSpec};

        let mut spec = test_workload();
        spec.kubernetes = Some(KubernetesSpec {
            workload_kind: Some(K8sWorkloadKind::Deployment),
            gateway: Some(K8sGatewaySpec {
                enabled: true,
                gateway_name: "edge".into(),
                gateway_namespace: "ingress-system".into(),
                host: "app.example.com".into(),
                provision_gateway: true,
                paths: vec![],
                gateway_class_name: None,
            }),
            ..Default::default()
        });
        let gw = build_gateway_provision_json("staging", &spec).expect("gateway json");
        assert_eq!(gw["metadata"]["namespace"], "ingress-system");
    }
}
