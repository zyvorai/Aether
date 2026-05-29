// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! GitOps reconciliation
//!
//! Monitors a Git repository for workload YAML changes and provides
//! sync primitives so the caller can apply them to the control plane.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for GitOps reconciliation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsConfig {
    /// Git repository URL (HTTPS or SSH).
    pub repo_url: String,

    /// Branch to watch.
    #[serde(default = "default_branch")]
    pub branch: String,

    /// Path within the repo to watch for YAML files.
    #[serde(default = "default_path")]
    pub path: String,

    /// How often to check for changes (seconds).
    #[serde(default = "default_poll_interval")]
    pub poll_interval_secs: u64,

    /// Whether to auto-apply detected changes.
    #[serde(default)]
    pub auto_apply: bool,

    /// Default kubectl context for sync apply (overridden per environment).
    #[serde(default)]
    pub kube_context: Option<String>,

    /// Default namespace for sync apply (overridden per environment).
    #[serde(default)]
    pub kube_namespace: Option<String>,

    /// Multi-environment GitOps targets (context/namespace/path per env).
    #[serde(default)]
    pub environments: Vec<GitOpsEnvironment>,
}

/// Per-environment GitOps target for multi-cluster pipelines.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct GitOpsEnvironment {
    pub name: String,
    #[serde(default)]
    pub kube_context: Option<String>,
    #[serde(default)]
    pub kube_namespace: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
}

impl GitOpsConfig {
    /// Resolve kubectl context for an environment name (or default).
    pub fn resolve_kube_context(&self, environment: Option<&str>) -> Option<&str> {
        if let Some(env_name) = environment {
            if let Some(env) = self.environments.iter().find(|e| e.name == env_name) {
                return env.kube_context.as_deref();
            }
        }
        self.kube_context.as_deref()
    }

    /// Resolve namespace for an environment name (or default).
    pub fn resolve_kube_namespace(&self, environment: Option<&str>) -> Option<&str> {
        if let Some(env_name) = environment {
            if let Some(env) = self.environments.iter().find(|e| e.name == env_name) {
                return env.kube_namespace.as_deref();
            }
        }
        self.kube_namespace.as_deref()
    }

    /// Resolve deploy target; uses explicit context or federation recommendation.
    pub async fn resolve_deploy_target(
        &self,
        environment: Option<&str>,
        spec: &crate::spec::Workload,
    ) -> anyhow::Result<DeployTarget> {
        if let Some(ctx) = self.resolve_kube_context(environment) {
            return Ok(DeployTarget {
                kube_context: ctx.to_string(),
                kube_namespace: self
                    .resolve_kube_namespace(environment)
                    .map(str::to_string),
                source: if environment.is_some() {
                    "environment".into()
                } else {
                    "explicit".into()
                },
            });
        }
        if let Some(cluster) = crate::fleet::federation::recommend_cluster(spec).await? {
            return Ok(DeployTarget {
                kube_context: cluster,
                kube_namespace: self.resolve_kube_namespace(environment).map(str::to_string),
                source: "federation".into(),
            });
        }
        anyhow::bail!(
            "no kube_context configured and federation could not recommend a cluster — set GitOps kube_context or AETHER_FEDERATION_CLUSTERS"
        )
    }
}

/// Resolved Kubernetes apply target for GitOps sync.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeployTarget {
    pub kube_context: String,
    pub kube_namespace: Option<String>,
    pub source: String,
}

fn default_branch() -> String {
    "main".to_string()
}

fn default_path() -> String {
    ".".to_string()
}

fn default_poll_interval() -> u64 {
    60
}

impl Default for GitOpsConfig {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            branch: default_branch(),
            path: default_path(),
            poll_interval_secs: default_poll_interval(),
            auto_apply: false,
            kube_context: None,
            kube_namespace: None,
            environments: Vec::new(),
        }
    }
}

