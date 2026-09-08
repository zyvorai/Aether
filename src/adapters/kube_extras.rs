// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Optional Kubernetes resources: ServiceAccount/RBAC, cert-manager, ServiceMonitor, registry secrets.

#![allow(clippy::needless_update)]

use crate::spec::{K8sDockerRegistrySecretSpec, K8sLimitRangeItemSpec, K8sRbacRuleSpec, Workload};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use k8s_openapi::api::core::v1::{
    LimitRange, LimitRangeItem, LimitRangeSpec, ResourceQuota, ResourceQuotaSpec, Secret,
    ServiceAccount,
};
use k8s_openapi::api::rbac::v1::{PolicyRule, Role, RoleBinding, RoleRef, Subject};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn workload_labels(spec: &Workload) -> BTreeMap<String, String> {
    let mut labels = BTreeMap::new();
    labels.insert("app".to_string(), spec.metadata.name.clone());
    labels.insert("managed-by".to_string(), "aether".to_string());
    labels
}

pub(crate) fn service_account_name(spec: &Workload) -> Option<String> {
    if let Some(sa) = spec.kubernetes.as_ref()?.service_account.as_ref() {
        if sa.create {
            return Some(
                sa.name
                    .clone()
                    .unwrap_or_else(|| spec.metadata.name.clone()),
            );
        }
    }
    spec.kubernetes
        .as_ref()
        .and_then(|k| k.service_account_name.clone())
}

pub(crate) fn build_service_account(spec: &Workload, namespace: &str) -> Option<ServiceAccount> {
    let sa_spec = spec.kubernetes.as_ref()?.service_account.as_ref()?;
    if !sa_spec.create {
        return None;
    }
    let name = sa_spec
        .name
        .clone()
        .unwrap_or_else(|| spec.metadata.name.clone());
    Some(ServiceAccount {
        metadata: ObjectMeta {
            name: Some(name),
            namespace: Some(namespace.to_string()),
            labels: Some(workload_labels(spec)),
            annotations: if sa_spec.annotations.is_empty() {
                None
            } else {
                Some(sa_spec.annotations.clone().into_iter().collect())
            },
            ..Default::default()
        },
        ..Default::default()
    })
}

pub(crate) fn build_role(spec: &Workload, namespace: &str) -> Option<Role> {
    let sa_spec = spec.kubernetes.as_ref()?.service_account.as_ref()?;
    if !sa_spec.create || sa_spec.rules.is_empty() {
        return None;
    }
    let name = format!(
        "{}-role",
        sa_spec
            .name
            .clone()
            .unwrap_or_else(|| spec.metadata.name.clone())
    );
    Some(Role {
        metadata: ObjectMeta {
            name: Some(name),
            namespace: Some(namespace.to_string()),
            labels: Some(workload_labels(spec)),
            ..Default::default()
        },
        rules: Some(sa_spec.rules.iter().map(rbac_rule_from_spec).collect()),
    })
}

fn rbac_rule_from_spec(rule: &K8sRbacRuleSpec) -> PolicyRule {
    PolicyRule {
        api_groups: if rule.api_groups.is_empty() {
            Some(vec!["".to_string()])
        } else {
            Some(rule.api_groups.clone())
        },
        resources: Some(rule.resources.clone()),
        verbs: rule.verbs.clone(),
        ..Default::default()
    }
}

pub(crate) fn build_role_binding(spec: &Workload, namespace: &str) -> Option<RoleBinding> {
    let sa_spec = spec.kubernetes.as_ref()?.service_account.as_ref()?;
    if !sa_spec.create || sa_spec.rules.is_empty() {
        return None;
    }
    let sa_name = sa_spec
        .name
        .clone()
        .unwrap_or_else(|| spec.metadata.name.clone());
    let role_name = format!("{}-role", sa_name);
    Some(RoleBinding {
        metadata: ObjectMeta {
            name: Some(format!("{}-rolebinding", sa_name)),
            namespace: Some(namespace.to_string()),
            labels: Some(workload_labels(spec)),
            ..Default::default()
        },
        role_ref: RoleRef {
            api_group: "rbac.authorization.k8s.io".to_string(),
            kind: "Role".to_string(),
            name: role_name,
        },
        subjects: Some(vec![Subject {
            kind: "ServiceAccount".to_string(),
            name: sa_name,
            namespace: Some(namespace.to_string()),
            ..Default::default()
        }]),
    })
}

pub(crate) fn build_docker_registry_secret(namespace: &str, spec: &Workload) -> Option<Secret> {
    let reg = spec.kubernetes.as_ref()?.docker_registry_secret.as_ref()?;
    Some(build_docker_config_secret(namespace, spec, reg))
}

pub(crate) fn build_docker_config_secret(
    namespace: &str,
    spec: &Workload,
    reg: &K8sDockerRegistrySecretSpec,
) -> Secret {
    let auth = BASE64.encode(format!("{}:{}", reg.username, reg.password));
    let email = reg
        .email
        .clone()
        .unwrap_or_else(|| "aether@local".to_string());
    let docker_config = json!({
        "auths": {
            reg.registry.clone(): {
                "username": reg.username,
                "password": reg.password,
                "email": email,
                "auth": auth
            }
        }
    });
    let data = BTreeMap::from([(
        ".dockerconfigjson".to_string(),
        k8s_openapi::ByteString(docker_config.to_string().into_bytes()),
    )]);
    Secret {
        metadata: ObjectMeta {
            name: Some(reg.name.clone()),
            namespace: Some(namespace.to_string()),
            labels: Some(workload_labels(spec)),
            ..Default::default()
        },
        type_: Some("kubernetes.io/dockerconfigjson".to_string()),
        data: Some(data),
        ..Default::default()
    }
}

