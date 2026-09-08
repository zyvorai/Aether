// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! PacketWolf live API bridge — proxy when `AETHER_PACKETWOLF_URL` is set.

use anyhow::{Context, Result};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

static LAST_STATUS: OnceLock<Mutex<Option<CachedStatus>>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketWolfConfig {
    pub configured: bool,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketWolfStatus {
    pub configured: bool,
    pub reachable: bool,
    pub version: Option<String>,
    pub hint: Option<String>,
    pub cached: bool,
}

#[derive(Debug, Clone)]
struct CachedStatus {
    status: PacketWolfStatus,
    at: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifyEgressRequest {
    pub namespace: String,
    pub pod: String,
    pub workload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacketWolfDeeplink {
    pub configured: bool,
    pub url: Option<String>,
    pub flows_url: Option<String>,
    pub anomalies_url: Option<String>,
}

pub fn config() -> PacketWolfConfig {
    let base_url = std::env::var("AETHER_PACKETWOLF_URL")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim_end_matches('/').to_string());
    PacketWolfConfig {
        configured: base_url.is_some(),
        base_url,
    }
}

pub async fn status() -> PacketWolfStatus {
    let cfg = config();
    if !cfg.configured {
        return PacketWolfStatus {
            configured: false,
            reachable: false,
            version: None,
            hint: Some("Set AETHER_PACKETWOLF_URL to enable PacketWolf bridge".into()),
            cached: false,
        };
    }
    let base = cfg.base_url.unwrap();
    match probe_health(&base).await {
        Ok(version) => {
            let status = PacketWolfStatus {
                configured: true,
                reachable: true,
                version,
                hint: None,
                cached: false,
            };
            cache_status(status.clone());
            status
        }
        Err(e) => {
            if let Some(cached) = cached_status() {
                return PacketWolfStatus {
                    hint: Some(format!("PacketWolf unreachable: {e}")),
                    cached: true,
                    ..cached
                };
            }
            PacketWolfStatus {
                configured: true,
                reachable: false,
                version: None,
                hint: Some(format!("PacketWolf unreachable: {e}")),
                cached: false,
            }
        }
    }
}

pub async fn flow_stats() -> Result<Value> {
    let base = base_url()?;
    proxy_get(&format!("{base}/api/v1/flows/stats")).await
}

pub async fn verify_egress(req: &VerifyEgressRequest) -> Result<Value> {
    let base = base_url()?;
    proxy_post(
        &format!("{base}/api/v1/policies/verify-egress"),
        &serde_json::to_value(req)?,
    )
    .await
}

pub async fn anomalies(limit: Option<u32>) -> Result<Value> {
    let base = base_url()?;
    let lim = limit.unwrap_or(50);
    proxy_get(&format!("{base}/api/v1/anomalies?limit={lim}")).await
}

pub fn deeplink(
    namespace: Option<&str>,
    pod: Option<&str>,
    workload: Option<&str>,
) -> PacketWolfDeeplink {
    let cfg = config();
    if !cfg.configured {
        return PacketWolfDeeplink {
            configured: false,
            url: None,
            flows_url: None,
            anomalies_url: None,
        };
    }
    let base = cfg.base_url.unwrap();
    let mut q = Vec::new();
    if let Some(ns) = namespace.filter(|s| !s.is_empty()) {
        q.push(format!("namespace={}", urlencoding(ns)));
    }
    if let Some(p) = pod.filter(|s| !s.is_empty()) {
        q.push(format!("pod={}", urlencoding(p)));
    }
    if let Some(w) = workload.filter(|s| !s.is_empty()) {
        q.push(format!("workload={}", urlencoding(w)));
    }
    let suffix = if q.is_empty() {
        String::new()
    } else {
        format!("?{}", q.join("&"))
    };
    PacketWolfDeeplink {
        configured: true,
        url: Some(format!("{base}/ui/flows{suffix}")),
        flows_url: Some(format!("{base}/ui/flows{suffix}")),
        anomalies_url: Some(format!("{base}/ui/anomalies")),
    }
}

fn base_url() -> Result<String> {
    config()
        .base_url
        .context("AETHER_PACKETWOLF_URL not configured")
}

async fn probe_health(base: &str) -> Result<Option<String>> {
    for path in ["/api/v1/health", "/health"] {
        let url = format!("{base}{path}");
        if let Ok(resp) = authed(client().get(&url)).send().await {
            if resp.status().is_success() {
                let body: Value = resp.json().await.unwrap_or(Value::Null);
                let version = body
                    .get("version")
                    .or_else(|| body.pointer("/data/version"))
                    .and_then(|v| v.as_str())
                    .map(str::to_string);
                return Ok(version);
            }
        }
    }
    anyhow::bail!("health probe failed")
}

async fn proxy_get(url: &str) -> Result<Value> {
    let resp = authed(client().get(url))
        .send()
        .await
        .with_context(|| format!("GET {url}"))?;
    let status = resp.status();
    let body: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        anyhow::bail!("PacketWolf returned {status}: {body}");
    }
    Ok(body)
}

async fn proxy_post(url: &str, body: &Value) -> Result<Value> {
    let resp = authed(
        client()
            .post(url)
            .header(CONTENT_TYPE, "application/json")
            .json(body),
    )
    .send()
    .await
    .with_context(|| format!("POST {url}"))?;
    let status = resp.status();
    let out: Value = resp.json().await.unwrap_or(Value::Null);
    if !status.is_success() {
        anyhow::bail!("PacketWolf returned {status}: {out}");
    }
    Ok(out)
}

fn authed(req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Ok(key) = std::env::var("AETHER_PACKETWOLF_API_KEY") {
        if !key.trim().is_empty() {
            return req.header(AUTHORIZATION, format!("Bearer {}", key.trim()));
        }
    }
    req
}

fn client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT
        .get_or_init(|| {
            reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("reqwest client")
        })
        .clone()
}

fn cache_status(status: PacketWolfStatus) {
    let lock = LAST_STATUS.get_or_init(|| Mutex::new(None));
    if let Ok(mut g) = lock.lock() {
        *g = Some(CachedStatus {
            status,
            at: Instant::now(),
        });
    }
}

fn cached_status() -> Option<PacketWolfStatus> {
    let lock = LAST_STATUS.get_or_init(|| Mutex::new(None));
    let g = lock.lock().ok()?;
    let cached = g.as_ref()?;
    if cached.at.elapsed() < Duration::from_secs(300) {
        Some(cached.status.clone())
    } else {
        None
    }
}

fn urlencoding(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unconfigured_status() {
        std::env::remove_var("AETHER_PACKETWOLF_URL");
        let cfg = config();
        assert!(!cfg.configured);
    }

    #[test]
    fn deeplink_without_config() {
        std::env::remove_var("AETHER_PACKETWOLF_URL");
        let link = deeplink(Some("default"), Some("nginx"), None);
        assert!(!link.configured);
    }

    #[tokio::test]
    async fn status_unconfigured() {
        std::env::remove_var("AETHER_PACKETWOLF_URL");
        let s = status().await;
        assert!(!s.configured);
        assert!(!s.reachable);
    }
}
