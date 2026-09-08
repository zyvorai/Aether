// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Knowledge Graph OS — impact, blast radius, CMDB, snapshots, export, placement.

use crate::dependencies::{DependencyGraph, ImpactReport, ImpactSeverity};
use crate::intelligence::graph::build_knowledge_graph;
use crate::intelligence::security::SecurityEngine;
use crate::intelligence::twin::{DigitalTwinEngine, TwinSimulateRequest};
use crate::kubecluster::list_clusters;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

fn default_dry_run() -> bool {
    true
}

fn snapshot_path() -> PathBuf {
    crate::resources::aether_path("graph-snapshots.json")
}

fn cmdb_path() -> PathBuf {
    crate::resources::aether_path("cmdb.json")
}

fn load_pairs(state_path: &Path) -> anyhow::Result<Vec<(Workload, WorkloadState)>> {
    let store = StateStore::load(state_path)?;
    Ok(store
        .list()
        .iter()
        .filter_map(|ws| {
            Workload::from_file(&ws.spec_path)
                .ok()
                .map(|s| (s, (*ws).clone()))
        })
        .collect())
}

// ── Phase 45: Interactive graph (filtered view) ──────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InteractiveGraphRequest {
    #[serde(default)]
    pub edge_kinds: Vec<String>,
    #[serde(default)]
    pub node_kinds: Vec<String>,
}

pub fn build_interactive_graph(
    state_path: &Path,
    edge_kinds: &[String],
    node_kinds: &[String],
) -> anyhow::Result<crate::intelligence::graph::KnowledgeGraphReport> {
    let mut graph = build_knowledge_graph(state_path)?;
    if !edge_kinds.is_empty() {
        let kinds: HashSet<_> = edge_kinds.iter().map(|s| s.as_str()).collect();
        graph.edges.retain(|e| kinds.contains(e.kind.as_str()));
    }
    if !node_kinds.is_empty() {
        let kinds: HashSet<_> = node_kinds.iter().map(|s| s.as_str()).collect();
        graph.nodes.retain(|n| kinds.contains(n.kind.as_str()));
        let node_ids: HashSet<_> = graph.nodes.iter().map(|n| n.id.clone()).collect();
        graph
            .edges
            .retain(|e| node_ids.contains(&e.from) && node_ids.contains(&e.to));
    }
    Ok(graph)
}

// ── Phase 46: Impact analysis ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphImpactReport {
    pub generated_at: String,
    pub workload: String,
    pub impact: ImpactReport,
    pub downstream: Vec<String>,
    pub summary: String,
}

pub fn build_impact_analysis(
    _state_path: &Path,
    workload: &str,
) -> anyhow::Result<GraphImpactReport> {
    let deps = DependencyGraph::load(&DependencyGraph::default_path()).unwrap_or_default();
    let impact = deps.impact_analysis(workload);
    let summary = format!(
        "Stopping {} affects {} downstream workload(s) — severity {}",
        workload, impact.cascade_count, impact.severity
    );
    Ok(GraphImpactReport {
        generated_at: crate::resources::now_rfc3339(),
        workload: workload.into(),
        downstream: impact.affected_workloads.clone(),
        impact,
        summary,
    })
}

// ── Phase 47: Blast radius scoring ───────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadiusReport {
    pub generated_at: String,
    pub workload: String,
    pub blast_score: f64,
    pub impact: ImpactReport,
    pub twin_risk_delta: f64,
    pub migration_blockers: Vec<String>,
    pub summary: String,
}

