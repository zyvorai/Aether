//! CLI-level tests for the spec tooling: `aether lint` formats, suppression and
//! explanations, `aether spec-merge` and `aether spec-render`.
//!
//! These run the real binary on temporary spec files and check stdout and the
//! exit status.

use std::path::Path;
use std::process::{Command, Output};

const GOOD: &str = "apiVersion: aether/v1\nkind: Workload\nmetadata:\n  name: web\n  owner: me\n  project: p\nbuild:\n  context: .\n  dockerfile: Dockerfile\n  registry: localhost\nrequirements:\n  cpu: \"500m\"\n  memory: 256Mi\n  storage: 1Gi\nruntime:\n  preferred: kube\n  allow:\n    - kube\nnetwork:\n  service: true\n  ports:\n    - containerPort: 80\n      servicePort: 80\n";

fn aether(home: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aether"))
        .args(args)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .output()
        .expect("run aether")
}

fn write(dir: &Path, name: &str, body: &str) -> String {
    let p = dir.join(name);
    std::fs::write(&p, body).unwrap();
    p.to_string_lossy().into_owned()
}

fn out(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}

#[test]
fn lint_sarif_output_and_exit_codes() {
    let d = tempfile::tempdir().unwrap();
    let typo = GOOD.replace("service: true", "servicetype: NodePort");
    let spec = write(d.path(), "s.yaml", &typo);
    let o = aether(d.path(), &["--spec", &spec, "lint", "--format", "sarif"]);
    assert!(o.status.success(), "warnings alone do not fail");
    let v: serde_json::Value = serde_json::from_str(&out(&o)).expect("valid JSON");
    assert_eq!(v["version"], "2.1.0");
    assert_eq!(v["runs"][0]["results"][0]["ruleId"], "W020");

    let o = aether(d.path(), &["--spec", &spec, "lint", "--deny-warnings"]);
    assert_eq!(o.status.code(), Some(1));
}

#[test]
fn lint_ignore_flag_and_inline_directive() {
    let d = tempfile::tempdir().unwrap();
    let typo = GOOD.replace("service: true", "servicetype: NodePort");
    let spec = write(d.path(), "s.yaml", &typo);
    let o = aether(
        d.path(),
        &[
            "--spec",
            &spec,
            "lint",
            "--ignore",
            "W020",
            "--deny-warnings",
        ],
    );
    assert!(o.status.success(), "{}", out(&o));
    assert!(out(&o).contains("1 suppressed"));

    let inline = typo.replace(
        "  servicetype: NodePort\n",
        "  servicetype: NodePort # aether-lint: ignore=W020\n",
    );
    let spec = write(d.path(), "i.yaml", &inline);
    let o = aether(d.path(), &["--spec", &spec, "lint", "--deny-warnings"]);
    assert!(o.status.success(), "{}", out(&o));
}

#[test]
fn lint_best_practices_junit_and_github() {
    let d = tempfile::tempdir().unwrap();
    let spec = write(d.path(), "s.yaml", GOOD);
    let o = aether(
        d.path(),
        &[
            "--spec",
            &spec,
            "lint",
            "--best-practices",
            "--format",
            "junit",
        ],
    );
    let x = out(&o);
    assert!(x.starts_with("<?xml"), "{x}");
    assert!(x.contains("B101") && x.contains("<failure type=\"warning\""));
    assert!(o.status.success());

    let o = aether(
        d.path(),
        &[
            "--spec",
            &spec,
            "lint",
            "--best-practices",
            "--format",
            "github",
        ],
    );
    assert!(out(&o).contains("::warning file="));

    let o = aether(d.path(), &["--spec", &spec, "lint", "--format", "yamlish"]);
    assert!(!o.status.success());
}

#[test]
fn lint_explain_and_list_rules() {
    let d = tempfile::tempdir().unwrap();
    let spec = write(d.path(), "s.yaml", GOOD);
    let o = aether(d.path(), &["--spec", &spec, "lint", "--explain", "w020"]);
    assert!(o.status.success());
    assert!(out(&o).contains("Unknown field"));
    let o = aether(d.path(), &["--spec", &spec, "lint", "--explain", "Z999"]);
    assert!(!o.status.success());
    let o = aether(d.path(), &["--spec", &spec, "lint", "--list-rules"]);
    let t = out(&o);
    assert!(t.contains("E010") && t.contains("B109") && t.contains("[--best-practices]"));
}

