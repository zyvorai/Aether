// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Workload dependency graph
//!
//! Tracks inter-workload dependencies for ordered startup, cascade
//! stop/delete, and impact analysis.

use crate::output;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

/// Dependency graph for workloads
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DependencyGraph {
    /// Edges: key depends on values (key requires values to be running)
    edges: HashMap<String, HashSet<String>>,
    /// All known workloads
    nodes: HashSet<String>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a workload to the graph
    pub fn add_workload(&mut self, name: &str) {
        self.nodes.insert(name.to_string());
        self.edges.entry(name.to_string()).or_default();
    }

    /// Declare that `workload` depends on `dependency`
    pub fn add_dependency(&mut self, workload: &str, dependency: &str) {
        self.nodes.insert(workload.to_string());
        self.nodes.insert(dependency.to_string());
        self.edges
            .entry(workload.to_string())
            .or_default()
            .insert(dependency.to_string());
    }

    /// Remove a dependency
    pub fn remove_dependency(&mut self, workload: &str, dependency: &str) {
        if let Some(deps) = self.edges.get_mut(workload) {
            deps.remove(dependency);
        }
    }

    /// Remove a workload and all its edges
    pub fn remove_workload(&mut self, name: &str) {
        self.nodes.remove(name);
        self.edges.remove(name);
        // Remove as a dependency from all others
        for deps in self.edges.values_mut() {
            deps.remove(name);
        }
    }

    /// Get direct dependencies of a workload
    pub fn dependencies_of(&self, workload: &str) -> Vec<String> {
        self.edges
            .get(workload)
            .map(|deps| deps.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Get workloads that depend on a given workload (reverse dependencies)
    pub fn dependents_of(&self, workload: &str) -> Vec<String> {
        self.edges
            .iter()
            .filter(|(_, deps)| deps.contains(workload))
            .map(|(name, _)| name.clone())
            .collect()
    }

    /// Topological sort for startup order (dependencies first)
    pub fn startup_order(&self) -> Result<Vec<String>, CycleError> {
        self.topological_sort()
    }

    /// Reverse topological sort for shutdown order (dependents first)
    pub fn shutdown_order(&self) -> Result<Vec<String>, CycleError> {
        let mut order = self.topological_sort()?;
        order.reverse();
        Ok(order)
    }

    /// Get all workloads that would be affected by stopping a workload
    pub fn impact_analysis(&self, workload: &str) -> ImpactReport {
        let mut affected = Vec::new();
        let mut visited = HashSet::new();
        self.collect_dependents(workload, &mut affected, &mut visited);

        let cascade_count = affected.len();
        let severity = if cascade_count == 0 {
            ImpactSeverity::None
        } else if cascade_count <= 2 {
            ImpactSeverity::Low
        } else if cascade_count <= 5 {
            ImpactSeverity::Medium
        } else {
            ImpactSeverity::High
        };

        ImpactReport {
            workload: workload.to_string(),
            affected_workloads: affected,
            cascade_count,
            severity,
        }
    }

    /// Check if the graph has cycles
    pub fn has_cycle(&self) -> bool {
        self.topological_sort().is_err()
    }

    /// Validate the dependency graph
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();

        // Check for cycles
        if self.has_cycle() {
            issues.push("Circular dependency detected".to_string());
        }

        // Check for missing dependencies
        for (workload, deps) in &self.edges {
            for dep in deps {
                if !self.nodes.contains(dep) {
                    issues.push(format!(
                        "'{}' depends on '{}' which is not registered",
                        workload, dep
                    ));
                }
            }
        }

        issues
    }

    /// All workload node names in the graph.
    pub fn node_names(&self) -> Vec<String> {
        self.nodes.iter().cloned().collect()
    }

    /// Directed dependency edges `(workload, depends_on)`.
    pub fn dependency_edges(&self) -> Vec<(String, String)> {
        self.edges
            .iter()
            .flat_map(|(workload, deps)| {
                deps.iter().map(move |dep| (workload.clone(), dep.clone()))
            })
            .collect()
    }

    /// Get graph statistics
    pub fn stats(&self) -> GraphStats {
        let total_workloads = self.nodes.len();
        let total_edges: usize = self.edges.values().map(|deps| deps.len()).sum();
        let root_workloads = self
            .nodes
            .iter()
            .filter(|n| self.dependencies_of(n).is_empty())
            .count();
        let leaf_workloads = self
            .nodes
            .iter()
            .filter(|n| self.dependents_of(n).is_empty())
            .count();

        let max_depth = self.max_depth();

        GraphStats {
            total_workloads,
            total_edges,
            root_workloads,
            leaf_workloads,
            max_depth,
            has_cycles: self.has_cycle(),
        }
    }

    // --- Private methods ---

    fn topological_sort(&self) -> Result<Vec<String>, CycleError> {
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        for node in &self.nodes {
            in_degree.entry(node.clone()).or_insert(0);
        }
        // Kahn's algorithm
        // In our graph: edges[A] contains B means "A depends on B"
        // For startup: B must come before A
        // in_degree[A] = number of things A depends on
        for (node, deps) in &self.edges {
            in_degree.entry(node.clone()).or_insert(0);
            for dep in deps {
                in_degree.entry(dep.clone()).or_insert(0);
            }
            *in_degree.entry(node.clone()).or_insert(0) = deps.len();
        }

        let mut queue: VecDeque<String> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();

        // Sort queue for deterministic output
        let mut sorted_queue: Vec<String> = queue.drain(..).collect();
        sorted_queue.sort();
        queue.extend(sorted_queue);

        let mut result = Vec::new();

        while let Some(node) = queue.pop_front() {
            result.push(node.clone());

            // Find all nodes that depend on this node and decrement their in-degree
            for (dependent, deps) in &self.edges {
                if deps.contains(&node) {
                    if let Some(deg) = in_degree.get_mut(dependent) {
                        *deg -= 1;
                        if *deg == 0 {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }

        if result.len() != self.nodes.len() {
            Err(CycleError {
                message: "Circular dependency detected in workload graph".to_string(),
            })
        } else {
            Ok(result)
        }
    }

    fn collect_dependents(
        &self,
        workload: &str,
        affected: &mut Vec<String>,
        visited: &mut HashSet<String>,
    ) {
        for dependent in self.dependents_of(workload) {
            if visited.insert(dependent.clone()) {
                affected.push(dependent.clone());
                self.collect_dependents(&dependent, affected, visited);
            }
        }
    }

    fn max_depth(&self) -> usize {
        if let Ok(order) = self.startup_order() {
            let mut depths: HashMap<String, usize> = HashMap::new();
            for node in &order {
                let max_dep_depth = self
                    .dependencies_of(node)
                    .iter()
                    .filter_map(|dep| depths.get(dep))
                    .max()
                    .copied()
                    .unwrap_or(0);
                depths.insert(node.clone(), max_dep_depth + 1);
            }
            depths.values().max().copied().unwrap_or(0)
        } else {
            0
        }
    }
}

/// Error when a cycle is detected
#[derive(Debug, Clone)]
pub struct CycleError {
    pub message: String,
}

impl std::fmt::Display for CycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for CycleError {}

/// Impact analysis report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactReport {
    pub workload: String,
    pub affected_workloads: Vec<String>,
    pub cascade_count: usize,
    pub severity: ImpactSeverity,
}

/// Impact severity
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImpactSeverity {
    None,
    Low,
    Medium,
    High,
}

impl std::fmt::Display for ImpactSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImpactSeverity::None => write!(f, "None"),
            ImpactSeverity::Low => write!(f, "Low"),
            ImpactSeverity::Medium => write!(f, "Medium"),
            ImpactSeverity::High => write!(f, "High"),
        }
    }
}

/// Graph statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_workloads: usize,
    pub total_edges: usize,
    pub root_workloads: usize,
    pub leaf_workloads: usize,
    pub max_depth: usize,
    pub has_cycles: bool,
}

