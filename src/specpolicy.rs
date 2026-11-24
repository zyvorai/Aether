// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Policy-as-code for specs (`aether spec-policy`).
//!
//! Organisation rules live in a YAML rule file, outside the binary, and are
//! evaluated against the spec as written (so every finding carries a source
//! line). Think Conftest / Kubescape controls, but declarative and offline: no
//! Rego runtime, only a small closed set of operators.
//!
//! ```yaml
//! rules:
//!   - id: P001
//!     description: every workload names an owner
//!     path: metadata.owner
//!     op: exists
//!   - id: P002
//!     severity: warning
//!     path: network.ports[*].containerPort
//!     op: gte
//!     value: 1024
//!     message: unprivileged ports only
//!   - id: P003
//!     path: requirements.memory
//!     op: lte
//!     value: 8Gi
//!     when: { path: runtime.preferred, op: equals, value: kube }
//! ```
//!
//! A rule passes when its assertion holds on every node the path selects.
//! `[*]` (or a `*` segment) selects every element of a list. A missing value
//! fails every operator except `absent`, `not_equals`, `not_in` and
//! `not_matches`, unless `skip_if_missing: true`.

use crate::lintfmt::xml_escape;
use crate::resources::parse_cpu;
use crate::speclint::{self, sanitize, LineIndex};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;

pub const MAX_RULE_BYTES: usize = 256 * 1024;
pub const MAX_RULES: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PolicySeverity {
    #[default]
    Error,
    Warning,
    Info,
}