#[test]
fn spec_merge_overlay_and_set() {
    let d = tempfile::tempdir().unwrap();
    let base = write(d.path(), "base.yaml", GOOD);
    let overlay = write(
        d.path(),
        "prod.yaml",
        "requirements:\n  memory: 2Gi\nmetadata:\n  labels:\n    env: prod\n",
    );
    let o = aether(
        d.path(),
        &[
            "--spec",
            &base,
            "spec-merge",
            &overlay,
            "--set",
            "network.ports[0].servicePort=8080",
        ],
    );
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let merged: serde_yaml::Value = serde_yaml::from_str(&out(&o)).unwrap();
    assert_eq!(
        merged["requirements"]["memory"],
        serde_yaml::Value::String("2Gi".into())
    );
    assert_eq!(
        merged["metadata"]["labels"]["env"],
        serde_yaml::Value::String("prod".into())
    );
    assert_eq!(
        merged["network"]["ports"][0]["servicePort"],
        serde_yaml::Value::Number(8080.into())
    );

    // -o writes the file and keeps stdout clean.
    let dest = d.path().join("out.yaml");
    let o = aether(
        d.path(),
        &[
            "--spec",
            &base,
            "spec-merge",
            &overlay,
            "-o",
            dest.to_str().unwrap(),
        ],
    );
    assert!(o.status.success());
    assert!(std::fs::read_to_string(&dest).unwrap().contains("2Gi"));
}

#[test]
fn spec_merge_fails_on_lint_errors_and_bad_input() {
    let d = tempfile::tempdir().unwrap();
    let base = write(d.path(), "base.yaml", GOOD);
    let o = aether(
        d.path(),
        &[
            "--spec",
            &base,
            "spec-merge",
            "--set",
            "network.ports[0].containerPort=0",
        ],
    );
    assert_eq!(o.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&o.stderr).contains("E032"));
    let o = aether(d.path(), &["--spec", &base, "spec-merge", "--set", "bogus"]);
    assert!(!o.status.success());
}

#[test]
fn spec_render_hash_is_stable_across_formatting() {
    let d = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.yaml", GOOD);
    let b = write(
        d.path(),
        "b.yaml",
        &format!(
            "# comment\n{}",
            GOOD.replace("  owner: me\n  project: p\n", "  project: p\n  owner: me\n")
        ),
    );
    let ha = out(&aether(d.path(), &["--spec", &a, "spec-render", "--hash"]));
    let hb = out(&aether(d.path(), &["--spec", &b, "spec-render", "--hash"]));
    assert!(ha.trim().starts_with("sha256:"));
    assert_eq!(ha, hb);
    let c = write(d.path(), "c.yaml", &GOOD.replace("256Mi", "512Mi"));
    let hc = out(&aether(d.path(), &["--spec", &c, "spec-render", "--hash"]));
    assert_ne!(ha, hc);
}

#[test]
fn spec_render_json_and_yaml() {
    let d = tempfile::tempdir().unwrap();
    let a = write(d.path(), "a.yaml", GOOD);
    let o = aether(d.path(), &["--spec", &a, "spec-render", "--format", "json"]);
    let v: serde_json::Value = serde_json::from_str(&out(&o)).unwrap();
    assert!(v["sections"]["requirements"]
        .as_str()
        .unwrap()
        .starts_with("sha256:"));
    assert_eq!(v["spec"]["metadata"]["name"], "web");
    let o = aether(d.path(), &["--spec", &a, "spec-render"]);
    assert!(out(&o).starts_with("# hash: sha256:"));
    let o = aether(d.path(), &["--spec", &a, "spec-render", "--format", "toml"]);
    assert!(!o.status.success());
    let bad = write(d.path(), "bad.yaml", "- a\n");
    assert!(!aether(d.path(), &["--spec", &bad, "spec-render"])
        .status
        .success());
}

// ---------------------------------------------------------------------------
// lint --fix, spec-policy, spec-lock, manifest-check, spec-ci
// ---------------------------------------------------------------------------

