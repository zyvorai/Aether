// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Replacing the container of a Podman or Docker workload without losing it.
//!
//! A stopped container keeps its name, so a new one cannot simply be run. It is
//! renamed out of the way first, removed only once its replacement runs, and
//! given its name back if the replacement fails.

use crate::runtime::{Image, Instance, InstanceState, Runtime};
use crate::spec::Workload;
use anyhow::Context;
use tokio::process::Command;

const ASIDE_SUFFIX: &str = "-aether-previous";

const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Container states in which nothing is executing.
const IDLE: [&str; 6] = [
    "exited",
    "stopped",
    "created",
    "configured",
    "initialized",
    "dead",
];

async fn engine(bin: &str, args: &[&str]) -> crate::Result<std::process::Output> {
    let output = tokio::time::timeout(TIMEOUT, Command::new(bin).args(args).output())
        .await
        .map_err(|_| anyhow::anyhow!("{bin} {} timed out after {TIMEOUT:?}", args[0]))??;
    Ok(output)
}

async fn engine_ok(bin: &str, args: &[&str]) -> crate::Result<()> {
    let output = engine(bin, args).await?;
    if !output.status.success() {
        anyhow::bail!(
            "{bin} {} failed: {}",
            args[0],
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(())
}

struct Existing {
    state: String,
    /// Created by Aether, as its label says.
    ours: bool,
}

/// The container called `name`, if there is one.
async fn find(bin: &str, name: &str) -> crate::Result<Option<Existing>> {
    let output = engine(
        bin,
        &[
            "container",
            "inspect",
            "--format",
            r#"{{.State.Status}}|{{index .Config.Labels "aether-managed"}}"#,
            name,
        ],
    )
    .await?;
    // A missing container is the usual failure; any other will show on `run`.
    if !output.status.success() {
        return Ok(None);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let (state, label) = text.trim().split_once('|').unwrap_or((text.trim(), ""));
    Ok(Some(Existing {
        state: state.to_string(),
        ours: label == "true",
    }))
}

/// A stopped container renamed to make room for its replacement.
#[must_use = "call `discard` or `restore`"]
pub struct Aside {
    bin: &'static str,
    name: String,
    aside: String,
}

impl Aside {
    /// The replacement runs: remove the old container.
    pub async fn discard(self) {
        if let Err(e) = engine_ok(self.bin, &["rm", "-f", &self.aside]).await {
            tracing::warn!("could not remove the replaced container: {e:#}");
        }
    }

    /// The replacement failed: remove what it left and give the name back.
    pub async fn restore(self) -> crate::Result<()> {
        // The name was free after the rename, so anything under it now is the
        // failed replacement. There may be nothing.
        let _ = engine(self.bin, &["rm", "-f", &self.name]).await;
        engine_ok(self.bin, &["rename", &self.aside, &self.name])
            .await
            .with_context(|| format!("the previous container is kept as '{}'", self.aside))
    }
}

/// Free `name` for a new container.
///
/// Only a stopped container that Aether created is moved. A running one is an
/// error; one that Aether did not create is left for `run` to refuse.
pub async fn set_aside(bin: &'static str, name: &str) -> crate::Result<Option<Aside>> {
    let Some(existing) = find(bin, name).await? else {
        return Ok(None);
    };
    if !existing.ours {
        return Ok(None);
    }
    if !IDLE.contains(&existing.state.as_str()) {
        anyhow::bail!(
            "container '{name}' is already running (state: {}); stop it first",
            existing.state
        );
    }
    let aside = format!("{name}{ASIDE_SUFFIX}");
    if find(bin, &aside)
        .await?
        .is_some_and(|old| old.ours && IDLE.contains(&old.state.as_str()))
    {
        engine_ok(bin, &["rm", "-f", &aside]).await?;
    }
    engine_ok(bin, &["rename", name, &aside]).await?;
    Ok(Some(Aside {
        bin,
        name: name.to_string(),
        aside,
    }))
}

/// [`Runtime::update`] for a local engine: if the new container cannot start,
/// the old one is put back and, if it was running, started again.
pub async fn update(
    bin: &'static str,
    rt: &dyn Runtime,
    instance: &Instance,
    image: &Image,
    spec: &Workload,
) -> crate::Result<Instance> {
    let before = rt.status(instance).await?;
    let was_running = before.state == InstanceState::Running;
    let exists = find(bin, &instance.name).await?.is_some();
    if exists {
        rt.stop(instance).await?;
    }
    match rt.run(image, spec).await {
        Ok(new) => {
            // `run` replaced a container of the same name; a renamed workload
            // still has the old one.
            if exists && instance.name != spec.metadata.name {
                if let Err(e) = rt.delete(instance).await {
                    tracing::warn!("could not remove the replaced container: {e:#}");
                }
            }
            Ok(new)
        }
        Err(e) => {
            if was_running {
                if let Err(restart) = engine_ok(bin, &["start", &instance.id]).await {
                    return Err(e.context(format!(
                        "the previous container could not be started again: {restart:#}"
                    )));
                }
            }
            Err(e)
        }
    }
}
