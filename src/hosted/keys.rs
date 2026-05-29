// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Per-tenant API keys (scoped Operator access).

use anyhow::{bail, Result};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantKeyRecord {
    pub name: String,
    pub tenant_id: String,
    pub key_hash: String,
    pub created_at: String,
    pub active: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TenantKeyStoreData {
    keys: HashMap<String, TenantKeyRecord>,
}

pub struct TenantKeyStore {
    path: PathBuf,
    inner: TenantKeyStoreData,
}

impl TenantKeyStore {
    pub fn load() -> Self {
        let path = crate::resources::aether_path("tenant-keys.json");
        let inner = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            TenantKeyStoreData::default()
        };
        Self { path, inner }
    }

    pub fn issue(&mut self, tenant_id: &str, name: &str) -> Result<(String, TenantKeyRecord)> {
        let plaintext = generate_key();
        let hash = hash_key(&plaintext);
        let rec = TenantKeyRecord {
            name: name.to_string(),
            tenant_id: tenant_id.to_string(),
            key_hash: hash.clone(),
            created_at: crate::resources::now_rfc3339(),
            active: true,
        };
        self.inner.keys.insert(hash, rec.clone());
        self.persist()?;
        Ok((plaintext, rec))
    }

    pub fn verify(&self, token: &str) -> Option<&TenantKeyRecord> {
        self.inner.keys.get(&hash_key(token)).filter(|k| k.active)
    }

    pub fn revoke(&mut self, tenant_id: &str, name: &str) -> Result<()> {
        let key = self
            .inner
            .keys
            .iter_mut()
            .find(|(_, v)| v.tenant_id == tenant_id && v.name == name)
            .map(|(k, _)| k.clone());
        let Some(hash) = key else {
            bail!("tenant key not found");
        };
        if let Some(rec) = self.inner.keys.get_mut(&hash) {
            rec.active = false;
        }
        self.persist()
    }

    pub fn list_for_tenant(&self, tenant_id: &str) -> Vec<TenantKeyRecord> {
        self.inner
            .keys
            .values()
            .filter(|k| k.tenant_id == tenant_id)
            .cloned()
            .collect()
    }

    fn persist(&self) -> Result<()> {
        atomic_write(&self.path, &self.inner)
    }
}

fn generate_key() -> String {
    use base64::Engine;
    let mut bytes = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut bytes);
    format!(
        "aeth_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

fn hash_key(key: &str) -> String {
    format!("{:x}", Sha256::digest(key.as_bytes()))
}

fn atomic_write<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_string_pretty(value)?)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_and_verify_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        let mut store = TenantKeyStore::load();
        let (plain, rec) = store.issue("tenant-1", "ci").unwrap();
        assert!(store.verify(&plain).is_some());
        assert_eq!(rec.tenant_id, "tenant-1");
    }
}
