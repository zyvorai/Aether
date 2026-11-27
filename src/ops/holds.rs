// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Change holds: an operator marks a workload as "do not touch".
//!
//! While a hold is active, Aether refuses to deploy, update, stop, restart,
//! scale, migrate, roll back or delete the workload. The check sits in the
//! runtime layer that every Aether-managed change goes through (see
//! [`crate::ops::journal`]), with earlier checks in the CLI and API so nothing
//! is partly done first.
//!
//! A hold is a guard against Aether, not against the platform underneath:
//! `kubectl`, `podman` or the cluster browser can still change the objects.

use crate::ops::{self, store};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Longest accepted reason.
const MAX_REASON: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Hold {
    pub workload: String,
    pub reason: String,
    pub actor: String,
    pub created_at: String,
    /// `None` holds until released.
    #[serde(default)]
    pub expires_at: Option<String>,
}

impl Hold {
    /// A hold with an unreadable expiry stays active: failing open would let a
    /// change through that the operator asked to block.
    pub fn is_active(&self, now: chrono::DateTime<chrono::Utc>) -> bool {
        match &self.expires_at {
            None => true,
            Some(ts) => ops::parse_time(ts).is_none_or(|t| t > now),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HoldStore {
    pub holds: BTreeMap<String, Hold>,
}

/// The error returned when a change is refused because of a hold.
#[derive(Debug, Clone)]
pub struct Held {
    pub hold: Hold,
    pub operation: String,
}

impl std::fmt::Display for Held {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let until = self.hold.expires_at.as_deref().unwrap_or("released");
        write!(
            f,
            "cannot {} '{}': on hold until {} by {} ({}). Release it with `aether hold release {}`",
            self.operation,
            self.hold.workload,
            until,
            self.hold.actor,
            self.hold.reason,
            self.hold.workload
        )
    }
}

impl std::error::Error for Held {}

pub fn path(dir: &Path) -> PathBuf {
    dir.join("holds.json")
}

/// Active holds, sorted by workload name.
pub fn active(dir: &Path) -> Result<Vec<Hold>> {
    let now = chrono::Utc::now();
    let s: HoldStore = store::load(&path(dir))?;
    Ok(s.holds.into_values().filter(|h| h.is_active(now)).collect())
}

/// The active hold on `workload`, if any.
pub fn get(dir: &Path, workload: &str) -> Result<Option<Hold>> {
    let now = chrono::Utc::now();
    let mut s: HoldStore = store::load(&path(dir))?;
    Ok(s.holds.remove(workload).filter(|h| h.is_active(now)))
}

/// Refuse `operation` when `workload` is held. An unreadable hold store also
/// refuses.
pub fn check(dir: &Path, workload: &str, operation: &str) -> Result<()> {
    match get(dir, workload)? {
        None => Ok(()),
        Some(hold) => Err(Held {
            hold,
            operation: operation.to_string(),
        }
        .into()),
    }
}

/// Place or replace a hold.
pub fn set(dir: &Path, workload: &str, reason: &str, duration_secs: Option<i64>) -> Result<Hold> {
    ops::check_name(workload)?;
    let reason = crate::speclint::sanitize(reason.trim());
    if reason.is_empty() {
        anyhow::bail!("a hold needs a reason");
    }
    if reason.chars().count() > MAX_REASON {
        anyhow::bail!("reason is longer than {MAX_REASON} characters");
    }
    let now = chrono::Utc::now();
    // Whole seconds: these timestamps are read by people.
    let stamp =
        |t: chrono::DateTime<chrono::Utc>| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let expires_at = match duration_secs {
        None => None,
        Some(s) => Some(
            now.checked_add_signed(chrono::Duration::seconds(s))
                .ok_or_else(|| anyhow::anyhow!("hold duration is too large"))
                .map(stamp)?,
        ),
    };
    let hold = Hold {
        workload: workload.to_string(),
        reason,
        actor: ops::current_actor(),
        created_at: stamp(now),
        expires_at,
    };
    store::update(&path(dir), |s: &mut HoldStore| {
        s.holds.retain(|_, h| h.is_active(now));
        s.holds.insert(workload.to_string(), hold.clone());
    })?;
    ops::journal::audit(
        dir,
        crate::audit::AuditAction::ConfigChange,
        workload,
        None,
        crate::audit::ActionResult::Success,
        &format!("hold placed: {}", hold.reason),
    )?;
    Ok(hold)
}

/// Release a hold. Returns the hold that was removed, if one was active.
pub fn release(dir: &Path, workload: &str) -> Result<Option<Hold>> {
    let now = chrono::Utc::now();
    let removed = store::update(&path(dir), |s: &mut HoldStore| s.holds.remove(workload))?
        .filter(|h| h.is_active(now));
    if let Some(h) = &removed {
        ops::journal::audit(
            dir,
            crate::audit::AuditAction::ConfigChange,
            workload,
            None,
            crate::audit::ActionResult::Success,
            &format!("hold released: {}", h.reason),
        )?;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hold_blocks_until_released() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        assert!(check(d, "web", "delete").is_ok());
        set(d, "web", "payroll run", None).unwrap();
        let err = check(d, "web", "delete").unwrap_err();
        assert!(err.downcast_ref::<Held>().is_some());
        assert!(err.to_string().contains("payroll run"), "{err}");
        assert!(check(d, "api", "delete").is_ok());
        assert!(release(d, "web").unwrap().is_some());
        assert!(check(d, "web", "delete").is_ok());
        assert!(release(d, "web").unwrap().is_none());
    }

    #[test]
    fn expired_holds_do_not_block_and_bad_expiry_does() {
        let dir = tempfile::tempdir().unwrap();
        let d = dir.path();
        set(d, "web", "r", Some(3600)).unwrap();
        assert_eq!(active(d).unwrap().len(), 1);
        store::update(&path(d), |s: &mut HoldStore| {
            s.holds.get_mut("web").unwrap().expires_at = Some("2000-01-01T00:00:00Z".into());
        })
        .unwrap();
        assert!(check(d, "web", "stop").is_ok());
        store::update(&path(d), |s: &mut HoldStore| {
            s.holds.get_mut("web").unwrap().expires_at = Some("garbage".into());
        })
        .unwrap();
        assert!(check(d, "web", "stop").is_err());
    }

    #[test]
    fn corrupt_hold_store_fails_closed() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(path(dir.path()), "{").unwrap();
        assert!(check(dir.path(), "web", "delete").is_err());
    }

    #[test]
    fn reason_is_required() {
        let dir = tempfile::tempdir().unwrap();
        assert!(set(dir.path(), "web", "  ", None).is_err());
        assert!(set(dir.path(), "../x", "r", None).is_err());
    }
}
