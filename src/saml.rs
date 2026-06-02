// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! SAML 2.0 Service Provider (HTTP-Redirect login + HTTP-POST ACS).
//!
//! Environment (required when SAML is enabled):
//! - `AETHER_SAML_IDP_SSO_URL` — IdP single sign-on URL
//! - `AETHER_SAML_IDP_ENTITY_ID` — IdP entity ID
//! - `AETHER_SAML_ACS_URL` — assertion consumer URL (e.g. `http://127.0.0.1:5090/api/auth/saml/acs`)
//! - `AETHER_SESSION_SECRET` — HMAC key for dashboard session cookies (min 16 chars)
//!
//! Optional:
//! - `AETHER_SAML_SP_ENTITY_ID` — SP entity ID (defaults to ACS URL without path)
//! - `AETHER_SAML_DEFAULT_ROLE` — `admin` | `operator` | `viewer` (default `operator`)
//! - `AETHER_SAML_ROLE_MAP` — `admin=admins;operator=devs;viewer=readonly` (matches group Attribute values)
//! - `AETHER_SAML_IDP_CERT` — PEM X.509 certificate; when set, SAMLResponse signatures are verified

use crate::ha::{PendingSamlJson, SharedCache};
use crate::rbac::Role;
use anyhow::Context;
use axum::http::{header, HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Redirect, Response};
use base64::Engine;
use flate2::Compression;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

#[derive(Debug, Clone)]
struct SamlRoleMapping {
    rules: Vec<(Role, Vec<String>)>,
}

pub struct SamlRuntime {
    idp_sso_url: String,
    #[allow(dead_code)]
    idp_entity_id: String,
    sp_entity_id: String,
    acs_url: String,
    idp_cert_pem: Option<String>,
    session_secret: Vec<u8>,
    default_role: Role,
    role_mapping: Option<SamlRoleMapping>,
    cache: SharedCache,
}

impl SamlRuntime {
    pub fn new(cache: SharedCache) -> anyhow::Result<Option<Arc<Self>>> {
        let idp_sso = std::env::var("AETHER_SAML_IDP_SSO_URL")
            .ok()
            .filter(|s| !s.is_empty());
        let idp_entity = std::env::var("AETHER_SAML_IDP_ENTITY_ID")
            .ok()
            .filter(|s| !s.is_empty());
        let acs = std::env::var("AETHER_SAML_ACS_URL")
            .ok()
            .filter(|s| !s.is_empty());
        let session = std::env::var("AETHER_SESSION_SECRET")
            .ok()
            .filter(|s| !s.is_empty());
        let Some(idp_sso_url) = idp_sso else {
            return Ok(None);
        };
        let Some(idp_entity_id) = idp_entity else {
            anyhow::bail!("AETHER_SAML_IDP_SSO_URL set but AETHER_SAML_IDP_ENTITY_ID missing");
        };
        let Some(acs_url) = acs else {
            anyhow::bail!("AETHER_SAML_IDP_SSO_URL set but AETHER_SAML_ACS_URL missing");
        };
        let Some(sess) = session else {
            anyhow::bail!("SAML enabled: set AETHER_SESSION_SECRET for signed session cookies");
        };
        if sess.len() < 16 {
            anyhow::bail!("AETHER_SESSION_SECRET must be at least 16 characters");
        }
        let sp_entity_id = std::env::var("AETHER_SAML_SP_ENTITY_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| acs_url.trim_end_matches('/').to_string());
        let default_role =
            parse_role_env(std::env::var("AETHER_SAML_DEFAULT_ROLE").ok().as_deref());
        let role_mapping = parse_saml_role_mapping_from_env()?;
        let idp_cert_pem = std::env::var("AETHER_SAML_IDP_CERT")
            .ok()
            .filter(|s| !s.is_empty());
        if idp_cert_pem.is_some() {
            tracing::info!("SAML IdP certificate configured: signature verification enabled");
        }
        tracing::info!(idp = %idp_sso_url, "SAML service provider enabled");
        Ok(Some(Arc::new(Self {
            idp_sso_url,
            idp_entity_id,
            sp_entity_id,
            acs_url,
            idp_cert_pem,
            session_secret: sess.into_bytes(),
            default_role,
            role_mapping,
            cache,
        })))
    }

    pub fn role_mapping_configured(&self) -> bool {
        self.role_mapping.is_some()
    }

    pub async fn begin_login(&self, next_path: Option<&str>) -> Result<Response, anyhow::Error> {
        let request_id = format!("_aether_{}", uuid_like());
        let instant = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
        let authn = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<saml2p:AuthnRequest xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol"
  xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion"
  ID="{request_id}" Version="2.0" IssueInstant="{instant}"
  Destination="{dest}" AssertionConsumerServiceURL="{acs}"
  ProtocolBinding="urn:oasis:names:tc:SAML:2.0:bindings:HTTP-POST">
  <saml2:Issuer>{issuer}</saml2:Issuer>
</saml2p:AuthnRequest>"#,
            request_id = request_id,
            instant = instant,
            dest = xml_escape(&self.idp_sso_url),
            acs = xml_escape(&self.acs_url),
            issuer = xml_escape(&self.sp_entity_id),
        );
        let saml_request = deflate_base64_url(&authn)?;
        let relay_state = next_path
            .filter(|p| p.starts_with('/') && !p.starts_with("//"))
            .unwrap_or("/");
        self.cache
            .put_saml_pending(
                &request_id,
                &PendingSamlJson {
                    relay_state: relay_state.to_string(),
                    exp_unix: unix_now() + 600,
                },
            )
            .await?;
        let mut login_url =
            url::Url::parse(&self.idp_sso_url).context("invalid AETHER_SAML_IDP_SSO_URL")?;
        login_url
            .query_pairs_mut()
            .append_pair("SAMLRequest", &saml_request)
            .append_pair("RelayState", relay_state);
        Ok(Redirect::temporary(login_url.as_str()).into_response())
    }