#[test]
fn lint_fix_rewrites_the_file_and_diff_changes_nothing() {
    let d = tempfile::tempdir().unwrap();
    let typo = GOOD.replace("service: true", "servicetype: NodePort # keep");
    let spec = write(d.path(), "s.yaml", &typo);

    let o = aether(d.path(), &["--spec", &spec, "lint", "--diff"]);
    assert!(o.status.success());
    assert!(
        out(&o).contains("-  servicetype: NodePort # keep"),
        "{}",
        out(&o)
    );
    assert_eq!(
        std::fs::read_to_string(&spec).unwrap(),
        typo,
        "--diff must not write"
    );

    let o = aether(
        d.path(),
        &["--spec", &spec, "lint", "--fix", "--deny-warnings"],
    );
    assert!(
        o.status.success(),
        "{}{}",
        out(&o),
        String::from_utf8_lossy(&o.stderr)
    );
    let fixed = std::fs::read_to_string(&spec).unwrap();
    assert!(fixed.contains("serviceType: NodePort # keep"));
    assert!(String::from_utf8_lossy(&o.stderr).contains("applied 1 fix(es)"));

    let o = aether(d.path(), &["--spec", &spec, "lint", "--fix"]);
    assert!(
        !String::from_utf8_lossy(&o.stderr).contains("applied"),
        "idempotent"
    );
}

const RULES: &str = "rules:\n  - {id: P1, path: metadata.team, op: exists, message: team required}\n  - {id: P2, severity: warning, path: 'network.ports[*].containerPort', op: gte, value: 1024}\n";

#[test]
fn spec_policy_formats_and_exit_codes() {
    let d = tempfile::tempdir().unwrap();
    let spec = write(d.path(), "s.yaml", GOOD);
    let rules = write(d.path(), "r.yaml", RULES);
    let o = aether(d.path(), &["--spec", &spec, "spec-policy", &rules]);
    assert_eq!(o.status.code(), Some(1));
    let t = out(&o);
    assert!(
        t.contains("error[P1]") && t.contains("warning[P2]") && t.contains("2 rule(s), 0 passed"),
        "{t}"
    );

    let o = aether(
        d.path(),
        &["--spec", &spec, "spec-policy", &rules, "--format", "json"],
    );
    let v: serde_json::Value = serde_json::from_str(&out(&o)).expect("json");
    assert_eq!(v["errors"], 1);
    assert_eq!(v["violations"][0]["path"], "metadata.team");

    let o = aether(
        d.path(),
        &["--spec", &spec, "spec-policy", &rules, "--format", "github"],
    );
    assert!(out(&o).contains("::error file="));
    let o = aether(
        d.path(),
        &["--spec", &spec, "spec-policy", &rules, "--format", "junit"],
    );
    assert!(out(&o).starts_with("<?xml"));

    let ok = write(
        d.path(),
        "ok.yaml",
        "rules:\n  - {id: P1, path: metadata.owner, op: exists}\n",
    );
    let o = aether(d.path(), &["--spec", &spec, "spec-policy", &ok]);
    assert!(o.status.success(), "{}", out(&o));

    let bad = write(
        d.path(),
        "bad.yaml",
        "rules:\n  - {id: P1, path: x, op: nope}\n",
    );
    let o = aether(d.path(), &["--spec", &spec, "spec-policy", &bad]);
    assert!(!o.status.success());
    assert!(String::from_utf8_lossy(&o.stderr).contains("invalid rule file"));
}

#[test]
fn shipped_policy_example_loads_and_runs() {
    let d = tempfile::tempdir().unwrap();
    let spec = write(d.path(), "s.yaml", GOOD);
    let o = aether(
        d.path(),
        &[
            "--spec",
            &spec,
            "spec-policy",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/examples/policy/org-baseline.yaml"
            ),
            "--format",
            "json",
        ],
    );
    let v: serde_json::Value = serde_json::from_str(&out(&o)).expect("json");
    assert_eq!(v["rules"], 5);
    // GOOD uses port 80 (ORG004) and a non-matching registry (ORG002): both warnings, no errors.
    assert_eq!(v["errors"], 0, "{v}");
    assert_eq!(v["warnings"], 2, "{v}");
}

#[test]
fn spec_lock_write_check_and_drift_exit_code() {
    let d = tempfile::tempdir().unwrap();
    let spec = write(d.path(), "s.yaml", GOOD);
    let o = aether(d.path(), &["--spec", &spec, "spec-lock", "write"]);
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    let lock = format!("{spec}.lock.json");
    assert!(std::path::Path::new(&lock).exists());

    let o = aether(d.path(), &["--spec", &spec, "spec-lock", "check"]);
    assert_eq!(o.status.code(), Some(0));
    assert!(out(&o).contains("no drift"));

    std::fs::write(&spec, GOOD.replace("256Mi", "512Mi")).unwrap();
    let o = aether(d.path(), &["--spec", &spec, "spec-lock", "check"]);
    assert_eq!(o.status.code(), Some(2));
    assert!(out(&o).contains("changed  requirements"), "{}", out(&o));

    let o = aether(
        d.path(),
        &[
            "--spec",
            &spec,
            "spec-lock",
            "check",
            "--ignore",
            "requirements",
        ],
    );
    assert_eq!(o.status.code(), Some(0));

    std::fs::write(&lock, "garbage").unwrap();
    let o = aether(d.path(), &["--spec", &spec, "spec-lock", "check"]);
    assert_eq!(o.status.code(), Some(1));
}

