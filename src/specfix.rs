// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Safe auto-fixes for lint findings (`aether lint --fix`).
//!
//! Only edits that cannot change what the engine does are applied:
//!
//! * `W020` with a "did you mean" hint: the misspelt key is renamed to the
//!   suggested key (the misspelt key was ignored, so the spec now says what the
//!   author meant; this can change behaviour, so it is reported as such).
//! * `I021`: a snake_case alias is renamed to the canonical camelCase key
//!   (accepted spellings are equivalent, so behaviour is identical).
//!
//! Edits are textual and line-based so comments and layout survive. After
//! editing, the result is linted again; if it has more errors than before,
//! nothing is applied. Opinionated findings (best-practice `B1xx`) are never
//! auto-fixed.

use crate::speclint::{self, LintOptions};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Fix {
    pub code: String,
    pub line: usize,
    pub from: String,
    pub to: String,
    /// True when the old key was silently ignored, so the fix changes behaviour.
    pub behaviour_change: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FixResult {
    pub fixed: String,
    pub applied: Vec<Fix>,
    /// Findings that have a suggestion but could not be edited safely.
    pub skipped: Vec<String>,
    pub changed: bool,
}

fn between_backticks(s: &str) -> Option<String> {
    let a = s.find('`')? + 1;
    let b = s[a..].find('`')? + a;
    Some(s[a..b].to_string())
}

/// Rename `from` to `to` where `from` is the mapping key on `line` (1-based).
fn rename_key_on_line(lines: &mut [String], line: usize, from: &str, to: &str) -> bool {
    let Some(l) = lines.get_mut(line.wrapping_sub(1)) else {
        return false;
    };
    let trimmed = l.trim_start();
    let indent = l.len() - trimmed.len();
    // A key may follow one or more `- ` sequence markers.
    let _ = indent;
    let mut rest = trimmed;
    while let Some(r) = rest.strip_prefix("- ") {
        rest = r.trim_start();
    }
    let off = l.len() - rest.len();
    for quote in ["", "\"", "'"] {
        let pat = format!("{quote}{from}{quote}");
        if let Some(after) = rest.strip_prefix(&pat) {
            if after.trim_start().starts_with(':') {
                l.replace_range(off..off + pat.len(), to);
                return true;
            }
        }
    }
    false
}

/// Compute fixes for `src`. Never fails: an unparsable spec yields no fixes.
pub fn fix_str(src: &str) -> FixResult {
    let before = speclint::lint_with(src, &LintOptions::default());
    let mut lines: Vec<String> = src.split('\n').map(str::to_string).collect();
    let mut applied = Vec::new();
    let mut skipped = Vec::new();
    // Apply from the bottom so earlier line numbers stay valid (renames keep
    // the line count, but this keeps the report order deterministic anyway).
    let mut diags: Vec<_> = before
        .diagnostics
        .iter()
        .filter(|d| d.code == "W020" || d.code == "I021")
        .collect();
    diags.sort_by_key(|d| d.line);
    for d in diags {
        let Some(from) = between_backticks(&d.message) else {
            continue;
        };
        let Some(hint) = d.hint.as_deref() else {
            skipped.push(format!("line {}: `{}` has no suggestion", d.line, from));
            continue;
        };
        let Some(to) = between_backticks(hint) else {
            continue;
        };
        if rename_key_on_line(&mut lines, d.line, &from, &to) {
            applied.push(Fix {
                code: d.code.to_string(),
                line: d.line,
                from,
                to,
                behaviour_change: d.code == "W020",
            });
        } else {
            skipped.push(format!(
                "line {}: could not edit `{}` safely (flow style or multi-line key)",
                d.line, from
            ));
        }
    }
    let candidate = lines.join("\n");
    if applied.is_empty() {
        return FixResult {
            fixed: src.to_string(),
            applied,
            skipped,
            changed: false,
        };
    }
    let after = speclint::lint_with(&candidate, &LintOptions::default());
    if after.errors > before.errors {
        skipped.push("fixes rejected: the edited spec would have more errors".into());
        return FixResult {
            fixed: src.to_string(),
            applied: Vec::new(),
            skipped,
            changed: false,
        };
    }
    FixResult {
        fixed: candidate,
        applied,
        skipped,
        changed: true,
    }
}

/// A minimal unified-style diff of changed lines (renames never move lines).
pub fn diff(old: &str, new: &str, file: &str) -> String {
    let mut out = String::new();
    for (i, (a, b)) in old.split('\n').zip(new.split('\n')).enumerate() {
        if a != b {
            out.push_str(&format!("--- {file}:{n}\n-{a}\n+{b}\n", n = i + 1));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "apiVersion: aether/v1\nkind: Workload\nmetadata:\n  name: web\n  owner: me\n  project: p\nbuild:\n  context: .\n  dockerfile: Dockerfile\n  registry: localhost\nrequirements:\n  cpu: \"500m\"\n  memory: 256Mi\n  storage: 1Gi\nruntime:\n  preferred: kube\n  allow:\n    - kube\nnetwork:\n  service: true\n  ports:\n    - containerPort: 80\n      servicePort: 80\n";

    #[test]
    fn clean_spec_is_untouched() {
        let r = fix_str(GOOD);
        assert!(!r.changed && r.applied.is_empty());
        assert_eq!(r.fixed, GOOD);
    }

    #[test]
    fn typo_is_renamed_and_comments_survive() {
        let src = GOOD.replace("service: true", "servicetype: NodePort # keep me");
        let r = fix_str(&src);
        assert!(r.changed, "{:?}", r);
        assert_eq!(r.applied[0].code, "W020");
        assert!(r.applied[0].behaviour_change);
        assert!(r.fixed.contains("serviceType: NodePort # keep me"));
        let after = speclint::lint_str(&r.fixed);
        assert_eq!(after.warnings, 0, "{:?}", after.diagnostics);
    }

    #[test]
    fn snake_case_alias_becomes_canonical() {
        let src = GOOD.replace("service: true", "service: true\n  service_type: NodePort");
        let r = fix_str(&src);
        assert_eq!(r.applied.len(), 1, "{:?}", r);
        assert_eq!(r.applied[0].code, "I021");
        assert!(!r.applied[0].behaviour_change);
        assert!(r.fixed.contains("  serviceType: NodePort"));
    }

    #[test]
    fn fix_is_idempotent() {
        let src = GOOD.replace("service: true", "servicetype: NodePort");
        let once = fix_str(&src);
        let twice = fix_str(&once.fixed);
        assert!(!twice.changed);
        assert_eq!(once.fixed, twice.fixed);
    }

    #[test]
    fn unparsable_spec_has_no_fixes() {
        let r = fix_str("- a\n- b\n");
        assert!(!r.changed);
    }

    #[test]
    fn diff_lists_changed_lines_only() {
        let d = diff("a: 1\nb: 2\n", "a: 1\nc: 2\n", "s.yaml");
        assert!(d.contains("-b: 2") && d.contains("+c: 2") && !d.contains("a: 1"));
    }
}
