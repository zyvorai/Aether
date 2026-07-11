// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Native Kubernetes cluster inventory and workload discovery for Aether.

pub mod cilium;

use anyhow::{Context, Result};
use chrono::Utc;
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, ReplicaSet, StatefulSet};
use k8s_openapi::api::autoscaling::v2::HorizontalPodAutoscaler;
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{
    ConfigMap, Endpoints, Event as KubeEvent, LimitRange, Namespace, Node, PersistentVolume,
    PersistentVolumeClaim, Pod, ResourceQuota, Secret, Service, ServiceAccount,
};
use k8s_openapi::api::discovery::v1::EndpointSlice;
use k8s_openapi::api::networking::v1::{Ingress, NetworkPolicy};
use k8s_openapi::api::storage::v1::StorageClass;
use kube::api::{Api, DeleteParams, ListParams, LogParams, Patch, PatchParams, PostParams};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::core::DynamicObject;
use kube::discovery::ApiResource;
use kube::{Client, Config, Resource, ResourceExt};
use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct ClusterSummaryResponse {
    pub enabled: bool,
    pub connected: bool,
    pub backend: String,
    pub cluster_count: usize,
    pub healthy_clusters: usize,
    pub workload_count: usize,
    pub clusters: Vec<ClusterInfo>,
    pub error: Option<String>,
    /// Present when the summary was derived from the active default client because
    /// multi-context kubeconfig inventory was unavailable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary_note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterInfo {
    pub name: String,
    pub server: Option<String>,
    pub version: Option<String>,
    pub reachable: bool,
}

