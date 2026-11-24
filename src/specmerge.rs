// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Spec overlays (`aether spec-merge`).
//!
//! One base spec plus per-environment overlays and `--set key=value`
//! overrides, the way kustomize patches and Score overrides work, but offline
//! and on Aether's own spec. Overlays use JSON Merge Patch semantics
//! (RFC 7386): mappings merge recursively, a `null` deletes the key, and
//! anything else (including a list) replaces the base value.
//!
//! Every input goes through [`speclint::guarded_parse`], and the merged
//! result is linted so a bad overlay is caught before it is deployed.

use crate::speclint::{self, LintOptions, LintReport};
use serde::Serialize;
use serde_yaml::{Mapping, Value};

/// Maximum overlay documents per merge.
pub const MAX_OVERLAYS: usize = 16;
/// Maximum `--set` overrides per merge.
pub const MAX_SETS: usize = 64;
/// Maximum depth of a `--set` path.
pub const MAX_SET_DEPTH: usize = 16;

#[derive(Debug, Serialize)]
pub struct MergeResult {
    /// The merged spec as YAML.
    pub spec: String,
    /// Lint of the merged spec.
    pub lint: LintReport,
}

/// Apply `patch` to `base` with RFC 7386 semantics.
pub fn merge_patch(base: &mut Value, patch: &Value) {
    match patch {
        Value::Mapping(pm) => {
            if !matches!(base, Value::Mapping(_)) {
                *base = Value::Mapping(Mapping::new());
            }
            let Value::Mapping(bm) = base else { return };
            for (k, v) in pm {
                if v.is_null() {
                    bm.remove(k);
                } else {
                    let slot = bm.entry(k.clone()).or_insert(Value::Null);
                    merge_patch(slot, v);
                }
            }
        }
        other => *base = other.clone(),
    }
}

#[derive(Debug, PartialEq)]
enum Seg {
    Key(String),
    Idx(usize),
}

fn parse_path(path: &str) -> Result<Vec<Seg>, String> {
    let mut segs = Vec::new();
    for part in path.split('.') {
        if part.is_empty() {
            return Err(format!("empty segment in path `{path}`"));
        }
        let (key, mut rest) = match part.find('[') {
            Some(i) => (&part[..i], &part[i..]),
            None => (part, ""),
        };
        if !key.is_empty() {
            segs.push(Seg::Key(key.to_string()));
        } else if rest.is_empty() {
            return Err(format!("empty segment in path `{path}`"));
        }
        while let Some(r) = rest.strip_prefix('[') {
            let end = r
                .find(']')
                .ok_or_else(|| format!("unclosed `[` in path `{path}`"))?;
            let idx: usize = r[..end]
                .parse()
                .map_err(|_| format!("`{}` is not a list index in `{path}`", &r[..end]))?;
            segs.push(Seg::Idx(idx));
            rest = &r[end + 1..];
        }
        if !rest.is_empty() {
            return Err(format!("unexpected `{rest}` in path `{path}`"));
        }
    }
    if segs.len() > MAX_SET_DEPTH {
        return Err(format!("path `{path}` is deeper than {MAX_SET_DEPTH}"));
    }
    Ok(segs)
}

fn remove_at(cur: &mut Value, segs: &[Seg]) {
    let Some((first, rest)) = segs.split_first() else {
        return;
    };
    match (first, cur) {
        (Seg::Key(k), Value::Mapping(m)) => {
            let key = Value::String(k.clone());
            if rest.is_empty() {
                m.remove(&key);
            } else if let Some(c) = m.get_mut(&key) {
                remove_at(c, rest);
            }
        }
        (Seg::Idx(i), Value::Sequence(s)) => {
            if rest.is_empty() {
                if *i < s.len() {
                    s.remove(*i);
                }
            } else if let Some(c) = s.get_mut(*i) {
                remove_at(c, rest);
            }
        }
        _ => {}
    }
}

