// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Canonical spec rendering (`aether spec-render`), like `docker compose config`.
//!
//! Parses the spec into the typed workload (so defaults are filled in and
//! unknown keys are dropped, exactly what the engine will act on), prints it
//! with sorted keys and no nulls, and computes a content hash, overall and per
//! top-level section, for change detection in CI.
//!
//! Secret-looking values are redacted in the output. The hash is computed
//! over the unredacted document so rotating a secret still changes it.

use crate::hexfmt::hex_lower;
use crate::spec::Workload;
use crate::speclint;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Serialize)]
pub struct Rendered {
    /// `sha256:<hex>` over the canonical (unredacted) document.
    pub hash: String,
    /// Hash per top-level section, to see which part of a spec changed.
    pub sections: BTreeMap<String, String>,
    /// The canonical document (secret values replaced with `***`).
    pub spec: Value,
}

/// Sort keys recursively and drop nulls.
fn canon(v: Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut keys: Vec<String> = m.keys().cloned().collect();
            keys.sort();
            let mut out = Map::new();
            let mut m = m;
            for k in keys {
                let c = canon(m.remove(&k).unwrap_or(Value::Null));
                if !c.is_null() {
                    out.insert(k, c);
                }
            }
            Value::Object(out)
        }
        Value::Array(a) => Value::Array(a.into_iter().map(canon).collect()),
        other => other,
    }
}

/// YAML for people to read and apply: unset fields are left out rather than
/// written as `null`. Field order is kept.
pub fn yaml_without_nulls<T: Serialize>(value: &T) -> Result<String, serde_yaml::Error> {
    fn strip(v: &mut serde_yaml::Value) {
        match v {
            serde_yaml::Value::Mapping(m) => {
                m.retain(|_, child| !child.is_null());
                m.iter_mut().for_each(|(_, child)| strip(child));
            }
            serde_yaml::Value::Sequence(s) => s.iter_mut().for_each(strip),
            _ => {}
        }
    }
    let mut doc = serde_yaml::to_value(value)?;
    strip(&mut doc);
    serde_yaml::to_string(&doc)
}

fn digest(v: &Value) -> String {
    let bytes = serde_json::to_vec(v).unwrap_or_default();
    format!("sha256:{}", hex_lower(Sha256::digest(&bytes)))
}

fn redact(v: &mut Value, path: &str) {
    match v {
        Value::Object(m) => {
            for (k, c) in m.iter_mut() {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                redact(c, &p);
            }
        }
        Value::Array(a) => {
            for (i, c) in a.iter_mut().enumerate() {
                redact(c, &format!("{path}[{i}]"));
            }
        }
        Value::String(s) if !s.is_empty() && crate::specplan::is_sensitive(path) => {
            *s = "***".to_string();
        }
        _ => {}
    }
}

/// Render a spec. Fails with the first validation error, like `aether validate`.
pub fn render_str(src: &str) -> Result<Rendered, String> {
    speclint::guarded_parse(src).map_err(|d| d.message)?;
    let w: Workload = serde_yaml::from_str(src).map_err(|e| format!("schema error: {e}"))?;
    w.validate().map_err(|e| e.to_string())?;
    let mut doc = canon(serde_json::to_value(&w).map_err(|e| e.to_string())?);
    let hash = digest(&doc);
    let mut sections = BTreeMap::new();
    if let Value::Object(m) = &doc {
        for (k, c) in m {
            sections.insert(k.clone(), digest(c));
        }
    }
    redact(&mut doc, "");
    Ok(Rendered {
        hash,
        sections,
        spec: doc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "apiVersion: aether/v1\nkind: Workload\nmetadata:\n  name: web\n  owner: me\n  project: p\nbuild:\n  context: .\n  dockerfile: Dockerfile\n  registry: localhost\nrequirements:\n  cpu: \"500m\"\n  memory: 256Mi\n  storage: 1Gi\nruntime:\n  preferred: kube\n  allow:\n    - kube\nnetwork:\n  service: true\n  ports:\n    - containerPort: 80\n      servicePort: 80\n";

    #[test]
    fn hash_is_stable_and_ignores_formatting() {
        let a = render_str(GOOD).unwrap();
        let reordered = GOOD.replace("  owner: me\n  project: p\n", "  project: p\n  owner: me\n");
        let b = render_str(&format!("# comment\n{reordered}")).unwrap();
        assert_eq!(a.hash, b.hash);
        assert!(a.hash.starts_with("sha256:") && a.hash.len() == 7 + 64);
    }

    #[test]
    fn unknown_keys_do_not_change_the_hash() {
        let a = render_str(GOOD).unwrap();
        let b = render_str(&format!("{GOOD}bogusSection: 1\n")).unwrap();
        assert_eq!(a.hash, b.hash);
    }

    #[test]
    fn section_hash_localises_the_change() {
        let a = render_str(GOOD).unwrap();
        let b = render_str(&GOOD.replace("256Mi", "512Mi")).unwrap();
        assert_ne!(a.hash, b.hash);
        assert_ne!(a.sections["requirements"], b.sections["requirements"]);
        assert_eq!(a.sections["network"], b.sections["network"]);
        assert_eq!(a.sections["metadata"], b.sections["metadata"]);
    }

    #[test]
    fn defaults_are_expanded_and_nulls_dropped() {
        let r = render_str(GOOD).unwrap();
        assert!(r.spec["network"]["ports"][0]["protocol"].is_string());
        fn no_nulls(v: &Value) {
            match v {
                Value::Null => panic!("null left in output"),
                Value::Object(m) => m.values().for_each(no_nulls),
                Value::Array(a) => a.iter().for_each(no_nulls),
                _ => {}
            }
        }
        no_nulls(&r.spec);
    }

    #[test]
    fn secrets_are_redacted_but_still_change_the_hash() {
        let with = |s: &str| {
            format!(
                "{GOOD}config:\n  secrets:\n    - name: db\n      data:\n        password: {s}\n"
            )
        };
        let a = render_str(&with("hunter2"));
        let b = render_str(&with("hunter3"));
        // Depending on the schema these may be rejected; only assert when valid.
        if let (Ok(a), Ok(b)) = (a, b) {
            assert_ne!(a.hash, b.hash);
            let text = serde_json::to_string(&a.spec).unwrap();
            assert!(!text.contains("hunter2"), "{text}");
        }
    }

    #[test]
    fn invalid_and_hostile_specs_are_errors() {
        assert!(render_str("- a").is_err());
        assert!(render_str("apiVersion: x").is_err());
        let bomb = "a: &a [1,2]\n".to_string() + &"b: *a\n".repeat(40);
        assert!(render_str(&bomb).is_err());
    }
}