/// Current status of the GitOps controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsStatus {
    /// Whether GitOps is configured.
    pub configured: bool,

    /// The repository URL being watched.
    pub repo_url: String,

    /// The branch being watched.
    pub branch: String,

    /// RFC 3339 timestamp of the last successful sync.
    pub last_sync: Option<String>,

    /// Latest commit SHA observed.
    pub last_commit: Option<String>,

    /// Total number of successful syncs.
    pub sync_count: u64,

    /// Total number of sync errors.
    pub error_count: u64,

    /// Most recent error message, if any.
    pub last_error: Option<String>,

    /// Workload YAML changes from the last successful sync (API/dashboard).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_changes: Option<Vec<GitOpsChange>>,

    /// Confidential compliance audit from the last successful sync.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_confidential_compliance: Option<Vec<GitOpsConfidentialAudit>>,
}

impl GitOpsStatus {
    /// Create a new status from config.
    pub fn from_config(config: &GitOpsConfig) -> Self {
        Self {
            configured: true,
            repo_url: config.repo_url.clone(),
            branch: config.branch.clone(),
            last_sync: None,
            last_commit: None,
            sync_count: 0,
            error_count: 0,
            last_error: None,
            last_changes: None,
            last_confidential_compliance: None,
        }
    }
}

/// A single change detected in the Git repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsChange {
    /// Relative path of the changed file.
    pub file_path: String,

    /// Kind of change.
    pub change_type: ChangeType,

    /// Commit SHA that introduced the change.
    pub commit: String,
}

/// Kind of file change detected by Git.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChangeType {
    Added,
    Modified,
    Deleted,
}

impl std::fmt::Display for ChangeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChangeType::Added => write!(f, "Added"),
            ChangeType::Modified => write!(f, "Modified"),
            ChangeType::Deleted => write!(f, "Deleted"),
        }
    }
}

/// GitOps controller that monitors a repository and detects workload changes.
pub struct GitOpsController {
    /// Active configuration.
    pub config: GitOpsConfig,

    /// Current sync status.
    status: GitOpsStatus,

    /// Local clone directory.
    pub repo_dir: PathBuf,
}

impl GitOpsController {
    /// Create a new controller from the given config.
    pub fn new(config: GitOpsConfig) -> Self {
        let repo_dir = Self::default_repo_dir(&config.repo_url);
        let status = GitOpsStatus::from_config(&config);
        Self {
            config,
            status,
            repo_dir,
        }
    }

    /// Return a reference to the current status.
    pub fn status(&self) -> &GitOpsStatus {
        &self.status
    }

