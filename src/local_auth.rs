// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Local password authentication for dashboard demo / install bootstrap.
//!
//! Default credentials (product-family convention, same as Aurora):
//! - username: `admin`
//! - password: `Admin@321`
//!
//! Override with `AETHER_DEMO_USER` / `AETHER_DEMO_PASSWORD`. Disable seeding
//! with `AETHER_DEMO_AUTH=0`. Session uses the shared `aether_session` JWT cookie
//! (same shape as LDAP/OIDC) when `AETHER_SESSION_SECRET` is set; otherwise a
//! short-lived bearer API key is minted into the RBAC store.

use crate::rbac::{Role, RbacStore};
use anyhow::Context;
use axum::http::{header, HeaderMap, HeaderValue};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;

const SESSION_COOKIE: &str = "aether_session";
const SESSION_HOURS: i64 = 168;
const DEMO_KEY_NAME: &str = "demo-admin-session";

pub const DEFAULT_DEMO_USER: &str = "admin";
pub const DEFAULT_DEMO_PASSWORD: &str = "Admin@321";

#[derive(Debug, Serialize, Deserialize)]
struct SessionClaims {
    sub: String,
    role: String,
    exp: i64,
}

#[derive(Clone)]
pub struct LocalAuthRuntime {
    username: String,
    password_hash: String,
    session_secret: Option<String>,
}

impl LocalAuthRuntime {
    /// Build from env. Returns `None` when `AETHER_DEMO_AUTH=0|false|off`.
    pub fn from_env() -> Option<Self> {
        let flag = std::env::var("AETHER_DEMO_AUTH").unwrap_or_else(|_| "1".into());
        let enabled = !matches!(
            flag.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no"
        );
        if !enabled {
            return None;
        }
        let username =
            std::env::var("AETHER_DEMO_USER").unwrap_or_else(|_| DEFAULT_DEMO_USER.to_string());
        let password = std::env::var("AETHER_DEMO_PASSWORD")
            .unwrap_or_else(|_| DEFAULT_DEMO_PASSWORD.to_string());
        let session_secret = std::env::var("AETHER_SESSION_SECRET")
            .ok()
            .filter(|s| s.trim().len() >= 16)
            .or_else(load_or_create_local_session_secret);
        Some(Self {
            username: username.trim().to_string(),
            password_hash: hash_password(&password),
            session_secret,
        })
    }

    pub fn username(&self) -> &str {
        &self.username
    }

    pub fn authenticate(&self, username: &str, password: &str) -> bool {
        constant_time_eq(
            username.trim().as_bytes(),
            self.username.as_bytes(),
        ) && constant_time_eq(
            hash_password(password).as_bytes(),
            self.password_hash.as_bytes(),
        )
    }

    pub fn session_cookie_for(&self, tls: bool) -> anyhow::Result<Option<String>> {
        let Some(secret) = self.session_secret.as_ref() else {
            return Ok(None);
        };
        let exp = chrono::Utc::now()
            .checked_add_signed(chrono::Duration::hours(SESSION_HOURS))
            .context("session expiry overflow")?
            .timestamp();
        let claims = SessionClaims {
            sub: self.username.clone(),
            role: Role::Admin.to_string(),
            exp,
        };
        let jwt = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .context("encode local-auth session")?;
        Ok(Some(session_cookie_value(&jwt, tls)?))
    }

    pub fn verify_session_cookie(&self, headers: &HeaderMap) -> Option<(Role, String)> {
        let secret = self.session_secret.as_ref()?;
        let raw = extract_cookie(headers, SESSION_COOKIE)?;
        let data = decode::<SessionClaims>(
            raw,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .ok()?
        .claims;
        let role = match data.role.as_str() {
            "admin" => Role::Admin,
            "operator" => Role::Operator,
            "viewer" => Role::Viewer,
            _ => return None,
        };
        Some((role, data.sub))
    }

    pub fn clear_session_cookie(tls: bool) -> HeaderValue {
        let sec = if tls { "; Secure" } else { "" };
        HeaderValue::from_str(&format!(
            "{SESSION_COOKIE}=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax{sec}"
        ))
        .unwrap_or_else(|_| HeaderValue::from_static("aether_session=; Max-Age=0"))
    }

    /// Mint (or reuse) a long-lived Admin RBAC key for bearer-token dashboard login
    /// when no session secret is configured.
    pub fn mint_bearer_key(&self, store: &mut RbacStore) -> String {
        if let Some(existing) = store
            .list_keys()
            .into_iter()
            .find(|e| e.name == DEMO_KEY_NAME)
        {
            // Cannot recover plaintext — revoke and recreate.
            let _ = existing;
            store.revoke_key(DEMO_KEY_NAME);
        }
        store.create_key(DEMO_KEY_NAME, Role::Admin)
    }
}

pub type SharedLocalAuth = Arc<LocalAuthRuntime>;

fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"aether-local-auth-v1:");
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn load_or_create_local_session_secret() -> Option<String> {
    let path = dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"))
        .join(".aether")
        .join("local_session_secret");
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let trimmed = existing.trim().to_string();
        if trimmed.len() >= 16 {
            return Some(trimmed);
        }
    }
    use aes_gcm::aead::rand_core::{OsRng, RngCore};
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let secret: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if std::fs::write(&path, &secret).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        }
        Some(secret)
    } else {
        Some(secret)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn extract_cookie<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    let cookie = headers.get(header::COOKIE)?.to_str().ok()?;
    cookie.split(';').find_map(|part| {
        let part = part.trim();
        let (k, v) = part.split_once('=')?;
        if k.trim() == name {
            Some(v.trim())
        } else {
            None
        }
    })
}

fn session_cookie_value(jwt: &str, tls: bool) -> anyhow::Result<String> {
    let sec = if tls { "; Secure" } else { "" };
    let max_age = Duration::from_secs((SESSION_HOURS * 3600) as u64).as_secs();
    Ok(format!(
        "{SESSION_COOKIE}={jwt}; HttpOnly; Path=/; Max-Age={max_age}; SameSite=Lax{sec}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_credentials_authenticate() {
        let auth = LocalAuthRuntime {
            username: DEFAULT_DEMO_USER.into(),
            password_hash: hash_password(DEFAULT_DEMO_PASSWORD),
            session_secret: None,
        };
        assert!(auth.authenticate("admin", "Admin@321"));
        assert!(!auth.authenticate("admin", "wrong"));
        assert!(!auth.authenticate("other", "Admin@321"));
    }
}
