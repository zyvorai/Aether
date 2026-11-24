// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Lint rule catalog and CI output formats.
//!
//! * [`RULES`] documents every diagnostic code (`aether lint --explain CODE`,
//!   `aether lint --list-rules`, `GET /api/spec/lint/rules`).
//! * [`render`] turns a [`LintReport`] into text, JSON, SARIF 2.1.0 (GitHub
//!   code scanning), JUnit XML or GitHub Actions workflow commands.
//!
//! Everything in a diagnostic can be influenced by the spec author, so each
//! format escapes for its own syntax (XML, workflow-command, JSON).

use crate::hexfmt::hex_lower;
use crate::speclint::{sanitize, Diagnostic, LintReport, Severity};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

/// One documented diagnostic code.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct RuleInfo {
    pub code: &'static str,
    pub severity: Severity,
    pub title: &'static str,
    pub explanation: &'static str,
    /// Only reported with `--best-practices`.
    pub opt_in: bool,
}

macro_rules! rule {
    ($code:literal, $sev:ident, $title:literal, $why:literal) => {
        RuleInfo {
            code: $code,
            severity: Severity::$sev,
            title: $title,
            explanation: $why,
            opt_in: false,
        }
    };
    ($code:literal, $sev:ident, $title:literal, $why:literal, opt_in) => {
        RuleInfo {
            code: $code,
            severity: Severity::$sev,
            title: $title,
            explanation: $why,
            opt_in: true,
        }
    };
}

pub const RULES: &[RuleInfo] = &[
    rule!("E001", Error, "Spec too large", "The document exceeds the size limit (1 MiB). Aether refuses to expand it. Split the spec or remove generated content."),
    rule!("E002", Error, "Alias expansion guard", "Too many YAML aliases, or aliases in a large document. Aliases can expand exponentially (billion laughs), so the limits are strict. Inline the repeated values."),
    rule!("E003", Error, "Spec rejected by the parser guard", "The document has too many nodes or is nested too deeply."),
    rule!("E010", Error, "Parse or schema error", "The YAML is invalid or does not match the workload schema (wrong type, missing required field)."),
    rule!("E011", Error, "Validation failed", "The spec parses but `aether validate` rejects it; the message names the field."),
    rule!("E012", Error, "Control character", "A key or string contains a control character, which can forge log lines or terminal escape sequences. Remove it."),
    rule!("W020", Warning, "Unknown field", "The key is not part of the schema and is silently ignored by every runtime. Usually a typo; the hint suggests the nearest valid key."),
    rule!("I021", Info, "snake_case alias", "The snake_case spelling is accepted but the canonical key is camelCase."),
    rule!("E030", Error, "Invalid label or annotation key", "Keys must be an optional DNS prefix plus a name of up to 63 alphanumerics, `-`, `_` or `.`."),
    rule!("E031", Error, "Invalid label value", "Label values must be scalars of up to 63 characters (alphanumerics, `-`, `_`, `.`)."),
    rule!("E032", Error, "Port 0", "Port numbers must be between 1 and 65535."),
    rule!("W033", Warning, "Duplicate port", "Two port mappings use the same protocol and containerPort."),
    rule!("E034", Error, "Preferred runtime not allowed", "`runtime.preferred` must appear in `runtime.allow` when that list is set."),
    rule!("E035", Error, "minReplicas above maxReplicas", "The autoscaler range is empty."),
    rule!("W036", Warning, "Scaling without metrics", "Scaling is enabled but no metric is configured, so the autoscaler never acts."),
    rule!("W037", Warning, "Ingress without host", "Ingress is enabled but `ingress.host` is empty."),
    rule!("E038", Error, "Request above limit", "A resource request is larger than its limit; the scheduler would reject the pod."),
    rule!("I039", Info, "Ports without probes", "The workload exposes ports but defines no health probes."),
    rule!("W040", Warning, "Field ignored by every runtime", "Every runtime in `runtime.allow` drops this field. See `aether portability`."),
    rule!("I041", Info, "Field ignored on some runtimes", "At least one allowed runtime drops this field. See `aether portability`."),
    rule!("B101", Warning, "No liveness probe", "Without a liveness probe a hung process is never restarted. Skipped for scheduled jobs.", opt_in),
    rule!("B102", Warning, "No readiness probe", "Without a readiness probe the Service sends traffic to pods that are still starting.", opt_in),
    rule!("B103", Warning, "May run as root", "Set `kubernetes.containerSecurityContext.runAsNonRoot: true` (or the pod-level equivalent).", opt_in),
    rule!("B104", Warning, "Writable root filesystem", "Set `readOnlyRootFilesystem: true` and mount volumes for paths that need writes.", opt_in),
    rule!("B105", Warning, "Privilege escalation allowed", "Set `allowPrivilegeEscalation: false` so setuid binaries cannot gain privileges.", opt_in),
    rule!("B106", Warning, "Single replica without disruption budget", "A node drain takes the workload down. Use `scaling.minReplicas >= 2` or a podDisruptionBudget.", opt_in),
    rule!("B107", Warning, "No network policy", "Without a network policy every pod in the cluster can reach the exposed ports.", opt_in),
    rule!("B108", Warning, "Floating image tag", "A sidecar or init container uses no tag, `:latest`, and no digest, so the image can change between rollouts.", opt_in),
    rule!("B109", Warning, "Default service account", "The pod runs as the namespace's default service account. Create one with only the permissions the workload needs.", opt_in),
];

