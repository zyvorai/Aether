// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Roll a workload back to an applied revision.
//!
//! The spec comes from the revision itself, so the result does not depend on
//! what has since happened to the original spec file. Only the spec is rolled
//! back: images are rebuilt from the build context and volume data is left as
//! it is.

use crate::ops::{self, holds, revisions, store};
use crate::runtime::{Image, Instance, InstanceState, Runtime, RuntimeKind};
use crate::spec::Workload;
use crate::state::WorkloadState;
use anyhow::{Context, Result};
use std::path::Path;

/// A rollback that was checked but not yet applied.
pub struct Planned {
    pub revision: u64,
    pub spec: Workload,
    yaml: String,
}

/// Choose and load the revision to apply. `None` picks
/// [`revisions::RevisionLog::rollback_target`].
pub fn plan(dir: &Path, ws: &WorkloadState, revision: Option<u64>) -> Result<Planned> {
    holds::check(dir, &ws.name, "roll back")?;
    let log = revisions::load(dir, &ws.name)?;
    let target = match revision {
        Some(id) => log
            .get(id)
            .ok_or_else(|| anyhow::anyhow!("revision {id} of '{}' is not recorded", ws.name))?,
        None => log.rollback_target()?,
    };
    if target.runtime != ws.runtime {
        anyhow::bail!(
            "revision {} ran on {} but '{}' now runs on {}; migrate it back first",
            target.id,
            target.runtime,
            ws.name,
            ws.runtime
        );
    }
    let spec: Workload = serde_yaml::from_str(&target.spec_yaml)
        .with_context(|| format!("revision {} cannot be read by this version", target.id))?;
    spec.validate()
        .with_context(|| format!("revision {} is no longer a valid spec", target.id))?;
    Ok(Planned {
        revision: target.id,
        spec,
        yaml: target.spec_yaml.clone(),
    })
}

/// Whether the local runtime reports that the container no longer exists.
async fn is_gone(rt: &dyn Runtime, instance: &Instance) -> bool {
    rt.status(instance).await.is_ok_and(|s| {
        s.state == InstanceState::Unknown && s.message.as_deref() == Some("Container not found")
    })
}

/// Put `spec` in place of a running instance without destroying data.
///
/// * Kubernetes patches the objects.
/// * Podman and Docker replace the container: a stopped one would keep its name.
/// * KubeVirt is stopped and applied again. Its [`Runtime::update`] falls back
///   to delete-and-recreate, which would delete the VM's disk.
pub async fn replace(
    rt: &dyn Runtime,
    runtime: RuntimeKind,
    instance: &Instance,
    image: &Image,
    spec: &Workload,
) -> Result<Instance> {
    match runtime {
        RuntimeKind::Kubernetes => rt.update(instance, image, spec).await,
        RuntimeKind::Podman | RuntimeKind::Docker => {
            // An earlier failed change may have removed the container already.
            if is_gone(rt, instance).await {
                rt.run(image, spec).await
            } else {
                rt.update(instance, image, spec).await
            }
        }
        RuntimeKind::KubeVirt => {
            rt.stop(instance).await?;
            rt.run(image, spec).await
        }
    }
}

/// Apply a planned rollback through `rt` and return the new workload state,
/// which the caller persists.
pub async fn apply(
    dir: &Path,
    ws: &WorkloadState,
    rt: &dyn Runtime,
    planned: &Planned,
) -> Result<WorkloadState> {
    ops::check_name(&ws.name)?;
    let reason = format!("rollback to revision {}", planned.revision);
    ops::with_reason(reason, async {
        // Build first: a build failure leaves the running instance untouched.
        let image = rt.build(&planned.spec).await?;
        let instance = replace(rt, ws.runtime, &ws.instance, &image, &planned.spec).await?;

        // Point the state at the spec that is now running, in a file owned by
        // Aether so the operator's own spec file is never overwritten.
        let spec_dir = dir.join("specs").join("rollback");
        std::fs::create_dir_all(&spec_dir)?;
        let spec_path = spec_dir.join(format!("{}.yaml", ws.name));
        store::write_atomic(&spec_path, planned.yaml.as_bytes())?;

        let mut new = ws.migrated(ws.runtime, instance);
        new.spec_path = spec_path;
        Ok(new)
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ops::journal::tests::journaled;
    use crate::ops::revisions::tests::spec;
    use std::sync::atomic::Ordering;

    async fn deploy(rt: &dyn Runtime, memory: &str) -> crate::runtime::Instance {
        let w = spec("web", memory);
        let image = rt.build(&w).await.unwrap();
        rt.run(&image, &w).await.unwrap()
    }

    #[tokio::test]
    async fn applies_the_recorded_spec_even_if_the_file_changed() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let (rt, _, _) = journaled(d);
        deploy(&rt, "256Mi").await;
        let instance = deploy(&rt, "512Mi").await;
        let file = d.join("web.yaml");
        std::fs::write(&file, "edited since").unwrap();
        let ws = WorkloadState::new("web".into(), RuntimeKind::Podman, instance, file.clone());

        let planned = plan(d, &ws, None).unwrap();
        assert_eq!(planned.revision, 1);
        let new = apply(d, &ws, &rt, &planned).await.unwrap();
        assert_eq!(
            Workload::from_file(&new.spec_path)
                .unwrap()
                .requirements
                .memory,
            "256Mi"
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "edited since");
        let log = revisions::load(d, "web").unwrap();
        let latest = log.latest().unwrap();
        assert_eq!(latest.reason.as_deref(), Some("rollback to revision 1"));
        assert_eq!(latest.spec_hash, log.get(1).unwrap().spec_hash);
    }

    #[tokio::test]
    async fn a_failed_rollback_reports_the_error() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let (rt, _, fail) = journaled(d);
        deploy(&rt, "256Mi").await;
        let instance = deploy(&rt, "512Mi").await;
        let ws = WorkloadState::new("web".into(), RuntimeKind::Podman, instance, "x".into());
        let planned = plan(d, &ws, None).unwrap();
        fail.store(true, Ordering::SeqCst);
        assert!(apply(d, &ws, &rt, &planned).await.is_err());
        assert!(revisions::load(d, "web").unwrap().failed_at.is_some());
    }

    #[tokio::test]
    async fn refuses_held_workloads_other_runtimes_and_unknown_revisions() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let (rt, _, _) = journaled(d);
        let instance = deploy(&rt, "256Mi").await;
        let mut ws = WorkloadState::new("web".into(), RuntimeKind::Podman, instance, "x".into());
        assert!(plan(d, &ws, Some(9)).is_err());
        assert!(plan(d, &ws, None).is_err());
        ws.runtime = RuntimeKind::Kubernetes;
        assert!(plan(d, &ws, Some(1)).is_err());
        ws.runtime = RuntimeKind::Podman;
        holds::set(d, "web", "freeze", None).unwrap();
        assert!(plan(d, &ws, Some(1)).is_err());
    }

    #[tokio::test]
    async fn a_vm_is_never_deleted_to_replace_its_spec() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        let (rt, _, _) = journaled(d);
        let instance = deploy(&rt, "256Mi").await;
        let w = spec("web", "512Mi");
        let image = rt.build(&w).await.unwrap();
        replace(&rt, RuntimeKind::KubeVirt, &instance, &image, &w)
            .await
            .unwrap();
        let audit: crate::audit::AuditLog =
            store::load(&crate::ops::journal::audit_path(d)).unwrap();
        assert!(audit
            .events()
            .iter()
            .all(|e| e.action != crate::audit::AuditAction::Delete));
    }
}
