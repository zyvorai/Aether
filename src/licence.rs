// Copyright 2026 Zyvor AI Labs
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0

//! Soft licence status for Aether.
//!
//! Key format (shared by every Zyvor product): `ZYV1.<b64url payload JSON>.<b64url Ed25519 signature over the payload bytes>`.
//! This module only VERIFIES. Keys are issued by Zyvor and no signing key is in this repository.
//!
//! Soft enforcement: the status is information only. Nothing in Aether is ever disabled, limited or deleted because a
//! trial or licence ended, or because there is no key. Aether stays free under the Apache License 2.0; the 30-day trial key
//! applies to the paid production licence and support, not to the software.
//!
//! Key source: `AETHER_LICENCE_KEY`, else the file named by `AETHER_LICENCE_FILE`, else `licence.key` in the Aether
//! config directory (`~/.config/aether/licence.key`).
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::Deserialize;
use serde_json::{json, Value};

/// Same text as zyvor-licensing/public-keys/aether.pub.
pub const PUBLIC_KEY_B64URL: &str = "jCOUfUkH7BNmxPFB5tVf2d3SM9Z_yeBvi72gBJgE5pQ";
pub const PRODUCT: &str = "aether";
const DAY: i64 = 86_400;

#[derive(Deserialize)]
struct Payload {
    v: u32,
    product: String,
    edition: String,
    #[serde(default)]
    macs: u32,
    expires: i64,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Status {
    /// licensed | trial | licence_expired | unlicensed | invalid
    pub state: &'static str,
    pub edition: Option<String>,
    pub days_left: Option<i64>,
    pub nodes: Option<u32>,
    pub message: String,
}

fn verifying_key() -> VerifyingKey {
    let raw = URL_SAFE_NO_PAD.decode(PUBLIC_KEY_B64URL).expect("embedded public key");
    VerifyingKey::from_bytes(&raw.try_into().expect("32-byte public key")).expect("valid public key")
}

fn parse(key: &str, pk: &VerifyingKey) -> Result<Payload, &'static str> {
    let parts: Vec<&str> = key.trim().split('.').collect();
    if parts.len() != 3 || parts[0] != "ZYV1" {
        return Err("not a Zyvor licence key");
    }
    let raw = URL_SAFE_NO_PAD.decode(parts[1]).map_err(|_| "bad key encoding")?;
    let sig = URL_SAFE_NO_PAD.decode(parts[2]).map_err(|_| "bad key encoding")?;
    let sig = Signature::from_slice(&sig).map_err(|_| "bad key signature")?;
    pk.verify(&raw, &sig).map_err(|_| "signature does not verify")?;
    let p: Payload = serde_json::from_slice(&raw).map_err(|_| "bad key payload")?;
    if p.v != 1 || p.product != PRODUCT {
        return Err("key is for a different product");
    }
    Ok(p)
}

/// Pure evaluation (testable): `key` if one is configured.
pub fn evaluate(key: Option<&str>, now: i64, pk: &VerifyingKey) -> Status {
    if let Some(k) = key.filter(|k| !k.trim().is_empty()) {
        return match parse(k, pk) {
            Ok(p) => {
                let left = (p.expires - now).div_euclid(DAY);
                let (state, msg) = if now >= p.expires {
                    ("licence_expired", "Licence ended. Aether keeps working; contact sales@zyvor.dev to renew.".to_string())
                } else if p.edition == "trial" {
                    ("trial", format!("30-day trial key, {left} day(s) left. Nothing is disabled when it ends."))
                } else {
                    ("licensed", format!("Licensed, {left} day(s) left."))
                };
                Status { state, edition: Some(p.edition), days_left: Some(left.max(0)), nodes: Some(p.macs), message: msg }
            }
            Err(why) => Status {
                state: "invalid",
                edition: None,
                days_left: None,
                nodes: None,
                message: format!("Licence key not accepted ({why}). Aether keeps working."),
            },
        };
    }
    Status {
        state: "unlicensed",
        edition: None,
        days_left: None,
        nodes: None,
        message: "No licence key. Aether keeps working (Apache-2.0); a 30-day trial key for the paid production licence and support is available from sales@zyvor.dev.".into(),
    }
}

