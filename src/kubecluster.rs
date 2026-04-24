//! Native Kubernetes cluster inventory and workload discovery for Aether.

use anyhow::{Context, Result};
use chrono::Utc;
use k8s_openapi::api::apps::v1::{DaemonSet, Deployment, StatefulSet};
use k8s_openapi::api::autoscaling::v2::HorizontalPodAutoscaler;
use k8s_openapi::api::batch::v1::{CronJob, Job};
use k8s_openapi::api::core::v1::{ConfigMap, Event as KubeEvent, Namespace, PersistentVolumeClaim, Pod, Secret, Service, ServiceAccount};
use k8s_openapi::api::networking::v1::{Ingress, NetworkPolicy};
use kube::api::{Api, DeleteParams, ListParams, LogParams, Patch, PatchParams, PostParams};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::core::DynamicObject;
use kube::discovery::ApiResource;
use kube::{Client, Config, Resource, ResourceExt};
use serde::Serialize;

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
}

#[derive(Debug, Clone)]
pub struct ClusterActionRequest {
    pub cluster: String,
    pub namespace: String,
    pub kind: String,
    pub name: String,
    pub action: String,
    pub replicas: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ClusterBrowseRequest {
    pub cluster: String,
    pub namespace: Option<String>,
    pub kind: String,
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
pub struct ClusterResourceDetail {
    pub cluster: String,
    pub namespace: String,
    pub kind: String,
    pub name: String,
    pub api_version: Option<String>,
    pub pods: Vec<ClusterPodSummary>,
    pub conditions: Vec<ClusterConditionSummary>,
    pub manifest: serde_json::Value,
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
            }
        }
        Err(error) => ClusterSummaryResponse {
            enabled: false,
            connected: false,
            backend: "Aether Kubernetes".to_string(),
            cluster_count: 0,
            healthy_clusters: 0,
            workload_count: 0,
            clusters: Vec::new(),
            error: Some(error.to_string()),
        },
    }
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
    let clusters = list_clusters().await?;
    let mut workloads = Vec::new();

    for cluster in clusters {
        if !cluster.reachable {
            continue;
        }

        let client = match client_for_context(&cluster.name).await {
            Ok(client) => client,
            Err(_) => continue,
        };

        workloads.extend(list_kind::<Deployment>(&client, &cluster.name, "Deployment", workload_status_deployment).await?);
        workloads.extend(list_kind::<StatefulSet>(&client, &cluster.name, "StatefulSet", workload_status_statefulset).await?);
        workloads.extend(list_kind::<DaemonSet>(&client, &cluster.name, "DaemonSet", workload_status_daemonset).await?);
    }

    workloads.sort_by(|a, b| {
        (&a.cluster, &a.namespace, &a.kind, &a.name).cmp(&(&b.cluster, &b.namespace, &b.kind, &b.name))
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
                    reason: item.get("reason").and_then(|value| value.as_str()).map(str::to_string),
                    message: item.get("message").and_then(|value| value.as_str()).map(str::to_string),
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

    Ok(ClusterResourceDetail {
        cluster: req.cluster.clone(),
        namespace: req.namespace.clone(),
        kind: req.kind.clone(),
        name: req.name.clone(),
        api_version: manifest
            .get("apiVersion")
            .and_then(|value| value.as_str())
            .map(str::to_string),
        pods,
        conditions,
        manifest,
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
        other => anyhow::bail!("unsupported Kubernetes action: {}", other),
    }

    Ok(format!(
        "{} {} {}/{} on cluster {}",
        req.action, req.kind, req.namespace, req.name, req.cluster
    ))
}

pub async fn apply_manifest(
    cluster: &str,
    namespace: &str,
    kind: &str,
    manifest: serde_json::Value,
) -> Result<String> {
    let client = client_for_context(cluster).await?;

    match kind {
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
        "ServiceAccount" => replace_resource::<ServiceAccount>(&client, namespace, manifest).await?,
        "Secret" => replace_resource::<Secret>(&client, namespace, manifest).await?,
        "PersistentVolumeClaim" => replace_resource::<PersistentVolumeClaim>(&client, namespace, manifest).await?,
        "HorizontalPodAutoscaler" => replace_resource::<HorizontalPodAutoscaler>(&client, namespace, manifest).await?,
        "NetworkPolicy" => replace_resource::<NetworkPolicy>(&client, namespace, manifest).await?,
        "DataVolume" => replace_dynamic_resource(&client, namespace, manifest, cdi_api_resource("DataVolume", "datavolumes")).await?,
        "VirtualMachine" => replace_dynamic_resource(&client, namespace, manifest, kubevirt_api_resource("VirtualMachine", "virtualmachines")).await?,
        "VirtualMachineInstance" => replace_dynamic_resource(&client, namespace, manifest, kubevirt_api_resource("VirtualMachineInstance", "virtualmachineinstances")).await?,
        "Event" => anyhow::bail!("apply is not supported for Event"),
        other => anyhow::bail!("apply is not supported for kind {}", other),
    }

    Ok(format!("applied {} in namespace {} on cluster {}", kind, namespace, cluster))
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
        "Namespace" => list_namespace_resources(&client, &req.cluster).await?,
        "Event" => list_event_resources(&client, &req.cluster, namespace).await?,
        "DataVolume" => list_dynamic_named_resources(&client, &req.cluster, namespace, "DataVolume", cdi_api_resource("DataVolume", "datavolumes")).await?,
        "VirtualMachine" => list_kubevirt_resources(&client, &req.cluster, namespace, "VirtualMachine", "virtualmachines").await?,
        "VirtualMachineInstance" => list_kubevirt_resources(&client, &req.cluster, namespace, "VirtualMachineInstance", "virtualmachineinstances").await?,
        "Pod" => list_pod_resources(&client, &req.cluster, namespace).await?,
        "ServiceAccount" => list_service_account_resources(&client, &req.cluster, namespace).await?,
        "Secret" => list_secret_resources(&client, &req.cluster, namespace).await?,
        "PersistentVolumeClaim" => list_pvc_resources(&client, &req.cluster, namespace).await?,
        "HorizontalPodAutoscaler" => list_hpa_resources(&client, &req.cluster, namespace).await?,
        "NetworkPolicy" => list_network_policy_resources(&client, &req.cluster, namespace).await?,
        "Job" => list_named_resources::<Job>(&client, &req.cluster, namespace, "Job", workload_status_job, job_detail).await?,
        "CronJob" => list_named_resources::<CronJob>(&client, &req.cluster, namespace, "CronJob", workload_status_cronjob, cronjob_detail).await?,
        "Deployment" => list_named_resources::<Deployment>(&client, &req.cluster, namespace, "Deployment", workload_status_deployment, deployment_detail).await?,
        "StatefulSet" => list_named_resources::<StatefulSet>(&client, &req.cluster, namespace, "StatefulSet", workload_status_statefulset, statefulset_detail).await?,
        "DaemonSet" => list_named_resources::<DaemonSet>(&client, &req.cluster, namespace, "DaemonSet", workload_status_daemonset, daemonset_detail).await?,
        "Ingress" => list_ingress_resources(&client, &req.cluster, namespace).await?,
        "Service" => list_service_resources(&client, &req.cluster, namespace).await?,
        "ConfigMap" => list_configmap_resources(&client, &req.cluster, namespace).await?,
        other => anyhow::bail!("unsupported Kubernetes browse kind: {}", other),
    };

    resources.sort_by(|a, b| (&a.namespace, &a.kind, &a.name).cmp(&(&b.namespace, &b.kind, &b.name)));
    Ok(resources)
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

async fn list_kind<K>(
    client: &Client,
    cluster: &str,
    kind: &str,
    status_fn: fn(&K) -> String,
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
            .pointer("/spec/template/spec/containers/0/image")
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
                    statuses.iter().map(|status| status.restart_count).sum::<i32>()
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
                    + secret.string_data.as_ref().map(|data| data.len()).unwrap_or(0)
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
                account.secrets.as_ref().map(|secrets| secrets.len()).unwrap_or(0)
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
            detail: hpa
                .spec
                .as_ref()
                .map(|spec| format!("min {} · max {}", spec.min_replicas.unwrap_or(1), spec.max_replicas)),
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
                .map(|entries| if entries.is_empty() { "Pending" } else { "Ready" })
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
    let list = list_dynamic_objects(client, namespace, kubevirt_api_resource(kind, plural)).await?;
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
    let list = list_dynamic_objects(client, namespace, api_resource).await?;
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
            detail: service.spec.as_ref().and_then(|spec| spec.cluster_ip.clone()),
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
            namespace: configmap.namespace().unwrap_or_else(|| "default".to_string()),
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

async fn list_dynamic_objects(
    client: &Client,
    namespace: Option<&str>,
    api_resource: ApiResource,
) -> Result<Vec<DynamicObject>> {
    let list = if let Some(namespace) = namespace {
        let api: Api<DynamicObject> = Api::namespaced_with(client.clone(), namespace, &api_resource);
        api.list(&ListParams::default()).await?
    } else {
        let api: Api<DynamicObject> = Api::all_with(client.clone(), &api_resource);
        api.list(&ListParams::default()).await?
    };
    Ok(list.items)
}

async fn selector_for_workload(client: &Client, req: &ClusterLogsRequest) -> Result<String> {
    if matches!(req.kind.as_str(), "VirtualMachine" | "VirtualMachineInstance") {
        return Ok(format!("kubevirt.io/vm={}", req.name));
    }
    let manifest = manifest_for_workload(client, req).await?;
    selector_for_value(&manifest).context("workload selector.matchLabels missing")
}

async fn manifest_for_workload(client: &Client, req: &ClusterLogsRequest) -> Result<serde_json::Value> {
    match req.kind.as_str() {
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
        "HorizontalPodAutoscaler" => {
            let api: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), &req.namespace);
            Ok(serde_json::to_value(api.get(&req.name).await?)?)
        }
        "NetworkPolicy" => {
            let api: Api<NetworkPolicy> = Api::namespaced(client.clone(), &req.namespace);
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
    if let Some(data) = manifest.get_mut("data").and_then(|value| value.as_object_mut()) {
        for value in data.values_mut() {
            *value = serde_json::Value::String("<redacted>".to_string());
        }
    }
    if let Some(data) = manifest.get_mut("stringData").and_then(|value| value.as_object_mut()) {
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
        "HorizontalPodAutoscaler" => {
            let api: Api<HorizontalPodAutoscaler> = Api::namespaced(client.clone(), &req.namespace);
            api.delete(&req.name, &DeleteParams::default()).await?;
        }
        "NetworkPolicy" => {
            let api: Api<NetworkPolicy> = Api::namespaced(client.clone(), &req.namespace);
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
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
        }
        "StatefulSet" => {
            let api: Api<StatefulSet> = Api::namespaced(client.clone(), &req.namespace);
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
        }
        "DaemonSet" => {
            let api: Api<DaemonSet> = Api::namespaced(client.clone(), &req.namespace);
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
        }
        "VirtualMachine" => {
            let api: Api<DynamicObject> = Api::namespaced_with(
                client.clone(),
                &req.namespace,
                &kubevirt_api_resource("VirtualMachine", "virtualmachines"),
            );
            let stop_patch = serde_json::json!({ "spec": { "running": false } });
            let start_patch = serde_json::json!({ "spec": { "running": true } });
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&stop_patch)).await?;
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&start_patch)).await?;
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
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
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
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
        }
        other => anyhow::bail!("stop is not supported for kind {}", other),
    }
    Ok(())
}

