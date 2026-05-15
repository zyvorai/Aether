//! OpenID Connect (authorization code + PKCE) for the Aether API and dashboard.
//!
//! Environment (all required when OIDC is enabled):
//! - `AETHER_OIDC_ISSUER` — issuer URL (e.g. `https://accounts.example.com`)
//! - `AETHER_OIDC_CLIENT_ID`
//! - `AETHER_OIDC_REDIRECT_URI` — must match the IdP registration (e.g. `http://127.0.0.1:5090/api/auth/oidc/callback`)
//! - `AETHER_SESSION_SECRET` — HMAC key for dashboard session cookies (min 16 chars)
//!
//! Optional:
//! - `AETHER_OIDC_CLIENT_SECRET` — omit for public clients where allowed
//! - `AETHER_OIDC_DEFAULT_ROLE` — `admin` | `operator` | `viewer` (default `operator`) when no group match
//! - `AETHER_OIDC_GROUPS_CLAIM` — JWT claim path for group membership (default `groups`; use dots for nesting, e.g. `realm_access.roles`)
//! - `AETHER_OIDC_ROLE_MAP` — map IdP groups to roles: `admin=admins,root;operator=devs;viewer=readonly`
//! - `AETHER_REDIS_URL` — shared storage for OAuth state across HA replicas (`redis://...`)

