// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Confidential Containers Key Broker Service (KBS) resource fetch.

use anyhow::{Context, Result};
use serde_json::Value;

pub struct KbsClient {
    base: String,
}

impl KbsClient {
    pub fn from_env() -> Option<Self> {
        let base = std::env::var("KBS_URL")
            .ok()
            .filter(|s| !s.trim().is_empty())?;
        Some(Self {
            base: base.trim_end_matches('/').to_string(),
        })
    }

    /// Fetch a resource after attestation (CoCo KBS-style path).
    pub async fn fetch_resource(
        &self,
        resource_id: &str,
        attestation_token: &str,
    ) -> Result<String> {
        let url = format!("{}/kbs/v0/resource/{resource_id}", self.base);
        let resp = reqwest::Client::new()
            .get(&url)
            .header("Authorization", format!("Bearer {attestation_token}"))
            .send()
            .await
            .context("kbs request")?
            .error_for_status()
            .context("kbs resource fetch failed")?
            .json::<Value>()
            .await?;

        if let Some(s) = resp.as_str() {
            return Ok(s.to_string());
        }
        if let Some(payload) = resp.get("payload").and_then(|v| v.as_str()) {
            return Ok(payload.to_string());
        }
        Ok(resp.to_string())
    }
}
