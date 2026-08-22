// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Multi-cloud federation platform — execute, arbitrage, mesh, migration waves.

use crate::cost::{self, CloudProvider};
use crate::fleet::edge::EdgeStore;
use crate::fleet::federation::{federation_policies, plan_placement};
use crate::intelligence::anomaly::load_placement_signals;
use crate::kubecluster::list_clusters;
use crate::migration::fleet::{plan_fleet_migration, FleetMigrationRequest};
use crate::migration::volume::{plan_volume_replication, VolumeReplicationRequest};
use crate::ragnarok::sovereign::{evaluate, SovereignConfig};
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::path::Path;

// ── Phase 25: Live federation execute ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationExecuteRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
    pub workload_name: Option<String>,
}

fn default_dry_run() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationExecuteReport {
    pub dry_run: bool,
    pub target_cluster: Option<String>,
    pub executed: Vec<String>,
    pub skipped: Vec<String>,
}

pub async fn execute_federation_sync(
    state_path: &Path,
    dry_run: bool,
    workload_name: Option<&str>,
) -> anyhow::Result<FederationExecuteReport> {
    let store = StateStore::load(state_path)?;
    let policy = federation_policies();
    if policy.clusters.is_empty() {
        return Ok(FederationExecuteReport {
            dry_run,
            target_cluster: None,
            executed: vec![],
            skipped: vec!["AETHER_FEDERATION_CLUSTERS not configured".into()],
        });
    }

    let ws: Option<WorkloadState> = workload_name
        .and_then(|name| store.get(name).cloned())
        .or_else(|| store.list().first().map(|ws| (*ws).clone()));

    let Some(ws) = ws else {
        return Ok(FederationExecuteReport {
            dry_run,
            target_cluster: None,
            executed: vec![],
            skipped: vec!["no workloads in fleet".into()],
        });
    };

    let spec = Workload::from_file(&ws.spec_path)?;
    let plan = plan_placement(&spec).await?;
    let target = plan.recommended_cluster.clone();

    let mut executed = Vec::new();
    let mut skipped = Vec::new();

    if let Some(ref cluster) = target {
        let line = format!("target federation sync for {} → cluster {cluster}", ws.name);
        if dry_run {
            executed.push(format!("dry-run: {line}"));
        } else {
            skipped.push(format!(
                "{line} — not applied: federation sync mutation not implemented"
            ));
        }
    } else {
        skipped.push(format!("no reachable cluster for {}", ws.name));
    }

    Ok(FederationExecuteReport {
        dry_run,
        target_cluster: target,
        executed,
        skipped,
    })
}

// ── Phase 26: Cross-cloud cost arbitrage ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostArbitrageEntry {
    pub workload: String,
    pub current_provider: String,
    pub suggested_provider: String,
    pub current_monthly_usd: f64,
    pub suggested_monthly_usd: f64,
    pub savings_usd: f64,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostArbitrageReport {
    pub generated_at: String,
    pub entries: Vec<CostArbitrageEntry>,
    pub total_savings_usd: f64,
}