fn set_at(cur: &mut Value, segs: &[Seg], value: Value) -> Result<(), String> {
    let Some((first, rest)) = segs.split_first() else {
        *cur = value;
        return Ok(());
    };
    match first {
        Seg::Key(k) => {
            if cur.is_null() {
                *cur = Value::Mapping(Mapping::new());
            }
            let Value::Mapping(m) = cur else {
                return Err(format!("`{k}` is not inside a mapping"));
            };
            let slot = m.entry(Value::String(k.clone())).or_insert(Value::Null);
            set_at(slot, rest, value)
        }
        Seg::Idx(i) => {
            if cur.is_null() {
                *cur = Value::Sequence(Vec::new());
            }
            let Value::Sequence(s) = cur else {
                return Err(format!("[{i}] is not inside a list"));
            };
            if *i > s.len() {
                return Err(format!(
                    "index {i} is past the end of a list of {} (use {} to append)",
                    s.len(),
                    s.len()
                ));
            }
            if *i == s.len() {
                s.push(Value::Null);
            }
            set_at(&mut s[*i], rest, value)
        }
    }
}

/// Apply one `path=value` override. The value is parsed as a YAML scalar, so
/// `replicas=3` is a number and `name="3"` a string; `path=null` deletes.
pub fn apply_set(doc: &mut Value, assignment: &str) -> Result<(), String> {
    let (path, raw) = assignment
        .split_once('=')
        .ok_or_else(|| format!("`{assignment}` must look like path=value"))?;
    let segs = parse_path(path.trim())?;
    let value: Value = if raw.is_empty() {
        Value::String(String::new())
    } else {
        // Flow collections would let a value smuggle structure; keep scalars.
        let v: Value = serde_yaml::from_str(raw)
            .map_err(|e| format!("value for `{path}` is not valid YAML: {e}"))?;
        if matches!(v, Value::Mapping(_) | Value::Sequence(_) | Value::Tagged(_)) {
            Value::String(raw.to_string())
        } else {
            v
        }
    };
    if value.is_null() {
        remove_at(doc, &segs);
        Ok(())
    } else {
        set_at(doc, &segs, value)
    }
}

fn parse_doc(src: &str, what: &str) -> Result<Value, String> {
    let v = speclint::guarded_parse(src).map_err(|d| format!("{what}: {}", d.message))?;
    if !matches!(v, Value::Mapping(_)) {
        return Err(format!("{what} must be a YAML mapping"));
    }
    Ok(v)
}

