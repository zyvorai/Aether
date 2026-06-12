// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Dev-only mock IdP for Playwright and local SSO testing (`AETHER_MOCK_IDP=1`).

use axum::extract::{Form, Query};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect};
use base64::Engine;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rsa::pkcs1v15::SigningKey;
use rsa::pkcs8::DecodePrivateKey;
use rsa::signature::SignatureEncoding;
use rsa::signature::Signer;
use rsa::traits::PublicKeyParts;
use rsa::RsaPrivateKey;
use serde_json::json;
use sha2::{Digest, Sha256, Sha384};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

const MOCK_SESSION_SECRET: &str = "mock-idp-dev-session-key-32chars";
const MOCK_OIDC_CLIENT_ID: &str = "aether-mock-client";
const MOCK_OIDC_CODE: &str = "mock-auth-code";

static MOCK_CERT_PEM: &str = include_str!("../packaging/mock-idp/cert.pem");
static MOCK_KEY_PEM: &str = include_str!("../packaging/mock-idp/key.pem");

struct MockKeys {
    private_key: RsaPrivateKey,
    cert_body_b64: String,
    jwk_n: String,
    jwk_e: String,
}

fn keys() -> &'static MockKeys {
    static KEYS: OnceLock<MockKeys> = OnceLock::new();
    KEYS.get_or_init(|| {
        let private_key =
            RsaPrivateKey::from_pkcs8_pem(MOCK_KEY_PEM).expect("mock IdP private key PEM");
        let cert_body = MOCK_CERT_PEM
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect::<String>();
        let public = private_key.to_public_key();
        let n = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public.n().to_bytes_be());
        let e = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(public.e().to_bytes_be());
        MockKeys {
            private_key,
            cert_body_b64: cert_body,
            jwk_n: n,
            jwk_e: e,
        }
    })
}

pub fn enabled() -> bool {
    std::env::var("AETHER_MOCK_IDP")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn apply_env_defaults(base_url: &str) {
    set_if_empty("AETHER_SESSION_SECRET", MOCK_SESSION_SECRET);
    set_if_empty(
        "AETHER_OIDC_ISSUER",
        &format!("{base_url}/api/mock-idp/oidc"),
    );
    set_if_empty("AETHER_OIDC_CLIENT_ID", MOCK_OIDC_CLIENT_ID);
    set_if_empty(
        "AETHER_OIDC_REDIRECT_URI",
        &format!("{base_url}/api/auth/oidc/callback"),
    );
    set_if_empty(
        "AETHER_SAML_IDP_SSO_URL",
        &format!("{base_url}/api/mock-idp/saml/sso"),
    );
    set_if_empty(
        "AETHER_SAML_IDP_ENTITY_ID",
        &format!("{base_url}/api/mock-idp/saml"),
    );
    set_if_empty(
        "AETHER_SAML_ACS_URL",
        &format!("{base_url}/api/auth/saml/acs"),
    );
    set_if_empty("AETHER_SAML_IDP_CERT", MOCK_CERT_PEM);
    tracing::warn!("AETHER_MOCK_IDP=1: embedded mock SAML/OIDC IdP enabled (development only)");
}

fn set_if_empty(key: &str, value: &str) {
    if std::env::var(key).ok().filter(|s| !s.is_empty()).is_none() {
        std::env::set_var(key, value);
    }
}

pub fn idp_certificate_pem() -> &'static str {
    MOCK_CERT_PEM
}

pub fn idp_private_key_pem() -> &'static str {
    MOCK_KEY_PEM
}

