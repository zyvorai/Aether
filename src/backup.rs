//! Backup and restore functionality for workload state

use crate::state::{StateStore, WorkloadState};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

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
    /// Orchestr8 version
    pub orchestr8_version: String,
}

/// Complete backup including metadata and state
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Backup {
    /// Backup metadata
    pub metadata: BackupMetadata,
    /// Workload states
    pub workloads: Vec<WorkloadState>,
}

impl Backup {
    /// Create a new backup from current state
    pub fn from_state(state: &StateStore, description: Option<String>) -> Self {
        let workloads: Vec<WorkloadState> = state.list().into_iter().cloned().collect();

        Self {
            metadata: BackupMetadata {
                version: "1.0".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                workload_count: workloads.len(),
                description,
                orchestr8_version: env!("CARGO_PKG_VERSION").to_string(),
            },
            workloads,
        }
    }

    /// Save backup to file
    pub fn save(&self, path: &Path) -> Result<()> {
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize backup")?;

        fs::write(path, json)
            .context(format!("Failed to write backup to {}", path.display()))?;

        tracing::info!("Backup saved to {}", path.display());
        Ok(())
    }

    /// Load backup from file
    pub fn load(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .context(format!("Failed to read backup from {}", path.display()))?;

        let backup: Backup = serde_json::from_str(&contents)
            .context("Failed to deserialize backup")?;

        tracing::info!("Backup loaded from {}", path.display());
        Ok(backup)
    }

    /// Restore backup to state store
    pub fn restore(&self, state_path: &Path) -> Result<()> {
        let mut state = StateStore::new();

        for workload in &self.workloads {
            state.upsert(workload.name.clone(), workload.clone());
        }

        state.save(&state_path.to_path_buf())?;

        tracing::info!(
            "Restored {} workloads from backup",
            self.workloads.len()
        );
        Ok(())
    }

    /// Merge backup into existing state (don't overwrite)
    pub fn merge(&self, state_path: &Path) -> Result<()> {
        let mut state = StateStore::load(&state_path.to_path_buf())
            .unwrap_or_else(|_| StateStore::new());

        let mut merged_count = 0;
        for workload in &self.workloads {
            if state.get(&workload.name).is_none() {
                state.upsert(workload.name.clone(), workload.clone());
                merged_count += 1;
            }
        }

        state.save(&state_path.to_path_buf())?;

        tracing::info!(
            "Merged {} new workloads from backup (total: {})",
            merged_count,
            self.workloads.len()
        );
        Ok(())
    }
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
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".orchestr8");
        path.push("backups");
        path
    }

    /// Ensure backup directory exists
    pub fn ensure_backup_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.backup_dir)
            .context("Failed to create backup directory")?;
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

        let filename = if let Some(n) = name {
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

        for entry in fs::read_dir(&self.backup_dir)
            .context("Failed to read backup directory")?
        {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                backups.push(path);
            }
        }

        backups.sort();
        Ok(backups)
    }

    /// Get backup info without loading full content
    pub fn get_backup_info(&self, path: &Path) -> Result<BackupMetadata> {
        let contents = fs::read_to_string(path)
            .context("Failed to read backup file")?;

        let backup: Backup = serde_json::from_str(&contents)
            .context("Failed to parse backup")?;

        Ok(backup.metadata)
    }

    /// Delete a backup
    pub fn delete_backup(&self, path: &Path) -> Result<()> {
        fs::remove_file(path)
            .context(format!("Failed to delete backup {}", path.display()))?;

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
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            spec_path: PathBuf::from("/tmp/workload.yaml"),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
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
}
