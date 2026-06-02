// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Edge job execution and offline queue replay.

use crate::fleet::edge::EdgeJob;
use crate::gitops::{GitOpsConfig, GitOpsController};
use crate::kubecluster::ClusterActionRequest;
use crate::runtime::{create_runtime, RuntimeKind};
use crate::spec::Workload;
use crate::state::StateStore;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeJobResult {
    pub job_id: String,
    pub action: String,
    pub success: bool,
    pub message: String,
}

pub async fn execute_edge_job(job: &EdgeJob) -> Result<String> {
    match job.action.as_str() {
        "gitops_sync" => gitops_sync()
            .await
            .map(|n| format!("gitops sync: {n} change(s)")),
        "stop" => {
            let name = workload_from_payload(&job.payload)?;
            let cascade = job
                .payload
                .get("cascade")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            stop_workload(&name, cascade).await?;
            Ok(format!("stopped {name}"))
        }
        "delete" => {
            let name = workload_from_payload(&job.payload)?;
            delete_workload(&name).await?;
            Ok(format!("deleted {name}"))
        }
        "restart" => {
            let name = workload_from_payload(&job.payload)?;
            restart_managed_workload(&name).await?;
            Ok(format!("restarted {name}"))
        }
        "start" => {
            let name = workload_from_payload(&job.payload)?;
            start_managed_workload(&name).await?;
            Ok(format!("started {name}"))
        }
        "drift_reconcile" => {
            let name = workload_from_payload(&job.payload)?;
            reconcile_drift(&name).await?;
            Ok(format!("drift reconciled for {name}"))
        }
        "cluster_action" => {
            let req = cluster_action_from_payload(&job.payload)?;
            crate::kubecluster::workload_action(&req).await
        }
        other => bail!("unsupported edge action '{other}'"),
    }
}