use crate::ha::{PendingOidcJson, SharedCache};
use crate::rbac::Role;
use anyhow::Context;
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Redirect, Response};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use openidconnect::core::{CoreAuthenticationFlow, CoreClient, CoreProviderMetadata};
use openidconnect::{
    AccessTokenHash, AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce,
    OAuth2TokenResponse, PkceCodeChallenge, RedirectUrl, Scope, TokenResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;

const SESSION_COOKIE: &str = "aether_session";
const SESSION_HOURS: i64 = 168;

#[derive(Debug, Serialize, Deserialize)]
struct SessionClaims {
    sub: String,
    email: Option<String>,
    name: Option<String>,
    role: String,
    exp: i64,
}

/// Maps IdP group names (case-insensitive) to Aether RBAC roles. Highest privilege wins.
#[derive(Debug, Clone)]
struct OidcRoleMapping {
    claim_path: Vec<String>,
    rules: Vec<(Role, Vec<String>)>,
}

pub struct OidcRuntime {
    http: reqwest::Client,
    issuer_url: IssuerUrl,
    client_id: ClientId,
    client_secret: Option<ClientSecret>,
    redirect_uri: RedirectUrl,
    session_secret: Vec<u8>,
    default_role: Role,
    role_mapping: Option<OidcRoleMapping>,
    cache: SharedCache,
    metadata: Mutex<Option<Arc<CoreProviderMetadata>>>,
}

impl OidcRuntime {
    pub fn new(cache: SharedCache) -> anyhow::Result<Option<Arc<Self>>> {
        let issuer = std::env::var("AETHER_OIDC_ISSUER").ok().filter(|s| !s.is_empty());
        let client_id = std::env::var("AETHER_OIDC_CLIENT_ID").ok().filter(|s| !s.is_empty());
        let redirect = std::env::var("AETHER_OIDC_REDIRECT_URI").ok().filter(|s| !s.is_empty());
        let session = std::env::var("AETHER_SESSION_SECRET").ok().filter(|s| !s.is_empty());
        let Some(issuer_s) = issuer else { return Ok(None) };
        let Some(cid) = client_id else {
            anyhow::bail!("AETHER_OIDC_ISSUER set but AETHER_OIDC_CLIENT_ID missing");
        };
        let Some(redir) = redirect else {
            anyhow::bail!("AETHER_OIDC_ISSUER set but AETHER_OIDC_REDIRECT_URI missing");
        };
        let Some(sess) = session else {
            anyhow::bail!("OIDC enabled: set AETHER_SESSION_SECRET for signed session cookies");
        };
        if sess.len() < 16 {
            anyhow::bail!("AETHER_SESSION_SECRET must be at least 16 characters");
        }
        let issuer_url = IssuerUrl::new(issuer_s).context("invalid AETHER_OIDC_ISSUER")?;
        let redirect_uri = RedirectUrl::new(redir).context("invalid AETHER_OIDC_REDIRECT_URI")?;
        let client_secret = std::env::var("AETHER_OIDC_CLIENT_SECRET")
            .ok()
            .filter(|s| !s.is_empty())
            .map(ClientSecret::new);
        let default_role = parse_role_env(std::env::var("AETHER_OIDC_DEFAULT_ROLE").ok().as_deref());
        let role_mapping = parse_role_mapping_from_env()?;
        if role_mapping.is_some() {
            tracing::info!("OIDC group → role mapping enabled");
        }
        let http = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .context("failed to build reqwest client for OIDC")?;
        tracing::info!(
            issuer = %issuer_url,
            "OpenID Connect relying-party mode enabled"
        );
        Ok(Some(Arc::new(Self {
            http,
            issuer_url,
            client_id: ClientId::new(cid),
            client_secret,
            redirect_uri,
            session_secret: sess.into_bytes(),
            default_role,
            role_mapping,
            cache,
            metadata: Mutex::new(None),
        })))
    }

    pub fn role_mapping_configured(&self) -> bool {
        self.role_mapping.is_some()
    }

    pub fn cache(&self) -> &SharedCache {
        &self.cache
    }

    /// Build authorization redirect and persist PKCE + nonce.
    pub async fn begin_login(&self, next_path: Option<&str>) -> Result<Response, anyhow::Error> {
        let meta = {
            let mut guard = self.metadata.lock().await;
            if guard.is_none() {
                let m = CoreProviderMetadata::discover_async(self.issuer_url.clone(), &self.http)
                    .await
                    .map_err(|e| anyhow::anyhow!("OIDC discovery failed: {}", e))?;
                *guard = Some(Arc::new(m));
            }
            guard.as_ref().unwrap().clone()
        };
        let client = CoreClient::from_provider_metadata(
            (*meta).clone(),
            self.client_id.clone(),
            self.client_secret.clone(),
        )
        .set_redirect_uri(self.redirect_uri.clone());
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (mut auth_url, csrf, nonce) = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .add_scope(Scope::new("openid".to_string()))
            .add_scope(Scope::new("email".to_string()))
            .add_scope(Scope::new("profile".to_string()))
            .set_pkce_challenge(pkce_challenge)
            .url();
        if let Some(n) = next_path.filter(|p| p.starts_with('/') && !p.starts_with("//")) {
            auth_url.query_pairs_mut().append_pair("next", n);
        }
        let pending = PendingOidcJson {
            nonce: nonce.secret().to_string(),
            pkce_verifier: pkce_verifier.secret().to_string(),
            exp_unix: unix_now() + 600,
        };
        self.cache
            .put_oidc_pending(csrf.secret(), &pending)
            .await?;
        Ok(Redirect::temporary(auth_url.as_str()).into_response())
    }

    pub async fn finish_login(
        &self,
        query: &[(String, String)],
        tls: bool,
    ) -> Result<Response, anyhow::Error> {
        let mut code = None;
        let mut state = None;
        let mut next = None;
        for (k, v) in query {
            match k.as_str() {
                "code" => code = Some(v.clone()),
                "state" => state = Some(v.clone()),
                "next" => next = Some(v.clone()),
                _ => {}
            }
        }
        let (Some(code), Some(state)) = (code, state) else {
            return Ok(bad_oidc("missing code or state"));
        };
        let pending = self
            .cache
            .take_oidc_pending(&state)
            .await?
            .ok_or_else(|| anyhow::anyhow!("unknown or expired OAuth state"))?;
        let meta = {
            let mut guard = self.metadata.lock().await;
            if guard.is_none() {
                let m = CoreProviderMetadata::discover_async(self.issuer_url.clone(), &self.http)
                    .await
                    .map_err(|e| anyhow::anyhow!("OIDC discovery failed: {}", e))?;
                *guard = Some(Arc::new(m));
            }
            guard.as_ref().unwrap().clone()
        };
        let client = CoreClient::from_provider_metadata(
            (*meta).clone(),
            self.client_id.clone(),
            self.client_secret.clone(),
        )
        .set_redirect_uri(self.redirect_uri.clone());
        let token_response = client
            .exchange_code(AuthorizationCode::new(code))?
            .set_pkce_verifier(openidconnect::PkceCodeVerifier::new(pending.pkce_verifier))
            .request_async(&self.http)
            .await
            .map_err(|e| anyhow::anyhow!("token exchange failed: {}", e))?;
        let id_token = token_response
            .id_token()
            .ok_or_else(|| anyhow::anyhow!("token response missing id_token"))?;
        let nonce = Nonce::new(pending.nonce);
        let id_token_verifier = client.id_token_verifier();
        let claims = id_token
            .claims(&id_token_verifier, &nonce)
            .map_err(|e| anyhow::anyhow!("id_token validation failed: {}", e))?;
        if let Some(expected_access_token_hash) = claims.access_token_hash() {
            let actual_access_token_hash = AccessTokenHash::from_token(
                token_response.access_token(),
                id_token.signing_alg().map_err(|e| anyhow::anyhow!("{}", e))?,
                id_token
                    .signing_key(&id_token_verifier)
                    .map_err(|e| anyhow::anyhow!("{}", e))?,
            )
            .map_err(|e| anyhow::anyhow!("{}", e))?;
            if actual_access_token_hash != *expected_access_token_hash {
                return Ok(bad_oidc("access token hash mismatch"));
            }
        }
        let sub = claims.subject().to_string();
        let email = claims.email().map(|e| e.as_str().to_string());
        let name = claims
            .name()
            .and_then(|n| n.get(None))
            .map(|l| l.to_string());
        let role = self.resolve_role_for_id_token(id_token.to_string().as_str());
        let jwt = self.issue_session_jwt(&sub, email.clone(), name.clone(), role)?;
        let loc = next
            .filter(|p| p.starts_with('/') && !p.starts_with("//"))
            .unwrap_or_else(|| "/".to_string());
        let mut res = Redirect::to(&loc).into_response();
        let cookie = session_cookie_value(&jwt, tls)?;
        res.headers_mut().insert(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookie).map_err(|e| anyhow::anyhow!("cookie header: {}", e))?,
        );
        Ok(res)
    }

    fn issue_session_jwt(
        &self,
        sub: &str,
        email: Option<String>,
        name: Option<String>,
        role: Role,
    ) -> Result<String, anyhow::Error> {
        let exp = chrono::Utc::now().timestamp() + SESSION_HOURS * 3600;
        let claims = SessionClaims {
            sub: sub.to_string(),
            email,
            name,
            role: role.to_string(),
            exp,
        };
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.session_secret),
        )?;
        Ok(token)
    }

    pub fn verify_session_cookie(&self, headers: &HeaderMap) -> Option<(Role, String)> {
        let raw = extract_cookie(headers, SESSION_COOKIE)?;
        let mut validation = Validation::default();
        validation.validate_exp = true;
        let data = decode::<SessionClaims>(
            &raw,
            &DecodingKey::from_secret(&self.session_secret),
            &validation,
        )
        .ok()?;
        let role = parse_role_str(&data.claims.role).unwrap_or_else(|| self.default_role.clone());
        let username = data
            .claims
            .name
            .clone()
            .or(data.claims.email.clone())
            .unwrap_or_else(|| data.claims.sub.clone());
        Some((role, username))
    }

    pub fn clear_session_cookie(tls: bool) -> HeaderValue {
        let sec = if tls { "; Secure" } else { "" };
        HeaderValue::from_str(&format!(
            "{SESSION_COOKIE}=; HttpOnly; Path=/; Max-Age=0; SameSite=Lax{sec}"
        ))
        .expect("static cookie")
    }
}

fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn session_cookie_value(jwt: &str, tls: bool) -> Result<String, anyhow::Error> {
    let sec = if tls { "; Secure" } else { "" };
    let max_age = SESSION_HOURS * 3600;
    Ok(format!(
        "{SESSION_COOKIE}={}; HttpOnly; Path=/; Max-Age={}; SameSite=Lax{}",
        jwt, max_age, sec
    ))
}

fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let c = headers.get(header::COOKIE)?.to_str().ok()?;
    for part in c.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix(name) {
            if let Some(v) = rest.strip_prefix('=') {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

fn bad_oidc(msg: &str) -> Response {
    (StatusCode::BAD_REQUEST, msg.to_string()).into_response()
}

fn parse_role_env(s: Option<&str>) -> Role {
    parse_role_str(s.unwrap_or("operator")).unwrap_or(Role::Operator)
}

fn parse_role_str(s: &str) -> Option<Role> {
    match s.to_ascii_lowercase().as_str() {
        "admin" => Some(Role::Admin),
        "operator" => Some(Role::Operator),
        "viewer" => Some(Role::Viewer),
        _ => None,
    }
}

impl OidcRuntime {
    fn resolve_role_for_id_token(&self, raw_jwt: &str) -> Role {
        let Some(mapping) = self.role_mapping.as_ref() else {
            return self.default_role.clone();
        };
        let Some(payload) = jwt_payload_json(raw_jwt) else {
            tracing::warn!("OIDC: could not parse id_token payload for group mapping");
            return self.default_role.clone();
        };
        let groups = claim_string_set(&payload, &mapping.claim_path);
        if groups.is_empty() {
            tracing::debug!("OIDC: no groups in claim path {:?}", mapping.claim_path);
            return self.default_role.clone();
        }
        role_from_groups(&groups, mapping).unwrap_or_else(|| self.default_role.clone())
    }
}

fn parse_role_mapping_from_env() -> anyhow::Result<Option<OidcRoleMapping>> {
    let raw = match std::env::var("AETHER_OIDC_ROLE_MAP").ok().filter(|s| !s.is_empty()) {
        Some(v) => v,
        None => return Ok(None),
    };
    let claim_path = std::env::var("AETHER_OIDC_GROUPS_CLAIM")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "groups".to_string())
        .split('.')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if claim_path.is_empty() {
        anyhow::bail!("AETHER_OIDC_GROUPS_CLAIM must not be empty");
    }
    let mut rules = Vec::new();
    for segment in raw.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let (role_s, groups_s) = segment
            .split_once('=')
            .ok_or_else(|| anyhow::anyhow!("invalid AETHER_OIDC_ROLE_MAP segment (expected role=group,group): {}", segment))?;
        let role = parse_role_str(role_s.trim())
            .ok_or_else(|| anyhow::anyhow!("unknown role in AETHER_OIDC_ROLE_MAP: {}", role_s))?;
        let groups = groups_s
            .split(',')
            .map(|g| g.trim().to_ascii_lowercase())
            .filter(|g| !g.is_empty())
            .collect::<Vec<_>>();
        if groups.is_empty() {
            anyhow::bail!("role {} in AETHER_OIDC_ROLE_MAP has no groups", role_s);
        }
        rules.push((role, groups));
    }
    if rules.is_empty() {
        anyhow::bail!("AETHER_OIDC_ROLE_MAP produced no rules");
    }
    Ok(Some(OidcRoleMapping { claim_path, rules }))
}

