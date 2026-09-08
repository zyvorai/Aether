// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Backup and restore functionality for workload state

use crate::state::{StateStore, WorkloadState};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Backups are encrypted at rest only when an explicit `AETHER_SECRET_KEY` is
/// set, so that key-less setups keep portable plaintext backups.
fn encryption_enabled() -> bool {
    std::env::var("AETHER_SECRET_KEY")
        .map(|k| !k.trim().is_empty())
        .unwrap_or(false)
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupMetadata {
    /// Backup version
    pub version: String,
    /// Creation timestamp
    pub created_at: String,
    /// Number of workloads
    pub workload_count: usize,
    /// Backup description
    pub description: Option<String>,
    /// Aether version
    pub aether_version: String,
}

/// Raw contents of the auxiliary JSON stores captured alongside workload state,
/// so a backup restores the full control-plane state — not just workloads.
/// Each field holds the verbatim file contents (`None` when the file is absent).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuxStores {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secrets: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rbac: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environments: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quotas: Option<String>,
}

impl AuxStores {
    fn is_empty(&self) -> bool {
        self.secrets.is_none()
            && self.rbac.is_none()
            && self.environments.is_none()
            && self.quotas.is_none()
    }
}

/// The files backed up under `~/.aether`, paired with whether they hold secrets
/// (and therefore need `0o600` on restore).
const AUX_STORE_FILES: [(&str, bool); 4] = [
    ("secrets.json", true),
    ("rbac.json", true),
    ("environments.json", false),
    ("quotas.json", false),
];

/// On-disk wrapper distinguishing an encrypted backup from a legacy plaintext one.
#[derive(Debug, Serialize, Deserialize)]
struct BackupEnvelope {
    encrypted: bool,
    /// `base64(nonce || ciphertext)` of the backup JSON (see [`crate::secrets::encrypt_blob`]).
    data: String,
}

/// Complete backup including metadata and state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// Backup metadata
    pub metadata: BackupMetadata,
    /// Workload states
    pub workloads: Vec<WorkloadState>,
    /// Auxiliary control-plane stores (secrets, rbac, environments, quotas).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stores: Option<AuxStores>,
}

impl Backup {
    /// Create a new backup from current state, also capturing the auxiliary
    /// control-plane stores (secrets/rbac/environments/quotas) from `~/.aether`.
    pub fn from_state(state: &StateStore, description: Option<String>) -> Self {
        let workloads: Vec<WorkloadState> = state.list().into_iter().cloned().collect();

        let mut aux = AuxStores::default();
        for (file, _secret) in AUX_STORE_FILES {
            let content = fs::read_to_string(crate::resources::aether_path(file)).ok();
            match file {
                "secrets.json" => aux.secrets = content,
                "rbac.json" => aux.rbac = content,
                "environments.json" => aux.environments = content,
                "quotas.json" => aux.quotas = content,
                _ => {}
            }
        }

        Self {
            metadata: BackupMetadata {
                version: "1.0".to_string(),
                created_at: crate::resources::now_rfc3339(),
                workload_count: workloads.len(),
                description,
                aether_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            workloads,
            stores: if aux.is_empty() { None } else { Some(aux) },
        }
    }

    /// Save backup to file with restricted permissions (0o600).
    ///
    /// When `AETHER_SECRET_KEY` is set the backup is encrypted at rest with
    /// AES-256-GCM (see [`crate::secrets::encrypt_blob`]); otherwise it is written
    /// as plaintext JSON for backward compatibility. [`Backup::load`] auto-detects
    /// the format either way.
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self).context("Failed to serialize backup")?;

        let payload = if encryption_enabled() {
            let data = crate::secrets::encrypt_blob(&json).context("Failed to encrypt backup")?;
            serde_json::to_string_pretty(&BackupEnvelope {
                encrypted: true,
                data,
            })
            .context("Failed to serialize backup envelope")?
        } else {
            json
        };

        fs::write(path, &payload)
            .context(format!("Failed to write backup to {}", path.display()))?;

