// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Optional XML-DSig verification for SAML responses (RSA-SHA256 / RSA-SHA384).

use anyhow::Context;
use base64::Engine;
use rsa::pkcs1v15::{Signature, VerifyingKey};
use rsa::pkcs8::DecodePublicKey;
use rsa::signature::Verifier;
use rsa::RsaPublicKey;
use sha2::{Digest, Sha256, Sha384};
use x509_parser::pem::parse_x509_pem;

const SIG_RSA_SHA256: &str = "http://www.w3.org/2001/04/xmldsig-more#rsa-sha256";
const SIG_RSA_SHA384: &str = "http://www.w3.org/2001/04/xmldsig-more#rsa-sha384";
const DIGEST_SHA256: &str = "http://www.w3.org/2001/04/xmlenc#sha256";
const DIGEST_SHA384: &str = "http://www.w3.org/2001/04/xmlenc#sha384";

/// When `trusted_cert_pem` is set, require a valid RSA XML signature signed by that certificate.
pub fn verify_response_signature(xml: &str, trusted_cert_pem: &str) -> anyhow::Result<()> {
    let signature_block = extract_signature_block(xml)
        .ok_or_else(|| anyhow::anyhow!("SAML response missing XML Signature"))?;
    let signed_info = extract_element_xml(&signature_block, "SignedInfo")
        .ok_or_else(|| anyhow::anyhow!("SAML signature missing SignedInfo"))?;
    let signature_value_b64 = extract_tag_text(&signature_block, "SignatureValue")
        .ok_or_else(|| anyhow::anyhow!("SAML signature missing SignatureValue"))?;
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_value_b64.trim())
        .context("invalid SignatureValue base64")?;

    let reference_uri = extract_reference_uri(&signature_block)
        .ok_or_else(|| anyhow::anyhow!("SAML signature missing Reference URI"))?;
    let digest_expected = extract_tag_text(&signature_block, "DigestValue")
        .ok_or_else(|| anyhow::anyhow!("SAML signature missing DigestValue"))?;
    let target_id = reference_uri.trim_start_matches('#');
    let signed_element = extract_element_by_id(xml, target_id)
        .ok_or_else(|| anyhow::anyhow!("signed SAML element #{target_id} not found"))?;
    let reference_xml = extract_element_xml(&signature_block, "Reference")
        .ok_or_else(|| anyhow::anyhow!("SAML signature missing Reference"))?;
    let transforms = crate::saml_c14n::extract_transform_algorithms(&reference_xml);
    let digest_bytes = if transforms.is_empty() {
        signed_element.as_bytes().to_vec()
    } else {
        crate::saml_c14n::digest_canonical_bytes(&signed_element, &transforms)?
    };
    let digest_method =
        extract_digest_method(&reference_xml).unwrap_or_else(|| DIGEST_SHA256.into());
    let digest_actual = digest_b64(&digest_bytes, &digest_method)?;
    if digest_actual != digest_expected.trim() {
        anyhow::bail!("SAML digest mismatch for #{target_id}");
    }

    let public_key = public_key_from_pem(trusted_cert_pem)?;
    let signature = Signature::try_from(sig_bytes.as_slice())
        .map_err(|e| anyhow::anyhow!("invalid RSA signature: {e}"))?;
    let c14n_algo = crate::saml_c14n::extract_canonicalization_method(&signed_info)
        .unwrap_or_else(|| crate::saml_c14n::EXC_C14N.to_string());
    let signed_info_bytes = crate::saml_c14n::canonicalize(&signed_info, &c14n_algo)
        .unwrap_or_else(|_| signed_info.as_bytes().to_vec());
    let sig_method =
        extract_signature_method(&signed_info).unwrap_or_else(|| SIG_RSA_SHA256.into());
    verify_rsa_signature(
        &public_key,
        &sig_method,
        signed_info_bytes.as_slice(),
        &signature,
    )?;
    Ok(())
}

fn digest_b64(bytes: &[u8], algorithm: &str) -> anyhow::Result<String> {
    let digest = match algorithm {
        DIGEST_SHA256 => base64::engine::general_purpose::STANDARD.encode(Sha256::digest(bytes)),
        DIGEST_SHA384 => base64::engine::general_purpose::STANDARD.encode(Sha384::digest(bytes)),
        other => anyhow::bail!("unsupported SAML digest algorithm: {other}"),
    };
    Ok(digest)
}