impl PolicySeverity {
    fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Exists,
    Absent,
    Equals,
    NotEquals,
    In,
    NotIn,
    /// Glob match (`*` and `?`) on a string.
    Matches,
    NotMatches,
    Lt,
    Lte,
    Gt,
    Gte,
    /// Minimum string length or list length.
    MinLen,
    MaxLen,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Condition {
    pub path: String,
    pub op: Op,
    #[serde(default)]
    pub value: Option<Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub severity: PolicySeverity,
    pub path: String,
    pub op: Op,
    #[serde(default)]
    pub value: Option<Value>,
    #[serde(default)]
    pub when: Option<Condition>,
    #[serde(default)]
    pub skip_if_missing: bool,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleFile {
    rules: Vec<Rule>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Violation {
    pub id: String,
    pub severity: PolicySeverity,
    pub path: String,
    pub line: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PolicyReport {
    pub rules: usize,
    pub passed: usize,
    pub violations: Vec<Violation>,
    pub errors: usize,
    pub warnings: usize,
}

impl PolicyReport {
    pub fn failed(&self, deny_warnings: bool) -> bool {
        self.errors > 0 || (deny_warnings && self.warnings > 0)
    }

    pub fn render_text(&self, file: &str) -> String {
        let mut out = String::new();
        for v in &self.violations {
            let loc = if v.line > 0 {
                format!("{file}:{}", v.line)
            } else {
                file.to_string()
            };
            out.push_str(&format!(
                "{loc}: {}[{}]: {} ({})\n",
                v.severity.as_str(),
                sanitize(&v.id),
                sanitize(&v.message),
                sanitize(&v.path)
            ));
        }
        out.push_str(&format!(
            "{} rule(s), {} passed, {} error(s), {} warning(s)\n",
            self.rules, self.passed, self.errors, self.warnings
        ));
        out
    }

    pub fn render_github(&self, file: &str) -> String {
        let esc = |s: &str| {
            sanitize(s)
                .replace('%', "%25")
                .replace('\r', "%0D")
                .replace('\n', "%0A")
        };
        let mut out = String::new();
        for v in &self.violations {
            let level = match v.severity {
                PolicySeverity::Error => "error",
                PolicySeverity::Warning => "warning",
                PolicySeverity::Info => "notice",
            };
            out.push_str(&format!(
                "::{level} file={},line={},title={}::{}\n",
                esc(file).replace(',', "%2C").replace(':', "%3A"),
                v.line.max(1),
                esc(&v.id).replace(',', "%2C").replace(':', "%3A"),
                esc(&v.message)
            ));
        }
        out
    }

    pub fn render_junit(&self, file: &str) -> String {
        let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(&format!(
            "<testsuite name=\"aether-spec-policy\" tests=\"{}\" failures=\"{}\">\n",
            self.rules.max(self.violations.len()),
            self.violations.len()
        ));
        for v in &self.violations {
            out.push_str(&format!(
                "  <testcase classname=\"{}\" name=\"{}\">\n    <failure type=\"{}\" message=\"{}\">{}:{}</failure>\n  </testcase>\n",
                xml_escape(file),
                xml_escape(&format!("{} {}", v.id, v.path)),
                v.severity.as_str(),
                xml_escape(&sanitize(&v.message)),
                xml_escape(file),
                v.line
            ));
        }
        out.push_str("</testsuite>\n");
        out
    }
}

// ---------------------------------------------------------------------------
// Path resolution
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Key(String),
    Index(usize),
    Wild,
}

fn tokenize(path: &str) -> Result<Vec<Tok>, String> {
    if path.is_empty() {
        return Err("empty path".into());
    }
    let mut toks = Vec::new();
    for seg in path.split('.') {
        let (name, mut rest) = match seg.find('[') {
            Some(i) => (&seg[..i], &seg[i..]),
            None => (seg, ""),
        };
        if name == "*" {
            toks.push(Tok::Wild);
        } else if name.is_empty() && rest.is_empty() {
            return Err(format!("empty segment in path `{path}`"));
        } else if !name.is_empty() {
            toks.push(Tok::Key(name.to_string()));
        }
        while let Some(r) = rest.strip_prefix('[') {
            let end = r
                .find(']')
                .ok_or_else(|| format!("unclosed `[` in path `{path}`"))?;
            let inner = &r[..end];
            if inner == "*" {
                toks.push(Tok::Wild);
            } else {
                toks.push(Tok::Index(
                    inner
                        .parse()
                        .map_err(|_| format!("bad index `{inner}` in path `{path}`"))?,
                ));
            }
            rest = &r[end + 1..];
        }
        if !rest.is_empty() {
            return Err(format!("unexpected `{rest}` in path `{path}`"));
        }
    }
    Ok(toks)
}

type Node<'a> = (String, Option<&'a Value>);

fn resolve<'a>(root: &'a Value, toks: &[Tok]) -> Vec<Node<'a>> {
    let mut cur: Vec<Node<'a>> = vec![(String::new(), Some(root))];
    for t in toks {
        let mut next = Vec::new();
        for (p, v) in cur {
            match t {
                Tok::Key(k) => {
                    let child = v.and_then(|v| v.get(k.as_str()));
                    let np = if p.is_empty() {
                        k.clone()
                    } else {
                        format!("{p}.{k}")
                    };
                    next.push((np, child.filter(|c| !c.is_null())));
                }
                Tok::Index(i) => {
                    let child = v.and_then(|v| v.get(*i));
                    next.push((format!("{p}[{i}]"), child));
                }
                Tok::Wild => match v {
                    Some(Value::Array(a)) => {
                        for (i, c) in a.iter().enumerate() {
                            next.push((format!("{p}[{i}]"), Some(c)));
                        }
                    }
                    Some(Value::Object(m)) => {
                        for (k, c) in m {
                            let np = if p.is_empty() {
                                k.clone()
                            } else {
                                format!("{p}.{k}")
                            };
                            next.push((np, Some(c)));
                        }
                    }
                    _ => {}
                },
            }
        }
        cur = next;
    }
    cur
}

// ---------------------------------------------------------------------------
// Operators
// ---------------------------------------------------------------------------

/// Parse a number or a Kubernetes quantity (`500m`, `256Mi`, `2Gi`) to f64.
pub fn quantity(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let s = s.trim();
            if let Ok(n) = s.parse::<f64>() {
                return Some(n);
            }
            const BIN: [(&str, f64); 4] = [
                ("Ki", 1024.0),
                ("Mi", 1048576.0),
                ("Gi", 1073741824.0),
                ("Ti", 1099511627776.0),
            ];
            for (suf, mul) in BIN {
                if let Some(n) = s.strip_suffix(suf) {
                    return n.parse::<f64>().ok().map(|n| n * mul);
                }
            }
            for (suf, mul) in [("k", 1e3), ("K", 1e3), ("M", 1e6), ("G", 1e9), ("T", 1e12)] {
                if let Some(n) = s.strip_suffix(suf) {
                    return n.parse::<f64>().ok().map(|n| n * mul);
                }
            }
            if s.ends_with('m') {
                return Some(parse_cpu(s));
            }
            None
        }
        _ => None,
    }
}

