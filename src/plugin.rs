//! Plugin/extension system for custom runtimes
//!
//! Allows third-party runtimes to be registered and invoked via a JSON-RPC
//! style protocol.  Plugins are discovered from `~/.orchestr8/plugins/` as
//! `*.json` manifest files and tracked in a persistent registry.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Manifest describing a plugin binary and the capabilities it exposes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginManifest {
    /// Human-readable plugin name (also used as the registry key).
    pub name: String,
    /// Semantic version of the plugin.
    pub version: String,
    /// Custom runtime identifier that this plugin provides.
    pub runtime_kind: String,
    /// Path to the plugin binary.
    pub command: String,
    /// Capabilities the plugin supports (e.g. "build", "run", "stop", …).
    pub capabilities: Vec<String>,
}

/// Persistent registry of installed plugins.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PluginRegistry {
    /// Registered plugins keyed by name.
    pub plugins: HashMap<String, PluginManifest>,
}

crate::impl_json_store!(PluginRegistry, "plugins.json");

impl PluginRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Discover plugin manifests by scanning `~/.orchestr8/plugins/` for
    /// `*.json` files.  Each valid manifest is merged into the registry,
    /// keyed by its `name` field.
    pub fn discover(&mut self) -> anyhow::Result<usize> {
        self.discover_from(&Self::plugins_dir())
    }

    /// Discover plugin manifests from an explicit directory path.
    pub fn discover_from(&mut self, plugins_dir: &std::path::Path) -> anyhow::Result<usize> {
        if !plugins_dir.exists() {
            return Ok(0);
        }

        let mut count = 0usize;
        for entry in std::fs::read_dir(plugins_dir)? {
            let entry = entry?;
            let path = entry.path();

            let is_json = path
                .extension()
                .map(|ext| ext == "json")
                .unwrap_or(false);
            if !is_json {
                continue;
            }

            match Self::load_manifest(&path) {
                Ok(manifest) => {
                    self.plugins.insert(manifest.name.clone(), manifest);
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!(
                        "Skipping invalid plugin manifest {}: {}",
                        path.display(),
                        e
                    );
                }
            }
        }

        Ok(count)
    }

    /// Register a plugin manifest.
    pub fn register(&mut self, manifest: PluginManifest) {
        self.plugins.insert(manifest.name.clone(), manifest);
    }

    /// Unregister a plugin by name, returning the removed manifest if present.
    pub fn unregister(&mut self, name: &str) -> Option<PluginManifest> {
        self.plugins.remove(name)
    }

    /// Look up a plugin by name.
    pub fn get(&self, name: &str) -> Option<&PluginManifest> {
        self.plugins.get(name)
    }

    /// Return the directory where plugin manifests are discovered.
    fn plugins_dir() -> PathBuf {
        crate::resources::orchestr8_dir().join("plugins")
    }

    /// Parse a single manifest file.
    fn load_manifest(path: &std::path::Path) -> anyhow::Result<PluginManifest> {
        let content = std::fs::read_to_string(path)?;
        let manifest: PluginManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }
}

