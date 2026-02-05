//! Local state store for tracking workload instances

use crate::runtime::{Instance, RuntimeKind};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Local state database
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StateStore {
    pub workloads: HashMap<String, WorkloadState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadState {
    pub name: String,
    pub runtime: RuntimeKind,
    pub instance: Instance,
    pub spec_path: PathBuf,
    pub created_at: String,
    pub updated_at: String,
}

impl StateStore {
    /// Create new state store
    pub fn new() -> Self {
        Self {
            workloads: HashMap::new(),
        }
    }

    /// Load state from disk
    pub fn load(path: &PathBuf) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let content = std::fs::read_to_string(path)?;
        let store: StateStore = serde_json::from_str(&content)?;
        Ok(store)
    }

    /// Save state to disk
    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Add or update workload state
    pub fn upsert(&mut self, name: String, state: WorkloadState) {
        self.workloads.insert(name, state);
    }

    /// Get workload state
    pub fn get(&self, name: &str) -> Option<&WorkloadState> {
        self.workloads.get(name)
    }

    /// Remove workload state
    pub fn remove(&mut self, name: &str) -> Option<WorkloadState> {
        self.workloads.remove(name)
    }

    /// List all workloads
    pub fn list(&self) -> Vec<&WorkloadState> {
        self.workloads.values().collect()
    }

    /// Get default state file path
    pub fn default_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".orchestr8/state.json")
    }

    /// Ensure state directory exists
    pub fn ensure_state_dir() -> anyhow::Result<()> {
        let state_file = Self::default_path();
        if let Some(parent) = state_file.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Instance;

    #[test]
    fn test_state_store_operations() {
        let mut store = StateStore::new();

        let state = create_test_state("test-app");
        store.upsert("test-app".to_string(), state.clone());

        assert!(store.get("test-app").is_some());
        assert_eq!(store.list().len(), 1);

        let removed = store.remove("test-app");
        assert!(removed.is_some());
        assert_eq!(store.list().len(), 0);
    }

    fn create_test_state(name: &str) -> WorkloadState {
        WorkloadState {
            name: name.to_string(),
            runtime: RuntimeKind::Podman,
            instance: Instance {
                id: "abc123".to_string(),
                name: name.to_string(),
                runtime: RuntimeKind::Podman,
                image: "test:latest".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
            },
            spec_path: PathBuf::from("workload.yaml"),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            updated_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }
}
