// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! The change journal.
//!
//! [`Journaled`] wraps every runtime adapter that
//! [`crate::runtime::create_runtime`] hands out, so deploys, updates, stops
//! and deletes are recorded the same way whether they came from the CLI, the
//! API, a migration, a compose file or the self-healing loop. For each change
//! it refuses held workloads, writes an audit event for success and failure
//! and records the applied spec revision.
//!
//! Bookkeeping failures never turn a successful runtime change into an error;
//! they are logged.

use crate::audit::{ActionResult, AuditAction, AuditLog};
use crate::events::{Event, EventBus, EventCategory, EventSeverity};
use crate::ops::{self, holds, revisions, store};
use crate::runtime::{Capacity, Image, Instance, Runtime, RuntimeKind, Status};
use crate::spec::Workload;
use anyhow::Result;
use async_trait::async_trait;
use std::path::{Path, PathBuf};

pub fn audit_path(dir: &Path) -> PathBuf {
    dir.join("audit.json")
}

pub fn events_path(dir: &Path) -> PathBuf {
    dir.join("events.json")
}

/// Append an audit event under the audit log's lock. The details name the
/// actor and, when stated, the reason.
pub fn audit(
    dir: &Path,
    action: AuditAction,
    workload: &str,
    runtime: Option<&str>,
    result: ActionResult,
    message: &str,
) -> Result<()> {
    audit_with(dir, action, workload, runtime, result, message, None)
}

/// [`audit`] with extra detail appended.
pub fn audit_with(
    dir: &Path,
    action: AuditAction,
    workload: &str,
    runtime: Option<&str>,
    result: ActionResult,
    message: &str,
    extra: Option<&str>,
) -> Result<()> {
    let mut details = format!("actor={}", ops::current_actor());
    if let Some(reason) = ops::current_reason() {
        details.push_str(&format!("; reason={reason}"));
    }
    if let Some(extra) = extra {
        details.push_str("; ");
        details.push_str(extra);
    }
    store::update(&audit_path(dir), |log: &mut AuditLog| {
        log.record(action, workload, runtime, result, message, Some(&details));
    })
}

/// Metadata key set on events written here; the change feed skips them because
/// the same change is already in the audit trail or the incident store.
pub const EVENT_MARKER: &str = "aether.journal";

/// Append an event (and deliver it to the configured channels) under the event
/// bus lock.
pub fn event(
    dir: &Path,
    severity: EventSeverity,
    category: EventCategory,
    workload: Option<&str>,
    title: &str,
    message: &str,
) -> Result<()> {
    let event = Event {
        id: 0,
        timestamp: crate::resources::now_rfc3339(),
        severity,
        category,
        source: ops::current_actor(),
        workload: workload.map(str::to_string),
        title: title.to_string(),
        message: message.to_string(),
        metadata: [(EVENT_MARKER.to_string(), "1".to_string())].into(),
        acknowledged: false,
    };
    publish(dir, vec![event])
}

/// Append events under the event bus lock, then deliver their notifications.
pub fn publish(dir: &Path, events: Vec<Event>) -> Result<()> {
    // Deliver after the lock is released: a slow webhook must not block
    // other writers.
    let (bus, notifications) = store::update(&events_path(dir), |bus: &mut EventBus| {
        let notifications: Vec<_> = events
            .into_iter()
            .flat_map(|event| bus.append(event).1)
            .collect();
        (bus.clone(), notifications)
    })?;
    bus.deliver(&notifications);
    Ok(())
}