        // Set restrictive permissions (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o600))
                .context("Failed to set backup file permissions")?;
        }

        tracing::info!("Backup saved to {}", path.display());
        Ok(())
    }

    /// Load a backup from file, transparently decrypting encrypted backups and
    /// falling back to legacy plaintext backups.
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .context(format!("Failed to read backup from {}", path.display()))?;

        // Encrypted backups are wrapped in a small envelope; a legacy plaintext
        // backup lacks the `encrypted`/`data` fields and fails this parse, so we
        // fall through to parsing it directly.
        if let Ok(env) = serde_json::from_str::<BackupEnvelope>(&contents) {
            if env.encrypted {
                let json = crate::secrets::decrypt_blob(&env.data)
                    .context("Failed to decrypt backup (wrong AETHER_SECRET_KEY?)")?;
                let backup: Backup =
                    serde_json::from_str(&json).context("Failed to deserialize backup")?;
                tracing::info!("Backup loaded (encrypted) from {}", path.display());
                return Ok(backup);
            }
        }

        let backup: Backup =
            serde_json::from_str(&contents).context("Failed to deserialize backup")?;

        tracing::info!("Backup loaded from {}", path.display());
        Ok(backup)
    }

    /// Load and validate a backup without applying it. Returns the workload count.
    pub fn verify(path: &Path) -> Result<usize> {
        let backup = Backup::load(path)?;
        Ok(backup.workloads.len())
    }

    /// Restore the auxiliary control-plane stores captured in this backup back to
    /// `~/.aether`, applying `0o600` to secret-bearing files.
    fn restore_aux_stores(&self) -> Result<()> {
        let Some(stores) = &self.stores else {
            return Ok(());
        };
        for (file, is_secret) in AUX_STORE_FILES {
            let content = match file {
                "secrets.json" => &stores.secrets,
                "rbac.json" => &stores.rbac,
                "environments.json" => &stores.environments,
                "quotas.json" => &stores.quotas,
                _ => &None,
            };
            if let Some(body) = content {
                let dest = crate::resources::aether_path(file);
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent).ok();
                }
                fs::write(&dest, body).context(format!("Failed to restore {}", dest.display()))?;
                #[cfg(unix)]
                if is_secret {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(0o600));
                }
            }
        }
        Ok(())
    }

    /// Restore backup to state store
    pub fn restore(&self, state_path: &Path) -> Result<()> {
        let mut state = StateStore::new();

        for workload in &self.workloads {
            state.upsert(workload.name.clone(), workload.clone());
        }

        state.save(state_path)?;
        self.restore_aux_stores()?;

        tracing::info!("Restored {} workloads from backup", self.workloads.len());
        Ok(())
    }

    /// Merge backup into existing state (don't overwrite)
    pub fn merge(&self, state_path: &Path) -> Result<()> {
        let mut state = StateStore::load(state_path).unwrap_or_else(|e| {
            tracing::warn!(
                "Failed to load existing state for merge (starting fresh): {}",
                e
            );
            StateStore::new()
        });

        let mut merged_count = 0;
        for workload in &self.workloads {
            if state.get(&workload.name).is_none() {
                state.upsert(workload.name.clone(), workload.clone());
                merged_count += 1;
            }
        }

        state.save(state_path)?;

        tracing::info!(
            "Merged {} new workloads from backup (total: {})",
            merged_count,
            self.workloads.len()
        );
        Ok(())
    }
}

/// Backup summary for API listings — mirrors the dashboard's `BackupInfo` type
/// (web/dashboard/src/types/api.ts). `list_backups()` returns bare paths for internal
/// path-based operations; this pairs each path with the metadata clients need to display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSummary {
    pub path: String,
    pub filename: String,
    pub workload_count: usize,
    pub created_at: String,
    pub aether_version: String,
    pub description: Option<String>,
}

/// Backup manager for creating and managing backups
pub struct BackupManager {
    backup_dir: PathBuf,
}

impl BackupManager {
    /// Create a new backup manager
    pub fn new(backup_dir: PathBuf) -> Self {
        Self { backup_dir }
    }

    /// Get default backup directory
    pub fn default_dir() -> PathBuf {
        let mut path = dirs::home_dir().unwrap_or_else(|| {
            tracing::warn!(
                "Could not determine home directory, using current directory for backups"
            );
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });
        path.push(".aether");
        path.push("backups");
        path
    }

