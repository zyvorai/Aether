// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Reconcile ancillary Kubernetes resources on create and update.

use crate::spec::{K8sWorkloadKind, Workload};
use k8s_openapi::api::autoscaling::v2::HorizontalPodAutoscaler;
use k8s_openapi::api::core::v1::{LimitRange, PersistentVolumeClaim, ResourceQuota};
use k8s_openapi::api::policy::v1::PodDisruptionBudget;
use kube::api::{Api, DeleteParams, Patch, PatchParams, PostParams};
use kube::core::NamespaceResourceScope;
use kube::Client;

use super::kube::{
    build_configmap_manifests, build_hpa_manifest, build_ingress_manifest,
    build_networkpolicy_manifest, build_pvc_manifest, build_secret_manifests,
    build_service_manifest, create_dynamic_resource, delete_dynamic_resource,
    http_route_api_resource, is_already_exists, keda_api_resource, kube_with_timeout,
    reconcile_dynamic_resource, vpa_api_resource,
};

/// Whether ancillary resources are being created (deploy) or reconciled (update).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconcileMode {
    Create,
    Update,
}

/// A resource tracked during create for rollback cleanup.
#[derive(Debug, Clone)]
pub struct ReconcileResource {
    pub kind: &'static str,
    pub name: String,
}

/// Reconcile all ancillary K8s resources except the workload controller.
pub async fn reconcile_k8s_ancillaries(
    client: &Client,
    namespace: &str,
    spec: &Workload,
    workload_kind: K8sWorkloadKind,
    mode: ReconcileMode,
) -> crate::Result<Vec<ReconcileResource>> {
    let mut tracked = Vec::new();
    let fail_fast = mode == ReconcileMode::Create;

    // ConfigMaps
    for configmap in build_configmap_manifests(namespace, spec) {
        let cm_name = configmap.metadata.name.clone().unwrap_or_default();
        apply_or_create(
            client,
            namespace,
            &cm_name,
            configmap,
            mode,
            fail_fast,
            "ConfigMap",
        )
        .await?;
        if mode == ReconcileMode::Create {
            tracked.push(ReconcileResource {
                kind: "configmap",
                name: cm_name,
            });
        }
    }

    // Secrets
    for secret in build_secret_manifests(namespace, spec) {
        let secret_name = secret.metadata.name.clone().unwrap_or_default();
        apply_or_create(
            client,
            namespace,
            &secret_name,
            secret,
            mode,
            fail_fast,
            "Secret",
        )
        .await?;
        if mode == ReconcileMode::Create {
            tracked.push(ReconcileResource {
                kind: "secret",
                name: secret_name,
            });
        }
    }

    // Docker registry pull secret
    if let Some(secret) =
        crate::adapters::kube_extras::build_docker_registry_secret(namespace, spec)
    {
        let secret_name = secret.metadata.name.clone().unwrap_or_default();
        apply_or_create(
            client,
            namespace,
            &secret_name,
            secret,
            mode,
            fail_fast,
            "Registry Secret",
        )
        .await?;
        if mode == ReconcileMode::Create {
            tracked.push(ReconcileResource {
                kind: "secret",
                name: secret_name,
            });
        }
    }

    // ServiceAccount + RBAC
    if let Some(sa) = crate::adapters::kube_extras::build_service_account(spec, namespace) {
        let sa_name = sa.metadata.name.clone().unwrap_or_default();
        apply_or_create(
            client,
            namespace,
            &sa_name,
            sa,
            mode,
            fail_fast,
            "ServiceAccount",
        )
        .await?;
        if mode == ReconcileMode::Create {
            tracked.push(ReconcileResource {
                kind: "serviceaccount",
                name: sa_name,
            });
        }
    }
    if let Some(role) = crate::adapters::kube_extras::build_role(spec, namespace) {
        let role_name = role.metadata.name.clone().unwrap_or_default();
        apply_or_create(client, namespace, &role_name, role, mode, fail_fast, "Role").await?;
        if mode == ReconcileMode::Create {
            tracked.push(ReconcileResource {
                kind: "role",
                name: role_name,
            });
        }
    }
    if let Some(binding) = crate::adapters::kube_extras::build_role_binding(spec, namespace) {
        let binding_name = binding.metadata.name.clone().unwrap_or_default();
        apply_or_create(
            client,
            namespace,
            &binding_name,
            binding,
            mode,
            fail_fast,
            "RoleBinding",
        )
        .await?;
        if mode == ReconcileMode::Create {
            tracked.push(ReconcileResource {
                kind: "rolebinding",
                name: binding_name,
            });
        }
    }

    // PVC (create only — skip shrink on update). When the workload is Atlas-backed
    // (`storageClass: atlas/…` + AETHER_ATLAS_URL set), Atlas already created the
    // PVC (`{name}-pvc`), so Aether must not create a native one.
    let atlas_backed =
        spec.atlas_policy().is_some() && crate::atlas::AtlasConfig::from_env().is_some();
    if mode == ReconcileMode::Create
        && !atlas_backed
        && crate::adapters::kube_manifest::needs_standalone_pvc(spec)
    {
        if let Some(pvc) = build_pvc_manifest(namespace, spec) {
            let pvc_name = format!("{}-pvc", spec.metadata.name);
            let pvcs: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), namespace);
            match kube_with_timeout("PVC create", pvcs.create(&PostParams::default(), &pvc)).await {
                Ok(_) => {
                    tracing::info!("Created PVC: {}", pvc_name);
                    tracked.push(ReconcileResource {
                        kind: "pvc",
                        name: pvc_name,
                    });
                }
                Err(e)
                    if e.downcast_ref::<kube::Error>()
                        .is_some_and(is_already_exists) =>
                {
                    tracing::info!("PVC already exists: {}", pvc_name);
                }
                Err(e) => return Err(e),
            }
        }
    }

    // Service
    reconcile_optional(
        client,
        namespace,
        spec,
        mode,
        fail_fast,
        build_service_manifest(namespace, spec),
        |s| format!("{}-service", s.metadata.name),
        "service",
        "Service",
        &mut tracked,
    )
    .await?;

    // Ingress
    reconcile_optional(
        client,
        namespace,
        spec,
        mode,
        fail_fast,
        build_ingress_manifest(namespace, spec),
        |s| format!("{}-ingress", s.metadata.name),
        "ingress",
        "Ingress",
        &mut tracked,
    )
    .await?;

    // cert-manager Certificate
    if let Some(cert) = crate::adapters::kube_extras::build_certificate_json(namespace, spec) {
        reconcile_dynamic_optional(
            client,
            namespace,
            &format!("{}-cert", spec.metadata.name),
            cert,
            crate::adapters::kube_extras::certificate_api_resource(),
            mode,
            "certificate",
            &mut tracked,
        )
        .await;
    } else if mode == ReconcileMode::Update {
        delete_dynamic_resource(
            client,
            namespace,
            &format!("{}-cert", spec.metadata.name),
            crate::adapters::kube_extras::certificate_api_resource(),
        )
        .await;
    }

    // ResourceQuota / LimitRange
    if let Some(rq) = crate::adapters::kube_extras::build_resource_quota(namespace, spec) {
        let name = rq.metadata.name.clone().unwrap_or_default();
        if apply_or_create(client, namespace, &name, rq, mode, false, "ResourceQuota")
            .await
            .is_ok()
            && mode == ReconcileMode::Create
        {
            tracked.push(ReconcileResource {
                kind: "resourcequota",
                name,
            });
        }
    } else if mode == ReconcileMode::Update {
        let quotas: Api<ResourceQuota> = Api::namespaced(client.clone(), namespace);
        let _ = quotas
            .delete(
                &format!("{}-quota", spec.metadata.name),
                &DeleteParams::default(),
            )
            .await;
    }

    if let Some(lr) = crate::adapters::kube_extras::build_limit_range(namespace, spec) {
        let name = lr.metadata.name.clone().unwrap_or_default();
        if apply_or_create(client, namespace, &name, lr, mode, false, "LimitRange")
            .await
            .is_ok()
            && mode == ReconcileMode::Create
        {
            tracked.push(ReconcileResource {
                kind: "limitrange",
                name,
            });
        }
    } else if mode == ReconcileMode::Update {
        let ranges: Api<LimitRange> = Api::namespaced(client.clone(), namespace);
        let _ = ranges
            .delete(
                &format!("{}-limits", spec.metadata.name),
                &DeleteParams::default(),
            )
            .await;
    }

    // NetworkPolicy
    reconcile_optional(
        client,
        namespace,
        spec,
        mode,
        fail_fast,
        build_networkpolicy_manifest(namespace, spec),
        |s| format!("{}-netpol", s.metadata.name),
        "networkpolicy",
        "NetworkPolicy",
        &mut tracked,
    )
    .await?;

    // Cilium / Calico
    if let Some(cnp) =
        crate::adapters::kube_policy_extras::build_cilium_network_policy_json(namespace, spec)
    {
        reconcile_dynamic_optional(
            client,
            namespace,
            &format!("{}-cilium", spec.metadata.name),
            cnp,
            crate::adapters::kube_policy_extras::cilium_network_policy_api_resource(),
            mode,
            "cilium",
            &mut tracked,
        )
        .await;
    } else if mode == ReconcileMode::Update {
        delete_dynamic_resource(
            client,
            namespace,
            &format!("{}-cilium", spec.metadata.name),
            crate::adapters::kube_policy_extras::cilium_network_policy_api_resource(),
        )
        .await;
    }

    if let Some(calico) =
        crate::adapters::kube_policy_extras::build_calico_network_policy_json(namespace, spec)
    {
        reconcile_dynamic_optional(
            client,
            namespace,
            &format!("{}-calico", spec.metadata.name),
            calico,
            crate::adapters::kube_policy_extras::calico_network_policy_api_resource(),
            mode,
            "calico",
            &mut tracked,
        )
        .await;
    } else if mode == ReconcileMode::Update {
        delete_dynamic_resource(
            client,
            namespace,
            &format!("{}-calico", spec.metadata.name),
            crate::adapters::kube_policy_extras::calico_network_policy_api_resource(),
        )
        .await;
    }

    // PDB
    if let Some(pdb) = crate::adapters::kube_manifest::build_pdb_manifest(namespace, spec) {
        let pdb_name = format!("{}-pdb", spec.metadata.name);
        if apply_or_create(client, namespace, &pdb_name, pdb, mode, false, "PDB")
            .await
            .is_ok()
            && mode == ReconcileMode::Create
        {
            tracked.push(ReconcileResource {
                kind: "pdb",
                name: pdb_name,
            });
        }
    } else if mode == ReconcileMode::Update {
        let pdbs: Api<PodDisruptionBudget> = Api::namespaced(client.clone(), namespace);
        let _ = pdbs
            .delete(
                &format!("{}-pdb", spec.metadata.name),
                &DeleteParams::default(),
            )
            .await;
    }

    // HPA (Deployment/StatefulSet only)
    let hpa_target = match workload_kind {
        K8sWorkloadKind::StatefulSet => "StatefulSet",
        K8sWorkloadKind::Deployment => "Deployment",
        _ => "",
    };
    if !hpa_target.is_empty() {
        reconcile_hpa(client, namespace, spec, hpa_target, mode).await;
    }

    // Gateway API HTTPRoute
    if let Some(gateway) =
        crate::adapters::kube_manifest::build_gateway_provision_json(namespace, spec)
    {
        reconcile_dynamic_optional(
            client,
            namespace,
            &spec
                .kubernetes
                .as_ref()
                .and_then(|k| k.gateway.as_ref())
                .map(|g| g.gateway_name.clone())
                .unwrap_or_else(|| format!("{}-gateway", spec.metadata.name)),
            gateway,
            kube::discovery::ApiResource {
                group: "gateway.networking.k8s.io".into(),
                version: "v1".into(),
                api_version: "gateway.networking.k8s.io/v1".into(),
                kind: "Gateway".into(),
                plural: "gateways".into(),
            },
            mode,
            "gateway",
            &mut tracked,
        )
        .await;
    }

    if let Some(route) =
        crate::adapters::kube_manifest::build_gateway_http_route_json(namespace, spec)
    {
        reconcile_dynamic_optional(
            client,
            namespace,
            &format!("{}-route", spec.metadata.name),
            route,
            http_route_api_resource(),
            mode,
            "httproute",
            &mut tracked,
        )
        .await;
    } else if mode == ReconcileMode::Update {
        delete_dynamic_resource(
            client,
            namespace,
            &format!("{}-route", spec.metadata.name),
            http_route_api_resource(),
        )
        .await;
    }

    // VPA
    if let Some(vpa) = crate::adapters::kube_manifest::build_vpa_json(namespace, spec, hpa_target) {
        reconcile_dynamic_optional(
            client,
            namespace,
            &format!("{}-vpa", spec.metadata.name),
            vpa,
            vpa_api_resource(),
            mode,
            "vpa",
            &mut tracked,
        )
        .await;
    } else if mode == ReconcileMode::Update {
        delete_dynamic_resource(
            client,
            namespace,
            &format!("{}-vpa", spec.metadata.name),
            vpa_api_resource(),
        )
        .await;
    }

    // KEDA
    if matches!(
        workload_kind,
        K8sWorkloadKind::Deployment | K8sWorkloadKind::StatefulSet
    ) {
        if let Some(keda) = crate::adapters::kube_manifest::build_keda_json(namespace, spec) {
            reconcile_dynamic_optional(
                client,
                namespace,
                &format!("{}-keda", spec.metadata.name),
                keda,
                keda_api_resource(),
                mode,
                "keda",
                &mut tracked,
            )
            .await;
        } else if mode == ReconcileMode::Update {
            delete_dynamic_resource(
                client,
                namespace,
                &format!("{}-keda", spec.metadata.name),
                keda_api_resource(),
            )
            .await;
        }
    }

    // ServiceMonitor
    if let Some(monitor) = crate::adapters::kube_extras::build_service_monitor_json(namespace, spec)
    {
        reconcile_dynamic_optional(
            client,
            namespace,
            &format!("{}-monitor", spec.metadata.name),
            monitor,
            crate::adapters::kube_extras::service_monitor_api_resource(),
            mode,
            "servicemonitor",
            &mut tracked,
        )
        .await;
    } else if mode == ReconcileMode::Update {
        delete_dynamic_resource(
            client,
            namespace,
            &format!("{}-monitor", spec.metadata.name),
            crate::adapters::kube_extras::service_monitor_api_resource(),
        )
        .await;
    }

    Ok(tracked)
}