/// Record the outcome of a runtime migration as one entry, beside the
/// individual stop, deploy and delete it is made of.
pub fn record_migration(
    dir: &Path,
    workload: &str,
    from: RuntimeKind,
    to: RuntimeKind,
    outcome: &Result<crate::migration::MigrationResult>,
) {
    let (result, message) = match outcome {
        Ok(r) if r.success => (
            ActionResult::Success,
            format!("migrated from {from} to {to}"),
        ),
        Ok(r) => (
            ActionResult::Failure,
            format!(
                "migration from {from} to {to} failed{}: {}",
                if r.rollback_performed {
                    " and was rolled back"
                } else {
                    ""
                },
                r.error.as_deref().unwrap_or("unknown error")
            ),
        ),
        Err(e) => (
            ActionResult::Failure,
            format!("migration from {from} to {to} failed: {e}"),
        ),
    };
    log_failure(
        "migration",
        audit(
            dir,
            AuditAction::Migrate,
            workload,
            Some(&to.to_string()),
            result,
            &message,
        ),
    );
}

/// Audit marker linking an audit event to a spec revision.
pub const REVISION_MARKER: &str = "revision=";

fn log_failure(what: &str, result: Result<()>) {
    if let Err(e) = result {
        tracing::warn!("change journal: could not record {what}: {e:#}");
    }
}

/// A runtime whose changes are guarded by holds and recorded.
pub struct Journaled {
    inner: Box<dyn Runtime>,
    kind: RuntimeKind,
    dir: PathBuf,
}

impl Journaled {
    pub fn new(inner: Box<dyn Runtime>, kind: RuntimeKind, dir: PathBuf) -> Self {
        Self { inner, kind, dir }
    }

    fn failed(&self, action: AuditAction, workload: &str, what: &str, error: &anyhow::Error) {
        let runtime = self.kind.to_string();
        let message = format!("{what} failed on {runtime}: {}", error.to_string().trim());
        log_failure(
            "failure",
            audit(
                &self.dir,
                action,
                workload,
                Some(&runtime),
                ActionResult::Failure,
                &message,
            ),
        );
        log_failure(
            "failure event",
            event(
                &self.dir,
                EventSeverity::Error,
                EventCategory::Deployment,
                Some(workload),
                &format!("{what} failed"),
                &message,
            ),
        );
    }

    fn applied(&self, op: revisions::RevisionOp, spec: &Workload, instance: &Instance) {
        let name = &spec.metadata.name;
        let revision = revisions::record(&self.dir, op, spec, instance);
        let extra = match &revision {
            Ok(id) => Some(format!("{REVISION_MARKER}{id}")),
            Err(e) => {
                tracing::warn!("change journal: could not record revision of {name}: {e:#}");
                None
            }
        };
        let verb = match op {
            revisions::RevisionOp::Deploy => "deployed",
            revisions::RevisionOp::Update => "updated",
        };
        let runtime = self.kind.to_string();
        log_failure(
            "deploy",
            audit_with(
                &self.dir,
                match op {
                    revisions::RevisionOp::Deploy => AuditAction::Deploy,
                    revisions::RevisionOp::Update => AuditAction::ConfigChange,
                },
                name,
                Some(&runtime),
                ActionResult::Success,
                &format!("{verb} on {runtime}: {}", instance.image),
                extra.as_deref(),
            ),
        );
    }

    fn change_failed(&self, action: AuditAction, name: &str, what: &str, error: &anyhow::Error) {
        log_failure(
            "failed revision",
            revisions::record_failure(&self.dir, name),
        );
        self.failed(action, name, what, error);
    }
}

#[async_trait]
impl Runtime for Journaled {
    async fn build(&self, spec: &Workload) -> crate::Result<Image> {
        let result = self.inner.build(spec).await;
        if let Err(e) = &result {
            self.failed(AuditAction::Build, &spec.metadata.name, "build", e);
        }
        result
    }

    async fn run(&self, image: &Image, spec: &Workload) -> crate::Result<Instance> {
        let name = &spec.metadata.name;
        holds::check(&self.dir, name, "deploy")?;
        match self.inner.run(image, spec).await {
            Ok(instance) => {
                self.applied(revisions::RevisionOp::Deploy, spec, &instance);
                Ok(instance)
            }
            Err(e) => {
                self.change_failed(AuditAction::Deploy, name, "deploy", &e);
                Err(e)
            }
        }
    }

