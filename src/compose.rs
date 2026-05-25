//! Multi-workload compose file support
//!
//! Defines a compose specification that groups multiple workloads into a
//! single deployment unit with dependency ordering, runtime overrides,
//! and per-workload environment variables.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};

// ── Compose specification ───────────────────────────────────────────

/// Top-level compose file schema
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComposeSpec {
    /// Schema version (currently "1")
    pub version: String,
    /// Named workload entries
    pub workloads: HashMap<String, ComposeWorkload>,
}

/// A single workload entry inside a compose file
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComposeWorkload {
    /// Path to the workload YAML spec file (relative to compose file on disk)
    #[serde(default)]
    pub spec: PathBuf,
    /// Inline workload YAML (API / dashboard compose deploy)
    #[serde(default)]
    pub spec_yaml: Option<String>,
    /// Optional runtime override (e.g. "container", "kube")
    #[serde(default)]
    pub runtime: Option<String>,
    /// Names of workloads that must be started before this one
    #[serde(default)]
    pub depends_on: Vec<String>,
    /// Extra environment variables injected at deploy time
    #[serde(default)]
    pub env: HashMap<String, String>,
}

impl ComposeWorkload {
    /// Load workload spec from inline YAML or from a file path.
    pub fn load_workload(&self, base_dir: Option<&Path>) -> Result<crate::spec::Workload> {
        use crate::legacy_workload_yaml::parse_workload_yaml;

        if let Some(yaml) = &self.spec_yaml {
            let trimmed = yaml.trim();
            if trimmed.is_empty() {
                bail!("spec_yaml is empty");
            }
            return parse_workload_yaml(trimmed).context("failed to parse inline spec_yaml");
        }
        if self.spec.as_os_str().is_empty() {
            bail!("workload entry requires spec (file path) or spec_yaml (inline YAML)");
        }
        let path = if self.spec.is_absolute() {
            self.spec.clone()
        } else if let Some(base) = base_dir {
            base.join(&self.spec)
        } else {
            self.spec.clone()
        };
        crate::spec::Workload::from_file(&path)
            .with_context(|| format!("failed to load workload spec '{}'", path.display()))
    }
}

// ── Loading ─────────────────────────────────────────────────────────

/// Read and parse a compose YAML file from disk
pub fn load(path: &Path) -> Result<ComposeSpec> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read compose file '{}'", path.display()))?;
    let spec: ComposeSpec = serde_yaml::from_str(&content)
        .with_context(|| format!("failed to parse compose file '{}'", path.display()))?;
    Ok(spec)
}

// ── Dependency resolution ───────────────────────────────────────────

/// Return workload names in dependency-sorted (topological) order.
///
/// Workloads with no dependencies appear first.  If the graph contains
/// a cycle, an error is returned.
pub fn resolve_order(spec: &ComposeSpec) -> Result<Vec<String>> {
    // Build in-degree map using Kahn's algorithm
    let mut in_degree: HashMap<&str, usize> = HashMap::new();
    // reverse adjacency: key -> list of nodes that depend on key
    let mut reverse: HashMap<&str, Vec<&str>> = HashMap::new();

    for name in spec.workloads.keys() {
        in_degree.entry(name.as_str()).or_insert(0);
        reverse.entry(name.as_str()).or_default();
    }

    for (name, workload) in &spec.workloads {
        for dep in &workload.depends_on {
            reverse.entry(dep.as_str()).or_default().push(name.as_str());
            *in_degree.entry(name.as_str()).or_insert(0) += 1;
        }
    }

    // Seed the queue with zero-degree nodes (sorted for determinism)
    let mut seeds: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();
    seeds.sort();
    let mut queue: VecDeque<&str> = seeds.into_iter().collect();

    let mut result: Vec<String> = Vec::with_capacity(spec.workloads.len());

    while let Some(node) = queue.pop_front() {
        result.push(node.to_string());
        if let Some(dependents) = reverse.get(node) {
            // Process dependents in sorted order for deterministic output
            let mut sorted_deps: Vec<&str> = dependents.clone();
            sorted_deps.sort();
            for &dep in &sorted_deps {
                let deg = in_degree.get_mut(dep).expect("node must exist");
                *deg -= 1;
                if *deg == 0 {
                    queue.push_back(dep);
                }
            }
        }
    }

    if result.len() != spec.workloads.len() {
        bail!("circular dependency detected in compose workloads");
    }

    Ok(result)
}