/// Format dependency graph as a report
pub fn format_dependency_report(graph: &DependencyGraph) -> String {
    let mut output = String::new();
    let stats = graph.stats();

    output.push_str(&output::property_section(&[
        ("Dependency Graph", String::new()),
        ("Workloads", format!("{}", stats.total_workloads)),
        ("Dependencies", format!("{}", stats.total_edges)),
        ("Max depth", format!("{}", stats.max_depth)),
        (
            "Cycles",
            if stats.has_cycles {
                "YES".to_string()
            } else {
                "none".to_string()
            },
        ),
    ]));

    if let Ok(order) = graph.startup_order() {
        output.push_str("Startup Order:\n");
        for (i, name) in order.iter().enumerate() {
            let deps = graph.dependencies_of(name);
            if deps.is_empty() {
                output.push_str(&format!("  {}. {} (root)\n", i + 1, name));
            } else {
                output.push_str(&format!(
                    "  {}. {} -> depends on: {}\n",
                    i + 1,
                    name,
                    deps.join(", ")
                ));
            }
        }
    }

    let issues = graph.validate();
    if !issues.is_empty() {
        output.push_str("\nIssues:\n");
        for issue in &issues {
            output.push_str(&output::tree_bullet("⚠", issue));
        }
    }

    output
}

