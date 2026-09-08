// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Lightweight XML canonicalization dialects for SAML XML-DSig verification.

use anyhow::{bail, Result};

pub const EXC_C14N: &str = "http://www.w3.org/2001/10/xml-exc-c14n#";
pub const EXC_C14N_COMMENTS: &str = "http://www.w3.org/2001/10/xml-exc-c14n#WithComments";
pub const INCLUSIVE_C14N: &str = "http://www.w3.org/TR/2001/REC-xml-c14n-20010315";
pub const INCLUSIVE_C14N_COMMENTS: &str =
    "http://www.w3.org/TR/2001/REC-xml-c14n-20010315#WithComments";

/// Canonicalize XML per the algorithm URI declared in SignedInfo / Transform.
pub fn canonicalize(xml: &str, algorithm: &str) -> Result<Vec<u8>> {
    match algorithm {
        EXC_C14N | EXC_C14N_COMMENTS => Ok(exc_c14n(xml, algorithm == EXC_C14N_COMMENTS)),
        INCLUSIVE_C14N | INCLUSIVE_C14N_COMMENTS => {
            Ok(inclusive_c14n(xml, algorithm == INCLUSIVE_C14N_COMMENTS))
        }
        other => bail!("unsupported XML canonicalization algorithm: {other}"),
    }
}

pub fn extract_canonicalization_method(signed_info: &str) -> Option<String> {
    extract_algorithm_attr(signed_info, "CanonicalizationMethod")
}

pub fn extract_transform_algorithms(reference_xml: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = reference_xml;
    while let Some(pos) = rest.find("Transform") {
        let slice = &rest[pos..];
        if let Some(algo) = extract_algorithm_attr(slice, "Transform") {
            out.push(algo);
        }
        rest = &rest[pos + 9..];
    }
    out
}

pub fn apply_enveloped_signature_transform(xml: &str) -> String {
    let mut out = xml.to_string();
    while let Some(start) = out.find("<ds:Signature") {
        let Some(end) = out[start..].find("</ds:Signature>") else {
            break;
        };
        let end = start + end + "</ds:Signature>".len();
        out.replace_range(start..end, "");
    }
    while let Some(start) = out.find("<Signature") {
        if out[start..].starts_with("<SignatureValue")
            || out[start..].starts_with("<SignatureMethod")
        {
            break;
        }
        let Some(end) = out[start..].find("</Signature>") else {
            break;
        };
        let end = start + end + "</Signature>".len();
        out.replace_range(start..end, "");
    }
    out
}

pub fn digest_canonical_bytes(xml: &str, transforms: &[String]) -> Result<Vec<u8>> {
    let mut current = xml.to_string();
    for transform in transforms {
        if transform == "http://www.w3.org/2000/09/xmldsig#enveloped-signature" {
            current = apply_enveloped_signature_transform(&current);
            continue;
        }
        if transform.starts_with("http://www.w3.org/2001/10/xml-exc-c14n")
            || transform.starts_with("http://www.w3.org/TR/2001/REC-xml-c14n")
        {
            return canonicalize(&current, transform);
        }
    }
    canonicalize(&current, EXC_C14N)
}

fn exc_c14n(xml: &str, keep_comments: bool) -> Vec<u8> {
    let normalized = normalize_xml(xml, keep_comments, true);
    normalized.into_bytes()
}

fn inclusive_c14n(xml: &str, keep_comments: bool) -> Vec<u8> {
    let normalized = normalize_xml(xml, keep_comments, false);
    normalized.into_bytes()
}

fn normalize_xml(xml: &str, keep_comments: bool, exclusive: bool) -> String {
    let mut s = xml.trim().to_string();
    if let Some(stripped) = s.strip_prefix("<?xml") {
        if let Some(end) = stripped.find("?>") {
            s = stripped[end + 2..].trim_start().to_string();
        }
    }
    if !keep_comments {
        s = strip_xml_comments(&s);
    }
    s = collapse_inter_element_whitespace(&s, exclusive);
    s = sort_attributes_in_tags(&s);
    s
}

