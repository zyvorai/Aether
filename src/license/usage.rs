// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

use anyhow::{Context, Result};
use k8s_openapi::api::core::v1::Node;
use kube::api::ListParams;
use kube::{Api, Client};
use serde::{Deserialize, Serialize};

/// Per-node breakdown for the /api/license/usage endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManagedNodeInfo {
    pub name: String,
    pub ready: bool,
    /// True when schedulable, non-control-plane, and Ready.
    pub billable: bool,
}

/// Result of counting managed (billable) nodes in the cluster.
pub struct NodeCountResult {
    /// Count of Ready, schedulable, non-control-plane nodes.
    pub managed_count: u32,
    /// Full breakdown of every node returned by the K8s API.
    pub all_nodes: Vec<ManagedNodeInfo>,
}

/// Count schedulable, non-control-plane nodes with `Ready=True`.
///
/// A node is billable if:
///   - `status.conditions` contains `Ready=True`
///   - `spec.unschedulable` is absent or `false`
///   - Does **not** have taint `node-role.kubernetes.io/control-plane`
///     or `node-role.kubernetes.io/master`
///
/// Returns `Err` when no K8s cluster is reachable; callers treat this
/// as "usage unknown" (not a license violation in v0.3.0).
pub async fn count_managed_nodes() -> Result<NodeCountResult> {
    let client = Client::try_default()
        .await
        .context("no Kubernetes cluster reachable for license node count")?;

    let nodes: Api<Node> = Api::all(client);
    let node_list = nodes
        .list(&ListParams::default())
        .await
        .context("failed to list Kubernetes nodes")?;

    let mut all_nodes = Vec::with_capacity(node_list.items.len());

    for node in &node_list.items {
        let name = node.metadata.name.clone().unwrap_or_default();

        let is_ready = node
            .status
            .as_ref()
            .and_then(|s| s.conditions.as_ref())
            .map(|conds| {
                conds
                    .iter()
                    .any(|c| c.type_ == "Ready" && c.status == "True")
            })
            .unwrap_or(false);

        let unschedulable = node
            .spec
            .as_ref()
            .and_then(|s| s.unschedulable)
            .unwrap_or(false);

        let is_control_plane = node
            .spec
            .as_ref()
            .and_then(|s| s.taints.as_ref())
            .map(|taints| {
                taints.iter().any(|t| {
                    t.key == "node-role.kubernetes.io/control-plane"
                        || t.key == "node-role.kubernetes.io/master"
                })
            })
            .unwrap_or(false);

        let billable = is_ready && !unschedulable && !is_control_plane;
        all_nodes.push(ManagedNodeInfo {
            name,
            ready: is_ready,
            billable,
        });
    }

    let managed_count = all_nodes.iter().filter(|n| n.billable).count() as u32;
    Ok(NodeCountResult {
        managed_count,
        all_nodes,
    })
}
