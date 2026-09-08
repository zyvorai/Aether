// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Persisted per-connection inventory snapshot.

use crate::discovery::kubernetes::RawInventory;
use crate::inventory::application::{group_into_applications, Application};
use crate::inventory::dependency::{discover_dependencies, DependencyEdges};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// A discovery snapshot for one connection: the raw objects plus the derived
/// applications and dependency edges. Persisted so assess/report are fast and
/// repeatable without re-hitting the cluster.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InventorySnapshot {
    pub connection: String,
    pub discovered_at: String,
    pub raw: RawInventory,
    #[serde(default)]
    pub applications: Vec<Application>,
    #[serde(default)]
    pub dependencies: DependencyEdges,
}

impl InventorySnapshot {
    /// Build a snapshot from a freshly-discovered raw inventory (groups apps +
    /// derives dependencies).
    pub fn from_raw(raw: RawInventory) -> Self {
        let applications = group_into_applications(&raw);
        let dependencies = discover_dependencies(&raw);
        Self {
            connection: raw.connection.clone(),
            discovered_at: raw.discovered_at.clone(),
            raw,
            applications,
            dependencies,
        }
    }

    /// On-disk path for a connection's snapshot (`~/.aether/inventory-<conn>.json`).
    pub fn path_for(connection: &str) -> PathBuf {
        let safe: String = connection
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        crate::resources::aether_path(&format!("inventory-{safe}.json"))
    }

    pub fn load_for(connection: &str) -> Result<Self> {
        let path = Self::path_for(connection);
        if !path.exists() {
            anyhow::bail!("no inventory snapshot at {}", path.display());
        }
        crate::resources::json_load(&path)
    }

    pub fn save(&self) -> Result<()> {
        crate::resources::json_save(self, &Self::path_for(&self.connection))
    }

    pub fn find_application(&self, name: &str) -> Option<&Application> {
        self.applications
            .iter()
            .find(|a| a.name == name || a.id == name)
    }
}