    async fn update(
        &self,
        instance: &Instance,
        image: &Image,
        spec: &Workload,
    ) -> crate::Result<Instance> {
        let name = &spec.metadata.name;
        holds::check(&self.dir, &instance.name, "update")?;
        holds::check(&self.dir, name, "update")?;
        match self.inner.update(instance, image, spec).await {
            Ok(new) => {
                self.applied(revisions::RevisionOp::Update, spec, &new);
                Ok(new)
            }
            Err(e) => {
                self.change_failed(AuditAction::ConfigChange, name, "update", &e);
                Err(e)
            }
        }
    }

    async fn stop(&self, instance: &Instance) -> crate::Result<()> {
        holds::check(&self.dir, &instance.name, "stop")?;
        let runtime = self.kind.to_string();
        match self.inner.stop(instance).await {
            Ok(()) => {
                log_failure(
                    "stop",
                    audit(
                        &self.dir,
                        AuditAction::Stop,
                        &instance.name,
                        Some(&runtime),
                        ActionResult::Success,
                        &format!("stopped on {runtime}"),
                    ),
                );
                Ok(())
            }
            Err(e) => {
                self.failed(AuditAction::Stop, &instance.name, "stop", &e);
                Err(e)
            }
        }
    }

    async fn status(&self, instance: &Instance) -> crate::Result<Status> {
        self.inner.status(instance).await
    }

    async fn logs(&self, instance: &Instance, follow: bool) -> crate::Result<String> {
        self.inner.logs(instance, follow).await
    }

    async fn delete(&self, instance: &Instance) -> crate::Result<()> {
        holds::check(&self.dir, &instance.name, "delete")?;
        let runtime = self.kind.to_string();
        match self.inner.delete(instance).await {
            Ok(()) => {
                log_failure(
                    "delete",
                    audit(
                        &self.dir,
                        AuditAction::Delete,
                        &instance.name,
                        Some(&runtime),
                        ActionResult::Success,
                        &format!("deleted from {runtime}"),
                    ),
                );
                Ok(())
            }
            Err(e) => {
                self.failed(AuditAction::Delete, &instance.name, "delete", &e);
                Err(e)
            }
        }
    }

    async fn list(&self) -> crate::Result<Vec<Instance>> {
        self.inner.list().await
    }

    async fn capacity(&self) -> crate::Result<Option<Capacity>> {
        self.inner.capacity().await
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ops::revisions::tests::spec;
    use crate::runtime::InstanceState;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    /// A runtime that only counts calls, so tests can prove what reached it.
    #[derive(Default)]
    struct Fake {
        calls: Arc<AtomicUsize>,
        fail: Arc<AtomicBool>,
    }

    impl Fake {
        fn call(&self) -> crate::Result<()> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            if self.fail.load(Ordering::SeqCst) {
                anyhow::bail!("boom");
            }
            Ok(())
        }
    }

    #[async_trait]
    impl Runtime for Fake {
        async fn build(&self, spec: &Workload) -> crate::Result<Image> {
            Ok(Image {
                name: spec.metadata.name.clone(),
                tag: "latest".into(),
                digest: None,
                runtime: RuntimeKind::Podman,
            })
        }
        async fn run(&self, image: &Image, spec: &Workload) -> crate::Result<Instance> {
            self.call()?;
            Ok(Instance::new(
                "id".into(),
                spec.metadata.name.clone(),
                RuntimeKind::Podman,
                image.full_name(),
            ))
        }
        async fn stop(&self, _: &Instance) -> crate::Result<()> {
            self.call()
        }
        async fn status(&self, _: &Instance) -> crate::Result<Status> {
            Ok(Status {
                state: InstanceState::Running,
                ready: true,
                message: None,
                restart_count: 0,
            })
        }
        async fn logs(&self, _: &Instance, _: bool) -> crate::Result<String> {
            Ok(String::new())
        }
        async fn delete(&self, _: &Instance) -> crate::Result<()> {
            self.call()
        }
        async fn list(&self) -> crate::Result<Vec<Instance>> {
            Ok(vec![])
        }
    }

