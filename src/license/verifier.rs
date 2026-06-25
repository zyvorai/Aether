// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.

use anyhow::{Context, Result};
use base64::Engine;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use rsa::RsaPublicKey;
use sha2::Sha256;

use super::claims::{LicenseClaims, ZyvorEnvelope};

/// Zyvor RSA-2048 public key (PKCS#8 PEM) — embedded at compile time.
/// The matching private key is held exclusively by Zyvor and never distributed.
const ZYVOR_PUBLIC_KEY: &str = include_str!("../../keys/zyvor_public.pem");

/// Parse, verify signature, and decode claims from the raw bytes of a .zyvor file.
///
/// Returns `LicenseClaims` on success. Errors describe the specific failure:
/// wrong format, bad base64, bad signature, wrong product, etc.
pub fn verify_and_decode(raw: &[u8]) -> Result<LicenseClaims> {
    let envelope: ZyvorEnvelope =
        serde_json::from_slice(raw).context("license file is not valid JSON")?;

    if envelope.format != "zyvor-v1" {
        anyhow::bail!("unsupported license format: {}", envelope.format);
    }

    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&envelope.payload)
        .context("license payload is not valid base64url")?;

    let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(&envelope.signature)
        .context("license signature is not valid base64url")?;

    let public_key = RsaPublicKey::from_public_key_pem(ZYVOR_PUBLIC_KEY)
        .context("embedded Zyvor public key is invalid")?;

    let sig = Signature::try_from(sig_bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("invalid license signature bytes: {e}"))?;

    VerifyingKey::<Sha256>::new(public_key)
        .verify(&payload_bytes, &sig)
        .map_err(|_| anyhow::anyhow!("license signature verification failed"))?;

    let claims: LicenseClaims = serde_json::from_slice(&payload_bytes)
        .context("license payload JSON is invalid")?;

    if claims.product != "zeus-os" {
        anyhow::bail!(
            "license is not for zeus-os (got product '{}')",
            claims.product
        );
    }

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use rsa::pkcs1v15::SigningKey;
    use rsa::pkcs8::{EncodePublicKey, LineEnding};
    use rsa::signature::{Signer, SignatureEncoding};
    use rsa::RsaPrivateKey;

    fn make_envelope(product: &str, key: &RsaPrivateKey, pub_pem: &str) -> Vec<u8> {
        let claims = serde_json::json!({
            "license_id": "TEST-001",
            "product": product,
            "customer": "Test",
            "customer_id": "test",
            "allowed_nodes": 10,
            "allowed_clusters": 2,
            "valid_from": "2026-01-01",
            "valid_until": "2099-12-31",
            "issued_at": "2026-01-01",
            "license_version": 1,
        });
        let payload_bytes = serde_json::to_vec(&claims).unwrap();
        let payload_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&payload_bytes);
        let signing_key = SigningKey::<Sha256>::new(key.clone());
        let sig = signing_key.sign(&payload_bytes);
        let sig_b64 =
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig.to_bytes().as_ref());
        // Temporarily override the embedded key by using a custom verifier call.
        // We store the public key PEM alongside the envelope for test use.
        let envelope = serde_json::json!({
            "format": "zyvor-v1",
            "payload": payload_b64,
            "signature": sig_b64,
            "_test_pub_pem": pub_pem,
        });
        serde_json::to_vec(&envelope).unwrap()
    }

    /// Verify using an explicitly provided public key PEM (for tests only).
    fn verify_with_key(raw: &[u8], pub_pem: &str) -> Result<LicenseClaims> {
        let envelope: ZyvorEnvelope = serde_json::from_slice(raw)?;
        if envelope.format != "zyvor-v1" {
            anyhow::bail!("unsupported format");
        }
        let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&envelope.payload)?;
        let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&envelope.signature)?;
        let public_key = RsaPublicKey::from_public_key_pem(pub_pem)?;
        let sig = Signature::try_from(sig_bytes.as_slice())?;
        VerifyingKey::<Sha256>::new(public_key)
            .verify(&payload_bytes, &sig)
            .map_err(|_| anyhow::anyhow!("signature verification failed"))?;
        let claims: LicenseClaims = serde_json::from_slice(&payload_bytes)?;
        Ok(claims)
    }

    fn test_keypair() -> (RsaPrivateKey, String) {
        let mut rng = rand::thread_rng();
        let priv_key = RsaPrivateKey::new(&mut rng, 2048).unwrap();
        let pub_pem = priv_key
            .to_public_key()
            .to_public_key_pem(LineEnding::LF)
            .unwrap();
        (priv_key, pub_pem)
    }

    #[test]
    fn valid_license_roundtrip() {
        let (priv_key, pub_pem) = test_keypair();
        let raw = make_envelope("zeus-os", &priv_key, &pub_pem);
        let claims = verify_with_key(&raw, &pub_pem).expect("should verify");
        assert_eq!(claims.allowed_nodes, 10);
        assert_eq!(claims.product, "zeus-os");
    }

    #[test]
    fn tampered_payload_rejected() {
        let (priv_key, pub_pem) = test_keypair();
        let raw = make_envelope("zeus-os", &priv_key, &pub_pem);
        let mut envelope: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        // Corrupt the payload
        envelope["payload"] = serde_json::json!("dGFtcGVyZWQ");
        let tampered = serde_json::to_vec(&envelope).unwrap();
        assert!(verify_with_key(&tampered, &pub_pem).is_err());
    }

    #[test]
    fn wrong_key_rejected() {
        let (priv_key, _) = test_keypair();
        let (_, other_pub_pem) = test_keypair();
        let raw = make_envelope("zeus-os", &priv_key, &other_pub_pem);
        assert!(verify_with_key(&raw, &other_pub_pem).is_err());
    }

    #[test]
    fn wrong_format_rejected() {
        let envelope = serde_json::json!({
            "format": "bad-format",
            "payload": "abc",
            "signature": "def",
        });
        let raw = serde_json::to_vec(&envelope).unwrap();
        let err = verify_with_key(&raw, ZYVOR_PUBLIC_KEY).unwrap_err();
        assert!(err.to_string().contains("format"));
    }

    #[test]
    fn bad_json_rejected() {
        assert!(verify_with_key(b"not json at all", ZYVOR_PUBLIC_KEY).is_err());
    }
}
