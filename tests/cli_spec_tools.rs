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