/// Look up a rule by code (case-insensitive).
pub fn rule(code: &str) -> Option<&'static RuleInfo> {
    RULES.iter().find(|r| r.code.eq_ignore_ascii_case(code))
}

/// Output format for `aether lint --format`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Text,
    Json,
    Sarif,
    Junit,
    Github,
}

impl std::str::FromStr for Format {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        match s.to_ascii_lowercase().as_str() {
            "text" => Ok(Format::Text),
            "json" => Ok(Format::Json),
            "sarif" => Ok(Format::Sarif),
            "junit" => Ok(Format::Junit),
            "github" => Ok(Format::Github),
            other => Err(format!(
                "unknown format `{other}` (expected text, json, sarif, junit or github)"
            )),
        }
    }
}

pub fn render(report: &LintReport, file: &str, format: Format) -> String {
    match format {
        Format::Text => report.render_text(file),
        Format::Json => serde_json::to_string_pretty(report).unwrap_or_default() + "\n",
        Format::Sarif => {
            serde_json::to_string_pretty(&sarif(report, file)).unwrap_or_default() + "\n"
        }
        Format::Junit => junit(report, file),
        Format::Github => github(report, file),
    }
}

fn level(s: Severity) -> &'static str {
    match s {
        Severity::Error => "error",
        Severity::Warning => "warning",
        Severity::Info => "note",
    }
}

fn fingerprint(d: &Diagnostic) -> String {
    let mut h = Sha256::new();
    h.update(d.code.as_bytes());
    h.update([0]);
    h.update(d.path.as_bytes());
    h.update([0]);
    h.update(d.message.as_bytes());
    hex_lower(h.finalize())[..32].to_string()
}

/// SARIF 2.1.0 log with the properties GitHub code scanning requires.
pub fn sarif(report: &LintReport, file: &str) -> serde_json::Value {
    let rules: Vec<_> = RULES
        .iter()
        .map(|r| {
            json!({
                "id": r.code,
                "name": r.title,
                "shortDescription": {"text": r.title},
                "fullDescription": {"text": r.explanation},
                "help": {"text": r.explanation},
                "defaultConfiguration": {"level": level(r.severity)},
            })
        })
        .collect();
    let results: Vec<_> = report
        .diagnostics
        .iter()
        .map(|d| {
            let line = d.line.max(1);
            let mut text = sanitize(&d.message);
            if !d.path.is_empty() {
                text.push_str(&format!(" ({})", sanitize(&d.path)));
            }
            json!({
                "ruleId": d.code,
                "level": level(d.severity),
                "message": {"text": text},
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": {"uri": file},
                        "region": {
                            "startLine": line, "startColumn": 1,
                            "endLine": line, "endColumn": 1
                        }
                    }
                }],
                "partialFingerprints": {"primaryLocationLineHash": fingerprint(d)},
            })
        })
        .collect();
    json!({
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {"driver": {
                "name": "aether lint",
                "informationUri": "https://github.com/zyvorai/Aether",
                "version": env!("CARGO_PKG_VERSION"),
                "rules": rules,
            }},
            "results": results,
        }]
    })
}

/// Escape for XML 1.0 text and attributes; drops characters XML cannot carry.
pub fn xml_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{fffe}' || c == '\u{ffff}' => {}
            c => out.push(c),
        }
    }
    out
}