    /// Ensure backup directory exists
    pub fn ensure_backup_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.backup_dir).context("Failed to create backup directory")?;
        Ok(())
    }

    /// Create a backup with automatic naming
    pub fn create_backup(
        &self,
        state: &StateStore,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<PathBuf> {
        self.ensure_backup_dir()?;

        let backup = Backup::from_state(state, description);

        let filename = if let Some(ref n) = name {
            // Validate backup name to prevent path traversal
            if n.contains('/') || n.contains('\\') || n.contains("..") || n.contains('\0') {
                anyhow::bail!("Invalid backup name '{}': must not contain path separators or traversal sequences", n);
            }
            format!("{}.json", n)
        } else {
            let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
            format!("backup-{}.json", timestamp)
        };

        let path = self.backup_dir.join(filename);
        backup.save(&path)?;

        Ok(path)
    }

    /// List all available backups
    pub fn list_backups(&self) -> Result<Vec<PathBuf>> {
        if !self.backup_dir.exists() {
            return Ok(vec![]);
        }

        let mut backups = Vec::new();

        for entry in fs::read_dir(&self.backup_dir).context("Failed to read backup directory")? {
            let entry = entry?;
            let path = entry.path();

            // Skip symlinks to prevent traversal attacks
            if path.is_symlink() {
                tracing::warn!("Skipping symlink in backup directory: {}", path.display());
                continue;
            }

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                backups.push(path);
            }
        }

        backups.sort();
        Ok(backups)
    }

    /// List backups enriched with their metadata (name, timestamp, description, ...) for API
    /// responses — `list_backups()` alone returns bare paths, which is fine for internal
    /// path-based operations (cleanup, restore) but leaves API clients with nothing to
    /// display. Backups that fail to parse (corrupted/foreign files) are logged and skipped
    /// rather than failing the whole listing.
    pub fn list_backups_detailed(&self) -> Result<Vec<BackupSummary>> {
        let paths = self.list_backups()?;
        let mut summaries = Vec::with_capacity(paths.len());
        for path in paths {
            match self.get_backup_info(&path) {
                Ok(metadata) => {
                    let filename = path
                        .file_name()
                        .map(|f| f.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    summaries.push(BackupSummary {
                        path: path.to_string_lossy().into_owned(),
                        filename,
                        workload_count: metadata.workload_count,
                        created_at: metadata.created_at,
                        aether_version: metadata.aether_version,
                        description: metadata.description,
                    });
                }
                Err(e) => {
                    tracing::warn!("Skipping unreadable backup {}: {}", path.display(), e);
                }
            }
        }
        Ok(summaries)
    }

    /// Get backup info without loading full content.
    /// Rejects symlinks to prevent path traversal attacks.
    pub fn get_backup_info(&self, path: &Path) -> Result<BackupMetadata> {
        if path.is_symlink() {
            anyhow::bail!("Refusing to read symlink backup path: {}", path.display());
        }

        let contents = fs::read_to_string(path).context("Failed to read backup file")?;

        let backup: Backup = serde_json::from_str(&contents).context("Failed to parse backup")?;

        Ok(backup.metadata)
    }

    /// Delete a backup.
    /// Rejects symlinks to prevent path traversal attacks.
    pub fn delete_backup(&self, path: &Path) -> Result<()> {
        if path.is_symlink() {
            anyhow::bail!("Refusing to delete symlink backup path: {}", path.display());
        }

        fs::remove_file(path).context(format!("Failed to delete backup {}", path.display()))?;

        tracing::info!("Deleted backup: {}", path.display());
        Ok(())
    }

    /// Clean old backups (keep last N)
    pub fn cleanup_old_backups(&self, keep: usize) -> Result<usize> {
        let mut backups = self.list_backups()?;

        if backups.len() <= keep {
            return Ok(0);
        }

        // Sort by modification time
        backups.sort_by_key(|p| {
            fs::metadata(p)
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });

        let to_delete = backups.len() - keep;
        let mut deleted = 0;

        for path in backups.iter().take(to_delete) {
            if let Err(e) = self.delete_backup(path) {
                tracing::warn!("Failed to delete backup {}: {}", path.display(), e);
            } else {
                deleted += 1;
            }
        }

        tracing::info!("Cleaned up {} old backups", deleted);
        Ok(deleted)
    }
}

