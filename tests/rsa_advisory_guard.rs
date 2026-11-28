// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Limits the RUSTSEC-2023-0071 ignore in `.cargo/audit.toml` to the case it
//! was written for. If one of these fails, read that file before changing it.

use std::fs;
use std::path::Path;

fn quoted(line: &str) -> Option<&str> {
    line.split('"').nth(1)
}

/// Releases the advisory covers: everything before 0.10.0.
fn is_affected(version: &str) -> bool {
    let mut parts = version.split('.');
    let major = parts.next();
    let minor = parts.next().and_then(|m| m.parse::<u32>().ok());
    major == Some("0") && (minor.is_some_and(|m| m < 10) || version.starts_with("0.10.0-"))
}

#[test]
fn only_openidconnect_depends_on_the_affected_rsa() {
    let lock = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock"))
        .expect("read Cargo.lock");
    let packages: Vec<&str> = lock.split("[[package]]").skip(1).collect();
    let field = |package: &str, key: &str| {
        package
            .lines()
            .find(|line| line.starts_with(key))
            .and_then(quoted)
            .map(str::to_string)
    };

    let rsa_versions: Vec<String> = packages
        .iter()
        .filter(|p| field(p, "name = ").as_deref() == Some("rsa"))
        .filter_map(|p| field(p, "version = "))
        .collect();

    let mut dependents = Vec::new();
    for package in &packages {
        // Dependency entries are the indented lines. Cargo writes "rsa 0.9.10"
        // when several versions are locked and plain "rsa" when there is one.
        let uses_affected = package
            .lines()
            .filter(|line| line.starts_with(" \""))
            .filter_map(quoted)
            .any(|entry| match entry.split_once(' ') {
                Some((name, version)) => name == "rsa" && is_affected(version),
                None => entry == "rsa" && rsa_versions.iter().any(|v| is_affected(v)),
            });
        if uses_affected {
            dependents.extend(field(package, "name = "));
        }
    }

    assert_eq!(
        dependents,
        ["openidconnect"],
        "the RUSTSEC-2023-0071 ignore covers openidconnect only; if the list is empty, \
         delete the ignore and this test"
    );
}

#[test]
fn aether_never_signs_with_the_affected_rsa() {
    fn visit(dir: &Path, hits: &mut Vec<String>) {
        for entry in fs::read_dir(dir).expect("read source directory") {
            let path = entry.expect("read source entry").path();
            if path.is_dir() {
                visit(&path, hits);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let source = fs::read_to_string(&path).expect("read source file");
                if source.contains("CoreRsaPrivateSigningKey") {
                    hits.push(path.display().to_string());
                }
            }
        }
    }

    let mut hits = Vec::new();
    visit(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut hits,
    );
    assert!(
        hits.is_empty(),
        "openidconnect's RSA signing key uses the affected rsa release: {hits:?}"
    );
}