    /// Clone the repository (or pull if it already exists).
    ///
    /// The clone is placed under `~/.aether/gitops/<repo-name>`.
    pub fn init_repo(&mut self) -> anyhow::Result<()> {
        if self.repo_dir.exists() {
            // Already cloned -- pull latest
            let output = std::process::Command::new("git")
                .args(["pull", "origin", &self.config.branch])
                .current_dir(&self.repo_dir)
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to run git pull: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("git pull failed: {}", stderr.trim());
            }
        } else {
            // Fresh clone
            if let Some(parent) = self.repo_dir.parent() {
                std::fs::create_dir_all(parent)?;
            }

            let output = std::process::Command::new("git")
                .args([
                    "clone",
                    "--branch",
                    &self.config.branch,
                    "--single-branch",
                    &self.config.repo_url,
                    &self.repo_dir.to_string_lossy(),
                ])
                .output()
                .map_err(|e| anyhow::anyhow!("Failed to run git clone: {}", e))?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("git clone failed: {}", stderr.trim());
            }
        }

        // Record the current HEAD commit
        self.update_last_commit()?;

        Ok(())
    }

    /// Detect changes in the most recent commit by running
    /// `git diff --name-status HEAD~1..HEAD -- '*.yaml' '*.yml'`.
    ///
    /// If the repository only has one commit (no parent), returns an empty
    /// change list instead of failing.
    pub fn detect_changes(&mut self) -> anyhow::Result<Vec<GitOpsChange>> {
        // Pull latest first
        let pull = std::process::Command::new("git")
            .args(["pull", "origin", &self.config.branch])
            .current_dir(&self.repo_dir)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run git pull: {}", e))?;

        if !pull.status.success() {
            let stderr = String::from_utf8_lossy(&pull.stderr);
            anyhow::bail!("git pull failed: {}", stderr.trim());
        }

        // Check if HEAD~1 exists (first commit has no parent)
        let has_parent = std::process::Command::new("git")
            .args(["rev-parse", "--verify", "HEAD~1"])
            .current_dir(&self.repo_dir)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !has_parent {
            self.update_last_commit()?;
            return Ok(vec![]);
        }

        // Validate the watch path (reject path traversal)
        let watch_path = if self.config.path == "." {
            String::new()
        } else {
            let normalized = self.config.path.replace('\\', "/");
            if normalized.contains("..") {
                anyhow::bail!(
                    "GitOps watch path '{}' contains '..', which is not allowed",
                    self.config.path
                );
            }
            if normalized.starts_with('/') {
                anyhow::bail!(
                    "GitOps watch path '{}' must be relative, not absolute",
                    self.config.path
                );
            }
            format!("{}/", normalized.trim_end_matches('/'))
        };

        // Build diff args
        let mut args = vec![
            "diff".to_string(),
            "--name-status".to_string(),
            "HEAD~1..HEAD".to_string(),
            "--".to_string(),
        ];

        if watch_path.is_empty() {
            args.push("*.yaml".to_string());
            args.push("*.yml".to_string());
        } else {
            args.push(format!("{}*.yaml", watch_path));
            args.push(format!("{}*.yml", watch_path));
        }

        let diff = std::process::Command::new("git")
            .args(&args)
            .current_dir(&self.repo_dir)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run git diff: {}", e))?;

        if !diff.status.success() {
            let stderr = String::from_utf8_lossy(&diff.stderr);
            anyhow::bail!("git diff failed: {}", stderr.trim());
        }

        let commit = self.current_commit()?;
        let stdout = String::from_utf8_lossy(&diff.stdout);
        let changes = parse_diff_output(&stdout, &commit);

        self.update_last_commit()?;

        Ok(changes)
    }

    /// Pull latest, detect changes, and update status counters.
    ///
    /// This does NOT deploy anything -- the caller is responsible for
    /// interpreting the returned changes and applying them.
    pub fn sync(&mut self) -> anyhow::Result<Vec<GitOpsChange>> {
        let now = chrono::Utc::now().to_rfc3339();

        match self.detect_changes() {
            Ok(changes) => {
                self.status.last_sync = Some(now);
                self.status.sync_count += 1;
                self.status.last_error = None;
                Ok(changes)
            }
            Err(e) => {
                self.status.error_count += 1;
                self.status.last_error = Some(e.to_string());
                Err(e)
            }
        }
    }

    /// Derive the default local clone directory from a repository URL.
    ///
    /// Extracts the repository name from HTTPS or SSH URLs and returns
    /// `~/.aether/gitops/<name>`.
    pub fn default_repo_dir(repo_url: &str) -> PathBuf {
        let name = extract_repo_name(repo_url);
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".aether").join("gitops").join(name)
    }

    // -- private helpers --------------------------------------------------

    fn current_commit(&self) -> anyhow::Result<String> {
        let output = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.repo_dir)
            .output()
            .map_err(|e| anyhow::anyhow!("Failed to run git rev-parse: {}", e))?;

        if !output.status.success() {
            anyhow::bail!("git rev-parse HEAD failed");
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn update_last_commit(&mut self) -> anyhow::Result<()> {
        let sha = self.current_commit()?;
        self.status.last_commit = Some(sha);
        Ok(())
    }
}

/// Extract a human-readable repo name from an HTTPS or SSH URL.
///
/// Examples:
///   `https://github.com/org/repo.git` -> `repo`
///   `git@github.com:org/repo.git`     -> `repo`
///   `https://github.com/org/repo`     -> `repo`
fn extract_repo_name(url: &str) -> String {
    // Take the last path segment, strip optional `.git` suffix
    let stripped = url.trim_end_matches('/');
    let name = stripped
        .rsplit('/')
        .next()
        // Handle SSH colon-separated paths (git@host:org/repo.git)
        .or_else(|| stripped.rsplit(':').next())
        .unwrap_or("repo");

    name.trim_end_matches(".git").to_string()
}