async fn suspend_workload(client: &Client, req: &ClusterActionRequest) -> Result<()> {
    match req.kind.as_str() {
        "CronJob" => {
            let api: Api<CronJob> = Api::namespaced(client.clone(), &req.namespace);
            let patch = serde_json::json!({ "spec": { "suspend": true } });
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
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
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
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
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
        }
        "StatefulSet" => {
            let api: Api<StatefulSet> = Api::namespaced(client.clone(), &req.namespace);
            api.patch(&req.name, &PatchParams::default(), &Patch::Merge(&patch)).await?;
        }
        "DaemonSet" => anyhow::bail!("scale is not supported for DaemonSet"),
        other => anyhow::bail!("scale is not supported for kind {}", other),
    }

    Ok(())
}

fn workload_status_deployment(item: &Deployment) -> String {
    let desired = item.spec.as_ref().and_then(|spec| spec.replicas).unwrap_or(1);
    let ready = item.status.as_ref().and_then(|status| status.ready_replicas).unwrap_or(0);
    match (ready, desired) {
        (r, d) if d > 0 && r >= d => "running".to_string(),
        (r, _) if r > 0 => "degraded".to_string(),
        _ => "pending".to_string(),
    }
}

fn workload_status_statefulset(item: &StatefulSet) -> String {
    let desired = item.spec.as_ref().and_then(|spec| spec.replicas).unwrap_or(1);
    let ready = item.status.as_ref().and_then(|status| status.ready_replicas).unwrap_or(0);
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
    let ready = item.status.as_ref().and_then(|status| status.ready_replicas).unwrap_or(0);
    let desired = item.spec.as_ref().and_then(|spec| spec.replicas).unwrap_or(1);
    Some(format!("{}/{} ready", ready, desired))
}

fn statefulset_detail(item: &StatefulSet) -> Option<String> {
    let ready = item.status.as_ref().and_then(|status| status.ready_replicas).unwrap_or(0);
    let desired = item.spec.as_ref().and_then(|spec| spec.replicas).unwrap_or(1);
    Some(format!("{}/{} ready", ready, desired))
}

fn daemonset_detail(item: &DaemonSet) -> Option<String> {
    let ready = item.status.as_ref().map(|status| status.number_ready).unwrap_or(0);
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
    Some(format!("{active} active · {succeeded} succeeded · {failed} failed"))
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

async fn replace_resource<K>(client: &Client, namespace: &str, manifest: serde_json::Value) -> Result<()>
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
    api.replace(&name, &PostParams::default(), &resource).await?;
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
    api.replace(&name, &PostParams::default(), &resource).await?;
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
    api.replace(&name, &PostParams::default(), &resource).await?;
    Ok(())
}