pub fn build_blast_radius(state_path: &Path, workload: &str) -> anyhow::Result<BlastRadiusReport> {
    let pairs = load_pairs(state_path)?;
    let deps = DependencyGraph::load(&DependencyGraph::default_path()).unwrap_or_default();
    let impact = deps.impact_analysis(workload);

    let twin_fail = DigitalTwinEngine::simulate(
        &pairs,
        &TwinSimulateRequest {
            workload: Some(workload.into()),
            scale_factor: 2.0,
            target_runtime: None,
        },
    );

    let severity_weight = match impact.severity {
        ImpactSeverity::None => 0.1,
        ImpactSeverity::Low => 0.35,
        ImpactSeverity::Medium => 0.65,
        ImpactSeverity::High => 0.95,
    };
    let blast_score =
        (severity_weight * 0.6 + (impact.cascade_count as f64 / 10.0).min(1.0) * 0.4).min(1.0);

    let mut migration_blockers = Vec::new();
    if impact.cascade_count > 3 {
        migration_blockers.push("High downstream fan-out — migrate dependents first".into());
    }
    if twin_fail.deltas.risk_delta > 0.15 {
        migration_blockers.push(format!(
            "Twin simulation risk +{:.0}% if removed",
            twin_fail.deltas.risk_delta * 100.0
        ));
    }

    Ok(BlastRadiusReport {
        generated_at: crate::resources::now_rfc3339(),
        workload: workload.into(),
        blast_score,
        impact,
        twin_risk_delta: twin_fail.deltas.risk_delta,
        migration_blockers,
        summary: format!("Blast radius {:.0}/100 for {workload}", blast_score * 100.0),
    })
}

