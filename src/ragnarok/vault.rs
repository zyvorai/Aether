// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! HashiCorp Vault dynamic secret fetch for attestation-gated release.

use anyhow::{bail, Context, Result};
use serde_json::Value;

pub struct VaultClient {
    addr: String,
    token: String,
}

impl VaultClient {
    pub fn from_env() -> Option<Self> {
        let addr = std::env::var("VAULT_ADDR").ok()?;
        let token = std::env::var("VAULT_TOKEN").ok()?;
        if addr.trim().is_empty() || token.trim().is_empty() {
            return None;
        }
        Some(Self {
            addr: addr.trim_end_matches('/').to_string(),
            token,
        })
    }

    pub async fn read_kv2(&self, secret_path: &str) -> Result<String> {
        let path = secret_path.trim_start_matches('/');
        let url = format!("{}/v1/secret/data/{}", self.addr, path);
        let resp = reqwest::Client::new()
            .get(&url)
            .header("X-Vault-Token", &self.token)
            .send()
            .await
            .context("vault request")?
            .error_for_status()
            .context("vault read failed")?
            .json::<Value>()
            .await?;

        extract_kv2_payload(&resp)
    }
}

fn extract_kv2_payload(body: &Value) -> Result<String> {
    let data = body
        .pointer("/data/data")
        .context("vault response missing data.data")?;
    if let Some(s) = data.as_str() {
        return Ok(s.to_string());
    }
    if let Some(obj) = data.as_object() {
        if let Some((_, v)) = obj.iter().next() {
            if let Some(s) = v.as_str() {
                return Ok(s.to_string());
            }
            return Ok(v.to_string());
        }
    }
    bail!("vault secret has no string payload")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kv2_string() {
        let body = serde_json::json!({
            "data": { "data": { "password": "s3cret" } }
        });
        assert_eq!(extract_kv2_payload(&body).unwrap(), "s3cret");
    }
}
