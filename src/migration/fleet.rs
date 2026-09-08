// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Coordinated fleet migrations across clusters.

use crate::migration::{MigrationPlan, MigrationStrategy};
use crate::runtime::RuntimeKind;
use crate::spec::Workload;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMigrationRequest {
    pub workloads: Vec<String>,
    pub target_cluster: String,
    pub strategy: Option<String>,
    pub replicate_volumes: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMigrationItem {
    pub workload: String,
    pub source_runtime: String,
    pub target_runtime: String,
    pub target_cluster: String,
    pub strategy: String,
    pub volume_replication: bool,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FleetMigrationPlan {
    pub target_cluster: String,
    pub strategy: String,
    pub items: Vec<FleetMigrationItem>,
    pub warnings: Vec<String>,
}

pub fn plan_fleet_migration(
    req: &FleetMigrationRequest,
    specs: &[(String, Workload, RuntimeKind)],
) -> Result<FleetMigrationPlan> {
    if req.target_cluster.trim().is_empty() {
        bail!("target_cluster is required");
    }
    if specs.is_empty() {
        bail!("no workloads resolved for fleet migration");
    }

    let strategy = req
        .strategy
        .as_deref()
        .unwrap_or("rolling")
        .parse::<MigrationStrategy>()
        .unwrap_or(MigrationStrategy::Rolling);
    let replicate = req.replicate_volumes.unwrap_or(false);

    let mut items = Vec::new();
    for (name, spec, runtime) in specs {
        let target_runtime = if spec.persistence.enabled {
            RuntimeKind::Kubernetes
        } else {
            *runtime
        };
        items.push(FleetMigrationItem {
            workload: name.clone(),
            source_runtime: format!("{runtime:?}"),
            target_runtime: format!("{target_runtime:?}"),
            target_cluster: req.target_cluster.clone(),
            strategy: strategy.to_string(),
            volume_replication: replicate && spec.persistence.enabled,
            status: "planned".into(),
        });
    }

    let mut warnings = Vec::new();
    if replicate {
        warnings.push("Volume replication steps run before workload cutover per item.".into());
    }
    if items.len() > 10 {
        warnings.push(format!(
            "Large fleet migration ({} workloads) — consider batching.",
            items.len()
        ));
    }

    Ok(FleetMigrationPlan {
        target_cluster: req.target_cluster.clone(),
        strategy: strategy.to_string(),
        items,
        warnings,
    })
}

pub fn item_to_migration_plan(
    item: &FleetMigrationItem,
    source: RuntimeKind,
    target: RuntimeKind,
) -> MigrationPlan {
    MigrationPlan::new(
        item.workload.clone(),
        source,
        target,
        item.strategy.parse().unwrap_or(MigrationStrategy::Rolling),
        true,
    )
}