const DEPLOY: &str = "apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: web\nspec:\n  selector:\n    matchLabels:\n      app: web\n  template:\n    metadata:\n      labels:\n        app: web\n    spec:\n      containers:\n        - name: web\n          image: nginx:1.27\n";

#[test]
fn manifest_check_valid_invalid_strict_and_formats() {
    let d = tempfile::tempdir().unwrap();
    let good = write(d.path(), "good.yaml", DEPLOY);
    let o = aether(d.path(), &["manifest-check", &good, "--strict"]);
    assert!(o.status.success(), "{}", out(&o));
    assert!(out(&o).contains("Summary: 1 valid, 0 invalid"));

    let typo = DEPLOY.replace(
        "          image:",
        "          imagePullPolcy: Always\n          image:",
    );
    let f = write(d.path(), "typo.yaml", &typo);
    assert!(
        aether(d.path(), &["manifest-check", &f]).status.success(),
        "lenient by default"
    );
    let o = aether(d.path(), &["manifest-check", &f, "--strict"]);
    assert_eq!(o.status.code(), Some(1));
    assert!(
        out(&o).contains("unknown field `spec.template.spec.containers[0].imagePullPolcy`"),
        "{}",
        out(&o)
    );

    let o = aether(
        d.path(),
        &["manifest-check", &f, "--strict", "--format", "junit"],
    );
    assert!(out(&o).contains("<failure message="));
    let o = aether(d.path(), &["manifest-check", &good, "--format", "json"]);
    let v: serde_json::Value = serde_json::from_str(&out(&o)).expect("json");
    assert_eq!(v["summary"]["valid"], 1);

    let vm = write(
        d.path(),
        "vm.yaml",
        "apiVersion: kubevirt.io/v1\nkind: VirtualMachine\nmetadata:\n  name: vm\n",
    );
    assert_eq!(
        aether(d.path(), &["manifest-check", &vm]).status.code(),
        Some(1)
    );
    assert!(aether(
        d.path(),
        &["manifest-check", &vm, "--ignore-missing-schemas"]
    )
    .status
    .success());

    assert!(
        !aether(d.path(), &["manifest-check"]).status.success(),
        "needs a file"
    );
    assert!(!aether(d.path(), &["manifest-check", "/nonexistent.yaml"])
        .status
        .success());
}

#[test]
fn spec_ci_prints_valid_config_and_rejects_bad_input() {
    let d = tempfile::tempdir().unwrap();
    for p in ["github", "gitlab", "pre-commit"] {
        let o = aether(d.path(), &["--spec", "deploy/workload.yaml", "spec-ci", p]);
        assert!(o.status.success(), "{p}");
        let v: serde_json::Value = serde_yaml::from_str(&out(&o)).expect("yaml");
        assert!(v.is_object());
        assert!(out(&o).contains("spec-lock check"));
    }
    assert!(!aether(d.path(), &["spec-ci", "jenkins"]).status.success());
    assert!(
        !aether(d.path(), &["--spec", "a b.yaml", "spec-ci", "github"])
            .status
            .success()
    );
}

#[test]
fn manifest_check_accepts_the_golden_rendered_manifests_strictly() {
    let d = tempfile::tempdir().unwrap();
    let root = env!("CARGO_MANIFEST_DIR");
    let files: Vec<String> = [
        "kube-deployment-demo-webserver.yaml",
        "kube-deployment-k8s-advanced.yaml",
        "kube-service-demo-webserver.yaml",
    ]
    .iter()
    .map(|f| format!("{root}/tests/golden/{f}"))
    .collect();
    let mut args = vec!["manifest-check", "--strict"];
    args.extend(files.iter().map(String::as_str));
    let o = aether(d.path(), &args);
    assert!(o.status.success(), "{}", out(&o));
    assert!(
        out(&o).contains("Summary: 3 valid, 0 invalid, 0 errors"),
        "{}",
        out(&o)
    );
}