/// JUnit XML: one testcase per diagnostic (errors fail, warnings are
/// failures of type `warning`, infos pass with output); one passing case when
/// the spec is clean.
pub fn junit(report: &LintReport, file: &str) -> String {
    let failures = report
        .diagnostics
        .iter()
        .filter(|d| d.severity != Severity::Info)
        .count();
    let tests = report.diagnostics.len().max(1);
    let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    out.push_str(&format!(
        "<testsuite name=\"aether-lint\" tests=\"{tests}\" failures=\"{failures}\" errors=\"0\">\n"
    ));
    if report.diagnostics.is_empty() {
        out.push_str(&format!(
            "  <testcase classname=\"{f}\" name=\"lint\"/>\n",
            f = xml_escape(file)
        ));
    }
    for d in &report.diagnostics {
        let name = if d.path.is_empty() {
            d.code.to_string()
        } else {
            format!("{} {}", d.code, d.path)
        };
        out.push_str(&format!(
            "  <testcase classname=\"{}\" name=\"{}\">\n",
            xml_escape(file),
            xml_escape(&name)
        ));
        let body = format!("line {}: {}", d.line, d.message);
        match d.severity {
            Severity::Info => out.push_str(&format!(
                "    <system-out>{}</system-out>\n",
                xml_escape(&body)
            )),
            s => out.push_str(&format!(
                "    <failure type=\"{}\" message=\"{}\">{}</failure>\n",
                level(s),
                xml_escape(&d.message),
                xml_escape(&body)
            )),
        }
        out.push_str("  </testcase>\n");
    }
    out.push_str("</testsuite>\n");
    out
}

fn gh_data(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

fn gh_prop(s: &str) -> String {
    gh_data(s).replace(':', "%3A").replace(',', "%2C")
}

/// GitHub Actions workflow commands (`::error file=..,line=..::msg`).
pub fn github(report: &LintReport, file: &str) -> String {
    let mut out = String::new();
    for d in &report.diagnostics {
        let cmd = match d.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "notice",
        };
        out.push_str(&format!(
            "::{cmd} file={},line={},title={}::{}\n",
            gh_prop(file),
            d.line.max(1),
            gh_prop(d.code),
            gh_data(&d.message)
        ));
    }
    out
}

