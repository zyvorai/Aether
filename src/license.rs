// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

//! Trial license enforcement for Aether.
//!
//! ## Trial mode (no key required)
//! Every binary ships with a BUILD_DATE baked in at compile time (build.rs).
//! Without a license key the binary runs freely for 30 days from that date,
//! then exits with an upgrade message pointing to sales@zyvor.dev.
//!
//! ## Licensed mode
//! Paid/extended customers receive a key from sales.
//! Key format: <base64url-payload>.<base64url-hmac>
//! Payload JSON: {"p":"aether","iss":"2026-06-21","exp":"2026-07-21","who":"Acme Corp"}
//!
//! Key lookup order (only checked when a key is provided):
//!   1. AETHER_LICENSE_KEY env var
//!   2. ~/.config/aether/license.key
//!   3. /etc/aether/license.key

use anyhow::{bail, Result};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use chrono::{Duration, NaiveDate, Utc};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::path::PathBuf;

const HMAC_SECRET: &[u8] = b"zyvor-aether-trial-v1-3b5e7f1a9c2d4e6f";

const BUILD_DATE: &str = env!("AETHER_BUILD_DATE");
const TRIAL_DAYS: i64 = 30;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug)]
pub struct License {
    pub licensee: String,
    pub issued: NaiveDate,
    pub expires: NaiveDate,
}

impl License {
    pub fn days_remaining(&self) -> i64 {
        let today = Utc::now().date_naive();
        (self.expires - today).num_days()
    }

    pub fn is_valid(&self) -> bool {
        self.days_remaining() > 0
    }
}

/// Load and validate the licence. Returns `Ok(License)` if allowed to run.
///
/// - If a key is configured, validates it via HMAC-SHA256.
/// - Otherwise runs on the 30-day build-date trial — no key required.
pub fn load_and_check() -> Result<License> {
    match find_key() {
        Ok(raw) => check_key(&raw),
        Err(_) => check_trial(),
    }
}

fn check_key(raw: &str) -> Result<License> {
    let lic = parse_and_verify(raw)?;
    if !lic.is_valid() {
        bail!(
            "Aether licence expired on {}.\n\
             Contact sales@zyvor.dev to renew.\n\
             https://zyvor.dev/aether",
            lic.expires
        );
    }
    let days = lic.days_remaining();
    if days <= 5 {
        eprintln!(
            "⚠  Aether licence expires in {} day(s) ({}). Contact sales@zyvor.dev.",
            days, lic.expires
        );
    }
    Ok(lic)
}

fn find_key() -> Result<String> {
    if let Ok(k) = std::env::var("AETHER_LICENSE_KEY") {
        if !k.trim().is_empty() {
            return Ok(k.trim().to_string());
        }
    }
    for path in &[dirs_key_path(), PathBuf::from("/etc/aether/license.key")] {
        if path.exists() {
            let content = std::fs::read_to_string(path)
                .map_err(|e| anyhow::anyhow!("Cannot read licence key at {}: {}", path.display(), e))?;
            let trimmed = content.trim().to_string();
            if !trimmed.is_empty() {
                return Ok(trimmed);
            }
        }
    }
    bail!("no key configured")
}

fn check_trial() -> Result<License> {
    let issued = NaiveDate::parse_from_str(BUILD_DATE, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("Internal error: bad build date '{}'", BUILD_DATE))?;
    let expires = issued + Duration::days(TRIAL_DAYS);
    let today = Utc::now().date_naive();
    let days_remaining = (expires - today).num_days();

    if days_remaining <= 0 {
        bail!(
            "Aether 30-day trial has expired (build: {}).\n\
             \n\
             To continue, contact sales@zyvor.dev for a licence key.\n\
             https://zyvor.dev/contact?intent=trial&product=aether",
            issued
        );
    }
    if days_remaining <= 5 {
        eprintln!(
            "⚠  Aether trial expires in {} day(s) ({}). Contact sales@zyvor.dev.",
            days_remaining, expires
        );
    }

    Ok(License {
        licensee: "Trial".to_string(),
        issued,
        expires,
    })
}

fn dirs_key_path() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());
    PathBuf::from(format!("{}/.config/aether/license.key", home))
}

fn parse_and_verify(raw: &str) -> Result<License> {
    let parts: Vec<&str> = raw.splitn(2, '.').collect();
    if parts.len() != 2 {
        bail!("Malformed licence key (expected <payload>.<signature>)");
    }
    let payload_b64 = parts[0];
    let sig_b64 = parts[1];

    let mut mac = HmacSha256::new_from_slice(HMAC_SECRET)
        .map_err(|_| anyhow::anyhow!("HMAC init failed"))?;
    mac.update(payload_b64.as_bytes());
    let sig_bytes = URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|_| anyhow::anyhow!("Invalid licence key signature encoding"))?;
    mac.verify_slice(&sig_bytes)
        .map_err(|_| anyhow::anyhow!("Licence key signature is invalid — contact sales@zyvor.dev"))?;

    let payload_bytes = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| anyhow::anyhow!("Invalid licence key payload encoding"))?;
    let payload: serde_json::Value = serde_json::from_slice(&payload_bytes)
        .map_err(|_| anyhow::anyhow!("Malformed licence key payload"))?;

    if payload.get("p").and_then(|v| v.as_str()) != Some("aether") {
        bail!("This licence key is not valid for Aether");
    }

    let issued = parse_date(&payload, "iss")?;
    let expires = parse_date(&payload, "exp")?;
    let licensee = payload
        .get("who")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown")
        .to_string();

    Ok(License { licensee, issued, expires })
}

fn parse_date(payload: &serde_json::Value, key: &str) -> Result<NaiveDate> {
    let s = payload
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("Missing '{}' in licence payload", key))?;
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| anyhow::anyhow!("Invalid date '{}' in licence payload", s))
}