#[derive(Debug, Clone)]
pub struct ClusterWorkload {
    pub cluster: String,
    pub namespace: String,
    pub kind: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct ClusterLogsRequest {
    pub cluster: String,
    pub namespace: String,
    pub kind: String,
    pub name: String,
    pub api_version: Option<String>,
    pub plural: Option<String>,
    pub namespaced: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ClusterActionRequest {
    pub cluster: String,
    pub namespace: String,
    pub kind: String,
    pub name: String,
    pub action: String,
    pub replicas: Option<i32>,
    pub api_version: Option<String>,
    pub plural: Option<String>,
    pub namespaced: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct ClusterBrowseRequest {
    pub cluster: String,
    pub namespace: Option<String>,
    pub kind: String,
    pub api_version: Option<String>,
    pub plural: Option<String>,
    pub namespaced: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterNamespaceSummary {
    pub name: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterResourceSummary {
    pub cluster: String,
    pub namespace: String,
    pub kind: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterConditionSummary {
    pub type_: String,
    pub status: String,
    pub reason: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterPodSummary {
    pub name: String,
    pub phase: String,
    pub ready: usize,
    pub total_containers: usize,
    pub restarts: i32,
    pub node: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterOwnerReference {
    pub api_version: String,
    pub kind: String,
    pub name: String,
    pub uid: Option<String>,
    pub controller: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterOwnedResource {
    pub kind: String,
    pub name: String,
    pub api_version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterResourceDetail {
    pub cluster: String,
    pub namespace: String,
    pub kind: String,
    pub name: String,
    pub api_version: Option<String>,
    pub uid: Option<String>,
    pub pods: Vec<ClusterPodSummary>,
    pub conditions: Vec<ClusterConditionSummary>,
    pub owner_references: Vec<ClusterOwnerReference>,
    pub owned_resources: Vec<ClusterOwnedResource>,
    pub manifest: serde_json::Value,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterHealthSummary {
    pub level: String,
    pub summary: String,
    pub ready_pods: usize,
    pub total_pods: usize,
    pub warning_events: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterRelatedEvent {
    pub type_: String,
    pub reason: String,
    pub message: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterTopMetric {
    pub name: String,
    pub cpu: String,
    pub memory: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClusterMetricsSummary {
    pub scope: String,
    pub pod_count: usize,
    pub total_cpu_millicores: i64,
    pub total_memory_mib: i64,
    pub pods: Vec<ClusterTopMetric>,
}

#[derive(Debug, Clone, Serialize)]
pub struct HelmRevisionEntry {
    pub revision: String,
    pub updated: String,
    pub status: String,
    pub chart: String,
    pub app_version: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
struct HelmListEntry {
    name: String,
    namespace: String,
    revision: String,
    updated: String,
    status: String,
    chart: String,
    #[serde(default)]
    app_version: String,
}

#[derive(Debug, serde::Deserialize)]
struct HelmHistoryEntry {
    revision: i64,
    updated: String,
    status: String,
    chart: String,
    #[serde(default)]
    app_version: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

pub async fn cluster_summary() -> ClusterSummaryResponse {
    match list_clusters().await {
        Ok(clusters) => {
            let workloads = list_workloads().await.unwrap_or_default();
            let healthy_clusters = clusters.iter().filter(|c| c.reachable).count();
            ClusterSummaryResponse {
                enabled: true,
                connected: healthy_clusters > 0 || !clusters.is_empty(),
                backend: "Aether Kubernetes".to_string(),
                cluster_count: clusters.len(),
                healthy_clusters,
                workload_count: workloads.len(),
                clusters,
                error: None,
                summary_note: None,
            }
        }
        Err(error) => {
            if let Some(fallback) = cluster_summary_from_default_client().await {
                ClusterSummaryResponse {
                    summary_note: None,
                    ..fallback
                }
            } else {
                ClusterSummaryResponse {
                    enabled: false,
                    connected: false,
                    backend: "Aether Kubernetes".to_string(),
                    cluster_count: 0,
                    healthy_clusters: 0,
                    workload_count: 0,
                    clusters: Vec::new(),
                    error: Some(error.to_string()),
                    summary_note: None,
                }
            }
        }
    }
}

/// When multi-context inventory fails but the default client works, return the inventory error
/// for the Platform setup recommendations page (not inline dashboard banners).
pub async fn kubeconfig_inventory_fallback_error() -> Option<String> {
    match list_clusters().await {
        Ok(_) => None,
        Err(e) => {
            if cluster_summary_from_default_client().await.is_some() {
                Some(e.to_string())
            } else {
                None
            }
        }
    }
}

/// When `Kubeconfig::read()` fails (missing file, permissions, etc.) but `Client::try_default()`
/// succeeds — align cluster summary with `/api/workloads` live discovery.
async fn cluster_summary_from_default_client() -> Option<ClusterSummaryResponse> {
    let client = Client::try_default().await.ok()?;
    let version = client
        .apiserver_version()
        .await
        .ok()
        .map(|v| v.git_version)?;
    let cfg = Config::infer().await.ok()?;
    let context_label = std::env::var("AETHER_CLUSTER_DISPLAY_NAME")
        .unwrap_or_else(|_| "active-client".to_string());
    let server = Some(cfg.cluster_url.to_string());
    let workload_count = list_workloads()
        .await
        .map(|workloads| workloads.len())
        .unwrap_or(0);
    Some(ClusterSummaryResponse {
        enabled: true,
        connected: true,
        backend: "Aether Kubernetes (active client)".to_string(),
        cluster_count: 1,
        healthy_clusters: 1,
        workload_count,
        clusters: vec![ClusterInfo {
            name: context_label,
            server,
            version: Some(version),
            reachable: true,
        }],
        error: None,
        summary_note: None,
    })
}

fn default_cluster_display_name() -> String {
    std::env::var("AETHER_CLUSTER_DISPLAY_NAME").unwrap_or_else(|_| "active-client".to_string())
}

async fn list_workloads_for_client(client: &Client, cluster: &str) -> Result<Vec<ClusterWorkload>> {
    let mut workloads = Vec::new();
    workloads.extend(
        list_kind::<Deployment>(
            client,
            cluster,
            "Deployment",
            workload_status_deployment,
            "/spec/template/spec/containers/0/image",
        )
        .await?,
    );
    workloads.extend(
        list_kind::<StatefulSet>(
            client,
            cluster,
            "StatefulSet",
            workload_status_statefulset,
            "/spec/template/spec/containers/0/image",
        )
        .await?,
    );
    workloads.extend(
        list_kind::<DaemonSet>(
            client,
            cluster,
            "DaemonSet",
            workload_status_daemonset,
            "/spec/template/spec/containers/0/image",
        )
        .await?,
    );
    workloads.extend(
        list_kind::<Job>(
            client,
            cluster,
            "Job",
            workload_status_job,
            "/spec/template/spec/containers/0/image",
        )
        .await?,
    );
    workloads.extend(
        list_kind::<CronJob>(
            client,
            cluster,
            "CronJob",
            workload_status_cronjob,
            "/spec/jobTemplate/spec/template/spec/containers/0/image",
        )
        .await?,
    );
    workloads.extend(list_standalone_pod_workloads(client, cluster).await?);
    workloads.extend(
        list_kubevirt_vmis(client, cluster)
            .await
            .unwrap_or_default(),
    );
    Ok(workloads)
}

fn pod_is_inventory_workload(pod: &Pod) -> bool {
    match pod.metadata.owner_references.as_ref() {
        None => true,
        Some(refs) if refs.is_empty() => true,
        Some(refs) => !refs.iter().any(|owner| {
            matches!(
                owner.kind.as_str(),
                "ReplicaSet" | "Job" | "DaemonSet" | "StatefulSet" | "Node"
            )
        }),
    }
}

fn workload_status_pod(pod: &Pod) -> String {
    pod.status
        .as_ref()
        .and_then(|status| status.phase.as_ref())
        .map(|phase| match phase.as_str() {
            "Running" => "running",
            "Succeeded" => "stopped",
            "Failed" => "failed",
            "Pending" => "pending",
            _ => "unknown",
        })
        .unwrap_or("unknown")
        .to_string()
}

async fn list_standalone_pod_workloads(
    client: &Client,
    cluster: &str,
) -> Result<Vec<ClusterWorkload>> {
    let api: Api<Pod> = Api::all(client.clone());
    let list = api.list(&ListParams::default()).await?;
    let mut results = Vec::new();

    for pod in list.items {
        if !pod_is_inventory_workload(&pod) {
            continue;
        }
        let name = pod.metadata.name.clone().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let namespace = pod
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let created_at = pod
            .metadata
            .creation_timestamp
            .as_ref()
            .map(|t| t.0.to_rfc3339())
            .unwrap_or_default();
        let value = serde_json::to_value(&pod)?;
        let image = value
            .pointer("/spec/containers/0/image")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        results.push(ClusterWorkload {
            cluster: cluster.to_string(),
            namespace,
            kind: "Pod".to_string(),
            name,
            image,
            status: workload_status_pod(&pod),
            created_at,
        });
    }

    Ok(results)
}

async fn list_workloads_from_default_client() -> Result<Vec<ClusterWorkload>> {
    let client = Client::try_default()
        .await
        .context("no default kubernetes client")?;
    list_workloads_for_client(&client, &default_cluster_display_name()).await
}

fn kubevirt_vmi_api_resource() -> ApiResource {
    ApiResource {
        group: "kubevirt.io".to_string(),
        version: "v1".to_string(),
        api_version: "kubevirt.io/v1".to_string(),
        kind: "VirtualMachineInstance".to_string(),
        plural: "virtualmachineinstances".to_string(),
    }
}

async fn list_kubevirt_vmis(client: &Client, cluster: &str) -> Result<Vec<ClusterWorkload>> {
    let vms: Api<DynamicObject> = Api::all_with(client.clone(), &kubevirt_vmi_api_resource());
    let list = vms.list(&ListParams::default()).await?;
    let mut results = Vec::new();

    for vm in list.items {
        let name = vm.metadata.name.clone().unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let namespace = vm
            .metadata
            .namespace
            .clone()
            .unwrap_or_else(|| "default".to_string());
        let phase = vm
            .data
            .get("status")
            .and_then(|s| s.get("phase"))
            .and_then(|p| p.as_str())
            .unwrap_or("Unknown");
        let status = match phase {
            "Running" => "running",
            "Succeeded" => "stopped",
            "Failed" => "failed",
            "Scheduling" | "Scheduled" | "Pending" => "pending",
            _ => "unknown",
        }
        .to_string();
        let created_at = vm
            .metadata
            .creation_timestamp
            .as_ref()
            .map(|t| t.0.to_rfc3339())
            .unwrap_or_default();

        results.push(ClusterWorkload {
            cluster: cluster.to_string(),
            namespace,
            kind: "VirtualMachineInstance".to_string(),
            name: name.clone(),
            image: format!("vm:{name}"),
            status,
            created_at,
        });
    }

    Ok(results)
}

pub async fn list_clusters() -> Result<Vec<ClusterInfo>> {
    let kubeconfig = Kubeconfig::read().context("failed to read kubeconfig")?;
    let mut contexts: Vec<String> = kubeconfig
        .contexts
        .iter()
        .map(|ctx| ctx.name.clone())
        .collect();

    if contexts.is_empty() {
        if let Some(current) = kubeconfig.current_context.clone() {
            contexts.push(current);
        }
    }

    contexts.sort();
    contexts.dedup();

    let mut clusters = Vec::new();
    for context_name in contexts {
        let server = kubeconfig
            .contexts
            .iter()
            .find(|ctx| ctx.name == context_name)
            .and_then(|ctx| ctx.context.as_ref())
            .map(|context| context.cluster.clone())
            .and_then(|cluster_name| {
                kubeconfig
                    .clusters
                    .iter()
                    .find(|cluster| cluster.name == cluster_name)
                    .and_then(|cluster| cluster.cluster.as_ref())
                    .and_then(|cluster| cluster.server.clone())
            });

        match client_for_context(&context_name).await {
            Ok(client) => {
                let version = client
                    .apiserver_version()
                    .await
                    .ok()
                    .map(|version| version.git_version);
                clusters.push(ClusterInfo {
                    name: context_name,
                    server,
                    version: version.clone(),
                    reachable: version.is_some(),
                });
            }
            Err(_) => {
                clusters.push(ClusterInfo {
                    name: context_name,
                    server,
                    version: None,
                    reachable: false,
                });
            }
        }
    }

    Ok(clusters)
}

pub async fn list_workloads() -> Result<Vec<ClusterWorkload>> {
    let clusters = match list_clusters().await {
        Ok(clusters) if !clusters.is_empty() => clusters,
        Ok(_) => {
            return list_workloads_from_default_client().await;
        }
        Err(_) => {
            return list_workloads_from_default_client().await;
        }
    };

    let mut workloads = Vec::new();

    for cluster in clusters {
        if !cluster.reachable {
            continue;
        }

        let client = match client_for_context(&cluster.name).await {
            Ok(client) => client,
            Err(_) => continue,
        };

        workloads.extend(list_workloads_for_client(&client, &cluster.name).await?);
    }

    if workloads.is_empty() {
        return list_workloads_from_default_client().await;
    }

    workloads.sort_by(|a, b| {
        (&a.cluster, &a.namespace, &a.kind, &a.name).cmp(&(
            &b.cluster,
            &b.namespace,
            &b.kind,
            &b.name,
        ))
    });
    Ok(workloads)
}

pub async fn workload_logs(req: &ClusterLogsRequest) -> Result<String> {
    let client = client_for_context(&req.cluster).await?;
    let pods: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
    let list = if req.kind == "Pod" {
        let pod = pods.get(&req.name).await?;
        kube::api::ObjectList {
            metadata: Default::default(),
            types: Default::default(),
            items: vec![pod],
        }
    } else {
        let selector = selector_for_workload(&client, req).await?;
        pods_for_selector(&pods, &selector).await?
    };

    let pod = list
        .items
        .iter()
        .find(|pod| {
            pod.status
                .as_ref()
                .and_then(|status| status.phase.as_deref())
                .map(|phase| phase == "Running")
                .unwrap_or(false)
        })
        .or_else(|| list.items.first())
        .context("no pods found for workload")?;

    let pod_name = pod
        .metadata
        .name
        .clone()
        .context("pod missing metadata.name")?;

    pods.logs(
        &pod_name,
        &LogParams {
            follow: false,
            tail_lines: Some(200),
            ..Default::default()
        },
    )
    .await
    .with_context(|| format!("failed to get logs for pod {}", pod_name))
}

pub async fn workload_detail(req: &ClusterLogsRequest) -> Result<ClusterResourceDetail> {
    let client = client_for_context(&req.cluster).await?;
    let mut manifest = manifest_for_workload(&client, req).await?;
    if req.kind == "Secret" {
        sanitize_secret_manifest(&mut manifest);
    }
    let selector = selector_for_value(&manifest);
    let conditions = manifest
        .pointer("/status/conditions")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .map(|item| ClusterConditionSummary {
                    type_: item
                        .get("type")
                        .and_then(|value| value.as_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    status: item
                        .get("status")
                        .and_then(|value| value.as_str())
                        .unwrap_or("Unknown")
                        .to_string(),
                    reason: item
                        .get("reason")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                    message: item
                        .get("message")
                        .and_then(|value| value.as_str())
                        .map(str::to_string),
                })
                .collect()
        })
        .unwrap_or_default();

    let pods = if req.kind == "Pod" {
        let pod_api: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
        vec![summarize_pod(&pod_api.get(&req.name).await?)]
    } else if let Some(selector) = selector {
        let pods_api: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
        let pod_list = pods_for_selector(&pods_api, &selector).await?;
        pod_list.items.iter().map(summarize_pod).collect()
    } else {
        Vec::new()
    };

    let uid = manifest
        .pointer("/metadata/uid")
        .and_then(|value| value.as_str())
        .map(str::to_string);
    let owner_references = owner_references_from_manifest(&manifest);
    let owned_resources = if req.namespace != "all" {
        list_owned_resources(
            &client,
            &req.namespace,
            uid.as_deref(),
            &req.kind,
            &req.name,
        )
        .await
        .unwrap_or_default()
    } else {
        Vec::new()
    };

    Ok(ClusterResourceDetail {
        cluster: req.cluster.clone(),
        namespace: req.namespace.clone(),
        kind: req.kind.clone(),
        name: req.name.clone(),
        api_version: manifest
            .get("apiVersion")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        uid,
        pods,
        conditions,
        owner_references,
        owned_resources,
        manifest,
    })
}

fn owner_references_from_manifest(manifest: &serde_json::Value) -> Vec<ClusterOwnerReference> {
    manifest
        .pointer("/metadata/ownerReferences")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|item| {
                    Some(ClusterOwnerReference {
                        api_version: item.get("apiVersion")?.as_str()?.to_string(),
                        kind: item.get("kind")?.as_str()?.to_string(),
                        name: item.get("name")?.as_str()?.to_string(),
                        uid: item
                            .get("uid")
                            .and_then(|value| value.as_str())
                            .map(str::to_string),
                        controller: item
                            .get("controller")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn owner_matches(
    owner: &k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference,
    owner_uid: Option<&str>,
    owner_kind: &str,
    owner_name: &str,
) -> bool {
    if let Some(uid) = owner_uid {
        owner.uid == uid
    } else {
        owner.kind == owner_kind && owner.name == owner_name
    }
}

async fn list_owned_resources(
    client: &Client,
    namespace: &str,
    owner_uid: Option<&str>,
    owner_kind: &str,
    owner_name: &str,
) -> Result<Vec<ClusterOwnedResource>> {
    let mut owned = Vec::new();

    let pod_api: Api<Pod> = Api::namespaced(client.clone(), namespace);
    if let Ok(list) = pod_api.list(&ListParams::default()).await {
        for pod in list.items {
            let owners = pod.metadata.owner_references.as_deref().unwrap_or(&[]);
            if owners
                .iter()
                .any(|owner| owner_matches(owner, owner_uid, owner_kind, owner_name))
            {
                owned.push(ClusterOwnedResource {
                    kind: "Pod".to_string(),
                    name: pod.metadata.name.clone().unwrap_or_default(),
                    api_version: Some("v1".to_string()),
                });
            }
        }
    }

    let rs_api: Api<ReplicaSet> = Api::namespaced(client.clone(), namespace);
    if let Ok(list) = rs_api.list(&ListParams::default()).await {
        for rs in list.items {
            let owners = rs.metadata.owner_references.as_deref().unwrap_or(&[]);
            if owners
                .iter()
                .any(|owner| owner_matches(owner, owner_uid, owner_kind, owner_name))
            {
                owned.push(ClusterOwnedResource {
                    kind: "ReplicaSet".to_string(),
                    name: rs.metadata.name.clone().unwrap_or_default(),
                    api_version: Some("apps/v1".to_string()),
                });
            }
        }
    }

    let svc_api: Api<Service> = Api::namespaced(client.clone(), namespace);
    if let Ok(list) = svc_api.list(&ListParams::default()).await {
        for svc in list.items {
            let owners = svc.metadata.owner_references.as_deref().unwrap_or(&[]);
            if owners
                .iter()
                .any(|owner| owner_matches(owner, owner_uid, owner_kind, owner_name))
            {
                owned.push(ClusterOwnedResource {
                    kind: "Service".to_string(),
                    name: svc.metadata.name.clone().unwrap_or_default(),
                    api_version: Some("v1".to_string()),
                });
            }
        }
    }

    owned.sort_by(|a, b| (&a.kind, &a.name).cmp(&(&b.kind, &b.name)));
    Ok(owned)
}

pub async fn health_summary(req: &ClusterLogsRequest) -> Result<ClusterHealthSummary> {
    let detail = workload_detail(req).await?;
    let events = related_events(&req.cluster, &req.namespace, &req.kind, &req.name)
        .await
        .unwrap_or_default();
    let ready_pods = detail
        .pods
        .iter()
        .filter(|pod| pod.ready == pod.total_containers && pod.total_containers > 0)
        .count();
    let total_pods = detail.pods.len();
    let warning_events = events
        .iter()
        .filter(|event| event.type_ == "Warning")
        .count();
    let level = if total_pods == 0 {
        if warning_events > 0 {
            "warning"
        } else {
            "unknown"
        }
    } else if ready_pods == total_pods && warning_events == 0 {
        "healthy"
    } else if ready_pods > 0 {
        "degraded"
    } else {
        "failing"
    };
    let summary = match level {
        "healthy" => format!("{ready_pods}/{total_pods} pods ready"),
        "degraded" => {
            format!("{ready_pods}/{total_pods} pods ready with {warning_events} warnings")
        }
        "failing" => format!("0/{total_pods} pods ready with {warning_events} warnings"),
        "warning" => format!("no pods but {warning_events} warning events"),
        _ => "health unavailable".to_string(),
    };
    Ok(ClusterHealthSummary {
        level: level.to_string(),
        summary,
        ready_pods,
        total_pods,
        warning_events,
    })
}

pub async fn workload_action(req: &ClusterActionRequest) -> Result<String> {
    let client = client_for_context(&req.cluster).await?;

    match req.action.as_str() {
        "delete" => delete_workload(&client, req).await?,
        "restart" => restart_workload(&client, req).await?,
        "start" => start_workload(&client, req).await?,
        "stop" => stop_workload(&client, req).await?,
        "suspend" => suspend_workload(&client, req).await?,
        "resume" => resume_workload(&client, req).await?,
        "scale" => scale_workload(&client, req).await?,
        "cordon" => cordon_node(&req.cluster, &req.name, true).await?,
        "uncordon" => cordon_node(&req.cluster, &req.name, false).await?,
        "drain" => drain_node(&req.cluster, &req.name).await?,
        other => anyhow::bail!("unsupported Kubernetes action: {}", other),
    }

    Ok(format!(
        "{} {} {}/{} on cluster {}",
        req.action, req.kind, req.namespace, req.name, req.cluster
    ))
}

pub async fn related_events(
    cluster: &str,
    namespace: &str,
    kind: &str,
    name: &str,
) -> Result<Vec<ClusterRelatedEvent>> {
    if kind == "HelmRelease" {
        return Ok(Vec::new());
    }

    let client = client_for_context(cluster).await?;
    let api: Api<KubeEvent> = Api::namespaced(client, namespace);
    let selector = format!("involvedObject.name={},involvedObject.kind={}", name, kind);
    let mut events = api
        .list(&ListParams::default().fields(&selector))
        .await?
        .items
        .into_iter()
        .map(|event| ClusterRelatedEvent {
            type_: event.type_.unwrap_or_else(|| "Normal".to_string()),
            reason: event.reason.unwrap_or_else(|| "Unknown".to_string()),
            message: event.message.unwrap_or_default(),
            timestamp: event
                .event_time
                .map(|time| time.0.to_rfc3339())
                .or_else(|| {
                    event
                        .metadata
                        .creation_timestamp
                        .map(|time| time.0.to_rfc3339())
                })
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    Ok(events)
}

pub async fn top_metrics(req: &ClusterLogsRequest) -> Result<Vec<ClusterTopMetric>> {
    let client = client_for_context(&req.cluster).await?;
    let all = list_pod_metrics(&client, Some(&req.namespace)).await?;
    if req.kind == "Pod" {
        return Ok(all.into_iter().filter(|m| m.name == req.name).collect());
    }
    let selector = selector_for_workload(&client, req).await?;
    let pods: Api<Pod> = Api::namespaced(client, &req.namespace);
    let matched = pods
        .list(&ListParams::default().labels(&selector))
        .await?
        .items
        .into_iter()
        .map(|p| p.name_any())
        .collect::<std::collections::HashSet<_>>();
    Ok(all
        .into_iter()
        .filter(|m| matched.contains(&m.name))
        .collect())
}

pub async fn metrics_summary(
    cluster: &str,
    namespace: Option<&str>,
) -> Result<ClusterMetricsSummary> {
    let client = client_for_context(cluster).await?;
    let namespace = namespace.filter(|value| *value != "_cluster");
    let pods = if let Some(ns) = namespace.filter(|value| *value != "all") {
        list_pod_metrics(&client, Some(ns)).await?
    } else {
        list_pod_metrics(&client, None).await?
    };

    let total_cpu_millicores = pods.iter().map(|pod| cpu_to_millicores(&pod.cpu)).sum();
    let total_memory_mib = pods.iter().map(|pod| memory_to_mib(&pod.memory)).sum();

    Ok(ClusterMetricsSummary {
        scope: if let Some(namespace) = namespace.filter(|value| *value != "all") {
            format!("{cluster}/{namespace}")
        } else {
            cluster.to_string()
        },
        pod_count: pods.len(),
        total_cpu_millicores,
        total_memory_mib,
        pods,
    })
}

async fn list_pod_metrics(
    client: &Client,
    namespace: Option<&str>,
) -> Result<Vec<ClusterTopMetric>> {
    let api_resource = ApiResource {
        group: "metrics.k8s.io".into(),
        version: "v1beta1".into(),
        api_version: "metrics.k8s.io/v1beta1".into(),
        kind: "PodMetrics".into(),
        plural: "pods".into(),
    };
    let items = if let Some(ns) = namespace {
        let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), ns, &api_resource);
        api.list(&ListParams::default()).await?.items
    } else {
        let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);
        api.list(&ListParams::default()).await?.items
    };

    Ok(items
        .into_iter()
        .map(|item| {
            let name = item.name_any();
            let mut cpu_mc: i64 = 0;
            let mut mem_mib: i64 = 0;
            if let Some(containers) = item.data.get("containers").and_then(|c| c.as_array()) {
                for container in containers {
                    if let Some(usage) = container.get("usage") {
                        if let Some(cpu) = usage.get("cpu").and_then(|v| v.as_str()) {
                            cpu_mc += cpu_to_millicores(cpu);
                        }
                        if let Some(mem) = usage.get("memory").and_then(|v| v.as_str()) {
                            mem_mib += memory_to_mib(mem);
                        }
                    }
                }
            }
            ClusterTopMetric {
                name,
                cpu: format!("{cpu_mc}m"),
                memory: format!("{mem_mib}Mi"),
            }
        })
        .collect())
}

/// Live per-workload resource utilization, normalized to a 0–1 ratio of usage
/// vs. the pod template's requests. Sourced from the real `metrics.k8s.io`
/// PodMetrics API — never synthetic.
#[derive(Debug, Clone)]
pub struct WorkloadUtilization {
    /// Mean CPU usage across pods as a fraction of the per-pod CPU request.
    pub cpu_ratio: f64,
    /// Mean memory usage across pods as a fraction of the per-pod memory request.
    pub mem_ratio: f64,
    /// Desired replicas from the workload's `spec.replicas`.
    pub current_replicas: u32,
    /// Number of pods observed for the workload.
    pub pod_count: usize,
}

/// Sum the pod template's CPU-request millicores and memory-request MiB across
/// all containers. Returns `(0, 0)` when no requests are declared.
fn pod_template_requests(manifest: &serde_json::Value) -> (i64, i64) {
    let mut cpu_mc = 0i64;
    let mut mem_mib = 0i64;
    if let Some(containers) = manifest
        .pointer("/spec/template/spec/containers")
        .and_then(|c| c.as_array())
    {
        for c in containers {
            if let Some(cpu) = c.pointer("/resources/requests/cpu").and_then(|v| v.as_str()) {
                cpu_mc += cpu_to_millicores(cpu);
            }
            if let Some(mem) = c
                .pointer("/resources/requests/memory")
                .and_then(|v| v.as_str())
            {
                mem_mib += memory_to_mib(mem);
            }
        }
    }
    (cpu_mc, mem_mib)
}

/// Read live utilization for a Deployment/StatefulSet from `metrics.k8s.io`,
/// normalized against the pod template's requests. Returns `None` (skip — cannot
/// make a safe scaling decision) when the workload declares no resource
/// requests, has no selector, has no running pods, or has no metrics yet.
pub async fn workload_utilization(
    cluster: &str,
    namespace: &str,
    kind: &str,
    name: &str,
) -> Result<Option<WorkloadUtilization>> {
    let client = client_for_context(cluster).await?;
    let req = ClusterLogsRequest {
        cluster: cluster.to_string(),
        namespace: namespace.to_string(),
        kind: kind.to_string(),
        name: name.to_string(),
        api_version: None,
        plural: None,
        namespaced: Some(true),
    };
    let manifest = manifest_for_workload(&client, &req).await?;
    let current_replicas = manifest
        .pointer("/spec/replicas")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;

    let (cpu_req_mc, mem_req_mib) = pod_template_requests(&manifest);
    if cpu_req_mc <= 0 && mem_req_mib <= 0 {
        return Ok(None); // no requests → cannot normalize to a ratio
    }
    let Some(selector) = selector_for_value(&manifest) else {
        return Ok(None);
    };

    let pods: Api<Pod> = Api::namespaced(client.clone(), namespace);
    let pod_names: std::collections::HashSet<String> = pods_for_selector(&pods, &selector)
        .await?
        .items
        .iter()
        .map(|p| p.name_any())
        .collect();
    if pod_names.is_empty() {
        return Ok(None);
    }

    let metrics = list_pod_metrics(&client, Some(namespace)).await?;
    let mut usage_cpu_mc = 0i64;
    let mut usage_mem_mib = 0i64;
    let mut matched = 0i64;
    for m in &metrics {
        if pod_names.contains(&m.name) {
            usage_cpu_mc += cpu_to_millicores(&m.cpu);
            usage_mem_mib += memory_to_mib(&m.memory);
            matched += 1;
        }
    }
    if matched == 0 {
        return Ok(None); // metrics not yet available for these pods
    }

    // Ratio = total usage / (per-pod request × pods observed) = mean per-pod
    // utilization, since all pods share the template's requests.
    let cpu_ratio = if cpu_req_mc > 0 {
        usage_cpu_mc as f64 / (cpu_req_mc * matched) as f64
    } else {
        0.0
    };
    let mem_ratio = if mem_req_mib > 0 {
        usage_mem_mib as f64 / (mem_req_mib * matched) as f64
    } else {
        0.0
    };

    Ok(Some(WorkloadUtilization {
        cpu_ratio,
        mem_ratio,
        current_replicas,
        pod_count: pod_names.len(),
    }))
}

#[allow(clippy::too_many_arguments)]
pub async fn manifest_diff(
    cluster: &str,
    namespace: &str,
    kind: &str,
    name: &str,
    draft_manifest: serde_json::Value,
    api_version: Option<&str>,
    plural: Option<&str>,
    namespaced: Option<bool>,
) -> Result<Vec<(String, String)>> {
    let current = manifest_for_workload(
        &client_for_context(cluster).await?,
        &ClusterLogsRequest {
            cluster: cluster.to_string(),
            namespace: namespace.to_string(),
            kind: kind.to_string(),
            name: name.to_string(),
            api_version: api_version.map(str::to_string),
            plural: plural.map(str::to_string),
            namespaced,
        },
    )
    .await?;

    let current_lines = serde_json::to_string_pretty(&current)?
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let draft_lines = serde_json::to_string_pretty(&draft_manifest)?
        .lines()
        .map(str::to_string)
        .collect::<Vec<_>>();
    let max = current_lines.len().max(draft_lines.len());
    let mut diff = Vec::new();

    for index in 0..max {
        let current_line = current_lines.get(index);
        let draft_line = draft_lines.get(index);
        if current_line == draft_line {
            if let Some(line) = current_line {
                diff.push(("same".to_string(), format!("  {line}")));
            }
            continue;
        }
        if let Some(line) = current_line {
            diff.push(("remove".to_string(), format!("- {line}")));
        }
        if let Some(line) = draft_line {
            diff.push(("add".to_string(), format!("+ {line}")));
        }
    }

    Ok(diff)
}

pub async fn helm_history(
    cluster: &str,
    namespace: &str,
    release: &str,
) -> Result<Vec<HelmRevisionEntry>> {
    let output = run_helm(vec![
        "history".to_string(),
        release.to_string(),
        "--kube-context".to_string(),
        cluster.to_string(),
        "-n".to_string(),
        namespace.to_string(),
        "-o".to_string(),
        "json".to_string(),
    ])
    .await?;
    let entries: Vec<HelmHistoryEntry> = serde_json::from_slice(&output)?;
    Ok(entries
        .into_iter()
        .map(|entry| HelmRevisionEntry {
            revision: entry.revision.to_string(),
            updated: entry.updated,
            status: entry.status,
            chart: entry.chart,
            app_version: entry.app_version,
            description: entry.description,
        })
        .collect())
}

pub async fn helm_action(
    cluster: &str,
    namespace: &str,
    release: &str,
    action: &str,
    chart: Option<&str>,
    values_yaml: Option<&str>,
    revision: Option<&str>,
) -> Result<String> {
    match action {
        "rollback" => {
            let rev = revision.context("revision is required for helm rollback")?;
            run_helm(vec![
                "rollback".to_string(),
                release.to_string(),
                rev.to_string(),
                "--kube-context".to_string(),
                cluster.to_string(),
                "-n".to_string(),
                namespace.to_string(),
            ])
            .await?;
            Ok(format!(
                "rolled back Helm release {} to revision {}",
                release, rev
            ))
        }
        "upgrade" | "install" => {
            let chart = chart.context("chart is required for helm install/upgrade")?;
            let mut args = if action == "install" {
                vec![
                    "install".to_string(),
                    release.to_string(),
                    chart.to_string(),
                ]
            } else {
                vec![
                    "upgrade".to_string(),
                    release.to_string(),
                    chart.to_string(),
                ]
            };
            args.extend([
                "--kube-context".to_string(),
                cluster.to_string(),
                "-n".to_string(),
                namespace.to_string(),
            ]);
            if action == "install" {
                args.push("--create-namespace".to_string());
            }

            let mut temp_path = None;
            if let Some(values) = values_yaml.filter(|value| !value.trim().is_empty()) {
                let path = std::env::temp_dir().join(format!(
                    "aether-helm-values-{}-{}.yaml",
                    release,
                    Utc::now().timestamp_millis()
                ));
                std::fs::write(&path, values)?;
                args.push("-f".to_string());
                args.push(path.to_string_lossy().to_string());
                temp_path = Some(path);
            }

            let result = run_helm(args).await;
            if let Some(path) = temp_path {
                let _ = std::fs::remove_file(path);
            }
            result?;
            Ok(format!("{}d Helm release {}", action, release))
        }
        other => anyhow::bail!("unsupported Helm action {}", other),
    }
}

pub async fn apply_manifest(
    cluster: &str,
    namespace: &str,
    kind: &str,
    manifest: serde_json::Value,
    api_version: Option<&str>,
    plural: Option<&str>,
    namespaced: Option<bool>,
) -> Result<String> {
    let client = client_for_context(cluster).await?;

    match kind {
        "CustomResource" => {
            replace_custom_resource(
                &client,
                namespace,
                manifest,
                kind,
                api_version.context("api_version is required for CustomResource apply")?,
                plural.context("plural is required for CustomResource apply")?,
                namespaced.unwrap_or(true),
            )
            .await?
        }
        "Node" => replace_cluster_resource::<Node>(&client, manifest).await?,
        "PersistentVolume" => {
            replace_cluster_resource::<PersistentVolume>(&client, manifest).await?
        }
        "StorageClass" => replace_cluster_resource::<StorageClass>(&client, manifest).await?,
        "Namespace" => replace_cluster_resource::<Namespace>(&client, manifest).await?,
        "Deployment" => replace_resource::<Deployment>(&client, namespace, manifest).await?,
        "StatefulSet" => replace_resource::<StatefulSet>(&client, namespace, manifest).await?,
        "DaemonSet" => replace_resource::<DaemonSet>(&client, namespace, manifest).await?,
        "Job" => replace_resource::<Job>(&client, namespace, manifest).await?,
        "CronJob" => replace_resource::<CronJob>(&client, namespace, manifest).await?,
        "Pod" => replace_resource::<Pod>(&client, namespace, manifest).await?,
        "Ingress" => replace_resource::<Ingress>(&client, namespace, manifest).await?,
        "Service" => replace_resource::<Service>(&client, namespace, manifest).await?,
        "ConfigMap" => replace_resource::<ConfigMap>(&client, namespace, manifest).await?,
        "ServiceAccount" => {
            replace_resource::<ServiceAccount>(&client, namespace, manifest).await?
        }
        "Secret" => replace_resource::<Secret>(&client, namespace, manifest).await?,
        "PersistentVolumeClaim" => {
            replace_resource::<PersistentVolumeClaim>(&client, namespace, manifest).await?
        }
        "ResourceQuota" => replace_resource::<ResourceQuota>(&client, namespace, manifest).await?,
        "LimitRange" => replace_resource::<LimitRange>(&client, namespace, manifest).await?,
        "HorizontalPodAutoscaler" => {
            replace_resource::<HorizontalPodAutoscaler>(&client, namespace, manifest).await?
        }
        "NetworkPolicy" => replace_resource::<NetworkPolicy>(&client, namespace, manifest).await?,
        "EndpointSlice" => replace_resource::<EndpointSlice>(&client, namespace, manifest).await?,
        "DataVolume" => {
            replace_dynamic_resource(
                &client,
                namespace,
                manifest,
                cdi_api_resource("DataVolume", "datavolumes"),
            )
            .await?
        }
        "VirtualMachine" => {
            replace_dynamic_resource(
                &client,
                namespace,
                manifest,
                kubevirt_api_resource("VirtualMachine", "virtualmachines"),
            )
            .await?
        }
        "VirtualMachineInstance" => {
            replace_dynamic_resource(
                &client,
                namespace,
                manifest,
                kubevirt_api_resource("VirtualMachineInstance", "virtualmachineinstances"),
            )
            .await?
        }
        "Event" => anyhow::bail!("apply is not supported for Event"),
        other => anyhow::bail!("apply is not supported for kind {}", other),
    }

    Ok(format!(
        "applied {} in namespace {} on cluster {}",
        kind, namespace, cluster
    ))
}

pub async fn list_namespaces(cluster: &str) -> Result<Vec<ClusterNamespaceSummary>> {
    let client = client_for_context(cluster).await?;
    let api: Api<Namespace> = Api::all(client);
    let mut namespaces = api
        .list(&ListParams::default())
        .await?
        .items
        .into_iter()
        .map(|namespace| ClusterNamespaceSummary {
            name: namespace.metadata.name.unwrap_or_default(),
            status: namespace
                .status
                .as_ref()
                .and_then(|status| status.phase.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            created_at: namespace
                .metadata
                .creation_timestamp
                .map(|timestamp| timestamp.0.to_rfc3339())
                .unwrap_or_default(),
        })
        .collect::<Vec<_>>();
    namespaces.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(namespaces)
}

pub async fn browse_resources(req: &ClusterBrowseRequest) -> Result<Vec<ClusterResourceSummary>> {
    let client = client_for_context(&req.cluster).await?;
    let namespace = normalized_namespace(req.namespace.as_deref());

    let mut resources = match req.kind.as_str() {
        "HelmRelease" => list_helm_releases(&req.cluster, namespace).await?,
        "Namespace" => list_namespace_resources(&client, &req.cluster).await?,
        "Node" => list_node_resources(&client, &req.cluster).await?,
        "PersistentVolume" => list_pv_resources(&client, &req.cluster).await?,
        "StorageClass" => list_storage_class_resources(&client, &req.cluster).await?,
        "CustomResource" => {
            list_custom_resources(
                &client,
                &req.cluster,
                namespace,
                req.api_version
                    .as_deref()
                    .context("api_version is required for CustomResource browse")?,
                req.plural
                    .as_deref()
                    .context("plural is required for CustomResource browse")?,
                req.namespaced.unwrap_or(true),
            )
            .await?
        }
        "Event" => list_event_resources(&client, &req.cluster, namespace).await?,
        "DataVolume" => {
            list_dynamic_named_resources(
                &client,
                &req.cluster,
                namespace,
                "DataVolume",
                cdi_api_resource("DataVolume", "datavolumes"),
            )
            .await?
        }
        "VirtualMachine" => {
            list_kubevirt_resources(
                &client,
                &req.cluster,
                namespace,
                "VirtualMachine",
                "virtualmachines",
            )
            .await?
        }
        "VirtualMachineInstance" => {
            list_kubevirt_resources(
                &client,
                &req.cluster,
                namespace,
                "VirtualMachineInstance",
                "virtualmachineinstances",
            )
            .await?
        }
        "Pod" => list_pod_resources(&client, &req.cluster, namespace).await?,
        "ServiceAccount" => {
            list_service_account_resources(&client, &req.cluster, namespace).await?
        }
        "Secret" => list_secret_resources(&client, &req.cluster, namespace).await?,
        "PersistentVolumeClaim" => list_pvc_resources(&client, &req.cluster, namespace).await?,
        "ResourceQuota" => list_resource_quota_resources(&client, &req.cluster, namespace).await?,
        "LimitRange" => list_limit_range_resources(&client, &req.cluster, namespace).await?,
        "HorizontalPodAutoscaler" => list_hpa_resources(&client, &req.cluster, namespace).await?,
        "NetworkPolicy" => list_network_policy_resources(&client, &req.cluster, namespace).await?,
        "CiliumNetworkPolicy" => {
            cilium::list_cilium_network_policies(&req.cluster, namespace).await?
        }
        "CiliumClusterwideNetworkPolicy" => {
            cilium::list_cilium_clusterwide_network_policies(&req.cluster).await?
        }
        "EndpointSlice" => list_endpoint_slice_resources(&client, &req.cluster, namespace).await?,
        "Job" => {
            list_named_resources::<Job>(
                &client,
                &req.cluster,
                namespace,
                "Job",
                workload_status_job,
                job_detail,
            )
            .await?
        }
        "CronJob" => {
            list_named_resources::<CronJob>(
                &client,
                &req.cluster,
                namespace,
                "CronJob",
                workload_status_cronjob,
                cronjob_detail,
            )
            .await?
        }
        "Deployment" => {
            list_named_resources::<Deployment>(
                &client,
                &req.cluster,
                namespace,
                "Deployment",
                workload_status_deployment,
                deployment_detail,
            )
            .await?
        }
        "StatefulSet" => {
            list_named_resources::<StatefulSet>(
                &client,
                &req.cluster,
                namespace,
                "StatefulSet",
                workload_status_statefulset,
                statefulset_detail,
            )
            .await?
        }
        "DaemonSet" => {
            list_named_resources::<DaemonSet>(
                &client,
                &req.cluster,
                namespace,
                "DaemonSet",
                workload_status_daemonset,
                daemonset_detail,
            )
            .await?
        }
        "Ingress" => list_ingress_resources(&client, &req.cluster, namespace).await?,
        "Service" => list_service_resources(&client, &req.cluster, namespace).await?,
        "ConfigMap" => list_configmap_resources(&client, &req.cluster, namespace).await?,
        other => anyhow::bail!("unsupported Kubernetes browse kind: {}", other),
    };

    resources
        .sort_by(|a, b| (&a.namespace, &a.kind, &a.name).cmp(&(&b.namespace, &b.kind, &b.name)));
    Ok(resources)
}

/// Label used when the API runs in-cluster with no kubeconfig contexts.
pub fn default_in_cluster_label() -> String {
    std::env::var("AETHER_CLUSTER_DISPLAY_NAME").unwrap_or_else(|_| "active-client".to_string())
}

pub fn is_in_cluster_label(context: &str) -> bool {
    context == "active-client" || context == "in-cluster" || context == default_in_cluster_label()
}

/// Resolve cluster name: explicit arg, first reachable kubeconfig context, or in-cluster fallback.
pub async fn resolve_reachable_cluster(cluster: Option<&str>) -> Result<String> {
    if let Some(c) = cluster.filter(|s| !s.is_empty()) {
        return Ok(c.to_string());
    }
    if let Ok(clusters) = list_clusters().await {
        if let Some(c) = clusters.into_iter().find(|c| c.reachable) {
            return Ok(c.name);
        }
    }
    if Client::try_default().await.is_ok() {
        return Ok(default_in_cluster_label());
    }
    Kubeconfig::read()
        .ok()
        .and_then(|kc| kc.current_context)
        .context("no Kubernetes cluster context available")
}

pub async fn client_for_cluster(context: &str) -> Result<Client> {
    client_for_context(context).await
}

async fn client_for_context(context: &str) -> Result<Client> {
    if is_in_cluster_label(context) {
        return Client::try_default()
            .await
            .context("in-cluster kubernetes client unavailable");
    }
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

async fn list_kind<K>(
    client: &Client,
    cluster: &str,
    kind: &str,
    status_fn: fn(&K) -> String,
    image_pointer: &str,
) -> Result<Vec<ClusterWorkload>>
where
    K: Clone
        + serde::de::DeserializeOwned
        + serde::Serialize
        + kube::Resource<DynamicType = ()>
        + std::fmt::Debug,
{
    let api: Api<K> = Api::all(client.clone());
    let list = api.list(&ListParams::default()).await?;
    let mut results = Vec::new();

    for item in list {
        let metadata = item.meta().clone();
        let name = metadata.name.unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let namespace = metadata.namespace.unwrap_or_else(|| "default".to_string());
        let created_at = metadata
            .creation_timestamp
            .map(|time| time.0.to_rfc3339())
            .unwrap_or_default();

        let value = serde_json::to_value(&item)?;
        let image = value
            .pointer(image_pointer)
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        results.push(ClusterWorkload {
            cluster: cluster.to_string(),
            namespace,
            kind: kind.to_string(),
            name,
            image,
            status: status_fn(&item),
            created_at,
        });
    }

    Ok(results)
}

async fn list_named_resources<K>(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
    kind: &str,
    status_fn: fn(&K) -> String,
    detail_fn: fn(&K) -> Option<String>,
) -> Result<Vec<ClusterResourceSummary>>
where
    K: Clone
        + serde::de::DeserializeOwned
        + serde::Serialize
        + kube::Resource<DynamicType = ()>
        + kube::Resource<Scope = kube::core::NamespaceResourceScope>
        + kube::ResourceExt
        + std::fmt::Debug,
{
    let list = list_objects::<K>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|item| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: item.namespace().unwrap_or_else(|| "default".to_string()),
            kind: kind.to_string(),
            name: item.name_any(),
            status: status_fn(&item),
            created_at: item
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: detail_fn(&item),
        })
        .collect())
}

async fn list_pod_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<Pod>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|pod| {
            let statuses = pod
                .status
                .as_ref()
                .and_then(|status| status.container_statuses.as_ref())
                .cloned()
                .unwrap_or_default();
            ClusterResourceSummary {
                cluster: cluster.to_string(),
                namespace: pod.namespace().unwrap_or_else(|| "default".to_string()),
                kind: "Pod".to_string(),
                name: pod.name_any(),
                status: pod
                    .status
                    .as_ref()
                    .and_then(|status| status.phase.clone())
                    .unwrap_or_else(|| "Unknown".to_string()),
                created_at: pod
                    .meta()
                    .creation_timestamp
                    .clone()
                    .map(|time| time.0.to_rfc3339())
                    .unwrap_or_default(),
                detail: Some(format!(
                    "{} ready · {} restarts",
                    statuses.iter().filter(|status| status.ready).count(),
                    statuses
                        .iter()
                        .map(|status| status.restart_count)
                        .sum::<i32>()
                )),
            }
        })
        .collect())
}

async fn list_secret_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<Secret>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|secret| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: secret.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "Secret".to_string(),
            name: secret.name_any(),
            status: secret.type_.clone().unwrap_or_else(|| "Opaque".to_string()),
            created_at: secret
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!(
                "{} keys",
                secret.data.as_ref().map(|data| data.len()).unwrap_or(0)
                    + secret
                        .string_data
                        .as_ref()
                        .map(|data| data.len())
                        .unwrap_or(0)
            )),
        })
        .collect())
}

async fn list_service_account_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<ServiceAccount>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|account| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: account.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "ServiceAccount".to_string(),
            name: account.name_any(),
            status: "active".to_string(),
            created_at: account
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!(
                "{} secrets",
                account
                    .secrets
                    .as_ref()
                    .map(|secrets| secrets.len())
                    .unwrap_or(0)
            )),
        })
        .collect())
}

async fn list_hpa_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<HorizontalPodAutoscaler>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|hpa| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: hpa.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "HorizontalPodAutoscaler".to_string(),
            name: hpa.name_any(),
            status: hpa
                .status
                .as_ref()
                .and_then(|status| status.current_replicas)
                .map(|current| format!("{current} replicas"))
                .unwrap_or_else(|| "unknown".to_string()),
            created_at: hpa
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: hpa.spec.as_ref().map(|spec| {
                format!(
                    "min {} · max {}",
                    spec.min_replicas.unwrap_or(1),
                    spec.max_replicas
                )
            }),
        })
        .collect())
}

