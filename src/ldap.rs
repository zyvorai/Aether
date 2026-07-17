// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Active Directory / LDAP bind authentication for the Aether API and dashboard.
//!
//! Environment (required when LDAP is enabled):
//! - `AETHER_LDAP_URL` — LDAP server URL (e.g. `ldap://dc.example.com:389` or `ldaps://dc.example.com:636`)
//! - `AETHER_LDAP_BASE_DN` — search base (e.g. `DC=example,DC=local`)
//! - `AETHER_SESSION_SECRET` — HMAC key for dashboard session cookies (min 16 chars)
//!
//! Optional:
//! - `AETHER_LDAP_DOMAIN` — append to bare usernames as UPN (e.g. `example.local`)
//! - `AETHER_LDAP_BIND_DN` / `AETHER_LDAP_BIND_PASSWORD` — service account for user lookup
//! - `AETHER_LDAP_USER_FILTER` — search filter with `{user}` placeholder (default AD UPN/sAMAccountName)
//! - `AETHER_LDAP_GROUP_ATTR` — group membership attribute (default `memberOf`)
//! - `AETHER_LDAP_DEFAULT_ROLE` — `admin` | `operator` | `viewer` (default `operator`)
//! - `AETHER_LDAP_ROLE_MAP` — map AD groups to roles: `admin=Domain Admins;operator=devs;viewer=readonly`
//! - `AETHER_LDAP_TLS_SKIP_VERIFY` — `1`/`true` to skip TLS certificate verification (lab only)

use crate::rbac::Role;
use anyhow::Context;
use axum::http::{header, HeaderMap, HeaderValue};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use ldap3::{LdapConn, LdapConnSettings, Scope, SearchEntry};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

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
struct LdapRoleMapping {
    rules: Vec<(Role, Vec<String>)>,
}

pub struct LdapAuthResult {
    pub username: String,
    pub display_name: Option<String>,
    pub role: Role,
}

pub struct LdapRuntime {
    url: String,
    base_dn: String,
    domain: Option<String>,
    bind_dn: Option<String>,
    bind_password: Option<String>,
    user_filter: String,
    group_attr: String,
    session_secret: Vec<u8>,
    default_role: Role,
    role_mapping: Option<LdapRoleMapping>,
    tls_skip_verify: bool,
}

impl LdapRuntime {
    pub fn new() -> anyhow::Result<Option<Arc<Self>>> {
        let url = std::env::var("AETHER_LDAP_URL")
            .ok()
            .filter(|s| !s.is_empty());
        let base_dn = std::env::var("AETHER_LDAP_BASE_DN")
            .ok()
            .filter(|s| !s.is_empty());
        let session = std::env::var("AETHER_SESSION_SECRET")
            .ok()
            .filter(|s| !s.is_empty());
        let Some(url) = url else {
            return Ok(None);
        };
        let Some(base_dn) = base_dn else {
            anyhow::bail!("AETHER_LDAP_URL set but AETHER_LDAP_BASE_DN missing");
        };
        let Some(sess) = session else {
            anyhow::bail!("LDAP enabled: set AETHER_SESSION_SECRET for signed session cookies");
        };
        if sess.len() < 16 {
            anyhow::bail!("AETHER_SESSION_SECRET must be at least 16 characters");
        }
        let domain = std::env::var("AETHER_LDAP_DOMAIN")
            .ok()
            .filter(|s| !s.is_empty());
        let bind_dn = std::env::var("AETHER_LDAP_BIND_DN")
            .ok()
            .filter(|s| !s.is_empty());
        let bind_password = std::env::var("AETHER_LDAP_BIND_PASSWORD")
            .ok()
            .filter(|s| !s.is_empty());
        if bind_dn.is_some() ^ bind_password.is_some() {
            anyhow::bail!("set both AETHER_LDAP_BIND_DN and AETHER_LDAP_BIND_PASSWORD, or neither");
        }
        let user_filter = std::env::var("AETHER_LDAP_USER_FILTER").unwrap_or_else(|_| {
            "(|(userPrincipalName={user})(sAMAccountName={sam})(mail={user}))".into()
        });
        if !user_filter.contains("{user}") && !user_filter.contains("{sam}") {
            anyhow::bail!("AETHER_LDAP_USER_FILTER must contain a {{user}} or {{sam}} placeholder");
        }
        let group_attr =
            std::env::var("AETHER_LDAP_GROUP_ATTR").unwrap_or_else(|_| "memberOf".into());
        let default_role =
            parse_role_env(std::env::var("AETHER_LDAP_DEFAULT_ROLE").ok().as_deref());
        let role_mapping = parse_ldap_role_mapping_from_env()?;
        let tls_skip_verify = std::env::var("AETHER_LDAP_TLS_SKIP_VERIFY")
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or(false);
        if role_mapping.is_some() {
            tracing::info!("LDAP group → role mapping enabled");
        }
        tracing::info!(url = %url, base = %base_dn, "Active Directory / LDAP authentication enabled");
        Ok(Some(Arc::new(Self {
            url,
            base_dn,
            domain,
            bind_dn,
            bind_password,
            user_filter,
            group_attr,
            session_secret: sess.into_bytes(),
            default_role,
            role_mapping,
            tls_skip_verify,
        })))
    }