/// Parse the output of `git diff --name-status` into `GitOpsChange` entries.
///
/// Git uses tab-separated output: `<status>\t<filepath>`.
fn parse_diff_output(output: &str, commit: &str) -> Vec<GitOpsChange> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() {
                return None;
            }

            // git diff --name-status uses tab separation
            let (status, file_path) = line.split_once('\t')
                .or_else(|| line.split_once(' '))?;
            let file_path = file_path.to_string();

            if file_path.is_empty() {
                return None;
            }

            let change_type = match status.trim() {
                "A" => ChangeType::Added,
                "M" => ChangeType::Modified,
                "D" => ChangeType::Deleted,
                _ => return None,
            };

            Some(GitOpsChange {
                file_path,
                change_type,
                commit: commit.to_string(),
            })
        })
        .collect()
}

/// Confidential policy issues for a workload spec (GitOps pre-apply audit).
pub fn confidential_policy_issues(spec: &crate::spec::Workload) -> Vec<String> {
    crate::ragnarok::scheduling::gitops_policy_issues(spec)
}

/// Per-file confidential compliance from GitOps sync (YAML in repo).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitOpsConfidentialAudit {
    pub file_path: String,
    pub workload: Option<String>,
    pub confidential_enabled: bool,
    pub gitops_issues: Vec<String>,
    pub sovereign_compliant: Option<bool>,
    pub sovereign_violations: Vec<String>,
}

/// Scan changed workload YAML files for confidential / sovereign policy issues.
pub fn audit_confidential_changes(
    repo_dir: &std::path::Path,
    changes: &[GitOpsChange],
) -> Vec<GitOpsConfidentialAudit> {
    use crate::ragnarok::sovereign::{evaluate, SovereignConfig};
    use crate::spec::Workload;

    let config = SovereignConfig::from_env();
    changes
        .iter()
        .filter(|c| c.change_type != ChangeType::Deleted)
        .filter(|c| {
            c.file_path.ends_with(".yaml") || c.file_path.ends_with(".yml")
        })
        .filter_map(|c| {
            let path = repo_dir.join(&c.file_path);
            let content = std::fs::read_to_string(&path).ok()?;
            let spec: Workload = serde_yaml::from_str(&content).ok()?;
            let enabled = spec
                .confidential
                .as_ref()
                .is_some_and(|conf| conf.enabled);
            if !enabled {
                return Some(GitOpsConfidentialAudit {
                    file_path: c.file_path.clone(),
                    workload: Some(spec.metadata.name.clone()),
                    confidential_enabled: false,
                    gitops_issues: vec![],
                    sovereign_compliant: None,
                    sovereign_violations: vec![],
                });
            }
            let gitops_issues = confidential_policy_issues(&spec);
            let verdict = evaluate(&spec, &config);
            Some(GitOpsConfidentialAudit {
                file_path: c.file_path.clone(),
                workload: Some(spec.metadata.name.clone()),
                confidential_enabled: true,
                gitops_issues,
                sovereign_compliant: Some(verdict.compliant),
                sovereign_violations: verdict.violations,
            })
        })
        .collect()
}

/// Format a `GitOpsStatus` for CLI display.
pub fn format_status(status: &GitOpsStatus) -> String {
    let mut out = String::new();

    out.push_str("GitOps Status\n");
    out.push_str(&format!("  Configured:  {}\n", status.configured));
    out.push_str(&format!("  Repository:  {}\n", status.repo_url));
    out.push_str(&format!("  Branch:      {}\n", status.branch));
    out.push_str(&format!(
        "  Last Sync:   {}\n",
        status.last_sync.as_deref().unwrap_or("-")
    ));
    out.push_str(&format!(
        "  Last Commit: {}\n",
        status
            .last_commit
            .as_deref()
            .map(|s| if s.len() > 12 { &s[..12] } else { s })
            .unwrap_or("-")
    ));
    out.push_str(&format!("  Sync Count:  {}\n", status.sync_count));
    out.push_str(&format!("  Error Count: {}\n", status.error_count));

    if let Some(err) = &status.last_error {
        out.push_str(&format!("  Last Error:  {}\n", err));
    }

    out
}

