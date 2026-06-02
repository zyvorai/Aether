// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Usage metering and billing summaries for hosted tenants.

use crate::hosted::tenant::{Tenant, TenantPlan, TenantStore};
use crate::state::StateStore;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TenantUsageLine {
    pub tenant_id: String,
    pub tenant_slug: String,
    pub plan: String,
    pub workload_count: usize,
    pub cluster_workloads_estimated: usize,
    pub api_requests_estimate: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BillingSummary {
    pub period: String,
    pub tenants: Vec<TenantUsageLine>,
    pub total_workloads: usize,
}

pub fn usage_summary(store: &StateStore, tenants: &TenantStore) -> BillingSummary {
    let period = chrono::Utc::now().format("%Y-%m").to_string();
    let global_workloads = store.workloads.len();
    let tenant_list = tenants.list();

    let lines: Vec<TenantUsageLine> = if tenant_list.is_empty() {
        vec![TenantUsageLine {
            tenant_id: "default".into(),
            tenant_slug: "default".into(),
            plan: "self-hosted".into(),
            workload_count: global_workloads,
            cluster_workloads_estimated: 0,
            api_requests_estimate: 0,
        }]
    } else {
        tenant_list
            .iter()
            .map(|t| tenant_usage_line(t, store, global_workloads, tenant_list.len()))
            .collect()
    };

    BillingSummary {
        total_workloads: global_workloads,
        period,
        tenants: lines,
    }
}

fn tenant_usage_line(t: &Tenant, store: &StateStore, total: usize, n: usize) -> TenantUsageLine {
    let owned = store
        .workloads
        .values()
        .filter(|w| w.name.starts_with(&format!("{}:", t.slug)) || w.name.contains(&t.slug))
        .count();
    let share = if n > 0 { total / n } else { 0 };
    TenantUsageLine {
        tenant_id: t.id.clone(),
        tenant_slug: t.slug.clone(),
        plan: format!("{:?}", t.plan),
        workload_count: if owned > 0 { owned } else { share },
        cluster_workloads_estimated: share,
        api_requests_estimate: plan_request_quota(&t.plan),
    }
}

fn plan_request_quota(plan: &TenantPlan) -> u64 {
    match plan {
        TenantPlan::Free => 10_000,
        TenantPlan::Team => 100_000,
        TenantPlan::Enterprise => 1_000_000,
    }
}
