//! Measured VM image sign / verify catalog.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageManifest {
    pub name: String,
    pub image_hash: String,
    pub kernel_hash: Option<String>,
    pub initrd_hash: Option<String>,
    pub launch_digest: Option<String>,
    pub signing_key_id: String,
    pub signed_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct ImageCatalog {
    path: PathBuf,
    manifests: Arc<Mutex<HashMap<String, ImageManifest>>>,
}

impl ImageCatalog {
    pub fn load(state_dir: &Path) -> Self {
        let path = state_dir.join("ragnarok-images.json");
        let manifests = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };
        Self {
            path,
            manifests: Arc::new(Mutex::new(manifests)),
        }
    }

    pub fn sign(&self, name: &str, image_path: &Path, key_id: &str) -> Result<ImageManifest> {
        let bytes = std::fs::read(image_path)
            .with_context(|| format!("read image {}", image_path.display()))?;
        let image_hash = format!("{:x}", Sha256::digest(&bytes));
        let manifest = ImageManifest {
            name: name.to_string(),
            image_hash: image_hash.clone(),
            kernel_hash: None,
            initrd_hash: None,
            launch_digest: Some(image_hash.clone()),
            signing_key_id: key_id.to_string(),
            signed_at: chrono::Utc::now().to_rfc3339(),
        };
        self.manifests
            .lock()
            .unwrap()
            .insert(name.to_string(), manifest.clone());
        self.persist()?;
        Ok(manifest)
    }

    pub fn verify(&self, name: &str, image_path: &Path) -> Result<bool> {
        let store = self.manifests.lock().unwrap();
        let manifest = store
            .get(name)
            .context("image not in verified catalog — run ragnarok image sign")?;
        let bytes = std::fs::read(image_path)?;
        let hash = format!("{:x}", Sha256::digest(&bytes));
        Ok(hash == manifest.image_hash)
    }

    pub fn verify_digest(&self, digest: &str) -> bool {
        self.manifests
            .lock()
            .unwrap()
            .values()
            .any(|m| m.launch_digest.as_deref() == Some(digest) || m.image_hash == digest)
    }

    pub fn get(&self, name: &str) -> Option<ImageManifest> {
        self.manifests.lock().unwrap().get(name).cloned()
    }

    pub fn list(&self) -> Vec<ImageManifest> {
        self.manifests.lock().unwrap().values().cloned().collect()
    }

    fn persist(&self) -> Result<()> {
        let store = self.manifests.lock().unwrap();
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.path, serde_json::to_string_pretty(&*store)?)?;
        Ok(())
    }
}

pub fn deploy_image_gate(spec: &crate::spec::Workload, catalog: &ImageCatalog) -> Result<()> {
    let Some(conf) = spec.confidential.as_ref() else {
        return Ok(());
    };
    if !conf.enabled {
        return Ok(());
    }
    if let Some(ref digest) = conf.image_digest {
        if !catalog.verify_digest(digest) {
            bail!(
                "confidential image digest '{}' not in Ragnarok verified catalog",
                digest
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_sign_verify_roundtrip() {
        let dir = tempdir().unwrap();
        let img = dir.path().join("test.qcow2");
        std::fs::write(&img, b"fake-qcow2").unwrap();
        let cat = ImageCatalog::load(dir.path());
        cat.sign("ubuntu", &img, "cosign://test").unwrap();
        assert!(cat.verify("ubuntu", &img).unwrap());
    }
}
