// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

//! End-to-end integration tests for the license subsystem.
//!
//! These tests exercise `LicenseStore`, `compute_state`, `verifier::verify_and_decode`,
//! and `emit_startup_audit` using real RSA keypairs generated at test time.
//!
//! # Note on the embedded Zyvor public key
//!
//! `LicenseStore::load_from_path` always verifies against the embedded Zyvor RSA-2048
//! public key, so any envelope we generate in tests with a throw-away keypair will
//! inevitably fail that check and produce `InvalidSignature` (not `Valid`).
//!
//! Tests that need a genuine round-trip through sign → verify → decode (tests 1, 7, 8)
//! therefore use a module-local `verify_with_key` helper that accepts an explicit PEM,
//! mirroring the pattern used in `src/license/verifier.rs` unit tests.
//!
//! Tests that target `LicenseStore::load_from_path` directly (tests 2, 7, 8, 9) rely
//! on the fact that any self-signed envelope is correctly classified as
//! `InvalidSignature` by the embedded-key check.

use std::path::PathBuf;

use aether::license::{enforcement, verifier, LicenseClaims, LicenseState, LicenseStore};
use anyhow::{Context as _, Result};
use base64::Engine as _;
use chrono::{Duration, Utc};
use rsa::{
    pkcs1v15::{Signature, SigningKey, VerifyingKey},
    pkcs8::{DecodePublicKey, EncodePublicKey, LineEnding},
    signature::{SignatureEncoding, Signer, Verifier as _},
    RsaPrivateKey, RsaPublicKey,
};
use sha2::Sha256;
use tempfile::tempdir;

// ── shared helpers ────────────────────────────────────────────────────────────

/// Generate a fresh RSA-2048 keypair.  Returns `(private_key, public_key_pem)`.
fn test_keypair() -> (RsaPrivateKey, String) {
    let mut rng = rand::thread_rng();
    let priv_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
    let pub_pem = priv_key
        .to_public_key()
        .to_public_key_pem(LineEnding::LF)
        .unwrap();
    (priv_key, pub_pem)
}

/// Construct a `LicenseClaims` value for use in tests.
fn make_claims(allowed_nodes: u32, valid_from: &str, valid_until: &str) -> LicenseClaims {
    LicenseClaims {
        license_id: "ZV-TEST-E2E-0001".into(),
        product: "zeus-os".into(),
        customer: "Acme Corp".into(),
        customer_id: "acme-corp".into(),
        allowed_nodes,
        allowed_clusters: 3,
        valid_from: valid_from.into(),
        valid_until: valid_until.into(),
        issued_at: "2026-01-01".into(),
        license_version: 1,
    }
}

/// Serialise `claims`, sign with `priv_key`, and return the raw bytes of a
/// `.zyvor` JSON envelope (`format`, `payload`, `signature`).
fn make_envelope(claims: &LicenseClaims, priv_key: &RsaPrivateKey) -> Vec<u8> {
    let payload_bytes = serde_json::to_vec(claims).unwrap();
    let payload_b64 =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&payload_bytes);

    let signing_key = SigningKey::<Sha256>::new(priv_key.clone());
    let sig: Signature = signing_key.sign(&payload_bytes);
    let sig_b64 =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes().as_ref());

    let envelope = serde_json::json!({
        "format": "zyvor-v1",
        "payload": payload_b64,
        "signature": sig_b64,
    });
    serde_json::to_vec(&envelope).unwrap()
}

/// Verify a `.zyvor` envelope using an *explicit* PEM public key (test-only).
///
/// Mirrors the private `verify_with_key` helper from `src/license/verifier.rs`
/// unit tests.  Includes the same `product == "zeus-os"` guard that
/// `verifier::verify_and_decode` enforces.
fn verify_with_key(raw: &[u8], pub_pem: &str) -> Result<LicenseClaims> {
    let envelope: serde_json::Value = serde_json::from_slice(raw)?;

    let format = envelope["format"].as_str().unwrap_or("");
    anyhow::ensure!(format == "zyvor-v1", "unsupported format: {}", format);

    let payload_b64 = envelope["payload"]
        .as_str()
        .context("missing payload field")?;
    let sig_b64 = envelope["signature"]
        .as_str()
        .context("missing signature field")?;

    let payload_bytes =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload_b64)?;
    let sig_bytes =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(sig_b64)?;

    let public_key = RsaPublicKey::from_public_key_pem(pub_pem)?;
    let sig = Signature::try_from(sig_bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("invalid signature bytes: {e}"))?;

    VerifyingKey::<Sha256>::new(public_key)
        .verify(&payload_bytes, &sig)
        .map_err(|_| anyhow::anyhow!("signature verification failed"))?;

    let claims: LicenseClaims = serde_json::from_slice(&payload_bytes)?;

    if claims.product != "zeus-os" {
        anyhow::bail!(
            "license is not for zeus-os (got product '{}')",
            claims.product
        );
    }
    Ok(claims)
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// 1. Generate RSA keypair, sign claims with allowed_nodes=10, valid 2026-2099,
///    verify with `verify_with_key` (local helper that accepts an explicit PEM),
///    compute state → VALID.
#[test]
fn valid_license_e2e() {
    let (priv_key, pub_pem) = test_keypair();
    let claims = make_claims(10, "2026-01-01", "2099-12-31");
    let envelope = make_envelope(&claims, &priv_key);

    // Round-trip through our local verifier (bypasses the embedded Zyvor key).
    let decoded =
        verify_with_key(&envelope, &pub_pem).expect("verification with matching key should succeed");
    assert_eq!(decoded.allowed_nodes, 10);
    assert_eq!(decoded.product, "zeus-os");
    assert_eq!(decoded.customer, "Acme Corp");
    assert_eq!(decoded.license_id, "ZV-TEST-E2E-0001");

    // compute_state should report Valid for a fresh, non-expiring, within-limit license.
    let state = enforcement::compute_state(&decoded, None);
    assert_eq!(state, LicenseState::Valid, "10-node license with no live count should be Valid");
}