// ── Validation ──────────────────────────────────────────────────────

/// Validate the compose spec for structural correctness.
///
/// Checks performed:
/// - No circular dependencies
/// - Every `depends_on` entry references an existing workload
pub fn validate(spec: &ComposeSpec) -> Result<()> {
    // Check for references to non-existent workloads
    for (name, workload) in &spec.workloads {
        for dep in &workload.depends_on {
            if !spec.workloads.contains_key(dep) {
                bail!(
                    "workload '{}' depends on '{}' which is not defined in the compose file",
                    name,
                    dep
                );
            }
        }
        if workload.spec_yaml.is_none() && workload.spec.as_os_str().is_empty() {
            bail!(
                "workload '{}' must define spec (file path) or spec_yaml (inline YAML)",
                name
            );
        }
    }

    // Check for circular dependencies (resolve_order returns Err on cycles)
    resolve_order(spec)?;

    Ok(())
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------

    fn make_workload(spec_path: &str, deps: Vec<&str>) -> ComposeWorkload {
        ComposeWorkload {
            spec: PathBuf::from(spec_path),
            spec_yaml: None,
            runtime: None,
            depends_on: deps.into_iter().map(String::from).collect(),
            env: HashMap::new(),
        }
    }

    fn make_compose(entries: Vec<(&str, ComposeWorkload)>) -> ComposeSpec {
        ComposeSpec {
            version: "1".to_string(),
            workloads: entries
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect(),
        }
    }

    // ---------------------------------------------------------------
    // YAML parsing
    // ---------------------------------------------------------------

    #[test]
    fn test_parse_minimal_yaml() {
        let yaml = r#"
version: "1"
workloads:
  web:
    spec: ./web.yaml
"#;
        let spec: ComposeSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.version, "1");
        assert_eq!(spec.workloads.len(), 1);
        assert_eq!(spec.workloads["web"].spec, PathBuf::from("./web.yaml"));
        assert!(spec.workloads["web"].runtime.is_none());
        assert!(spec.workloads["web"].depends_on.is_empty());
        assert!(spec.workloads["web"].env.is_empty());
    }

    #[test]
    fn test_parse_full_yaml() {
        let yaml = r#"
version: "1"
workloads:
  database:
    spec: ./db.yaml
    runtime: container
    env:
      POSTGRES_DB: mydb
      POSTGRES_USER: admin
  api:
    spec: ./api.yaml
    runtime: kube
    depends_on:
      - database
    env:
      LOG_LEVEL: debug
  web:
    spec: ./web.yaml
    depends_on:
      - api
"#;
        let spec: ComposeSpec = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(spec.workloads.len(), 3);

        let db = &spec.workloads["database"];
        assert_eq!(db.spec, PathBuf::from("./db.yaml"));
        assert_eq!(db.runtime.as_deref(), Some("container"));
        assert!(db.depends_on.is_empty());
        assert_eq!(db.env.get("POSTGRES_DB").unwrap(), "mydb");

        let api = &spec.workloads["api"];
        assert_eq!(api.depends_on, vec!["database"]);
        assert_eq!(api.runtime.as_deref(), Some("kube"));

        let web = &spec.workloads["web"];
        assert_eq!(web.depends_on, vec!["api"]);
        assert!(web.runtime.is_none());
    }

    // ---------------------------------------------------------------
    // Dependency ordering
    // ---------------------------------------------------------------

    #[test]
    fn test_resolve_order_no_dependencies() {
        let spec = make_compose(vec![
            ("alpha", make_workload("a.yaml", vec![])),
            ("beta", make_workload("b.yaml", vec![])),
        ]);
        let order = resolve_order(&spec).unwrap();
        assert_eq!(order.len(), 2);
        // Alphabetical since no ordering constraints
        assert_eq!(order, vec!["alpha", "beta"]);
    }

    #[test]
    fn test_resolve_order_linear_chain() {
        let spec = make_compose(vec![
            ("web", make_workload("web.yaml", vec!["api"])),
            ("api", make_workload("api.yaml", vec!["database"])),
            ("database", make_workload("db.yaml", vec![])),
        ]);
        let order = resolve_order(&spec).unwrap();
        assert_eq!(order, vec!["database", "api", "web"]);
    }

    #[test]
    fn test_resolve_order_diamond() {
        // database -> api, database -> worker, api -> web, worker -> web
        let spec = make_compose(vec![
            ("web", make_workload("web.yaml", vec!["api", "worker"])),
            ("api", make_workload("api.yaml", vec!["database"])),
            ("worker", make_workload("worker.yaml", vec!["database"])),
            ("database", make_workload("db.yaml", vec![])),
        ]);
        let order = resolve_order(&spec).unwrap();

        let pos = |name: &str| order.iter().position(|n| n == name).unwrap();
        assert!(pos("database") < pos("api"));
        assert!(pos("database") < pos("worker"));
        assert!(pos("api") < pos("web"));
        assert!(pos("worker") < pos("web"));
    }

    #[test]
    fn test_resolve_order_single_workload() {
        let spec = make_compose(vec![
            ("solo", make_workload("solo.yaml", vec![])),
        ]);
        let order = resolve_order(&spec).unwrap();
        assert_eq!(order, vec!["solo"]);
    }

    // ---------------------------------------------------------------
    // Circular dependency detection
    // ---------------------------------------------------------------

    #[test]
    fn test_circular_dependency_direct() {
        let spec = make_compose(vec![
            ("a", make_workload("a.yaml", vec!["b"])),
            ("b", make_workload("b.yaml", vec!["a"])),
        ]);
        let err = resolve_order(&spec).unwrap_err();
        assert!(
            err.to_string().contains("circular"),
            "expected circular error, got: {}",
            err
        );
    }

    #[test]
    fn test_circular_dependency_transitive() {
        let spec = make_compose(vec![
            ("a", make_workload("a.yaml", vec!["b"])),
            ("b", make_workload("b.yaml", vec!["c"])),
            ("c", make_workload("c.yaml", vec!["a"])),
        ]);
        assert!(resolve_order(&spec).is_err());
    }

    // ---------------------------------------------------------------
    // Validation
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_valid_spec() {
        let spec = make_compose(vec![
            ("db", make_workload("db.yaml", vec![])),
            ("api", make_workload("api.yaml", vec!["db"])),
        ]);
        assert!(validate(&spec).is_ok());
    }

    #[test]
    fn test_validate_missing_dependency() {
        let spec = make_compose(vec![
            ("api", make_workload("api.yaml", vec!["database"])),
        ]);
        let err = validate(&spec).unwrap_err();
        assert!(
            err.to_string().contains("database"),
            "expected missing dep error, got: {}",
            err
        );
        assert!(
            err.to_string().contains("not defined"),
            "expected 'not defined' message, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_circular_dependency() {
        let spec = make_compose(vec![
            ("a", make_workload("a.yaml", vec!["b"])),
            ("b", make_workload("b.yaml", vec!["a"])),
        ]);
        assert!(validate(&spec).is_err());
    }

    #[test]
    fn test_validate_empty_workloads() {
        let spec = ComposeSpec {
            version: "1".to_string(),
            workloads: HashMap::new(),
        };
        assert!(validate(&spec).is_ok());
    }

    #[test]
    fn test_validate_requires_spec_or_spec_yaml() {
        let spec = make_compose(vec![(
            "web",
            ComposeWorkload {
                spec: PathBuf::new(),
                spec_yaml: None,
                runtime: None,
                depends_on: vec![],
                env: HashMap::new(),
            },
        )]);
        assert!(validate(&spec).is_err());
    }

    #[test]
    fn test_parse_inline_spec_yaml() {
        let yaml = r#"
version: "1"
workloads:
  web:
    spec_yaml: |
      apiVersion: aether/v1
      kind: Workload
      metadata:
        name: inline-web
"#;
        let spec: ComposeSpec = serde_yaml::from_str(yaml).unwrap();
        assert!(spec.workloads["web"].spec_yaml.is_some());
    }

    // ---------------------------------------------------------------
    // Load (file I/O)
    // ---------------------------------------------------------------

    #[test]
    fn test_load_nonexistent_file() {
        let result = load(Path::new("/tmp/does-not-exist-aether.yaml"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_valid_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("compose.yaml");
        std::fs::write(
            &path,
            r#"
version: "1"
workloads:
  app:
    spec: ./app.yaml
"#,
        )
        .unwrap();

        let spec = load(&path).unwrap();
        assert_eq!(spec.version, "1");
        assert_eq!(spec.workloads.len(), 1);
    }

    #[test]
    fn test_load_invalid_yaml() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("bad.yaml");
        std::fs::write(&path, "not: [valid: yaml: {{").unwrap();

        assert!(load(&path).is_err());
    }
}