fn key_file() -> Option<std::path::PathBuf> {
    if let Some(p) = std::env::var_os("AETHER_LICENCE_FILE").filter(|p| !p.is_empty()) {
        return Some(p.into());
    }
    dirs::config_dir().map(|d| d.join("aether").join("licence.key"))
}

fn now_secs() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Current status from the environment and files. Best-effort; never fails and never gates anything.
pub fn status() -> Status {
    let key = std::env::var("AETHER_LICENCE_KEY")
        .ok()
        .filter(|k| !k.trim().is_empty())
        .or_else(|| key_file().and_then(|p| std::fs::read_to_string(p).ok()));
    evaluate(key.as_deref(), now_secs(), &verifying_key())
}

pub fn status_json() -> Value {
    let s = status();
    json!({
        "product": PRODUCT,
        "state": s.state,
        "edition": s.edition,
        "days_left": s.days_left,
        "nodes": s.nodes,
        "enforcement": "soft",
        "message": s.message,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn key_for(sk: &SigningKey, product: &str, edition: &str, expires: i64) -> String {
        let payload = json!({"v":1,"product":product,"edition":edition,"id":"t","licensee":"x","email":"x@example.com","macs":2,"issued":0,"expires":expires});
        let raw = serde_json::to_vec(&payload).unwrap();
        format!("ZYV1.{}.{}", URL_SAFE_NO_PAD.encode(&raw), URL_SAFE_NO_PAD.encode(sk.sign(&raw).to_bytes()))
    }

    #[test]
    fn embedded_public_key_is_valid() {
        let _ = verifying_key();
    }

    #[test]
    fn keys_and_trial_clock() {
        // Throwaway keypair: the real signing key is never here.
        let sk = SigningKey::from_bytes(&[7u8; 32]);
        let pk = sk.verifying_key();
        let now = 1_000_000_000;
        let ok = key_for(&sk, "aether", "pro", now + 10 * DAY);
        let s = evaluate(Some(&ok), now, &pk);
        assert_eq!((s.state, s.days_left), ("licensed", Some(10)));
        let t = key_for(&sk, "aether", "trial", now + 5 * DAY);
        let s = evaluate(Some(&t), now, &pk);
        assert_eq!((s.state, s.days_left), ("trial", Some(5)));
        let old = key_for(&sk, "aether", "trial", now - 1);
        assert_eq!(evaluate(Some(&old), now, &pk).state, "licence_expired");
        let other = key_for(&sk, "velora", "pro", now + DAY);
        assert_eq!(evaluate(Some(&other), now, &pk).state, "invalid");
        let wrong = SigningKey::from_bytes(&[9u8; 32]);
        assert_eq!(evaluate(Some(&key_for(&wrong, "aether", "pro", now + DAY)), now, &pk).state, "invalid");
        let mut tampered = ok.clone();
        tampered.push('A');
        assert_eq!(evaluate(Some(&tampered), now, &pk).state, "invalid");
        // Flip a payload character: the signature no longer matches.
        let mut parts: Vec<String> = ok.split('.').map(String::from).collect();
        let mut body: Vec<u8> = URL_SAFE_NO_PAD.decode(&parts[1]).unwrap();
        body[10] ^= 1;
        parts[1] = URL_SAFE_NO_PAD.encode(&body);
        assert_eq!(evaluate(Some(&parts.join(".")), now, &pk).state, "invalid");
        assert_eq!(evaluate(Some("garbage"), now, &pk).state, "invalid");
        // no key: unlicensed but still running
        assert_eq!(evaluate(None, now, &pk).state, "unlicensed");
        assert_eq!(evaluate(Some("  "), now, &pk).state, "unlicensed");
    }
}
