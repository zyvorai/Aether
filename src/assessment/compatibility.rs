// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Source→target compatibility preflight.

use crate::discovery::kubernetes::RawInventory;
use crate::discovery::Connection;
use crate::inventory::application::Application;
use anyhow::Result;
use k8s_openapi::api::core::v1::Node;
use k8s_openapi::api::storage::v1::StorageClass;
use kube::api::ListParams;
use kube::{Api, Client};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckStatus {
    Pass,
    Warn,
    Fail,
}

impl CheckStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            CheckStatus::Pass => "✓",
            CheckStatus::Warn => "!",
            CheckStatus::Fail => "✗",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompatCheck {
    pub name: String,
    pub status: CheckStatus,
    pub detail: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompatibilityReport {
    pub target: String,
    pub checks: Vec<CompatCheck>,
}

impl CompatibilityReport {
    pub fn has_failures(&self) -> bool {
        self.checks.iter().any(|c| c.status == CheckStatus::Fail)
    }

    pub fn failure_count(&self) -> usize {
        self.checks
            .iter()
            .filter(|c| c.status == CheckStatus::Fail)
            .count()
    }
}

/// What an application needs from a target cluster, derived from source discovery.
struct AppNeeds {
    storage_classes: Vec<String>,
    needs_default_sc: bool,
    needs_gpu: bool,
    needs_rwx: bool,
}

fn app_needs(app: &Application, source: &RawInventory) -> AppNeeds {
    let mut storage_classes = Vec::new();
    let mut needs_default_sc = false;
    let mut needs_rwx = false;
    for pvc in source.pvcs.iter().filter(|p| p.namespace == app.namespace) {
        if !app.pvcs.contains(&pvc.name) {
            continue;
        }
        match &pvc.storage_class {
            Some(sc) if !sc.is_empty() => storage_classes.push(sc.clone()),
            _ => needs_default_sc = true,
        }
        if pvc.access_modes.iter().any(|m| m == "ReadWriteMany") {
            needs_rwx = true;
        }
    }
    storage_classes.sort();
    storage_classes.dedup();

    let needs_gpu = source
        .workloads
        .iter()
        .filter(|w| {
            w.namespace == app.namespace
                && app.workloads.contains(&format!("{}/{}", w.kind, w.name))
        })
        .any(|w| w.containers.iter().any(|c| c.wants_gpu));

    AppNeeds {
        storage_classes,
        needs_default_sc,
        needs_gpu,
        needs_rwx,
    }
}

/// Probed capabilities of a target cluster.
struct TargetCaps {
    storage_classes: Vec<String>,
    default_storage_class: Option<String>,
    gpu_nodes: usize,
}

async fn probe_target(client: &Client) -> Result<TargetCaps> {
    let sc_api: Api<StorageClass> = Api::all(client.clone());
    let mut storage_classes = Vec::new();
    let mut default_storage_class = None;
    for c in sc_api.list(&ListParams::default()).await?.items {
        let name = c.metadata.name.clone().unwrap_or_default();
        let is_default = c
            .metadata
            .annotations
            .as_ref()
            .and_then(|a| a.get("storageclass.kubernetes.io/is-default-class"))
            .map(|v| v == "true")
            .unwrap_or(false);
        if is_default {
            default_storage_class = Some(name.clone());
        }
        storage_classes.push(name);
    }

    let node_api: Api<Node> = Api::all(client.clone());
    let gpu_nodes = node_api
        .list(&ListParams::default())
        .await
        .map(|list| {
            list.items
                .iter()
                .filter(|n| {
                    let cap = n
                        .status
                        .as_ref()
                        .and_then(|s| s.capacity.as_ref())
                        .map(|c| c.keys().any(|k| k.contains("gpu")))
                        .unwrap_or(false);
                    let lbl = n
                        .metadata
                        .labels
                        .as_ref()
                        .map(|l| l.keys().any(|k| k.contains("gpu")))
                        .unwrap_or(false);
                    cap || lbl
                })
                .count()
        })
        .unwrap_or(0);

    Ok(TargetCaps {
        storage_classes,
        default_storage_class,
        gpu_nodes,
    })
}

/// Run a compatibility preflight of an application against a target connection.
pub async fn check_compatibility(
    app: &Application,
    source: &RawInventory,
    target: &Connection,
) -> Result<CompatibilityReport> {
    let (client, _version) = target.connect().await?;
    let caps = probe_target(&client).await?;
    let needs = app_needs(app, source);

    let mut checks = Vec::new();

    // Storage classes.
    for sc in &needs.storage_classes {
        if caps.storage_classes.contains(sc) {
            checks.push(CompatCheck {
                name: format!("StorageClass '{}'", sc),
                status: CheckStatus::Pass,
                detail: "present on target".to_string(),
            });
        } else if caps.default_storage_class.is_some() {
            checks.push(CompatCheck {
                name: format!("StorageClass '{}'", sc),
                status: CheckStatus::Warn,
                detail: format!(
                    "absent; would fall back to default '{}'",
                    caps.default_storage_class.as_deref().unwrap_or("?")
                ),
            });
        } else {
            checks.push(CompatCheck {
                name: format!("StorageClass '{}'", sc),
                status: CheckStatus::Fail,
                detail: "absent and no default StorageClass on target".to_string(),
            });
        }
    }
    if needs.needs_default_sc {
        checks.push(CompatCheck {
            name: "Default StorageClass".to_string(),
            status: if caps.default_storage_class.is_some() {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            },
            detail: caps
                .default_storage_class
                .clone()
                .unwrap_or_else(|| "no default StorageClass".to_string()),
        });
    }

    // GPU.
    if needs.needs_gpu {
        checks.push(CompatCheck {
            name: "GPU nodes".to_string(),
            status: if caps.gpu_nodes > 0 {
                CheckStatus::Pass
            } else {
                CheckStatus::Fail
            },
            detail: format!("{} GPU-capable node(s) on target", caps.gpu_nodes),
        });
    }

    // RWX — CephFS/NFS-class support can't be proven from class names alone.
    if needs.needs_rwx {
        checks.push(CompatCheck {
            name: "ReadWriteMany".to_string(),
            status: CheckStatus::Warn,
            detail: "app uses RWX; confirm a target StorageClass supports ReadWriteMany"
                .to_string(),
        });
    }

    if checks.is_empty() {
        checks.push(CompatCheck {
            name: "Storage & hardware".to_string(),
            status: CheckStatus::Pass,
            detail: "no special storage/GPU requirements".to_string(),
        });
    }

    Ok(CompatibilityReport {
        target: target.name.clone(),
        checks,
    })
}
