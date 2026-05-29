// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Stripe checkout and webhook integration for hosted billing.

use crate::hosted::tenant::{TenantPlan, TenantStore};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCheckoutRequest {
    pub tenant_id: String,
    pub plan: String,
    pub success_url: String,
    pub cancel_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeCheckoutResponse {
    pub session_id: String,
    pub checkout_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StripeWebhookResult {
    pub event_type: String,
    pub tenant_id: Option<String>,
    pub plan: Option<String>,
    pub message: String,
}

pub fn configured() -> bool {
    std::env::var("AETHER_STRIPE_SECRET_KEY")
        .ok()
        .is_some_and(|k| !k.is_empty())
}

fn secret_key() -> Result<String> {
    std::env::var("AETHER_STRIPE_SECRET_KEY").context("AETHER_STRIPE_SECRET_KEY not set")
}

fn webhook_secret() -> Result<String> {
    std::env::var("AETHER_STRIPE_WEBHOOK_SECRET").context("AETHER_STRIPE_WEBHOOK_SECRET not set")
}

fn price_id_for_plan(plan: &str) -> Result<String> {
    let key = match plan.to_lowercase().as_str() {
        "team" => "AETHER_STRIPE_PRICE_TEAM",
        "enterprise" => "AETHER_STRIPE_PRICE_ENTERPRISE",
        _ => bail!("unsupported plan for checkout: {plan}"),
    };
    std::env::var(key).with_context(|| format!("{key} not set"))
}

pub async fn create_checkout_session(req: &StripeCheckoutRequest) -> Result<StripeCheckoutResponse> {
    let tenants = TenantStore::load();
    let tenant = tenants
        .get(&req.tenant_id)
        .context("tenant not found")?;
    if !tenant.active {
        bail!("tenant is inactive");
    }
    let price_id = price_id_for_plan(&req.plan)?;
    let secret = secret_key()?;

    let body = serde_json::json!({
        "mode": "subscription",
        "success_url": req.success_url,
        "cancel_url": req.cancel_url,
        "client_reference_id": tenant.id,
        "metadata": {
            "tenant_id": tenant.id,
            "tenant_slug": tenant.slug,
            "plan": req.plan,
        },
        "line_items": [{ "price": price_id, "quantity": 1 }],
    });

    let client = reqwest::Client::new();
    let resp = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .basic_auth(&secret, Some(""))
        .form(&flatten_json(&body))
        .send()
        .await
        .context("stripe checkout request failed")?;

    let status = resp.status();
    let text = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        bail!("stripe checkout failed ({status}): {text}");
    }

    let parsed: serde_json::Value =
        serde_json::from_str(&text).context("invalid stripe checkout response")?;
    let session_id = parsed
        .get("id")
        .and_then(|v| v.as_str())
        .context("missing session id")?
        .to_string();
    let checkout_url = parsed
        .get("url")
        .and_then(|v| v.as_str())
        .context("missing checkout url")?
        .to_string();

    Ok(StripeCheckoutResponse {
        session_id,
        checkout_url,
    })
}

pub fn verify_webhook_signature(payload: &[u8], sig_header: &str) -> Result<()> {
    let secret = webhook_secret()?;
    let mut timestamp = None;
    let mut signatures = Vec::new();
    for part in sig_header.split(',') {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        match k.trim() {
            "t" => timestamp = Some(v.trim().to_string()),
            "v1" => signatures.push(v.trim().to_string()),
            _ => {}
        }
    }
    let Some(ts) = timestamp else {
        bail!("missing stripe timestamp");
    };
    if signatures.is_empty() {
        bail!("missing stripe signature");
    }
    let signed = format!("{ts}.{}", String::from_utf8_lossy(payload));
    let expected = hmac_sha256_hex(secret.as_bytes(), signed.as_bytes());
    let ok = signatures.iter().any(|s| constant_time_eq(s, &expected));
    if !ok {
        bail!("stripe signature mismatch");
    }
    Ok(())
}

pub fn handle_webhook_event(payload: &[u8]) -> Result<StripeWebhookResult> {
    let event: serde_json::Value =
        serde_json::from_slice(payload).context("invalid stripe event json")?;
    let event_type = event
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    match event_type.as_str() {
        "checkout.session.completed" => {
            let obj = event
                .pointer("/data/object")
                .context("missing checkout session object")?;
            let tenant_id = obj
                .get("client_reference_id")
                .or_else(|| obj.pointer("/metadata/tenant_id"))
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let plan = obj
                .pointer("/metadata/plan")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            if let (Some(tid), Some(plan_str)) = (&tenant_id, &plan) {
                upgrade_tenant_plan(tid, plan_str)?;
            }
            Ok(StripeWebhookResult {
                event_type,
                tenant_id,
                plan,
                message: "checkout completed".into(),
            })
        }
        other => Ok(StripeWebhookResult {
            event_type: other.to_string(),
            tenant_id: None,
            plan: None,
            message: "ignored".into(),
        }),
    }
}

fn upgrade_tenant_plan(tenant_id: &str, plan: &str) -> Result<()> {
    let new_plan = match plan.to_lowercase().as_str() {
        "team" => TenantPlan::Team,
        "enterprise" => TenantPlan::Enterprise,
        _ => TenantPlan::Free,
    };
    let mut store = TenantStore::load();
    store.upgrade_plan(tenant_id, new_plan)
}

fn flatten_json(value: &serde_json::Value) -> Vec<(String, String)> {
    let mut out = Vec::new();
    flatten_json_inner("", value, &mut out);
    out
}

fn flatten_json_inner(prefix: &str, value: &serde_json::Value, out: &mut Vec<(String, String)>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}[{k}]")
                };
                flatten_json_inner(&key, v, out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, v) in items.iter().enumerate() {
                let key = format!("{prefix}[{i}]");
                flatten_json_inner(&key, v, out);
            }
        }
        serde_json::Value::String(s) => out.push((prefix.to_string(), s.clone())),
        serde_json::Value::Number(n) => out.push((prefix.to_string(), n.to_string())),
        serde_json::Value::Bool(b) => out.push((prefix.to_string(), b.to_string())),
        serde_json::Value::Null => {}
    }
}

fn hmac_sha256_hex(key: &[u8], data: &[u8]) -> String {
    const BLOCK: usize = 64;
    let mut k = [0u8; BLOCK];
    if key.len() > BLOCK {
        let hash = Sha256::digest(key);
        k[..32].copy_from_slice(&hash);
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK];
    let mut opad = [0x5cu8; BLOCK];
    for i in 0..BLOCK {
        ipad[i] ^= k[i];
        opad[i] ^= k[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_hash = inner.finalize();
    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_hash);
    format!("{:x}", outer.finalize())
}

fn constant_time_eq(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        diff |= x ^ y;
    }
    diff == 0
}