fn verify_rsa_signature(
    public_key: &RsaPublicKey,
    algorithm: &str,
    signed_info: &[u8],
    signature: &Signature,
) -> anyhow::Result<()> {
    match algorithm {
        SIG_RSA_SHA256 => VerifyingKey::<Sha256>::new(public_key.clone())
            .verify(signed_info, signature)
            .map_err(|_| anyhow::anyhow!("SAML RSA-SHA256 signature verification failed")),
        SIG_RSA_SHA384 => VerifyingKey::<Sha384>::new(public_key.clone())
            .verify(signed_info, signature)
            .map_err(|_| anyhow::anyhow!("SAML RSA-SHA384 signature verification failed")),
        other => anyhow::bail!("unsupported SAML signature algorithm: {other}"),
    }
}

fn extract_signature_method(signed_info: &str) -> Option<String> {
    extract_method_algorithm(signed_info, "SignatureMethod")
}

fn extract_digest_method(reference_xml: &str) -> Option<String> {
    extract_method_algorithm(reference_xml, "DigestMethod")
}

fn extract_method_algorithm(xml: &str, element: &str) -> Option<String> {
    let fragment = extract_element_xml(xml, element)?;
    extract_xml_attr(&fragment, "Algorithm")
}

fn public_key_from_pem(pem: &str) -> anyhow::Result<RsaPublicKey> {
    let (_, cert) = parse_x509_pem(pem.as_bytes()).context("parse IdP certificate PEM")?;
    let parsed = cert.parse_x509().context("parse IdP X509 certificate")?;
    RsaPublicKey::from_public_key_der(parsed.public_key().raw).context("IdP certificate is not RSA")
}

fn extract_signature_block(xml: &str) -> Option<String> {
    extract_element_xml(xml, "Signature")
}

fn extract_reference_uri(signature_block: &str) -> Option<String> {
    let reference = extract_element_xml(signature_block, "Reference")?;
    extract_xml_attr(&reference, "URI")
}

fn extract_element_by_id(xml: &str, id: &str) -> Option<String> {
    for attr in ["ID", "Id"] {
        let needle = format!("{attr}=\"{id}\"");
        if let Some(attr_pos) = xml.find(&needle) {
            let before = &xml[..attr_pos];
            let tag_start = before.rfind('<')?;
            let slice = &xml[tag_start..];
            let tag_end = slice.find('>')?;
            let tag_open = &slice[..=tag_end];
            let mut tag_name = tag_open
                .trim_start_matches('<')
                .split([' ', '/', '>'])
                .next()?
                .to_string();
            if let Some((_prefix, local)) = tag_name.split_once(':') {
                tag_name = local.to_string();
            }
            for prefix in ["", "saml2:", "saml2p:", "saml:", "samlp:", "ds:"] {
                let close = format!("</{prefix}{tag_name}>");
                if let Some(rel) = xml[tag_start..].find(&close) {
                    return Some(xml[tag_start..tag_start + rel + close.len()].to_string());
                }
            }
        }
    }
    None
}

fn extract_element_xml(xml: &str, name: &str) -> Option<String> {
    for marker in [
        format!("<{name}"),
        format!("<ds:{name}"),
        format!("<saml2:{name}"),
        format!("<samlp:{name}"),
    ] {
        let Some(start) = xml.find(&marker) else {
            continue;
        };
        for end_marker in [
            format!("</{name}>"),
            format!("</ds:{name}>"),
            format!("</saml2:{name}>"),
            format!("</samlp:{name}>"),
        ] {
            if let Some(rel) = xml[start..].find(&end_marker) {
                let end = start + rel + end_marker.len();
                return Some(xml[start..end].to_string());
            }
        }
        let slice = &xml[start..];
        if let Some(rel) = slice.find("/>") {
            let end = start + rel + 2;
            return Some(xml[start..end].to_string());
        }
    }
    None
}