    pub(crate) fn journaled(dir: &Path) -> (Journaled, Arc<AtomicUsize>, Arc<AtomicBool>) {
        let fake = Fake::default();
        let (calls, fail) = (fake.calls.clone(), fake.fail.clone());
        (
            Journaled::new(Box::new(fake), RuntimeKind::Podman, dir.to_path_buf()),
            calls,
            fail,
        )
    }

    fn log(dir: &Path) -> AuditLog {
        store::load(&audit_path(dir)).unwrap()
    }

    #[tokio::test]
    async fn lifecycle_is_audited_and_revisions_recorded() {
        let dir = tempfile::tempdir().unwrap();
        let (rt, _, _) = journaled(dir.path());
        let w = spec("web", "256Mi");
        let image = rt.build(&w).await.unwrap();
        let instance = ops::with_reason("test run".into(), rt.run(&image, &w))
            .await
            .unwrap();
        rt.stop(&instance).await.unwrap();
        rt.delete(&instance).await.unwrap();

        let audit = log(dir.path());
        let actions: Vec<_> = audit.events().iter().map(|e| e.action.clone()).collect();
        assert_eq!(
            actions,
            vec![AuditAction::Deploy, AuditAction::Stop, AuditAction::Delete]
        );
        let details = audit.events()[0].details.clone().unwrap();
        assert!(details.contains("revision=1"), "{details}");
        assert!(details.contains("reason=test run"), "{details}");
        assert!(audit.events().iter().all(AuditLog::verify_event_integrity));
        let revs = revisions::load(dir.path(), "web").unwrap();
        assert_eq!(revs.revisions.len(), 1);
        assert_eq!(revs.revisions[0].reason.as_deref(), Some("test run"));
    }

    #[tokio::test]
    async fn failures_are_recorded_and_not_counted_as_revisions() {
        let dir = tempfile::tempdir().unwrap();
        let (rt, _, fail) = journaled(dir.path());
        let w = spec("web", "256Mi");
        let image = rt.build(&w).await.unwrap();
        rt.run(&image, &w).await.unwrap();
        fail.store(true, Ordering::SeqCst);
        assert!(rt.run(&image, &w).await.is_err());
        let revs = revisions::load(dir.path(), "web").unwrap();
        assert_eq!(revs.revisions.len(), 1);
        assert!(revs.failed_at.is_some());
        let audit = log(dir.path());
        let last = audit.events().last().unwrap();
        assert_eq!(last.result, ActionResult::Failure);
        assert!(last.message.contains("boom"));
        let bus: EventBus = store::load(&events_path(dir.path())).unwrap();
        assert!(bus.events().iter().any(|e| e.title == "deploy failed"));
    }

    #[tokio::test]
    async fn held_workloads_never_reach_the_runtime() {
        let dir = tempfile::tempdir().unwrap();
        let (rt, calls, _) = journaled(dir.path());
        let w = spec("web", "256Mi");
        let image = rt.build(&w).await.unwrap();
        let instance = rt.run(&image, &w).await.unwrap();
        holds::set(dir.path(), "web", "freeze", None).unwrap();
        let before = calls.load(Ordering::SeqCst);
        assert!(rt.run(&image, &w).await.is_err());
        assert!(rt.update(&instance, &image, &w).await.is_err());
        assert!(rt.stop(&instance).await.is_err());
        assert!(rt.delete(&instance).await.is_err());
        assert_eq!(calls.load(Ordering::SeqCst), before);
        // Reading is still allowed.
        assert!(rt.status(&instance).await.is_ok());
    }
}