/// JSON-RPC style protocol messages exchanged with plugin binaries.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum PluginProtocol {
    // ── Requests ──────────────────────────────────────────────────────

    /// Ask the plugin to build an image from the given spec.
    BuildRequest {
        spec_json: String,
    },
    /// Ask the plugin to run a workload.
    RunRequest {
        image_json: String,
        spec_json: String,
    },
    /// Ask the plugin to stop a running instance.
    StopRequest {
        instance_json: String,
    },
    /// Ask the plugin for the status of an instance.
    StatusRequest {
        instance_json: String,
    },
    /// Ask the plugin to delete an instance.
    DeleteRequest {
        instance_json: String,
    },
    /// Ask the plugin to list all instances it manages.
    ListRequest,

    // ── Responses ─────────────────────────────────────────────────────

    /// Response to a `BuildRequest`.
    BuildResponse {
        image_json: String,
    },
    /// Response to a `RunRequest`.
    RunResponse {
        instance_json: String,
    },
    /// Response to a `StopRequest`.
    StopResponse {
        success: bool,
    },
    /// Response to a `StatusRequest`.
    StatusResponse {
        status_json: String,
    },
    /// Response to a `DeleteRequest`.
    DeleteResponse {
        success: bool,
    },
    /// Response to a `ListRequest`.
    ListResponse {
        instances_json: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── PluginManifest serialization ──────────────────────────────────

    fn sample_manifest() -> PluginManifest {
        PluginManifest {
            name: "my-runtime".to_string(),
            version: "0.1.0".to_string(),
            runtime_kind: "custom-wasm".to_string(),
            command: "/usr/local/bin/my-runtime".to_string(),
            capabilities: vec![
                "build".to_string(),
                "run".to_string(),
                "stop".to_string(),
                "status".to_string(),
            ],
        }
    }

    #[test]
    fn test_manifest_serialize_deserialize() {
        let manifest = sample_manifest();
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: PluginManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest, parsed);
    }

    #[test]
    fn test_manifest_fields() {
        let manifest = sample_manifest();
        assert_eq!(manifest.name, "my-runtime");
        assert_eq!(manifest.version, "0.1.0");
        assert_eq!(manifest.runtime_kind, "custom-wasm");
        assert_eq!(manifest.command, "/usr/local/bin/my-runtime");
        assert_eq!(manifest.capabilities.len(), 4);
        assert!(manifest.capabilities.contains(&"build".to_string()));
    }

    #[test]
    fn test_manifest_deserialize_from_json_string() {
        let json = r#"{
            "name": "wasm-plugin",
            "version": "1.2.3",
            "runtime_kind": "wasm",
            "command": "/opt/wasm-runner",
            "capabilities": ["run", "stop"]
        }"#;
        let m: PluginManifest = serde_json::from_str(json).unwrap();
        assert_eq!(m.name, "wasm-plugin");
        assert_eq!(m.capabilities, vec!["run", "stop"]);
    }

    // ── PluginRegistry operations ─────────────────────────────────────

    #[test]
    fn test_registry_new_is_empty() {
        let reg = PluginRegistry::new();
        assert!(reg.plugins.is_empty());
    }

    #[test]
    fn test_registry_default_is_empty() {
        let reg = PluginRegistry::default();
        assert!(reg.plugins.is_empty());
    }

    #[test]
    fn test_registry_register_and_get() {
        let mut reg = PluginRegistry::new();
        reg.register(sample_manifest());
        assert!(reg.get("my-runtime").is_some());
        assert_eq!(reg.get("my-runtime").unwrap().version, "0.1.0");
    }

    #[test]
    fn test_registry_get_returns_none_for_missing() {
        let reg = PluginRegistry::new();
        assert!(reg.get("nonexistent").is_none());
    }

    #[test]
    fn test_registry_unregister() {
        let mut reg = PluginRegistry::new();
        reg.register(sample_manifest());
        let removed = reg.unregister("my-runtime");
        assert!(removed.is_some());
        assert_eq!(removed.unwrap().name, "my-runtime");
        assert!(reg.get("my-runtime").is_none());
    }

    #[test]
    fn test_registry_unregister_missing_returns_none() {
        let mut reg = PluginRegistry::new();
        assert!(reg.unregister("ghost").is_none());
    }

    #[test]
    fn test_registry_register_overwrites() {
        let mut reg = PluginRegistry::new();
        reg.register(sample_manifest());

        let mut updated = sample_manifest();
        updated.version = "0.2.0".to_string();
        reg.register(updated);

        assert_eq!(reg.plugins.len(), 1);
        assert_eq!(reg.get("my-runtime").unwrap().version, "0.2.0");
    }

    #[test]
    fn test_registry_multiple_plugins() {
        let mut reg = PluginRegistry::new();
        reg.register(sample_manifest());

        let mut second = sample_manifest();
        second.name = "other-runtime".to_string();
        reg.register(second);

        assert_eq!(reg.plugins.len(), 2);
        assert!(reg.get("my-runtime").is_some());
        assert!(reg.get("other-runtime").is_some());
    }

    // ── Persistence roundtrip ─────────────────────────────────────────

    #[test]
    fn test_registry_save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("plugins.json");

        let mut reg = PluginRegistry::new();
        reg.register(sample_manifest());
        reg.save(&path).unwrap();

        let loaded = PluginRegistry::load(&path).unwrap();
        assert_eq!(loaded.plugins.len(), 1);
        assert_eq!(
            loaded.get("my-runtime").unwrap().runtime_kind,
            "custom-wasm"
        );
    }

    #[test]
    fn test_registry_load_missing_file_returns_default() {
        let path = std::path::Path::new("/tmp/orchestr8_test_no_such_plugins.json");
        let reg = PluginRegistry::load(path).unwrap();
        assert!(reg.plugins.is_empty());
    }

    #[test]
    fn test_registry_default_path_ends_with_plugins_json() {
        let path = PluginRegistry::default_path();
        assert!(path.ends_with(".orchestr8/plugins.json"));
    }

    // ── Discovery from temp dir ───────────────────────────────────────

    #[test]
    fn test_discover_from_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let plugins_dir = dir.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();

        let mut reg = PluginRegistry::new();
        let count = reg.discover_from(&plugins_dir).unwrap();
        assert_eq!(count, 0);
        assert!(reg.plugins.is_empty());
    }

    #[test]
    fn test_discover_finds_manifests() {
        let dir = tempfile::tempdir().unwrap();
        let plugins_dir = dir.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();

        // Write two valid manifests
        let m1 = PluginManifest {
            name: "alpha".to_string(),
            version: "1.0.0".to_string(),
            runtime_kind: "alpha-rt".to_string(),
            command: "/bin/alpha".to_string(),
            capabilities: vec!["run".to_string()],
        };
        let m2 = PluginManifest {
            name: "beta".to_string(),
            version: "2.0.0".to_string(),
            runtime_kind: "beta-rt".to_string(),
            command: "/bin/beta".to_string(),
            capabilities: vec!["build".to_string(), "run".to_string()],
        };
        std::fs::write(
            plugins_dir.join("alpha.json"),
            serde_json::to_string_pretty(&m1).unwrap(),
        )
        .unwrap();
        std::fs::write(
            plugins_dir.join("beta.json"),
            serde_json::to_string_pretty(&m2).unwrap(),
        )
        .unwrap();

        let mut reg = PluginRegistry::new();
        let count = reg.discover_from(&plugins_dir).unwrap();
        assert_eq!(count, 2);
        assert_eq!(reg.get("alpha").unwrap().runtime_kind, "alpha-rt");
        assert_eq!(reg.get("beta").unwrap().version, "2.0.0");
    }

    #[test]
    fn test_discover_skips_non_json_files() {
        let dir = tempfile::tempdir().unwrap();
        let plugins_dir = dir.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();

        std::fs::write(plugins_dir.join("readme.txt"), "not a manifest").unwrap();
        let m = sample_manifest();
        std::fs::write(
            plugins_dir.join("good.json"),
            serde_json::to_string(&m).unwrap(),
        )
        .unwrap();

        let mut reg = PluginRegistry::new();
        let count = reg.discover_from(&plugins_dir).unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_discover_skips_invalid_json() {
        let dir = tempfile::tempdir().unwrap();
        let plugins_dir = dir.path().join("plugins");
        std::fs::create_dir_all(&plugins_dir).unwrap();

        std::fs::write(plugins_dir.join("bad.json"), "{ not valid json }").unwrap();

        let mut reg = PluginRegistry::new();
        let count = reg.discover_from(&plugins_dir).unwrap();
        assert_eq!(count, 0);
        assert!(reg.plugins.is_empty());
    }

    #[test]
    fn test_discover_nonexistent_dir_returns_zero() {
        let dir = tempfile::tempdir().unwrap();
        let nonexistent = dir.path().join("no-such-dir");
        let mut reg = PluginRegistry::new();
        let count = reg.discover_from(&nonexistent).unwrap();
        assert_eq!(count, 0);
    }

    // ── PluginProtocol serialization ──────────────────────────────────

    #[test]
    fn test_protocol_build_request_roundtrip() {
        let msg = PluginProtocol::BuildRequest {
            spec_json: r#"{"name":"app"}"#.to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: PluginProtocol = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn test_protocol_run_request_roundtrip() {
        let msg = PluginProtocol::RunRequest {
            image_json: r#"{"tag":"v1"}"#.to_string(),
            spec_json: r#"{"cpu":"2"}"#.to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: PluginProtocol = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn test_protocol_list_request_roundtrip() {
        let msg = PluginProtocol::ListRequest;
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: PluginProtocol = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn test_protocol_stop_response_roundtrip() {
        let msg = PluginProtocol::StopResponse { success: true };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"success\":true"));
        let parsed: PluginProtocol = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn test_protocol_list_response_roundtrip() {
        let msg = PluginProtocol::ListResponse {
            instances_json: "[]".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let parsed: PluginProtocol = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, parsed);
    }

    #[test]
    fn test_protocol_tagged_serialization() {
        let msg = PluginProtocol::DeleteRequest {
            instance_json: r#"{"id":"abc"}"#.to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"DeleteRequest""#));
    }

}