pub fn build_cost_arbitrage(state_path: &Path) -> anyhow::Result<CostArbitrageReport> {
    let store = StateStore::load(state_path)?;
    let mut entries = Vec::new();
    let mut total = 0.0;

    for ws in store.list() {
        let Ok(spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        let Ok(estimates) = cost::estimate_all_providers(&spec) else {
            continue;
        };
        if estimates.len() < 2 {
            continue;
        }
        let cheapest = estimates
            .iter()
            .min_by(|a, b| a.total_monthly.partial_cmp(&b.total_monthly).unwrap())
            .unwrap();
        let current = estimates
            .iter()
            .find(|e| e.provider == CloudProvider::AWS)
            .or_else(|| estimates.first())
            .unwrap();
        if cheapest.provider == current.provider {
            continue;
        }
        let savings = current.total_monthly - cheapest.total_monthly;
        if savings <= 0.0 {
            continue;
        }
        total += savings;
        entries.push(CostArbitrageEntry {
            workload: ws.name.clone(),
            current_provider: format!("{:?}", current.provider),
            suggested_provider: format!("{:?}", cheapest.provider),
            current_monthly_usd: current.total_monthly,
            suggested_monthly_usd: cheapest.total_monthly,
            savings_usd: savings,
            reason: format!(
                "Switch from {:?} to {:?} for {:.0}% savings",
                current.provider,
                cheapest.provider,
                (savings / current.total_monthly.max(0.01)) * 100.0
            ),
        });
    }

    entries.sort_by(|a, b| b.savings_usd.partial_cmp(&a.savings_usd).unwrap());

    Ok(CostArbitrageReport {
        generated_at: crate::resources::now_rfc3339(),
        entries,
        total_savings_usd: total,
    })
}

// ── Phase 27: Cluster health mesh ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMeshNode {
    pub id: String,
    pub label: String,
    pub reachable: bool,
    pub score: f64,
    pub anomaly_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterMeshEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterHealthMeshReport {
    pub generated_at: String,
    pub nodes: Vec<ClusterMeshNode>,
    pub edges: Vec<ClusterMeshEdge>,
}

pub async fn build_cluster_health_mesh() -> anyhow::Result<ClusterHealthMeshReport> {
    let clusters = list_clusters().await.unwrap_or_default();
    let anomalies = load_placement_signals(&clusters).await;
    let policy = federation_policies();

    let nodes: Vec<ClusterMeshNode> = clusters
        .iter()
        .map(|c| {
            let anomaly_count = anomalies
                .by_cluster
                .get(&c.name)
                .map(|s| s.count)
                .unwrap_or(0);
            let weight = policy.weights.get(&c.name).copied().unwrap_or(1.0);
            let score = if c.reachable {
                (weight - anomaly_count as f64 * 0.05).clamp(0.0, 1.0)
            } else {
                0.0
            };
            ClusterMeshNode {
                id: c.name.clone(),
                label: c.name.clone(),
                reachable: c.reachable,
                score,
                anomaly_count,
            }
        })
        .collect();

    let mut edges = Vec::new();
    let names: Vec<String> = clusters.iter().map(|c| c.name.clone()).collect();
    for i in 0..names.len() {
        for j in (i + 1)..names.len() {
            edges.push(ClusterMeshEdge {
                from: names[i].clone(),
                to: names[j].clone(),
                kind: if policy.clusters.contains(&names[i]) && policy.clusters.contains(&names[j])
                {
                    "federation-peer".into()
                } else {
                    "discovered".into()
                },
            });
        }
    }

    Ok(ClusterHealthMeshReport {
        generated_at: crate::resources::now_rfc3339(),
        nodes,
        edges,
    })
}

// ── Phase 28: Edge + cloud unified fabric ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFabricNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub online: bool,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFabricEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedFabricReport {
    pub generated_at: String,
    pub nodes: Vec<UnifiedFabricNode>,
    pub edges: Vec<UnifiedFabricEdge>,
}

pub async fn build_unified_fabric(state_path: &Path) -> anyhow::Result<UnifiedFabricReport> {
    let store = StateStore::load(state_path)?;
    let edge_store = EdgeStore::load();
    let clusters = list_clusters().await.unwrap_or_default();
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    for c in &clusters {
        nodes.push(UnifiedFabricNode {
            id: format!("cluster:{name}", name = c.name),
            label: c.name.clone(),
            kind: "cluster".into(),
            online: c.reachable,
            detail: c.server.clone().unwrap_or_else(|| "kube context".into()),
        });
    }

    for agent in edge_store.list_agents() {
        nodes.push(UnifiedFabricNode {
            id: format!("edge:{site}", site = agent.site),
            label: agent.site.clone(),
            kind: "edge".into(),
            online: agent.online,
            detail: agent
                .kube_context
                .clone()
                .unwrap_or_else(|| "edge agent".into()),
        });
        if let Some(ctx) = &agent.kube_context {
            if clusters.iter().any(|c| c.name == *ctx) {
                edges.push(UnifiedFabricEdge {
                    from: format!("edge:{site}", site = agent.site),
                    to: format!("cluster:{ctx}"),
                });
            }
        }
    }

    for ws in store.list().into_iter().take(12) {
        let node_id = format!("workload:{name}", name = ws.name);
        nodes.push(UnifiedFabricNode {
            id: node_id.clone(),
            label: ws.name.clone(),
            kind: "workload".into(),
            online: true,
            detail: format!("{}", ws.runtime),
        });
        edges.push(UnifiedFabricEdge {
            from: node_id,
            to: format!("cluster:{}", ws.runtime),
        });
    }

    Ok(UnifiedFabricReport {
        generated_at: crate::resources::now_rfc3339(),
        nodes,
        edges,
    })
}

