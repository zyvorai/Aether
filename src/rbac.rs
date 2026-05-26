//! Role-Based Access Control (RBAC) for the Aether API
//!
//! Manages API keys with associated roles and enforces permission checks
//! on API endpoints based on the caller's role.

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Roles that can be assigned to API keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Full access to all endpoints, including RBAC management.
    Admin,
    /// Read and write access to workload endpoints, but no RBAC management.
    Operator,
    /// Read-only access to all endpoints.
    Viewer,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::Admin => write!(f, "admin"),
            Role::Operator => write!(f, "operator"),
            Role::Viewer => write!(f, "viewer"),
        }
    }
}

/// An entry in the RBAC store representing a registered API key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyEntry {
    /// SHA-256 hash of the plaintext API key (hex-encoded).
    pub key_hash: String,
    /// Role assigned to this key.
    pub role: Role,
    /// Human-readable name/label for this key.
    pub name: String,
    /// ISO-8601 timestamp of when this key was created.
    pub created_at: String,
}

/// Persistent store for RBAC API key entries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RbacStore {
    entries: HashMap<String, ApiKeyEntry>,
}

impl RbacStore {
    /// Create a new empty RBAC store.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Load an RBAC store from a JSON file.
    ///
    /// Returns an empty store if the file does not exist.
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let data = std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read RBAC store from {}", path.display()))?;
        let store: RbacStore = serde_json::from_str(&data)
            .with_context(|| format!("Failed to parse RBAC store from {}", path.display()))?;
        Ok(store)
    }

    /// Save the RBAC store to a JSON file with restricted permissions (0o600).
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory {}", parent.display()))?;
        }
        let data = serde_json::to_string_pretty(self)
            .context("Failed to serialize RBAC store")?;
        std::fs::write(path, &data)
            .with_context(|| format!("Failed to write RBAC store to {}", path.display()))?;

        // Set restrictive permissions (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .context("Failed to set RBAC store file permissions")?;
        }

        Ok(())
    }

    /// Generate a new API key, store its hash, and return the plaintext key.
    ///
    /// The plaintext key is returned exactly once and is never stored.
    pub fn create_key(&mut self, name: &str, role: Role) -> String {
        let plaintext = generate_api_key();
        let key_hash = hash_key(&plaintext);
        let created_at = chrono::Utc::now().to_rfc3339();

        let entry = ApiKeyEntry {
            key_hash: key_hash.clone(),
            role,
            name: name.to_string(),
            created_at,
        };

        self.entries.insert(key_hash, entry);
        plaintext
    }

    /// Verify an API key and return its entry if valid.
    pub fn verify_key(&self, key: &str) -> Option<&ApiKeyEntry> {
        let key_hash = hash_key(key);
        self.entries.get(&key_hash)
    }

    /// Revoke an API key by its human-readable name.
    ///
    /// Returns `true` if a key was found and removed, `false` otherwise.
    pub fn revoke_key(&mut self, name: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|_, entry| entry.name != name);
        self.entries.len() < before
    }

    /// List all registered API key entries (without exposing raw hashes
    /// in a way that could be used to forge keys, since the hashes are
    /// already one-way).
    pub fn list_keys(&self) -> Vec<&ApiKeyEntry> {
        self.entries.values().collect()
    }

    /// Default filesystem path for the RBAC store: `~/.aether/rbac.json`.
    ///
    /// Falls back to `/tmp/.aether/rbac.json` if the home directory is not
    /// available, rather than writing to the current working directory.
    pub fn default_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".aether")
            .join("rbac.json")
    }
}

impl Default for RbacStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Hash a plaintext API key using salted SHA-256 and return the hex-encoded digest.
///
/// The salt prevents rainbow-table attacks if the RBAC store file is compromised.
const HASH_SALT: &str = "aether-rbac-v1";

fn hash_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(HASH_SALT.as_bytes());
    hasher.update(key.as_bytes());
    let hash = hasher.finalize();
    format!("{:x}", hash)
}

/// Generate a cryptographically random API key (48 random bytes, hex-encoded
/// to produce a 96-character string prefixed with `aether_`).
fn generate_api_key() -> String {
    use aes_gcm::aead::rand_core::{OsRng, RngCore};
    let mut bytes = [0u8; 48];
    OsRng.fill_bytes(&mut bytes);
    let hex: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    format!("aether_{}", hex)
}

