// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Offline schema validation of Kubernetes manifests (`aether manifest-check`),
//! in the spirit of kubeconform.
//!
//! Each document is deserialised into the typed `k8s-openapi` object for its
//! `apiVersion`/`kind`. That rejects wrong types, missing required fields,
//! required fields that were left out, malformed base64 in Secret data, and apiVersions the bundled API no longer
//! serves (for example `extensions/v1beta1` Ingress). With `strict`, the typed
//! object is serialised back and every key in the input that did not survive
//! is reported as an unknown field (a misspelt `readinessProbe`, say), the same
//! idea as kubeconform's `-strict`.
//!
//! Limits, stated plainly: only the built-in kinds listed in [`KINDS`] have a
//! schema, so CRDs (KubeVirt `VirtualMachine`, Cilium policies, ...) are
//! "missing schema" and are errors unless `ignore_missing_schemas` is set. The
//! schema is the one bundled in the `k8s-openapi` crate, not fetched. CEL rules
//! and server-side admission are not evaluated.

use crate::lintfmt::xml_escape;
use crate::speclint::{self, sanitize};
use serde::Serialize;
use serde_json::Value;

/// Hard cap on the manifest text accepted in one call.
pub const MAX_BYTES: usize = 4 * 1024 * 1024;
/// Hard cap on documents per call.
pub const MAX_DOCS: usize = 2000;

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub strict: bool,
    pub ignore_missing_schemas: bool,
    /// Kinds to skip entirely (case-insensitive).
    pub skip: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Valid,
    Invalid,
    Skipped,
    /// Not validated because there is no schema or the document did not parse.
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct ResourceResult {
    pub source: String,
    pub line: usize,
    pub api_version: String,
    pub kind: String,
    pub name: String,
    pub status: Status,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Summary {
    pub valid: usize,
    pub invalid: usize,
    pub skipped: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckReport {
    pub results: Vec<ResourceResult>,
    pub summary: Summary,
}

impl CheckReport {
    pub fn failed(&self) -> bool {
        self.summary.invalid > 0 || self.summary.errors > 0
    }

    pub fn render_text(&self) -> String {
        let mut out = String::new();
        for r in &self.results {
            let label = match r.status {
                Status::Valid => "valid",
                Status::Invalid => "INVALID",
                Status::Skipped => "skipped",
                Status::Error => "ERROR",
            };
            out.push_str(&format!(
                "{}:{}: {} {}/{} {}\n",
                sanitize(&r.source),
                r.line,
                sanitize(&r.api_version),
                sanitize(&r.kind),
                sanitize(&r.name),
                label
            ));
            for m in &r.messages {
                out.push_str(&format!("    {}\n", sanitize(m)));
            }
        }
        let s = &self.summary;
        out.push_str(&format!(
            "Summary: {} valid, {} invalid, {} errors, {} skipped\n",
            s.valid, s.invalid, s.errors, s.skipped
        ));
        out
    }

    pub fn render_junit(&self) -> String {
        let s = &self.summary;
        let mut out = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        out.push_str(&format!(
            "<testsuite name=\"aether-manifest-check\" tests=\"{}\" failures=\"{}\" skipped=\"{}\">\n",
            self.results.len(),
            s.invalid + s.errors,
            s.skipped
        ));
        for r in &self.results {
            out.push_str(&format!(
                "  <testcase classname=\"{}\" name=\"{}\">",
                xml_escape(&sanitize(&r.source)),
                xml_escape(&sanitize(&format!("{}/{}", r.kind, r.name)))
            ));
            match r.status {
                Status::Valid => {}
                Status::Skipped => out.push_str("<skipped/>"),
                Status::Invalid | Status::Error => out.push_str(&format!(
                    "<failure message=\"{}\">{}:{}</failure>",
                    xml_escape(&sanitize(&r.messages.join("; "))),
                    xml_escape(&sanitize(&r.source)),
                    r.line
                )),
            }
            out.push_str("</testcase>\n");
        }
        out.push_str("</testsuite>\n");
        out
    }
}

macro_rules! kinds {
    ($(($av:literal, $k:literal, $t:ty)),* $(,)?) => {
        /// Every `(apiVersion, kind)` that has a bundled schema.
        pub const KINDS: &[(&str, &str)] = &[$(($av, $k)),*];

        /// Typed round trip: `None` when there is no schema for the pair.
        fn roundtrip(av: &str, kind: &str, v: &Value) -> Option<Result<Value, String>> {
            match (av, kind) {
                $(($av, $k) => Some(
                    serde_json::from_value::<$t>(v.clone())
                        .map_err(|e| e.to_string())
                        .and_then(|t| serde_json::to_value(&t).map_err(|e| e.to_string())),
                ),)*
                _ => None,
            }
        }
    };
}

kinds! {
    ("v1", "Pod", k8s_openapi::api::core::v1::Pod),
    ("v1", "Service", k8s_openapi::api::core::v1::Service),
    ("v1", "ConfigMap", k8s_openapi::api::core::v1::ConfigMap),
    ("v1", "Secret", k8s_openapi::api::core::v1::Secret),
    ("v1", "Namespace", k8s_openapi::api::core::v1::Namespace),
    ("v1", "ServiceAccount", k8s_openapi::api::core::v1::ServiceAccount),
    ("v1", "PersistentVolumeClaim", k8s_openapi::api::core::v1::PersistentVolumeClaim),
    ("v1", "ResourceQuota", k8s_openapi::api::core::v1::ResourceQuota),
    ("v1", "LimitRange", k8s_openapi::api::core::v1::LimitRange),
    ("apps/v1", "Deployment", k8s_openapi::api::apps::v1::Deployment),
    ("apps/v1", "StatefulSet", k8s_openapi::api::apps::v1::StatefulSet),
    ("apps/v1", "DaemonSet", k8s_openapi::api::apps::v1::DaemonSet),
    ("apps/v1", "ReplicaSet", k8s_openapi::api::apps::v1::ReplicaSet),
    ("batch/v1", "Job", k8s_openapi::api::batch::v1::Job),
    ("batch/v1", "CronJob", k8s_openapi::api::batch::v1::CronJob),
    ("networking.k8s.io/v1", "Ingress", k8s_openapi::api::networking::v1::Ingress),
    ("networking.k8s.io/v1", "NetworkPolicy", k8s_openapi::api::networking::v1::NetworkPolicy),
    ("autoscaling/v2", "HorizontalPodAutoscaler", k8s_openapi::api::autoscaling::v2::HorizontalPodAutoscaler),
    ("policy/v1", "PodDisruptionBudget", k8s_openapi::api::policy::v1::PodDisruptionBudget),
    ("rbac.authorization.k8s.io/v1", "Role", k8s_openapi::api::rbac::v1::Role),
    ("rbac.authorization.k8s.io/v1", "ClusterRole", k8s_openapi::api::rbac::v1::ClusterRole),
    ("rbac.authorization.k8s.io/v1", "RoleBinding", k8s_openapi::api::rbac::v1::RoleBinding),
    ("rbac.authorization.k8s.io/v1", "ClusterRoleBinding", k8s_openapi::api::rbac::v1::ClusterRoleBinding),
}

/// Keys present in `orig` that are missing from the typed round trip.
fn unknown_fields(orig: &Value, rt: &Value, path: &str, out: &mut Vec<String>) {
    match (orig, rt) {
        (Value::Object(o), Value::Object(r)) => {
            for (k, v) in o {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                match r.get(k) {
                    Some(rv) => unknown_fields(v, rv, &p, out),
                    None => {
                        // Null and empty values may legitimately be dropped.
                        let empty = v.is_null()
                            || v.as_object().is_some_and(|m| m.is_empty())
                            || v.as_array().is_some_and(|a| a.is_empty());
                        if !empty {
                            out.push(format!("unknown field `{p}`"));
                        }
                    }
                }
            }
        }
        (Value::Array(o), Value::Array(r)) => {
            for (i, (ov, rv)) in o.iter().zip(r).enumerate() {
                unknown_fields(ov, rv, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

/// Required fields that were absent from the input. The typed objects fill
/// missing required (non-optional) fields with defaults and always serialise
/// them, while optional fields are omitted when unset, so a key that exists
/// only in the round trip is a required field the author left out.
fn missing_required(orig: &Value, rt: &Value, path: &str, out: &mut Vec<String>) {
    match (orig, rt) {
        (Value::Object(o), Value::Object(r)) => {
            for (k, rv) in r {
                let p = if path.is_empty() {
                    k.clone()
                } else {
                    format!("{path}.{k}")
                };
                match o.get(k) {
                    Some(ov) => missing_required(ov, rv, &p, out),
                    None if path.is_empty() && (k == "apiVersion" || k == "kind") => {}
                    None => out.push(format!("missing required field `{p}`")),
                }
            }
        }
        (Value::Array(o), Value::Array(r)) => {
            for (i, (ov, rv)) in o.iter().zip(r).enumerate() {
                missing_required(ov, rv, &format!("{path}[{i}]"), out);
            }
        }
        _ => {}
    }
}

/// Split a multi-document stream on `---` lines, with each document's start line.
fn split_docs(src: &str) -> Vec<(usize, String)> {
    let mut docs = Vec::new();
    let mut cur = String::new();
    let mut start = 1usize;
    for (i, line) in src.lines().enumerate() {
        let n = i + 1;
        if line == "---" || line.starts_with("--- ") || line.trim_end() == "---" {
            if !cur.trim().is_empty() {
                docs.push((start, std::mem::take(&mut cur)));
            }
            cur.clear();
            start = n + 1;
            continue;
        }
        if cur.is_empty() && line.trim().is_empty() {
            start = n + 1;
            continue;
        }
        cur.push_str(line);
        cur.push('\n');
    }
    if !cur.trim().is_empty() {
        docs.push((start, cur));
    }
    docs
}

fn check_doc(
    source: &str,
    line: usize,
    doc: &Value,
    opts: &Options,
    results: &mut Vec<ResourceResult>,
) {
    let s = |k: &str| {
        doc.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string()
    };
    let (av, kind) = (s("apiVersion"), s("kind"));
    let name = doc
        .get("metadata")
        .and_then(|m| m.get("name"))
        .and_then(|n| n.as_str())
        .unwrap_or("")
        .to_string();
    let mk = |status, messages: Vec<String>| ResourceResult {
        source: source.to_string(),
        line,
        api_version: av.clone(),
        kind: kind.clone(),
        name: name.clone(),
        status,
        messages,
    };
    if av.is_empty() || kind.is_empty() {
        results.push(mk(
            Status::Invalid,
            vec!["a manifest needs `apiVersion` and `kind`".into()],
        ));
        return;
    }
    if kind == "List" && av == "v1" {
        match doc.get("items").and_then(|i| i.as_array()) {
            Some(items) => {
                for it in items {
                    check_doc(source, line, it, opts, results);
                }
            }
            None => results.push(mk(Status::Invalid, vec!["List without `items`".into()])),
        }
        return;
    }
    if opts.skip.iter().any(|k| k.eq_ignore_ascii_case(&kind)) {
        results.push(mk(Status::Skipped, vec!["skipped by --skip".into()]));
        return;
    }
    match roundtrip(&av, &kind, doc) {
        Some(Ok(rt)) => {
            let mut msgs = Vec::new();
            missing_required(doc, &rt, "", &mut msgs);
            if opts.strict {
                unknown_fields(doc, &rt, "", &mut msgs);
            }
            let status = if msgs.is_empty() {
                Status::Valid
            } else {
                Status::Invalid
            };
            results.push(mk(status, msgs));
        }
        Some(Err(e)) => results.push(mk(Status::Invalid, vec![e])),
        None => {
            if let Some((expected, _)) = KINDS
                .iter()
                .find(|(_, k)| *k == kind)
                .map(|(a, k)| (*a, *k))
            {
                results.push(mk(
                    Status::Invalid,
                    vec![format!(
                        "apiVersion `{av}` is not served for {kind} (use `{expected}`)"
                    )],
                ));
            } else if opts.ignore_missing_schemas {
                results.push(mk(
                    Status::Skipped,
                    vec!["no bundled schema (custom resource)".into()],
                ));
            } else {
                results.push(mk(
                    Status::Error,
                    vec![format!(
                        "no bundled schema for {av} {kind}; pass --ignore-missing-schemas to skip custom resources"
                    )],
                ));
            }
        }
    }
}

/// Validate one named manifest stream, appending to `results`.
fn check_source(source: &str, text: &str, opts: &Options, results: &mut Vec<ResourceResult>) {
    for (line, chunk) in split_docs(text) {
        if results.len() >= MAX_DOCS {
            return;
        }
        let parsed = speclint::guarded_parse(&chunk)
            .map_err(|d| d.message)
            .and_then(|y| serde_json::to_value(&y).map_err(|e| format!("unsupported YAML: {e}")));
        match parsed {
            Ok(Value::Null) => {}
            Ok(v) if v.is_object() => check_doc(source, line, &v, opts, results),
            Ok(_) => results.push(ResourceResult {
                source: source.to_string(),
                line,
                api_version: String::new(),
                kind: String::new(),
                name: String::new(),
                status: Status::Invalid,
                messages: vec!["document is not a mapping".into()],
            }),
            Err(e) => results.push(ResourceResult {
                source: source.to_string(),
                line,
                api_version: String::new(),
                kind: String::new(),
                name: String::new(),
                status: Status::Error,
                messages: vec![format!("parse error: {e}")],
            }),
        }
    }
}

/// Validate several `(source name, text)` inputs.
pub fn check(inputs: &[(String, String)], opts: &Options) -> Result<CheckReport, String> {
    let total: usize = inputs.iter().map(|(_, t)| t.len()).sum();
    if total > MAX_BYTES {
        return Err(format!("manifests exceed {MAX_BYTES} bytes"));
    }
    let mut results = Vec::new();
    for (name, text) in inputs {
        check_source(name, text, opts, &mut results);
    }
    if results.len() >= MAX_DOCS {
        return Err(format!("more than {MAX_DOCS} documents"));
    }
    let count = |st: Status| results.iter().filter(|r| r.status == st).count();
    let summary = Summary {
        valid: count(Status::Valid),
        invalid: count(Status::Invalid),
        skipped: count(Status::Skipped),
        errors: count(Status::Error),
    };
    Ok(CheckReport { results, summary })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEPLOY: &str = "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: web\nspec:\n  replicas: 2\n  selector:\n    matchLabels:\n      app: web\n  template:\n    metadata:\n      labels:\n        app: web\n    spec:\n      containers:\n        - name: web\n          image: nginx:1.27\n          ports:\n            - containerPort: 80\n          resources:\n            limits:\n              memory: 128Mi\n";

    fn run(text: &str, opts: Options) -> CheckReport {
        check(&[("m.yaml".into(), text.into())], &opts).unwrap()
    }

    #[test]
    fn valid_deployment_passes_even_strict() {
        let r = run(
            DEPLOY,
            Options {
                strict: true,
                ..Default::default()
            },
        );
        assert_eq!(r.summary.valid, 1, "{}", r.render_text());
        assert!(!r.failed());
    }

    #[test]
    fn wrong_type_and_missing_required_field_are_invalid() {
        let r = run(
            &DEPLOY.replace("replicas: 2", "replicas: many"),
            Options::default(),
        );
        assert_eq!(r.summary.invalid, 1);
        let r = run(
            &DEPLOY.replace("          image: nginx:1.27\n", ""),
            Options::default(),
        );
        assert_eq!(r.summary.invalid, 0, "image is optional in the schema");
        let r = run(
            &DEPLOY.replace("  selector:\n    matchLabels:\n      app: web\n", ""),
            Options::default(),
        );
        assert!(
            r.results[0]
                .messages
                .iter()
                .any(|m| m.contains("missing required field `spec.selector`")),
            "{:?}",
            r.results
        );
        let r = run(
            &DEPLOY.replace("        - name: web\n          image", "        - image"),
            Options::default(),
        );
        assert_eq!(
            r.summary.invalid,
            1,
            "container name is required: {}",
            r.render_text()
        );
        assert!(
            r.results[0].messages[0].contains("containers[0].name"),
            "{:?}",
            r.results[0].messages
        );
    }

    #[test]
    fn strict_reports_unknown_fields_with_paths() {
        let typo = DEPLOY.replace(
            "          image:",
            "          imagePullPolcy: Always\n          image:",
        );
        let lenient = run(&typo, Options::default());
        assert_eq!(lenient.summary.valid, 1);
        let strict = run(
            &typo,
            Options {
                strict: true,
                ..Default::default()
            },
        );
        assert_eq!(strict.summary.invalid, 1);
        assert!(
            strict.results[0].messages[0]
                .contains("spec.template.spec.containers[0].imagePullPolcy"),
            "{:?}",
            strict.results[0].messages
        );
    }

    #[test]
    fn multi_doc_lines_and_empty_docs() {
        let text = format!("# header\n---\n{DEPLOY}---\n---\napiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: c\ndata:\n  a: b\n");
        let r = run(&text, Options::default());
        assert_eq!(r.results.len(), 2);
        assert_eq!(r.results[0].line, 3);
        assert_eq!(r.results[1].kind, "ConfigMap");
        assert!(r.results[1].line > r.results[0].line);
    }

    #[test]
    fn deprecated_api_version_is_named() {
        let ing = "apiVersion: extensions/v1beta1\nkind: Ingress\nmetadata:\n  name: i\n";
        let r = run(ing, Options::default());
        assert_eq!(r.results[0].status, Status::Invalid);
        assert!(r.results[0].messages[0].contains("networking.k8s.io/v1"));
    }

    #[test]
    fn custom_resources_need_the_flag() {
        let vm = "apiVersion: kubevirt.io/v1\nkind: VirtualMachine\nmetadata:\n  name: vm\n";
        let r = run(vm, Options::default());
        assert_eq!(r.results[0].status, Status::Error);
        assert!(r.failed());
        let r = run(
            vm,
            Options {
                ignore_missing_schemas: true,
                ..Default::default()
            },
        );
        assert_eq!(r.results[0].status, Status::Skipped);
        assert!(!r.failed());
        let r = run(
            vm,
            Options {
                skip: vec!["virtualmachine".into()],
                ..Default::default()
            },
        );
        assert_eq!(r.results[0].status, Status::Skipped);
    }

    #[test]
    fn secret_data_must_be_base64() {
        let s =
            "apiVersion: v1\nkind: Secret\nmetadata:\n  name: s\ndata:\n  k: \"not base64!!\"\n";
        assert_eq!(
            run(s, Options::default()).results[0].status,
            Status::Invalid
        );
        let ok = "apiVersion: v1\nkind: Secret\nmetadata:\n  name: s\ndata:\n  k: aHVudGVyMg==\nstringData:\n  x: y\n";
        assert_eq!(
            run(
                ok,
                Options {
                    strict: true,
                    ..Default::default()
                }
            )
            .results[0]
                .status,
            Status::Valid
        );
    }

    #[test]
    fn list_kind_is_expanded() {
        let l = "apiVersion: v1\nkind: List\nitems:\n  - apiVersion: v1\n    kind: ConfigMap\n    metadata:\n      name: a\n  - apiVersion: v1\n    kind: ConfigMap\n    metadata:\n      name: b\n";
        assert_eq!(run(l, Options::default()).summary.valid, 2);
    }

    #[test]
    fn missing_kind_duplicate_keys_and_non_mappings() {
        assert_eq!(
            run("metadata:\n  name: x\n", Options::default())
                .summary
                .invalid,
            1
        );
        let dup = "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: a\n  name: b\n";
        let r = run(dup, Options::default());
        assert_eq!(r.results[0].status, Status::Error, "{:?}", r.results);
        assert_eq!(run("- a\n- b\n", Options::default()).summary.invalid, 1);
    }

    #[test]
    fn junit_and_text_escape_hostile_names() {
        let c = "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: \"a<b>&\"\ndata: 5\n";
        let r = run(c, Options::default());
        let x = r.render_junit();
        assert!(
            x.contains("a&lt;b&gt;&amp;") && x.contains("<failure"),
            "{x}"
        );
        assert!(r.render_text().contains("Summary: 0 valid, 1 invalid"));
    }

    #[test]
    fn size_limits_are_enforced() {
        let big = "a".repeat(MAX_BYTES + 1);
        assert!(check(&[("x".into(), big)], &Options::default()).is_err());
    }

    #[test]
    fn every_listed_kind_has_a_schema_entry() {
        for (av, k) in KINDS {
            let v = serde_json::json!({"apiVersion": av, "kind": k});
            assert!(roundtrip(av, k, &v).is_some(), "{av} {k}");
        }
    }
}