    pub fn role_mapping_configured(&self) -> bool {
        self.role_mapping.is_some()
    }

    pub fn domain(&self) -> Option<&str> {
        self.domain.as_deref()
    }

    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> Result<LdapAuthResult, anyhow::Error> {
        let password = password.trim();
        if password.is_empty() {
            anyhow::bail!("password required");
        }
        let normalized = normalize_username(username, self.domain.as_deref())?;
        let settings = self.conn_settings();
        let url = self.url.clone();
        let base_dn = self.base_dn.clone();
        let bind_dn = self.bind_dn.clone();
        let bind_password = self.bind_password.clone();
        let user_filter = self.user_filter.clone();
        let group_attr = self.group_attr.clone();
        let default_role = self.default_role;
        let role_mapping = self.role_mapping.clone();
        let normalized_for_task = normalized.clone();
        let password_owned = password.to_string();

        tokio::task::spawn_blocking(move || {
            ldap_bind_and_profile(LdapBindContext {
                url: &url,
                settings: &settings,
                base_dn: &base_dn,
                service_bind_dn: bind_dn.as_deref(),
                service_bind_password: bind_password.as_deref(),
                user_filter_tpl: &user_filter,
                group_attr: &group_attr,
                normalized_user: &normalized_for_task,
                password: &password_owned,
                default_role,
                role_mapping: role_mapping.as_ref(),
            })
        })
        .await
        .context("ldap task join")?
    }

