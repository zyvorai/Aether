// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Edge site registry, heartbeat, and offline reconcile queue.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeAgentRecord {
    pub site: String,
    pub kube_context: Option<String>,
    pub labels: HashMap<String, String>,
    pub registered_at: String,
    pub last_heartbeat: String,
    pub queue_depth: usize,
    pub last_error: Option<String>,
    pub online: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeRegisterRequest {
    pub site: String,
    pub kube_context: Option<String>,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeHeartbeatRequest {
    pub site: String,
    #[serde(default)]
    pub queue_depth: usize,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeEnqueueRequest {
    pub site: String,
    pub action: String,
    #[serde(default)]
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeJob {
    pub id: String,
    pub site: String,
    pub action: String,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct EdgeStoreData {
    agents: HashMap<String, EdgeAgentRecord>,
    jobs: Vec<EdgeJob>,
}

pub struct EdgeStore {
    path: PathBuf,
    queue_path: PathBuf,
    inner: Arc<Mutex<EdgeStoreData>>,
}

impl EdgeStore {
    pub fn load() -> Self {
        let path = crate::resources::aether_path("edge-agents.json");
        let queue_path = crate::resources::aether_path("edge-queue.json");
        let inner = load_json(&path).unwrap_or_default();
        Self {
            path,
            queue_path,
            inner: Arc::new(Mutex::new(inner)),
        }
    }

    pub fn register(&self, req: EdgeRegisterRequest) -> Result<EdgeAgentRecord> {
        let now = crate::resources::now_rfc3339();
        let mut g = self.inner.lock().unwrap();
        let rec = EdgeAgentRecord {
            site: req.site.clone(),
            kube_context: req.kube_context,
            labels: req.labels,
            registered_at: g
                .agents
                .get(&req.site)
                .map(|a| a.registered_at.clone())
                .unwrap_or_else(|| now.clone()),
            last_heartbeat: now,
            queue_depth: pending_count(&g.jobs, &req.site),
            last_error: None,
            online: true,
        };
        g.agents.insert(req.site, rec.clone());
        self.persist(&g)?;
        Ok(rec)
    }

    pub fn heartbeat(&self, req: EdgeHeartbeatRequest) -> Result<EdgeAgentRecord> {
        let now = crate::resources::now_rfc3339();
        let mut g = self.inner.lock().unwrap();
        let agent = g
            .agents
            .get_mut(&req.site)
            .context("edge site not registered")?;
        agent.last_heartbeat = now;
        agent.queue_depth = req.queue_depth;
        agent.last_error = req.last_error;
        agent.online = true;
        let rec = agent.clone();
        self.persist(&g)?;
        Ok(rec)
    }

    pub fn list_agents(&self) -> Vec<EdgeAgentRecord> {
        let g = self.inner.lock().unwrap();
        let mut agents: Vec<_> = g.agents.values().cloned().collect();
        for a in &mut agents {
            a.queue_depth = pending_count(&g.jobs, &a.site);
            a.online = heartbeat_fresh(&a.last_heartbeat);
        }
        agents.sort_by(|a, b| a.site.cmp(&b.site));
        agents
    }

    pub fn enqueue(&self, req: EdgeEnqueueRequest) -> Result<EdgeJob> {
        let mut g = self.inner.lock().unwrap();
        if !g.agents.contains_key(&req.site) {
            anyhow::bail!("edge site '{}' not registered", req.site);
        }
        let job = EdgeJob {
            id: format!("edge-{}", short_id()),
            site: req.site.clone(),
            action: req.action,
            payload: req.payload,
            created_at: crate::resources::now_rfc3339(),
        };
        g.jobs.push(job.clone());
        self.persist(&g)?;
        self.persist_queue(&g.jobs)?;
        Ok(job)
    }

    pub fn poll_queue(&self, site: &str) -> Vec<EdgeJob> {
        let mut g = self.inner.lock().unwrap();
        let (keep, take): (Vec<_>, Vec<_>) = g.jobs.drain(..).partition(|j| j.site != site);
        g.jobs = keep;
        let _ = self.persist(&g);
        let _ = self.persist_queue(&g.jobs);
        take
    }

    pub fn queue_path(&self) -> &Path {
        &self.queue_path
    }

    fn persist(&self, data: &EdgeStoreData) -> Result<()> {
        atomic_write_json(&self.path, data)
    }

    fn persist_queue(&self, jobs: &[EdgeJob]) -> Result<()> {
        atomic_write_json(&self.queue_path, &jobs.to_vec())
    }
}

pub fn edge_token_ok(token: Option<&str>) -> bool {
    match std::env::var("AETHER_EDGE_TOKEN") {
        Ok(expected) if !expected.is_empty() => token == Some(expected.as_str()),
        _ => true,
    }
}

fn pending_count(jobs: &[EdgeJob], site: &str) -> usize {
    jobs.iter().filter(|j| j.site == site).count()
}

fn heartbeat_fresh(last: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(last)
        .map(|t| (chrono::Utc::now() - t.with_timezone(&chrono::Utc)).num_seconds() < 120)
        .unwrap_or(false)
}

fn short_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let n = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{:x}", n & 0xffff_ffff)
}

fn load_json(path: &Path) -> Option<EdgeStoreData> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

fn atomic_write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let text = serde_json::to_string_pretty(value)?;
    std::fs::write(&tmp, &text)?;
    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn register_enqueue_poll() {
        let dir = tempfile::tempdir().unwrap();
        env::set_var("HOME", dir.path());
        let store = EdgeStore::load();
        store
            .register(EdgeRegisterRequest {
                site: "lab".into(),
                kube_context: Some("kind-lab".into()),
                labels: HashMap::new(),
            })
            .unwrap();
        store
            .enqueue(EdgeEnqueueRequest {
                site: "lab".into(),
                action: "gitops_sync".into(),
                payload: serde_json::json!({"spec": "demo"}),
            })
            .unwrap();
        let jobs = store.poll_queue("lab");
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].action, "gitops_sync");
    }
}
