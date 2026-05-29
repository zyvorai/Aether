// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Per-tenant API request metering and quota enforcement.

use crate::hosted::tenant::{TenantPlan, TenantStore};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static CACHE: OnceLock<Mutex<UsageMeter>> = OnceLock::new();

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TenantUsageCounter {
    pub tenant_id: String,
    pub period: String,
    pub requests: u64,
    pub last_path: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct MeterData {
    period: String,
    counters: HashMap<String, TenantUsageCounter>,
}

pub struct UsageMeter {
    path: PathBuf,
    data: MeterData,
}

impl UsageMeter {
    pub fn load() -> Self {
        let path = crate::resources::aether_path("tenant-usage.json");
        let data = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            MeterData::default()
        };
        let mut meter = Self { path, data };
        meter.roll_period_if_needed();
        meter
    }

    fn global() -> std::sync::MutexGuard<'static, UsageMeter> {
        CACHE
            .get_or_init(|| Mutex::new(UsageMeter::load()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    pub fn record(tenant_id: &str, path: &str) {
        let mut meter = Self::global();
        meter.roll_period_if_needed();
        let period = meter.data.period.clone();
        let entry = meter
            .data
            .counters
            .entry(tenant_id.to_string())
            .or_insert_with(|| TenantUsageCounter {
                tenant_id: tenant_id.to_string(),
                period: period.clone(),
                requests: 0,
                last_path: None,
            });
        entry.requests += 1;
        entry.last_path = Some(path.to_string());
        let _ = meter.persist();
    }

    pub fn snapshot(&self) -> Vec<TenantUsageCounter> {
        let mut v: Vec<_> = self.data.counters.values().cloned().collect();
        v.sort_by(|a, b| a.tenant_id.cmp(&b.tenant_id));
        v
    }

    pub fn requests_for(&self, tenant_id: &str) -> u64 {
        self.data
            .counters
            .get(tenant_id)
            .map(|c| c.requests)
            .unwrap_or(0)
    }

    pub fn quota_exceeded(tenant_id: &str) -> bool {
        let tenants = TenantStore::load();
        let Some(tenant) = tenants.get(tenant_id) else {
            return false;
        };
        if !tenant.active {
            return true;
        }
        let meter = Self::global();
        let limit = plan_quota(&tenant.plan);
        meter.requests_for(tenant_id) >= limit
    }

    fn roll_period_if_needed(&mut self) {
        let current = chrono::Utc::now().format("%Y-%m").to_string();
        if self.data.period != current {
            self.data.period = current;
            self.data.counters.clear();
        }
    }

    fn persist(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_string_pretty(&self.data)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

pub fn plan_quota(plan: &TenantPlan) -> u64 {
    match plan {
        TenantPlan::Free => 10_000,
        TenantPlan::Team => 100_000,
        TenantPlan::Enterprise => 1_000_000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_increments_counter() {
        let dir = tempfile::tempdir().unwrap();
        std::env::set_var("HOME", dir.path());
        UsageMeter::record("tenant-a", "/api/workloads");
        let meter = UsageMeter::load();
        assert!(meter.requests_for("tenant-a") >= 1);
    }
}
