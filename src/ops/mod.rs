// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Operator features built on what Aether itself records.
//!
//! Everything here is derived from files Aether writes: the audit trail, the
//! event bus, health observations, applied spec revisions and holds. Nothing
//! is estimated or simulated; where there is no record, the report says so.
//!
//! Every function takes the Aether data directory (normally `~/.aether`, see
//! [`data_dir`]) so the logic can be tested against a temporary directory.

pub mod attention;
pub mod changes;
pub mod explain;
pub mod holds;
pub mod incidents;
pub mod journal;
pub mod revisions;
pub mod rollback;
pub mod store;

use std::path::PathBuf;
use std::sync::OnceLock;

/// The Aether data directory (`~/.aether`).
pub fn data_dir() -> PathBuf {
    crate::resources::aether_dir()
}

static PROCESS_ACTOR: OnceLock<String> = OnceLock::new();

tokio::task_local! {
    static ACTOR: String;
    static REASON: String;
}

/// Set the default actor for this process (for example `cli:alice` or `serve`).
/// The first call wins.
pub fn set_process_actor(actor: impl Into<String>) {
    let _ = PROCESS_ACTOR.set(clean_actor(&actor.into()));
}

/// Who is making the current change: the task-scoped actor (an API caller),
/// else the process actor, else `unknown`.
pub fn current_actor() -> String {
    ACTOR
        .try_with(Clone::clone)
        .ok()
        .or_else(|| PROCESS_ACTOR.get().cloned())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Why the current change is being made (`rollback to revision 3`), if stated.
pub fn current_reason() -> Option<String> {
    REASON.try_with(Clone::clone).ok()
}

/// Run `fut` with every recorded change attributed to `actor`.
pub async fn with_actor<F: std::future::Future>(actor: String, fut: F) -> F::Output {
    ACTOR.scope(clean_actor(&actor), fut).await
}

/// Actor names come from keys and identity providers; keep them short and free
/// of the separator used in audit details.
fn clean_actor(actor: &str) -> String {
    actor
        .chars()
        .filter(|c| !c.is_control() && *c != ';')
        .take(80)
        .collect()
}

/// Run `fut` with every recorded change annotated with `reason`.
pub async fn with_reason<F: std::future::Future>(reason: String, fut: F) -> F::Output {
    REASON.scope(reason, fut).await
}

/// Parse an RFC 3339 timestamp.
pub fn parse_time(ts: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|d| d.with_timezone(&chrono::Utc))
}

/// Parse a time window such as `30m`, `24h`, `7d`, `2w` or an RFC 3339
/// timestamp into the instant the window starts.
pub fn parse_since(
    value: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    let value = value.trim();
    if let Some(t) = parse_time(value) {
        return Ok(t);
    }
    let secs = parse_duration_secs(value)?;
    now.checked_sub_signed(chrono::Duration::seconds(secs))
        .ok_or_else(|| anyhow::anyhow!("time window '{value}' is too large"))
}

/// Parse `90s`, `30m`, `24h`, `7d` or `2w` into seconds. The longest accepted
/// duration is ten years.
pub fn parse_duration_secs(value: &str) -> anyhow::Result<i64> {
    const MAX: i64 = 10 * 366 * 86_400;
    let bad =
        || anyhow::anyhow!("invalid duration '{value}': use a number followed by s, m, h, d or w");
    let value_trimmed = value.trim();
    let unit = value_trimmed.chars().last().ok_or_else(bad)?;
    let number = &value_trimmed[..value_trimmed.len() - unit.len_utf8()];
    let n: i64 = number.parse().map_err(|_| bad())?;
    let mult = match unit {
        's' => 1,
        'm' => 60,
        'h' => 3_600,
        'd' => 86_400,
        'w' => 604_800,
        _ => return Err(bad()),
    };
    let secs = n.checked_mul(mult).ok_or_else(bad)?;
    if secs <= 0 || secs > MAX {
        return Err(bad());
    }
    Ok(secs)
}

/// Human duration: `45s`, `3m 10s`, `2h 5m`, `3d 4h`.
pub fn format_duration(secs: i64) -> String {
    let s = secs.max(0);
    if s < 60 {
        format!("{s}s")
    } else if s < 3_600 {
        format!("{}m {}s", s / 60, s % 60)
    } else if s < 86_400 {
        format!("{}h {}m", s / 3_600, (s % 3_600) / 60)
    } else {
        format!("{}d {}h", s / 86_400, (s % 86_400) / 3_600)
    }
}

/// A workload name that is safe to use as a file name.
pub fn check_name(name: &str) -> anyhow::Result<()> {
    let ok = !name.is_empty()
        && name.len() <= 253
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.');
    if ok {
        Ok(())
    } else {
        anyhow::bail!("invalid workload name '{}'", name.escape_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_parse_and_reject_nonsense() {
        assert_eq!(parse_duration_secs("90s").unwrap(), 90);
        assert_eq!(parse_duration_secs("2h").unwrap(), 7200);
        assert_eq!(parse_duration_secs("7d").unwrap(), 604_800);
        for bad in ["", "h", "10", "-5m", "0h", "5y", "99999999999999999w", "5é"] {
            assert!(parse_duration_secs(bad).is_err(), "{bad:?} should fail");
        }
    }

    #[test]
    fn since_accepts_timestamps() {
        let now = parse_time("2026-03-01T12:00:00Z").unwrap();
        assert_eq!(
            parse_since("1h", now).unwrap(),
            parse_time("2026-03-01T11:00:00Z").unwrap()
        );
        assert_eq!(
            parse_since("2026-02-01T00:00:00+00:00", now).unwrap(),
            parse_time("2026-02-01T00:00:00Z").unwrap()
        );
    }

    #[test]
    fn names_cannot_escape_the_directory() {
        assert!(check_name("web-1").is_ok());
        for bad in ["", "..", ".hidden", "a/b", "a\\b", "a b"] {
            assert!(check_name(bad).is_err(), "{bad:?}");
        }
    }

    #[tokio::test]
    async fn reason_and_actor_are_task_scoped() {
        assert!(current_reason().is_none());
        with_reason("because".into(), async {
            with_actor("api:ops".into(), async {
                assert_eq!(current_reason().as_deref(), Some("because"));
                assert_eq!(current_actor(), "api:ops");
            })
            .await;
        })
        .await;
        assert!(current_reason().is_none());
    }
}