    pub async fn finish_acs(
        &self,
        saml_response_b64: &str,
        relay_state: Option<&str>,
        tls: bool,
    ) -> Result<Response, anyhow::Error> {
        let xml = decode_saml_response(saml_response_b64)?;
        if let Some(ref cert) = self.idp_cert_pem {
            crate::saml_verify::verify_response_signature(&xml, cert)
                .context("SAML signature verification failed")?;
        }
        let in_response_to = extract_xml_attr(&xml, "InResponseTo")
            .or_else(|| extract_between(&xml, "InResponseTo=\"", "\""));
        if let Some(ref id) = in_response_to {
            let pending = self.cache.take_saml_pending(id).await?;
            if pending.is_none() {
                tracing::warn!("SAML InResponseTo not found in pending cache (continuing)");
            }
        }
        let sub = extract_name_id(&xml).unwrap_or_else(|| "saml-user".to_string());
        let email = extract_attribute(&xml, &["email", "mail", "EmailAddress"]);
        let name = extract_attribute(&xml, &["name", "displayName", "cn"]);
        let groups = extract_attribute_values(&xml, &["groups", "memberOf", "Group", "Roles"]);
        let role = self.resolve_role(&groups);
        let jwt = self.issue_session_jwt(&sub, email.clone(), name.clone(), role)?;
        let loc = relay_state
            .filter(|p| p.starts_with('/') && !p.starts_with("//"))
            .unwrap_or("/");
        let mut res = Redirect::to(loc).into_response();
        let cookie = session_cookie_value(&jwt, tls)?;
        res.headers_mut().insert(
            header::SET_COOKIE,
            HeaderValue::from_str(&cookie).map_err(|e| anyhow::anyhow!("cookie header: {}", e))?,
        );
        Ok(res)
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

    fn resolve_role(&self, groups: &[String]) -> Role {
        let Some(mapping) = self.role_mapping.as_ref() else {
            return self.default_role.clone();
        };
        if groups.is_empty() {
            return self.default_role.clone();
        }
        let set: std::collections::HashSet<String> =
            groups.iter().map(|g| g.to_ascii_lowercase()).collect();
        role_from_groups(&set, mapping).unwrap_or_else(|| self.default_role.clone())
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
        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(&self.session_secret),
        )
        .context("session jwt encode")
    }
}

fn uuid_like() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..16)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect()
}

fn deflate_base64_url(xml: &str) -> Result<String, anyhow::Error> {
    use flate2::{Compress, FlushCompress, Status};
    let input = xml.as_bytes();
    let mut compress = Compress::new(Compression::new(9), false);
    let mut buf = vec![0u8; input.len() + 64];
    loop {
        let status = compress
            .compress(input, &mut buf, FlushCompress::Finish)
            .context("deflate authn request")?;
        match status {
            Status::Ok | Status::BufError => {
                buf.resize(buf.len() + 64, 0);
            }
            Status::StreamEnd => break,
        }
    }
    buf.truncate(compress.total_out() as usize);
    Ok(base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(buf))
}

