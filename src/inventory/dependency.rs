// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Static dependency discovery from a raw inventory.
//!
//! Edges are derived from Kubernetes wiring only (Ingress→Service→workload,
//! mounts, env endpoints). Flow-observed dependencies (PacketWolf) are a later
//! enrichment. Nodes are namespace-qualified (`{ns}/{name}`) to avoid the
//! cross-namespace collisions that bare names would cause.

use crate::discovery::kubernetes::RawInventory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Routes,   // ingress → service
    Selects,  // service → workload
    Config,   // workload → configmap
    Secret,   // workload → secret
    Storage,  // workload → pvc
    External, // workload → external endpoint
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepEdge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyEdges {
    pub edges: Vec<DepEdge>,
}

fn q(ns: &str, kind: &str, name: &str) -> String {
    format!("{ns}/{kind}:{name}")
}

/// Build the static dependency edge set for a raw inventory.
pub fn discover_dependencies(inv: &RawInventory) -> DependencyEdges {
    let mut edges = Vec::new();

    // Ingress → Service.
    for ing in &inv.ingresses {
        for svc in &ing.backend_services {
            edges.push(DepEdge {
                from: q(&ing.namespace, "Ingress", &ing.name),
                to: q(&ing.namespace, "Service", svc),
                kind: EdgeKind::Routes,
            });
        }
    }

    // Service → Workload (selector subset of workload labels).
    for svc in &inv.services {
        if svc.selector.is_empty() {
            continue;
        }
        for w in inv
            .workloads
            .iter()
            .filter(|w| w.namespace == svc.namespace)
        {
            let matches: BTreeMap<_, _> = w.labels.iter().collect();
            if svc.selector.iter().all(|(k, v)| matches.get(k) == Some(&v)) {
                edges.push(DepEdge {
                    from: q(&svc.namespace, "Service", &svc.name),
                    to: q(&w.namespace, &w.kind, &w.name),
                    kind: EdgeKind::Selects,
                });
            }
        }
    }

    // Workload → mounts + external endpoints.
    for w in &inv.workloads {
        let from = q(&w.namespace, &w.kind, &w.name);
        for cm in &w.config_map_refs {
            edges.push(DepEdge {
                from: from.clone(),
                to: q(&w.namespace, "ConfigMap", cm),
                kind: EdgeKind::Config,
            });
        }
        for s in &w.secret_refs {
            edges.push(DepEdge {
                from: from.clone(),
                to: q(&w.namespace, "Secret", s),
                kind: EdgeKind::Secret,
            });
        }
        for p in &w.pvc_refs {
            edges.push(DepEdge {
                from: from.clone(),
                to: q(&w.namespace, "PVC", p),
                kind: EdgeKind::Storage,
            });
        }
        for e in &w.env_endpoints {
            edges.push(DepEdge {
                from: from.clone(),
                to: format!("external:{e}"),
                kind: EdgeKind::External,
            });
        }
    }

    DependencyEdges { edges }
}

impl DependencyEdges {
    /// Edges whose source node is within the given namespace/name prefix set
    /// (used to show one application's dependencies).
    pub fn for_prefixes(&self, prefixes: &[String]) -> Vec<&DepEdge> {
        self.edges
            .iter()
            .filter(|e| {
                prefixes
                    .iter()
                    .any(|p| e.from.starts_with(p) || e.to.starts_with(p))
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::kubernetes::{DiscoveredWorkload, IngressInfo, PodSignals, ServiceInfo};
    use std::collections::BTreeMap;

    #[test]
    fn test_ingress_service_workload_chain() {
        let mut labels = BTreeMap::new();
        labels.insert("app".to_string(), "web".to_string());
        let mut selector = BTreeMap::new();
        selector.insert("app".to_string(), "web".to_string());

        let inv = RawInventory {
            workloads: vec![DiscoveredWorkload {
                namespace: "prod".into(),
                kind: "Deployment".into(),
                name: "web".into(),
                labels,
                annotations: BTreeMap::new(),
                owner_refs: vec![],
                replicas: 1,
                containers: vec![],
                signals: PodSignals::default(),
                config_map_refs: vec!["web-config".into()],
                secret_refs: vec![],
                pvc_refs: vec![],
                env_endpoints: vec!["https://api.stripe.com".into()],
                manifest: None,
            }],
            services: vec![ServiceInfo {
                namespace: "prod".into(),
                name: "web".into(),
                labels: BTreeMap::new(),
                selector,
                type_: "ClusterIP".into(),
                manifest: None,
            }],
            ingresses: vec![IngressInfo {
                namespace: "prod".into(),
                name: "web-ing".into(),
                hosts: vec!["shop.example.com".into()],
                backend_services: vec!["web".into()],
            }],
            ..Default::default()
        };
        let deps = discover_dependencies(&inv);
        assert!(deps.edges.iter().any(|e| e.kind == EdgeKind::Routes));
        assert!(deps.edges.iter().any(|e| e.kind == EdgeKind::Selects));
        assert!(deps.edges.iter().any(|e| e.kind == EdgeKind::Config));
        assert!(deps.edges.iter().any(|e| e.kind == EdgeKind::External));
    }
}
