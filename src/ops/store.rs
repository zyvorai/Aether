// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Locked, atomic read-modify-write of a JSON store.
//!
//! The CLI and `aether serve` write the same files. [`update`] takes an
//! advisory lock next to the file, loads it, applies the change and renames a
//! freshly written temporary file over it, so concurrent writers do not lose
//! records and readers never see a half-written document. A store that does not
//! parse is reported as an error and left untouched.

use anyhow::{Context, Result};
use serde::{de::DeserializeOwned, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}

#[cfg(unix)]
fn lock_exclusive(file: &std::fs::File) -> Result<()> {
    use std::os::unix::io::AsRawFd;
    // SAFETY: flock on a descriptor owned by `file`, which outlives the call.
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } != 0 {
        return Err(std::io::Error::last_os_error()).context("flock");
    }
    Ok(())
}

#[cfg(not(unix))]
fn lock_exclusive(_file: &std::fs::File) -> Result<()> {
    Ok(())
}

/// Load a store, returning the default value when the file does not exist.
pub fn load<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    crate::resources::json_load(path).with_context(|| format!("failed to read {}", path.display()))
}

/// Write `content` to `path` atomically with owner-only permissions.
pub fn write_atomic(path: &Path, content: &[u8]) -> Result<()> {
    let tmp = sibling(
        path,
        &format!(
            ".tmp.{}.{}",
            std::process::id(),
            TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ),
    );
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let result = opts
        .open(&tmp)
        .and_then(|mut f| f.write_all(content))
        .and_then(|()| std::fs::rename(&tmp, path));
    if result.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    result.with_context(|| format!("failed to write {}", path.display()))
}

/// Apply `change` to the store at `path` while holding its lock.
pub fn update<T, R>(path: &Path, change: impl FnOnce(&mut T) -> R) -> Result<R>
where
    T: Serialize + DeserializeOwned + Default,
{
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
    }
    let lock_path = sibling(path, ".lock");
    let lock = std::fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .with_context(|| format!("failed to open {}", lock_path.display()))?;
    lock_exclusive(&lock)?;
    let mut value: T = load(path)?;
    let out = change(&mut value);
    let content = serde_json::to_vec_pretty(&value).context("failed to serialise store")?;
    write_atomic(path, &content)?;
    drop(lock);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    type Map = BTreeMap<String, u32>;

    #[test]
    fn concurrent_updates_are_not_lost() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.json");
        std::thread::scope(|s| {
            for i in 0..8 {
                let path = &path;
                s.spawn(move || {
                    for j in 0..10 {
                        update(path, |m: &mut Map| {
                            m.insert(format!("{i}-{j}"), j);
                        })
                        .unwrap();
                    }
                });
            }
        });
        assert_eq!(load::<Map>(&path).unwrap().len(), 80);
    }

    #[test]
    fn corrupt_store_is_reported_and_preserved() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.json");
        std::fs::write(&path, "not json").unwrap();
        assert!(update(&path, |_: &mut Map| ()).is_err());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "not json");
    }

    #[cfg(unix)]
    #[test]
    fn written_files_are_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("m.json");
        update(&path, |_: &mut Map| ()).unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
