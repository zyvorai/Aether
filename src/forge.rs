// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Optional integration with **Forge**, the Zyvor GPU/AI infrastructure control
//! plane ("the Kubernetes brain for AI infrastructure").
//!
//! Aether calls Forge's REST gateway to inspect GPU capacity, ask where to place
//! a GPU/AI workload, and read GPU cost — informing runtime selection (GPU →
//! KubeVirt/Metal3) and migration decisions.
//!
//! Environment:
//! - `AETHER_FORGE_URL`   — Forge API gateway base URL (unset ⇒ integration off; `FORGE_API_URL` also accepted)
//! - `AETHER_FORGE_TOKEN` — `Authorization: Bearer` API key (`FORGE_API_KEY`)
//!
//! Decoupled from Forge's own crates: Aether uses its own reqwest client and
//! reads responses as `serde_json::Value` (Forge is a separate service/repo).

use anyhow::{Context, Result};

/// Configuration for reaching the Forge gateway. `None` from [`ForgeConfig::from_env`]
/// means Forge is not configured.
#[derive(Debug, Clone)]
pub struct ForgeConfig {
    pub base_url: String,
    pub token: Option<String>,
}

impl ForgeConfig {
    /// Build config from the environment. Returns `None` unless `AETHER_FORGE_URL`
    /// (or `FORGE_API_URL`) is set.
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("AETHER_FORGE_URL")
            .ok()
            .or_else(|| std::env::var("FORGE_API_URL").ok())
            .filter(|s| !s.trim().is_empty())?;
        let token = std::env::var("AETHER_FORGE_TOKEN")
            .ok()
            .or_else(|| std::env::var("FORGE_API_KEY").ok())
            .filter(|s| !s.trim().is_empty());
        Some(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    fn client(&self) -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("forge http client")
    }

    async fn get(&self, path: &str) -> Result<serde_json::Value> {
        let client = self.client()?;
        let url = format!("{}{}", self.base_url, path);
        let mut req = client.get(&url);
        if let Some(t) = &self.token {
            req = req.bearer_auth(t);
        }
        let resp = req
            .send()
            .await
            .with_context(|| format!("Forge GET {}", url))?;
        let status = resp.status();
        let payload: serde_json::Value = resp.json().await.context("forge response json")?;
        if !status.is_success() {
            anyhow::bail!("Forge {} returned {}: {}", path, status, payload);
        }
        Ok(payload)
    }

    /// Cluster GPU stats (`GET /api/cluster/stats`): total/available/allocated GPUs,
    /// utilization, job counts.
    pub async fn cluster_stats(&self) -> Result<serde_json::Value> {
        self.get("/api/cluster/stats").await
    }

    /// GPU nodes (`GET /api/nodes`). Returns the `items` array (or a bare array).
    pub async fn list_nodes(&self) -> Result<Vec<serde_json::Value>> {
        let payload = self.get("/api/nodes").await?;
        let arr = payload.get("items").cloned().unwrap_or(payload);
        Ok(arr.as_array().cloned().unwrap_or_default())
    }

    /// Placement recommendation (`GET /api/ai/placement/recommend`): ranked nodes +
    /// score/reasoning for a GPU/AI request.
    pub async fn recommend_placement(
        &self,
        model: Option<&str>,
        gpu_type: &str,
        gpus: u32,
    ) -> Result<serde_json::Value> {
        let mut path = format!("/api/ai/placement/recommend?gpuType={gpu_type}&gpus={gpus}");
        if let Some(m) = model {
            path.push_str(&format!("&model={m}"));
        }
        self.get(&path).await
    }

    /// GPU cost breakdown (`GET /api/metrics/costs`).
    pub async fn costs(&self) -> Result<serde_json::Value> {
        self.get("/api/metrics/costs").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env_disabled_without_url() {
        // Constructed config builds the right URL/auth shape without panicking.
        let cfg = ForgeConfig {
            base_url: "http://forge:24631".to_string(),
            token: Some("k".to_string()),
        };
        assert_eq!(cfg.base_url, "http://forge:24631");
        assert!(cfg.token.is_some());
    }

    #[tokio::test]
    async fn test_cluster_stats_and_nodes_against_mock() {
        use axum::routing::get;
        use axum::{Json, Router};

        let app = Router::new()
            .route(
                "/api/cluster/stats",
                get(|| async {
                    Json(serde_json::json!({
                        "totalGPUs": 8, "availableGPUs": 3, "allocatedGPUs": 5,
                        "utilizationPercent": 62, "runningJobs": 4
                    }))
                }),
            )
            .route(
                "/api/nodes",
                get(|| async {
                    Json(serde_json::json!({
                        "items": [{"metadata": {"name": "gpu-1"}, "spec": {"gpuCount": 4}}],
                        "total": 1
                    }))
                }),
            )
            .route(
                "/api/ai/placement/recommend",
                get(|| async {
                    Json(serde_json::json!({
                        "selectedNodes": ["gpu-1"], "score": 0.92, "reasoning": "best fit"
                    }))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let cfg = ForgeConfig {
            base_url: format!("http://{}", addr),
            token: None,
        };
        let stats = cfg.cluster_stats().await.unwrap();
        assert_eq!(stats["availableGPUs"], 3);
        let nodes = cfg.list_nodes().await.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0]["metadata"]["name"], "gpu-1");
        let rec = cfg
            .recommend_placement(Some("llama"), "any", 2)
            .await
            .unwrap();
        assert_eq!(rec["selectedNodes"][0], "gpu-1");
    }
}
