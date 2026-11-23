// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Golden-file tests for the Kubernetes and KubeVirt renderers.
//!
//! Each case renders a shipped example spec and compares the YAML with a
//! reviewed file under `tests/golden/`. A renderer change (or a dependency bump
//! such as kube / k8s-openapi) that alters output fails with a line diff.
//! Regenerate intentionally with `AETHER_UPDATE_GOLDEN=1 cargo test golden`.

use super::kube_manifest::{build_deployment_manifest_v2, build_service_manifest};
use super::kubevirt::build_virtualmachine_json;
use crate::runtime::{Image, RuntimeKind};
use crate::spec::Workload;
use std::path::{Path, PathBuf};

fn repo_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn load(rel: &str) -> Workload {
    let text = std::fs::read_to_string(repo_path(rel)).unwrap();
    serde_yaml::from_str::<Workload>(&text).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn image(spec: &Workload) -> Image {
    Image {
        name: spec.image_name(),
        tag: "latest".to_string(),
        digest: None,
        runtime: RuntimeKind::Kubernetes,
    }
}

/// First differing line, with context, for a readable failure.
fn first_diff(want: &str, got: &str) -> String {
    for (i, (w, g)) in want.lines().zip(got.lines()).enumerate() {
        if w != g {
            return format!("line {}:\n  golden: {w}\n  actual: {g}", i + 1);
        }
    }
    format!(
        "length differs: golden has {} lines, actual has {}",
        want.lines().count(),
        got.lines().count()
    )
}

/// Compare `actual` with `tests/golden/<name>`. `name` must be a plain file name.
fn assert_golden(name: &str, actual: &str) {
    assert!(
        !name.is_empty() && !name.contains('/') && !name.contains('\\') && !name.contains(".."),
        "golden name must be a plain file name: {name}"
    );
    let path = repo_path(&format!("tests/golden/{name}"));
    if std::env::var("AETHER_UPDATE_GOLDEN").is_ok_and(|v| v == "1") {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let want = std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!(
            "missing golden file {}; create it with AETHER_UPDATE_GOLDEN=1 cargo test golden",
            path.display()
        )
    });
    assert!(
        want == actual,
        "rendered output differs from {} ({}); if intended, rerun with AETHER_UPDATE_GOLDEN=1",
        path.display(),
        first_diff(&want, actual)
    );
}

fn yaml<T: serde::Serialize>(v: &T) -> String {
    serde_yaml::to_string(v).unwrap()
}

#[test]
fn golden_kube_deployment_demo_webserver() {
    let spec = load("examples/demo-webserver.yaml");
    let d = build_deployment_manifest_v2("aether-demo", &image(&spec), &spec);
    assert_golden("kube-deployment-demo-webserver.yaml", &yaml(&d));
}

#[test]
fn golden_kube_service_demo_webserver() {
    let spec = load("examples/demo-webserver.yaml");
    let s = build_service_manifest("aether-demo", &spec).expect("service");
    assert_golden("kube-service-demo-webserver.yaml", &yaml(&s));
}

#[test]
fn golden_kube_deployment_k8s_advanced() {
    let spec = load("examples/workload-k8s-advanced.yaml");
    let d = build_deployment_manifest_v2("aether-demo", &image(&spec), &spec);
    assert_golden("kube-deployment-k8s-advanced.yaml", &yaml(&d));
}

#[test]
fn golden_kubevirt_vm_lab() {
    let spec = load("examples/labs/kubevirt/workload.yaml");
    let vm = build_virtualmachine_json("labs", &spec);
    assert_golden("kubevirt-vm-lab.yaml", &yaml(&vm));
}

#[test]
fn golden_names_must_be_plain_file_names() {
    for bad in ["../x", "a/b", "..", "", "a\\b"] {
        let r = std::panic::catch_unwind(|| assert_golden(bad, "x"));
        assert!(r.is_err(), "{bad:?} should be rejected");
    }
}

#[test]
fn golden_diff_pinpoints_first_changed_line() {
    assert_eq!(
        first_diff("a\nb\nc\n", "a\nX\nc\n"),
        "line 2:\n  golden: b\n  actual: X"
    );
    assert!(first_diff("a\n", "a\nb\n").contains("length differs"));
}