pub async fn execute_edge_jobs(jobs: &[EdgeJob]) -> Vec<EdgeJobResult> {
    let mut out = Vec::with_capacity(jobs.len());
    for job in jobs {
        let result = match execute_edge_job(job).await {
            Ok(message) => EdgeJobResult {
                job_id: job.id.clone(),
                action: job.action.clone(),
                success: true,
                message,
            },
            Err(e) => EdgeJobResult {
                job_id: job.id.clone(),
                action: job.action.clone(),
                success: false,
                message: e.to_string(),
            },
        };
        out.push(result);
    }
    out
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct LocalQueueData {
    jobs: Vec<EdgeJob>,
}

pub struct EdgeLocalQueue {
    path: PathBuf,
    inner: LocalQueueData,
}

impl EdgeLocalQueue {
    pub fn for_site(site: &str) -> Self {
        Self::load_from_path(Self::site_queue_path(site))
    }

    fn site_queue_path(site: &str) -> PathBuf {
        let safe = site
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>();
        crate::resources::aether_path(&format!("edge-{safe}-offline.json"))
    }

    fn load_from_path(path: PathBuf) -> Self {
        let inner = if path.exists() {
            std::fs::read_to_string(&path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            LocalQueueData::default()
        };
        Self { path, inner }
    }

    #[cfg(test)]
    fn for_site_in(base: &Path, site: &str) -> Self {
        let safe = site
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
            .collect::<String>();
        Self::load_from_path(base.join(format!("edge-{safe}-offline.json")))
    }

    pub fn len(&self) -> usize {
        self.inner.jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.jobs.is_empty()
    }

    pub fn push(&mut self, job: EdgeJob) -> Result<()> {
        self.inner.jobs.push(job);
        self.persist()
    }

    pub async fn replay_all(&mut self) -> (usize, usize) {
        let jobs = std::mem::take(&mut self.inner.jobs);
        let mut ok = 0usize;
        let mut fail = 0usize;
        let mut retry = Vec::new();
        for job in jobs {
            match execute_edge_job(&job).await {
                Ok(_) => ok += 1,
                Err(_) => {
                    fail += 1;
                    retry.push(job);
                }
            }
        }
        self.inner.jobs = retry;
        let _ = self.persist();
        (ok, fail)
    }

    fn persist(&self) -> Result<()> {
        atomic_write_json(&self.path, &self.inner)
    }
}

fn workload_from_payload(payload: &serde_json::Value) -> Result<String> {
    payload
        .get("workload")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .context("payload.workload is required")
}

fn cluster_action_from_payload(payload: &serde_json::Value) -> Result<ClusterActionRequest> {
    let cluster = payload
        .get("cluster")
        .and_then(|v| v.as_str())
        .context("payload.cluster is required")?
        .to_string();
    let namespace = payload
        .get("namespace")
        .and_then(|v| v.as_str())
        .unwrap_or("default")
        .to_string();
    let kind = payload
        .get("kind")
        .and_then(|v| v.as_str())
        .context("payload.kind is required")?
        .to_string();
    let name = payload
        .get("name")
        .and_then(|v| v.as_str())
        .context("payload.name is required")?
        .to_string();
    let action = payload
        .get("action")
        .and_then(|v| v.as_str())
        .context("payload.action is required")?
        .to_string();
    Ok(ClusterActionRequest {
        cluster,
        namespace,
        kind,
        name,
        action,
        replicas: payload
            .get("replicas")
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        api_version: payload
            .get("api_version")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        plural: payload
            .get("plural")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        namespaced: payload.get("namespaced").and_then(|v| v.as_bool()),
    })
}

async fn gitops_sync() -> Result<usize> {
    let state_path = crate::resources::aether_path("gitops.json");
    if !state_path.exists() {
        bail!("gitops is not configured");
    }
    let data = std::fs::read_to_string(&state_path)?;
    let config: GitOpsConfig = serde_json::from_str(&data)?;
    let mut ctrl = GitOpsController::new(config);
    let changes = ctrl.sync()?;
    let status_json = serde_json::to_string_pretty(ctrl.status())?;
    std::fs::write(&state_path, status_json)?;
    Ok(changes.len())
}

async fn stop_workload(name: &str, cascade: bool) -> Result<()> {
    let state_path = StateStore::default_path();
    let state = StateStore::load(&state_path)?;
    let ws = state.get(name).cloned().context("workload not found")?;
    if cascade && ws.runtime == RuntimeKind::Kubernetes {
        let ns = std::env::var("AETHER_NAMESPACE").unwrap_or_else(|_| "default".into());
        let kube = crate::adapters::KubernetesRuntime::with_namespace(ns).await?;
        kube.stop_cascade(&ws.instance).await?;
    } else {
        let rt = create_runtime(&ws.runtime).await?;
        rt.stop(&ws.instance).await?;
    }
    Ok(())
}

async fn delete_workload(name: &str) -> Result<()> {
    let state_path = StateStore::default_path();
    let mut state = StateStore::load(&state_path)?;
    let ws = state.get(name).cloned().context("workload not found")?;
    let rt = create_runtime(&ws.runtime).await?;
    rt.delete(&ws.instance).await?;
    state.remove(name);
    state.save(&state_path)?;
    Ok(())
}

async fn restart_managed_workload(name: &str) -> Result<()> {
    let state_path = StateStore::default_path();
    let mut state = StateStore::load(&state_path)?;
    let ws = state.get(name).cloned().context("workload not found")?;
    let rt = create_runtime(&ws.runtime).await?;
    rt.stop(&ws.instance).await?;
    let spec = Workload::from_file(&ws.spec_path)?;
    let image = rt.build(&spec).await?;
    let instance = rt.run(&image, &spec).await?;
    state.upsert(name.to_string(), ws.migrated(ws.runtime, instance));
    state.save(&state_path)?;
    Ok(())
}

async fn start_managed_workload(name: &str) -> Result<()> {
    let state_path = StateStore::default_path();
    let state = StateStore::load(&state_path)?;
    let ws = state.get(name).context("workload not found")?;
    let rt = create_runtime(&ws.runtime).await?;
    let status = rt.status(&ws.instance).await?;
    if status.ready {
        return Ok(());
    }
    restart_managed_workload(name).await
}

async fn reconcile_drift(name: &str) -> Result<()> {
    use crate::drift::{execute_reconciliation, DriftDetector};
    let state_path = StateStore::default_path();
    let state = StateStore::load(&state_path)?;
    let ws = state.get(name).context("workload not found")?.clone();
    let spec = Workload::from_file(&ws.spec_path)?;
    let report = DriftDetector::new().detect(&spec, &ws);
    if !report.has_drift {
        return Ok(());
    }
    let mut state = state;
    let _ = execute_reconciliation(&report, &mut state).await?;
    state.save(&state_path)?;
    Ok(())
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
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
    use crate::fleet::edge::EdgeJob;

    #[test]
    fn cluster_action_payload_parse() {
        let req = cluster_action_from_payload(&serde_json::json!({
            "cluster": "prod",
            "namespace": "apps",
            "kind": "Deployment",
            "name": "api",
            "action": "restart",
        }))
        .unwrap();
        assert_eq!(req.cluster, "prod");
        assert_eq!(req.action, "restart");
    }

    #[tokio::test]
    async fn unsupported_action_errors() {
        let job = EdgeJob {
            id: "j1".into(),
            site: "lab".into(),
            action: "unknown".into(),
            payload: serde_json::json!({}),
            created_at: "now".into(),
        };
        assert!(execute_edge_job(&job).await.is_err());
    }

    #[test]
    fn local_queue_push_and_persist() {
        let dir = tempfile::tempdir().unwrap();
        let mut q = EdgeLocalQueue::for_site_in(dir.path(), "dc1");
        q.push(EdgeJob {
            id: "j1".into(),
            site: "dc1".into(),
            action: "gitops_sync".into(),
            payload: serde_json::json!({}),
            created_at: "now".into(),
        })
        .unwrap();
        assert_eq!(q.len(), 1);
        assert_eq!(EdgeLocalQueue::for_site_in(dir.path(), "dc1").len(), 1);
    }
}