/// `*` matches any run, `?` any one character.
pub fn glob(pat: &str, text: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti, mut star, mut mark) = (0, 0, None, 0);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

fn scalar_eq(a: &Value, b: &Value) -> bool {
    if let (Some(x), Some(y)) = (quantity_if_numeric(a), quantity_if_numeric(b)) {
        return x == y;
    }
    a == b
}

fn quantity_if_numeric(v: &Value) -> Option<f64> {
    v.as_f64()
}

fn len_of(v: &Value) -> Option<usize> {
    match v {
        Value::String(s) => Some(s.chars().count()),
        Value::Array(a) => Some(a.len()),
        Value::Object(m) => Some(m.len()),
        _ => None,
    }
}

/// True when the assertion holds. `Err` means the operand has the wrong type.
fn holds(op: Op, v: Option<&Value>, want: Option<&Value>) -> Result<bool, String> {
    let missing_ok = matches!(op, Op::Absent | Op::NotEquals | Op::NotIn | Op::NotMatches);
    let Some(v) = v else {
        return Ok(missing_ok);
    };
    let need = || want.ok_or_else(|| "rule has no `value`".to_string());
    Ok(match op {
        Op::Exists => true,
        Op::Absent => false,
        Op::Equals => scalar_eq(v, need()?),
        Op::NotEquals => !scalar_eq(v, need()?),
        Op::In | Op::NotIn => {
            let list = need()?
                .as_array()
                .ok_or_else(|| "`value` must be a list".to_string())?;
            let found = list.iter().any(|w| scalar_eq(v, w));
            if op == Op::In {
                found
            } else {
                !found
            }
        }
        Op::Matches | Op::NotMatches => {
            let pat = need()?
                .as_str()
                .ok_or_else(|| "`value` must be a string".to_string())?;
            let s = v
                .as_str()
                .ok_or_else(|| "value is not a string".to_string())?;
            let m = glob(pat, s);
            if op == Op::Matches {
                m
            } else {
                !m
            }
        }
        Op::Lt | Op::Lte | Op::Gt | Op::Gte => {
            let a = quantity(v).ok_or_else(|| "value is not a number or quantity".to_string())?;
            let b = quantity(need()?)
                .ok_or_else(|| "`value` is not a number or quantity".to_string())?;
            match op {
                Op::Lt => a < b,
                Op::Lte => a <= b,
                Op::Gt => a > b,
                _ => a >= b,
            }
        }
        Op::MinLen | Op::MaxLen => {
            let n = len_of(v).ok_or_else(|| "value has no length".to_string())? as f64;
            let b = need()?
                .as_f64()
                .ok_or_else(|| "`value` must be a number".to_string())?;
            if op == Op::MinLen {
                n >= b
            } else {
                n <= b
            }
        }
    })
}

fn needs_value(op: Op) -> bool {
    !matches!(op, Op::Exists | Op::Absent)
}

// ---------------------------------------------------------------------------
// Loading and evaluation
// ---------------------------------------------------------------------------

/// Parse and validate a rule file.
pub fn parse_rules(src: &str) -> Result<Vec<Rule>, String> {
    if src.len() > MAX_RULE_BYTES {
        return Err(format!("rule file exceeds {MAX_RULE_BYTES} bytes"));
    }
    speclint::guarded_parse(src).map_err(|d| d.message)?;
    let file: RuleFile =
        serde_yaml::from_str(src).map_err(|e| format!("invalid rule file: {e}"))?;
    if file.rules.len() > MAX_RULES {
        return Err(format!("more than {MAX_RULES} rules"));
    }
    let mut seen = BTreeSet::new();
    for r in &file.rules {
        if r.id.trim().is_empty() || r.id.chars().any(|c| c.is_control()) {
            return Err("rule id must be non-empty and free of control characters".into());
        }
        if !seen.insert(r.id.clone()) {
            return Err(format!("duplicate rule id `{}`", sanitize(&r.id)));
        }
        tokenize(&r.path).map_err(|e| format!("rule {}: {e}", sanitize(&r.id)))?;
        if needs_value(r.op) && r.value.is_none() {
            return Err(format!(
                "rule {}: operator {:?} needs a `value`",
                sanitize(&r.id),
                r.op
            ));
        }
        if let Some(w) = &r.when {
            tokenize(&w.path).map_err(|e| format!("rule {} (when): {e}", sanitize(&r.id)))?;
            if needs_value(w.op) && w.value.is_none() {
                return Err(format!("rule {} (when): needs a `value`", sanitize(&r.id)));
            }
        }
    }
    Ok(file.rules)
}

