// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Per-application portability / migration-readiness assessment.
//!
//! Bridges the discovered inventory into a synthetic [`Workload`] so the existing
//! scoring, migration-advice, and cost engines can be reused unchanged.

use crate::ai::migration::{MigrationAdvisor, RiskLevel};
use crate::ai::scoring::ScoringEngine;
use crate::discovery::kubernetes::{DiscoveredWorkload, RawInventory};
use crate::inventory::application::{Application, MigrationClass};
use crate::migration::MigrationStrategy;
use crate::runtime::RuntimeKind;
use crate::spec::{GpuRequirements, Workload};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Complexity {
    Low,
    Medium,
    High,
}

impl std::fmt::Display for Complexity {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(match self {
            Complexity::Low => "Low",
            Complexity::Medium => "Medium",
            Complexity::High => "High",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortabilityAssessment {
    pub application: String,
    pub namespace: String,
    pub class: MigrationClass,
    /// 0–100 portability score (higher = easier/safer to move).
    pub score: u8,
    pub complexity: Complexity,
    pub downtime_class: String,
    pub recommended_target: RuntimeKind,
    pub recommended_strategy: MigrationStrategy,
    pub risk: RiskLevel,
    pub estimated_downtime_secs: u64,
    pub monthly_cost_usd: Option<f64>,
    /// % of dependencies that are portable/internal (external cloud deps lower it).
    pub dependency_completeness: u8,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub remediations: Vec<String>,
    /// Target-compatibility preflight (filled by `assessment::compatibility`).
    #[serde(default)]
    pub compatibility: Option<crate::assessment::compatibility::CompatibilityReport>,
}

/// Assess one application's portability, reusing the scoring/advice/cost engines.
pub fn assess(app: &Application, raw: &RawInventory) -> Result<PortabilityAssessment> {
    let workload = application_to_workload(app, raw).context("building synthetic workload")?;

    // Recommended target runtime (reuse the scoring engine).
    let scoring = ScoringEngine::new(crate::config::EngineConfig::default());
    let score_result = scoring.score(&workload);
    let recommended_target = score_result.recommended;

    // Strategy / risk / downtime (reuse the migration advisor).
    let advisor = MigrationAdvisor::new(crate::config::MigrationConfig::default());
    let advice = advisor.advise(&workload, RuntimeKind::Kubernetes, recommended_target);

    // Cost (reuse the cost comparison across providers).
    let monthly_cost_usd = crate::cost::CostComparison::for_workload(&workload)
        .ok()
        .and_then(|c| {
            c.estimates
                .iter()
                .map(|e| e.total_monthly)
                .fold(None, |acc, v| Some(acc.map_or(v, |a: f64| a.min(v))))
        });

    // Dependency completeness: external cloud endpoints reduce portability.
    let cloud_deps = app
        .external_deps
        .iter()
        .filter(|d| is_cloud_managed(d))
        .count();
    let dependency_completeness = if app.external_deps.is_empty() {
        100
    } else {
        (100u32.saturating_sub((cloud_deps as u32) * 25)).min(100) as u8
    };

    // Portability score: class baseline − penalties.
    let base = match app.class {
        MigrationClass::StatelessPortable => 95,
        MigrationClass::StatefulNative => 80,
        MigrationClass::OperatorManaged => 55,
        MigrationClass::CloudManagedDependency => 60,
        MigrationClass::PrivilegedNodeDependent => 45,
        MigrationClass::NonPortable => 30,
    };
    let penalty = (app.blockers.len() as i32) * 8 + (cloud_deps as i32) * 5;
    let score = (base - penalty).clamp(0, 100) as u8;

    let complexity = match score {
        s if s >= 75 => Complexity::Low,
        s if s >= 50 => Complexity::Medium,
        _ => Complexity::High,
    };

    let downtime_class = downtime_bucket(advice.estimated_downtime_secs);
    let remediations = remediations_for(app);

    Ok(PortabilityAssessment {
        application: app.name.clone(),
        namespace: app.namespace.clone(),
        class: app.class,
        score,
        complexity,
        downtime_class,
        recommended_target,
        recommended_strategy: advice.recommended_strategy,
        risk: advice.risk_level,
        estimated_downtime_secs: advice.estimated_downtime_secs,
        monthly_cost_usd,
        dependency_completeness,
        blockers: app.blockers.clone(),
        warnings: app.warnings.clone(),
        remediations,
        compatibility: None,
    })
}

/// Build a synthetic [`Workload`] from a discovered application so the existing
/// engines (which take `&Workload`) can be reused.
pub fn application_to_workload(app: &Application, raw: &RawInventory) -> Result<Workload> {
    let members: Vec<&DiscoveredWorkload> = raw
        .workloads
        .iter()
        .filter(|w| {
            w.namespace == app.namespace
                && app.workloads.contains(&format!("{}/{}", w.kind, w.name))
        })
        .collect();

    // Aggregate CPU/memory across replicas, defaulting when requests are absent.
    let mut cpu_cores = 0.0f64;
    let mut mem_gib = 0.0f64;
    let mut gpu = 0u32;
    for w in &members {
        let replicas = w.replicas.max(1) as f64;
        for c in &w.containers {
            let cc = c
                .cpu_request
                .as_deref()
                .map(crate::resources::parse_cpu)
                .filter(|v| *v > 0.0)
                .unwrap_or(0.25);
            let cm = c
                .memory_request
                .as_deref()
                .map(crate::resources::parse_memory_gi)
                .filter(|v| *v > 0.0)
                .unwrap_or(0.5);
            cpu_cores += cc * replicas;
            mem_gib += cm * replicas;
            if c.wants_gpu {
                gpu += 1;
            }
        }
    }
    if cpu_cores <= 0.0 {
        cpu_cores = 0.5;
    }
    if mem_gib <= 0.0 {
        mem_gib = 0.5;
    }

    // Storage from the app's PVCs (or a stateful default).
    let mut storage_gib = 0.0f64;
    for pvc in raw.pvcs.iter().filter(|p| p.namespace == app.namespace) {
        if app.pvcs.contains(&pvc.name) {
            if let Some(sz) = &pvc.size {
                storage_gib += crate::resources::parse_memory_gi(sz);
            }
        }
    }
    if storage_gib <= 0.0 {
        storage_gib = if app.pvcs.is_empty() { 1.0 } else { 10.0 };
    }

    let cpu = format!("{}m", (cpu_cores * 1000.0).round() as u64);
    let memory = format!("{}Mi", (mem_gib * 1024.0).round() as u64);
    let storage = format!("{}Gi", storage_gib.round() as u64);

    let yaml = format!(
        "apiVersion: aether/v1\n\
         kind: Workload\n\
         metadata:\n  name: {name}\n  owner: discovered\n  project: {project}\n\
         build:\n  context: \".\"\n  dockerfile: Dockerfile\n  registry: discovered\n\
         requirements:\n  cpu: \"{cpu}\"\n  memory: \"{memory}\"\n  storage: \"{storage}\"\n\
         runtime:\n  preferred: kube\n  allow: [container, kube, kubevirt, metal]\n",
        name = sanitize_name(&app.name),
        project = sanitize_name(&app.namespace),
        cpu = cpu,
        memory = memory,
        storage = storage,
    );

    let mut wl: Workload = serde_yaml::from_str(&yaml).context("parsing synthetic workload yaml")?;
    if gpu > 0 {
        wl.requirements.gpu = Some(GpuRequirements {
            count: gpu,
            vendor: "nvidia".to_string(),
        });
    }
    if !app.pvcs.is_empty() {
        wl.persistence.enabled = true;
        wl.persistence.size = storage;
    }
    Ok(wl)
}

fn sanitize_name(s: &str) -> String {
    let cleaned: String = s
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' { c } else { '-' })
        .collect();
    let trimmed = cleaned.trim_matches('-');
    if trimmed.is_empty() {
        "app".to_string()
    } else {
        trimmed.to_lowercase()
    }
}

fn is_cloud_managed(endpoint: &str) -> bool {
    const FRAGMENTS: &[&str] = &[
        "amazonaws.com",
        "database.azure.com",
        "blob.core.windows.net",
        "cloudsql",
        "googleapis.com",
        "database.windows.net",
    ];
    FRAGMENTS.iter().any(|f| endpoint.contains(f))
}

fn downtime_bucket(secs: u64) -> String {
    match secs {
        0..=60 => "< 1 minute".to_string(),
        61..=600 => "< 10 minutes".to_string(),
        601..=3600 => "< 1 hour".to_string(),
        _ => "> 1 hour".to_string(),
    }
}

fn remediations_for(app: &Application) -> Vec<String> {
    let mut r = Vec::new();
    if app.blockers.iter().any(|b| b.contains("mutable tag")) {
        r.push("Pin container images to immutable tags or digests.".to_string());
    }
    if app.blockers.iter().any(|b| b.contains("naked Pod")) {
        r.push("Wrap naked Pods in a Deployment/StatefulSet.".to_string());
    }
    if app.warnings.iter().any(|w| w.contains("resource requests")) {
        r.push("Add CPU/memory resource requests for correct sizing.".to_string());
    }
    if app.class == MigrationClass::CloudManagedDependency {
        r.push("Decide per cloud dependency: keep, migrate managed, or replace.".to_string());
    }
    if app.class == MigrationClass::OperatorManaged {
        r.push("Install CRDs + a compatible operator on the target before restore.".to_string());
    }
    r
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::discovery::kubernetes::{ContainerInfo, PodSignals};
    use std::collections::BTreeMap;

    fn app_with(class: MigrationClass, blockers: Vec<String>) -> Application {
        Application {
            id: "prod/web".into(),
            name: "web".into(),
            namespace: "prod".into(),
            connection: "c".into(),
            workloads: vec!["Deployment/web".into()],
            services: vec![],
            pvcs: vec![],
            config_map_refs: vec![],
            secret_refs: vec![],
            external_deps: vec![],
            class,
            blockers,
            warnings: vec![],
        }
    }

    fn raw_with_web() -> RawInventory {
        RawInventory {
            connection: "c".into(),
            workloads: vec![DiscoveredWorkload {
                namespace: "prod".into(),
                kind: "Deployment".into(),
                name: "web".into(),
                labels: BTreeMap::new(),
                annotations: BTreeMap::new(),
                owner_refs: vec![],
                replicas: 2,
                containers: vec![ContainerInfo {
                    name: "c".into(),
                    image: "web:1.0".into(),
                    has_requests: true,
                    cpu_request: Some("250m".into()),
                    memory_request: Some("256Mi".into()),
                    wants_gpu: false,
                    privileged: false,
                }],
                signals: PodSignals::default(),
                config_map_refs: vec![],
                secret_refs: vec![],
                pvc_refs: vec![],
                env_endpoints: vec![],
            }],
            ..Default::default()
        }
    }

    #[test]
    fn test_application_to_workload_aggregates() {
        let app = app_with(MigrationClass::StatelessPortable, vec![]);
        let wl = application_to_workload(&app, &raw_with_web()).unwrap();
        // 2 replicas × 250m = 500m ; 2 × 256Mi = 512Mi.
        assert_eq!(wl.requirements.cpu, "500m");
        assert_eq!(wl.requirements.memory, "512Mi");
        assert!(!wl.persistence.enabled);
    }

    #[test]
    fn test_score_monotonicity() {
        let clean = assess(&app_with(MigrationClass::StatelessPortable, vec![]), &raw_with_web()).unwrap();
        let blocked = assess(
            &app_with(MigrationClass::NonPortable, vec!["naked Pod".into(), "mutable tag".into()]),
            &raw_with_web(),
        )
        .unwrap();
        assert!(clean.score > blocked.score);
        assert_eq!(clean.complexity, Complexity::Low);
        assert!(!blocked.remediations.is_empty());
    }
}