async fn list_network_policy_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<NetworkPolicy>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|policy| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: policy.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "NetworkPolicy".to_string(),
            name: policy.name_any(),
            status: "active".to_string(),
            created_at: policy
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!(
                "{} policy types",
                policy
                    .spec
                    .as_ref()
                    .and_then(|spec| spec.policy_types.as_ref())
                    .map(|types| types.len())
                    .unwrap_or(0)
            )),
        })
        .collect())
}

async fn list_ingress_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<Ingress>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|ingress| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: ingress.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "Ingress".to_string(),
            name: ingress.name_any(),
            status: ingress
                .status
                .as_ref()
                .and_then(|status| status.load_balancer.as_ref())
                .and_then(|lb| lb.ingress.as_ref())
                .map(|entries| {
                    if entries.is_empty() {
                        "Pending"
                    } else {
                        "Ready"
                    }
                })
                .unwrap_or("Pending")
                .to_string(),
            created_at: ingress
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: ingress
                .spec
                .as_ref()
                .and_then(|spec| spec.rules.as_ref())
                .and_then(|rules| rules.first())
                .and_then(|rule| rule.host.clone()),
        })
        .collect())
}

async fn list_pvc_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<PersistentVolumeClaim>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|pvc| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: pvc.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "PersistentVolumeClaim".to_string(),
            name: pvc.name_any(),
            status: pvc
                .status
                .as_ref()
                .and_then(|status| status.phase.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            created_at: pvc
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!(
                "{} · {}",
                pvc.spec
                    .as_ref()
                    .and_then(|spec| spec.storage_class_name.clone())
                    .unwrap_or_else(|| "default-sc".to_string()),
                pvc.spec
                    .as_ref()
                    .and_then(|spec| spec.resources.as_ref())
                    .and_then(|resources| resources.requests.as_ref())
                    .and_then(|requests| requests.get("storage"))
                    .map(|q| q.0.clone())
                    .unwrap_or_else(|| "unknown size".to_string())
            )),
        })
        .collect())
}