/// Snapshot manager for pre-deploy/pre-migrate snapshots of individual workloads.
pub struct SnapshotManager {
    snapshot_dir: PathBuf,
}

impl Default for SnapshotManager {
    fn default() -> Self {
        Self::new()
    }
}

impl SnapshotManager {
    /// Create a snapshot manager using the default directory.
    pub fn new() -> Self {
        let mut path = dirs::home_dir().unwrap_or_else(|| {
            tracing::warn!(
                "Could not determine home directory, using current directory for snapshots"
            );
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
        });
        path.push(".aether");
        path.push("snapshots");
        Self { snapshot_dir: path }
    }

    /// Create a snapshot manager with a custom directory (useful for testing).
    pub fn with_dir(path: PathBuf) -> Self {
        Self { snapshot_dir: path }
    }

    /// Create a snapshot of a single workload state.
    /// Returns the path to the saved snapshot file.
    pub fn create_snapshot(&self, ws: &WorkloadState) -> Result<PathBuf> {
        fs::create_dir_all(&self.snapshot_dir).context("Failed to create snapshot directory")?;

        let now = chrono::Utc::now();
        let timestamp = now.format("%Y%m%dT%H%M%S");
        let nanos = now.timestamp_subsec_nanos();
        let filename = format!("{}-{}-{:09}.json", ws.name, timestamp, nanos);
        let path = self.snapshot_dir.join(filename);

        let backup = Backup {
            metadata: BackupMetadata {
                version: "1.0".to_string(),
                created_at: crate::resources::now_rfc3339(),
                workload_count: 1,
                description: Some(format!("Pre-deploy snapshot of {}", ws.name)),
                aether_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            workloads: vec![ws.clone()],
            stores: None,
        };

        let json = serde_json::to_string_pretty(&backup).context("Failed to serialize snapshot")?;
        fs::write(&path, &json)
            .context(format!("Failed to write snapshot to {}", path.display()))?;

        // Set restrictive permissions (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
                .context("Failed to set snapshot file permissions")?;
        }

        tracing::info!("Snapshot saved: {}", path.display());
        Ok(path)
    }

    /// Find the latest snapshot for a workload by name.
    /// Snapshots are matched by filename prefix `{name}-` and sorted lexicographically.
    pub fn latest_snapshot(&self, name: &str) -> Result<Option<PathBuf>> {
        let snapshots = self.list_snapshots(name)?;
        Ok(snapshots.into_iter().last())
    }