fn strip_xml_comments(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(start) = rest.find("<!--") {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find("-->") else {
            break;
        };
        rest = &rest[start + end + 3..];
    }
    out.push_str(rest);
    out
}

fn collapse_inter_element_whitespace(xml: &str, exclusive: bool) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut in_tag = false;
    let mut last_was_close = false;
    let mut pending_space = false;
    for ch in xml.chars() {
        match ch {
            '<' => {
                if pending_space {
                    out.push(' ');
                    pending_space = false;
                }
                in_tag = true;
                out.push(ch);
                last_was_close = false;
            }
            '>' => {
                in_tag = false;
                out.push(ch);
                last_was_close = true;
            }
            c if c.is_whitespace() && !in_tag => {
                if last_was_close {
                    if !exclusive {
                        pending_space = true;
                    }
                } else if !pending_space {
                    pending_space = true;
                }
            }
            c => {
                if pending_space {
                    out.push(' ');
                    pending_space = false;
                }
                out.push(c);
                last_was_close = false;
            }
        }
    }
    out
}

fn sort_attributes_in_tags(xml: &str) -> String {
    let mut out = String::with_capacity(xml.len());
    let mut rest = xml;
    while let Some(lt) = rest.find('<') {
        out.push_str(&rest[..lt]);
        let after = &rest[lt + 1..];
        if after.starts_with('?') || after.starts_with('!') {
            let gt = after.find('>').unwrap_or(after.len());
            out.push('<');
            out.push_str(&after[..=gt]);
            rest = &after[gt + 1..];
            continue;
        }
        let gt = after.find('>').unwrap_or(after.len());
        let tag_content = &after[..gt];
        let is_close = tag_content.starts_with('/');
        let is_self_close = tag_content.ends_with('/');
        let inner = tag_content.trim_start_matches('/').trim_end_matches('/');
        let mut parts = inner.splitn(2, ' ');
        let tag_name = parts.next().unwrap_or("");
        let attrs = parts.next().unwrap_or("");
        out.push('<');
        if is_close {
            out.push('/');
        }
        out.push_str(tag_name);
        if !is_close && !attrs.is_empty() {
            let mut attr_pairs: Vec<_> = attrs
                .split_whitespace()
                .filter_map(|p| {
                    let (k, v) = p.split_once('=')?;
                    Some((k, v))
                })
                .collect();
            attr_pairs.sort_by(|a, b| a.0.cmp(b.0));
            for (k, v) in attr_pairs {
                out.push(' ');
                out.push_str(k);
                out.push('=');
                out.push_str(v);
            }
        }
        if is_self_close {
            out.push('/');
        }
        out.push('>');
        rest = &after[gt + 1..];
    }
    out.push_str(rest);
    out
}

fn extract_algorithm_attr(fragment: &str, element: &str) -> Option<String> {
    for marker in [format!("<{element}"), format!("<ds:{element}")] {
        let Some(start) = fragment.find(&marker) else {
            continue;
        };
        let slice = &fragment[start..];
        let gt = slice.find('>')?;
        let open = &slice[..gt];
        for part in open.split_whitespace() {
            if let Some(algo) = part.strip_prefix("Algorithm=\"") {
                return Some(algo.trim_end_matches('"').to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_comments_for_exc_c14n_with_comments() {
        let xml = "<a><!--note--><b/></a>";
        let out = String::from_utf8(canonicalize(xml, EXC_C14N_COMMENTS).unwrap()).unwrap();
        assert!(out.contains("note"));
    }

    #[test]
    fn strips_comments_for_exc_c14n() {
        let xml = "<a><!--c--> <b x=\"2\" y=\"1\"/></a>";
        let out = String::from_utf8(canonicalize(xml, EXC_C14N).unwrap()).unwrap();
        assert!(!out.contains("<!--"));
    }

    #[test]
    fn enveloped_signature_removes_signature_block() {
        let xml = "<Response><ds:Signature>sig</ds:Signature><Assertion/></Response>";
        let out = apply_enveloped_signature_transform(xml);
        assert!(!out.contains("Signature"));
        assert!(out.contains("Assertion"));
    }
}