fn decode_saml_response(b64: &str) -> Result<String, anyhow::Error> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64.trim())
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(b64.trim()))
        .context("invalid SAMLResponse base64")?;
    let xml = String::from_utf8(bytes).context("SAMLResponse not UTF-8")?;
    crate::saml_decrypt::normalize_saml_response_xml(&xml)
}

fn extract_name_id(xml: &str) -> Option<String> {
    for marker in ["<saml:NameID", "<NameID"] {
        if let Some(start) = xml.find(marker) {
            let rest = &xml[start..];
            if let Some(gt) = rest.find('>') {
                let after = &rest[gt + 1..];
                if let Some(end) = after.find("</") {
                    let val = after[..end].trim();
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

fn extract_attribute(xml: &str, names: &[&str]) -> Option<String> {
    extract_attribute_values(xml, names).into_iter().next()
}

fn extract_attribute_values(xml: &str, names: &[&str]) -> Vec<String> {
    let mut out = Vec::new();
    for name in names {
        let needle = format!("Name=\"{}\"", name);
        let mut search_from = 0;
        while let Some(idx) = xml[search_from..].find(&needle) {
            let start = search_from + idx;
            let slice = &xml[start..];
            if let Some(val) =
                extract_between(slice, "<saml:AttributeValue", "</saml:AttributeValue>")
                    .or_else(|| extract_between(slice, "<AttributeValue", "</AttributeValue>"))
            {
                let cleaned = val.rsplit('>').next().unwrap_or(&val).trim().to_string();
                if !cleaned.is_empty() {
                    out.push(cleaned);
                }
            }
            search_from = start + needle.len();
        }
    }
    out
}

fn extract_xml_attr(xml: &str, attr: &str) -> Option<String> {
    let pattern = format!("{attr}=\"");
    extract_between(xml, &pattern, "\"")
}

fn extract_between(haystack: &str, start: &str, end: &str) -> Option<String> {
    let s = haystack.find(start)? + start.len();
    let e = haystack[s..].find(end)? + s;
    Some(haystack[s..e].to_string())
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
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

fn parse_saml_role_mapping_from_env() -> anyhow::Result<Option<SamlRoleMapping>> {
    let raw = match std::env::var("AETHER_SAML_ROLE_MAP")
        .ok()
        .filter(|s| !s.is_empty())
    {
        Some(v) => v,
        None => return Ok(None),
    };
    let mut rules = Vec::new();
    for segment in raw.split(';').map(str::trim).filter(|s| !s.is_empty()) {
        let (role_s, groups_s) = segment.split_once('=').ok_or_else(|| {
            anyhow::anyhow!(
                "invalid AETHER_SAML_ROLE_MAP segment (expected role=group,group): {}",
                segment
            )
        })?;
        let role = parse_role_str(role_s.trim())
            .ok_or_else(|| anyhow::anyhow!("unknown role in AETHER_SAML_ROLE_MAP: {}", role_s))?;
        let groups = groups_s
            .split(',')
            .map(|g| g.trim().to_ascii_lowercase())
            .filter(|g| !g.is_empty())
            .collect::<Vec<_>>();
        if groups.is_empty() {
            anyhow::bail!("role {} in AETHER_SAML_ROLE_MAP has no groups", role_s);
        }
        rules.push((role, groups));
    }
    Ok(Some(SamlRoleMapping { rules }))
}

fn role_from_groups(
    member_groups: &std::collections::HashSet<String>,
    mapping: &SamlRoleMapping,
) -> Option<Role> {
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

fn unix_now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_name_id_and_attributes() {
        let xml = r#"<Response InResponseTo="_aether_abc">
          <Assertion>
            <Subject><NameID>user@example.com</NameID></Subject>
            <AttributeStatement>
              <Attribute Name="email"><AttributeValue>user@example.com</AttributeValue></Attribute>
              <Attribute Name="groups"><AttributeValue>admins</AttributeValue></Attribute>
            </AttributeStatement>
          </Assertion>
        </Response>"#;
        assert_eq!(extract_name_id(xml).as_deref(), Some("user@example.com"));
        assert_eq!(
            extract_attribute(xml, &["email"]).as_deref(),
            Some("user@example.com")
        );
        assert_eq!(
            extract_attribute_values(xml, &["groups"]),
            vec!["admins".to_string()]
        );
    }

    #[test]
    fn test_deflate_authn_request() {
        let xml = "<AuthnRequest ID=\"x\"/>";
        let encoded = deflate_base64_url(xml).unwrap();
        assert!(!encoded.is_empty());
        assert!(!encoded.contains('+'));
    }
}