// =========================================================================
// Tests
// =========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ----- GitOpsConfig defaults -----------------------------------------

    #[test]
    fn test_config_defaults() {
        let config = GitOpsConfig::default();
        assert_eq!(config.branch, "main");
        assert_eq!(config.path, ".");
        assert_eq!(config.poll_interval_secs, 60);
        assert!(!config.auto_apply);
        assert!(config.repo_url.is_empty());
        assert!(config.kube_context.is_none());
        assert!(config.environments.is_empty());
    }

    #[test]
    fn test_gitops_resolve_kube_context_environment() {
        let config = GitOpsConfig {
            repo_url: "https://example.com/repo.git".into(),
            kube_context: Some("dev-context".into()),
            kube_namespace: Some("dev".into()),
            environments: vec![GitOpsEnvironment {
                name: "prod".into(),
                kube_context: Some("prod-context".into()),
                kube_namespace: Some("production".into()),
                path: Some("envs/prod".into()),
            }],
            ..Default::default()
        };
        assert_eq!(config.resolve_kube_context(Some("prod")), Some("prod-context"));
        assert_eq!(config.resolve_kube_namespace(Some("prod")), Some("production"));
        assert_eq!(config.resolve_kube_context(None), Some("dev-context"));
    }

    // ----- GitOpsStatus initialization -----------------------------------

    #[test]
    fn test_status_from_config() {
        let config = GitOpsConfig {
            repo_url: "https://github.com/org/infra.git".to_string(),
            branch: "develop".to_string(),
            ..Default::default()
        };

        let status = GitOpsStatus::from_config(&config);
        assert!(status.configured);
        assert_eq!(status.repo_url, "https://github.com/org/infra.git");
        assert_eq!(status.branch, "develop");
        assert!(status.last_sync.is_none());
        assert!(status.last_commit.is_none());
        assert_eq!(status.sync_count, 0);
        assert_eq!(status.error_count, 0);
        assert!(status.last_error.is_none());
    }

    // ----- default_repo_dir with various URL formats ---------------------

    #[test]
    fn test_default_repo_dir_https_with_git_suffix() {
        let dir = GitOpsController::default_repo_dir("https://github.com/org/my-repo.git");
        assert!(dir.ends_with("gitops/my-repo"));
    }

    #[test]
    fn test_default_repo_dir_https_without_git_suffix() {
        let dir = GitOpsController::default_repo_dir("https://github.com/org/my-repo");
        assert!(dir.ends_with("gitops/my-repo"));
    }

    #[test]
    fn test_default_repo_dir_ssh() {
        let dir = GitOpsController::default_repo_dir("git@github.com:org/infra.git");
        assert!(dir.ends_with("gitops/infra"));
    }

    #[test]
    fn test_default_repo_dir_trailing_slash() {
        let dir = GitOpsController::default_repo_dir("https://github.com/org/repo/");
        assert!(dir.ends_with("gitops/repo"));
    }

    // ----- ChangeType Display / serialization ----------------------------

    #[test]
    fn test_change_type_display() {
        assert_eq!(ChangeType::Added.to_string(), "Added");
        assert_eq!(ChangeType::Modified.to_string(), "Modified");
        assert_eq!(ChangeType::Deleted.to_string(), "Deleted");
    }

    #[test]
    fn test_change_type_serde_roundtrip() {
        let original = ChangeType::Modified;
        let json = serde_json::to_string(&original).unwrap();
        let deserialized: ChangeType = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, original);
    }

    // ----- Config serde roundtrip ----------------------------------------

    #[test]
    fn test_config_serde_roundtrip() {
        let config = GitOpsConfig {
            repo_url: "https://github.com/org/deploy.git".to_string(),
            branch: "staging".to_string(),
            path: "k8s/".to_string(),
            poll_interval_secs: 30,
            auto_apply: true,
            kube_context: None,
            kube_namespace: None,
            environments: vec![],
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: GitOpsConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.repo_url, config.repo_url);
        assert_eq!(deserialized.branch, config.branch);
        assert_eq!(deserialized.path, config.path);
        assert_eq!(deserialized.poll_interval_secs, config.poll_interval_secs);
        assert_eq!(deserialized.auto_apply, config.auto_apply);
    }

    #[test]
    fn test_config_serde_defaults_when_omitted() {
        let json = r#"{"repo_url":"https://github.com/org/repo.git"}"#;
        let config: GitOpsConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.branch, "main");
        assert_eq!(config.path, ".");
        assert_eq!(config.poll_interval_secs, 60);
        assert!(!config.auto_apply);
    }

    // ----- parse_diff_output ---------------------------------------------

    #[test]
    fn test_parse_diff_output_mixed() {
        let output = "A\tworkloads/new-app.yaml\nM\tworkloads/api.yml\nD\tworkloads/old.yaml\n";
        let changes = parse_diff_output(output, "abc123");

        assert_eq!(changes.len(), 3);
        assert_eq!(changes[0].change_type, ChangeType::Added);
        assert_eq!(changes[0].file_path, "workloads/new-app.yaml");
        assert_eq!(changes[1].change_type, ChangeType::Modified);
        assert_eq!(changes[2].change_type, ChangeType::Deleted);
        assert_eq!(changes[0].commit, "abc123");
    }

    #[test]
    fn test_parse_diff_output_empty() {
        let changes = parse_diff_output("", "abc123");
        assert!(changes.is_empty());
    }

    #[test]
    fn test_parse_diff_output_skips_unknown_status() {
        let output = "R100\told.yaml\tnew.yaml\n";
        let changes = parse_diff_output(output, "abc123");
        assert!(changes.is_empty());
    }

    #[test]
    fn test_audit_confidential_changes_parses_enabled_workload() {
        let dir = tempfile::tempdir().unwrap();
        let yaml = include_str!("../examples/confidential-snp.yaml");
        let path = dir.path().join("workloads/conf.yaml");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, yaml).unwrap();

        let changes = vec![GitOpsChange {
            file_path: "workloads/conf.yaml".into(),
            change_type: ChangeType::Added,
            commit: "abc".into(),
        }];
        let audits = audit_confidential_changes(dir.path(), &changes);
        assert_eq!(audits.len(), 1);
        assert!(audits[0].confidential_enabled);
        assert_eq!(audits[0].workload.as_deref(), Some("confidential-app"));
    }

    // ----- extract_repo_name ---------------------------------------------

    #[test]
    fn test_extract_repo_name_various() {
        assert_eq!(
            extract_repo_name("https://github.com/org/repo.git"),
            "repo"
        );
        assert_eq!(
            extract_repo_name("https://github.com/org/repo"),
            "repo"
        );
        assert_eq!(
            extract_repo_name("git@github.com:org/infra.git"),
            "infra"
        );
        assert_eq!(
            extract_repo_name("https://gitlab.com/team/project/"),
            "project"
        );
    }

    // ----- GitOpsController::new -----------------------------------------

    #[test]
    fn test_controller_new() {
        let config = GitOpsConfig {
            repo_url: "https://github.com/org/deploy.git".to_string(),
            branch: "main".to_string(),
            ..Default::default()
        };

        let ctrl = GitOpsController::new(config.clone());
        assert!(ctrl.status().configured);
        assert_eq!(ctrl.status().repo_url, "https://github.com/org/deploy.git");
        assert_eq!(ctrl.status().sync_count, 0);
        assert!(ctrl.repo_dir.ends_with("gitops/deploy"));
    }

    // ----- format_status -------------------------------------------------

    #[test]
    fn test_format_status_contains_fields() {
        let status = GitOpsStatus {
            configured: true,
            repo_url: "https://github.com/org/repo.git".to_string(),
            branch: "main".to_string(),
            last_sync: Some("2026-01-01T00:00:00Z".to_string()),
            last_commit: Some("abc123def456".to_string()),
            sync_count: 5,
            error_count: 1,
            last_error: Some("network timeout".to_string()),
            last_changes: None,
            last_confidential_compliance: None,
        };

        let output = format_status(&status);
        assert!(output.contains("Configured:  true"));
        assert!(output.contains("org/repo.git"));
        assert!(output.contains("main"));
        assert!(output.contains("Sync Count:  5"));
        assert!(output.contains("Error Count: 1"));
        assert!(output.contains("network timeout"));
    }
}
