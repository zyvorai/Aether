//! Shared resource parsing utilities
//!
//! Centralized parsing for Kubernetes-style resource quantities (CPU, memory, storage).
//! Used across the engine, AI modules, policy, drift detection, and cost estimation.

use std::path::PathBuf;

/// Parse a CPU resource string to fractional cores.
///
/// Accepts whole cores (`"4"`) or millicores (`"500m"`).
/// Returns `0.0` on invalid input.
pub fn parse_cpu(cpu: &str) -> f64 {
    if let Some(stripped) = cpu.strip_suffix('m') {
        stripped.parse::<f64>().unwrap_or(0.0) / 1000.0
    } else {
        cpu.parse::<f64>().unwrap_or(0.0)
    }
}

/// Parse a memory/storage resource string to GiB.
///
/// Accepts `Gi`, `Mi`, `Ki` suffixes (binary) and `G`, `M` (decimal).
/// Returns `0.0` on invalid input.
pub fn parse_memory_gi(memory: &str) -> f64 {
    let memory = memory.trim();
    if let Some(stripped) = memory.strip_suffix("Gi") {
        stripped.parse::<f64>().unwrap_or(0.0)
    } else if let Some(stripped) = memory.strip_suffix("Mi") {
        stripped.parse::<f64>().unwrap_or(0.0) / 1024.0
    } else if let Some(stripped) = memory.strip_suffix("Ki") {
        stripped.parse::<f64>().unwrap_or(0.0) / (1024.0 * 1024.0)
    } else if let Some(stripped) = memory.strip_suffix('G') {
        stripped.parse::<f64>().unwrap_or(0.0)
    } else if let Some(stripped) = memory.strip_suffix('M') {
        stripped.parse::<f64>().unwrap_or(0.0) / 1024.0
    } else {
        0.0
    }
}

/// Return the orchestr8 home directory (`~/.orchestr8`).
pub fn orchestr8_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".orchestr8")
}

/// Return a path inside the orchestr8 home directory.
pub fn orchestr8_path(filename: &str) -> PathBuf {
    orchestr8_dir().join(filename)
}

/// Load a JSON-serialisable value from a file.
///
/// Returns `Ok(default)` when the file does not exist,
/// avoiding an extra existence check (TOCTOU-safe).
pub fn json_load<T>(path: &std::path::Path) -> anyhow::Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    match std::fs::read_to_string(path) {
        Ok(content) => {
            let value: T = serde_json::from_str(&content)?;
            Ok(value)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(T::default()),
        Err(e) => Err(e.into()),
    }
}

/// Save a JSON-serialisable value to a file, creating parent directories as needed.
pub fn json_save<T: serde::Serialize>(value: &T, path: &std::path::Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(value)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// Return the current UTC time as an RFC 3339 string.
///
/// Replaces the ubiquitous `chrono::Utc::now().to_rfc3339()` one-liner.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Implement `default_path()`, `load()`, and `save()` for a JSON-persisted store type.
///
/// Eliminates the identical 3-method boilerplate across `EventBus`, `AuditLog`,
/// `Scheduler`, `Orchestrator`, `DependencyGraph`, `EnvironmentManager`, etc.
///
/// The type must derive `Serialize + DeserializeOwned + Default`.
#[macro_export]
macro_rules! impl_json_store {
    ($ty:ty, $filename:expr) => {
        impl $ty {
            /// Return the default on-disk path for this store.
            pub fn default_path() -> std::path::PathBuf {
                $crate::resources::orchestr8_path($filename)
            }

            /// Load from disk, returning `Default` if the file does not exist.
            pub fn load(path: &std::path::Path) -> anyhow::Result<Self> {
                $crate::resources::json_load(path)
            }

            /// Persist to disk, creating parent directories as needed.
            pub fn save(&self, path: &std::path::Path) -> anyhow::Result<()> {
                $crate::resources::json_save(self, path)
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_cpu_whole() {
        assert!((parse_cpu("4") - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_cpu_millicore() {
        assert!((parse_cpu("500m") - 0.5).abs() < f64::EPSILON);
        assert!((parse_cpu("2000m") - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_cpu_invalid() {
        assert!((parse_cpu("abc")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_memory_gi() {
        assert!((parse_memory_gi("4Gi") - 4.0).abs() < f64::EPSILON);
        assert!((parse_memory_gi("512Mi") - 0.5).abs() < f64::EPSILON);
        assert!((parse_memory_gi("8G") - 8.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_memory_invalid() {
        assert!((parse_memory_gi("abc")).abs() < f64::EPSILON);
    }

    #[test]
    fn test_orchestr8_path() {
        let p = orchestr8_path("state.json");
        assert!(p.ends_with(".orchestr8/state.json"));
    }

    #[test]
    fn test_json_load_missing_file() {
        #[derive(serde::Deserialize, Default)]
        struct Dummy {
            x: i32,
        }
        let path = std::path::Path::new("/tmp/orchestr8_test_nonexistent_file.json");
        let v: Dummy = json_load(path).unwrap();
        assert_eq!(v.x, 0);
    }

    #[test]
    fn test_json_roundtrip() {
        #[derive(serde::Serialize, serde::Deserialize, Default, PartialEq, Debug)]
        struct Dummy {
            x: i32,
        }
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.json");
        let val = Dummy { x: 42 };
        json_save(&val, &path).unwrap();
        let loaded: Dummy = json_load(&path).unwrap();
        assert_eq!(loaded, val);
    }
}
