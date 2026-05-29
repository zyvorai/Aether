// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use aether::saml_c14n::{canonicalize, INCLUSIVE_C14N, EXC_C14N};

#[test]
fn inclusive_c14n_strips_comments() {
    let xml = "<a><!--x--><b/></a>";
    let out = String::from_utf8(canonicalize(xml, INCLUSIVE_C14N).unwrap()).unwrap();
    assert!(!out.contains("<!--"));
}

#[test]
fn exc_c14n_and_inclusive_differ_on_whitespace() {
    let xml = "<a>  <b/></a>";
    let exc = canonicalize(xml, EXC_C14N).unwrap();
    let inc = canonicalize(xml, INCLUSIVE_C14N).unwrap();
    assert_ne!(exc, inc);
}