/// Evaluate rules against the spec text.
pub fn evaluate(spec_src: &str, rules_src: &str) -> Result<PolicyReport, String> {
    let rules = parse_rules(rules_src)?;
    let yaml = speclint::guarded_parse(spec_src).map_err(|d| d.message)?;
    let doc: Value = serde_json::to_value(&yaml).map_err(|e| format!("unsupported spec: {e}"))?;
    if !doc.is_object() {
        return Err("the spec must be a YAML mapping".into());
    }
    let index = LineIndex::build(spec_src);
    let mut violations = Vec::new();
    let mut passed = 0usize;
    for r in &rules {
        if let Some(w) = &r.when {
            let toks = tokenize(&w.path)?;
            let nodes = resolve(&doc, &toks);
            let active = nodes
                .iter()
                .all(|(_, v)| holds(w.op, *v, w.value.as_ref()).unwrap_or(false));
            if nodes.is_empty() || !active {
                passed += 1;
                continue;
            }
        }
        let toks = tokenize(&r.path)?;
        let before = violations.len();
        for (path, v) in resolve(&doc, &toks) {
            if v.is_none() && r.skip_if_missing {
                continue;
            }
            let (ok, why) = match holds(r.op, v, r.value.as_ref()) {
                Ok(b) => (b, None),
                Err(e) => (false, Some(e)),
            };
            if ok {
                continue;
            }
            let base = r
                .message
                .clone()
                .or_else(|| r.description.clone())
                .unwrap_or_else(|| format!("`{}` violates {:?}", r.path, r.op));
            let detail = match (&why, v) {
                (Some(w), _) => format!(": {w}"),
                (None, None) => ": value is missing".to_string(),
                (None, Some(val)) => format!(": found {}", short(val)),
            };
            violations.push(Violation {
                id: r.id.clone(),
                severity: r.severity,
                line: index.line_of(&path),
                path,
                message: format!("{base}{detail}"),
            });
        }
        if violations.len() == before {
            passed += 1;
        }
    }
    violations.sort_by(|a, b| (a.line == 0, a.line, &a.id).cmp(&(b.line == 0, b.line, &b.id)));
    let errors = violations
        .iter()
        .filter(|v| v.severity == PolicySeverity::Error)
        .count();
    let warnings = violations
        .iter()
        .filter(|v| v.severity == PolicySeverity::Warning)
        .count();
    Ok(PolicyReport {
        rules: rules.len(),
        passed,
        violations,
        errors,
        warnings,
    })
}