fn extract_tag_text(xml: &str, tag: &str) -> Option<String> {
    for open in [format!("<{tag}"), format!("<ds:{tag}")] {
        let Some(start) = xml.find(&open) else {
            continue;
        };
        let rest = &xml[start..];
        let gt = rest.find('>')? + 1;
        for close in [format!("</{tag}>"), format!("</ds:{tag}>")] {
            if let Some(end) = rest[gt..].find(&close) {
                let inner = rest[gt..gt + end].trim();
                if !inner.is_empty() {
                    return Some(inner.to_string());
                }
            }
        }
    }
    None
}

fn extract_xml_attr(fragment: &str, attr: &str) -> Option<String> {
    let pattern = format!("{attr}=\"");
    let start = fragment.find(&pattern)? + pattern.len();
    let end = fragment[start..].find('"')? + start;
    Some(fragment[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mock_idp;

    #[test]
    fn test_extract_element_by_id() {
        let xml = r#"<Response ID="resp1"><Assertion ID="assert1"><Subject><NameID>u@x.com</NameID></Subject></Assertion></Response>"#;
        let el = extract_element_by_id(xml, "assert1").unwrap();
        assert!(el.contains("NameID"));
    }

    #[test]
    fn test_mock_idp_signature_roundtrip() {
        std::env::remove_var("AETHER_MOCK_IDP_EXCLUSIVE_COMMENTS");
        std::env::remove_var("AETHER_MOCK_IDP_SHA384");
        std::env::remove_var("AETHER_MOCK_IDP_AZURE_AD");
        let assertion_id = "_mock_assert_test";
        let assertion = format!(
            r#"<saml2:Assertion xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="{assertion_id}" Version="2.0"><saml2:Subject><saml2:NameID>mock-user@aether.local</saml2:NameID></saml2:Subject></saml2:Assertion>"#
        );
        let signature = mock_idp::sign_saml_element_for_test(&assertion, assertion_id);
        let xml = format!("<Response>{signature}{assertion}</Response>");
        verify_response_signature(&xml, mock_idp::idp_certificate_pem())
            .expect("signature roundtrip");
    }

    #[test]
    fn test_mock_idp_exclusive_comments_signature_roundtrip() {
        let assertion_id = "_mock_assert_comments";
        let assertion = format!(
            r#"<saml2:Assertion xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="{assertion_id}" Version="2.0"><!-- idp --><saml2:Subject><saml2:NameID>mock-user@aether.local</saml2:NameID></saml2:Subject></saml2:Assertion>"#
        );
        let signature =
            mock_idp::sign_saml_element_for_test_with_comments(&assertion, assertion_id);
        let xml = format!("<Response>{signature}{assertion}</Response>");
        verify_response_signature(&xml, mock_idp::idp_certificate_pem())
            .expect("exclusive comments signature roundtrip");
    }

    #[test]
    fn test_mock_idp_sha384_signature_roundtrip() {
        let assertion_id = "_mock_assert_sha384";
        let assertion = format!(
            r#"<saml2:Assertion xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="{assertion_id}" Version="2.0"><saml2:Subject><saml2:NameID>mock-user@aether.local</saml2:NameID></saml2:Subject></saml2:Assertion>"#
        );
        let signature = mock_idp::sign_saml_element_for_test_sha384(&assertion, assertion_id);
        let xml = format!("<Response>{signature}{assertion}</Response>");
        verify_response_signature(&xml, mock_idp::idp_certificate_pem()).expect("sha384 roundtrip");
    }

    #[test]
    fn test_mock_idp_azure_ad_signature_roundtrip() {
        let response_id = "_mock_resp_azure";
        let assertion_id = "_mock_assert_azure";
        let response = format!(
            r#"<saml2p:Response xmlns:saml2p="urn:oasis:names:tc:SAML:2.0:protocol" xmlns:saml2="urn:oasis:names:tc:SAML:2.0:assertion" ID="{response_id}" Version="2.0"><saml2p:Status><saml2p:StatusCode Value="urn:oasis:names:tc:SAML:2.0:status:Success"/></saml2p:Status><saml2:Assertion ID="{assertion_id}" Version="2.0"><saml2:Subject><saml2:NameID>mock-user@aether.local</saml2:NameID></saml2:Subject></saml2:Assertion></saml2p:Response>"#
        );
        let signed = mock_idp::sign_saml_response_for_test_azure(&response, response_id);
        verify_response_signature(&signed, mock_idp::idp_certificate_pem())
            .expect("azure ad dialect roundtrip");
    }
}
