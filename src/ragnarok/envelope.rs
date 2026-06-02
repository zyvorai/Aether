// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Per-VM ephemeral secret envelopes — pending, released, revoked, injected.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvelopeState {
    Pending,
    Released,
    Revoked,
    Injected,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEnvelope {
    pub secret_name: String,
    pub vm_id: String,
    pub provider: String,
    pub state: EnvelopeState,
    pub release_token: Option<String>,
    pub k8s_secret_name: Option<String>,
    pub expires_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct EnvelopeFile {
    vms: HashMap<String, Vec<SecretEnvelope>>,
}

#[derive(Clone)]
pub struct EnvelopeStore {
    path: PathBuf,
    inner: Arc<Mutex<EnvelopeFile>>,
}

impl EnvelopeStore {
    pub fn load(state_dir: &Path) -> Self {
        let path = state_dir.join("attest-secret-envelopes.json");
        let inner = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            EnvelopeFile::default()
        };
        Self {
            path,
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    fn persist(&self) -> Result<()> {
        let data = self.inner.lock().unwrap().clone();
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(&data)?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, json)?;
        std::fs::rename(tmp, &self.path)?;
        Ok(())
    }

    pub fn register_pending(
        &self,
        vm_id: &str,
        secret_names: &[String],
        provider: &str,
    ) -> Result<()> {
        if secret_names.is_empty() {
            return Ok(());
        }
        let now = chrono::Utc::now().to_rfc3339();
        let mut file = self.inner.lock().unwrap();
        let entries = file.vms.entry(vm_id.to_string()).or_default();
        for name in secret_names {
            if entries.iter().any(|e| e.secret_name == *name) {
                continue;
            }
            entries.push(SecretEnvelope {
                secret_name: name.clone(),
                vm_id: vm_id.to_string(),
                provider: provider.to_string(),
                state: EnvelopeState::Pending,
                release_token: None,
                k8s_secret_name: None,
                expires_at: None,
                updated_at: now.clone(),
            });
        }
        drop(file);
        self.persist()
    }

    pub fn list_for_vm(&self, vm_id: &str) -> Vec<SecretEnvelope> {
        self.inner
            .lock()
            .unwrap()
            .vms
            .get(vm_id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn mark_released(
        &self,
        vm_id: &str,
        secret_name: &str,
        token: &str,
        expires_at: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut file = self.inner.lock().unwrap();
        let Some(entries) = file.vms.get_mut(vm_id) else {
            return Ok(());
        };
        for entry in entries.iter_mut() {
            if entry.secret_name == secret_name {
                entry.state = EnvelopeState::Released;
                entry.release_token = Some(token.to_string());
                entry.expires_at = Some(expires_at.to_string());
                entry.updated_at = now.clone();
            }
        }
        drop(file);
        self.persist()
    }

    pub fn mark_injected(
        &self,
        vm_id: &str,
        secret_name: &str,
        k8s_secret_name: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let mut file = self.inner.lock().unwrap();
        let Some(entries) = file.vms.get_mut(vm_id) else {
            return Ok(());
        };
        for entry in entries.iter_mut() {
            if entry.secret_name == secret_name {
                entry.state = EnvelopeState::Injected;
                entry.k8s_secret_name = Some(k8s_secret_name.to_string());
                entry.updated_at = now.clone();
            }
        }
        drop(file);
        self.persist()
    }

    pub fn revoke_vm(&self, vm_id: &str) -> usize {
        let now = chrono::Utc::now().to_rfc3339();
        let mut file = self.inner.lock().unwrap();
        let Some(entries) = file.vms.get_mut(vm_id) else {
            return 0;
        };
        let mut n = 0;
        for entry in entries.iter_mut() {
            if entry.state == EnvelopeState::Pending || entry.state == EnvelopeState::Released {
                entry.state = EnvelopeState::Revoked;
                entry.release_token = None;
                entry.updated_at = now.clone();
                n += 1;
            }
        }
        drop(file);
        let _ = self.persist();
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn envelope_lifecycle() {
        let dir = tempdir().unwrap();
        let store = EnvelopeStore::load(dir.path());
        store
            .register_pending("vm-a", &["db-pass".into()], "vault")
            .unwrap();
        assert_eq!(store.list_for_vm("vm-a").len(), 1);
        assert_eq!(store.list_for_vm("vm-a")[0].state, EnvelopeState::Pending);

        store
            .mark_released("vm-a", "db-pass", "tok", "2099-01-01T00:00:00Z")
            .unwrap();
        assert_eq!(store.list_for_vm("vm-a")[0].state, EnvelopeState::Released);

        store.revoke_vm("vm-a");
        assert_eq!(store.list_for_vm("vm-a")[0].state, EnvelopeState::Revoked);
    }
}