/// 2. `LicenseStore::load_from_path` on a non-existent path → state MISSING.
#[test]
fn missing_license_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("license.zyvor"); // file does not exist

    let store = LicenseStore::load_from_path(path);
    assert_eq!(store.state, LicenseState::Missing);
    assert!(store.claims.is_none());
    assert!(store.last_node_count.is_none());

    // Sanity-check that the public verifier also rejects garbage bytes.
    assert!(
        verifier::verify_and_decode(b"not json at all").is_err(),
        "verify_and_decode must error on non-JSON input"
    );
}

/// 3. Sign license with valid_until in the past → `compute_state` returns EXPIRED_GRACE.
#[test]
fn expired_license_grace() {
    let claims = make_claims(100, "2020-01-01", "2021-06-30");
    let state = enforcement::compute_state(&claims, None);
    assert_eq!(
        state,
        LicenseState::ExpiredGrace,
        "a license whose valid_until is in the past should be ExpiredGrace (warn-only in v0.3.0)"
    );
}

/// 4. Sign license with valid_until 20 days from today → `compute_state` returns EXPIRING_SOON.
#[test]
fn expiring_soon() {
    let today = Utc::now().date_naive();
    let soon = (today + Duration::days(20)).format("%Y-%m-%d").to_string();
    let claims = make_claims(100, "2026-01-01", &soon);

    let state = enforcement::compute_state(&claims, None);
    assert_eq!(
        state,
        LicenseState::ExpiringSoon,
        "20 days remaining is less than the 30-day warning threshold"
    );
}

/// 5. allowed_nodes=5, `refresh_state(Some(6))` → state OVER_LIMIT_GRACE.
#[test]
fn over_limit_grace() {
    let claims = make_claims(5, "2026-01-01", "2099-12-31");

    // Construct a LicenseStore directly — its public fields allow this without
    // needing the embedded Zyvor key.
    let mut store = LicenseStore {
        claims: Some(claims),
        state: LicenseState::Valid,
        last_node_count: None,
        path: PathBuf::from("/tmp/test-over-limit.zyvor"),
    };

    store.refresh_state(Some(6));
    assert_eq!(
        store.state,
        LicenseState::OverLimitGrace,
        "6 nodes exceeds the allowed_nodes=5 limit"
    );
    assert_eq!(store.last_node_count, Some(6));
}

/// 6. allowed_nodes=5, `refresh_state(Some(5))` → state VALID (at-limit is not over-limit).
#[test]
fn not_over_limit_at_limit() {
    let claims = make_claims(5, "2026-01-01", "2099-12-31");

    let mut store = LicenseStore {
        claims: Some(claims),
        state: LicenseState::Valid,
        last_node_count: None,
        path: PathBuf::from("/tmp/test-at-limit.zyvor"),
    };

    store.refresh_state(Some(5));
    assert_eq!(
        store.state,
        LicenseState::Valid,
        "exactly 5 nodes against a 5-node limit should remain Valid"
    );
    assert_eq!(store.last_node_count, Some(5));
}

