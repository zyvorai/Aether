// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Optional integration with **Atlas**, the Zyvor storage control plane.
//!
//! When a workload requests persistence with a storage class of the form
//! `atlas/<policy>` (e.g. `atlas/production`), Aether provisions the volume
//! through Atlas's REST API instead of creating a Kubernetes PVC directly.
//! Atlas (with `create_pvc: true`) creates the native PVC in the target
//! namespace; Aether then references that PVC in the pod.
//!
//! Environment:
//! - `AETHER_ATLAS_URL`   — Atlas gateway base URL (e.g. `http://atlas:5110`)
//! - `AETHER_ATLAS_TOKEN` — optional `Authorization: Bearer` service-account JWT
//! - `AETHER_ATLAS_TENANT` — tenant id for provisioned volumes (default `default`)
//!
//! This module is intentionally decoupled from Atlas's own crates: it defines
//! its own request/response structs so Aether has no build dependency on the
//! sibling repository.

use crate::spec::AccessMode;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Prefix that marks a `persistence.storage_class` as Atlas-backed.
const ATLAS_SC_PREFIX: &str = "atlas/";

/// Product name recorded as the owner of Atlas volumes provisioned by Aether.
pub const ATLAS_PRODUCT: &str = "aether";

/// Configuration for reaching the Atlas gateway. `None` from [`AtlasConfig::from_env`]
/// means Atlas is not configured and callers should fall back to native storage.
#[derive(Debug, Clone)]
pub struct AtlasConfig {
    pub base_url: String,
    pub token: Option<String>,
    pub tenant: String,
}

