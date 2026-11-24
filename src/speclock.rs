// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Spec lock file and drift check (`aether spec-lock write|check`).
//!
//! A lock records the canonical content hash of a spec, overall and per
//! top-level section (see `spec-render`). `check` re-renders the spec and says
//! which sections drifted from the recorded baseline, so CI can fail a change
//! that was not meant to touch, say, `network` or `security`. It is offline:
//! the baseline is a file in the repository, not live cluster state (use
//! `aether diff` for that).
//!
//! The lock contains hashes only, never spec values, so it is safe to commit
//! even when the spec holds secrets.

use crate::specrender;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const LOCK_VERSION: u32 = 1;
const MAX_LOCK_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Lock {
    pub version: u32,
    pub hash: String,
    pub sections: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LockCheck {
    pub drift: bool,
    pub expected: String,
    pub actual: String,
    pub changed: Vec<String>,
    pub added: Vec<String>,
    pub removed: Vec<String>,
    /// Sections excluded from the comparison with `--ignore`.
    pub ignored: Vec<String>,
}

impl LockCheck {
    pub fn render_text(&self) -> String {
        if !self.drift {
            return format!("no drift ({})\n", self.actual);
        }
        let mut out = format!("drift: {} -> {}\n", self.expected, self.actual);
        for (label, list) in [
            ("changed", &self.changed),
            ("added", &self.added),
            ("removed", &self.removed),
        ] {
            for s in list {
                out.push_str(&format!("  {label:<8} {}\n", crate::speclint::sanitize(s)));
            }
        }
        out
    }
}

fn valid_hash(h: &str) -> bool {
    h.strip_prefix("sha256:")
        .map(|x| x.len() == 64 && x.chars().all(|c| c.is_ascii_hexdigit()))
        .unwrap_or(false)
}

/// Build a lock from spec text.
pub fn make_lock(spec_src: &str) -> Result<Lock, String> {
    let r = specrender::render_str(spec_src)?;
    Ok(Lock {
        version: LOCK_VERSION,
        hash: r.hash,
        sections: r.sections,
    })
}

/// Lock file text: stable, pretty JSON with a trailing newline.
pub fn lock_text(lock: &Lock) -> String {
    let mut s = serde_json::to_string_pretty(lock).unwrap_or_default();
    s.push('\n');
    s
}

pub fn parse_lock(src: &str) -> Result<Lock, String> {
    if src.len() > MAX_LOCK_BYTES {
        return Err("lock file is too large".into());
    }
    let lock: Lock = serde_json::from_str(src).map_err(|e| format!("invalid lock file: {e}"))?;
    if lock.version != LOCK_VERSION {
        return Err(format!(
            "unsupported lock version {} (this build reads version {LOCK_VERSION})",
            lock.version
        ));
    }
    if !valid_hash(&lock.hash) || lock.sections.values().any(|h| !valid_hash(h)) {
        return Err("invalid lock file: malformed hash".into());
    }
    Ok(lock)
}

/// Compare a spec with a lock. Sections named in `ignore` are not compared;
/// with any ignored section the overall hash is not used for the verdict.
pub fn check(spec_src: &str, lock_src: &str, ignore: &[String]) -> Result<LockCheck, String> {
    let lock = parse_lock(lock_src)?;
    let now = make_lock(spec_src)?;
    let skip = |s: &str| ignore.iter().any(|i| i == s);
    let mut changed = Vec::new();
    let mut added = Vec::new();
    let mut removed = Vec::new();
    for (k, h) in &now.sections {
        if skip(k) {
            continue;
        }
        match lock.sections.get(k) {
            None => added.push(k.clone()),
            Some(old) if old != h => changed.push(k.clone()),
            _ => {}
        }
    }
    for k in lock.sections.keys() {
        if !skip(k) && !now.sections.contains_key(k) {
            removed.push(k.clone());
        }
    }
    let drift = if ignore.is_empty() {
        lock.hash != now.hash || !(changed.is_empty() && added.is_empty() && removed.is_empty())
    } else {
        !(changed.is_empty() && added.is_empty() && removed.is_empty())
    };
    Ok(LockCheck {
        drift,
        expected: lock.hash,
        actual: now.hash,
        changed,
        added,
        removed,
        ignored: ignore.to_vec(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "apiVersion: aether/v1\nkind: Workload\nmetadata:\n  name: web\n  owner: me\n  project: p\nbuild:\n  context: .\n  dockerfile: Dockerfile\n  registry: localhost\nrequirements:\n  cpu: \"500m\"\n  memory: 256Mi\n  storage: 1Gi\nruntime:\n  preferred: kube\n  allow:\n    - kube\nnetwork:\n  service: true\n  ports:\n    - containerPort: 80\n      servicePort: 80\n";

    #[test]
    fn unchanged_spec_has_no_drift_even_when_reformatted() {
        let lock = lock_text(&make_lock(GOOD).unwrap());
        let reformatted = format!(
            "# edited comment\n{}",
            GOOD.replace("  owner: me\n  project: p\n", "  project: p\n  owner: me\n")
        );
        let c = check(&reformatted, &lock, &[]).unwrap();
        assert!(!c.drift, "{:?}", c);
    }

    #[test]
    fn drift_names_the_section() {
        let lock = lock_text(&make_lock(GOOD).unwrap());
        let c = check(&GOOD.replace("256Mi", "512Mi"), &lock, &[]).unwrap();
        assert!(c.drift);
        assert_eq!(c.changed, vec!["requirements"]);
        assert!(c.render_text().contains("changed  requirements"));
    }

    #[test]
    fn added_and_removed_sections() {
        let lock = lock_text(&make_lock(GOOD).unwrap());
        let with = format!("{GOOD}scaling:\n  enabled: true\n  minReplicas: 1\n  maxReplicas: 3\n  cpuTarget: 70\n");
        let c = check(&with, &lock, &[]).unwrap();
        assert!(
            c.drift && c.added.contains(&"scaling".to_string()),
            "{:?}",
            c
        );
        let lock2 = lock_text(&make_lock(&with).unwrap());
        let c = check(GOOD, &lock2, &[]).unwrap();
        assert!(c.removed.contains(&"scaling".to_string()), "{:?}", c);
    }

    #[test]
    fn ignored_sections_do_not_count() {
        let lock = lock_text(&make_lock(GOOD).unwrap());
        let c = check(
            &GOOD.replace("256Mi", "512Mi"),
            &lock,
            &["requirements".into()],
        )
        .unwrap();
        assert!(!c.drift);
        assert_eq!(c.ignored, vec!["requirements"]);
    }

    #[test]
    fn lock_contains_no_spec_values() {
        let spec = format!(
            "{GOOD}config:\n  secrets:\n    - name: db\n      data:\n        password: hunter2\n"
        );
        let text = lock_text(&make_lock(&spec).unwrap());
        assert!(!text.contains("hunter2") && !text.contains("256Mi"));
    }

    #[test]
    fn bad_locks_are_rejected() {
        for bad in [
            "not json",
            "{\"version\":2,\"hash\":\"sha256:00\",\"sections\":{}}",
            "{\"version\":1,\"hash\":\"nope\",\"sections\":{}}",
            "{\"version\":1,\"hash\":\"sha256:00\",\"sections\":{},\"extra\":1}",
        ] {
            assert!(check(GOOD, bad, &[]).is_err(), "{bad}");
        }
    }

    #[test]
    fn lock_text_is_deterministic() {
        assert_eq!(
            lock_text(&make_lock(GOOD).unwrap()),
            lock_text(&make_lock(GOOD).unwrap())
        );
    }
}