fn mock_idp_encrypted() -> bool {
    std::env::var("AETHER_MOCK_IDP_ENCRYPTED")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn mock_idp_encrypted_gcm() -> bool {
    std::env::var("AETHER_MOCK_IDP_ENCRYPTED_GCM")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn mock_idp_exclusive_comments() -> bool {
    std::env::var("AETHER_MOCK_IDP_EXCLUSIVE_COMMENTS")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn mock_idp_sha384() -> bool {
    std::env::var("AETHER_MOCK_IDP_SHA384")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

fn mock_idp_azure_ad() -> bool {
    std::env::var("AETHER_MOCK_IDP_AZURE_AD")
        .ok()
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

struct SignOptions {
    exclusive_comments: bool,
    sha384: bool,
    azure_transforms: bool,
}

impl SignOptions {
    fn from_env() -> Self {
        Self {
            exclusive_comments: mock_idp_exclusive_comments(),
            sha384: mock_idp_sha384(),
            azure_transforms: mock_idp_azure_ad(),
        }
    }
}

#[cfg(test)]
pub fn sign_saml_element_for_test(element_xml: &str, element_id: &str) -> String {
    sign_saml_element_with_mode(element_xml, element_id, false)
}

#[cfg(test)]
pub fn sign_saml_element_for_test_with_comments(element_xml: &str, element_id: &str) -> String {
    sign_saml_element_with_mode(element_xml, element_id, true)
}

#[cfg(test)]
pub fn sign_saml_element_for_test_sha384(element_xml: &str, element_id: &str) -> String {
    sign_saml_element_with_options(
        element_xml,
        element_id,
        SignOptions {
            exclusive_comments: false,
            sha384: true,
            azure_transforms: false,
        },
    )
}

#[cfg(test)]
pub fn sign_saml_response_for_test_azure(unsigned_response: &str, response_id: &str) -> String {
    let signature = sign_saml_element_with_options(
        unsigned_response,
        response_id,
        SignOptions {
            exclusive_comments: false,
            sha384: false,
            azure_transforms: true,
        },
    );
    if let Some(pos) = unsigned_response.find("<saml2:Assertion") {
        format!(
            "{}{}{}",
            &unsigned_response[..pos],
            signature,
            &unsigned_response[pos..]
        )
    } else {
        unsigned_response.replacen('>', &format!(">{signature}"), 1)
    }
}

pub async fn saml_sso(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let relay_state = params.get("RelayState").map(String::as_str).unwrap_or("/");
    let acs = std::env::var("AETHER_SAML_ACS_URL").unwrap_or_else(|_| "/api/auth/saml/acs".into());
    let response_id = format!("_mock_resp_{}", short_id());
    let assertion_id = format!("_mock_assert_{}", short_id());
    let instant = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let issuer = std::env::var("AETHER_SAML_IDP_ENTITY_ID").unwrap_or_else(|_| "mock-idp".into());
    let assertion = format!(
        r#"<saml2:Assertion xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="{assertion_id}" Version="2.0" IssueInstant="{instant}">
  <!-- mock-idp assertion -->
  <saml2:Issuer>{issuer}</saml2:Issuer>
  <saml2:Subject>
    <saml2:NameID Format="urn:oasis:names:tc:SAML:1.1:nameid-format:emailAddress">mock-user@aether.local</saml2:NameID>
  </saml2:Subject>
  <saml2:AttributeStatement>
    <saml2:Attribute Name="email"><saml2:AttributeValue>mock-user@aether.local</saml2:AttributeValue></saml2:Attribute>
    <saml2:Attribute Name="name"><saml2:AttributeValue>Mock IdP User</saml2:AttributeValue></saml2:Attribute>
    <saml2:Attribute Name="groups"><saml2:AttributeValue>operators</saml2:AttributeValue></saml2:Attribute>
  </saml2:AttributeStatement>
</saml2:Assertion>"#,
        issuer = xml_escape(&issuer),
    );
    let assertion = if mock_idp_exclusive_comments() {
        assertion
    } else {
        assertion.replace("  <!-- mock-idp assertion -->\n", "")
    };
    let body = if mock_idp_encrypted() || mock_idp_encrypted_gcm() {
        std::env::set_var("AETHER_SAML_SP_KEY", MOCK_KEY_PEM);
        if mock_idp_encrypted_gcm() {
            crate::saml_decrypt::encrypt_assertion_gcm_for_test(&assertion, MOCK_CERT_PEM)
        } else {
            crate::saml_decrypt::encrypt_assertion_for_test(&assertion, MOCK_CERT_PEM)
        }
        .unwrap_or(assertion)
    } else {
        assertion
    };
    let signature = if mock_idp_encrypted() || mock_idp_encrypted_gcm() {
        String::new()
    } else if mock_idp_azure_ad() {
        let unsigned = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="{response_id}" Version="2.0" IssueInstant="{instant}" Destination="{acs}">
  <saml2:Issuer>{issuer}</saml2:Issuer>
  <saml2p:Status><saml2p:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></saml2p:Status>
  {body}
</saml2p:Response>"#,
            issuer = xml_escape(&issuer),
            acs = xml_escape(&acs),
            body = body,
        );
        sign_saml_element_with_options(&unsigned, &response_id, SignOptions {
            exclusive_comments: false,
            sha384: mock_idp_sha384(),
            azure_transforms: true,
        })
    } else {
        sign_saml_element(&body, &assertion_id)
    };
    let response_xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="{response_id}" Version="2.0" IssueInstant="{instant}" Destination="{acs}">
  <saml2:Issuer>{issuer}</saml2:Issuer>
  <saml2p:Status><saml2p:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></saml2p:Status>
  {signature}
  {body}
</saml2p:Response>"#,
        issuer = xml_escape(&issuer),
        acs = xml_escape(&acs),
        body = body,
        signature = signature,
    );
    let saml_response = base64::engine::general_purpose::STANDARD.encode(response_xml.as_bytes());
    let html = format!(
        r#"<!DOCTYPE html><html><body onload="document.forms[0].submit()">
<form method="post" action="{acs}">
<input type="hidden" name="SAMLResponse" value="{saml_response}"/>
<input type="hidden" name="RelayState" value="{relay_state}"/>
<noscript><button type="submit">Continue</button></noscript>
</form></body></html>"#,
        acs = html_escape(&acs),
        saml_response = html_escape(&saml_response),
        relay_state = html_escape(relay_state),
    );
    Html(html).into_response()
}

fn sign_saml_element(element_xml: &str, element_id: &str) -> String {
    sign_saml_element_with_options(element_xml, element_id, SignOptions::from_env())
}

#[cfg(test)]
fn sign_saml_element_with_mode(element_xml: &str, element_id: &str, exclusive_comments: bool) -> String {
    sign_saml_element_with_options(
        element_xml,
        element_id,
        SignOptions {
            exclusive_comments,
            sha384: false,
            azure_transforms: false,
        },
    )
}

fn sign_saml_element_with_options(element_xml: &str, element_id: &str, options: SignOptions) -> String {
    let c14n = if options.exclusive_comments {
        crate::saml_c14n::EXC_C14N_COMMENTS
    } else {
        crate::saml_c14n::EXC_C14N
    };
    let reference_uri = format!("#{element_id}");
    let transform_xml = if options.azure_transforms {
        format!(
            r#"<ds:Transforms><ds:Transform Algorithm="http://www.w3.org/2000/09/xmldsig#enveloped-signature" /><ds:Transform Algorithm="{c14n}" /></ds:Transforms>"#
        )
    } else if options.exclusive_comments {
        format!(r#"<ds:Transforms><ds:Transform Algorithm="{c14n}" /></ds:Transforms>"#)
    } else {
        String::new()
    };
    let digest = {
        let digest_bytes = if options.azure_transforms {
            crate::saml_c14n::digest_canonical_bytes(
                element_xml,
                &[
                    "http://www.w3.org/2000/09/xmldsig#enveloped-signature".into(),
                    c14n.to_string(),
                ],
            )
            .unwrap_or_else(|_| element_xml.as_bytes().to_vec())
        } else if options.exclusive_comments {
            crate::saml_c14n::digest_canonical_bytes(element_xml, &[c14n.to_string()])
                .unwrap_or_else(|_| element_xml.as_bytes().to_vec())
        } else {
            element_xml.as_bytes().to_vec()
        };
        if options.sha384 {
            base64::engine::general_purpose::STANDARD.encode(Sha384::digest(&digest_bytes))
        } else {
            base64::engine::general_purpose::STANDARD.encode(Sha256::digest(&digest_bytes))
        }
    };
    let (sig_method, digest_method) = if options.sha384 {
        (
            "http://www.w3.org/2001/04/xmldsig-more#rsa-sha384",
            "http://www.w3.org/2001/04/xmlenc#sha384",
        )
    } else {
        (
            "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256",
            "http://www.w3.org/2001/04/xmlenc#sha256",
        )
    };
    let signed_info = [
        r#"<ds:SignedInfo xmlns:ds="http://www.w3.org/2000/09/xmldsig#"><ds:CanonicalizationMethod Algorithm=""#,
        c14n,
        r#"" /><ds:SignatureMethod Algorithm=""#,
        sig_method,
        r#""/><ds:Reference URI=""#,
        reference_uri.as_str(),
        r#"">"#,
        transform_xml.as_str(),
        r#"<ds:DigestMethod Algorithm=""#,
        digest_method,
        r#""/><ds:DigestValue>"#,
        digest.as_str(),
        r#"</ds:DigestValue></ds:Reference></ds:SignedInfo>"#,
    ]
    .concat();
    let signed_info_bytes =
        crate::saml_c14n::canonicalize(&signed_info, c14n)
            .unwrap_or_else(|_| signed_info.as_bytes().to_vec());
    let sig_b64 = if options.sha384 {
        let signing_key = SigningKey::<Sha384>::new(keys().private_key.clone());
        let signature = signing_key.sign(&signed_info_bytes);
        base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
    } else {
        let signing_key = SigningKey::<Sha256>::new(keys().private_key.clone());
        let signature = signing_key.sign(&signed_info_bytes);
        base64::engine::general_purpose::STANDARD.encode(signature.to_bytes())
    };
    [
        r#"<ds:Signature xmlns:ds="http://www.w3.org/2000/09/xmldsig#">"#,
        signed_info.as_str(),
        r#"<ds:SignatureValue>"#,
        sig_b64.as_str(),
        r#"</ds:SignatureValue><ds:KeyInfo><ds:X509Data><ds:X509Certificate>"#,
        keys().cert_body_b64.as_str(),
        r#"</ds:X509Certificate></ds:X509Data></ds:KeyInfo></ds:Signature>"#,
    ]
    .concat()
}

pub async fn oidc_discovery() -> impl IntoResponse {
    let issuer = std::env::var("AETHER_OIDC_ISSUER")
        .unwrap_or_else(|_| "http://127.0.0.1:5090/api/mock-idp/oidc".into());
    let base = issuer
        .trim_end_matches("/api/mock-idp/oidc")
        .trim_end_matches('/');
    axum::Json(json!({
        "issuer": issuer,
        "authorization_endpoint": format!("{base}/api/mock-idp/oidc/authorize"),
        "token_endpoint": format!("{base}/api/mock-idp/oidc/token"),
        "jwks_uri": format!("{base}/api/mock-idp/oidc/jwks"),
        "response_types_supported": ["code"],
        "subject_types_supported": ["public"],
        "id_token_signing_alg_values_supported": ["RS256"],
        "scopes_supported": ["openid", "email", "profile"],
    }))
}

pub async fn oidc_authorize(Query(params): Query<HashMap<String, String>>) -> impl IntoResponse {
    let redirect_uri = params.get("redirect_uri").cloned().unwrap_or_default();
    let state = params.get("state").cloned().unwrap_or_default();
    if redirect_uri.is_empty() {
        return (StatusCode::BAD_REQUEST, "redirect_uri required").into_response();
    }
    let code = if params.contains_key("nonce") {
        format!("mock-{}", short_id())
    } else {
        MOCK_OIDC_CODE.to_string()
    };
    if let Some(nonce) = params.get("nonce") {
        oidc_pending_nonces()
            .lock()
            .expect("mock oidc nonce lock")
            .insert(code.clone(), nonce.clone());
    }
    let mut url = match url::Url::parse(&redirect_uri) {
        Ok(u) => u,
        Err(_) => return (StatusCode::BAD_REQUEST, "invalid redirect_uri").into_response(),
    };
    url.query_pairs_mut()
        .append_pair("code", &code)
        .append_pair("state", &state);
    Redirect::temporary(url.as_str()).into_response()
}

#[derive(serde::Deserialize)]
pub struct TokenForm {
    code: String,
    #[serde(default)]
    #[allow(dead_code)]
    grant_type: Option<String>,
}

pub async fn oidc_token(Form(form): Form<TokenForm>) -> impl IntoResponse {
    if form.code != MOCK_OIDC_CODE && !form.code.starts_with("mock-") {
        return (StatusCode::BAD_REQUEST, "invalid code").into_response();
    }
    let issuer = std::env::var("AETHER_OIDC_ISSUER").unwrap_or_else(|_| "mock-idp".into());
    let now = chrono::Utc::now().timestamp();
    let nonce = oidc_pending_nonces()
        .lock()
        .expect("mock oidc nonce lock")
        .remove(&form.code);
    let mut claims = json!({
        "iss": issuer,
        "sub": "mock-oidc-user",
        "aud": MOCK_OIDC_CLIENT_ID,
        "exp": now + 3600,
        "iat": now,
        "email": "mock-user@aether.local",
        "name": "Mock IdP User",
        "groups": ["operators"],
    });
    if let Some(n) = nonce {
        claims["nonce"] = json!(n);
    }
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("mock-idp-1".to_string());
    let id_token = encode(
        &header,
        &claims,
        &EncodingKey::from_rsa_pem(MOCK_KEY_PEM.as_bytes()).expect("mock key"),
    )
    .unwrap_or_default();
    axum::Json(json!({
        "access_token": "mock-access-token",
        "token_type": "Bearer",
        "expires_in": 3600,
        "id_token": id_token,
    }))
    .into_response()
}

pub async fn oidc_jwks() -> impl IntoResponse {
    axum::Json(json!({
        "keys": [{
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "kid": "mock-idp-1",
            "n": keys().jwk_n,
            "e": keys().jwk_e,
        }]
    }))
}

fn short_id() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    (0..8)
        .map(|_| format!("{:x}", rng.gen_range(0..16)))
        .collect()
}

fn oidc_pending_nonces() -> &'static Mutex<HashMap<String, String>> {
    static PENDING: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();
    PENDING.get_or_init(|| Mutex::new(HashMap::new()))
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod signing_tests {
    use super::*;

    #[test]
    fn exclusive_comments_algorithm_extracts_cleanly() {
        let assertion = r#"<saml2:Assertion ID="_a" Version="2.0"><!-- c --></saml2:Assertion>"#;
        let sig = sign_saml_element_with_mode(assertion, "_a", true);
        assert!(
            sig.contains(r#"Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#WithComments" />"#)
                || sig.contains(r#"Algorithm="http://www.w3.org/2001/10/xml-exc-c14n#WithComments"/>"#),
            "{sig}"
        );
    }
}
