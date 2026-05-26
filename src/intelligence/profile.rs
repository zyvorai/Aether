// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Behavioral workload profiling for adaptive runtime intelligence.

use crate::health::HealthHistory;
use crate::intelligence::store::IntelligenceStore;
use crate::spec::Workload;
use crate::state::WorkloadState;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkloadBehaviorProfile {
    pub workload_name: String,
    pub cpu_bursty: bool,
    pub memory_stable: bool,
    pub network_intensive: bool,
    pub disk_heavy: bool,
    pub startup_sensitive: bool,
    pub gpu_contention: bool,
    pub restart_velocity: f64,
    pub uptime_pct: f64,
    pub classification: String,
    pub updated_at: String,
}

pub struct BehaviorProfiler;

impl BehaviorProfiler {
    pub fn profile_workload(workload: &Workload, ws: &WorkloadState) -> WorkloadBehaviorProfile {
        let health = HealthHistory::load(&HealthHistory::default_path()).unwrap_or_default();
        let uptime = health.uptime_percent(&workload.metadata.name);
        let restarts = health.restart_count(&workload.metadata.name) as f64;

        let cpu_req = crate::resources::parse_cpu(&workload.requirements.cpu);
        let mem_req = crate::resources::parse_memory_gi(&workload.requirements.memory);
        let has_gpu = workload.requirements.gpu.is_some();
        let has_persistence = workload.persistence.enabled;
        let has_service = workload.network.service;
        let has_ingress = workload.ingress.as_ref().is_some();

        WorkloadBehaviorProfile {
            workload_name: workload.metadata.name.clone(),
            cpu_bursty: cpu_req >= 2.0 && has_service,
            memory_stable: mem_req >= 1.0 && uptime > 95.0,
            network_intensive: has_service && has_ingress,
            disk_heavy: has_persistence && mem_req >= 4.0,
            startup_sensitive: workload.health.is_some() && restarts > 2.0,
            gpu_contention: has_gpu,
            restart_velocity: restarts,
            uptime_pct: uptime,
            classification: classify_behavior(workload, ws),
            updated_at: crate::resources::now_rfc3339(),
        }
    }

    pub fn refresh_fleet(workloads: &[(Workload, WorkloadState)]) -> anyhow::Result<usize> {
        let path = IntelligenceStore::default_path();
        let mut store = IntelligenceStore::load(&path).unwrap_or_default();
        for (spec, ws) in workloads {
            let profile = Self::profile_workload(spec, ws);
            store.upsert_behavior_profile(profile);
        }
        let count = store.behavior_profiles.len();
        store.save(&path)?;
        Ok(count)
    }
}

fn classify_behavior(workload: &Workload, ws: &WorkloadState) -> String {
    if workload.requirements.gpu.is_some() {
        return "gpu_compute".into();
    }
    if workload.persistence.enabled {
        return "stateful".into();
    }
    if workload.network.service {
        return format!("stateless_{}", ws.runtime).to_lowercase();
    }
    "general".into()
}
