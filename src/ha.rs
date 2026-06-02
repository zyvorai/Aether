// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Optional Redis-backed cache for multi-node API deployments.
//!
//! Used for OIDC login state (PKCE verifier + nonce). When `AETHER_REDIS_URL` is unset,
//! an in-process map is used (single-node only).

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const OIDC_PENDING_PREFIX: &str = "aether:oidc:pending:";
const SAML_PENDING_PREFIX: &str = "aether:saml:pending:";

#[derive(Clone)]
pub struct SharedCache {
    inner: Arc<Inner>,
}

enum Inner {
    Memory(Mutex<CacheMaps>),
    Redis(redis::aio::ConnectionManager),
}

struct CacheMaps {
    oidc: HashMap<String, PendingOidcJson>,
    saml: HashMap<String, PendingSamlJson>,
}

impl CacheMaps {
    fn new() -> Self {
        Self {
            oidc: HashMap::new(),
            saml: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingOidcJson {
    pub nonce: String,
    pub pkce_verifier: String,
    pub exp_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingSamlJson {
    pub relay_state: String,
    pub exp_unix: u64,
}

impl SharedCache {
    pub async fn connect(redis_url: Option<&str>) -> anyhow::Result<Self> {
        if let Some(url) = redis_url.filter(|u| !u.is_empty()) {
            let client = redis::Client::open(url).context("invalid AETHER_REDIS_URL")?;
            let mgr = redis::aio::ConnectionManager::new(client)
                .await
                .context("failed to connect to Redis")?;
            tracing::info!("HA: Redis shared cache enabled for OIDC state");
            return Ok(Self {
                inner: Arc::new(Inner::Redis(mgr)),
            });
        }
        tracing::info!("HA: in-memory OIDC state cache (set AETHER_REDIS_URL for multi-node)");
        Ok(Self {
            inner: Arc::new(Inner::Memory(Mutex::new(CacheMaps::new()))),
        })
    }

    pub async fn redis_ping_ok(&self) -> bool {
        match self.inner.as_ref() {
            Inner::Redis(m) => {
                let mut c = m.clone();
                redis::cmd("PING")
                    .query_async::<String>(&mut c)
                    .await
                    .map(|p| p == "PONG")
                    .unwrap_or(false)
            }
            Inner::Memory(_) => true,
        }
    }

    pub async fn put_oidc_pending(
        &self,
        csrf: &str,
        pending: &PendingOidcJson,
    ) -> anyhow::Result<()> {
        let key = format!("{OIDC_PENDING_PREFIX}{csrf}");
        let payload = serde_json::to_string(pending)?;
        match self.inner.as_ref() {
            Inner::Redis(m) => {
                use redis::AsyncCommands;
                let mut c = m.clone();
                let ttl: u64 = 600;
                let _: () = c.set_ex(&key, payload, ttl).await?;
            }
            Inner::Memory(map) => {
                let mut g = map.lock().await;
                g.oidc.retain(|_, v| v.exp_unix > now_unix());
                g.oidc.insert(key, pending.clone());
            }
        }
        Ok(())
    }

    pub async fn take_oidc_pending(&self, csrf: &str) -> anyhow::Result<Option<PendingOidcJson>> {
        let key = format!("{OIDC_PENDING_PREFIX}{csrf}");
        match self.inner.as_ref() {
            Inner::Redis(m) => {
                use redis::AsyncCommands;
                let mut c = m.clone();
                let v: Option<String> = c.get(&key).await?;
                if v.is_some() {
                    let _: () = c.del(&key).await?;
                }
                let Some(s) = v else { return Ok(None) };
                let p: PendingOidcJson = serde_json::from_str(&s)?;
                if p.exp_unix <= now_unix() {
                    return Ok(None);
                }
                Ok(Some(p))
            }
            Inner::Memory(map) => {
                let mut g = map.lock().await;
                g.oidc.retain(|_, v| v.exp_unix > now_unix());
                Ok(g.oidc.remove(&key))
            }
        }
    }

    pub fn uses_redis(&self) -> bool {
        matches!(self.inner.as_ref(), Inner::Redis(_))
    }

    pub async fn put_saml_pending(
        &self,
        request_id: &str,
        pending: &PendingSamlJson,
    ) -> anyhow::Result<()> {
        let key = format!("{SAML_PENDING_PREFIX}{request_id}");
        let payload = serde_json::to_string(pending)?;
        match self.inner.as_ref() {
            Inner::Redis(m) => {
                use redis::AsyncCommands;
                let mut c = m.clone();
                let ttl: u64 = 600;
                let _: () = c.set_ex(&key, payload, ttl).await?;
            }
            Inner::Memory(map) => {
                let mut g = map.lock().await;
                g.saml.retain(|_, v| v.exp_unix > now_unix());
                g.saml.insert(key, pending.clone());
            }
        }
        Ok(())
    }

    pub async fn take_saml_pending(
        &self,
        request_id: &str,
    ) -> anyhow::Result<Option<PendingSamlJson>> {
        let key = format!("{SAML_PENDING_PREFIX}{request_id}");
        match self.inner.as_ref() {
            Inner::Redis(m) => {
                use redis::AsyncCommands;
                let mut c = m.clone();
                let v: Option<String> = c.get(&key).await?;
                if v.is_some() {
                    let _: () = c.del(&key).await?;
                }
                let Some(s) = v else { return Ok(None) };
                let p: PendingSamlJson = serde_json::from_str(&s)?;
                if p.exp_unix <= now_unix() {
                    return Ok(None);
                }
                Ok(Some(p))
            }
            Inner::Memory(map) => {
                let mut g = map.lock().await;
                g.saml.retain(|_, v| v.exp_unix > now_unix());
                Ok(g.saml.remove(&key))
            }
        }
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