async fn list_resource_quota_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<ResourceQuota>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|quota| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: quota.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "ResourceQuota".to_string(),
            name: quota.name_any(),
            status: "active".to_string(),
            created_at: quota
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!(
                "{} hard limits",
                quota
                    .status
                    .as_ref()
                    .and_then(|status| status.hard.as_ref())
                    .map(|hard| hard.len())
                    .unwrap_or(0)
            )),
        })
        .collect())
}

async fn list_limit_range_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<LimitRange>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|limit_range| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: limit_range
                .namespace()
                .unwrap_or_else(|| "default".to_string()),
            kind: "LimitRange".to_string(),
            name: limit_range.name_any(),
            status: "active".to_string(),
            created_at: limit_range
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!(
                "{} entries",
                limit_range
                    .spec
                    .as_ref()
                    .map(|spec| spec.limits.len())
                    .unwrap_or(0)
            )),
        })
        .collect())
}

async fn list_endpoint_slice_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<EndpointSlice>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|slice| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: slice.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "EndpointSlice".to_string(),
            name: slice.name_any(),
            status: slice.address_type.clone(),
            created_at: slice
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!("{} endpoints", slice.endpoints.len())),
        })
        .collect())
}

async fn list_namespace_resources(
    client: &Client,
    cluster: &str,
) -> Result<Vec<ClusterResourceSummary>> {
    let api: Api<Namespace> = Api::all(client.clone());
    let list = api.list(&ListParams::default()).await?;
    Ok(list
        .items
        .into_iter()
        .map(|namespace| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: "_cluster".to_string(),
            kind: "Namespace".to_string(),
            name: namespace.name_any(),
            status: namespace
                .status
                .as_ref()
                .and_then(|status| status.phase.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            created_at: namespace
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: None,
        })
        .collect())
}