/// Merge `base` with `overlays` (in order) and `sets`, then lint the result.
pub fn merge_str(base: &str, overlays: &[String], sets: &[String]) -> Result<MergeResult, String> {
    if overlays.len() > MAX_OVERLAYS {
        return Err(format!("at most {MAX_OVERLAYS} overlays are accepted"));
    }
    if sets.len() > MAX_SETS {
        return Err(format!("at most {MAX_SETS} --set overrides are accepted"));
    }
    let mut doc = parse_doc(base, "base spec")?;
    for (i, o) in overlays.iter().enumerate() {
        let patch = parse_doc(o, &format!("overlay {}", i + 1))?;
        merge_patch(&mut doc, &patch);
    }
    for s in sets {
        apply_set(&mut doc, s)?;
    }
    let spec = serde_yaml::to_string(&doc).map_err(|e| e.to_string())?;
    let lint = speclint::lint_with(&spec, &LintOptions::default());
    Ok(MergeResult { spec, lint })
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = "apiVersion: aether/v1\nkind: Workload\nmetadata:\n  name: web\n  owner: me\n  project: p\n  labels:\n    tier: web\n    team: a\nbuild:\n  context: .\n  dockerfile: Dockerfile\n  registry: localhost\nrequirements:\n  cpu: \"500m\"\n  memory: 256Mi\n  storage: 1Gi\nruntime:\n  preferred: kube\n  allow:\n    - kube\nnetwork:\n  service: true\n  ports:\n    - containerPort: 80\n      servicePort: 80\n";

    fn y(s: &str) -> Value {
        serde_yaml::from_str(s).unwrap()
    }

    #[test]
    fn merge_patch_follows_rfc7386() {
        let mut b = y("a: {b: 1, c: 2}\nl: [1,2]\nz: 9");
        merge_patch(&mut b, &y("a: {b: 5, c: null, d: 7}\nl: [3]\nz: null"));
        assert_eq!(b, y("a: {b: 5, d: 7}\nl: [3]"));
    }

    #[test]
    fn overlay_changes_and_lints_the_result() {
        let r = merge_str(
            BASE,
            &["requirements:\n  memory: 1Gi\nmetadata:\n  labels:\n    team: null\n    env: prod\n".into()],
            &[],
        )
        .unwrap();
        let v = y(&r.spec);
        assert_eq!(v["requirements"]["memory"], y("1Gi"));
        assert_eq!(v["metadata"]["labels"]["env"], y("prod"));
        assert!(v["metadata"]["labels"].get("team").is_none());
        assert_eq!(v["metadata"]["labels"]["tier"], y("web"));
        assert_eq!(r.lint.errors, 0, "{:?}", r.lint.diagnostics);
    }

    #[test]
    fn bad_overlay_is_reported_by_lint() {
        let r = merge_str(
            BASE,
            &["network:\n  ports:\n    - containerPort: 0\n      servicePort: 1\n".into()],
            &[],
        )
        .unwrap();
        assert!(r.lint.diagnostics.iter().any(|d| d.code == "E032"));
    }

    #[test]
    fn set_scalars_lists_and_delete() {
        let mut d = y(BASE);
        apply_set(&mut d, "requirements.memory=2Gi").unwrap();
        apply_set(&mut d, "network.ports[0].servicePort=8080").unwrap();
        apply_set(&mut d, "network.ports[1].containerPort=9090").unwrap();
        apply_set(&mut d, "metadata.labels.tier=null").unwrap();
        apply_set(&mut d, "metadata.name=\"123\"").unwrap();
        assert_eq!(d["requirements"]["memory"], y("2Gi"));
        assert_eq!(d["network"]["ports"][0]["servicePort"], y("8080"));
        assert_eq!(d["network"]["ports"][1]["containerPort"], y("9090"));
        assert!(d["metadata"]["labels"].get("tier").is_none());
        assert_eq!(d["metadata"]["name"], Value::String("123".into()));
    }

    #[test]
    fn set_rejects_bad_paths_and_values() {
        let mut d = y(BASE);
        for bad in [
            "noequals",
            ".a=1",
            "a..b=1",
            "a[x]=1",
            "a[1=1",
            "network.ports[5].x=1",
            "metadata.name.deeper=1",
            "network[0]=1",
        ] {
            assert!(apply_set(&mut d, bad).is_err(), "{bad}");
        }
        let deep = format!("{}=1", vec!["a"; MAX_SET_DEPTH + 1].join("."));
        assert!(apply_set(&mut d, &deep).is_err());
    }

    #[test]
    fn set_value_cannot_smuggle_structure() {
        let mut d = y(BASE);
        apply_set(&mut d, "metadata.name={evil: 1}").unwrap();
        assert_eq!(d["metadata"]["name"], Value::String("{evil: 1}".into()));
    }

    #[test]
    fn limits_and_hostile_input() {
        assert!(merge_str(BASE, &vec![String::new(); MAX_OVERLAYS + 1], &[]).is_err());
        assert!(merge_str(BASE, &[], &vec!["a=1".to_string(); MAX_SETS + 1]).is_err());
        assert!(merge_str("- a", &[], &[]).is_err());
        let bomb = "a: &a [1,2]\n".to_string() + &"b: *a\n".repeat(40);
        assert!(merge_str(BASE, &[bomb], &[])
            .unwrap_err()
            .contains("overlay 1"));
        assert!(merge_str(BASE, &["scalar".into()], &[]).is_err());
    }
}