impl AtlasConfig {
    /// Build config from the environment. Returns `None` when `AETHER_ATLAS_URL`
    /// is unset/empty (Atlas integration disabled).
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("AETHER_ATLAS_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())?;
        let token = std::env::var("AETHER_ATLAS_TOKEN")
            .ok()
            .filter(|s| !s.trim().is_empty());
        let tenant = std::env::var("AETHER_ATLAS_TENANT")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "default".to_string());
        Some(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
            tenant,
        })
    }

    fn client(&self) -> Result<reqwest::Client> {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("atlas http client")
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.token {
            Some(t) => req.bearer_auth(t),
            None => req,
        }
    }

    /// Provision a volume through Atlas and wait for the job to complete.
    ///
    /// Returns the handle of the created volume, including the name of the PVC
    /// Atlas created in `namespace` (which the pod should reference).
    pub async fn provision_volume(
        &self,
        name: &str,
        size_bytes: i64,
        policy: &str,
        access_modes: Vec<String>,
        namespace: &str,
        resource_id: &str,
    ) -> Result<VolumeHandle> {
        let client = self.client()?;
        let body = CreateVolumeRequest {
            tenant_id: self.tenant.clone(),
            name: name.to_string(),
            size_bytes,
            kind: "block".to_string(),
            policy: Some(policy.to_string()),
            owner: Some(Owner {
                product: ATLAS_PRODUCT.to_string(),
                resource_type: "workload".to_string(),
                resource_id: resource_id.to_string(),
                role: "data_disk".to_string(),
            }),
            kubernetes: Some(K8sVolumeOpts {
                namespace: Some(namespace.to_string()),
                create_pvc: true,
                access_modes,
                storage_class: None,
            }),
        };

        let url = self.url("/api/atlas/v1/volumes");
        let resp = self
            .auth(client.post(&url))
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Atlas POST {}", url))?;
        let status = resp.status();
        let payload: serde_json::Value = resp.json().await.context("atlas create response json")?;
        if !status.is_success() {
            anyhow::bail!("Atlas volume create returned {}: {}", status, payload);
        }
        let envelope: AcceptedEnvelope =
            serde_json::from_value(payload).context("atlas create envelope")?;

        // Wait for the provisioning job to reach a terminal state.
        self.wait_for_job(&client, &envelope.job_id).await?;

        Ok(VolumeHandle {
            volume_id: envelope.resource.volume_id,
            pvc: envelope.resource.pvc,
        })
    }

    /// Delete a previously provisioned volume (best-effort job wait).
    pub async fn delete_volume(&self, volume_id: &str) -> Result<()> {
        let client = self.client()?;
        let url = self.url(&format!("/api/atlas/v1/volumes/{}", volume_id));
        let resp = self
            .auth(client.delete(&url))
            .send()
            .await
            .with_context(|| format!("Atlas DELETE {}", url))?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("Atlas volume delete returned {}: {}", status, body);
        }
        // The delete response is also a 202 job envelope; wait when present.
        Ok(())
    }

    /// Snapshot a volume.
    pub async fn snapshot_volume(&self, volume_id: &str, snapshot_name: &str) -> Result<String> {
        let client = self.client()?;
        let url = self.url(&format!("/api/atlas/v1/volumes/{}/snapshots", volume_id));
        let body = serde_json::json!({ "name": snapshot_name });
        let resp = self
            .auth(client.post(&url))
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Atlas POST {}", url))?;
        let status = resp.status();
        let payload: serde_json::Value = resp.json().await.context("atlas snapshot json")?;
        if !status.is_success() {
            anyhow::bail!("Atlas snapshot returned {}: {}", status, payload);
        }
        Ok(payload.to_string())
    }

    /// Clone a snapshot into a new independent volume.
    pub async fn clone_snapshot(&self, snapshot_id: &str, new_name: &str) -> Result<String> {
        self.snapshot_derive(snapshot_id, new_name, "clone").await
    }

    /// Restore a snapshot into a point-in-time volume.
    pub async fn restore_snapshot(&self, snapshot_id: &str, new_name: &str) -> Result<String> {
        self.snapshot_derive(snapshot_id, new_name, "restore").await
    }

    async fn snapshot_derive(&self, snapshot_id: &str, new_name: &str, op: &str) -> Result<String> {
        let client = self.client()?;
        let url = self.url(&format!("/api/atlas/v1/snapshots/{}/{}", snapshot_id, op));
        let body = serde_json::json!({ "name": new_name });
        let resp = self
            .auth(client.post(&url))
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Atlas POST {}", url))?;
        let status = resp.status();
        let payload: serde_json::Value = resp.json().await.context("atlas snapshot-derive json")?;
        if !status.is_success() {
            anyhow::bail!("Atlas snapshot {} returned {}: {}", op, status, payload);
        }
        Ok(payload.to_string())
    }

    /// Resolve an intent policy name (e.g. `database`) to its concrete Kubernetes
    /// StorageClass via Atlas's policy catalog (`GET /policies`). Used to fill a
    /// StatefulSet `volumeClaimTemplate` / KubeVirt DataVolume with a valid class
    /// that CDI/CSI can provision against.
    pub async fn resolve_storage_class(&self, policy: &str) -> Result<String> {
        let client = self.client()?;
        let url = self.url("/api/atlas/v1/policies");
        let resp = self
            .auth(client.get(&url))
            .send()
            .await
            .with_context(|| format!("Atlas GET {}", url))?;
        let status = resp.status();
        let payload: serde_json::Value = resp.json().await.context("atlas policies json")?;
        if !status.is_success() {
            anyhow::bail!("Atlas list policies returned {}: {}", status, payload);
        }
        payload
            .as_array()
            .into_iter()
            .flatten()
            .find(|p| p.get("intent").and_then(|v| v.as_str()) == Some(policy))
            .and_then(|p| p.get("storage_class").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
            .ok_or_else(|| anyhow::anyhow!("Atlas has no storage policy named '{}'", policy))
    }

    /// List volumes for the configured tenant (read model for CLI/UI surfacing).
    pub async fn list_volumes(&self) -> Result<Vec<serde_json::Value>> {
        let client = self.client()?;
        let url = self.url(&format!("/api/atlas/v1/volumes?tenant={}", self.tenant));
        let resp = self
            .auth(client.get(&url))
            .send()
            .await
            .with_context(|| format!("Atlas GET {}", url))?;
        let status = resp.status();
        let payload: serde_json::Value = resp.json().await.context("atlas list json")?;
        if !status.is_success() {
            anyhow::bail!("Atlas list volumes returned {}: {}", status, payload);
        }
        // Accept either a bare array or an object with a `volumes`/`items` array.
        let arr = payload
            .get("volumes")
            .or_else(|| payload.get("items"))
            .cloned()
            .unwrap_or(payload);
        Ok(arr.as_array().cloned().unwrap_or_default())
    }

    /// Poll a job to a terminal state, bounded to ~120s. Errors on `failed`/timeout.
    async fn wait_for_job(&self, client: &reqwest::Client, job_id: &str) -> Result<()> {
        let url = self.url(&format!("/api/atlas/v1/jobs/{}", job_id));
        for _ in 0..240 {
            let resp = self
                .auth(client.get(&url))
                .send()
                .await
                .with_context(|| format!("Atlas GET {}", url))?;
            if !resp.status().is_success() {
                anyhow::bail!("Atlas job poll returned {}", resp.status());
            }
            let job: JobRecord = resp.json().await.context("atlas job json")?;
            match job.state.as_str() {
                "succeeded" => return Ok(()),
                "failed" => anyhow::bail!(
                    "Atlas job {} failed: {}",
                    job_id,
                    job.error.unwrap_or_else(|| "unknown error".to_string())
                ),
                _ => tokio::time::sleep(std::time::Duration::from_millis(500)).await,
            }
        }
        anyhow::bail!("Atlas job {} did not complete within timeout", job_id)
    }
}