async fn list_node_resources(
    client: &Client,
    cluster: &str,
) -> Result<Vec<ClusterResourceSummary>> {
    let api: Api<Node> = Api::all(client.clone());
    let list = api.list(&ListParams::default()).await?;
    Ok(list
        .items
        .into_iter()
        .map(|node| {
            let conditions = node
                .status
                .as_ref()
                .and_then(|status| status.conditions.as_ref())
                .cloned()
                .unwrap_or_default();
            let ready = conditions
                .iter()
                .find(|condition| condition.type_ == "Ready")
                .map(|condition| condition.status.clone())
                .unwrap_or_else(|| "Unknown".to_string());
            let schedulable = !node
                .spec
                .as_ref()
                .and_then(|spec| spec.unschedulable)
                .unwrap_or(false);
            ClusterResourceSummary {
                cluster: cluster.to_string(),
                namespace: "_cluster".to_string(),
                kind: "Node".to_string(),
                name: node.name_any(),
                status: if ready == "True" { "Ready" } else { "NotReady" }.to_string(),
                created_at: node
                    .meta()
                    .creation_timestamp
                    .clone()
                    .map(|time| time.0.to_rfc3339())
                    .unwrap_or_default(),
                detail: Some(format!(
                    "{} · {}",
                    if schedulable {
                        "schedulable"
                    } else {
                        "cordoned"
                    },
                    node.spec
                        .as_ref()
                        .and_then(|spec| spec.provider_id.clone())
                        .unwrap_or_else(|| "provider unknown".to_string())
                )),
            }
        })
        .collect())
}