/// Text for `aether lint --explain CODE`.
pub fn explain(info: &RuleInfo) -> String {
    format!(
        "{} ({}){}\n{}\n\n{}\n\nSuppress with `--ignore {}` or `# aether-lint: ignore={}` on or above the line.\n",
        info.code,
        info.severity.as_str(),
        if info.opt_in { " [--best-practices]" } else { "" },
        info.title,
        info.explanation,
        info.code,
        info.code
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::speclint::{lint_str, lint_with, LintOptions};

    const BAD: &str = "apiVersion: aether/v1\nkind: Workload\nmetadata:\n  name: web\n  owner: me\n  project: p\nbuild:\n  context: .\n  dockerfile: Dockerfile\n  registry: localhost\nrequirements:\n  cpu: \"500m\"\n  memory: 256Mi\n  storage: 1Gi\nruntime:\n  preferred: kube\n  allow:\n    - kube\nnetwork:\n  servicetype: NodePort\n  ports:\n    - containerPort: 80\n      servicePort: 80\n";

    #[test]
    fn every_code_in_speclint_is_documented() {
        let src = include_str!("speclint.rs");
        let body = src.split("#[cfg(test)]").next().unwrap();
        let mut seen = std::collections::BTreeSet::new();
        let b = body.as_bytes();
        for i in 0..b.len().saturating_sub(6) {
            if b[i] == b'"'
                && matches!(b[i + 1], b'E' | b'W' | b'I' | b'B')
                && b[i + 2..i + 5].iter().all(u8::is_ascii_digit)
                && b[i + 5] == b'"'
            {
                seen.insert(String::from_utf8_lossy(&b[i + 1..i + 5]).to_string());
            }
        }
        assert!(seen.len() > 20, "scan found {seen:?}");
        for code in seen {
            assert!(rule(&code).is_some(), "{code} is not in RULES");
        }
    }

    #[test]
    fn catalog_codes_are_unique_and_sorted_by_family() {
        let mut codes: Vec<_> = RULES.iter().map(|r| r.code).collect();
        let n = codes.len();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), n);
        assert!(RULES.iter().all(|r| !r.explanation.is_empty()));
    }

    #[test]
    fn format_parsing() {
        assert_eq!("SARIF".parse::<Format>().unwrap(), Format::Sarif);
        assert!("xml".parse::<Format>().is_err());
    }

    #[test]
    fn sarif_has_required_properties() {
        let r = lint_str(BAD);
        let v = sarif(&r, "spec.yaml");
        assert_eq!(v["version"], "2.1.0");
        let res = &v["runs"][0]["results"];
        let first = &res[0];
        assert_eq!(first["ruleId"], "W020");
        assert_eq!(first["level"], "warning");
        let region = &first["locations"][0]["physicalLocation"]["region"];
        assert_eq!(region["startLine"], 20);
        assert_eq!(
            first["locations"][0]["physicalLocation"]["artifactLocation"]["uri"],
            "spec.yaml"
        );
        assert_eq!(
            first["partialFingerprints"]["primaryLocationLineHash"]
                .as_str()
                .unwrap()
                .len(),
            32
        );
        assert!(
            v["runs"][0]["tool"]["driver"]["rules"]
                .as_array()
                .unwrap()
                .len()
                >= RULES.len()
        );
    }

    #[test]
    fn sarif_fingerprints_are_stable_and_distinct() {
        let r = lint_str(BAD);
        let a = sarif(&r, "x");
        let b = sarif(&lint_str(BAD), "x");
        assert_eq!(a, b);
        let ds = &r.diagnostics;
        assert!(ds.len() >= 2);
        assert_ne!(fingerprint(&ds[0]), fingerprint(&ds[1]));
    }

    #[test]
    fn junit_counts_and_escapes() {
        let mut r = lint_str(BAD);
        r.diagnostics[0].message = "a <b> & \"c\" \u{1b}[31m".to_string();
        let x = junit(&r, "we<ird>.yaml");
        assert!(x.starts_with("<?xml"));
        assert!(x.contains("failures=\"1\""));
        assert!(x.contains("&lt;b&gt; &amp; &quot;c&quot; [31m"));
        assert!(x.contains("we&lt;ird&gt;.yaml"));
        assert!(!x.contains('\u{1b}'));
    }

    #[test]
    fn junit_clean_spec_is_one_passing_case() {
        let mut r = lint_str(BAD);
        r.diagnostics.clear();
        let x = junit(&r, "s.yaml");
        assert!(x.contains("tests=\"1\" failures=\"0\""));
        assert!(x.contains("<testcase"));
    }

    #[test]
    fn github_commands_escape_injection() {
        let mut r = lint_str(BAD);
        r.diagnostics[0].message = "x\n::error::forged%".to_string();
        let g = github(&r, "a,b:c.yaml");
        let first = g.lines().next().unwrap();
        assert!(first.starts_with("::warning file=a%2Cb%3Ac.yaml,line=20,title=W020::"));
        assert!(first.contains("x%0A::error::forged%25"));
        assert_eq!(g.lines().filter(|l| l.starts_with("::error")).count(), 0);
    }

    #[test]
    fn best_practices_are_opt_in_and_suppressible() {
        let plain = lint_str(BAD);
        assert!(!plain.diagnostics.iter().any(|d| d.code.starts_with('B')));
        let bp = lint_with(
            BAD,
            &LintOptions {
                best_practices: true,
                ..Default::default()
            },
        );
        let codes: Vec<_> = bp.diagnostics.iter().map(|d| d.code).collect();
        for c in [
            "B101", "B102", "B103", "B104", "B105", "B106", "B107", "B109",
        ] {
            assert!(codes.contains(&c), "missing {c}: {codes:?}");
        }
        let ign = lint_with(
            BAD,
            &LintOptions {
                best_practices: true,
                ignore: vec!["b101".into(), "B10".into()],
            },
        );
        assert!(!ign.diagnostics.iter().any(|d| d.code == "B101"));
        assert_eq!(ign.suppressed, 1);
    }

    #[test]
    fn inline_directives() {
        let src = BAD.replace(
            "  servicetype: NodePort\n",
            "  # aether-lint: ignore=W020\n  servicetype: NodePort\n",
        );
        let r = lint_str(&src);
        assert!(
            !r.diagnostics.iter().any(|d| d.code == "W020"),
            "{:?}",
            r.diagnostics
        );
        assert_eq!(r.suppressed, 1);
        let trailing = BAD.replace(
            "  servicetype: NodePort\n",
            "  servicetype: NodePort  # aether-lint: ignore=w020\n",
        );
        assert_eq!(lint_str(&trailing).suppressed, 1);
        let file = format!("# aether-lint: ignore-file=W020\n{BAD}");
        assert_eq!(lint_str(&file).suppressed, 1);
        // A directive does not leak to unrelated lines.
        let other = BAD.replace(
            "  servicetype: NodePort\n",
            "  # aether-lint: ignore=W033\n  servicetype: NodePort\n",
        );
        assert!(lint_str(&other)
            .diagnostics
            .iter()
            .any(|d| d.code == "W020"));
    }

    #[test]
    fn parse_errors_cannot_be_suppressed() {
        let src = "# aether-lint: ignore-file=E010\n- a\n- b\n";
        let r = lint_with(
            src,
            &LintOptions {
                ignore: vec!["E010".into()],
                ..Default::default()
            },
        );
        assert_eq!(r.errors, 1);
        assert_eq!(r.suppressed, 0);
    }

    #[test]
    fn floating_image_detection() {
        use crate::speclint::image_is_floating as f;
        assert!(f("nginx"));
        assert!(f("nginx:latest"));
        assert!(f("localhost:5000/nginx"));
        assert!(!f("localhost:5000/nginx:1.27"));
        assert!(!f("nginx@sha256:abcd"));
    }

    #[test]
    fn explain_mentions_suppression() {
        let e = explain(rule("b103").unwrap());
        assert!(e.contains("B103") && e.contains("--ignore B103"));
    }
}
