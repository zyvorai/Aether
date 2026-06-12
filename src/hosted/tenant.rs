// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Tenant registry for hosted multi-tenant control plane.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum TenantPlan {
    #[default]
    Free,
    Team,
    Enterprise,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub slug: String,
    pub plan: TenantPlan,
    pub created_at: String,
    pub api_key_prefix: String,
    #[serde(default)]
    pub labels: HashMap<String, String>,
    pub active: bool,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct TenantStoreData {
    tenants: HashMap<String, Tenant>,
}

pub struct TenantStore {
    path: PathBuf,
    inner: TenantStoreData,
}

impl TenantStore {
    pub fn load() -> Self {
        let path = crate::resources::aether_path("tenants.json");
        let inner = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            TenantStoreData::default()
        };
        Self { path, inner }
    }

    pub fn list(&self) -> Vec<Tenant> {
        let mut v: Vec<_> = self.inner.tenants.values().cloned().collect();
        v.sort_by(|a, b| a.slug.cmp(&b.slug));
        v
    }

    pub fn get(&self, id: &str) -> Option<Tenant> {
        self.inner.tenants.get(id).cloned()
    }

    pub fn get_by_slug(&self, slug: &str) -> Option<Tenant> {
        self.inner
            .tenants
            .values()
            .find(|t| t.slug == slug)
            .cloned()
    }

    pub fn create(&mut self, name: &str, slug: &str, plan: TenantPlan) -> Result<Tenant> {
        let slug = normalize_slug(slug);
        if self.get_by_slug(&slug).is_some() {
            bail!("tenant slug '{slug}' already exists");
        }
        let id = format!("tenant-{}", short_id());
        let tenant = Tenant {
            id: id.clone(),
            name: name.to_string(),
            slug,
            plan,
            created_at: crate::resources::now_rfc3339(),
            api_key_prefix: format!("aeth_{}", &id[id.len().saturating_sub(6)..]),
            labels: HashMap::new(),
            active: true,
        };
        self.inner.tenants.insert(id, tenant.clone());
        self.persist()?;
        Ok(tenant)
    }

    pub fn deactivate(&mut self, id: &str) -> Result<()> {
        let t = self.inner.tenants.get_mut(id).context("tenant not found")?;
        t.active = false;
        self.persist()
    }

    pub fn upgrade_plan(&mut self, id: &str, plan: TenantPlan) -> Result<()> {
        let t = self.inner.tenants.get_mut(id).context("tenant not found")?;
        t.plan = plan;
        self.persist()
    }

    pub fn set_stripe_customer_id(&mut self, id: &str, customer_id: &str) -> Result<()> {
        let t = self.inner.tenants.get_mut(id).context("tenant not found")?;
        t.labels
            .insert("stripe_customer_id".into(), customer_id.to_string());
        self.persist()
    }

    pub fn stripe_customer_id(&self, id: &str) -> Option<String> {
        self.inner
            .tenants
            .get(id)
            .and_then(|t| t.labels.get("stripe_customer_id"))
            .cloned()
            .filter(|s| !s.is_empty())
    }

    fn persist(&self) -> Result<()> {
        atomic_write(&self.path, &self.inner)
    }
}

pub fn tenant_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Ok(v) = std::env::var("AETHER_TENANT_ID") {
        if !v.is_empty() {
            return Some(v);
        }
    }
    headers
        .get("x-aether-tenant")
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn enforce_tenant_isolation(tenant_id: Option<&str>, resource_owner: Option<&str>) -> bool {
    match (tenant_id, resource_owner) {
        (None, _) => true,
        (Some(t), Some(o)) => o == t || o.starts_with(&format!("{t}:")),
        (Some(_), None) => true,
    }
}

fn normalize_slug(slug: &str) -> String {
    slug.trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    format!(
        "{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u32
    )
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
    fn create_tenant_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        let mut store = TenantStore::load();
        let t = store.create("Acme", "acme-corp", TenantPlan::Team).unwrap();
        assert_eq!(store.get(&t.id).unwrap().slug, "acme-corp");
        store.upgrade_plan(&t.id, TenantPlan::Enterprise).unwrap();
        assert_eq!(store.get(&t.id).unwrap().plan, TenantPlan::Enterprise);
    }

    #[test]
    fn stripe_customer_id_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        let mut store = TenantStore::load();
        let t = store.create("Acme", "acme", TenantPlan::Free).unwrap();
        assert!(store.stripe_customer_id(&t.id).is_none());
        store
            .set_stripe_customer_id(&t.id, "cus_test123")
            .unwrap();
        assert_eq!(
            store.stripe_customer_id(&t.id).as_deref(),
            Some("cus_test123")
        );
    }
}