async fn list_pv_resources(client: &Client, cluster: &str) -> Result<Vec<ClusterResourceSummary>> {
    let api: Api<PersistentVolume> = Api::all(client.clone());
    let list = api.list(&ListParams::default()).await?;
    Ok(list
        .items
        .into_iter()
        .map(|pv| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: "_cluster".to_string(),
            kind: "PersistentVolume".to_string(),
            name: pv.name_any(),
            status: pv
                .status
                .as_ref()
                .and_then(|status| status.phase.clone())
                .unwrap_or_else(|| "Unknown".to_string()),
            created_at: pv
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!(
                "{} · {}",
                pv.spec
                    .as_ref()
                    .and_then(|spec| spec.storage_class_name.clone())
                    .unwrap_or_else(|| "no storage class".to_string()),
                pv.spec
                    .as_ref()
                    .and_then(|spec| spec.capacity.as_ref())
                    .and_then(|cap| cap.get("storage"))
                    .map(|qty| qty.0.clone())
                    .unwrap_or_else(|| "unknown size".to_string())
            )),
        })
        .collect())
}

async fn list_storage_class_resources(
    client: &Client,
    cluster: &str,
) -> Result<Vec<ClusterResourceSummary>> {
    let api: Api<StorageClass> = Api::all(client.clone());
    let list = api.list(&ListParams::default()).await?;
    Ok(list
        .items
        .into_iter()
        .map(|sc| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: "_cluster".to_string(),
            kind: "StorageClass".to_string(),
            name: sc.name_any(),
            status: if sc.allow_volume_expansion.unwrap_or(false) {
                "Expandable"
            } else {
                "Standard"
            }
            .to_string(),
            created_at: sc
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!("provisioner {}", sc.provisioner)),
        })
        .collect())
}

async fn list_custom_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
    api_version: &str,
    plural: &str,
    namespaced: bool,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_dynamic_objects(
        client,
        if namespaced { namespace } else { None },
        custom_api_resource("CustomResource", api_version, plural),
        namespaced,
    )
    .await?;
    Ok(list
        .into_iter()
        .map(|item| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: item.namespace().unwrap_or_else(|| "_cluster".to_string()),
            kind: "CustomResource".to_string(),
            name: item.name_any(),
            status: item
                .data
                .pointer("/status/phase")
                .or_else(|| item.data.pointer("/status/state"))
                .or_else(|| item.data.pointer("/status/health"))
                .and_then(|value| value.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            created_at: item
                .metadata
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!("{api_version} · {plural}")),
        })
        .collect())
}

async fn list_event_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<KubeEvent>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|event| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: event.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "Event".to_string(),
            name: event.name_any(),
            status: event.type_.clone().unwrap_or_else(|| "Normal".to_string()),
            created_at: event
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: event.reason.clone().or(event.message.clone()),
        })
        .collect())
}

async fn list_kubevirt_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
    kind: &str,
    plural: &str,
) -> Result<Vec<ClusterResourceSummary>> {
    let list =
        list_dynamic_objects(client, namespace, kubevirt_api_resource(kind, plural), true).await?;
    Ok(list
        .into_iter()
        .map(|item| {
            let name = item.name_any();
            let namespace = item.namespace().unwrap_or_else(|| "default".to_string());
            let status = item
                .data
                .pointer("/status/printableStatus")
                .or_else(|| item.data.pointer("/status/phase"))
                .and_then(|value| value.as_str())
                .map(str::to_string)
                .unwrap_or_else(|| {
                    if item
                        .data
                        .pointer("/spec/running")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false)
                    {
                        "Running".to_string()
                    } else {
                        "Stopped".to_string()
                    }
                });

            let detail = match kind {
                "VirtualMachine" => {
                    let ready = item
                        .data
                        .pointer("/status/ready")
                        .and_then(|value| value.as_bool())
                        .unwrap_or(false);
                    Some(if ready { "ready" } else { "not ready" }.to_string())
                }
                "VirtualMachineInstance" => item
                    .data
                    .pointer("/status/nodeName")
                    .and_then(|value| value.as_str())
                    .map(|value| format!("node {}", value)),
                _ => None,
            };

            ClusterResourceSummary {
                cluster: cluster.to_string(),
                namespace,
                kind: kind.to_string(),
                name,
                status,
                created_at: item
                    .metadata
                    .creation_timestamp
                    .clone()
                    .map(|time| time.0.to_rfc3339())
                    .unwrap_or_default(),
                detail,
            }
        })
        .collect())
}

async fn list_dynamic_named_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
    kind: &str,
    api_resource: ApiResource,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_dynamic_objects(client, namespace, api_resource, true).await?;
    Ok(list
        .into_iter()
        .map(|item| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: item.namespace().unwrap_or_else(|| "default".to_string()),
            kind: kind.to_string(),
            name: item.name_any(),
            status: item
                .data
                .pointer("/status/phase")
                .or_else(|| item.data.pointer("/status/phaseName"))
                .and_then(|value| value.as_str())
                .unwrap_or("Unknown")
                .to_string(),
            created_at: item
                .metadata
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: item
                .data
                .pointer("/status/progress")
                .and_then(|value| value.as_str())
                .map(str::to_string),
        })
        .collect())
}

async fn list_service_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<Service>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|service| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: service.namespace().unwrap_or_else(|| "default".to_string()),
            kind: "Service".to_string(),
            name: service.name_any(),
            status: service
                .spec
                .as_ref()
                .and_then(|spec| spec.type_.clone())
                .unwrap_or_else(|| "ClusterIP".to_string()),
            created_at: service
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: service
                .spec
                .as_ref()
                .and_then(|spec| spec.cluster_ip.clone()),
        })
        .collect())
}

async fn list_configmap_resources(
    client: &Client,
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let list = list_objects::<ConfigMap>(client, namespace).await?;
    Ok(list
        .into_iter()
        .map(|configmap| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: configmap
                .namespace()
                .unwrap_or_else(|| "default".to_string()),
            kind: "ConfigMap".to_string(),
            name: configmap.name_any(),
            status: "active".to_string(),
            created_at: configmap
                .meta()
                .creation_timestamp
                .clone()
                .map(|time| time.0.to_rfc3339())
                .unwrap_or_default(),
            detail: Some(format!(
                "{} keys",
                configmap.data.as_ref().map(|data| data.len()).unwrap_or(0)
            )),
        })
        .collect())
}

async fn list_objects<K>(client: &Client, namespace: Option<&str>) -> Result<Vec<K>>
where
    K: Clone
        + serde::de::DeserializeOwned
        + kube::Resource<DynamicType = ()>
        + kube::Resource<Scope = kube::core::NamespaceResourceScope>
        + std::fmt::Debug,
{
    let list = if let Some(namespace) = namespace {
        let api: Api<K> = Api::namespaced(client.clone(), namespace);
        api.list(&ListParams::default()).await?
    } else {
        let api: Api<K> = Api::all(client.clone());
        api.list(&ListParams::default()).await?
    };
    Ok(list.items)
}

async fn list_helm_releases(
    cluster: &str,
    namespace: Option<&str>,
) -> Result<Vec<ClusterResourceSummary>> {
    let mut args = vec![
        "list".to_string(),
        "--kube-context".to_string(),
        cluster.to_string(),
        "-o".to_string(),
        "json".to_string(),
    ];
    if let Some(namespace) = namespace {
        args.push("-n".to_string());
        args.push(namespace.to_string());
    } else {
        args.push("-A".to_string());
    }

    let output = run_helm(args).await?;
    let releases: Vec<HelmListEntry> = serde_json::from_slice(&output)?;
    Ok(releases
        .into_iter()
        .map(|release| ClusterResourceSummary {
            cluster: cluster.to_string(),
            namespace: release.namespace,
            kind: "HelmRelease".to_string(),
            name: release.name,
            status: release.status,
            created_at: release.updated,
            detail: Some(format!(
                "chart {} · app {} · rev {}",
                release.chart,
                if release.app_version.is_empty() {
                    "unknown"
                } else {
                    &release.app_version
                },
                release.revision
            )),
        })
        .collect())
}

async fn list_dynamic_objects(
    client: &Client,
    namespace: Option<&str>,
    api_resource: ApiResource,
    namespaced: bool,
) -> Result<Vec<DynamicObject>> {
    let list = if namespaced {
        if let Some(namespace) = namespace {
            let api: Api<DynamicObject> =
                Api::namespaced_with(client.clone(), namespace, &api_resource);
            api.list(&ListParams::default()).await?
        } else {
            let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);
            api.list(&ListParams::default()).await?
        }
    } else {
        let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);
        api.list(&ListParams::default()).await?
    };
    Ok(list.items)
}

async fn selector_for_workload(client: &Client, req: &ClusterLogsRequest) -> Result<String> {
    if matches!(
        req.kind.as_str(),
        "VirtualMachine" | "VirtualMachineInstance"
    ) {
        return Ok(format!("kubevirt.io/vm={}", req.name));
    }
    if req.kind == "HelmRelease" {
        return Ok(format!("app.kubernetes.io/instance={}", req.name));
    }
    let manifest = manifest_for_workload(client, req).await?;
    selector_for_value(&manifest).context("workload selector.matchLabels missing")
}