pub(crate) fn build_certificate_json(namespace: &str, spec: &Workload) -> Option<Value> {
    let cm = spec.kubernetes.as_ref()?.cert_manager.as_ref()?;
    if !cm.enabled {
        return None;
    }
    let ingress = spec.ingress.as_ref()?;
    let secret_name = cm
        .secret_name
        .clone()
        .or_else(|| ingress.tls_secret_name.clone())
        .unwrap_or_else(|| format!("{}-tls", spec.metadata.name));
    let issuer_ref = if cm.issuer_kind.eq_ignore_ascii_case("issuer") {
        json!({ "name": cm.issuer_name, "kind": "Issuer" })
    } else {
        json!({ "name": cm.issuer_name, "kind": "ClusterIssuer" })
    };
    Some(json!({
        "apiVersion": "cert-manager.io/v1",
        "kind": "Certificate",
        "metadata": {
            "name": format!("{}-cert", spec.metadata.name),
            "namespace": namespace,
            "labels": { "app": spec.metadata.name, "managed-by": "aether" }
        },
        "spec": {
            "secretName": secret_name,
            "issuerRef": issuer_ref,
            "dnsNames": [ingress.host]
        }
    }))
}

pub(crate) fn build_service_monitor_json(namespace: &str, spec: &Workload) -> Option<Value> {
    let sm = spec.kubernetes.as_ref()?.service_monitor.as_ref()?;
    if !sm.enabled {
        return None;
    }
    Some(json!({
        "apiVersion": "monitoring.coreos.com/v1",
        "kind": "ServiceMonitor",
        "metadata": {
            "name": format!("{}-monitor", spec.metadata.name),
            "namespace": namespace,
            "labels": { "app": spec.metadata.name, "managed-by": "aether" }
        },
        "spec": {
            "selector": {
                "matchLabels": { "app": spec.metadata.name }
            },
            "endpoints": [{
                "port": sm.port,
                "path": sm.path,
                "interval": sm.interval
            }]
        }
    }))
}

pub(crate) fn certificate_api_resource() -> kube::api::ApiResource {
    kube::api::ApiResource {
        group: "cert-manager.io".into(),
        version: "v1".into(),
        api_version: "cert-manager.io/v1".into(),
        kind: "Certificate".into(),
        plural: "certificates".into(),
    }
}

pub(crate) fn service_monitor_api_resource() -> kube::api::ApiResource {
    kube::api::ApiResource {
        group: "monitoring.coreos.com".into(),
        version: "v1".into(),
        api_version: "monitoring.coreos.com/v1".into(),
        kind: "ServiceMonitor".into(),
        plural: "servicemonitors".into(),
    }
}

fn quantity_map(
    values: &std::collections::HashMap<String, String>,
) -> BTreeMap<String, k8s_openapi::apimachinery::pkg::api::resource::Quantity> {
    values
        .iter()
        .map(|(k, v)| {
            (
                k.clone(),
                k8s_openapi::apimachinery::pkg::api::resource::Quantity(v.clone()),
            )
        })
        .collect()
}

pub(crate) fn build_resource_quota(namespace: &str, spec: &Workload) -> Option<ResourceQuota> {
    let rq = spec.kubernetes.as_ref()?.resource_quota.as_ref()?;
    if !rq.enabled || rq.hard.is_empty() {
        return None;
    }
    Some(ResourceQuota {
        metadata: ObjectMeta {
            name: Some(format!("{}-quota", spec.metadata.name)),
            namespace: Some(namespace.to_string()),
            labels: Some(workload_labels(spec)),
            ..Default::default()
        },
        spec: Some(ResourceQuotaSpec {
            hard: Some(quantity_map(&rq.hard)),
            scopes: if rq.scopes.is_empty() {
                None
            } else {
                Some(rq.scopes.clone())
            },
            ..Default::default()
        }),
        ..Default::default()
    })
}

fn limit_range_item(item: &K8sLimitRangeItemSpec) -> LimitRangeItem {
    LimitRangeItem {
        type_: item.limit_type.clone(),
        default: if item.default.is_empty() {
            None
        } else {
            Some(quantity_map(&item.default))
        },
        default_request: if item.default_request.is_empty() {
            None
        } else {
            Some(quantity_map(&item.default_request))
        },
        max: if item.max.is_empty() {
            None
        } else {
            Some(quantity_map(&item.max))
        },
        min: if item.min.is_empty() {
            None
        } else {
            Some(quantity_map(&item.min))
        },
        ..Default::default()
    }
}

pub(crate) fn build_limit_range(namespace: &str, spec: &Workload) -> Option<LimitRange> {
    let lr = spec.kubernetes.as_ref()?.limit_range.as_ref()?;
    if !lr.enabled || lr.limits.is_empty() {
        return None;
    }
    Some(LimitRange {
        metadata: ObjectMeta {
            name: Some(format!("{}-limits", spec.metadata.name)),
            namespace: Some(namespace.to_string()),
            labels: Some(workload_labels(spec)),
            ..Default::default()
        },
        spec: Some(LimitRangeSpec {
            limits: lr.limits.iter().map(limit_range_item).collect(),
        }),
    })
}