    /// List all snapshots for a workload, sorted by filename (oldest first).
    pub fn list_snapshots(&self, name: &str) -> Result<Vec<PathBuf>> {
        if !self.snapshot_dir.exists() {
            return Ok(vec![]);
        }

        let prefix = format!("{}-", name);
        let mut matches = Vec::new();

        for entry in
            fs::read_dir(&self.snapshot_dir).context("Failed to read snapshot directory")?
        {
            let entry = entry?;
            let path = entry.path();
            if let Some(fname) = path.file_name().and_then(|f| f.to_str()) {
                if fname.starts_with(&prefix) && fname.ends_with(".json") {
                    matches.push(path);
                }
            }
        }

        matches.sort();
        Ok(matches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{Instance, RuntimeKind};
    use tempfile::tempdir;

    fn create_test_workload_state() -> WorkloadState {
        WorkloadState {
            name: "test-workload".to_string(),
            runtime: RuntimeKind::Podman,
            instance: Instance {
                id: "test-id".to_string(),
                name: "test-name".to_string(),
                runtime: RuntimeKind::Podman,
                image: "test:latest".to_string(),
                created_at: crate::resources::now_rfc3339(),
            },
            spec_path: PathBuf::from("/tmp/workload.yaml"),
            created_at: crate::resources::now_rfc3339(),
            updated_at: crate::resources::now_rfc3339(),
            os_version: None,
            node_labels: vec![],
            atlas_volume_ids: Vec::new(),
            namespace: None,
            cluster_context: None,
            k8s_kind: None,
        }
    }

    #[test]
    fn test_backup_create_and_load() {
        let dir = tempdir().unwrap();
        let backup_path = dir.path().join("test-backup.json");

        let mut state = StateStore::new();
        state.upsert("test".to_string(), create_test_workload_state());

        let backup = Backup::from_state(&state, Some("Test backup".to_string()));
        backup.save(&backup_path).unwrap();

        let loaded = Backup::load(&backup_path).unwrap();
        assert_eq!(loaded.metadata.workload_count, 1);
        assert_eq!(loaded.workloads.len(), 1);
        assert_eq!(loaded.metadata.description, Some("Test backup".to_string()));
    }

    #[test]
    fn test_backup_restore() {
        let dir = tempdir().unwrap();
        let backup_path = dir.path().join("backup.json");
        let state_path = dir.path().join("state.json");

        // Create backup
        let mut state = StateStore::new();
        state.upsert("test".to_string(), create_test_workload_state());
        let backup = Backup::from_state(&state, None);
        backup.save(&backup_path).unwrap();

        // Restore to new location
        let loaded = Backup::load(&backup_path).unwrap();
        loaded.restore(&state_path).unwrap();

        // Verify
        let restored_state = StateStore::load(&state_path).unwrap();
        assert_eq!(restored_state.list().len(), 1);
    }

    #[test]
    fn test_backup_manager() {
        let dir = tempdir().unwrap();
        let manager = BackupManager::new(dir.path().to_path_buf());

        let mut state = StateStore::new();
        state.upsert("test".to_string(), create_test_workload_state());

        // Create backup
        let path = manager
            .create_backup(&state, Some("test".to_string()), None)
            .unwrap();
        assert!(path.exists());

        // List backups
        let backups = manager.list_backups().unwrap();
        assert_eq!(backups.len(), 1);

        // Get info
        let info = manager.get_backup_info(&path).unwrap();
        assert_eq!(info.workload_count, 1);

        // Delete
        manager.delete_backup(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn test_list_backups_detailed_includes_metadata() {
        let dir = tempdir().unwrap();
        let manager = BackupManager::new(dir.path().to_path_buf());

        let mut state = StateStore::new();
        state.upsert("test".to_string(), create_test_workload_state());

        manager
            .create_backup(
                &state,
                Some("smoke-test".to_string()),
                Some("a test description".to_string()),
            )
            .unwrap();

        let summaries = manager.list_backups_detailed().unwrap();
        assert_eq!(summaries.len(), 1);
        let summary = &summaries[0];
        assert_eq!(summary.filename, "smoke-test.json");
        assert_eq!(summary.workload_count, 1);
        assert_eq!(summary.description.as_deref(), Some("a test description"));
        assert!(!summary.aether_version.is_empty());
        assert!(!summary.created_at.is_empty());
        assert!(summary.path.ends_with("smoke-test.json"));
    }

    #[test]
    fn test_list_backups_detailed_skips_unreadable_files() {
        let dir = tempdir().unwrap();
        let manager = BackupManager::new(dir.path().to_path_buf());
        manager.ensure_backup_dir().unwrap();

        // A non-backup JSON file should be skipped, not fail the whole listing.
        fs::write(dir.path().join("not-a-backup.json"), "{\"garbage\": true}").unwrap();

        let mut state = StateStore::new();
        state.upsert("test".to_string(), create_test_workload_state());
        manager
            .create_backup(&state, Some("valid".to_string()), None)
            .unwrap();

        let summaries = manager.list_backups_detailed().unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].filename, "valid.json");
    }

    #[test]
    fn test_cleanup_old_backups() {
        let dir = tempdir().unwrap();
        let manager = BackupManager::new(dir.path().to_path_buf());

        let mut state = StateStore::new();
        state.upsert("test".to_string(), create_test_workload_state());

        // Create multiple backups
        for i in 0..5 {
            manager
                .create_backup(&state, Some(format!("backup-{}", i)), None)
                .unwrap();
        }

        assert_eq!(manager.list_backups().unwrap().len(), 5);

        // Cleanup, keep only 2
        let deleted = manager.cleanup_old_backups(2).unwrap();
        assert_eq!(deleted, 3);
        assert_eq!(manager.list_backups().unwrap().len(), 2);
    }

    #[test]
    fn test_snapshot_create_and_find_latest() {
        let dir = tempdir().unwrap();
        let mgr = SnapshotManager::with_dir(dir.path().to_path_buf());

        let ws = create_test_workload_state();
        let path = mgr.create_snapshot(&ws).unwrap();
        assert!(path.exists());

        let latest = mgr.latest_snapshot("test-workload").unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap(), path);
    }

    #[test]
    fn test_snapshot_latest_none_when_empty() {
        let dir = tempdir().unwrap();
        let mgr = SnapshotManager::with_dir(dir.path().to_path_buf());

        let latest = mgr.latest_snapshot("nonexistent").unwrap();
        assert!(latest.is_none());
    }

    #[test]
    fn test_snapshot_list_multiple() {
        let dir = tempdir().unwrap();
        let mgr = SnapshotManager::with_dir(dir.path().to_path_buf());

        let ws = create_test_workload_state();
        mgr.create_snapshot(&ws).unwrap();
        // Small sleep to ensure different nanosecond timestamp in filename
        std::thread::sleep(std::time::Duration::from_millis(1));
        mgr.create_snapshot(&ws).unwrap();

        let snapshots = mgr.list_snapshots("test-workload").unwrap();
        assert_eq!(snapshots.len(), 2);
        // Sorted, so first should be older
        assert!(snapshots[0] < snapshots[1]);
    }

    #[test]
    fn test_encrypt_blob_roundtrip() {
        // Uses the ambient (machine-derived) key — no global env mutation, so this
        // is safe to run alongside other tests in parallel.
        let plaintext = r#"{"hello":"world","n":42}"#;
        let ct = crate::secrets::encrypt_blob(plaintext).unwrap();
        assert_ne!(ct, plaintext, "ciphertext must differ from plaintext");
        let pt = crate::secrets::decrypt_blob(&ct).unwrap();
        assert_eq!(pt, plaintext);
    }

    #[test]
    fn test_backup_encrypted_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("enc-backup.json");

        let mut state = StateStore::new();
        state.upsert("test".to_string(), create_test_workload_state());
        let backup = Backup::from_state(&state, Some("enc".to_string()));

        // Simulate an encrypted-at-rest backup via the shared crypto path.
        let json = serde_json::to_string_pretty(&backup).unwrap();
        let data = crate::secrets::encrypt_blob(&json).unwrap();
        let envelope = BackupEnvelope {
            encrypted: true,
            data,
        };
        fs::write(&path, serde_json::to_string(&envelope).unwrap()).unwrap();

        // On-disk contents must not leak the workload name.
        let raw = fs::read_to_string(&path).unwrap();
        assert!(raw.contains("\"encrypted\""));
        assert!(!raw.contains("test-workload"));

        // load() auto-detects the envelope and decrypts.
        let loaded = Backup::load(&path).unwrap();
        assert_eq!(loaded.workloads.len(), 1);
        assert_eq!(loaded.metadata.description, Some("enc".to_string()));
    }

    #[test]
    fn test_backup_aux_stores_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("aux-backup.json");

        let mut state = StateStore::new();
        state.upsert("test".to_string(), create_test_workload_state());
        let mut backup = Backup::from_state(&state, None);
        backup.stores = Some(AuxStores {
            secrets: Some("{\"secrets\":{}}".to_string()),
            rbac: Some("{\"keys\":[]}".to_string()),
            environments: None,
            quotas: Some("{\"quotas\":{}}".to_string()),
        });

        backup.save(&path).unwrap();
        let loaded = Backup::load(&path).unwrap();
        let stores = loaded.stores.expect("aux stores preserved");
        assert_eq!(stores.secrets.as_deref(), Some("{\"secrets\":{}}"));
        assert_eq!(stores.rbac.as_deref(), Some("{\"keys\":[]}"));
        assert!(stores.environments.is_none());
        assert_eq!(stores.quotas.as_deref(), Some("{\"quotas\":{}}"));
    }

    #[test]
    fn test_backup_verify() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("verify-backup.json");
        let mut state = StateStore::new();
        state.upsert("test".to_string(), create_test_workload_state());
        Backup::from_state(&state, None).save(&path).unwrap();

        assert_eq!(Backup::verify(&path).unwrap(), 1);
    }
}
