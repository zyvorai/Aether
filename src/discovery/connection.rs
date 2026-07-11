// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Saved cluster **connections** for discovery and assessment.
//!
//! A connection is a friendly name over a kubeconfig context plus metadata about
//! the source platform. All connections reach the cluster through the kubeconfig
//! (EKS/AKS/GKE kubeconfigs already carry cloud auth via exec-plugins), so no
//! cloud SDK credentials are stored here.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// Source-platform label for a connection. Informational only — every kind
/// reaches the cluster via its kubeconfig context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionKind {
    Eks,
    Aks,
    Gke,
    Openshift,
    Rancher,
    Tanzu,
    Kubernetes,
    K3s,
    Rke2,
    Podman,
    Compose,
}

impl ConnectionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ConnectionKind::Eks => "eks",
            ConnectionKind::Aks => "aks",
            ConnectionKind::Gke => "gke",
            ConnectionKind::Openshift => "openshift",
            ConnectionKind::Rancher => "rancher",
            ConnectionKind::Tanzu => "tanzu",
            ConnectionKind::Kubernetes => "kubernetes",
            ConnectionKind::K3s => "k3s",
            ConnectionKind::Rke2 => "rke2",
            ConnectionKind::Podman => "podman",
            ConnectionKind::Compose => "compose",
        }
    }
}

impl std::str::FromStr for ConnectionKind {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self> {
        Ok(match s.to_lowercase().as_str() {
            "eks" => ConnectionKind::Eks,
            "aks" => ConnectionKind::Aks,
            "gke" => ConnectionKind::Gke,
            "openshift" | "ocp" => ConnectionKind::Openshift,
            "rancher" => ConnectionKind::Rancher,
            "tanzu" | "tkg" => ConnectionKind::Tanzu,
            "kubernetes" | "kube" | "k8s" => ConnectionKind::Kubernetes,
            "k3s" => ConnectionKind::K3s,
            "rke2" => ConnectionKind::Rke2,
            "podman" => ConnectionKind::Podman,
            "compose" => ConnectionKind::Compose,
            other => anyhow::bail!(
                "unknown connection kind '{}'. Valid: eks, aks, gke, openshift, rancher, \
                 tanzu, kubernetes, k3s, rke2, podman, compose",
                other
            ),
        })
    }
}

impl std::fmt::Display for ConnectionKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A saved connection to a source or target cluster.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub name: String,
    pub kind: ConnectionKind,
    /// Kubeconfig context to use. Empty means "current-context".
    #[serde(default)]
    pub context: String,
    /// Optional explicit kubeconfig file (defaults to the ambient KUBECONFIG).
    #[serde(default)]
    pub kubeconfig: Option<PathBuf>,
    pub created_at: String,
}

impl Connection {
    /// Build a Kubernetes client for this connection, honouring an explicit
    /// kubeconfig path when set. Returns the client and the apiserver version.
    pub async fn connect(&self) -> Result<(kube::Client, Option<String>)> {
        // A custom kubeconfig is applied via the KUBECONFIG env var, which
        // `Kubeconfig::read()` (used by client_for_cluster) honours. The CLI is
        // single-threaded per invocation, so this is safe here.
        if let Some(path) = &self.kubeconfig {
            std::env::set_var("KUBECONFIG", path);
        }
        let ctx = if self.context.is_empty() {
            crate::kubecluster::resolve_reachable_cluster(None).await?
        } else {
            self.context.clone()
        };
        let client = crate::kubecluster::client_for_cluster(&ctx)
            .await
            .with_context(|| format!("connecting to context '{}'", ctx))?;
        let version = client
            .apiserver_version()
            .await
            .ok()
            .map(|info| info.git_version);
        Ok((client, version))
    }
}

/// Persistent store of named connections (`~/.aether/connections.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ConnectionStore {
    #[serde(default)]
    pub connections: BTreeMap<String, Connection>,
}

crate::impl_json_store!(ConnectionStore, "connections.json");

impl ConnectionStore {
    pub fn upsert(&mut self, conn: Connection) {
        self.connections.insert(conn.name.clone(), conn);
    }

    pub fn get(&self, name: &str) -> Option<&Connection> {
        self.connections.get(name)
    }

    pub fn remove(&mut self, name: &str) -> Option<Connection> {
        self.connections.remove(name)
    }

    pub fn list(&self) -> Vec<&Connection> {
        self.connections.values().collect()
    }

    /// Load the default store, or an empty one if none exists.
    pub fn load_default() -> Result<Self> {
        Self::load(&Self::default_path())
    }

    pub fn save_default(&self) -> Result<()> {
        self.save(&Self::default_path())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_kind_roundtrip() {
        for s in ["eks", "aks", "gke", "kubernetes", "k3s", "rke2", "podman"] {
            let k: ConnectionKind = s.parse().unwrap();
            assert_eq!(k.as_str(), s);
        }
        assert_eq!("k8s".parse::<ConnectionKind>().unwrap(), ConnectionKind::Kubernetes);
        assert!("bogus".parse::<ConnectionKind>().is_err());
    }

    #[test]
    fn test_store_crud() {
        let mut store = ConnectionStore::default();
        store.upsert(Connection {
            name: "prod".to_string(),
            kind: ConnectionKind::Eks,
            context: "arn:aws:eks:...".to_string(),
            kubeconfig: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        });
        assert_eq!(store.list().len(), 1);
        assert_eq!(store.get("prod").unwrap().kind, ConnectionKind::Eks);
        assert!(store.remove("prod").is_some());
        assert!(store.get("prod").is_none());
    }
}