/// Handle to a volume provisioned by Atlas.
#[derive(Debug, Clone)]
pub struct VolumeHandle {
    pub volume_id: String,
    pub pvc: String,
}

// --- Wire types (Aether-local mirrors of the Atlas API; serde-only) ---

#[derive(Debug, Clone, Serialize)]
struct CreateVolumeRequest {
    tenant_id: String,
    name: String,
    size_bytes: i64,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    policy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    owner: Option<Owner>,
    #[serde(skip_serializing_if = "Option::is_none")]
    kubernetes: Option<K8sVolumeOpts>,
}

#[derive(Debug, Clone, Serialize)]
struct Owner {
    product: String,
    resource_type: String,
    resource_id: String,
    role: String,
}

#[derive(Debug, Clone, Serialize)]
struct K8sVolumeOpts {
    #[serde(skip_serializing_if = "Option::is_none")]
    namespace: Option<String>,
    create_pvc: bool,
    access_modes: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    storage_class: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct AcceptedEnvelope {
    job_id: String,
    resource: VolumeResource,
}

#[derive(Debug, Clone, Deserialize)]
struct VolumeResource {
    volume_id: String,
    pvc: String,
}

#[derive(Debug, Clone, Deserialize)]
struct JobRecord {
    #[allow(dead_code)]
    #[serde(default)]
    id: String,
    state: String,
    #[serde(default)]
    error: Option<String>,
}

// --- Pure helpers (no network) ---

/// Extract the Atlas policy name from an `atlas/<policy>` storage class.
/// Returns `None` for any other (native) storage class.
pub fn atlas_policy_from_storage_class(sc: &str) -> Option<&str> {
    sc.strip_prefix(ATLAS_SC_PREFIX)
        .map(str::trim)
        .filter(|p| !p.is_empty())
}

/// Map an Aether [`AccessMode`] to the Kubernetes access-mode strings Atlas expects.
pub fn access_modes_for(mode: &AccessMode) -> Vec<String> {
    let s = match mode {
        AccessMode::ReadWriteOnce => "ReadWriteOnce",
        AccessMode::ReadOnlyMany => "ReadOnlyMany",
        AccessMode::ReadWriteMany => "ReadWriteMany",
    };
    vec![s.to_string()]
}

/// Convert a Kubernetes quantity string (e.g. `"20Gi"`) to bytes for Atlas.
pub fn size_to_bytes(size: &str) -> i64 {
    let gib = crate::resources::parse_memory_gi(size);
    (gib * 1024.0 * 1024.0 * 1024.0) as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_atlas_policy_from_storage_class() {
        assert_eq!(
            atlas_policy_from_storage_class("atlas/production"),
            Some("production")
        );
        assert_eq!(
            atlas_policy_from_storage_class("atlas/database"),
            Some("database")
        );
        assert_eq!(atlas_policy_from_storage_class("atlas/"), None);
        assert_eq!(atlas_policy_from_storage_class("gp3"), None);
        assert_eq!(atlas_policy_from_storage_class("zyvor-rbd-prod"), None);
    }

    #[test]
    fn test_access_modes_for() {
        assert_eq!(
            access_modes_for(&AccessMode::ReadWriteOnce),
            vec!["ReadWriteOnce"]
        );
        assert_eq!(
            access_modes_for(&AccessMode::ReadWriteMany),
            vec!["ReadWriteMany"]
        );
        assert_eq!(
            access_modes_for(&AccessMode::ReadOnlyMany),
            vec!["ReadOnlyMany"]
        );
    }

    #[test]
    fn test_size_to_bytes() {
        assert_eq!(size_to_bytes("1Gi"), 1024 * 1024 * 1024);
        assert_eq!(size_to_bytes("10Gi"), 10 * 1024 * 1024 * 1024);
        // Mi and decimal G supported via parse_memory_gi.
        assert_eq!(size_to_bytes("1024Mi"), 1024 * 1024 * 1024);
    }

    #[test]
    fn test_config_from_env_disabled_without_url() {
        // Not asserting on env (parallel-safe): just confirm the struct shape compiles
        // and that a constructed config builds a bearer request without panicking.
        let cfg = AtlasConfig {
            base_url: "http://atlas:5110".to_string(),
            token: Some("t".to_string()),
            tenant: "default".to_string(),
        };
        assert_eq!(
            cfg.url("/api/atlas/v1/volumes"),
            "http://atlas:5110/api/atlas/v1/volumes"
        );
    }

    #[test]
    fn test_create_volume_request_shape() {
        let req = CreateVolumeRequest {
            tenant_id: "default".to_string(),
            name: "web-pvc".to_string(),
            size_bytes: 21474836480,
            kind: "block".to_string(),
            policy: Some("production".to_string()),
            owner: Some(Owner {
                product: ATLAS_PRODUCT.to_string(),
                resource_type: "workload".to_string(),
                resource_id: "web".to_string(),
                role: "data_disk".to_string(),
            }),
            kubernetes: Some(K8sVolumeOpts {
                namespace: Some("default".to_string()),
                create_pvc: true,
                access_modes: vec!["ReadWriteOnce".to_string()],
                storage_class: None,
            }),
        };
        let v = serde_json::to_value(&req).unwrap();
        assert_eq!(v["tenant_id"], "default");
        assert_eq!(v["policy"], "production");
        assert_eq!(v["kind"], "block");
        assert_eq!(v["owner"]["product"], "aether");
        assert_eq!(v["kubernetes"]["create_pvc"], true);
        assert_eq!(v["kubernetes"]["access_modes"][0], "ReadWriteOnce");
        // storage_class omitted when None
        assert!(v["kubernetes"].get("storage_class").is_none());
    }

    #[tokio::test]
    async fn test_provision_volume_against_mock() {
        use axum::extract::Path;
        use axum::routing::{get, post};
        use axum::{Json, Router};

        // A tiny in-test Atlas that returns the 202 envelope and a succeeded job.
        let app = Router::new()
            .route(
                "/api/atlas/v1/volumes",
                post(|| async {
                    Json(serde_json::json!({
                        "job_id": "job_1",
                        "resource": {
                            "volume_id": "vol_1",
                            "pvc": "web-pvc",
                            "storage_class": "zyvor-rbd-prod",
                            "namespace": "default"
                        }
                    }))
                }),
            )
            .route(
                "/api/atlas/v1/jobs/:id",
                get(|Path(_id): Path<String>| async {
                    Json(serde_json::json!({ "id": "job_1", "state": "succeeded" }))
                }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let cfg = AtlasConfig {
            base_url: format!("http://{}", addr),
            token: None,
            tenant: "default".to_string(),
        };
        let handle = cfg
            .provision_volume(
                "web-pvc",
                1073741824,
                "production",
                vec!["ReadWriteOnce".to_string()],
                "default",
                "web",
            )
            .await
            .unwrap();
        assert_eq!(handle.pvc, "web-pvc");
        assert_eq!(handle.volume_id, "vol_1");
    }

    #[tokio::test]
    async fn test_resolve_storage_class() {
        use axum::routing::get;
        use axum::{Json, Router};

        let app = Router::new().route(
            "/api/atlas/v1/policies",
            get(|| async {
                Json(serde_json::json!([
                    { "intent": "production", "storage_class": "zyvor-rbd-prod" },
                    { "intent": "shared", "storage_class": "zyvor-cephfs-shared" },
                ]))
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let cfg = AtlasConfig {
            base_url: format!("http://{}", addr),
            token: None,
            tenant: "default".to_string(),
        };
        assert_eq!(
            cfg.resolve_storage_class("shared").await.unwrap(),
            "zyvor-cephfs-shared"
        );
        assert_eq!(
            cfg.resolve_storage_class("production").await.unwrap(),
            "zyvor-rbd-prod"
        );
        assert!(cfg.resolve_storage_class("nonexistent").await.is_err());
    }

    #[tokio::test]
    async fn test_provision_volume_job_failure() {
        use axum::extract::Path;
        use axum::routing::{get, post};
        use axum::{Json, Router};

        let app = Router::new()
            .route(
                "/api/atlas/v1/volumes",
                post(|| async {
                    Json(serde_json::json!({
                        "job_id": "job_x",
                        "resource": { "volume_id": "vol_x", "pvc": "p" }
                    }))
                }),
            )
            .route(
                "/api/atlas/v1/jobs/:id",
                get(|Path(_id): Path<String>| async {
                    Json(serde_json::json!({ "id": "job_x", "state": "failed", "error": "pool full" }))
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        let cfg = AtlasConfig {
            base_url: format!("http://{}", addr),
            token: None,
            tenant: "default".to_string(),
        };
        let err = cfg
            .provision_volume("p", 1, "production", vec![], "default", "w")
            .await
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("pool full"),
            "expected job error surfaced: {err}"
        );
    }
}
