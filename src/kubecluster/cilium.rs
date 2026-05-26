//! Cilium CNI detection and Aether-managed bootstrap policy inventory.

use anyhow::{Context, Result};
use kube::api::{Api, ListParams};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::core::DynamicObject;
use kube::{Client, Config, Resource, ResourceExt};
use serde::Serialize;
use tokio::process::Command;

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
    let cluster = resolve_cluster(cluster).await?;
    let namespace = namespace
        .filter(|n| !n.is_empty() && *n != "all")
        .unwrap_or(DEFAULT_AETHER_NAMESPACE)
        .to_string();

    let crd_cnp = kubectl_crd_exists(&cluster, "ciliumnetworkpolicies.cilium.io").await?;
    let crd_ccnp = kubectl_crd_exists(&cluster, "ciliumclusterwidenetworkpolicies.cilium.io").await?;

    let cni = if crd_cnp || crd_ccnp {
        if cilium_daemonset_ready(&cluster).await? {
            "cilium".to_string()
        } else {
            "cilium".to_string()
        }
    } else {
        "other".to_string()
    };

    let mut managed_policies = Vec::new();
    for name in MANAGED_CNP_NAMES {
        let exists = if crd_cnp {
            kubectl_namespaced_resource_exists(&cluster, "cnp", &namespace, name).await?
        } else {
            false
        };
        managed_policies.push(ManagedPolicyStatus {
            name: (*name).to_string(),
            scope: "namespace".to_string(),
            namespace: Some(namespace.clone()),
            exists,
            aether_managed: true,
        });
    }
    for name in MANAGED_CCNP_NAMES {
        let exists = if crd_ccnp {
            kubectl_cluster_resource_exists(&cluster, "ccnp", name).await?
        } else {
            false
        };
        managed_policies.push(ManagedPolicyStatus {
            name: (*name).to_string(),
            scope: "cluster".to_string(),
            namespace: None,
            exists,
            aether_managed: true,
        });
    }

    let egress_mode = infer_egress_mode(&managed_policies);
    let metrics_server = metrics_server_available(&cluster).await?;
    let cilium_daemonset_ready = cilium_daemonset_ready(&cluster).await.unwrap_or(false);
    let connectivity_check = read_connectivity_check(&cluster, &namespace).await;

    Ok(CiliumStatusResponse {
        cluster,
        namespace,
        cni: if crd_cnp || crd_ccnp { cni } else { "unknown".to_string() },
        crds: CiliumCrdsStatus {
            ciliumnetworkpolicies: crd_cnp,
            ciliumclusterwidenetworkpolicies: crd_ccnp,
        },
        cilium_daemonset_ready,
        managed_policies,
        egress_mode,
        metrics_server,
        connectivity_check,
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

async fn resolve_cluster(cluster: Option<&str>) -> Result<String> {
    if let Some(c) = cluster.filter(|s| !s.is_empty()) {
        return Ok(c.to_string());
    }
    let clusters = super::list_clusters().await?;
    clusters
        .into_iter()
        .find(|c| c.reachable)
        .map(|c| c.name)
        .or_else(|| {
            Kubeconfig::read()
                .ok()
                .and_then(|kc| kc.current_context)
        })
        .context("no Kubernetes cluster context available")
}

async fn kubectl_crd_exists(cluster: &str, crd_name: &str) -> Result<bool> {
    if which::which("kubectl").is_err() {
        return Ok(false);
    }
    let output = Command::new("kubectl")
        .args(["--context", cluster, "get", "crd", crd_name, "-o", "name"])
        .output()
        .await
        .context("kubectl get crd")?;
    Ok(output.status.success())
}

async fn kubectl_namespaced_resource_exists(
    cluster: &str,
    resource: &str,
    namespace: &str,
    name: &str,
) -> Result<bool> {
    if which::which("kubectl").is_err() {
        return Ok(false);
    }
    let output = Command::new("kubectl")
        .args([
            "--context",
            cluster,
            "-n",
            namespace,
            "get",
            resource,
            name,
            "-o",
            "name",
        ])
        .output()
        .await
        .context("kubectl get namespaced resource")?;
    Ok(output.status.success())
}

async fn kubectl_cluster_resource_exists(cluster: &str, resource: &str, name: &str) -> Result<bool> {
    if which::which("kubectl").is_err() {
        return Ok(false);
    }
    let output = Command::new("kubectl")
        .args(["--context", cluster, "get", resource, name, "-o", "name"])
        .output()
        .await
        .context("kubectl get cluster resource")?;
    Ok(output.status.success())
}

async fn cilium_daemonset_ready(cluster: &str) -> Result<bool> {
    if which::which("kubectl").is_err() {
        return Ok(false);
    }
    let output = Command::new("kubectl")
        .args([
            "--context",
            cluster,
            "-n",
            "kube-system",
            "get",
            "ds",
            "cilium",
            "-o",
            "jsonpath={.status.numberReady}",
        ])
        .output()
        .await
        .context("kubectl get cilium ds")?;
    if !output.status.success() {
        return Ok(false);
    }
    let ready = String::from_utf8_lossy(&output.stdout).trim().parse::<u32>().unwrap_or(0);
    Ok(ready > 0)
}

async fn read_connectivity_check(cluster: &str, namespace: &str) -> String {
    if which::which("kubectl").is_err() {
        return "unknown".to_string();
    }
    let output = Command::new("kubectl")
        .args([
            "--context",
            cluster,
            "-n",
            namespace,
            "get",
            "configmap",
            "aether-cilium-connectivity",
            "-o",
            "jsonpath={.data.status}",
        ])
        .output()
        .await;
    match output {
        Ok(o) if o.status.success() => {
            let status = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if status.is_empty() {
                "unknown".to_string()
            } else {
                status
            }
        }
        _ => "unknown".to_string(),
    }
}

pub async fn metrics_server_available(cluster: &str) -> Result<bool> {
    if which::which("kubectl").is_err() {
        return Ok(false);
    }
    let output = Command::new("kubectl")
        .args(["--context", cluster, "top", "nodes", "--no-headers"])
        .output()
        .await
        .context("kubectl top nodes")?;
    Ok(output.status.success())
}

pub async fn list_cilium_network_policies(
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<super::ClusterResourceSummary>> {
    let client = client_for_context(cluster).await?;
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
    let client = client_for_context(cluster).await?;
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

async fn client_for_context(context: &str) -> Result<Client> {
    let kubeconfig = Kubeconfig::read().context("failed to read kubeconfig")?;
    let config = Config::from_custom_kubeconfig(
        kubeconfig,
        &KubeConfigOptions {
            context: Some(context.to_string()),
            ..Default::default()
        },
    )
    .await
    .with_context(|| format!("failed to create kube client for context {}", context))?;
    Client::try_from(config).context("failed to create kube client")
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
