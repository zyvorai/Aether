// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! CycloneDX SBOM generation from Cargo.lock and release artifacts.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

const CARGO_LOCK: &str = include_str!("../Cargo.lock");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SbomMetadata {
    pub bom_format: String,
    pub spec_version: String,
    pub version: u32,
    pub serial_number: String,
    pub component_count: usize,
    pub aether_version: String,
    pub binary_sha256: Option<String>,
    pub dashboard_sha256: Option<String>,
}

/// Build CycloneDX 1.5 JSON SBOM from embedded Cargo.lock.
pub fn generate_cyclonedx(binary_path: Option<&Path>) -> Result<Value> {
    let components = parse_cargo_lock_components(CARGO_LOCK)?;
    let aether_version = env!("CARGO_PKG_VERSION").to_string();
    let binary_sha256 = binary_path.and_then(hash_file);
    let dashboard_sha256 = hash_file(&PathBuf::from("web/dashboard/dist/assets/aether-dashboard.js"));

    let mut metadata_component = json!({
        "type": "application",
        "name": "aether",
        "version": aether_version,
        "bom-ref": "aether-control-plane"
    });
    if let Some(ref h) = binary_sha256 {
        metadata_component["hashes"] = json!([{
            "alg": "SHA-256",
            "content": h
        }]);
    }

    let bom = json!({
        "bomFormat": "CycloneDX",
        "specVersion": "1.5",
        "version": 1,
        "serialNumber": format!("urn:uuid:aether-sbom-{}", aether_version),
        "metadata": {
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "component": metadata_component,
            "properties": [
                {"name": "dashboard.bundle.sha256", "value": dashboard_sha256.clone().unwrap_or_default()},
                {"name": "embedded.ui.build", "value": env!("AETHER_EMBEDDED_UI_BUILD")}
            ]
        },
        "components": components
    });
    Ok(bom)
}

pub fn sbom_metadata(bom: &Value) -> SbomMetadata {
    let components = bom.get("components").and_then(|c| c.as_array());
    SbomMetadata {
        bom_format: bom
            .get("bomFormat")
            .and_then(|v| v.as_str())
            .unwrap_or("CycloneDX")
            .to_string(),
        spec_version: bom
            .get("specVersion")
            .and_then(|v| v.as_str())
            .unwrap_or("1.5")
            .to_string(),
        version: bom.get("version").and_then(|v| v.as_u64()).unwrap_or(1) as u32,
        serial_number: bom
            .get("serialNumber")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        component_count: components.map(|a| a.len()).unwrap_or(0),
        aether_version: env!("CARGO_PKG_VERSION").to_string(),
        binary_sha256: bom
            .pointer("/metadata/component/hashes/0/content")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        dashboard_sha256: bom
            .pointer("/metadata/properties/0/value")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
    }
}

pub fn export_to_path(output: &Path, binary_path: Option<&Path>) -> Result<PathBuf> {
    let bom = generate_cyclonedx(binary_path)?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(&bom)?;
    std::fs::write(output, &text)?;
    let cache = crate::resources::aether_path("sbom.json");
    if let Some(parent) = cache.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&cache, &text);
    Ok(output.to_path_buf())
}

pub fn load_cached() -> Option<Value> {
    let cache = crate::resources::aether_path("sbom.json");
    let text = std::fs::read_to_string(cache).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn verify_file(path: &Path) -> Result<bool> {
    let text = std::fs::read_to_string(path).context("read SBOM file")?;
    let bom: Value = serde_json::from_str(&text).context("parse SBOM JSON")?;
    Ok(bom.get("bomFormat").and_then(|v| v.as_str()) == Some("CycloneDX"))
}

fn hash_file(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    Some(format!("{:x}", Sha256::digest(&bytes)))
}

fn parse_cargo_lock_components(lock: &str) -> Result<Vec<Value>> {
    let mut components = Vec::new();
    let mut current_name: Option<String> = None;
    let mut current_version: Option<String> = None;

    for line in lock.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("name = ") {
            current_name = Some(trimmed.trim_start_matches("name = ").trim_matches('"').to_string());
        } else if trimmed.starts_with("version = ") {
            current_version = Some(trimmed.trim_start_matches("version = ").trim_matches('"').to_string());
        } else if trimmed == "[[package]]" || trimmed.is_empty() {
            if let (Some(name), Some(version)) = (current_name.take(), current_version.take()) {
                if name != "aether" {
                    components.push(json!({
                        "type": "library",
                        "name": name,
                        "version": version,
                        "purl": format!("pkg:cargo/{}@{}", name, version),
                        "bom-ref": format!("pkg:cargo/{}@{}", name, version)
                    }));
                }
            }
        }
    }
    if let (Some(name), Some(version)) = (current_name, current_version) {
        if name != "aether" {
            components.push(json!({
                "type": "library",
                "name": name,
                "version": version,
                "purl": format!("pkg:cargo/{}@{}", name, version),
                "bom-ref": format!("pkg:cargo/{}@{}", name, version)
            }));
        }
    }
    components.sort_by(|a, b| {
        let an = a.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let bn = b.get("name").and_then(|v| v.as_str()).unwrap_or("");
        an.cmp(bn)
    });
    components.dedup_by(|a, b| a.get("bom-ref") == b.get("bom-ref"));
    Ok(components)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_cyclonedx_with_components() {
        let bom = generate_cyclonedx(None).unwrap();
        assert_eq!(bom["bomFormat"], "CycloneDX");
        let comps = bom["components"].as_array().unwrap();
        assert!(comps.len() > 10);
        assert!(comps.iter().any(|c| c["name"] == "tokio"));
    }

    #[test]
    fn parses_cargo_lock_packages() {
        let comps = parse_cargo_lock_components(CARGO_LOCK).unwrap();
        assert!(comps.iter().any(|c| c["name"] == "serde"));
    }
}
