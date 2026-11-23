// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Every spec shipped for validation in CI must lint without errors and without
//! silently-ignored fields. This is the regression test for keys such as
//! `serviceType`, `buildArgs` and `configMaps`, which used to be dropped.

use aether::speclint::lint_str;
use std::path::Path;

const SHIPPED: &[&str] = &[
    "examples/demo-webserver.yaml",
    "examples/workload-full-featured.yaml",
    "examples/workload-k8s-advanced.yaml",
    "examples/labs/kubevirt/workload.yaml",
    "examples/labs/kubernetes/workload.yaml",
    "workload.yaml",
    "workload-k8s.yaml",
    "workload-kubevirt.yaml",
];

#[test]
fn shipped_specs_lint_clean() {
    for rel in SHIPPED {
        let text = std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
            .unwrap_or_else(|e| panic!("{rel}: {e}"));
        let report = lint_str(&text);
        let bad: Vec<_> = report
            .diagnostics
            .iter()
            .filter(|d| d.code.starts_with('E') || d.code == "W020")
            .collect();
        assert!(bad.is_empty(), "{rel}: {bad:#?}");
    }
}