// ── Phase 29: Volume replication status ──────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeReplicationStatusEntry {
    pub workload: String,
    pub has_persistence: bool,
    pub plan_ready: bool,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeReplicationStatusReport {
    pub generated_at: String,
    pub entries: Vec<VolumeReplicationStatusEntry>,
}

pub fn build_volume_replication_status(
    state_path: &Path,
) -> anyhow::Result<VolumeReplicationStatusReport> {
    let store = StateStore::load(state_path)?;
    let policy = federation_policies();
    let source = policy
        .clusters
        .first()
        .cloned()
        .unwrap_or_else(|| "source".into());
    let target = policy
        .clusters
        .get(1)
        .cloned()
        .unwrap_or_else(|| "target".into());

    let mut entries = Vec::new();
    for ws in store.list() {
        let Ok(spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        if !spec.persistence.enabled {
            entries.push(VolumeReplicationStatusEntry {
                workload: ws.name.clone(),
                has_persistence: false,
                plan_ready: false,
                summary: "No persistent volume — replication N/A".into(),
            });
            continue;
        }
        let req = VolumeReplicationRequest {
            source_cluster: source.clone(),
            target_cluster: target.clone(),
            namespace: "default".into(),
            pvc_name: format!("{}-data", ws.name),
            workload_name: Some(ws.name.clone()),
        };
        let plan_ready = plan_volume_replication(&req, Some(&spec)).is_ok();
        entries.push(VolumeReplicationStatusEntry {
            workload: ws.name.clone(),
            has_persistence: true,
            plan_ready,
            summary: if plan_ready {
                format!("CSI replication plan ready {source} → {target}")
            } else {
                "Replication plan blocked — check cluster pair".into()
            },
        });
    }

    Ok(VolumeReplicationStatusReport {
        generated_at: crate::resources::now_rfc3339(),
        entries,
    })
}

// ── Phase 30: Migration wave planner ─────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationWaveReport {
    pub generated_at: String,
    pub target_cluster: String,
    pub wave_size: u32,
    pub plan: crate::migration::fleet::FleetMigrationPlan,
}

pub async fn build_migration_wave(state_path: &Path) -> anyhow::Result<MigrationWaveReport> {
    let store = StateStore::load(state_path)?;
    let clusters = list_clusters().await.unwrap_or_default();
    let policy = federation_policies();

    let target = policy
        .clusters
        .first()
        .cloned()
        .or_else(|| {
            clusters
                .iter()
                .find(|c| c.reachable)
                .map(|c| c.name.clone())
        })
        .unwrap_or_else(|| "primary".into());

    let names: Vec<String> = store
        .list()
        .iter()
        .map(|ws| ws.name.clone())
        .take(10)
        .collect();
    let mut specs = Vec::new();
    for name in &names {
        if let Some(ws) = store.get(name) {
            if let Ok(spec) = Workload::from_file(&ws.spec_path) {
                specs.push((name.clone(), spec, ws.runtime));
            }
        }
    }

    let plan = if specs.is_empty() {
        crate::migration::fleet::FleetMigrationPlan {
            target_cluster: target.clone(),
            strategy: "rolling".into(),
            items: Vec::new(),
            warnings: vec!["no Aether-managed workloads available to plan a migration wave".into()],
        }
    } else {
        plan_fleet_migration(
            &FleetMigrationRequest {
                workloads: names,
                target_cluster: target.clone(),
                strategy: Some("rolling".into()),
                replicate_volumes: Some(true),
            },
            &specs,
        )?
    };

    Ok(MigrationWaveReport {
        generated_at: crate::resources::now_rfc3339(),
        target_cluster: target,
        wave_size: plan.items.len() as u32,
        plan,
    })
}