    pub fn session_cookie_for(
        &self,
        auth: &LdapAuthResult,
        tls: bool,
    ) -> Result<String, anyhow::Error> {
        let jwt = self.issue_session_jwt(
            &auth.username,
            Some(auth.username.clone()),
            auth.display_name.clone(),
            auth.role,
        )?;
        session_cookie_value(&jwt, tls)
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
        let role = parse_role_str(&data.claims.role).unwrap_or(self.default_role);
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

    fn conn_settings(&self) -> LdapConnSettings {
        let mut settings = LdapConnSettings::new().set_conn_timeout(Duration::from_secs(10));
        if self.tls_skip_verify {
            settings = settings.set_no_tls_verify(true);
        }
        settings
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

struct LdapBindContext<'a> {
    url: &'a str,
    settings: &'a LdapConnSettings,
    base_dn: &'a str,
    service_bind_dn: Option<&'a str>,
    service_bind_password: Option<&'a str>,
    user_filter_tpl: &'a str,
    group_attr: &'a str,
    normalized_user: &'a str,
    password: &'a str,
    default_role: Role,
    role_mapping: Option<&'a LdapRoleMapping>,
}

fn ldap_bind_and_profile(ctx: LdapBindContext<'_>) -> Result<LdapAuthResult, anyhow::Error> {
    let LdapBindContext {
        url,
        settings,
        base_dn,
        service_bind_dn,
        service_bind_password,
        user_filter_tpl,
        group_attr,
        normalized_user,
        password,
        default_role,
        role_mapping,
    } = ctx;
    let mut ldap = LdapConn::with_settings(settings.clone(), url)
        .with_context(|| format!("ldap connect to {url}"))?;

    let filter = user_filter_tpl
        .replace("{user}", &ldap_filter_escape(normalized_user))
        .replace(
            "{sam}",
            &ldap_filter_escape(sam_account_name(normalized_user)),
        );
    let attrs = vec![
        "cn",
        "displayName",
        "mail",
        "userPrincipalName",
        "sAMAccountName",
        group_attr,
    ];

    let profile = if let (Some(bind_dn), Some(bind_pw)) = (service_bind_dn, service_bind_password) {
        ldap.simple_bind(bind_dn, bind_pw)
            .context("ldap service bind")?
            .success()
            .context("ldap service bind failed")?;
        let (entries, _rs) = ldap
            .search(base_dn, Scope::Subtree, &filter, attrs.clone())
            .context("ldap user search")?
            .success()
            .context("ldap user search failed")?;
        let entry = entries
            .into_iter()
            .map(SearchEntry::construct)
            .next()
            .ok_or_else(|| anyhow::anyhow!("ldap user not found"))?;
        let dn = entry.dn.clone();
        let profile = entry_to_profile(&entry, group_attr);
        ldap.simple_bind(&dn, password)
            .context("ldap user bind")?
            .success()
            .context("invalid username or password")?;
        profile
    } else {
        let bind_identity = upn_for_bind(normalized_user);
        ldap.simple_bind(&bind_identity, password)
            .context("ldap user bind")?
            .success()
            .context("invalid username or password")?;
        let (entries, _rs) = ldap
            .search(base_dn, Scope::Subtree, &filter, attrs)
            .context("ldap profile search")?
            .success()
            .context("ldap profile search failed")?;
        let entry = entries
            .into_iter()
            .map(SearchEntry::construct)
            .next()
            .ok_or_else(|| anyhow::anyhow!("ldap user profile not found"))?;
        entry_to_profile(&entry, group_attr)
    };

    let role = resolve_role(&profile.groups, role_mapping, default_role);
    let username = profile
        .upn
        .or(profile.sam)
        .unwrap_or_else(|| normalized_user.to_string());
    Ok(LdapAuthResult {
        username,
        display_name: profile.display_name.or(profile.cn),
        role,
    })
}

#[derive(Debug)]
struct LdapProfile {
    cn: Option<String>,
    display_name: Option<String>,
    upn: Option<String>,
    sam: Option<String>,
    groups: Vec<String>,
}

fn entry_to_profile(entry: &SearchEntry, group_attr: &str) -> LdapProfile {
    let cn = first_attr(entry, &["cn"]);
    let display_name = first_attr(entry, &["displayName"]);
    let upn = first_attr(entry, &["userPrincipalName", "mail"]);
    let sam = first_attr(entry, &["sAMAccountName"]);
    let groups = entry
        .attrs
        .get(group_attr)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|dn| cn_from_dn(&dn).unwrap_or(dn))
        .collect();
    LdapProfile {
        cn,
        display_name,
        upn,
        sam,
        groups,
    }
}

fn first_attr(entry: &SearchEntry, names: &[&str]) -> Option<String> {
    for name in names {
        if let Some(values) = entry.attrs.get(*name) {
            if let Some(v) = values.first().filter(|v| !v.is_empty()) {
                return Some(v.clone());
            }
        }
    }
    None
}

pub fn normalize_username(raw: &str, domain: Option<&str>) -> Result<String, anyhow::Error> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        anyhow::bail!("username required");
    }
    if trimmed.contains('\\') {
        let (left, right) = trimmed
            .split_once('\\')
            .ok_or_else(|| anyhow::anyhow!("invalid DOMAIN\\user username"))?;
        if left.is_empty() || right.is_empty() {
            anyhow::bail!("invalid DOMAIN\\user username");
        }
        let dom = domain.unwrap_or(left);
        return Ok(format!("{right}@{dom}"));
    }
    if trimmed.contains('@') {
        return Ok(trimmed.to_string());
    }
    let Some(domain) = domain else {
        anyhow::bail!("bare username requires AETHER_LDAP_DOMAIN");
    };
    Ok(format!("{trimmed}@{domain}"))
}