crate::impl_json_store!(DependencyGraph, "dependencies.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_dependency() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("web", "database");
        graph.add_dependency("web", "cache");

        let deps = graph.dependencies_of("web");
        assert_eq!(deps.len(), 2);
        assert!(deps.contains(&"database".to_string()));
        assert!(deps.contains(&"cache".to_string()));
    }

    #[test]
    fn test_startup_order() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("web", "api");
        graph.add_dependency("api", "database");
        graph.add_dependency("api", "cache");

        let order = graph.startup_order().unwrap();
        let db_pos = order.iter().position(|n| n == "database").unwrap();
        let cache_pos = order.iter().position(|n| n == "cache").unwrap();
        let api_pos = order.iter().position(|n| n == "api").unwrap();
        let web_pos = order.iter().position(|n| n == "web").unwrap();

        // database and cache must come before api
        assert!(db_pos < api_pos);
        assert!(cache_pos < api_pos);
        // api must come before web
        assert!(api_pos < web_pos);
    }

    #[test]
    fn test_shutdown_order() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("web", "api");
        graph.add_dependency("api", "database");

        let order = graph.shutdown_order().unwrap();
        let web_pos = order.iter().position(|n| n == "web").unwrap();
        let api_pos = order.iter().position(|n| n == "api").unwrap();
        let db_pos = order.iter().position(|n| n == "database").unwrap();

        // web should stop first, then api, then database
        assert!(web_pos < api_pos);
        assert!(api_pos < db_pos);
    }

    #[test]
    fn test_cycle_detection() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("a", "b");
        graph.add_dependency("b", "c");
        graph.add_dependency("c", "a");

        assert!(graph.has_cycle());
        assert!(graph.startup_order().is_err());
    }

    #[test]
    fn test_impact_analysis() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("web", "api");
        graph.add_dependency("worker", "api");
        graph.add_dependency("api", "database");

        let impact = graph.impact_analysis("database");
        assert!(impact.affected_workloads.contains(&"api".to_string()));
        assert!(impact.cascade_count >= 2); // api, web, and worker depend on database
    }

    #[test]
    fn test_graph_stats() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("web", "api");
        graph.add_dependency("api", "database");

        let stats = graph.stats();
        assert_eq!(stats.total_workloads, 3);
        assert!(!stats.has_cycles);
    }

    #[test]
    fn test_remove_workload() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("web", "api");
        graph.add_dependency("api", "database");

        graph.remove_workload("api");
        assert!(graph.dependencies_of("web").is_empty());
    }

    #[test]
    fn test_format_report() {
        let mut graph = DependencyGraph::new();
        graph.add_dependency("web", "api");
        graph.add_dependency("api", "database");
        let report = format_dependency_report(&graph);
        assert!(report.contains("Startup Order"));
    }
}
