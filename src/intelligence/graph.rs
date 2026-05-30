// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Infrastructure knowledge graph — workloads, dependencies, threats, drift.

use crate::dependencies::DependencyGraph;
use crate::drift::fleet::scan_fleet;
use crate::intelligence::security::SecurityEngine;
use crate::spec::Workload;
use crate::state::{StateStore, WorkloadState};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub severity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphEdge {
    pub from: String,
    pub to: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KnowledgeGraphStats {
    pub workloads: u32,
    pub dependencies: u32,
    pub threats: u32,
    pub drifted: u32,
    pub clusters: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphReport {
    pub generated_at: String,
    pub nodes: Vec<KnowledgeGraphNode>,
    pub edges: Vec<KnowledgeGraphEdge>,
    pub stats: KnowledgeGraphStats,
}

pub fn build_knowledge_graph(state_path: &Path) -> anyhow::Result<KnowledgeGraphReport> {
    let store = StateStore::load(state_path)?;
    let pairs: Vec<(Workload, WorkloadState)> = store
        .list()
        .iter()
        .filter_map(|ws| {
            Workload::from_file(&ws.spec_path)
                .ok()
                .map(|s| (s, (*ws).clone()))
        })
        .collect();

    let threats = SecurityEngine::scan_fleet(&pairs);
    let drift = scan_fleet(&store);
    let deps = DependencyGraph::load(&DependencyGraph::default_path()).unwrap_or_default();

    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut seen_nodes = HashSet::new();
    let mut clusters = HashSet::new();

    let add_node = |nodes: &mut Vec<KnowledgeGraphNode>, seen: &mut HashSet<String>, node: KnowledgeGraphNode| {
        if seen.insert(node.id.clone()) {
            nodes.push(node);
        }
    };

    for (spec, ws) in &pairs {
        let wl_id = format!("wl:{}", spec.metadata.name);
        add_node(
            &mut nodes,
            &mut seen_nodes,
            KnowledgeGraphNode {
                id: wl_id.clone(),
                label: spec.metadata.name.clone(),
                kind: "workload".into(),
                severity: None,
            },
        );

        let rt_id = format!("rt:{}", ws.runtime);
        add_node(
            &mut nodes,
            &mut seen_nodes,
            KnowledgeGraphNode {
                id: rt_id.clone(),
                label: format!("{}", ws.runtime),
                kind: "runtime".into(),
                severity: None,
            },
        );
        edges.push(KnowledgeGraphEdge {
            from: wl_id.clone(),
            to: rt_id,
            kind: "runs_on".into(),
        });

        let cluster_name = format!("{}", ws.runtime);
        clusters.insert(cluster_name.clone());
        let cl_id = format!("cluster:{cluster_name}");
        add_node(
            &mut nodes,
            &mut seen_nodes,
            KnowledgeGraphNode {
                id: cl_id.clone(),
                label: cluster_name,
                kind: "cluster".into(),
                severity: None,
            },
        );
        edges.push(KnowledgeGraphEdge {
            from: wl_id,
            to: cl_id,
            kind: "placed_on".into(),
        });
    }

    for (from, to) in deps.dependency_edges() {
        edges.push(KnowledgeGraphEdge {
            from: format!("wl:{from}"),
            to: format!("wl:{to}"),
            kind: "depends_on".into(),
        });
        add_node(
            &mut nodes,
            &mut seen_nodes,
            KnowledgeGraphNode {
                id: format!("wl:{from}"),
                label: from,
                kind: "workload".into(),
                severity: None,
            },
        );
        add_node(
            &mut nodes,
            &mut seen_nodes,
            KnowledgeGraphNode {
                id: format!("wl:{to}"),
                label: to,
                kind: "workload".into(),
                severity: None,
            },
        );
    }

    for threat in &threats.threats {
        let th_id = format!("threat:{}:{}", threat.workload, threat.category);
        add_node(
            &mut nodes,
            &mut seen_nodes,
            KnowledgeGraphNode {
                id: th_id.clone(),
                label: threat.category.clone(),
                kind: "threat".into(),
                severity: Some(threat.severity.clone()),
            },
        );
        edges.push(KnowledgeGraphEdge {
            from: th_id,
            to: format!("wl:{}", threat.workload),
            kind: "threatens".into(),
        });
    }

    for row in drift.rows.iter().filter(|r| r.has_drift) {
        let dr_id = format!("drift:{}", row.workload);
        add_node(
            &mut nodes,
            &mut seen_nodes,
            KnowledgeGraphNode {
                id: dr_id.clone(),
                label: "drift".into(),
                kind: "drift".into(),
                severity: Some(format!("{:?}", row.severity).to_lowercase()),
            },
        );
        edges.push(KnowledgeGraphEdge {
            from: dr_id,
            to: format!("wl:{}", row.workload),
            kind: "drifts_from".into(),
        });
    }

    let stats = KnowledgeGraphStats {
        workloads: pairs.len() as u32,
        dependencies: edges.iter().filter(|e| e.kind == "depends_on").count() as u32,
        threats: threats.threats.len() as u32,
        drifted: drift.drifted as u32,
        clusters: clusters.len() as u32,
    };

    Ok(KnowledgeGraphReport {
        generated_at: crate::resources::now_rfc3339(),
        nodes,
        edges,
        stats,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn knowledge_graph_empty_fleet() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        StateStore::new().save(&path).unwrap();
        let graph = build_knowledge_graph(&path).unwrap();
        assert_eq!(graph.stats.workloads, 0);
    }
}