fn upn_for_bind(normalized_user: &str) -> String {
    normalized_user.to_string()
}

fn sam_account_name(normalized_user: &str) -> &str {
    normalized_user.split('@').next().unwrap_or(normalized_user)
}

fn cn_from_dn(dn: &str) -> Option<String> {
    dn.split(',')
        .next()
        .and_then(|part| {
            part.strip_prefix("CN=")
                .or_else(|| part.strip_prefix("cn="))
        })
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn ldap_filter_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'*' | b'(' | b')' | b'\\' | 0 => out.push_str(&format!("\\{byte:02x}")),
            _ => out.push(byte as char),
        }
    }
    out
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

fn parse_ldap_role_mapping_from_env() -> anyhow::Result<Option<LdapRoleMapping>> {
    let raw = match std::env::var("AETHER_LDAP_ROLE_MAP")
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
                "invalid AETHER_LDAP_ROLE_MAP segment (expected role=group,group): {}",
                segment
            )
        })?;
        let role = parse_role_str(role_s.trim())
            .ok_or_else(|| anyhow::anyhow!("unknown role in AETHER_LDAP_ROLE_MAP: {}", role_s))?;
        let groups = groups_s
            .split(',')
            .map(|g| g.trim().to_ascii_lowercase())
            .filter(|g| !g.is_empty())
            .collect::<Vec<_>>();
        if groups.is_empty() {
            anyhow::bail!("role {} in AETHER_LDAP_ROLE_MAP has no groups", role_s);
        }
        rules.push((role, groups));
    }
    Ok(Some(LdapRoleMapping { rules }))
}

fn resolve_role(groups: &[String], mapping: Option<&LdapRoleMapping>, default_role: Role) -> Role {
    let Some(mapping) = mapping else {
        return default_role;
    };
    if groups.is_empty() {
        return default_role;
    }
    let set: HashSet<String> = groups.iter().map(|g| g.to_ascii_lowercase()).collect();
    role_from_groups(&set, mapping).unwrap_or(default_role)
}

fn role_from_groups(member_groups: &HashSet<String>, mapping: &LdapRoleMapping) -> Option<Role> {
    let mut best: Option<Role> = None;
    for (role, mapped) in &mapping.rules {
        if mapped.iter().any(|g| member_groups.contains(g)) {
            best = Some(match (&best, role) {
                (None, r) => *r,
                (Some(Role::Viewer), r) => *r,
                (Some(Role::Operator), Role::Admin) => Role::Admin,
                (Some(Role::Operator), r) => *r,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_upn_and_domain_user() {
        assert_eq!(
            normalize_username("sshant@zyvorai.local", Some("zyvorai.local")).unwrap(),
            "sshant@zyvorai.local"
        );
        assert_eq!(
            normalize_username("sshant", Some("zyvorai.local")).unwrap(),
            "sshant@zyvorai.local"
        );
        assert_eq!(
            normalize_username("ZYVORAI\\sshant", Some("zyvorai.local")).unwrap(),
            "sshant@zyvorai.local"
        );
    }

    #[test]
    fn bare_username_requires_domain() {
        assert!(normalize_username("sshant", None).is_err());
    }

    #[test]
    fn cn_from_memberof_dn() {
        assert_eq!(
            cn_from_dn("CN=Domain Admins,CN=Users,DC=zyvorai,DC=local").as_deref(),
            Some("Domain Admins")
        );
    }

    #[test]
    fn ldap_filter_escape_specials() {
        assert_eq!(ldap_filter_escape("user*"), "user\\2a");
        assert_eq!(ldap_filter_escape("(admin)"), "\\28admin\\29");
    }

    #[test]
    fn role_map_picks_highest_privilege() {
        let mapping = LdapRoleMapping {
            rules: vec![
                (Role::Viewer, vec!["readonly".into()]),
                (Role::Admin, vec!["domain admins".into()]),
            ],
        };
        let groups = vec!["Domain Admins".into(), "readonly".into()];
        assert_eq!(
            resolve_role(&groups, Some(&mapping), Role::Operator),
            Role::Admin
        );
    }
}
