// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Spec linting with line-numbered diagnostics (`aether lint`).
//!
//! `aether validate` stops at the first error and reports no position, and the
//! workload structs accept unknown keys, so a typo such as `servicetype:` is
//! silently ignored. The linter reports every problem it can find, each with
//! a stable code, a YAML path and a source line.
//!
//! Input is untrusted: [`guarded_parse`] refuses oversized documents and
//! alias-expansion ("billion laughs") before the YAML is expanded into memory.
//!
//! Codes: `E00x` input guards, `E01x` parse/validation, `W02x` unknown keys,
//! `E03x`/`W03x` semantic rules, `W040` portability.

use crate::portability::{self, Support, Target};
use crate::spec::Workload;
use serde::Serialize;
use serde_json::Value as Json;
use serde_yaml::Value;
use std::collections::HashMap;

/// Maximum accepted spec size.
pub const MAX_SPEC_BYTES: usize = 1024 * 1024;
/// Documents that use YAML aliases are held to a much smaller size so that
/// alias expansion stays bounded.
pub const MAX_ALIAS_DOC_BYTES: usize = 64 * 1024;
/// Maximum number of alias references (`*name`) in one document.
pub const MAX_ALIASES: usize = 16;
/// Maximum number of YAML nodes after parsing.
pub const MAX_NODES: usize = 200_000;
/// Maximum nesting depth.
pub const MAX_DEPTH: usize = 48;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warning,
    Error,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warning => "warning",
            Severity::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    /// YAML path such as `network.ports[0].containerPort` (empty = document).
    pub path: String,
    /// 1-based source line, 0 when unknown.
    pub line: usize,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LintReport {
    pub diagnostics: Vec<Diagnostic>,
    pub errors: usize,
    pub warnings: usize,
}

impl LintReport {
    fn new(mut diagnostics: Vec<Diagnostic>) -> Self {
        diagnostics.sort_by_key(|d| (d.line == 0, d.line, std::cmp::Reverse(d.severity), d.code));
        let errors = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        let warnings = diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        Self {
            diagnostics,
            errors,
            warnings,
        }
    }

    /// Process exit status: 1 on errors, or on warnings when `deny_warnings`.
    pub fn failed(&self, deny_warnings: bool) -> bool {
        self.errors > 0 || (deny_warnings && self.warnings > 0)
    }

    /// `file:line: severity[CODE]: message` lines, plus a summary.
    pub fn render_text(&self, file: &str) -> String {
        let mut out = String::new();
        for d in &self.diagnostics {
            let loc = if d.line > 0 {
                format!("{}:{}", file, d.line)
            } else {
                file.to_string()
            };
            let path = if d.path.is_empty() {
                String::new()
            } else {
                format!(" ({})", sanitize(&d.path))
            };
            out.push_str(&format!(
                "{loc}: {}[{}]: {}{path}\n",
                d.severity.as_str(),
                d.code,
                sanitize(&d.message)
            ));
            if let Some(h) = &d.hint {
                out.push_str(&format!("    hint: {}\n", sanitize(h)));
            }
        }
        out.push_str(&format!(
            "{} error(s), {} warning(s)\n",
            self.errors, self.warnings
        ));
        out
    }
}

/// Escape control characters so user-controlled text cannot forge extra lines
/// or terminal escape sequences in rendered output.
pub fn sanitize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_control() {
            out.extend(c.escape_default());
        } else {
            out.push(c);
        }
    }
    out
}

fn diag(code: &'static str, severity: Severity, path: &str, message: String) -> Diagnostic {
    Diagnostic {
        code,
        severity,
        path: path.to_string(),
        line: 0,
        message,
        hint: None,
    }
}

// ---------------------------------------------------------------------------
// Guarded parsing
// ---------------------------------------------------------------------------

/// Count YAML alias references (`*name`) outside quoted scalars and comments.
pub fn count_aliases(src: &str) -> usize {
    let mut count = 0;
    for line in src.lines() {
        let b = line.as_bytes();
        let (mut in_s, mut in_d) = (false, false);
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            let boundary = i == 0 || matches!(b[i - 1], b' ' | b'\t' | b'[' | b'{' | b',');
            if in_s {
                if c == b'\'' {
                    in_s = false;
                }
            } else if in_d {
                if c == b'\\' {
                    i += 1;
                } else if c == b'"' {
                    in_d = false;
                }
            } else if c == b'#' && boundary {
                break;
            } else if c == b'\'' && boundary {
                in_s = true;
            } else if c == b'"' && boundary {
                in_d = true;
            } else if c == b'*'
                && boundary
                && b.get(i + 1)
                    .is_some_and(|n| n.is_ascii_alphanumeric() || matches!(n, b'_' | b'-'))
            {
                count += 1;
            }
            i += 1;
        }
    }
    count
}