/// Check whether a given role has permission to perform a request.
///
/// Permission matrix:
/// - **Viewer**: GET requests only
/// - **Operator**: GET; POST except `/api/rbac/*`; PUT/PATCH on workload updates;
///   DELETE on workloads, secrets, backups, and dependency edges; no admin-only audit append.
/// - **Admin**: all methods, all paths
pub fn check_permission(role: &Role, method: &str, path: &str) -> bool {
    match role {
        Role::Admin => true,
        Role::Operator => {
            let method_upper = method.to_uppercase();
            if method_upper == "GET" {
                return true;
            }
            if method_upper == "POST" {
                // Operators cannot manage RBAC or append external audit events
                if path == "/api/rbac" || path.starts_with("/api/rbac/") {
                    return false;
                }
                if path == "/api/audit/events" {
                    return false;
                }
                return true;
            }
            if method_upper == "PUT" || method_upper == "PATCH" {
                return path.starts_with("/api/workloads/");
            }
            if method_upper == "DELETE" {
                return path.starts_with("/api/workloads/")
                    || path.starts_with("/api/secrets/")
                    || path.starts_with("/api/backups/")
                    || path == "/api/dependencies";
            }
            false
        }
        Role::Viewer => {
            method.to_uppercase() == "GET"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_and_verify_key() {
        let mut store = RbacStore::new();
        let key = store.create_key("test-key", Role::Admin);

        assert!(key.starts_with("aether_"));
        // 7 (prefix) + 96 (hex) = 103 characters
        assert_eq!(key.len(), 103);

        let entry = store.verify_key(&key);
        assert!(entry.is_some());
        let entry = entry.unwrap();
        assert_eq!(entry.name, "test-key");
        assert_eq!(entry.role, Role::Admin);
    }

    #[test]
    fn test_verify_invalid_key() {
        let mut store = RbacStore::new();
        store.create_key("test-key", Role::Viewer);

        assert!(store.verify_key("aether_bogus_key").is_none());
        assert!(store.verify_key("").is_none());
    }

    #[test]
    fn test_create_multiple_keys() {
        let mut store = RbacStore::new();
        let key1 = store.create_key("admin-key", Role::Admin);
        let key2 = store.create_key("viewer-key", Role::Viewer);
        let key3 = store.create_key("operator-key", Role::Operator);

        // All keys are distinct
        assert_ne!(key1, key2);
        assert_ne!(key2, key3);
        assert_ne!(key1, key3);

        // All verify correctly
        assert_eq!(store.verify_key(&key1).unwrap().role, Role::Admin);
        assert_eq!(store.verify_key(&key2).unwrap().role, Role::Viewer);
        assert_eq!(store.verify_key(&key3).unwrap().role, Role::Operator);
    }

    #[test]
    fn test_revoke_key() {
        let mut store = RbacStore::new();
        let key = store.create_key("to-revoke", Role::Operator);

        assert!(store.verify_key(&key).is_some());
        assert!(store.revoke_key("to-revoke"));
        assert!(store.verify_key(&key).is_none());
    }

    #[test]
    fn test_revoke_nonexistent_key() {
        let mut store = RbacStore::new();
        assert!(!store.revoke_key("does-not-exist"));
    }

    #[test]
    fn test_list_keys() {
        let mut store = RbacStore::new();
        store.create_key("alpha", Role::Admin);
        store.create_key("beta", Role::Viewer);

        let keys = store.list_keys();
        assert_eq!(keys.len(), 2);

        let names: Vec<&str> = keys.iter().map(|e| e.name.as_str()).collect();
        assert!(names.contains(&"alpha"));
        assert!(names.contains(&"beta"));
    }

    #[test]
    fn test_save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("rbac.json");

        let mut store = RbacStore::new();
        let key = store.create_key("persistent", Role::Operator);
        store.save(&path).unwrap();

        let loaded = RbacStore::load(&path).unwrap();
        let entry = loaded.verify_key(&key);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().name, "persistent");
        assert_eq!(entry.unwrap().role, Role::Operator);
    }

    #[test]
    fn test_load_nonexistent_returns_empty() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("does_not_exist.json");

        let store = RbacStore::load(&path).unwrap();
        assert!(store.list_keys().is_empty());
    }

    #[test]
    fn test_save_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("nested").join("deep").join("rbac.json");

        let store = RbacStore::new();
        store.save(&path).unwrap();
        assert!(path.exists());
    }

    #[cfg(unix)]
    #[test]
    fn test_save_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempdir().unwrap();
        let path = dir.path().join("rbac.json");

        let store = RbacStore::new();
        store.save(&path).unwrap();

        let metadata = std::fs::metadata(&path).unwrap();
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn test_default_path() {
        let path = RbacStore::default_path();
        let path_str = path.to_string_lossy();
        assert!(path_str.ends_with(".aether/rbac.json") || path_str.ends_with(".aether\\rbac.json"));
    }

    // --- Permission matrix tests ---

    #[test]
    fn test_admin_permissions() {
        assert!(check_permission(&Role::Admin, "GET", "/api/workloads"));
        assert!(check_permission(&Role::Admin, "POST", "/api/workloads"));
        assert!(check_permission(&Role::Admin, "PUT", "/api/workloads/foo"));
        assert!(check_permission(&Role::Admin, "DELETE", "/api/workloads/foo"));
        assert!(check_permission(&Role::Admin, "GET", "/api/rbac/keys"));
        assert!(check_permission(&Role::Admin, "POST", "/api/rbac/keys"));
        assert!(check_permission(&Role::Admin, "DELETE", "/api/rbac/keys/foo"));
    }

    #[test]
    fn test_operator_permissions() {
        // Operators can GET anything
        assert!(check_permission(&Role::Operator, "GET", "/api/workloads"));
        assert!(check_permission(&Role::Operator, "GET", "/api/rbac/keys"));

        // Operators can POST to workload endpoints
        assert!(check_permission(&Role::Operator, "POST", "/api/workloads"));
        assert!(check_permission(&Role::Operator, "POST", "/api/deploy"));

        // Operators cannot POST to RBAC endpoints
        assert!(!check_permission(&Role::Operator, "POST", "/api/rbac/keys"));
        assert!(!check_permission(&Role::Operator, "POST", "/api/rbac/revoke"));

        // Operators can PUT/PATCH workload updates and DELETE workloads / secrets / backups / deps
        assert!(check_permission(&Role::Operator, "PUT", "/api/workloads/foo"));
        assert!(check_permission(&Role::Operator, "PATCH", "/api/workloads/foo"));
        assert!(check_permission(&Role::Operator, "DELETE", "/api/workloads/foo"));
        assert!(check_permission(&Role::Operator, "DELETE", "/api/secrets/foo"));
        assert!(check_permission(&Role::Operator, "DELETE", "/api/backups/foo"));
        assert!(check_permission(&Role::Operator, "DELETE", "/api/dependencies"));
        assert!(!check_permission(&Role::Operator, "PUT", "/api/other/foo"));
        assert!(!check_permission(&Role::Operator, "DELETE", "/api/rbac/keys"));
        assert!(!check_permission(&Role::Operator, "POST", "/api/audit/events"));
    }

    #[test]
    fn test_viewer_permissions() {
        // Viewers can GET
        assert!(check_permission(&Role::Viewer, "GET", "/api/workloads"));
        assert!(check_permission(&Role::Viewer, "GET", "/api/rbac/keys"));

        // Viewers cannot POST, PUT, DELETE
        assert!(!check_permission(&Role::Viewer, "POST", "/api/workloads"));
        assert!(!check_permission(&Role::Viewer, "PUT", "/api/workloads/foo"));
        assert!(!check_permission(&Role::Viewer, "DELETE", "/api/workloads/foo"));
        assert!(!check_permission(&Role::Viewer, "POST", "/api/rbac/keys"));
    }

    #[test]
    fn test_permission_case_insensitivity() {
        // Method matching should be case-insensitive
        assert!(check_permission(&Role::Viewer, "get", "/api/workloads"));
        assert!(check_permission(&Role::Viewer, "Get", "/api/workloads"));
        assert!(check_permission(&Role::Operator, "post", "/api/workloads"));
    }

    #[test]
    fn test_hash_key_deterministic() {
        let hash1 = hash_key("test-key-123");
        let hash2 = hash_key("test-key-123");
        assert_eq!(hash1, hash2);

        let hash3 = hash_key("different-key");
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_hash_key_format() {
        let hash = hash_key("anything");
        // SHA-256 hex digest is 64 characters
        assert_eq!(hash.len(), 64);
        assert!(hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_role_display() {
        assert_eq!(Role::Admin.to_string(), "admin");
        assert_eq!(Role::Operator.to_string(), "operator");
        assert_eq!(Role::Viewer.to_string(), "viewer");
    }

    #[test]
    fn test_role_serialization_roundtrip() {
        let roles = vec![Role::Admin, Role::Operator, Role::Viewer];
        for role in &roles {
            let json = serde_json::to_string(role).unwrap();
            let deserialized: Role = serde_json::from_str(&json).unwrap();
            assert_eq!(*role, deserialized);
        }
    }

    #[test]
    fn test_store_default_trait() {
        let store = RbacStore::default();
        assert!(store.list_keys().is_empty());
    }

    #[test]
    fn test_revoke_only_removes_matching_name() {
        let mut store = RbacStore::new();
        let key_a = store.create_key("keep-me", Role::Admin);
        let _key_b = store.create_key("remove-me", Role::Viewer);

        assert_eq!(store.list_keys().len(), 2);
        assert!(store.revoke_key("remove-me"));
        assert_eq!(store.list_keys().len(), 1);

        // The remaining key should still verify
        assert!(store.verify_key(&key_a).is_some());
    }
}