/// 7. Write a structurally valid envelope to disk but signed with our test key
///    (not Zyvor's key).  `LicenseStore::load_from_path` must classify this as
///    INVALID_SIGNATURE because the embedded Zyvor public key does not match.
///
///    Additionally: corrupt the signature bytes in the JSON and confirm that
///    our local `verify_with_key` also rejects the tampered file.
#[test]
fn invalid_signature() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("license.zyvor");

    let (priv_key, pub_pem) = test_keypair();
    let claims = make_claims(10, "2026-01-01", "2099-12-31");
    let envelope_bytes = make_envelope(&claims, &priv_key);

    // --- Part A: valid envelope for our key, but Zyvor's embedded key rejects it ---
    std::fs::write(&path, &envelope_bytes).unwrap();
    let store = LicenseStore::load_from_path(path.clone());
    assert_eq!(
        store.state,
        LicenseState::InvalidSignature,
        "envelope signed with a non-Zyvor key must be InvalidSignature"
    );
    assert!(store.claims.is_none());

    // --- Part B: corrupt the signature bytes and confirm local verifier rejects it ---
    let mut envelope_value: serde_json::Value =
        serde_json::from_slice(&envelope_bytes).unwrap();
    envelope_value["signature"] = serde_json::json!("AAAAAAAAAAAAAAAAAAAAAA");
    let tampered = serde_json::to_vec(&envelope_value).unwrap();

    let result = verify_with_key(&tampered, &pub_pem);
    assert!(
        result.is_err(),
        "corrupted signature bytes must fail local verify_with_key"
    );
}

/// 8. Sign a license with product="wrong-product".
///
///    Via `LicenseStore::load_from_path`: the embedded Zyvor key cannot verify
///    our test-generated signature, so the state is INVALID_SIGNATURE (the product
///    check is never reached).
///
///    Via our local `verify_with_key` helper: the signature is valid for the test
///    key, but the product guard rejects it with a "zeus-os" error message —
///    demonstrating that the verifier enforces product correctness once the
///    signature passes.
#[test]
fn wrong_product() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("wrong-product.zyvor");

    let (priv_key, pub_pem) = test_keypair();
    let mut claims = make_claims(10, "2026-01-01", "2099-12-31");
    claims.product = "wrong-product".into();
    let envelope_bytes = make_envelope(&claims, &priv_key);

    // --- Via LicenseStore::load_from_path (uses embedded Zyvor key) ---
    std::fs::write(&path, &envelope_bytes).unwrap();
    let store = LicenseStore::load_from_path(path);
    assert_eq!(
        store.state,
        LicenseState::InvalidSignature,
        "embedded Zyvor key rejects our test key before reaching the product check"
    );

    // --- Via local verify_with_key (uses our test key) ---
    let err = verify_with_key(&envelope_bytes, &pub_pem)
        .expect_err("wrong product must be rejected even when signature is valid");
    assert!(
        err.to_string().contains("zeus-os"),
        "error message should mention the expected product; got: {err}"
    );
}

/// 9. Load a missing license (MISSING), then write a valid-structured (but
///    self-signed) license to the same path and reload.
///
///    Because the embedded Zyvor key cannot verify our test signature, the
///    reloaded state is INVALID_SIGNATURE rather than VALID.  The important
///    invariant is that state transitions from MISSING to a non-MISSING value
///    once the file exists — confirming that reload observes the filesystem.
#[test]
fn reload_updates_state() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("license.zyvor");

    // First load: file does not exist.
    let store1 = LicenseStore::load_from_path(path.clone());
    assert_eq!(
        store1.state,
        LicenseState::Missing,
        "initial state should be MISSING when no file exists"
    );

    // Write a self-signed (structurally valid) license file.
    let (priv_key, _) = test_keypair();
    let claims = make_claims(10, "2026-01-01", "2099-12-31");
    let envelope = make_envelope(&claims, &priv_key);
    std::fs::write(&path, &envelope).unwrap();

    // Second load: file now exists, but our test key fails the embedded Zyvor check.
    let store2 = LicenseStore::load_from_path(path);
    assert_ne!(
        store2.state,
        LicenseState::Missing,
        "state must change from MISSING once a file is present"
    );
    assert_eq!(
        store2.state,
        LicenseState::InvalidSignature,
        "self-signed envelope must fail the embedded Zyvor public key check"
    );
}

/// 10. `emit_startup_audit` must not panic for any `LicenseState` variant.
///
///     The function writes to the default audit log path (`~/.aether/audit.json`);
///     if the path is not writable in the test environment it logs a warning and
///     returns gracefully — it never panics.
#[test]
fn startup_audit_emits_event() {
    let all_states = [
        LicenseState::Valid,
        LicenseState::ExpiringSoon,
        LicenseState::ExpiredGrace,
        LicenseState::ExpiredBlocked,
        LicenseState::OverLimitGrace,
        LicenseState::OverLimitBlocked,
        LicenseState::Missing,
        LicenseState::InvalidSignature,
        LicenseState::WrongProduct,
    ];

    for state in &all_states {
        let store = LicenseStore {
            claims: None,
            state: state.clone(),
            last_node_count: None,
            path: PathBuf::from("/etc/zeus/license/license.zyvor"),
        };
        // Must complete without panicking for every variant.
        aether::license::emit_startup_audit(&store);
    }
}