async fn manifest_for_workload(
    client: &Client,
    req: &ClusterLogsRequest,
) -> Result<serde_json::Value> {
    match req.kind.as_str() {
        "CustomResource" => {
            let api_version = req
                .api_version
                .as_deref()
                .context("api_version is required for CustomResource detail")?;
            let plural = req
                .plural
                .as_deref()
                .context("plural is required for CustomResource detail")?;
            let api_resource = custom_api_resource("CustomResource", api_version, plural);
            if req.namespaced.unwrap_or(true) {
                let api: Api<DynamicObject> =
                    Api::namespaced_with(client.clone(), &req.namespace, &api_resource);
                Ok(serde_json::to_value(api.get(&req.name).await?)?)
            } else {
                let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);
                Ok(serde_json::to_value(api.get(&req.name).await?)?)
            }
        }
        "Node" => {
            let api: Api<Node> = Api::all(client.clone());
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "PersistentVolume" => {
            let api: Api<PersistentVolume> = Api::all(client.clone());
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "StorageClass" => {
            let api: Api<StorageClass> = Api::all(client.clone());
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "Deployment" => {
            let api: Api<Deployment> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "StatefulSet" => {
            let api: Api<StatefulSet> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "DaemonSet" => {
            let api: Api<DaemonSet> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "Pod" => {
            let api: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "ServiceAccount" => {
            let api: Api<ServiceAccount> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "Secret" => {
            let api: Api<Secret> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "PersistentVolumeClaim" => {
            let api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "ResourceQuota" => {
            let api: Api<ResourceQuota> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "LimitRange" => {
            let api: Api<LimitRange> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "HorizontalPodAutoscaler" => {
            let api: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "NetworkPolicy" => {
            let api: Api<NetworkPolicy> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "EndpointSlice" => {
            let api: Api<EndpointSlice> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "Endpoints" => {
            let api: Api<Endpoints> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "Job" => {
            let api: Api<Job> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "CronJob" => {
            let api: Api<CronJob> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "Ingress" => {
            let api: Api<Ingress> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "Service" => {
            let api: Api<Service> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "ConfigMap" => {
            let api: Api<ConfigMap> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "DataVolume" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &cdi_api_resource("DataVolume", "datavolumes"),
            );
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "Namespace" => {
            let api: Api<Namespace> = Api::all(client.clone());
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "Event" => {
            let api: Api<KubeEvent> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "VirtualMachine" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &kubevirt_api_resource("VirtualMachine", "virtualmachines"),
            );
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "VirtualMachineInstance" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &kubevirt_api_resource("VirtualMachineInstance", "virtualmachineinstances"),
            );
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "HelmRelease" => {
            let status = String::from_utf8(
                run_helm(vec![
                    "status".to_string(),
                    req.name.clone(),
                    "--kube-context".to_string(),
                    req.cluster.clone(),
                    "-n".to_string(),
                    req.namespace.clone(),
                    "-o".to_string(),
                    "json".to_string(),
                ])
                .await?,
            )?;
            let values = String::from_utf8(
                run_helm(vec![
                    "get".to_string(),
                    "values".to_string(),
                    req.name.clone(),
                    "--kube-context".to_string(),
                    req.cluster.clone(),
                    "-n".to_string(),
                    req.namespace.clone(),
                    "-o".to_string(),
                    "yaml".to_string(),
                ])
                .await
                .unwrap_or_default(),
            )
            .unwrap_or_default();
            let manifest = String::from_utf8(
                run_helm(vec![
                    "get".to_string(),
                    "manifest".to_string(),
                    req.name.clone(),
                    "--kube-context".to_string(),
                    req.cluster.clone(),
                    "-n".to_string(),
                    req.namespace.clone(),
                ])
                .await
                .unwrap_or_default(),
            )
            .unwrap_or_default();
            Ok(serde_json::json!({
                "apiVersion": "helm.sh/v1",
                "kind": "HelmRelease",
                "metadata": {
                    "name": req.name,
                    "namespace": req.namespace,
                },
                "status": serde_json::from_str::<serde_json::Value>(&status).unwrap_or_else(|_| serde_json::json!({ "raw": status })),
                "valuesYaml": values,
                "renderedManifest": manifest,
            }))
        }
        other => anyhow::bail!("unsupported Kubernetes workload kind: {}", other),
    }
}

fn selector_for_value(value: &serde_json::Value) -> Option<String> {
    let labels = value
        .pointer("/spec/selector/matchLabels")
        .or_else(|| value.pointer("/spec/selector"))
        .and_then(|value| value.as_object())?;

    let selector = labels
        .iter()
        .filter_map(|(key, value)| value.as_str().map(|value| format!("{}={}", key, value)))
        .collect::<Vec<_>>()
        .join(",");

    if selector.is_empty() {
        return None;
    }

    Some(selector)
}

fn sanitize_secret_manifest(manifest: &mut serde_json::Value) {
    if let Some(data) = manifest
        .get_mut("data")
        .and_then(|value| value.as_object_mut())
    {
        for value in data.values_mut() {
            *value = serde_json::Value::String("<redacted>".to_string());
        }
    }
    if let Some(data) = manifest
        .get_mut("stringData")
        .and_then(|value| value.as_object_mut())
    {
        for value in data.values_mut() {
            *value = serde_json::Value::String("<redacted>".to_string());
        }
    }
}

async fn pods_for_selector(pods: &Api<Pod>, selector: &str) -> Result<kube::api::ObjectList<Pod>> {
    pods.list(&ListParams::default().labels(selector))
        .await
        .with_context(|| format!("failed to list pods for selector {}", selector))
}

fn summarize_pod(pod: &Pod) -> ClusterPodSummary {
    let statuses = pod
        .status
        .as_ref()
        .and_then(|status| status.container_statuses.as_ref())
        .cloned()
        .unwrap_or_default();
    let ready = statuses.iter().filter(|status| status.ready).count();
    let restarts = statuses.iter().map(|status| status.restart_count).sum();

    ClusterPodSummary {
        name: pod.metadata.name.clone().unwrap_or_default(),
        phase: pod
            .status
            .as_ref()
            .and_then(|status| status.phase.clone())
            .unwrap_or_else(|| "Unknown".to_string()),
        ready,
        total_containers: statuses.len(),
        restarts,
        node: pod.spec.as_ref().and_then(|spec| spec.node_name.clone()),
    }
}

async fn delete_workload(client: &Client, req: &ClusterActionRequest) -> Result<()> {
    match req.kind.as_str() {
        "CustomResource" => {
            let api_version = req
                .api_version
                .as_deref()
                .context("api_version is required for CustomResource delete")?;
            let plural = req
                .plural
                .as_deref()
                .context("plural is required for CustomResource delete")?;
            let api_resource = custom_api_resource("CustomResource", api_version, plural);
            if req.namespaced.unwrap_or(true) {
                let api: Api<DynamicObject> =
                    Api::namespaced_with(client.clone(), &req.namespace, &api_resource);
                api.delete(&req.name, &DeleteParams::default()).await?;
            } else {
                let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);
                api.delete(&req.name, &DeleteParams::default()).await?;
            }
        }
        "PersistentVolume" => {
            let api: Api<PersistentVolume> = Api::all(client.clone());
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "StorageClass" => {
            let api: Api<StorageClass> = Api::all(client.clone());
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "Pod" => {
            let api: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "ServiceAccount" => {
            let api: Api<ServiceAccount> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "Secret" => {
            let api: Api<Secret> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "PersistentVolumeClaim" => {
            let api: Api<PersistentVolumeClaim> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "ResourceQuota" => {
            let api: Api<ResourceQuota> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "LimitRange" => {
            let api: Api<LimitRange> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "HorizontalPodAutoscaler" => {
            let api: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "NetworkPolicy" => {
            let api: Api<NetworkPolicy> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "EndpointSlice" => {
            let api: Api<EndpointSlice> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "Job" => {
            let api: Api<Job> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "CronJob" => {
            let api: Api<CronJob> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "Ingress" => {
            let api: Api<Ingress> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "Deployment" => {
            let api: Api<Deployment> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "StatefulSet" => {
            let api: Api<StatefulSet> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "DaemonSet" => {
            let api: Api<DaemonSet> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "Service" => {
            let api: Api<Service> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "ConfigMap" => {
            let api: Api<ConfigMap> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "DataVolume" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &cdi_api_resource("DataVolume", "datavolumes"),
            );
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "Namespace" => {
            let api: Api<Namespace> = Api::all(client.clone());
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "VirtualMachine" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &kubevirt_api_resource("VirtualMachine", "virtualmachines"),
            );
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "VirtualMachineInstance" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &kubevirt_api_resource("VirtualMachineInstance", "virtualmachineinstances"),
            );
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "HelmRelease" => {
            run_helm(vec![
                "uninstall".to_string(),
                req.name.clone(),
                "--kube-context".to_string(),
                req.cluster.clone(),
                "-n".to_string(),
                req.namespace.clone(),
            ])
            .await?;
        }
        other => anyhow::bail!("delete is not supported for kind {}", other),
    }

    Ok(())
}

async fn restart_workload(client: &Client, req: &ClusterActionRequest) -> Result<()> {
    if req.kind == "Pod" {
        let api: Api<Pod> = Api::namespaced(client.clone(), &req.namespace);
        api.delete(&req.name, &DeleteParams::default()).await?;
        return Ok(());
    }
    if req.kind == "VirtualMachineInstance" {
        let api: Api<DynamicObject> = Api::namespaced_with(
            client.clone(),
            &req.namespace,
            &kubevirt_api_resource("VirtualMachineInstance", "virtualmachineinstances"),
        );
        api.delete(&req.name, &DeleteParams::default()).await?;
        return Ok(());
    }

    let restarted_at = Utc::now().to_rfc3339();
    let patch = serde_json::json!({
        "spec": {
            "template": {
                "metadata": {
                    "annotations": {
                        "kubectl.kubernetes.io/restartedAt": restarted_at
                    }
                }
            }
        }
    });

    match req.kind.as_str() {
        "Deployment" => {
            let api: Api<Deployment> = Api::namespaced(client.clone(), &req.namespace);
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await?;
        }
        "StatefulSet" => {
            let api: Api<StatefulSet> = Api::namespaced(client.clone(), &req.namespace);
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await?;
        }
        "DaemonSet" => {
            let api: Api<DaemonSet> = Api::namespaced(client.clone(), &req.namespace);
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await?;
        }
        "VirtualMachine" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &kubevirt_api_resource("VirtualMachine", "virtualmachines"),
            );
            let stop_patch = serde_json::json!({ "spec": { "running": false } });
            let start_patch = serde_json::json!({ "spec": { "running": true } });
            api.patch(
                &req.name,
                &PatchParams::default(),
                &Patch::Merge(&stop_patch),
            )
            .await?;
            api.patch(
                &req.name,
                &PatchParams::default(),
                &Patch::Merge(&start_patch),
            )
            .await?;
        }
        other => anyhow::bail!("restart is not supported for kind {}", other),
    }

    Ok(())
}

async fn start_workload(client: &Client, req: &ClusterActionRequest) -> Result<()> {
    match req.kind.as_str() {
        "VirtualMachine" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &kubevirt_api_resource("VirtualMachine", "virtualmachines"),
            );
            let patch = serde_json::json!({ "spec": { "running": true } });
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await?;
        }
        other => anyhow::bail!("start is not supported for kind {}", other),
    }
    Ok(())
}

async fn stop_workload(client: &Client, req: &ClusterActionRequest) -> Result<()> {
    match req.kind.as_str() {
        "VirtualMachine" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &kubevirt_api_resource("VirtualMachine", "virtualmachines"),
            );
            let patch = serde_json::json!({ "spec": { "running": false } });
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await?;
        }
        other => anyhow::bail!("stop is not supported for kind {}", other),
    }
    Ok(())
}

async fn cordon_node(cluster: &str, node: &str, cordon: bool) -> Result<()> {
    let output = Command::new("kubectl")
        .args([
            "--context",
            cluster,
            if cordon { "cordon" } else { "uncordon" },
            node,
        ])
        .output()
        .await
        .context("failed to spawn kubectl")?;
    if !output.status.success() {
        anyhow::bail!(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

async fn drain_node(cluster: &str, node: &str) -> Result<()> {
    let output = Command::new("kubectl")
        .args([
            "--context",
            cluster,
            "drain",
            node,
            "--ignore-daemonsets",
            "--delete-emptydir-data",
            "--force",
            "--grace-period=30",
            "--timeout=120s",
        ])
        .output()
        .await
        .context("failed to spawn kubectl")?;
    if !output.status.success() {
        anyhow::bail!(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(())
}

async fn suspend_workload(client: &Client, req: &ClusterActionRequest) -> Result<()> {
    match req.kind.as_str() {
        "CronJob" => {
            let api: Api<CronJob> = Api::namespaced(client.clone(), &req.namespace);
            let patch = serde_json::json!({ "spec": { "suspend": true } });
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await?;
        }
        other => anyhow::bail!("suspend is not supported for kind {}", other),
    }
    Ok(())
}

async fn resume_workload(client: &Client, req: &ClusterActionRequest) -> Result<()> {
    match req.kind.as_str() {
        "CronJob" => {
            let api: Api<CronJob> = Api::namespaced(client.clone(), &req.namespace);
            let patch = serde_json::json!({ "spec": { "suspend": false } });
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await?;
        }
        other => anyhow::bail!("resume is not supported for kind {}", other),
    }
    Ok(())
}

async fn scale_workload(client: &Client, req: &ClusterActionRequest) -> Result<()> {
    let replicas = req
        .replicas
        .context("replicas is required for scale action")?;
    if replicas < 0 {
        anyhow::bail!("replicas must be zero or greater");
    }

    let patch = serde_json::json!({
        "spec": {
            "replicas": replicas
        }
    });

    match req.kind.as_str() {
        "Deployment" => {
            let api: Api<Deployment> = Api::namespaced(client.clone(), &req.namespace);
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await?;
        }
        "StatefulSet" => {
            let api: Api<StatefulSet> = Api::namespaced(client.clone(), &req.namespace);
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch))
                .await?;
        }
        "DaemonSet" => anyhow::bail!("scale is not supported for DaemonSet"),
        other => anyhow::bail!("scale is not supported for kind {}", other),
    }

    Ok(())
}

fn workload_status_deployment(item: &Deployment) -> String {
    let desired = item
        .spec
        .as_ref()
        .and_then(|spec| spec.replicas)
        .unwrap_or(1);
    let ready = item
        .status
        .as_ref()
        .and_then(|status| status.ready_replicas)
        .unwrap_or(0);
    match (ready, desired) {
        (r, d) if d > 0 && r >= d => "running".to_string(),
        (r, _) if r > 0 => "degraded".to_string(),
        _ => "pending".to_string(),
    }
}

fn workload_status_statefulset(item: &StatefulSet) -> String {
    let desired = item
        .spec
        .as_ref()
        .and_then(|spec| spec.replicas)
        .unwrap_or(1);
    let ready = item
        .status
        .as_ref()
        .and_then(|status| status.ready_replicas)
        .unwrap_or(0);
    match (ready, desired) {
        (r, d) if d > 0 && r >= d => "running".to_string(),
        (r, _) if r > 0 => "degraded".to_string(),
        _ => "pending".to_string(),
    }
}

fn workload_status_daemonset(item: &DaemonSet) -> String {
    let desired = item
        .status
        .as_ref()
        .map(|status| status.desired_number_scheduled)
        .unwrap_or(0);
    let ready = item
        .status
        .as_ref()
        .map(|status| status.number_ready)
        .unwrap_or(0);
    match (ready, desired) {
        (r, d) if d > 0 && r >= d => "running".to_string(),
        (r, _) if r > 0 => "degraded".to_string(),
        _ => "pending".to_string(),
    }
}

fn workload_status_job(item: &Job) -> String {
    let status = item.status.as_ref();
    let succeeded = status.and_then(|status| status.succeeded).unwrap_or(0);
    let failed = status.and_then(|status| status.failed).unwrap_or(0);
    let active = status.and_then(|status| status.active).unwrap_or(0);
    if succeeded > 0 {
        "succeeded".to_string()
    } else if failed > 0 {
        "failed".to_string()
    } else if active > 0 {
        "running".to_string()
    } else {
        "pending".to_string()
    }
}

fn workload_status_cronjob(item: &CronJob) -> String {
    let suspended = item
        .spec
        .as_ref()
        .and_then(|spec| spec.suspend)
        .unwrap_or(false);
    if suspended {
        "suspended".to_string()
    } else {
        "scheduled".to_string()
    }
}

fn deployment_detail(item: &Deployment) -> Option<String> {
    let ready = item
        .status
        .as_ref()
        .and_then(|status| status.ready_replicas)
        .unwrap_or(0);
    let desired = item
        .spec
        .as_ref()
        .and_then(|spec| spec.replicas)
        .unwrap_or(1);
    Some(format!("{}/{} ready", ready, desired))
}

fn statefulset_detail(item: &StatefulSet) -> Option<String> {
    let ready = item
        .status
        .as_ref()
        .and_then(|status| status.ready_replicas)
        .unwrap_or(0);
    let desired = item
        .spec
        .as_ref()
        .and_then(|spec| spec.replicas)
        .unwrap_or(1);
    Some(format!("{}/{} ready", ready, desired))
}

fn daemonset_detail(item: &DaemonSet) -> Option<String> {
    let ready = item
        .status
        .as_ref()
        .map(|status| status.number_ready)
        .unwrap_or(0);
    let desired = item
        .status
        .as_ref()
        .map(|status| status.desired_number_scheduled)
        .unwrap_or(0);
    Some(format!("{}/{} ready", ready, desired))
}

fn job_detail(item: &Job) -> Option<String> {
    let status = item.status.as_ref();
    let succeeded = status.and_then(|status| status.succeeded).unwrap_or(0);
    let failed = status.and_then(|status| status.failed).unwrap_or(0);
    let active = status.and_then(|status| status.active).unwrap_or(0);
    Some(format!(
        "{active} active · {succeeded} succeeded · {failed} failed"
    ))
}

fn cronjob_detail(item: &CronJob) -> Option<String> {
    item.spec
        .as_ref()
        .map(|spec| format!("schedule {}", spec.schedule))
}

fn normalized_namespace(namespace: Option<&str>) -> Option<&str> {
    match namespace {
        Some("all") | Some("") | None => None,
        other => other,
    }
}

fn kubevirt_api_resource(kind: &str, plural: &str) -> ApiResource {
    ApiResource {
        group: "kubevirt.io".to_string(),
        version: "v1".to_string(),
        api_version: "kubevirt.io/v1".to_string(),
        kind: kind.to_string(),
        plural: plural.to_string(),
    }
}

fn cdi_api_resource(kind: &str, plural: &str) -> ApiResource {
    ApiResource {
        group: "cdi.kubevirt.io".to_string(),
        version: "v1beta1".to_string(),
        api_version: "cdi.kubevirt.io/v1beta1".to_string(),
        kind: kind.to_string(),
        plural: plural.to_string(),
    }
}

fn custom_api_resource(kind: &str, api_version: &str, plural: &str) -> ApiResource {
    let (group, version) = api_version
        .split_once('/')
        .map(|(group, version)| (group.to_string(), version.to_string()))
        .unwrap_or_else(|| ("".to_string(), api_version.to_string()));
    ApiResource {
        group,
        version: version.clone(),
        api_version: api_version.to_string(),
        kind: kind.to_string(),
        plural: plural.to_string(),
    }
}

async fn replace_resource<K>(
    client: &Client,
    namespace: &str,
    manifest: serde_json::Value,
) -> Result<()>
where
    K: Clone
        + serde::de::DeserializeOwned
        + serde::Serialize
        + kube::Resource<DynamicType = ()>
        + kube::Resource<Scope = kube::core::NamespaceResourceScope>
        + kube::ResourceExt
        + std::fmt::Debug,
{
    let resource: K = serde_json::from_value(manifest)?;
    let api: Api<K> = Api::namespaced(client.clone(), namespace);
    let name = resource.name_any();
    api.replace(&name, &PostParams::default(), &resource)
        .await?;
    Ok(())
}

async fn replace_cluster_resource<K>(client: &Client, manifest: serde_json::Value) -> Result<()>
where
    K: Clone
        + serde::de::DeserializeOwned
        + serde::Serialize
        + kube::Resource<DynamicType = ()>
        + kube::Resource<Scope = kube::core::ClusterResourceScope>
        + kube::ResourceExt
        + std::fmt::Debug,
{
    let resource: K = serde_json::from_value(manifest)?;
    let api: Api<K> = Api::all(client.clone());
    let name = resource.name_any();
    api.replace(&name, &PostParams::default(), &resource)
        .await?;
    Ok(())
}

async fn replace_dynamic_resource(
    client: &Client,
    namespace: &str,
    manifest: serde_json::Value,
    api_resource: ApiResource,
) -> Result<()> {
    let resource: DynamicObject = serde_json::from_value(manifest)?;
    let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace, &api_resource);
    let name = resource.name_any();
    api.replace(&name, &PostParams::default(), &resource)
        .await?;
    Ok(())
}

async fn replace_custom_resource(
    client: &Client,
    namespace: &str,
    manifest: serde_json::Value,
    kind: &str,
    api_version: &str,
    plural: &str,
    namespaced: bool,
) -> Result<()> {
    let resource: DynamicObject = serde_json::from_value(manifest)?;
    let api_resource = custom_api_resource(kind, api_version, plural);
    let name = resource.name_any();
    if namespaced {
        let api: Api<DynamicObject> =
            Api::namespaced_with(client.clone(), namespace, &api_resource);
        api.replace(&name, &PostParams::default(), &resource)
            .await?;
    } else {
        let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);
        api.replace(&name, &PostParams::default(), &resource)
            .await?;
    }
    Ok(())
}

async fn run_helm(args: Vec<String>) -> Result<Vec<u8>> {
    if which::which("helm").is_err() {
        anyhow::bail!("helm binary not found");
    }
    let output = Command::new("helm")
        .args(args)
        .output()
        .await
        .context("failed to spawn helm")?;
    if !output.status.success() {
        anyhow::bail!(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }
    Ok(output.stdout)
}

fn cpu_to_millicores(value: &str) -> i64 {
    if let Some(raw) = value.strip_suffix('m') {
        raw.parse::<i64>().unwrap_or(0)
    } else {
        value
            .parse::<f64>()
            .map(|cores| (cores * 1000.0).round() as i64)
            .unwrap_or(0)
    }
}

fn memory_to_mib(value: &str) -> i64 {
    if let Some(raw) = value.strip_suffix("Ki") {
        raw.parse::<f64>()
            .map(|v| (v / 1024.0).round() as i64)
            .unwrap_or(0)
    } else if let Some(raw) = value.strip_suffix("Mi") {
        raw.parse::<i64>().unwrap_or(0)
    } else if let Some(raw) = value.strip_suffix("Gi") {
        raw.parse::<f64>()
            .map(|v| (v * 1024.0).round() as i64)
            .unwrap_or(0)
    } else {
        0
    }
}

#[cfg(test)]
mod inventory_tests {
    use super::*;
    use k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference;

    fn pod_with_owners(owners: Vec<OwnerReference>) -> Pod {
        let mut pod: Pod = serde_json::from_value(serde_json::json!({
            "apiVersion": "v1",
            "kind": "Pod",
            "metadata": { "name": "demo", "namespace": "default" },
            "spec": { "containers": [{ "image": "nginx:latest" }] },
            "status": { "phase": "Running" }
        }))
        .expect("pod json");
        pod.metadata.owner_references = Some(owners);
        pod
    }

    #[test]
    fn pod_is_inventory_workload_when_unowned() {
        assert!(pod_is_inventory_workload(&pod_with_owners(vec![])));
    }

    #[test]
    fn pod_is_not_inventory_when_owned_by_replicaset() {
        let owners = vec![OwnerReference {
            api_version: "apps/v1".into(),
            kind: "ReplicaSet".into(),
            name: "web-abc".into(),
            uid: "uid".into(),
            ..Default::default()
        }];
        assert!(!pod_is_inventory_workload(&pod_with_owners(owners)));
    }

    #[test]
    fn pod_is_inventory_when_owned_by_config_map() {
        let owners = vec![OwnerReference {
            api_version: "v1".into(),
            kind: "ConfigMap".into(),
            name: "cfg".into(),
            uid: "uid".into(),
            ..Default::default()
        }];
        assert!(pod_is_inventory_workload(&pod_with_owners(owners)));
    }
}