// ── Phase 31: Geo latency placement ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPlacementEntry {
    pub cluster: String,
    pub region_hint: String,
    pub estimated_rtt_ms: u32,
    pub placement_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPlacementReport {
    pub generated_at: String,
    pub origin_region: String,
    pub clusters: Vec<GeoPlacementEntry>,
}

pub async fn build_geo_placement(_state_path: &Path) -> anyhow::Result<GeoPlacementReport> {
    let clusters = list_clusters().await.unwrap_or_default();
    let origin = std::env::var("AETHER_ORIGIN_REGION").unwrap_or_else(|_| "us-east".into());
    let policy = federation_policies();

    let mut entries: Vec<GeoPlacementEntry> = clusters
        .iter()
        .map(|c| {
            let region = infer_region(&c.name);
            let rtt = estimate_rtt(&origin, &region);
            let weight = policy.weights.get(&c.name).copied().unwrap_or(1.0);
            let latency_penalty = (rtt as f64 / 300.0).min(0.5);
            let score = if c.reachable {
                (weight - latency_penalty).clamp(0.0, 1.0)
            } else {
                0.0
            };
            GeoPlacementEntry {
                cluster: c.name.clone(),
                region_hint: region,
                estimated_rtt_ms: rtt,
                placement_score: score,
            }
        })
        .collect();

    entries.sort_by(|a, b| {
        b.placement_score
            .partial_cmp(&a.placement_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(GeoPlacementReport {
        generated_at: crate::resources::now_rfc3339(),
        origin_region: origin,
        clusters: entries,
    })
}

fn infer_region(cluster: &str) -> String {
    let lower = cluster.to_lowercase();
    if lower.contains("eu") {
        "eu-west".into()
    } else if lower.contains("ap") || lower.contains("asia") {
        "ap-southeast".into()
    } else {
        "us-east".into()
    }
}

fn estimate_rtt(origin: &str, target: &str) -> u32 {
    if origin == target {
        5
    } else if origin.starts_with("us") && target.starts_with("eu") {
        85
    } else if origin.starts_with("us") && target.starts_with("ap") {
        150
    } else {
        45
    }
}

// ── Phase 32: Cloud account linking ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudAccountLink {
    pub provider: String,
    pub configured: bool,
    pub account_hint: String,
    pub env_var: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudAccountVaultReport {
    pub generated_at: String,
    pub accounts: Vec<CloudAccountLink>,
}

pub fn build_cloud_account_vault() -> CloudAccountVaultReport {
    let accounts = vec![
        link("AWS", "AWS_PROFILE", "AWS_ACCESS_KEY_ID"),
        link("GCP", "GOOGLE_APPLICATION_CREDENTIALS", "GCP_PROJECT"),
        link("Azure", "AZURE_SUBSCRIPTION_ID", "AZURE_CLIENT_ID"),
    ];
    CloudAccountVaultReport {
        generated_at: crate::resources::now_rfc3339(),
        accounts,
    }
}

fn link(provider: &str, primary: &str, fallback: &str) -> CloudAccountLink {
    let primary_val = std::env::var(primary).ok();
    let fallback_val = std::env::var(fallback).ok();
    let configured = primary_val.is_some() || fallback_val.is_some();
    let hint = primary_val
        .or(fallback_val)
        .map(|v| mask_secret(&v))
        .unwrap_or_else(|| "not linked".into());
    CloudAccountLink {
        provider: provider.into(),
        configured,
        account_hint: hint,
        env_var: primary.into(),
    }
}

fn mask_secret(value: &str) -> String {
    if value.len() <= 4 {
        "****".into()
    } else {
        format!("{}…{}", &value[..2], &value[value.len() - 2..])
    }
}

// ── Phase 33: Region lock enforcement ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionLockEntry {
    pub workload: String,
    pub compliant: bool,
    pub region_lock: Option<String>,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionLockReport {
    pub generated_at: String,
    pub sovereign_region_lock: Option<String>,
    pub workloads: Vec<RegionLockEntry>,
}

pub fn build_region_lock_posture(state_path: &Path) -> anyhow::Result<RegionLockReport> {
    let store = StateStore::load(state_path)?;
    let config = SovereignConfig::from_env();
    let mut workloads = Vec::new();

    for ws in store.list() {
        let Ok(spec) = Workload::from_file(&ws.spec_path) else {
            continue;
        };
        let verdict = evaluate(&spec, &config);
        workloads.push(RegionLockEntry {
            workload: ws.name.clone(),
            compliant: verdict.compliant,
            region_lock: spec
                .confidential
                .as_ref()
                .and_then(|c| c.region_lock.clone()),
            violations: verdict.violations,
        });
    }

    Ok(RegionLockReport {
        generated_at: crate::resources::now_rfc3339(),
        sovereign_region_lock: config.region_lock.clone(),
        workloads,
    })
}

// ── Phase 34: PacketWolf placement guard ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketWolfGuardEntry {
    pub cluster: String,
    pub anomaly_count: u32,
    pub blocked: bool,
    pub penalty: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketWolfGuardReport {
    pub generated_at: String,
    pub configured: bool,
    pub blocked_clusters: Vec<String>,
    pub entries: Vec<PacketWolfGuardEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketWolfGuardApplyRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketWolfGuardApplyReport {
    pub dry_run: bool,
    pub applied: Vec<String>,
    pub skipped: Vec<String>,
}

pub async fn build_packetwolf_guard() -> anyhow::Result<PacketWolfGuardReport> {
    let clusters = list_clusters().await.unwrap_or_default();
    let signals = load_placement_signals(&clusters).await;
    let threshold = 3u32;

    let entries: Vec<PacketWolfGuardEntry> = clusters
        .iter()
        .map(|c| {
            let count = signals
                .by_cluster
                .get(&c.name)
                .map(|s| s.count)
                .unwrap_or(0);
            let penalty = signals.cluster_penalty(&c.name);
            PacketWolfGuardEntry {
                cluster: c.name.clone(),
                anomaly_count: count,
                blocked: count >= threshold,
                penalty,
            }
        })
        .collect();

    let blocked_clusters: Vec<String> = entries
        .iter()
        .filter(|e| e.blocked)
        .map(|e| e.cluster.clone())
        .collect();

    Ok(PacketWolfGuardReport {
        generated_at: crate::resources::now_rfc3339(),
        configured: signals.configured,
        blocked_clusters,
        entries,
    })
}

pub async fn apply_packetwolf_guard(dry_run: bool) -> anyhow::Result<PacketWolfGuardApplyReport> {
    let report = build_packetwolf_guard().await?;
    let mut applied = Vec::new();
    let mut skipped = Vec::new();

    for entry in &report.entries {
        if entry.blocked {
            let line = format!(
                "avoid placement on {} ({} anomalies, penalty {:.1})",
                entry.cluster, entry.anomaly_count, entry.penalty
            );
            if dry_run {
                applied.push(format!("dry-run: {line}"));
            } else {
                skipped.push(format!(
                    "{line} — not applied: placement guard mutation not implemented"
                ));
            }
        } else {
            skipped.push(format!("{}: within anomaly threshold", entry.cluster));
        }
    }

    Ok(PacketWolfGuardApplyReport {
        dry_run,
        applied,
        skipped,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cloud_vault_lists_providers() {
        assert_eq!(build_cloud_account_vault().accounts.len(), 3);
    }

    #[test]
    fn estimate_rtt_same_region() {
        assert_eq!(estimate_rtt("us-east", "us-east"), 5);
    }

    #[tokio::test]
    async fn federation_execute_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        std::env::remove_var("AETHER_FEDERATION_CLUSTERS");
        let report = execute_federation_sync(&path, true, None).await.unwrap();
        assert!(!report.skipped.is_empty());
    }
}