async fn reconcile_hpa(
    client: &Client,
    namespace: &str,
    spec: &Workload,
    _target_kind: &str,
    mode: ReconcileMode,
) {
    let hpa_name = format!("{}-hpa", spec.metadata.name);
    let hpas: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), namespace);

    if let Some(hpa) = build_hpa_manifest(namespace, spec) {
        let pp = PatchParams::apply("aether").force();
        match mode {
            ReconcileMode::Update => {
                if kube_with_timeout("HPA patch", hpas.patch(&hpa_name, &pp, &Patch::Apply(hpa)))
                    .await
                    .is_ok()
                {
                    tracing::info!("Reconciled HPA: {}", hpa_name);
                }
            }
            ReconcileMode::Create => {
                match kube_with_timeout("HPA create", hpas.create(&PostParams::default(), &hpa))
                    .await
                {
                    Ok(_) => tracing::info!("Created HPA: {}", hpa_name),
                    Err(e)
                        if e.downcast_ref::<kube::Error>()
                            .is_some_and(is_already_exists) =>
                    {
                        tracing::info!("HPA already exists: {}", hpa_name);
                    }
                    Err(e) => tracing::warn!("HPA creation failed: {}", e),
                }
            }
        }
    } else if mode == ReconcileMode::Update {
        if let Err(e) = hpas.delete(&hpa_name, &DeleteParams::default()).await {
            tracing::debug!("HPA deletion failed (may not exist): {}", e);
        } else {
            tracing::info!("Deleted HPA: {}", hpa_name);
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn reconcile_optional<T>(
    client: &Client,
    namespace: &str,
    spec: &Workload,
    mode: ReconcileMode,
    fail_fast: bool,
    manifest: Option<T>,
    name_for_spec: fn(&Workload) -> String,
    track_kind: &'static str,
    log_kind: &str,
    tracked: &mut Vec<ReconcileResource>,
) -> crate::Result<()>
where
    T: kube::Resource<Scope = NamespaceResourceScope, DynamicType = ()>
        + Clone
        + serde::Serialize
        + serde::de::DeserializeOwned
        + std::fmt::Debug,
{
    let name = name_for_spec(spec);
    if let Some(resource) = manifest {
        apply_or_create(
            client, namespace, &name, resource, mode, fail_fast, log_kind,
        )
        .await?;
        if mode == ReconcileMode::Create {
            tracked.push(ReconcileResource {
                kind: track_kind,
                name,
            });
        }
    } else if mode == ReconcileMode::Update {
        let api: Api<T> = Api::namespaced(client.clone(), namespace);
        if let Err(e) = api.delete(&name, &DeleteParams::default()).await {
            tracing::debug!("{} deletion failed (may not exist): {}", log_kind, e);
        }
    }
    Ok(())
}

async fn apply_or_create<T>(
    client: &Client,
    namespace: &str,
    name: &str,
    resource: T,
    mode: ReconcileMode,
    fail_fast: bool,
    log_kind: &str,
) -> crate::Result<()>
where
    T: kube::Resource<Scope = NamespaceResourceScope, DynamicType = ()>
        + Clone
        + serde::Serialize
        + serde::de::DeserializeOwned
        + std::fmt::Debug,
{
    let api: Api<T> = Api::namespaced(client.clone(), namespace);
    match mode {
        ReconcileMode::Update => {
            let pp = PatchParams::apply("aether").force();
            match kube_with_timeout(
                &format!("{log_kind} patch"),
                api.patch(name, &pp, &Patch::Apply(resource)),
            )
            .await
            {
                Ok(_) => tracing::info!("Reconciled {log_kind}: {name}"),
                Err(e) => {
                    if fail_fast {
                        return Err(e);
                    }
                    tracing::warn!("{log_kind} reconcile failed: {e}");
                }
            }
        }
        ReconcileMode::Create => match kube_with_timeout(
            &format!("{log_kind} create"),
            api.create(&PostParams::default(), &resource),
        )
        .await
        {
            Ok(_) => tracing::info!("Created {log_kind}: {name}"),
            Err(e)
                if e.downcast_ref::<kube::Error>()
                    .is_some_and(is_already_exists) =>
            {
                tracing::info!("{log_kind} already exists: {name}");
            }
            Err(e) => {
                if fail_fast {
                    return Err(e);
                }
                tracing::warn!("{log_kind} creation failed: {e}");
            }
        },
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn reconcile_dynamic_optional(
    client: &Client,
    namespace: &str,
    name: &str,
    manifest: serde_json::Value,
    api_resource: kube::discovery::ApiResource,
    mode: ReconcileMode,
    track_kind: &'static str,
    tracked: &mut Vec<ReconcileResource>,
) {
    let ok = match mode {
        ReconcileMode::Create => {
            create_dynamic_resource(client, namespace, manifest, api_resource.clone())
                .await
                .is_ok()
        }
        ReconcileMode::Update => {
            reconcile_dynamic_resource(client, namespace, manifest, api_resource.clone())
                .await
                .is_ok()
        }
    };
    if ok && mode == ReconcileMode::Create {
        tracked.push(ReconcileResource {
            kind: track_kind,
            name: name.to_string(),
        });
    }
}

/// RBAC / registry secret names for delete helpers.
pub fn managed_rbac_names(spec: &Workload) -> (Option<String>, Option<String>, Option<String>) {
    let sa_spec = spec
        .kubernetes
        .as_ref()
        .and_then(|k| k.service_account.as_ref());
    let Some(sa_spec) = sa_spec else {
        return (None, None, None);
    };
    if !sa_spec.create {
        return (None, None, None);
    }
    let sa_name = sa_spec
        .name
        .clone()
        .unwrap_or_else(|| spec.metadata.name.clone());
    let role_name = if sa_spec.rules.is_empty() {
        None
    } else {
        Some(format!("{sa_name}-role"))
    };
    let binding_name = role_name.as_ref().map(|_| format!("{sa_name}-rolebinding"));
    (Some(sa_name), role_name, binding_name)
}

pub fn managed_registry_secret_name(spec: &Workload) -> Option<String> {
    spec.kubernetes
        .as_ref()
        .and_then(|k| k.docker_registry_secret.as_ref())
        .map(|r| r.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::{K8sRbacRuleSpec, K8sServiceAccountSpec, KubernetesSpec};

    fn test_spec() -> Workload {
        serde_yaml::from_str(
            r#"
apiVersion: aether/v1
kind: Workload
metadata:
  name: myapp
  owner: test
  project: test
build:
  context: .
  dockerfile: Dockerfile
  registry: ghcr.io/test
requirements:
  cpu: "1"
  memory: 512Mi
  storage: 1Gi
runtime:
  preferred: kube
  allow: [kube]
"#,
        )
        .expect("test workload yaml")
    }

    #[test]
    fn test_managed_rbac_names_with_rules() {
        let mut spec = test_spec();
        spec.kubernetes = Some(KubernetesSpec {
            service_account: Some(K8sServiceAccountSpec {
                create: true,
                name: Some("custom-sa".into()),
                rules: vec![K8sRbacRuleSpec {
                    api_groups: vec!["apps".into()],
                    resources: vec!["deployments".into()],
                    verbs: vec!["get".into()],
                }],
                annotations: std::collections::HashMap::new(),
            }),
            ..Default::default()
        });
        let (sa, role, binding) = managed_rbac_names(&spec);
        assert_eq!(sa.as_deref(), Some("custom-sa"));
        assert_eq!(role.as_deref(), Some("custom-sa-role"));
        assert_eq!(binding.as_deref(), Some("custom-sa-rolebinding"));
    }

    #[test]
    fn test_managed_rbac_names_no_create() {
        let mut spec = test_spec();
        spec.kubernetes = Some(KubernetesSpec {
            service_account: Some(K8sServiceAccountSpec {
                create: false,
                name: None,
                rules: vec![],
                annotations: std::collections::HashMap::new(),
            }),
            ..Default::default()
        });
        assert_eq!(managed_rbac_names(&spec), (None, None, None));
    }
}
