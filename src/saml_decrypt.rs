// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! SAML 2.0 EncryptedAssertion decryption (AES-128-CBC + RSA-OAEP key transport).

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use anyhow::{bail, Context, Result};
use base64::Engine;
use rsa::oaep::Oaep;
use rsa::pkcs8::DecodePrivateKey;
use rsa::RsaPrivateKey;
use sha2::Sha256;
use x509_parser::pem::parse_x509_pem;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

/// Normalize SAML response XML: decrypt EncryptedAssertion when present.
pub fn normalize_saml_response_xml(xml: &str) -> Result<String> {
    if xml.contains("EncryptedAssertion") || xml.contains("xenc:EncryptedData") {
        let key_pem = std::env::var("AETHER_SAML_SP_KEY")
            .ok()
            .filter(|s| !s.is_empty())
            .context("Encrypted SAML assertion requires AETHER_SAML_SP_KEY")?;
        return decrypt_encrypted_assertion(xml, &key_pem);
    }
    Ok(xml.to_string())
}

pub fn encrypt_assertion_for_test(assertion_xml: &str, sp_cert_pem: &str) -> Result<String> {
    use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyIvInit};
    use rand::RngCore;
    use rsa::pkcs8::DecodePublicKey;
    use rsa::RsaPublicKey;

    let (_, cert) = parse_x509_pem(sp_cert_pem.as_bytes()).context("parse SP cert")?;
    let parsed = cert.parse_x509().context("parse SP X509")?;
    let public_key = RsaPublicKey::from_public_key_der(parsed.public_key().raw)
        .context("SP cert is not RSA")?;

    let mut key = [0u8; 16];
    let mut iv = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut key);
    rand::thread_rng().fill_bytes(&mut iv);

    let plaintext = assertion_xml.as_bytes();
    let mut buf = plaintext.to_vec();
    let msg_len = buf.len();
    buf.resize(msg_len + 16, 0);
    let cipher = cbc::Encryptor::<aes::Aes128>::new_from_slices(&key, &iv).context("aes key")?;
    let ciphertext = cipher
        .encrypt_padded_mut::<Pkcs7>(&mut buf, msg_len)
        .map_err(|e| anyhow::anyhow!("AES encrypt: {:?}", e))?
        .to_vec();

    let padding = Oaep::new::<Sha256>();
    let encrypted_key = public_key
        .encrypt(&mut rand::thread_rng(), padding, &key)
        .context("rsa encrypt aes key")?;

    let mut payload = iv.to_vec();
    payload.extend_from_slice(&ciphertext);

    let enc_key_b64 = base64::engine::general_purpose::STANDARD.encode(&encrypted_key);
    let cipher_b64 = base64::engine::general_purpose::STANDARD.encode(&payload);

    Ok(format!(
        r#"<saml2:EncryptedAssertion xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion">
  <xenc:EncryptedData xmlns:xenc="http://www.w3.org/2001/04/xmlenc#" Type="http://www.w3.org/2001/04/xmlenc#Element">
    <xenc:EncryptionMethod Algorithm="http://www.w3.org/2001/04/xmlenc#aes128-cbc"/>
    <ds:KeyInfo xmlns:ds="http://www.w3.org/2000/09/xmldsig#">
      <xenc:EncryptedKey>
        <xenc:EncryptionMethod Algorithm="http://www.w3.org/2001/04/xmlenc#rsa-oaep-mgf1p"/>
        <xenc:CipherData><xenc:CipherValue>{enc_key_b64}</xenc:CipherValue></xenc:CipherData>
      </xenc:EncryptedKey>
    </ds:KeyInfo>
    <xenc:CipherData><xenc:CipherValue>{cipher_b64}</xenc:CipherValue></xenc:CipherData>
  </xenc:EncryptedData>
</saml2:EncryptedAssertion>"#
    ))
}

fn decrypt_encrypted_assertion(xml: &str, sp_key_pem: &str) -> Result<String> {
    let private_key = RsaPrivateKey::from_pkcs8_pem(sp_key_pem).context("parse AETHER_SAML_SP_KEY")?;
    let enc_key_b64 = extract_cipher_values(xml)
        .into_iter()
        .next()
        .context("missing EncryptedKey CipherValue")?;
    let data_b64 = extract_cipher_values(xml)
        .into_iter()
        .nth(1)
        .context("missing EncryptedData CipherValue")?;

    let enc_key = base64::engine::general_purpose::STANDARD
        .decode(enc_key_b64.trim())
        .context("EncryptedKey base64")?;
    let mut ciphertext = base64::engine::general_purpose::STANDARD
        .decode(data_b64.trim())
        .context("EncryptedData base64")?;

    let padding = Oaep::new::<Sha256>();
    let aes_key = private_key
        .decrypt(padding, &enc_key)
        .context("RSA-OAEP decrypt session key")?;
    if aes_key.len() != 16 {
        bail!("expected AES-128 key, got {} bytes", aes_key.len());
    }

    if ciphertext.len() < 17 {
        bail!("ciphertext too short");
    }
    let iv: [u8; 16] = ciphertext[..16].try_into().unwrap();
    ciphertext.drain(..16);
    let cipher = Aes128CbcDec::new_from_slices(&aes_key, &iv).context("aes decryptor")?;
    let plaintext = cipher
        .decrypt_padded_mut::<Pkcs7>(&mut ciphertext)
        .map_err(|e| anyhow::anyhow!("AES-CBC decrypt: {:?}", e))?;
    String::from_utf8(plaintext.to_vec()).context("decrypted assertion UTF-8")
}

fn extract_cipher_values(xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find("CipherValue>") {
        let after_tag = &rest[start + 12..];
        let close = after_tag.find('<').unwrap_or(after_tag.len());
        let value = after_tag[..close].trim();
        if !value.is_empty() && value.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=') {
            out.push(value.to_string());
        }
        rest = &rest[start + 12 + close..];
    }
    out
}

fn extract_cipher_value(xml: &str, container: &str) -> Option<String> {
    let start = xml.find(container)?;
    let slice = &xml[start..];
    let cv = "<xenc:CipherValue>";
    let alt = "<CipherValue>";
    let (open, close) = if slice.contains(cv) {
        (cv, "</xenc:CipherValue>")
    } else {
        (alt, "</CipherValue>")
    };
    let i = slice.find(open)? + open.len();
    let j = slice[i..].find(close)? + i;
    Some(slice[i..j].trim().to_string())
}

#[allow(dead_code)]
fn extract_all_cipher_values(xml: &str) -> Vec<String> {
    extract_cipher_values(xml)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_idp;

    #[test]
    fn roundtrip_encrypted_assertion_with_mock_keys() {
        let assertion = r#"<saml2:Assertion ID="a1"><saml2:Subject><saml2:NameID>u@test</saml2:NameID></saml2:Subject></saml2:Assertion>"#;
        let enc = encrypt_assertion_for_test(assertion, mock_idp::idp_certificate_pem()).unwrap();
        std::env::set_var("AETHER_SAML_SP_KEY", mock_idp::idp_private_key_pem());
        let plain = normalize_saml_response_xml(&enc).unwrap();
        assert!(plain.contains("NameID"));
        std::env::remove_var("AETHER_SAML_SP_KEY");
    }
}
