//! Local state store for tracking workload instances

use anyhow::Context;
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

impl WorkloadState {
    /// Create a new WorkloadState, setting both timestamps to now.
    pub fn new(name: String, runtime: RuntimeKind, instance: Instance, spec_path: PathBuf) -> Self {
        let now = crate::resources::now_rfc3339();
        Self {
            name,
            runtime,
            instance,
            spec_path,
            created_at: now.clone(),
            updated_at: now,
        }
    }

    /// Return a copy migrated to a new runtime/instance, preserving `created_at`.
    pub fn migrated(&self, runtime: RuntimeKind, instance: Instance) -> Self {
        Self {
            name: self.name.clone(),
            runtime,
            instance,
            spec_path: self.spec_path.clone(),
            created_at: self.created_at.clone(),
            updated_at: crate::resources::now_rfc3339(),
        }
    }
}

impl StateStore {
    /// Create new state store
    pub fn new() -> Self {
        Self {
            workloads: HashMap::new(),
        }
    }

    /// Load state from disk
    pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
        crate::resources::json_load(path)
    }

    /// Save state to disk atomically (write to temp file, then rename)
    pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
        let content = serde_json::to_string_pretty(self)
            .context("failed to serialize state")?;

        // Write to a temporary file in the same directory, then rename.
        // This ensures the state file is never left in a half-written state.
        let dir = path.parent().unwrap_or(std::path::Path::new("."));
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create state directory: {}", dir.display()))?;

        let tmp_path = path.with_extension("json.tmp");
        std::fs::write(&tmp_path, &content)
            .with_context(|| format!("failed to write temp state file: {}", tmp_path.display()))?;
        std::fs::rename(&tmp_path, path)
            .with_context(|| format!("failed to rename temp file to {}", path.display()))?;
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
        crate::resources::orchestr8_path("state.json")
    }

    /// Ensure state directory exists
    pub fn ensure_state_dir() -> anyhow::Result<()> {
        std::fs::create_dir_all(crate::resources::orchestr8_dir())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::Instance;

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

    fn create_test_state_with_runtime(name: &str, runtime: RuntimeKind) -> WorkloadState {
        WorkloadState {
            name: name.to_string(),
            runtime,
            instance: Instance {
                id: format!("id-{}", name),
                name: name.to_string(),
                runtime,
                image: format!("{}:latest", name),
                created_at: "2024-06-15T12:00:00Z".to_string(),
            },
            spec_path: PathBuf::from(format!("{}.yaml", name)),
            created_at: "2024-06-15T12:00:00Z".to_string(),
            updated_at: "2024-06-15T12:00:00Z".to_string(),
        }
    }

    // ── Construction and defaults ──────────────────────────────────────

    #[test]
    fn test_state_store_new_is_empty() {
        let store = StateStore::new();
        assert!(store.workloads.is_empty());
        assert_eq!(store.list().len(), 0);
    }

    #[test]
    fn test_state_store_default_is_empty() {
        let store = StateStore::default();
        assert!(store.workloads.is_empty());
    }

    #[test]
    fn test_default_path_ends_with_state_json() {
        let path = StateStore::default_path();
        assert!(path.ends_with(".orchestr8/state.json"));
    }

    #[test]
    fn test_default_path_is_absolute_when_home_set() {
        // The default_path uses $HOME; if HOME is set it should be absolute
        if std::env::var("HOME").is_ok() {
            let path = StateStore::default_path();
            assert!(path.is_absolute());
        }
    }

    // ── WorkloadState construction ─────────────────────────────────────

    #[test]
    fn test_workload_state_all_fields() {
        let state = WorkloadState {
            name: "my-service".to_string(),
            runtime: RuntimeKind::Kubernetes,
            instance: Instance {
                id: "k8s-pod-xyz".to_string(),
                name: "my-service".to_string(),
                runtime: RuntimeKind::Kubernetes,
                image: "ghcr.io/org/my-service:v2".to_string(),
                created_at: "2025-03-01T08:30:00Z".to_string(),
            },
            spec_path: PathBuf::from("/etc/orchestr8/my-service.yaml"),
            created_at: "2025-03-01T08:30:00Z".to_string(),
            updated_at: "2025-03-02T10:00:00Z".to_string(),
        };
        assert_eq!(state.name, "my-service");
        assert_eq!(state.runtime, RuntimeKind::Kubernetes);
        assert_eq!(state.instance.id, "k8s-pod-xyz");
        assert_eq!(state.instance.image, "ghcr.io/org/my-service:v2");
        assert_eq!(
            state.spec_path,
            PathBuf::from("/etc/orchestr8/my-service.yaml")
        );
        assert_eq!(state.created_at, "2025-03-01T08:30:00Z");
        assert_eq!(state.updated_at, "2025-03-02T10:00:00Z");
    }

    #[test]
    fn test_workload_state_with_every_runtime_kind() {
        for runtime in [
            RuntimeKind::Podman,
            RuntimeKind::Kubernetes,
            RuntimeKind::KubeVirt,
            RuntimeKind::Metal3,
        ] {
            let state = create_test_state_with_runtime("rt-test", runtime);
            assert_eq!(state.runtime, runtime);
            assert_eq!(state.instance.runtime, runtime);
        }
    }

    // ── Basic CRUD operations ──────────────────────────────────────────

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

    #[test]
    fn test_upsert_inserts_new_entry() {
        let mut store = StateStore::new();
        store.upsert("app-a".to_string(), create_test_state("app-a"));
        assert_eq!(store.workloads.len(), 1);
        assert_eq!(store.get("app-a").unwrap().name, "app-a");
    }

    #[test]
    fn test_upsert_overwrites_existing_entry() {
        let mut store = StateStore::new();
        let state1 = create_test_state("app-a");
        store.upsert("app-a".to_string(), state1);

        // Create a second state with different timestamps
        let mut state2 = create_test_state("app-a");
        state2.updated_at = "2025-12-31T23:59:59Z".to_string();
        store.upsert("app-a".to_string(), state2);

        // Still only one entry
        assert_eq!(store.workloads.len(), 1);
        // It should have the updated timestamp
        assert_eq!(
            store.get("app-a").unwrap().updated_at,
            "2025-12-31T23:59:59Z"
        );
    }

    #[test]
    fn test_get_returns_none_for_missing_workload() {
        let store = StateStore::new();
        assert!(store.get("nonexistent").is_none());
    }

    #[test]
    fn test_remove_returns_none_for_missing_workload() {
        let mut store = StateStore::new();
        assert!(store.remove("ghost").is_none());
    }

    #[test]
    fn test_remove_returns_the_removed_state() {
        let mut store = StateStore::new();
        store.upsert("svc".to_string(), create_test_state("svc"));
        let removed = store.remove("svc").unwrap();
        assert_eq!(removed.name, "svc");
        assert!(store.get("svc").is_none());
    }

    // ── List with multiple entries ─────────────────────────────────────

    #[test]
    fn test_list_multiple_entries() {
        let mut store = StateStore::new();
        store.upsert("alpha".to_string(), create_test_state("alpha"));
        store.upsert("beta".to_string(), create_test_state("beta"));
        store.upsert("gamma".to_string(), create_test_state("gamma"));

        let items = store.list();
        assert_eq!(items.len(), 3);

        let names: Vec<&str> = items.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
        assert!(names.contains(&"gamma"));
    }

    #[test]
    fn test_list_after_remove() {
        let mut store = StateStore::new();
        store.upsert("a".to_string(), create_test_state("a"));
        store.upsert("b".to_string(), create_test_state("b"));
        store.remove("a");
        let items = store.list();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "b");
    }

    // ── Empty store edge cases ─────────────────────────────────────────

    #[test]
    fn test_list_empty_store() {
        let store = StateStore::new();
        assert!(store.list().is_empty());
    }

    #[test]
    fn test_remove_from_empty_store() {
        let mut store = StateStore::new();
        assert!(store.remove("anything").is_none());
    }

    #[test]
    fn test_get_from_empty_store() {
        let store = StateStore::new();
        assert!(store.get("anything").is_none());
    }

    // ── Duplicate names (same key, different key) ──────────────────────

    #[test]
    fn test_duplicate_key_replaces() {
        let mut store = StateStore::new();
        let s1 = create_test_state_with_runtime("dup", RuntimeKind::Podman);
        let s2 = create_test_state_with_runtime("dup", RuntimeKind::Kubernetes);
        store.upsert("dup".to_string(), s1);
        store.upsert("dup".to_string(), s2);

        assert_eq!(store.workloads.len(), 1);
        assert_eq!(store.get("dup").unwrap().runtime, RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_same_name_different_keys() {
        // The HashMap key is the lookup key; the WorkloadState.name can differ
        let mut store = StateStore::new();
        let state = create_test_state("shared-name");
        store.upsert("key-1".to_string(), state.clone());
        store.upsert("key-2".to_string(), state);

        assert_eq!(store.workloads.len(), 2);
        assert!(store.get("key-1").is_some());
        assert!(store.get("key-2").is_some());
    }

    // ── Serialization / deserialization roundtrip ───────────────────────

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        let mut store = StateStore::new();
        store.upsert("web".to_string(), create_test_state("web"));
        store.upsert(
            "db".to_string(),
            create_test_state_with_runtime("db", RuntimeKind::Kubernetes),
        );
        store.save(&path).unwrap();

        let loaded = StateStore::load(&path).unwrap();
        assert_eq!(loaded.workloads.len(), 2);
        assert_eq!(loaded.get("web").unwrap().name, "web");
        assert_eq!(loaded.get("db").unwrap().runtime, RuntimeKind::Kubernetes);
    }

    #[test]
    fn test_load_nonexistent_file_returns_empty_store() {
        let path = PathBuf::from("/tmp/orchestr8_test_does_not_exist_98765.json");
        let store = StateStore::load(&path).unwrap();
        assert!(store.workloads.is_empty());
    }

    #[test]
    fn test_save_creates_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("new_state.json");
        assert!(!path.exists());

        let store = StateStore::new();
        store.save(&path).unwrap();
        assert!(path.exists());
    }

    #[test]
    fn test_save_overwrite_existing() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        // Save with one entry
        let mut store = StateStore::new();
        store.upsert("first".to_string(), create_test_state("first"));
        store.save(&path).unwrap();

        // Save with a different entry
        let mut store2 = StateStore::new();
        store2.upsert("second".to_string(), create_test_state("second"));
        store2.save(&path).unwrap();

        // Load should show only the second entry
        let loaded = StateStore::load(&path).unwrap();
        assert_eq!(loaded.workloads.len(), 1);
        assert!(loaded.get("first").is_none());
        assert!(loaded.get("second").is_some());
    }

    #[test]
    fn test_save_empty_store_and_load_back() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.json");

        let store = StateStore::new();
        store.save(&path).unwrap();

        let loaded = StateStore::load(&path).unwrap();
        assert!(loaded.workloads.is_empty());
    }

    #[test]
    fn test_roundtrip_preserves_all_fields() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fields.json");

        let state = WorkloadState {
            name: "full-check".to_string(),
            runtime: RuntimeKind::KubeVirt,
            instance: Instance {
                id: "inst-999".to_string(),
                name: "full-check".to_string(),
                runtime: RuntimeKind::KubeVirt,
                image: "registry.io/img:v3".to_string(),
                created_at: "2025-07-04T00:00:00Z".to_string(),
            },
            spec_path: PathBuf::from("/specs/full-check.yaml"),
            created_at: "2025-07-04T00:00:00Z".to_string(),
            updated_at: "2025-07-05T12:00:00Z".to_string(),
        };

        let mut store = StateStore::new();
        store.upsert("full-check".to_string(), state);
        store.save(&path).unwrap();

        let loaded = StateStore::load(&path).unwrap();
        let ws = loaded.get("full-check").unwrap();
        assert_eq!(ws.name, "full-check");
        assert_eq!(ws.runtime, RuntimeKind::KubeVirt);
        assert_eq!(ws.instance.id, "inst-999");
        assert_eq!(ws.instance.image, "registry.io/img:v3");
        assert_eq!(ws.spec_path, PathBuf::from("/specs/full-check.yaml"));
        assert_eq!(ws.created_at, "2025-07-04T00:00:00Z");
        assert_eq!(ws.updated_at, "2025-07-05T12:00:00Z");
    }

    // ── JSON structure verification ────────────────────────────────────

    #[test]
    fn test_serialized_json_is_valid() {
        let mut store = StateStore::new();
        store.upsert("svc".to_string(), create_test_state("svc"));

        let json = serde_json::to_string(&store).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(parsed.get("workloads").is_some());
        assert!(parsed["workloads"].get("svc").is_some());
    }

    #[test]
    fn test_deserialize_from_json_string() {
        let json = r#"{"workloads":{}}"#;
        let store: StateStore = serde_json::from_str(json).unwrap();
        assert!(store.workloads.is_empty());
    }

    // ── Workloads map accessor ─────────────────────────────────────────

    #[test]
    fn test_workloads_field_directly() {
        let mut store = StateStore::new();
        store.upsert("x".to_string(), create_test_state("x"));
        store.upsert("y".to_string(), create_test_state("y"));

        // Access the HashMap directly
        assert!(store.workloads.contains_key("x"));
        assert!(store.workloads.contains_key("y"));
        assert!(!store.workloads.contains_key("z"));
        assert_eq!(store.workloads.len(), 2);
    }

    // ── Large store ────────────────────────────────────────────────────

    #[test]
    fn test_many_workloads() {
        let mut store = StateStore::new();
        for i in 0..100 {
            let name = format!("workload-{}", i);
            store.upsert(name.clone(), create_test_state(&name));
        }
        assert_eq!(store.workloads.len(), 100);
        assert_eq!(store.list().len(), 100);
        assert!(store.get("workload-0").is_some());
        assert!(store.get("workload-99").is_some());
        assert!(store.get("workload-100").is_none());
    }

    #[test]
    fn test_many_workloads_save_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.json");

        let mut store = StateStore::new();
        for i in 0..50 {
            let name = format!("svc-{}", i);
            store.upsert(name.clone(), create_test_state(&name));
        }
        store.save(&path).unwrap();

        let loaded = StateStore::load(&path).unwrap();
        assert_eq!(loaded.workloads.len(), 50);
    }
}
