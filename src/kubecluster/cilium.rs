// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Cilium CNI detection and Aether-managed bootstrap policy inventory.

use anyhow::Result;
use k8s_openapi::api::apps::v1::DaemonSet;
use k8s_openapi::api::core::v1::{ConfigMap, Service};
use k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition;
use kube::api::{Api, ListParams, PostParams};
use kube::core::DynamicObject;
use kube::{Client, Resource, ResourceExt};
use serde::Serialize;
use std::collections::BTreeMap;

use crate::adapters::kube_policy_extras::cilium_network_policy_api_resource;

/// Namespaced bootstrap policies applied by deploy scripts.
pub const MANAGED_CNP_NAMES: &[&str] = &[
    "allow-aether-egress",
    "allow-aether-egress-strict",
    "allow-aether-egress-strict-cluster",
];

/// Cluster-wide bootstrap policies applied by deploy scripts.
pub const MANAGED_CCNP_NAMES: &[&str] = &[
    "aether-control-plane-egress",
    "aether-control-plane-egress-strict",
    "aether-control-plane-egress-strict-cluster",
];

pub const DEFAULT_AETHER_NAMESPACE: &str = "aether-system";

#[derive(Debug, Clone, Serialize)]
pub struct ManagedPolicyStatus {
    pub name: String,
    pub scope: String,
    pub namespace: Option<String>,
    pub exists: bool,
    pub aether_managed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CiliumStatusResponse {
    pub cluster: String,
    pub namespace: String,
    pub cni: String,
    pub crds: CiliumCrdsStatus,
    pub cilium_daemonset_ready: bool,
    pub managed_policies: Vec<ManagedPolicyStatus>,
    pub egress_mode: String,
    pub metrics_server: bool,
    pub connectivity_check: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_checked_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connectivity_detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConnectivityProbeResult {
    pub status: String,
    pub detail: String,
    pub checked_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct HubbleDiscoveryResponse {
    pub url: Option<String>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CiliumCrdsStatus {
    pub ciliumnetworkpolicies: bool,
    pub ciliumclusterwidenetworkpolicies: bool,
}

pub fn cilium_clusterwide_network_policy_api_resource() -> kube::api::ApiResource {
    kube::api::ApiResource {
        group: "cilium.io".into(),
        version: "v2".into(),
        api_version: "cilium.io/v2".into(),
        kind: "CiliumClusterwideNetworkPolicy".into(),
        plural: "ciliumclusterwidenetworkpolicies".into(),
    }
}

pub fn is_aether_managed_policy(name: &str) -> bool {
    MANAGED_CNP_NAMES.contains(&name) || MANAGED_CCNP_NAMES.contains(&name)
}

pub async fn cilium_status(cluster: Option<&str>, namespace: Option<&str>) -> Result<CiliumStatusResponse> {
    let cluster = super::resolve_reachable_cluster(cluster).await?;
    let client = super::client_for_cluster(&cluster).await?;
    let namespace = namespace
        .filter(|n| !n.is_empty() && *n != "all")
        .unwrap_or(DEFAULT_AETHER_NAMESPACE)
        .to_string();

    let crd_cnp = crd_exists(&client, "ciliumnetworkpolicies.cilium.io").await?;
    let crd_ccnp = crd_exists(&client, "ciliumclusterwidenetworkpolicies.cilium.io").await?;

    let cni = if crd_cnp || crd_ccnp {
        "cilium".to_string()
    } else {
        "other".to_string()
    };

    let mut managed_policies = Vec::new();
    for name in MANAGED_CNP_NAMES {
        let exists = if crd_cnp {
            namespaced_policy_exists(&client, &namespace, name).await?
        } else {
            false
        };
        managed_policies.push(ManagedPolicyStatus {
            name: (*name).to_string(),
            scope: "namespace".into(),
            namespace: Some(namespace.clone()),
            exists,
            aether_managed: true,
        });
    }
    for name in MANAGED_CCNP_NAMES {
        let exists = if crd_ccnp {
            clusterwide_policy_exists(&client, name).await?
        } else {
            false
        };
        managed_policies.push(ManagedPolicyStatus {
            name: (*name).to_string(),
            scope: "cluster".into(),
            namespace: None,
            exists,
            aether_managed: true,
        });
    }

    let egress_mode = infer_egress_mode(&managed_policies);
    let metrics_server = metrics_server_available(&cluster).await?;
    let cilium_daemonset_ready = cilium_daemonset_ready(&client).await.unwrap_or(false);
    let connectivity_check = read_connectivity_check(&client, &namespace).await;
    let (last_checked_at, connectivity_detail) = read_connectivity_metadata(&client, &namespace).await;

    Ok(CiliumStatusResponse {
        cluster,
        namespace,
        cni: if crd_cnp || crd_ccnp {
            cni
        } else {
            "unknown".to_string()
        },
        crds: CiliumCrdsStatus {
            ciliumnetworkpolicies: crd_cnp,
            ciliumclusterwidenetworkpolicies: crd_ccnp,
        },
        cilium_daemonset_ready,
        managed_policies,
        egress_mode,
        metrics_server,
        connectivity_check,
        last_checked_at,
        connectivity_detail,
    })
}

fn infer_egress_mode(policies: &[ManagedPolicyStatus]) -> String {
    let exists = |name: &str| policies.iter().any(|p| p.name == name && p.exists);
    if exists("allow-aether-egress") {
        "permissive".to_string()
    } else if exists("allow-aether-egress-strict") || exists("allow-aether-egress-strict-cluster") {
        "strict".to_string()
    } else {
        "unknown".to_string()
    }
}

async fn crd_exists(client: &Client, crd_name: &str) -> Result<bool> {
    let api: Api<CustomResourceDefinition> = Api::all(client.clone());
    match api.get(crd_name).await {
        Ok(_) => Ok(true),
        Err(kube::Error::Api(e)) if e.code == 404 => Ok(false),
        Err(e) => Err(e.into()),
    }
}

async fn namespaced_policy_exists(client: &Client, namespace: &str, name: &str) -> Result<bool> {
    let api_resource = cilium_network_policy_api_resource();
    let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace, &api_resource);
    match api.get(name).await {
        Ok(_) => Ok(true),
        Err(kube::Error::Api(e)) if e.code == 404 || e.code == 403 => Ok(false),
        Err(e) => Err(e.into()),
    }
}

async fn clusterwide_policy_exists(client: &Client, name: &str) -> Result<bool> {
    let api_resource = cilium_clusterwide_network_policy_api_resource();
    let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);
    match api.get(name).await {
        Ok(_) => Ok(true),
        Err(kube::Error::Api(e)) if e.code == 404 || e.code == 403 => Ok(false),
        Err(e) => Err(e.into()),
    }
}

async fn cilium_daemonset_ready(client: &Client) -> Result<bool> {
    let api: Api<DaemonSet> = Api::namespaced(client.clone(), "kube-system");
    match api.get("cilium").await {
        Ok(ds) => {
            let ready = ds.status.map(|s| s.number_ready).unwrap_or(0);
            Ok(ready > 0)
        }
        Err(kube::Error::Api(e)) if e.code == 404 => Ok(false),
        Err(e) => Err(e.into()),
    }
}

async fn read_connectivity_metadata(client: &Client, namespace: &str) -> (Option<String>, Option<String>) {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    match api.get("aether-cilium-connectivity").await {
        Ok(cm) => {
            let data = cm.data.unwrap_or_default();
            (
                data.get("checked_at").cloned(),
                data.get("detail").cloned(),
            )
        }
        Err(kube::Error::Api(e)) if e.code == 404 => (None, None),
        Err(_) => (None, None),
    }
}

/// Run a native Cilium connectivity probe and persist the result to ConfigMap.
pub async fn probe_cilium_connectivity(
    cluster: Option<&str>,
    namespace: Option<&str>,
) -> Result<ConnectivityProbeResult> {
    let cluster = super::resolve_reachable_cluster(cluster).await?;
    let client = super::client_for_cluster(&cluster).await?;
    let namespace = namespace
        .filter(|n| !n.is_empty() && *n != "all")
        .unwrap_or(DEFAULT_AETHER_NAMESPACE)
        .to_string();

    let crd_cnp = crd_exists(&client, "ciliumnetworkpolicies.cilium.io").await?;
    let crd_ccnp = crd_exists(&client, "ciliumclusterwidenetworkpolicies.cilium.io").await?;
    if !crd_cnp && !crd_ccnp {
        let result = ConnectivityProbeResult {
            status: "skipped".into(),
            detail: "Cilium CRDs not installed".into(),
            checked_at: chrono::Utc::now().to_rfc3339(),
        };
        write_connectivity_status(&client, &namespace, &result).await?;
        return Ok(result);
    }

    let ready = cilium_daemonset_ready(&client).await.unwrap_or(false);
    let (status, detail) = if ready {
        ("ok", "cilium daemonset has ready pods".to_string())
    } else {
        ("failed", "cilium daemonset has no ready pods".to_string())
    };

    let result = ConnectivityProbeResult {
        status: status.into(),
        detail,
        checked_at: chrono::Utc::now().to_rfc3339(),
    };
    write_connectivity_status(&client, &namespace, &result).await?;
    Ok(result)
}

async fn write_connectivity_status(
    client: &Client,
    namespace: &str,
    result: &ConnectivityProbeResult,
) -> Result<()> {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    let mut data = BTreeMap::new();
    data.insert("status".into(), result.status.clone());
    data.insert("checked_at".into(), result.checked_at.clone());
    data.insert("detail".into(), result.detail.clone());
    let cm = ConfigMap {
        metadata: kube::api::ObjectMeta {
            name: Some("aether-cilium-connectivity".into()),
            namespace: Some(namespace.to_string()),
            ..Default::default()
        },
        data: Some(data),
        ..Default::default()
    };
    match api.get("aether-cilium-connectivity").await {
        Ok(_) => {
            let pp = kube::api::PatchParams::apply("aether").force();
            api.patch(
                "aether-cilium-connectivity",
                &pp,
                &kube::api::Patch::Apply(cm),
            )
            .await?;
        }
        Err(kube::Error::Api(e)) if e.code == 404 => {
            api.create(&PostParams::default(), &cm).await?;
        }
        Err(e) => return Err(e.into()),
    }
    Ok(())
}

/// Discover Hubble UI URL from env, ConfigMap, or Service.
pub async fn discover_hubble_ui(cluster: Option<&str>) -> Result<HubbleDiscoveryResponse> {
    if let Ok(url) = std::env::var("AETHER_HUBBLE_UI_URL") {
        if !url.is_empty() {
            return Ok(HubbleDiscoveryResponse {
                url: Some(url),
                source: "env".into(),
            });
        }
    }

    let cluster = super::resolve_reachable_cluster(cluster).await?;
    let client = super::client_for_cluster(&cluster).await?;

    let cm_api: Api<ConfigMap> = Api::namespaced(client.clone(), "kube-system");
    for name in ["cilium-config", "hubble-ui-nginx"] {
        if let Ok(cm) = cm_api.get(name).await {
            if let Some(data) = cm.data {
                for key in ["hubble-ui-url", "hubble-ui", "ui-url"] {
                    if let Some(url) = data.get(key).filter(|u| !u.is_empty()) {
                        return Ok(HubbleDiscoveryResponse {
                            url: Some(url.clone()),
                            source: format!("configmap:{name}"),
                        });
                    }
                }
            }
        }
    }

    let svc_api: Api<Service> = Api::namespaced(client.clone(), "kube-system");
    for name in ["hubble-ui", "hubble-ui-nginx"] {
        if let Ok(svc) = svc_api.get(name).await {
            if let Some(spec) = svc.spec {
                if let Some(t) = spec.type_ {
                    if t == "LoadBalancer" {
                        if let Some(ingress) = svc.status.and_then(|s| s.load_balancer).and_then(|lb| lb.ingress) {
                            if let Some(host) = ingress.first().and_then(|i| i.hostname.clone().or(i.ip.clone())) {
                                let port = spec
                                    .ports
                                    .as_ref()
                                    .and_then(|p| p.first())
                                    .map(|p| p.port)
                                    .unwrap_or(80);
                                return Ok(HubbleDiscoveryResponse {
                                    url: Some(format!("http://{host}:{port}")),
                                    source: format!("service:{name}"),
                                });
                            }
                        }
                    }
                    if t == "NodePort" {
                        if let Some(port) = spec.ports.as_ref().and_then(|p| p.first()) {
                            if let Some(np) = port.node_port {
                                return Ok(HubbleDiscoveryResponse {
                                    url: Some(format!("http://localhost:{np}")),
                                    source: format!("service:{name}:nodeport"),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(HubbleDiscoveryResponse {
        url: None,
        source: "none".into(),
    })
}

pub fn hubble_workload_url(base: &str, namespace: &str, pod: &str) -> String {
    let trimmed = base.trim_end_matches('/');
    format!("{trimmed}/?namespace={namespace}&pod={pod}")
}

async fn read_connectivity_check(client: &Client, namespace: &str) -> String {
    let api: Api<ConfigMap> = Api::namespaced(client.clone(), namespace);
    match api.get("aether-cilium-connectivity").await {
        Ok(cm) => cm
            .data
            .and_then(|d| d.get("status").cloned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".to_string()),
        Err(kube::Error::Api(e)) if e.code == 404 => "unknown".to_string(),
        Err(_) => "unknown".to_string(),
    }
}

pub async fn metrics_server_available(cluster: &str) -> Result<bool> {
    let client = super::client_for_cluster(cluster).await?;
    metrics_server_available_client(&client).await
}

async fn metrics_server_available_client(client: &Client) -> Result<bool> {
    let discovery = kube::discovery::Discovery::new(client.clone());
    let resources = discovery.run().await?;
    let has_metrics = resources
        .groups()
        .any(|group| group.name() == "metrics.k8s.io");
    Ok(has_metrics)
}

pub async fn list_cilium_network_policies(
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<super::ClusterResourceSummary>> {
    let client = super::client_for_cluster(cluster).await?;
    let api_resource = cilium_network_policy_api_resource();
    let list = if let Some(ns) = namespace.filter(|n| *n != "all" && *n != "_cluster") {
        let api: Api<DynamicObject> = Api::namespaced_with(client, ns, &api_resource);
        api.list(&ListParams::default()).await?.items
    } else {
        let api: Api<DynamicObject> = Api::all_with(client, &api_resource);
        api.list(&ListParams::default()).await?.items
    };

    Ok(list
        .into_iter()
        .map(|item| {
            let name = item.name_any();
            super::ClusterResourceSummary {
                cluster: cluster.to_string(),
                namespace: item.namespace().unwrap_or_else(|| "default".to_string()),
                kind: "CiliumNetworkPolicy".to_string(),
                name: name.clone(),
                status: if is_aether_managed_policy(&name) {
                    "aether-managed".to_string()
                } else {
                    "active".to_string()
                },
                created_at: item
                    .meta()
                    .creation_timestamp
                    .clone()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
                detail: Some("cilium.io/v2".to_string()),
            }
        })
        .collect())
}

pub async fn list_cilium_clusterwide_network_policies(
    cluster: &str,
) -> Result<Vec<super::ClusterResourceSummary>> {
    let client = super::client_for_cluster(cluster).await?;
    let api_resource = cilium_clusterwide_network_policy_api_resource();
    let api: Api<DynamicObject> = Api::all_with(client, &api_resource);
    let list = api.list(&ListParams::default()).await?.items;

    Ok(list
        .into_iter()
        .map(|item| {
            let name = item.name_any();
            super::ClusterResourceSummary {
                cluster: cluster.to_string(),
                namespace: "_cluster".to_string(),
                kind: "CiliumClusterwideNetworkPolicy".to_string(),
                name: name.clone(),
                status: if is_aether_managed_policy(&name) {
                    "aether-managed".to_string()
                } else {
                    "active".to_string()
                },
                created_at: item
                    .meta()
                    .creation_timestamp
                    .clone()
                    .map(|t| t.0.to_rfc3339())
                    .unwrap_or_default(),
                detail: Some("cilium.io/v2 clusterwide".to_string()),
            }
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hubble_workload_url() {
        let url = super::hubble_workload_url("http://hubble.local", "prod", "api-abc");
        assert_eq!(url, "http://hubble.local/?namespace=prod&pod=api-abc");
    }

    #[test]
    fn test_infer_egress_permissive() {
        let policies = vec![ManagedPolicyStatus {
            name: "allow-aether-egress".into(),
            scope: "namespace".into(),
            namespace: Some("aether-system".into()),
            exists: true,
            aether_managed: true,
        }];
        assert_eq!(infer_egress_mode(&policies), "permissive");
    }

    #[test]
    fn test_infer_egress_strict() {
        let policies = vec![ManagedPolicyStatus {
            name: "allow-aether-egress-strict".into(),
            scope: "namespace".into(),
            namespace: Some("aether-system".into()),
            exists: true,
            aether_managed: true,
        }];
        assert_eq!(infer_egress_mode(&policies), "strict");
    }

    #[test]
    fn test_is_aether_managed_policy() {
        assert!(is_aether_managed_policy("allow-aether-egress"));
        assert!(is_aether_managed_policy("aether-control-plane-egress"));
        assert!(!is_aether_managed_policy("custom-policy"));
    }

    #[test]
    fn test_in_cluster_label_matches_default() {
        assert!(super::super::is_in_cluster_label("active-client"));
        assert!(super::super::is_in_cluster_label(
            &super::super::default_in_cluster_label()
        ));
    }
}