fn role_from_groups(member_groups: &std::collections::HashSet<String>, mapping: &OidcRoleMapping) -> Option<Role> {
    let mut best: Option<Role> = None;
    for (role, mapped) in &mapping.rules {
        if mapped.iter().any(|g| member_groups.contains(g)) {
            best = Some(match (&best, role) {
                (None, r) => r.clone(),
                (Some(Role::Viewer), r) => r.clone(),
                (Some(Role::Operator), Role::Admin) => Role::Admin,
                (Some(Role::Operator), r) => r.clone(),
                (Some(Role::Admin), _) => Role::Admin,
            });
        }
    }
    best
}

fn jwt_payload_json(jwt: &str) -> Option<serde_json::Value> {
    use base64::Engine;
    let payload_b64 = jwt.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn claim_string_set(value: &serde_json::Value, path: &[String]) -> std::collections::HashSet<String> {
    let mut cur = value;
    for key in path {
        cur = match cur {
            serde_json::Value::Object(map) => map.get(key).unwrap_or(&serde_json::Value::Null),
            _ => return std::collections::HashSet::new(),
        };
    }
    strings_from_json_value(cur)
}

fn strings_from_json_value(v: &serde_json::Value) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    match v {
        serde_json::Value::String(s) => {
            out.insert(s.trim().to_ascii_lowercase());
        }
        serde_json::Value::Array(items) => {
            for item in items {
                if let serde_json::Value::String(s) = item {
                    if !s.trim().is_empty() {
                        out.insert(s.trim().to_ascii_lowercase());
                    }
                }
            }
        }
        _ => {}
    }
    out
}

#[cfg(test)]
mod role_map_tests {
    use super::*;

    #[test]
    fn parse_role_map_env_format() {
        std::env::set_var(
            "AETHER_OIDC_ROLE_MAP",
            "admin=Admins,root;operator=dev-team;viewer=readonly",
        );
        std::env::set_var("AETHER_OIDC_GROUPS_CLAIM", "groups");
        let m = parse_role_mapping_from_env().unwrap().unwrap();
        assert_eq!(m.claim_path, vec!["groups".to_string()]);
        assert_eq!(m.rules.len(), 3);
        std::env::remove_var("AETHER_OIDC_ROLE_MAP");
        std::env::remove_var("AETHER_OIDC_GROUPS_CLAIM");
    }

    #[test]
    fn role_from_groups_picks_highest() {
        let mapping = OidcRoleMapping {
            claim_path: vec!["groups".into()],
            rules: vec![
                (Role::Viewer, vec!["readonly".into()]),
                (Role::Admin, vec!["admins".into()]),
            ],
        };
        let mut groups = std::collections::HashSet::new();
        groups.insert("readonly".into());
        groups.insert("admins".into());
        assert_eq!(role_from_groups(&groups, &mapping), Some(Role::Admin));
    }

    #[test]
    fn nested_claim_path() {
        let json: serde_json::Value = serde_json::json!({
            "realm_access": { "roles": ["ops", "viewer"] }
        });
        let got = claim_string_set(&json, &["realm_access".into(), "roles".into()]);
        assert!(got.contains("ops"));
    }
}