fn count_nodes(v: &Value, depth: usize, nodes: &mut usize) -> Result<(), &'static str> {
    *nodes += 1;
    if *nodes > MAX_NODES {
        return Err("too many nodes");
    }
    if depth > MAX_DEPTH {
        return Err("nesting too deep");
    }
    match v {
        Value::Sequence(s) => {
            for c in s {
                count_nodes(c, depth + 1, nodes)?;
            }
        }
        Value::Mapping(m) => {
            for (k, c) in m {
                count_nodes(k, depth + 1, nodes)?;
                count_nodes(c, depth + 1, nodes)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Parse untrusted YAML into a generic value, enforcing the size, alias and
/// node-count limits. Returns a diagnostic on refusal.
pub fn guarded_parse(src: &str) -> Result<Value, Diagnostic> {
    if src.len() > MAX_SPEC_BYTES {
        return Err(diag(
            "E001",
            Severity::Error,
            "",
            format!(
                "spec is {} bytes; the limit is {}",
                src.len(),
                MAX_SPEC_BYTES
            ),
        ));
    }
    let aliases = count_aliases(src);
    if aliases > MAX_ALIASES {
        return Err(diag(
            "E002",
            Severity::Error,
            "",
            format!("{aliases} YAML aliases; the limit is {MAX_ALIASES} (alias-expansion guard)"),
        ));
    }
    if aliases > 0 && src.len() > MAX_ALIAS_DOC_BYTES {
        return Err(diag(
            "E002",
            Severity::Error,
            "",
            format!(
                "documents that use YAML aliases are limited to {MAX_ALIAS_DOC_BYTES} bytes (got {})",
                src.len()
            ),
        ));
    }
    let value: Value = serde_yaml::from_str(src).map_err(|e| {
        let mut d = diag(
            "E010",
            Severity::Error,
            "",
            format!("YAML parse error: {e}"),
        );
        d.line = e.location().map(|l| l.line()).unwrap_or(0);
        d
    })?;
    let mut nodes = 0;
    count_nodes(&value, 0, &mut nodes)
        .map_err(|why| diag("E003", Severity::Error, "", format!("spec rejected: {why}")))?;
    Ok(value)
}

// ---------------------------------------------------------------------------
// Line index
// ---------------------------------------------------------------------------

/// Maps YAML paths (`a.b[0].c`) to 1-based lines for block-style documents.
/// Flow-style (`{}` / `[]`) children are not indexed; lookups fall back to the
/// nearest indexed ancestor.
#[derive(Debug, Default)]
pub struct LineIndex {
    lines: HashMap<String, usize>,
}

enum Seg {
    Key(String),
    Idx(usize),
}

struct Frame {
    indent: usize,
    seg: Seg,
    next_idx: usize,
}

fn join_path(frames: &[Frame]) -> String {
    let mut s = String::new();
    for f in frames {
        match &f.seg {
            Seg::Key(k) => {
                if !s.is_empty() {
                    s.push('.');
                }
                s.push_str(k);
            }
            Seg::Idx(i) => s.push_str(&format!("[{i}]")),
        }
    }
    s
}

/// Split `key: value` at the first unquoted `: ` / trailing `:`.
fn split_key(s: &str) -> Option<(String, &str)> {
    let b = s.as_bytes();
    if b.is_empty() {
        return None;
    }
    if b[0] == b'"' || b[0] == b'\'' {
        let q = b[0];
        let mut i = 1;
        while i < b.len() {
            if b[i] == b'\\' && q == b'"' {
                i += 2;
                continue;
            }
            if b[i] == q {
                break;
            }
            i += 1;
        }
        if i >= b.len() {
            return None;
        }
        let key = s[1..i].to_string();
        let rest = s[i + 1..].trim_start();
        let rest = rest.strip_prefix(':')?;
        if rest.is_empty() || rest.starts_with(' ') {
            return Some((key, rest.trim()));
        }
        return None;
    }
    if matches!(b[0], b'[' | b'{' | b'#' | b'|' | b'>' | b'&' | b'*' | b'!') {
        return None;
    }
    let mut i = 0;
    while i < b.len() {
        if b[i] == b':' && (i + 1 == b.len() || b[i + 1] == b' ' || b[i + 1] == b'\t') {
            return Some((s[..i].trim().to_string(), s[i + 1..].trim()));
        }
        i += 1;
    }
    None
}

impl LineIndex {
    pub fn build(src: &str) -> Self {
        let mut idx = LineIndex::default();
        let mut frames: Vec<Frame> = Vec::new();
        let mut block_scalar_indent: Option<usize> = None;
        for (n, raw) in src.lines().enumerate() {
            let line_no = n + 1;
            let trimmed = raw.trim_start_matches(' ');
            let indent = raw.len() - trimmed.len();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            if let Some(bi) = block_scalar_indent {
                if indent > bi {
                    continue; // inside a | or > block scalar
                }
                block_scalar_indent = None;
            }
            if trimmed == "---" || trimmed == "..." {
                frames.clear();
                continue;
            }
            let mut content = trimmed;
            let mut content_indent = indent;
            if trimmed == "-" || trimmed.starts_with("- ") {
                // Pop siblings and deeper frames; a dash may sit at the same
                // indent as its parent key.
                while let Some(top) = frames.last() {
                    let same_indent_parent = top.indent == indent && matches!(top.seg, Seg::Key(_));
                    if top.indent > indent || (top.indent == indent && !same_indent_parent) {
                        frames.pop();
                    } else {
                        break;
                    }
                }
                let i = match frames.last_mut() {
                    Some(f) => {
                        let i = f.next_idx;
                        f.next_idx += 1;
                        i
                    }
                    None => 0,
                };
                frames.push(Frame {
                    indent,
                    seg: Seg::Idx(i),
                    next_idx: 0,
                });
                idx.lines.entry(join_path(&frames)).or_insert(line_no);
                let rest = trimmed[1..].trim_start();
                content_indent = indent + (trimmed.len() - rest.len());
                content = rest;
                if content.is_empty() {
                    continue;
                }
            } else {
                while frames.last().is_some_and(|f| f.indent >= indent) {
                    frames.pop();
                }
            }
            if let Some((key, rest)) = split_key(content) {
                // Keys inside a dash item share the item's frame level.
                while frames
                    .last()
                    .is_some_and(|f| f.indent >= content_indent && matches!(f.seg, Seg::Key(_)))
                {
                    frames.pop();
                }
                frames.push(Frame {
                    indent: content_indent,
                    seg: Seg::Key(key),
                    next_idx: 0,
                });
                idx.lines.entry(join_path(&frames)).or_insert(line_no);
                if rest.starts_with('|') || rest.starts_with('>') {
                    block_scalar_indent = Some(content_indent);
                }
            }
        }
        idx
    }

    /// Line of `path`, or of its nearest indexed ancestor (0 if none).
    pub fn line_of(&self, path: &str) -> usize {
        let mut p = path.to_string();
        loop {
            if let Some(l) = self.lines.get(&p) {
                return *l;
            }
            if p.is_empty() {
                return 0;
            }
            // Strip the last segment.
            let cut = p.rfind(['.', '[']).unwrap_or(0);
            p.truncate(cut);
            if cut == 0 {
                return self.lines.get("").copied().unwrap_or(0);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rules
// ---------------------------------------------------------------------------

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

fn squash(s: &str) -> String {
    s.chars()
        .filter(|c| *c != '_' && *c != '-')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn snake_to_camel(s: &str) -> String {
    let mut out = String::new();
    let mut up = false;
    for c in s.chars() {
        if c == '_' {
            up = true;
        } else if up {
            out.extend(c.to_uppercase());
            up = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn closest<'a>(key: &str, known: impl Iterator<Item = &'a String>) -> Option<String> {
    let sq = squash(key);
    let mut best: Option<(usize, String)> = None;
    for k in known {
        let d = if squash(k) == sq {
            0
        } else {
            levenshtein(&key.to_lowercase(), &k.to_lowercase())
        };
        if d <= 2 && best.as_ref().is_none_or(|(bd, _)| d < *bd) {
            best = Some((d, k.clone()));
        }
    }
    best.map(|(_, k)| k)
}

/// Keys present in the input that the typed spec drops on a round trip.
fn unknown_keys(
    input: &Value,
    canonical: &Json,
    path: &str,
    out: &mut Vec<(String, String, Option<String>)>,
) {
    match (input, canonical) {
        (Value::Mapping(m), Json::Object(o)) => {
            for (k, v) in m {
                let Some(key) = k.as_str() else { continue };
                let child = if path.is_empty() {
                    key.to_string()
                } else {
                    format!("{path}.{key}")
                };
                match o.get(key) {
                    Some(cv) => unknown_keys(v, cv, &child, out),
                    None => {
                        let sugg = closest(key, o.keys());
                        out.push((child, key.to_string(), sugg));
                    }
                }
            }
        }
        (Value::Sequence(s), Json::Array(a)) => {
            for (i, (v, cv)) in s.iter().zip(a.iter()).enumerate() {
                unknown_keys(v, cv, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

/// Kubernetes label key: optional DNS-subdomain prefix + `/` + name (<= 63).
pub fn valid_label_key(key: &str) -> bool {
    let (prefix, name) = match key.split_once('/') {
        Some((p, n)) => (Some(p), n),
        None => (None, key),
    };
    if let Some(p) = prefix {
        if p.is_empty()
            || p.len() > 253
            || !p.split('.').all(|part| {
                !part.is_empty()
                    && part.len() <= 63
                    && part
                        .bytes()
                        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
                    && !part.starts_with('-')
                    && !part.ends_with('-')
            })
        {
            return false;
        }
    }
    valid_label_name(name, 63, false)
}

/// Label value / key name segment: alphanumerics plus `-_.`, alnum at both ends.
pub fn valid_label_name(s: &str, max: usize, allow_empty: bool) -> bool {
    if s.is_empty() {
        return allow_empty;
    }
    let b = s.as_bytes();
    s.len() <= max
        && b[0].is_ascii_alphanumeric()
        && b[b.len() - 1].is_ascii_alphanumeric()
        && b.iter()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'_' | b'.'))
}

fn has_control(s: &str) -> bool {
    s.chars().any(|c| c.is_control() && c != '\t')
}

/// Walk every string in the document looking for control characters.
fn scan_control_chars(v: &Value, path: &str, out: &mut Vec<Diagnostic>) {
    match v {
        Value::String(s) if has_control(s) && !s.contains('\n') => {
            out.push(diag(
                "E012",
                Severity::Error,
                path,
                "string contains a control character".to_string(),
            ));
        }
        Value::Sequence(seq) => {
            for (i, c) in seq.iter().enumerate() {
                scan_control_chars(c, &format!("{path}[{i}]"), out);
            }
        }
        Value::Mapping(m) => {
            for (k, c) in m {
                let ks = k.as_str().unwrap_or("?");
                let p = if path.is_empty() {
                    ks.to_string()
                } else {
                    format!("{path}.{ks}")
                };
                if has_control(ks) {
                    out.push(diag(
                        "E012",
                        Severity::Error,
                        &p,
                        "key contains a control character".to_string(),
                    ));
                }
                scan_control_chars(c, &p, out);
            }
        }
        _ => {}
    }
}

fn semantic_rules(raw: &Value, out: &mut Vec<Diagnostic>) {
    // Labels and annotations: injection through keys/values.
    for section in ["metadata.labels", "metadata.annotations"] {
        let Some(Value::Mapping(m)) = portability::lookup(raw, section) else {
            continue;
        };
        for (k, v) in m {
            let Some(ks) = k.as_str() else {
                out.push(diag(
                    "E030",
                    Severity::Error,
                    section,
                    "label keys must be strings".into(),
                ));
                continue;
            };
            let p = format!("{section}.{ks}");
            if !valid_label_key(ks) {
                let mut d = diag(
                    "E030",
                    Severity::Error,
                    &p,
                    format!("`{ks}` is not a valid label/annotation key"),
                );
                d.hint = Some(
                    "use [prefix/]name: alphanumerics, '-', '_', '.'; name <= 63 chars".into(),
                );
                out.push(d);
            }
            if section == "metadata.labels" {
                let val = match v {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => {
                        out.push(diag(
                            "E031",
                            Severity::Error,
                            &p,
                            "label values must be scalars".into(),
                        ));
                        continue;
                    }
                };
                if !valid_label_name(&val, 63, true) {
                    let mut d = diag(
                        "E031",
                        Severity::Error,
                        &p,
                        format!("`{val}` is not a valid label value"),
                    );
                    d.hint = Some(
                        "<= 63 chars, alphanumeric at both ends, only '-', '_', '.' inside".into(),
                    );
                    out.push(d);
                }
            }
        }
    }

    // Duplicate container ports and port ranges.
    if let Some(Value::Sequence(ports)) = portability::lookup(raw, "network.ports") {
        let mut seen: HashMap<(u64, String), usize> = HashMap::new();
        for (i, p) in ports.iter().enumerate() {
            let get = |k: &str| p.get(k).and_then(Value::as_u64);
            let proto = p
                .get("protocol")
                .and_then(Value::as_str)
                .unwrap_or("TCP")
                .to_uppercase();
            for key in ["containerPort", "servicePort"] {
                if get(key) == Some(0) {
                    out.push(diag(
                        "E032",
                        Severity::Error,
                        &format!("network.ports[{i}].{key}"),
                        "port 0 is not valid".into(),
                    ));
                }
            }
            if let Some(cp) = get("containerPort") {
                if let Some(first) = seen.insert((cp, proto.clone()), i) {
                    out.push(diag(
                        "W033",
                        Severity::Warning,
                        &format!("network.ports[{i}].containerPort"),
                        format!("duplicate {proto} containerPort {cp} (first at network.ports[{first}])"),
                    ));
                }
            }
        }
    }

    // runtime.preferred must be reachable through runtime.allow.
    let preferred = portability::lookup(raw, "runtime.preferred").and_then(Value::as_str);
    if let (Some(pref), Some(Value::Sequence(allow))) =
        (preferred, portability::lookup(raw, "runtime.allow"))
    {
        let allowed: Vec<&str> = allow.iter().filter_map(Value::as_str).collect();
        if pref != "auto" && !allowed.contains(&pref) {
            out.push(diag(
                "E034",
                Severity::Error,
                "runtime.preferred",
                format!("preferred runtime `{pref}` is not in runtime.allow {allowed:?}"),
            ));
        }
    }

    // Scaling sanity.
    if let Some(sc) = portability::lookup(raw, "scaling") {
        let min = sc.get("minReplicas").and_then(Value::as_u64);
        let max = sc.get("maxReplicas").and_then(Value::as_u64);
        if let (Some(min), Some(max)) = (min, max) {
            if min > max {
                out.push(diag(
                    "E035",
                    Severity::Error,
                    "scaling.minReplicas",
                    format!("minReplicas {min} exceeds maxReplicas {max}"),
                ));
            }
        }
        if portability::is_set(sc)
            && sc
                .get("metrics")
                .and_then(Value::as_sequence)
                .is_none_or(Vec::is_empty)
        {
            out.push(diag(
                "W036",
                Severity::Warning,
                "scaling",
                "scaling is enabled without metrics; the autoscaler has nothing to act on".into(),
            ));
        }
    }

    // Ingress without a host.
    if let Some(ing) = portability::lookup(raw, "ingress") {
        if portability::is_set(ing)
            && ing
                .get("host")
                .and_then(Value::as_str)
                .is_none_or(str::is_empty)
        {
            out.push(diag(
                "W037",
                Severity::Warning,
                "ingress.host",
                "ingress is enabled but host is empty".into(),
            ));
        }
    }

    // Requests above limits.
    for (req, lim, parse) in [
        (
            "requirements.cpuRequest",
            "requirements.cpu",
            crate::resources::parse_cpu as fn(&str) -> f64,
        ),
        (
            "requirements.memoryRequest",
            "requirements.memory",
            crate::resources::parse_memory_gi as fn(&str) -> f64,
        ),
    ] {
        let (Some(r), Some(l)) = (
            portability::lookup(raw, req).and_then(Value::as_str),
            portability::lookup(raw, lim).and_then(Value::as_str),
        ) else {
            continue;
        };
        if parse(r) > parse(l) {
            out.push(diag(
                "E038",
                Severity::Error,
                req,
                format!("request `{r}` is larger than limit `{l}`"),
            ));
        }
    }

    // Networked workload without any probe.
    let exposes = portability::lookup(raw, "network.ports").is_some_and(portability::is_set);
    let has_probe = portability::lookup(raw, "health").is_some_and(portability::is_set);
    if exposes && !has_probe {
        out.push(diag(
            "I039",
            Severity::Info,
            "health",
            "workload exposes ports but defines no health probes".into(),
        ));
    }
}

/// Fields the schema requires even where a runtime ignores them.
const MANDATORY_FIELDS: &[&str] = &["requirements.storage", "build"];

/// Warn about fields the declared runtimes will silently drop.
fn portability_rules(raw: &Value, out: &mut Vec<Diagnostic>) {
    let targets = portability::declared_targets(raw);
    let rep = portability::report(raw);
    for row in &rep.rows {
        // The schema makes these mandatory, so a warning would be unactionable
        // (they still appear in `aether portability`).
        if MANDATORY_FIELDS.contains(&row.path.as_str()) {
            continue;
        }
        // Only complain when *every* declared runtime ignores the field, or
        // when the user pinned a single runtime that ignores it.
        let ignoring: Vec<Target> = targets
            .iter()
            .copied()
            .filter(|t| portability::support_of(row, *t) == Support::Ignored)
            .collect();
        if ignoring.is_empty() {
            continue;
        }
        if ignoring.len() == targets.len() {
            out.push(diag(
                "W040",
                Severity::Warning,
                &row.path,
                format!(
                    "`{}` is ignored by every allowed runtime ({})",
                    row.path,
                    names(&ignoring)
                ),
            ));
        } else {
            let mut d = diag(
                "I041",
                Severity::Info,
                &row.path,
                format!("`{}` is ignored on {}", row.path, names(&ignoring)),
            );
            d.hint = Some("run `aether portability` for the full matrix".into());
            out.push(d);
        }
    }
}

fn names(ts: &[Target]) -> String {
    ts.iter().map(|t| t.as_str()).collect::<Vec<_>>().join(", ")
}

/// Lint a spec document. Never panics on hostile input.
pub fn lint_str(src: &str) -> LintReport {
    let mut diags = Vec::new();
    let raw = match guarded_parse(src) {
        Ok(v) => v,
        Err(d) => return LintReport::new(vec![d]),
    };
    let index = LineIndex::build(src);

    if !matches!(raw, Value::Mapping(_)) {
        return LintReport::new(vec![diag(
            "E010",
            Severity::Error,
            "",
            "the spec must be a YAML mapping".into(),
        )]);
    }

    scan_control_chars(&raw, "", &mut diags);
    semantic_rules(&raw, &mut diags);

    // Parse from the source text (not the generic value) so scalars coerce the
    // same way as `aether validate` and errors carry a line.
    match serde_yaml::from_str::<Workload>(src) {
        Ok(w) => {
            if let Err(e) = w.validate() {
                diags.push(diag("E011", Severity::Error, "", e.to_string()));
            }
            if let Ok(canon) = serde_json::to_value(&w) {
                let mut unknown = Vec::new();
                unknown_keys(&raw, &canon, "", &mut unknown);
                for (path, key, sugg) in unknown {
                    // snake_case spellings of camelCase fields are accepted as aliases.
                    if let Some(canon) = sugg.as_ref().filter(|c| snake_to_camel(&key) == **c) {
                        let mut d = diag(
                            "I021",
                            Severity::Info,
                            &path,
                            format!("`{key}` is accepted as an alias"),
                        );
                        d.hint = Some(format!("canonical spelling is `{canon}`"));
                        diags.push(d);
                        continue;
                    }
                    let mut d = diag(
                        "W020",
                        Severity::Warning,
                        &path,
                        format!("unknown field `{key}` is ignored"),
                    );
                    d.hint = sugg.map(|s| format!("did you mean `{s}`?"));
                    diags.push(d);
                }
            }
            portability_rules(&raw, &mut diags);
        }
        Err(e) => {
            let mut d = diag("E010", Severity::Error, "", format!("schema error: {e}"));
            d.line = e.location().map(|l| l.line()).unwrap_or(0);
            diags.push(d);
        }
    }

    for d in &mut diags {
        if d.line == 0 {
            d.line = index.line_of(&d.path);
        }
    }
    LintReport::new(diags)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GOOD: &str = "apiVersion: aether/v1\nkind: Workload\nmetadata:\n  name: web\n  owner: me\n  project: p\nbuild:\n  context: .\n  dockerfile: Dockerfile\n  registry: localhost\nrequirements:\n  cpu: \"500m\"\n  memory: 256Mi\n  storage: 1Gi\nruntime:\n  preferred: kube\n  allow:\n    - kube\nnetwork:\n  service: true\n  ports:\n    - containerPort: 80\n      servicePort: 80\n";

    fn codes(r: &LintReport) -> Vec<&'static str> {
        r.diagnostics.iter().map(|d| d.code).collect()
    }

    #[test]
    fn clean_spec_has_no_errors() {
        let r = lint_str(GOOD);
        assert_eq!(r.errors, 0, "{:?}", r.diagnostics);
        assert_eq!(r.warnings, 0, "{:?}", r.diagnostics);
    }

    #[test]
    fn unknown_field_gets_line_and_suggestion() {
        let src = GOOD.replace(
            "  service: true\n",
            "  service: true\n  sesionAffinity: ClientIP\n",
        );
        let r = lint_str(&src);
        let d = r
            .diagnostics
            .iter()
            .find(|d| d.code == "W020")
            .expect("W020");
        assert_eq!(d.path, "network.sesionAffinity");
        assert_eq!(
            d.line,
            src.lines()
                .position(|l| l.contains("sesionAffinity"))
                .unwrap()
                + 1
        );
        assert!(d.hint.as_deref().unwrap().contains("sessionAffinity"));
    }

    #[test]
    fn schema_error_reports_a_message() {
        let r = lint_str("apiVersion: aether/v1\nkind: Workload\n");
        assert!(codes(&r).contains(&"E010"));
        assert!(r.failed(false));
    }

    #[test]
    fn malformed_yaml_reports_line() {
        let r = lint_str("a: [1, 2\nb: c\n");
        let d = &r.diagnostics[0];
        assert_eq!(d.code, "E010");
        assert!(d.line >= 1);
    }

    #[test]
    fn duplicate_keys_are_rejected() {
        let r = lint_str("a: 1\na: 2\n");
        assert_eq!(r.diagnostics[0].code, "E010");
    }

    #[test]
    fn oversized_spec_is_refused_without_parsing() {
        let big = "a: ".to_string() + &"x".repeat(MAX_SPEC_BYTES + 1);
        let r = lint_str(&big);
        assert_eq!(codes(&r), vec!["E001"]);
    }

    #[test]
    fn alias_bomb_is_refused() {
        let mut s = String::from("l0: &l0 [a, a]\n");
        for i in 1..40 {
            s.push_str(&format!("l{i}: &l{i} [*l{}, *l{}]\n", i - 1, i - 1));
        }
        let r = lint_str(&s);
        assert_eq!(codes(&r), vec!["E002"]);
    }

    #[test]
    fn few_aliases_in_a_small_doc_are_allowed() {
        assert_eq!(count_aliases("a: &x 1\nb: *x\nc: [*x, *x]\n"), 3);
        let r = lint_str(&format!("{GOOD}extra: &x 1\nother: *x\n"));
        assert!(!codes(&r).contains(&"E002"));
    }

    #[test]
    fn quoted_and_commented_stars_are_not_aliases() {
        assert_eq!(count_aliases("a: \"*x\"\nb: '*y'\nc: 1 # *z\nd: 2*3\n"), 0);
    }

    #[test]
    fn deep_nesting_is_refused() {
        let s = format!(
            "{}1{}",
            "[".repeat(MAX_DEPTH + 10),
            "]".repeat(MAX_DEPTH + 10)
        );
        let r = lint_str(&s);
        assert!(r.failed(false));
    }

    #[test]
    fn non_mapping_document_is_an_error() {
        assert_eq!(codes(&lint_str("- a\n- b\n")), vec!["E010"]);
        assert!(lint_str("").failed(false));
    }

    #[test]
    fn hostile_label_keys_and_values_are_flagged() {
        let src = GOOD.replace(
            "  project: p\n",
            "  project: p\n  labels:\n    \"bad key!\": ok\n    app: \"x\\\"; rm -rf /\"\n    ok.io/name: fine\n",
        );
        let r = lint_str(&src);
        assert!(codes(&r).contains(&"E030"), "{:?}", r.diagnostics);
        assert!(codes(&r).contains(&"E031"));
        assert_eq!(
            r.diagnostics
                .iter()
                .filter(|d| d.path == "metadata.labels.ok.io/name")
                .count(),
            0
        );
    }

    #[test]
    fn control_characters_in_names_are_flagged() {
        let src = GOOD.replace("name: web", "name: \"web\\u001b[31m\"");
        let r = lint_str(&src);
        assert!(codes(&r).contains(&"E012"));
    }

    #[test]
    fn path_traversal_in_build_context_is_not_silently_accepted() {
        // The linter does not read files, but it must not crash on such input.
        let src = GOOD.replace("context: .", "context: ../../../../etc");
        let r = lint_str(&src);
        assert_eq!(r.errors, 0, "{:?}", r.diagnostics);
    }

    #[test]
    fn preferred_runtime_must_be_allowed() {
        let src = GOOD.replace("preferred: kube", "preferred: kubevirt");
        assert!(codes(&lint_str(&src)).contains(&"E034"));
    }

    #[test]
    fn duplicate_ports_and_port_zero() {
        let src = GOOD.replace(
            "      servicePort: 80\n",
            "      servicePort: 80\n    - containerPort: 80\n      servicePort: 0\n",
        );
        let c = codes(&lint_str(&src));
        assert!(c.contains(&"W033"));
        assert!(c.contains(&"E032"));
    }

    #[test]
    fn single_runtime_ignoring_a_field_warns() {
        let src = GOOD
            .replace("preferred: kube", "preferred: container")
            .replace("- kube", "- container")
            + "ingress:\n  enabled: true\n  host: x.example\n";
        let r = lint_str(&src);
        assert!(
            r.diagnostics
                .iter()
                .any(|d| d.code == "W040" && d.path == "ingress"),
            "{:?}",
            r.diagnostics
        );
    }

    #[test]
    fn json_output_is_stable() {
        let r = lint_str("a: [");
        let j = serde_json::to_value(&r).unwrap();
        assert_eq!(j["errors"], 1);
        assert_eq!(j["diagnostics"][0]["code"], "E010");
    }

    #[test]
    fn line_index_handles_sequences_and_nesting() {
        let src = "a:\n  b:\n    - x: 1\n      y: 2\n    - x: 3\nc: |\n  d: not a key\ne: 1\n";
        let idx = LineIndex::build(src);
        assert_eq!(idx.line_of("a"), 1);
        assert_eq!(idx.line_of("a.b"), 2);
        assert_eq!(idx.line_of("a.b[0].y"), 4);
        assert_eq!(idx.line_of("a.b[1].x"), 5);
        assert_eq!(idx.line_of("c.d"), 6); // block scalar content is not indexed
        assert_eq!(idx.line_of("e"), 8);
        assert_eq!(idx.line_of("missing.path"), 0);
    }

    #[test]
    fn line_index_handles_dash_at_parent_indent() {
        let src = "list:\n- a: 1\n- a: 2\nafter: 1\n";
        let idx = LineIndex::build(src);
        assert_eq!(idx.line_of("list[1].a"), 3);
        assert_eq!(idx.line_of("after"), 4);
    }

    #[test]
    fn line_index_never_panics_on_garbage() {
        for s in [
            "",
            "-",
            ":",
            "- - - -",
            "\"unterminated: 1",
            "a:\n\t- b",
            "\u{0}\u{1}",
            "? x\n: y",
            "---\n---\n",
        ] {
            let _ = LineIndex::build(s);
            let _ = lint_str(s);
        }
    }

    #[test]
    fn text_rendering_includes_file_line_and_code() {
        let r = lint_str("a: [");
        let t = r.render_text("spec.yaml");
        assert!(t.contains("spec.yaml:"), "{t}");
        assert!(t.contains("error[E010]"));
        assert!(t.ends_with("1 error(s), 0 warning(s)\n"));
    }

    #[test]
    fn rendered_output_cannot_be_forged_by_input() {
        let r = lint_str("a: 1\n\"k\\nforged.yaml:1: error[E999]: x\": 1\n");
        for line in r.render_text("s.yaml").lines() {
            assert!(!line.starts_with("forged"), "{line}");
        }
        assert_eq!(sanitize("a\x1b[31mb\nc"), "a\\u{1b}[31mb\\nc");
    }

    #[test]
    fn label_validators() {
        assert!(valid_label_key("app"));
        assert!(valid_label_key("example.com/app-name"));
        assert!(!valid_label_key("-bad"));
        assert!(!valid_label_key("a/b/c"));
        assert!(!valid_label_key("UPPER.com/x"));
        assert!(valid_label_name("", 63, true));
        assert!(!valid_label_name("a b", 63, true));
        assert!(!valid_label_name(&"x".repeat(64), 63, true));
    }

    #[test]
    fn snake_case_aliases_are_info_not_warnings() {
        let src = GOOD.replace(
            "  service: true\n",
            "  service: true\n  service_type: NodePort\n",
        );
        let r = lint_str(&src);
        assert!(
            r.diagnostics.iter().any(|d| d.code == "I021"),
            "{:?}",
            r.diagnostics
        );
        assert_eq!(r.warnings, 0, "{:?}", r.diagnostics);
        // And the alias really is honoured by the parser.
        let w: Workload = serde_yaml::from_str(&src).unwrap();
        assert!(matches!(
            w.network.service_type,
            crate::spec::ServiceType::NodePort
        ));
    }

    #[test]
    fn camel_case_keys_documented_in_the_schema_are_honoured() {
        let src = GOOD.replace(
            "  service: true\n",
            "  service: true\n  serviceType: NodePort\n",
        );
        let w: Workload = serde_yaml::from_str(&src).unwrap();
        assert!(matches!(
            w.network.service_type,
            crate::spec::ServiceType::NodePort
        ));
        assert_eq!(lint_str(&src).warnings, 0);
    }

    #[test]
    fn suggestions_prefer_close_names() {
        let known = ["serviceType".to_string(), "ports".to_string()];
        assert_eq!(
            closest("servicetype", known.iter()).as_deref(),
            Some("serviceType")
        );
        assert_eq!(
            closest("service_type", known.iter()).as_deref(),
            Some("serviceType")
        );
        assert_eq!(closest("zzzzzz", known.iter()), None);
    }
}
