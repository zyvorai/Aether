// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Container-image mirroring for cross-cluster Move.
//!
//! Copies each source image (e.g. from ECR) to the target registry with `skopeo`
//! (falls back to `crane`), producing a source→target map that feeds the manifest
//! image rewrite. Commands are built with arg-arrays only, never string concat.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageMap {
    pub source: String,
    pub target: String,
}

/// Compute the target image ref: strip the source registry host, keep the
/// repository path + tag, and prefix the target registry.
pub fn target_ref(image: &str, target_registry: &str) -> String {
    let reg = target_registry.trim_end_matches('/');
    let rest = match image.split_once('/') {
        // First segment is a registry host (has a dot/port) or localhost.
        Some((first, r)) if first.contains('.') || first.contains(':') || first == "localhost" => r,
        _ => image, // docker-hub-style `repo[:tag]`
    };
    format!("{reg}/{rest}")
}

/// The mirroring tool available on this host, if any.
fn find_tool() -> Option<(&'static str, Vec<&'static str>)> {
    if which::which("skopeo").is_ok() {
        // skopeo copy docker://SRC docker://DST
        Some(("skopeo", vec!["copy"]))
    } else if which::which("crane").is_ok() {
        // crane copy SRC DST
        Some(("crane", vec!["copy"]))
    } else {
        None
    }
}

/// Plan (and optionally execute) mirroring of `images` to `target_registry`.
/// When `dry_run` is true no copy runs — only the source→target map is returned.
pub async fn mirror_images(
    images: &[String],
    target_registry: &str,
    dry_run: bool,
) -> Result<Vec<ImageMap>> {
    let mut maps = Vec::new();
    let tool = find_tool();

    for image in images {
        let target = target_ref(image, target_registry);
        maps.push(ImageMap {
            source: image.clone(),
            target: target.clone(),
        });

        if dry_run {
            continue;
        }
        let Some((bin, base_args)) = &tool else {
            anyhow::bail!(
                "no image mirroring tool found (install skopeo or crane), or use --dry-run"
            );
        };
        run_copy(bin, base_args, image, &target).await?;
    }
    Ok(maps)
}

async fn run_copy(bin: &str, base_args: &[&str], src: &str, dst: &str) -> Result<()> {
    let mut cmd = tokio::process::Command::new(bin);
    cmd.args(base_args);
    // skopeo needs docker:// transport prefixes; crane takes bare refs.
    if bin == "skopeo" {
        cmd.arg(format!("docker://{src}"));
        cmd.arg(format!("docker://{dst}"));
    } else {
        cmd.arg(src);
        cmd.arg(dst);
    }
    let out = cmd
        .output()
        .await
        .with_context(|| format!("spawning {bin}"))?;
    if !out.status.success() {
        anyhow::bail!(
            "{bin} copy {src} → {dst} failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_ref() {
        assert_eq!(
            target_ref(
                "1.dkr.ecr.us-east-1.amazonaws.com/team/api:1.2",
                "registry.zyvor.internal"
            ),
            "registry.zyvor.internal/team/api:1.2"
        );
        assert_eq!(
            target_ref("nginx:1.25", "reg.local"),
            "reg.local/nginx:1.25"
        );
        assert_eq!(target_ref("nginx", "reg.local/"), "reg.local/nginx");
        assert_eq!(
            target_ref("quay.io/prometheus/node-exporter:v1", "reg.local"),
            "reg.local/prometheus/node-exporter:v1"
        );
    }

    #[tokio::test]
    async fn test_dry_run_builds_map_without_tool() {
        let imgs = vec![
            "1.dkr.ecr.us-east-1.amazonaws.com/api:1".to_string(),
            "redis:7".to_string(),
        ];
        let maps = mirror_images(&imgs, "reg.local", true).await.unwrap();
        assert_eq!(maps.len(), 2);
        assert_eq!(maps[0].target, "reg.local/api:1");
        assert_eq!(maps[1].target, "reg.local/redis:7");
    }
}