fn short(v: &Value) -> String {
    let s = v.to_string();
    if s.chars().count() > 60 {
        format!("{}...", s.chars().take(57).collect::<String>())
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SPEC: &str = "apiVersion: aether/v1\nkind: Workload\nmetadata:\n  name: web\n  owner: me\nrequirements:\n  cpu: \"500m\"\n  memory: 256Mi\nruntime:\n  preferred: kube\nbuild:\n  registry: registry.corp/team\nnetwork:\n  ports:\n    - containerPort: 80\n    - containerPort: 8080\n";

    fn run(rules: &str) -> PolicyReport {
        evaluate(SPEC, rules).unwrap()
    }

    #[test]
    fn exists_and_absent() {
        let r = run("rules:\n  - {id: A, path: metadata.owner, op: exists}\n  - {id: B, path: metadata.team, op: exists}\n  - {id: C, path: metadata.team, op: absent}\n");
        assert_eq!(r.violations.len(), 1);
        assert_eq!(r.violations[0].id, "B");
        assert_eq!(r.passed, 2);
        assert!(r.violations[0].line > 0);
        assert!(r.failed(false));
    }

    #[test]
    fn wildcard_over_lists_reports_each_bad_element() {
        let r = run("rules:\n  - id: P\n    severity: warning\n    path: network.ports[*].containerPort\n    op: gte\n    value: 1024\n");
        assert_eq!(r.violations.len(), 1);
        assert_eq!(r.violations[0].path, "network.ports[0].containerPort");
        assert_eq!(r.warnings, 1);
        assert!(!r.failed(false) && r.failed(true));
    }

    #[test]
    fn quantities_compare_across_units() {
        let ok = run("rules:\n  - {id: M, path: requirements.memory, op: lte, value: 1Gi}\n  - {id: C, path: requirements.cpu, op: lte, value: 1}\n");
        assert!(ok.violations.is_empty(), "{:?}", ok.violations);
        let bad = run("rules:\n  - {id: M, path: requirements.memory, op: lte, value: 128Mi}\n");
        assert_eq!(bad.violations.len(), 1);
        assert!(bad.violations[0].message.contains("256Mi"));
    }

    #[test]
    fn glob_in_and_equals() {
        let r = run("rules:\n  - {id: G, path: build.registry, op: matches, value: 'registry.corp/*'}\n  - {id: I, path: runtime.preferred, op: in, value: [kube, podman]}\n  - {id: N, path: runtime.preferred, op: not_equals, value: kubevirt}\n  - {id: X, path: build.registry, op: matches, value: 'docker.io/*', message: only docker hub}\n");
        assert_eq!(r.violations.len(), 1);
        assert!(r.violations[0].message.starts_with("only docker hub"));
    }

    #[test]
    fn when_clause_scopes_a_rule() {
        let r = run("rules:\n  - id: W\n    path: requirements.memory\n    op: lte\n    value: 1Mi\n    when: {path: runtime.preferred, op: equals, value: podman}\n");
        assert!(r.violations.is_empty());
        assert_eq!(r.passed, 1);
        let r = run("rules:\n  - id: W\n    path: requirements.memory\n    op: lte\n    value: 1Mi\n    when: {path: runtime.preferred, op: equals, value: kube}\n");
        assert_eq!(r.violations.len(), 1);
    }

    #[test]
    fn missing_value_fails_unless_skipped() {
        let r = run("rules:\n  - {id: A, path: requirements.gpu, op: lte, value: 2}\n");
        assert_eq!(r.violations.len(), 1);
        assert!(r.violations[0].message.contains("missing"));
        let r = run("rules:\n  - {id: A, path: requirements.gpu, op: lte, value: 2, skip_if_missing: true}\n");
        assert!(r.violations.is_empty());
    }

    #[test]
    fn type_mismatch_is_a_violation_not_a_panic() {
        let r = run("rules:\n  - {id: T, path: metadata.name, op: gt, value: 3}\n");
        assert_eq!(r.violations.len(), 1);
        assert!(r.violations[0].message.contains("not a number"));
    }

    #[test]
    fn bad_rule_files_are_rejected() {
        for (src, needle) in [
            (
                "rules:\n  - {id: A, path: x, op: bogus}\n",
                "invalid rule file",
            ),
            (
                "rules:\n  - {id: A, path: x, op: exists, extra: 1}\n",
                "invalid rule file",
            ),
            (
                "rules:\n  - {id: A, path: x, op: exists}\n  - {id: A, path: y, op: exists}\n",
                "duplicate",
            ),
            (
                "rules:\n  - {id: A, path: x, op: equals}\n",
                "needs a `value`",
            ),
            ("rules:\n  - {id: A, path: 'a[', op: exists}\n", "unclosed"),
            ("rules:\n  - {id: A, path: '', op: exists}\n", "empty path"),
        ] {
            let e = parse_rules(src).unwrap_err();
            assert!(e.contains(needle), "{src} -> {e}");
        }
    }

    #[test]
    fn output_formats_escape_and_carry_lines() {
        let r = run(
            "rules:\n  - {id: 'A<1>', path: metadata.team, op: exists, message: 'needs <team>'}\n",
        );
        let x = r.render_junit("s&.yaml");
        assert!(
            x.contains("&lt;team&gt;") && x.contains("s&amp;.yaml") && x.contains("failures=\"1\"")
        );
        let g = r.render_github("s.yaml");
        assert!(g.starts_with("::error file=s.yaml,line="));
        assert!(r
            .render_text("s.yaml")
            .contains("1 rule(s), 0 passed, 1 error(s)"));
    }

    #[test]
    fn glob_matcher() {
        assert!(glob("a*c", "abbbc") && glob("a?c", "abc") && glob("*", "") && !glob("a?c", "ac"));
        assert!(!glob("a*b", "abc"));
    }
}
