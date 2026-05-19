//! Optional [OPA](https://www.openpolicyagent.org/) integration for manifest and workload admission.
//!
//! Environment:
//! - `AETHER_OPA_URL` — OPA base URL (e.g. `http://opa:8181`)
//! - `AETHER_OPA_PACKAGE` — Rego package path (default `aether.k8s.admit` → `/v1/data/aether/k8s/admit`)
//! - `AETHER_OPA_ENFORCE` — when `1`/`true`, cluster apply and enforced API paths reject on OPA deny

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaConfig {
    pub base_url: String,
    pub package: String,
    pub enforce: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaEvaluation {
    pub configured: bool,
    pub allowed: bool,
    pub denials: Vec<String>,
}

impl OpaConfig {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("AETHER_OPA_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())?;
        let package = std::env::var("AETHER_OPA_PACKAGE")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "aether.k8s.admit".to_string());
        let enforce = std::env::var("AETHER_OPA_ENFORCE")
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        Some(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            package,
            enforce,
        })
    }

    fn data_path(&self) -> String {
        format!("/v1/data/{}", self.package.replace('.', "/"))
    }

    /// Evaluate arbitrary JSON input against the configured OPA data document.
    pub async fn evaluate(&self, input: Value) -> Result<OpaEvaluation> {
        let url = format!("{}{}", self.base_url, self.data_path());
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .context("opa http client")?;
        let body = serde_json::json!({ "input": input });
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("OPA POST {}", url))?;
        let status = resp.status();
        let payload: Value = resp
            .json()
            .await
            .context("opa response json")?;
        if !status.is_success() {
            anyhow::bail!("OPA returned {}: {}", status, payload);
        }
        Ok(parse_opa_result(&payload))
    }

    pub async fn evaluate_manifest(&self, manifest: &Value) -> Result<OpaEvaluation> {
        self.evaluate(serde_json::json!({ "manifest": manifest }))
            .await
    }
}

pub fn configured() -> bool {
    OpaConfig::from_env().is_some()
}

pub fn enforce_enabled() -> bool {
    OpaConfig::from_env().map(|c| c.enforce).unwrap_or(false)
}

/// Evaluate when OPA is configured; otherwise allow.
pub async fn evaluate_manifest_optional(manifest: &Value) -> Result<OpaEvaluation> {
    match OpaConfig::from_env() {
        Some(cfg) => {
            let mut ev = cfg.evaluate_manifest(manifest).await?;
            ev.configured = true;
            Ok(ev)
        }
        None => Ok(OpaEvaluation {
            configured: false,
            allowed: true,
            denials: vec![],
        }),
    }
}

fn parse_opa_result(payload: &Value) -> OpaEvaluation {
    let result = payload.get("result").unwrap_or(payload);
    let mut denials = Vec::new();

    if let Some(arr) = result.get("deny").and_then(|v| v.as_array()) {
        for item in arr {
            if let Some(s) = item.as_str() {
                denials.push(s.to_string());
            } else {
                denials.push(item.to_string());
            }
        }
    }
    if let Some(arr) = result.as_array() {
        for item in arr {
            if let Some(s) = item.as_str() {
                denials.push(s.to_string());
            }
        }
    }

    let allowed = if let Some(b) = result.get("allow").and_then(|v| v.as_bool()) {
        b && denials.is_empty()
    } else if !denials.is_empty() {
        false
    } else if result.is_boolean() {
        result.as_bool().unwrap_or(false)
    } else {
        true
    };

    OpaEvaluation {
        configured: true,
        allowed,
        denials,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_allow_with_deny_list() {
        let payload = serde_json::json!({
            "result": { "allow": false, "deny": ["missing label owner"] }
        });
        let ev = parse_opa_result(&payload);
        assert!(!ev.allowed);
        assert_eq!(ev.denials.len(), 1);
    }

    #[test]
    fn parse_allow_true() {
        let payload = serde_json::json!({ "result": { "allow": true } });
        let ev = parse_opa_result(&payload);
        assert!(ev.allowed);
    }
}