// ── Phase 48: K8s service dependency import ────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sServiceNode {
    pub service: String,
    pub workload: String,
    pub service_type: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sImportReport {
    pub generated_at: String,
    pub services: Vec<K8sServiceNode>,
    pub edges_added: u32,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct K8sImportRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

pub fn import_k8s_services(state_path: &Path, dry_run: bool) -> anyhow::Result<K8sImportReport> {
    let pairs = load_pairs(state_path)?;
    let mut services = Vec::new();
    let mut edges_added = 0u32;
    let mut deps = DependencyGraph::load(&DependencyGraph::default_path()).unwrap_or_default();

    for (spec, ws) in &pairs {
        if !matches!(ws.runtime, crate::runtime::RuntimeKind::Kubernetes) {
            continue;
        }
        if !spec.network.service {
            continue;
        }
        let port = spec
            .network
            .ports
            .first()
            .map(|p| p.service_port)
            .unwrap_or(80);
        services.push(K8sServiceNode {
            service: format!("{}-svc", spec.metadata.name),
            workload: spec.metadata.name.clone(),
            service_type: format!("{:?}", spec.network.service_type),
            port,
        });
        if let Some(ext) = &spec.network.external_name {
            if !dry_run {
                deps.add_dependency(&spec.metadata.name, ext);
            }
            edges_added += 1;
        }
    }

    if !dry_run && edges_added > 0 {
        deps.save(&DependencyGraph::default_path())?;
    }

    Ok(K8sImportReport {
        generated_at: crate::resources::now_rfc3339(),
        services,
        edges_added,
        dry_run,
    })
}

// ── Phase 49: Threat propagation paths ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatPath {
    pub path: Vec<String>,
    pub severity: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatPathsReport {
    pub generated_at: String,
    pub paths: Vec<ThreatPath>,
}

pub fn build_threat_paths(state_path: &Path) -> anyhow::Result<ThreatPathsReport> {
    let graph = build_knowledge_graph(state_path)?;
    let pairs = load_pairs(state_path)?;
    let threats = SecurityEngine::scan_fleet(&pairs);
    let deps = DependencyGraph::load(&DependencyGraph::default_path()).unwrap_or_default();

    let mut paths = Vec::new();
    for threat in &threats.threats {
        let mut chain = vec![format!("threat:{}", threat.category)];
        chain.push(format!("wl:{}", threat.workload));
        for dep in deps.dependents_of(&threat.workload).into_iter().take(4) {
            chain.push(format!("wl:{dep}"));
        }
        paths.push(ThreatPath {
            path: chain.clone(),
            severity: threat.severity.clone(),
            summary: format!(
                "{} on {} may propagate to {} dependent(s)",
                threat.category,
                threat.workload,
                deps.dependents_of(&threat.workload).len()
            ),
        });
    }

    if paths.is_empty() && !graph.nodes.is_empty() {
        paths.push(ThreatPath {
            path: vec!["fleet".into(), "no-active-threats".into()],
            severity: "info".into(),
            summary: "No threat nodes in graph — fleet posture clear".into(),
        });
    }

    Ok(ThreatPathsReport {
        generated_at: crate::resources::now_rfc3339(),
        paths,
    })
}

// ── Phase 50: Graph search ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSearchHit {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub score: f64,
    pub route: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSearchReport {
    pub generated_at: String,
    pub query: String,
    pub hits: Vec<GraphSearchHit>,
}

pub fn search_graph(state_path: &Path, query: &str) -> anyhow::Result<GraphSearchReport> {
    let graph = build_knowledge_graph(state_path)?;
    let q = query.trim().to_lowercase();
    let mut hits: Vec<GraphSearchHit> = graph
        .nodes
        .iter()
        .filter_map(|n| {
            if q.is_empty() {
                return None;
            }
            let label = n.label.to_lowercase();
            let id = n.id.to_lowercase();
            let kind = n.kind.to_lowercase();
            let score = if label == q || id == q {
                1.0
            } else if label.contains(&q) || id.contains(&q) {
                0.75
            } else if kind.contains(&q) {
                0.5
            } else {
                return None;
            };
            let route = if n.id.starts_with("wl:") {
                format!(
                    "/workloads?workload={}",
                    n.id.strip_prefix("wl:").unwrap_or("")
                )
            } else {
                "/labs".into()
            };
            Some(GraphSearchHit {
                id: n.id.clone(),
                label: n.label.clone(),
                kind: n.kind.clone(),
                score,
                route,
            })
        })
        .collect();
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    hits.truncate(12);

    Ok(GraphSearchReport {
        generated_at: crate::resources::now_rfc3339(),
        query: query.into(),
        hits,
    })
}

// ── Phase 51: Historical graph snapshots ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshot {
    pub id: String,
    pub captured_at: String,
    pub label: String,
    pub node_count: u32,
    pub edge_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct GraphSnapshotStore {
    snapshots: Vec<StoredGraphSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredGraphSnapshot {
    pub id: String,
    pub captured_at: String,
    pub label: String,
    pub graph: crate::intelligence::graph::KnowledgeGraphReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshotsReport {
    pub generated_at: String,
    pub snapshots: Vec<GraphSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshotCaptureRequest {
    pub label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphSnapshotCaptureReport {
    pub captured: GraphSnapshot,
}

fn load_snapshot_store() -> GraphSnapshotStore {
    crate::resources::json_load(&snapshot_path()).unwrap_or_default()
}

fn save_snapshot_store(store: &GraphSnapshotStore) -> anyhow::Result<()> {
    crate::resources::json_save(store, &snapshot_path())
}

pub fn list_graph_snapshots() -> GraphSnapshotsReport {
    let store = load_snapshot_store();
    GraphSnapshotsReport {
        generated_at: crate::resources::now_rfc3339(),
        snapshots: store
            .snapshots
            .iter()
            .rev()
            .map(|s| GraphSnapshot {
                id: s.id.clone(),
                captured_at: s.captured_at.clone(),
                label: s.label.clone(),
                node_count: s.graph.nodes.len() as u32,
                edge_count: s.graph.edges.len() as u32,
            })
            .collect(),
    }
}

pub fn capture_graph_snapshot(
    state_path: &Path,
    label: Option<&str>,
) -> anyhow::Result<GraphSnapshotCaptureReport> {
    let graph = build_knowledge_graph(state_path)?;
    let id = format!("snap-{}", uuid_simple());
    let captured_at = crate::resources::now_rfc3339();
    let label = label
        .map(|s| s.to_string())
        .unwrap_or_else(|| "Manual capture".into());

    let mut store = load_snapshot_store();
    store.snapshots.push(StoredGraphSnapshot {
        id: id.clone(),
        captured_at: captured_at.clone(),
        label: label.clone(),
        graph,
    });
    if store.snapshots.len() > 50 {
        let drain = store.snapshots.len() - 50;
        store.snapshots.drain(..drain);
    }
    save_snapshot_store(&store)?;

    Ok(GraphSnapshotCaptureReport {
        captured: GraphSnapshot {
            id,
            captured_at,
            label,
            node_count: store
                .snapshots
                .last()
                .map(|s| s.graph.nodes.len() as u32)
                .unwrap_or(0),
            edge_count: store
                .snapshots
                .last()
                .map(|s| s.graph.edges.len() as u32)
                .unwrap_or(0),
        },
    })
}

fn uuid_simple() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos:x}")
}

// ── Phase 52: CMDB sync ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdbItem {
    pub id: String,
    pub name: String,
    pub item_type: String,
    pub dependencies: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdbSyncReport {
    pub generated_at: String,
    pub source: String,
    pub items: Vec<CmdbItem>,
    pub synced_edges: u32,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmdbSyncRequest {
    #[serde(default = "default_dry_run")]
    pub dry_run: bool,
}

pub fn build_cmdb_inventory() -> CmdbSyncReport {
    let path = cmdb_path();
    let items: Vec<CmdbItem> = if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|c| serde_json::from_str(&c).ok())
            .unwrap_or_default()
    } else {
        vec![
            CmdbItem {
                id: "cmdb-api".into(),
                name: "api-gateway".into(),
                item_type: "service".into(),
                dependencies: vec!["cmdb-db".into()],
            },
            CmdbItem {
                id: "cmdb-db".into(),
                name: "postgres".into(),
                item_type: "database".into(),
                dependencies: vec![],
            },
        ]
    };

    CmdbSyncReport {
        generated_at: crate::resources::now_rfc3339(),
        source: if path.exists() {
            path.display().to_string()
        } else {
            "embedded-default".into()
        },
        items,
        synced_edges: 0,
        dry_run: true,
    }
}

pub fn sync_cmdb_to_graph(dry_run: bool) -> anyhow::Result<CmdbSyncReport> {
    let mut report = build_cmdb_inventory();
    report.dry_run = dry_run;
    if dry_run {
        report.synced_edges = report
            .items
            .iter()
            .map(|i| i.dependencies.len() as u32)
            .sum();
        return Ok(report);
    }

    let mut deps = DependencyGraph::load(&DependencyGraph::default_path()).unwrap_or_default();
    let mut count = 0u32;
    for item in &report.items {
        deps.add_workload(&item.name);
        for dep_name in &item.dependencies {
            deps.add_workload(dep_name);
            deps.add_dependency(&item.name, dep_name);
            count += 1;
        }
    }
    deps.save(&DependencyGraph::default_path())?;
    report.synced_edges = count;
    Ok(report)
}

// ── Phase 53: Graph-based placement ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPlacementEntry {
    pub workload: String,
    pub startup_order: u32,
    pub depends_on: Vec<String>,
    pub recommended_cluster: Option<String>,
    pub co_locate_with: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphPlacementReport {
    pub generated_at: String,
    pub entries: Vec<GraphPlacementEntry>,
}

pub async fn build_graph_placement(state_path: &Path) -> anyhow::Result<GraphPlacementReport> {
    let pairs = load_pairs(state_path)?;
    let deps = DependencyGraph::load(&DependencyGraph::default_path()).unwrap_or_default();
    let order = deps
        .startup_order()
        .unwrap_or_else(|_| pairs.iter().map(|(s, _)| s.metadata.name.clone()).collect());
    let clusters = list_clusters().await.unwrap_or_default();
    let default_cluster = clusters
        .iter()
        .find(|c| c.reachable)
        .map(|c| c.name.clone());

    let mut entries = Vec::new();
    for (idx, name) in order.iter().enumerate() {
        let depends = deps.dependencies_of(name);
        let co_locate = depends.first().cloned();
        entries.push(GraphPlacementEntry {
            workload: name.clone(),
            startup_order: (idx + 1) as u32,
            depends_on: depends,
            recommended_cluster: default_cluster.clone(),
            co_locate_with: co_locate,
        });
    }

    Ok(GraphPlacementReport {
        generated_at: crate::resources::now_rfc3339(),
        entries,
    })
}

// ── Phase 54: Graph export (Lab) ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphExportReport {
    pub generated_at: String,
    pub format: String,
    pub payload: String,
    pub node_count: u32,
    pub edge_count: u32,
}

pub fn export_graph(state_path: &Path, format: &str) -> anyhow::Result<GraphExportReport> {
    let graph = build_knowledge_graph(state_path)?;
    let fmt = format.to_lowercase();
    let payload = if fmt == "jsonld" {
        export_jsonld(&graph)
    } else {
        export_neo4j_cypher(&graph)
    };

    Ok(GraphExportReport {
        generated_at: crate::resources::now_rfc3339(),
        format: fmt,
        node_count: graph.nodes.len() as u32,
        edge_count: graph.edges.len() as u32,
        payload,
    })
}

fn export_neo4j_cypher(graph: &crate::intelligence::graph::KnowledgeGraphReport) -> String {
    let mut lines = vec![
        "// Aether Knowledge Graph — Neo4j import".into(),
        "CREATE CONSTRAINT IF NOT EXISTS FOR (n:AetherNode) REQUIRE n.id IS UNIQUE;".into(),
    ];
    for n in &graph.nodes {
        lines.push(format!(
            "MERGE (n:AetherNode {{id:'{}'}}) SET n.label='{}', n.kind='{}';",
            escape_cypher(&n.id),
            escape_cypher(&n.label),
            escape_cypher(&n.kind),
        ));
    }
    for e in &graph.edges {
        lines.push(format!(
            "MATCH (a:AetherNode {{id:'{}'}}), (b:AetherNode {{id:'{}'}}) MERGE (a)-[:{}]->(b);",
            escape_cypher(&e.from),
            escape_cypher(&e.to),
            escape_cypher(&e.kind.to_uppercase()),
        ));
    }
    lines.join("\n")
}

fn export_jsonld(graph: &crate::intelligence::graph::KnowledgeGraphReport) -> String {
    let nodes: Vec<_> = graph
        .nodes
        .iter()
        .map(|n| {
            serde_json::json!({
                "@id": n.id,
                "@type": n.kind,
                "label": n.label,
            })
        })
        .collect();
    let edges: Vec<_> = graph
        .edges
        .iter()
        .map(|e| {
            serde_json::json!({
                "@id": format!("{}->{}", e.from, e.to),
                "@type": e.kind,
                "from": e.from,
                "to": e.to,
            })
        })
        .collect();
    serde_json::to_string_pretty(&serde_json::json!({
        "@context": {
            "@vocab": "https://zyvor.dev/aether/graph#",
            "label": "rdfs:label",
        },
        "@graph": [nodes, edges],
    }))
    .unwrap_or_else(|_| "{}".into())
}

fn escape_cypher(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "\\'")
}

