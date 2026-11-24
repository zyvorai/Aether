// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! CI and pre-commit integration snippets (`aether spec-ci`).
//!
//! Prints a ready-to-paste configuration that runs the spec tools (lint with
//! best practices, spec-lock drift check) in GitHub Actions, GitLab CI or
//! pre-commit. The spec path is restricted to a conservative character set so
//! it can never inject YAML or shell syntax into the snippet.

pub const PROVIDERS: &[&str] = &["github", "gitlab", "pre-commit"];

/// Accept only `[A-Za-z0-9._/-]`, no leading `-` and no `..` segments.
pub fn safe_path(p: &str) -> Result<&str, String> {
    let ok = !p.is_empty()
        && p.len() <= 200
        && !p.starts_with('-')
        && !p.split('/').any(|s| s == "..")
        && p.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'));
    if ok {
        Ok(p)
    } else {
        Err(format!(
            "unsupported spec path `{}` (use letters, digits, `.`, `_`, `-`, `/`)",
            crate::speclint::sanitize(p)
        ))
    }
}

pub fn snippet(provider: &str, spec: &str) -> Result<String, String> {
    let spec = safe_path(spec)?;
    let lock = format!("{spec}.lock.json");
    match provider {
        "github" => Ok(format!(
            "name: aether-spec\non:\n  pull_request:\n    paths: ['{spec}']\njobs:\n  spec:\n    runs-on: ubuntu-latest\n    permissions:\n      contents: read\n      security-events: write\n    steps:\n      - uses: actions/checkout@v7\n      # Install the aether binary first (release download or cargo install).\n      - name: Lint (annotations)\n        run: aether --spec {spec} lint --best-practices --format github --deny-warnings\n      - name: Lint (SARIF for code scanning)\n        if: always()\n        run: aether --spec {spec} lint --best-practices --format sarif > aether.sarif\n      - uses: github/codeql-action/upload-sarif@v4\n        if: always()\n        with:\n          sarif_file: aether.sarif\n      - name: Spec drift against the committed lock\n        run: aether --spec {spec} spec-lock check --lock {lock}\n"
        )),
        "gitlab" => Ok(format!(
            "aether-spec:\n  stage: test\n  rules:\n    - changes: ['{spec}']\n  script:\n    - aether --spec {spec} lint --best-practices --format junit --deny-warnings > aether-junit.xml\n    - aether --spec {spec} spec-lock check --lock {lock}\n  artifacts:\n    when: always\n    reports:\n      junit: aether-junit.xml\n"
        )),
        "pre-commit" => Ok(format!(
            "repos:\n  - repo: local\n    hooks:\n      - id: aether-lint\n        name: aether lint\n        entry: aether --spec {spec} lint --deny-warnings\n        language: system\n        pass_filenames: false\n        files: '^{pattern}$'\n      - id: aether-spec-lock\n        name: aether spec-lock check\n        entry: aether --spec {spec} spec-lock check --lock {lock}\n        language: system\n        pass_filenames: false\n        files: '^{pattern}$'\n",
            pattern = spec.replace('.', "\\.")
        )),
        other => Err(format!(
            "unknown provider `{}` (expected one of: {})",
            crate::speclint::sanitize(other),
            PROVIDERS.join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_provider_yields_valid_yaml_with_the_commands() {
        for p in PROVIDERS {
            let s = snippet(p, "deploy/workload.yaml").unwrap();
            let v: serde_yaml::Value =
                serde_yaml::from_str(&s).unwrap_or_else(|e| panic!("{p}: {e}\n{s}"));
            assert!(v.is_mapping(), "{p}");
            assert!(s.contains("aether --spec deploy/workload.yaml lint"), "{p}");
            assert!(
                s.contains("spec-lock check --lock deploy/workload.yaml.lock.json"),
                "{p}"
            );
        }
    }

    #[test]
    fn pre_commit_pattern_escapes_dots() {
        let s = snippet("pre-commit", "a.yaml").unwrap();
        assert!(s.contains("files: '^a\\.yaml$'"), "{s}");
    }

    #[test]
    fn hostile_paths_and_providers_are_rejected() {
        for p in [
            "",
            "-x",
            "../x",
            "a b",
            "a;rm -rf /",
            "a\nb",
            "x'y",
            "a$(b)",
        ] {
            assert!(snippet("github", p).is_err(), "{p:?}");
        }
        assert!(snippet("jenkins", "a.yaml")
            .unwrap_err()
            .contains("expected one of"));
    }
}