// ── Runtime Fabric topology (Command Center / Fabric graph) ─────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricTopologyNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricTopologyEdge {
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FabricTopologyReport {
    pub generated_at: String,
    pub nodes: Vec<FabricTopologyNode>,
    pub edges: Vec<FabricTopologyEdge>,
}

fn fabric_workload_pairs(store: &StateStore) -> Vec<(Workload, WorkloadState)> {
    store
        .list()
        .iter()
        .filter_map(|ws| {
            Workload::from_file(&ws.spec_path)
                .ok()
                .map(|spec| (spec, (*ws).clone()))
        })
        .collect()
}

pub fn build_runtime_fabric_topology(state_path: &Path) -> anyhow::Result<FabricTopologyReport> {
    let store = StateStore::load(state_path)?;
    let pairs = fabric_workload_pairs(&store);
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen = HashSet::new();

    let mut add_node = |node: FabricTopologyNode| {
        if seen.insert(node.id.clone()) {
            nodes.push(node);
        }
    };
    let mut add_edge = |from: String, to: String| {
        edges.push(FabricTopologyEdge { from, to });
    };

    for (spec, ws) in &pairs {
        let name = ws.name.clone();
        let runtime = ws.runtime.to_string();
        let cluster = "local".to_string();

        let app_id = format!("app:{name}");
        add_node(FabricTopologyNode {
            id: app_id.clone(),
            label: name.clone(),
            kind: "application".into(),
            sub: Some(spec.kind.clone()),
            workload: Some(name.clone()),
        });

        let rt_id = format!("rt:{name}:{runtime}");
        add_node(FabricTopologyNode {
            id: rt_id.clone(),
            label: runtime.clone(),
            kind: "runtime".into(),
            sub: Some("Runtime".into()),
            workload: Some(name.clone()),
        });
        add_edge(app_id, rt_id.clone());

        let cluster_id = format!("cluster:{cluster}");
        add_node(FabricTopologyNode {
            id: cluster_id.clone(),
            label: cluster.clone(),
            kind: "cluster".into(),
            sub: Some("Cluster".into()),
            workload: None,
        });
        add_edge(rt_id.clone(), cluster_id.clone());

        let node_id = format!("node:{cluster}:{name}");
        add_node(FabricTopologyNode {
            id: node_id.clone(),
            label: name.clone(),
            kind: "node".into(),
            sub: Some("Node".into()),
            workload: Some(name.clone()),
        });
        add_edge(cluster_id, node_id.clone());

        for (res, icon) in [
            ("CPU", "cpu"),
            ("Memory", "mem"),
            ("Network", "nic"),
            ("Storage", "disk"),
        ] {
            let res_id = format!("res:{name}:{icon}");
            add_node(FabricTopologyNode {
                id: res_id.clone(),
                label: res.into(),
                kind: "resource".into(),
                sub: Some(icon.to_uppercase()),
                workload: Some(name.clone()),
            });
            add_edge(node_id.clone(), res_id);
        }

        let rt_lower = runtime.to_lowercase();
        if rt_lower.contains("virt") {
            let gpu_id = format!("res:{name}:gpu");
            add_node(FabricTopologyNode {
                id: gpu_id.clone(),
                label: "GPU".into(),
                kind: "resource".into(),
                sub: Some("Accelerator".into()),
                workload: Some(name.clone()),
            });
            add_edge(node_id, gpu_id);
        }
    }

    Ok(FabricTopologyReport {
        generated_at: crate::resources::now_rfc3339(),
        nodes,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_graph_filter() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let g = build_interactive_graph(&path, &["depends_on".into()], &[]).unwrap();
        assert!(g.edges.iter().all(|e| e.kind == "depends_on"));
    }

    #[test]
    fn graph_search_empty_query() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let r = search_graph(&path, "workload").unwrap();
        assert!(r.hits.is_empty());
    }

    #[test]
    fn export_neo4j_format() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let r = export_graph(&path, "neo4j").unwrap();
        assert!(r.payload.contains("Aether Knowledge Graph"));
    }

    #[test]
    fn cmdb_default_inventory() {
        let r = build_cmdb_inventory();
        assert!(!r.items.is_empty());
    }

    #[tokio::test]
    async fn graph_placement_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let r = build_graph_placement(&path).await.unwrap();
        assert!(r.entries.is_empty());
    }
}
