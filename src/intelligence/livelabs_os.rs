// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Live Labs & Reference Cluster — Era N (phases 135–144).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn env_set(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
}

fn env_flag(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .filter(|s| !s.is_empty())
        .is_some_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn repo_file(rel: &str) -> PathBuf {
    repo_root().join(rel)
}

fn repo_file_exists(rel: &str) -> bool {
    repo_file(rel).exists()
}

fn kubeconfig_available() -> bool {
    if env_set("KUBECONFIG") {
        return true;
    }
    std::env::var("HOME")
        .ok()
        .map(|h| PathBuf::from(h).join(".kube/config"))
        .is_some_and(|p| p.exists())
}

fn labs_live_enabled() -> bool {
    env_flag("AETHER_LABS_LIVE")
}

fn e2e_kind_enabled() -> bool {
    env_flag("AETHER_E2E_KIND")
}

fn kind_cluster_name() -> String {
    std::env::var("AETHER_KIND_CLUSTER").unwrap_or_else(|_| "kind-aether-playwright".into())
}

// ── Overview ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLabsFeature {
    pub phase: u32,
    pub name: String,
    pub status: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveLabsOverview {
    pub generated_at: String,
    pub era: String,
    pub feature_count: u32,
    pub labs_live_enabled: bool,
    pub kubeconfig_available: bool,
    pub features: Vec<LiveLabsFeature>,
}

pub fn build_livelabs_overview() -> LiveLabsOverview {
    LiveLabsOverview {
        generated_at: crate::resources::now_rfc3339(),
        era: "N".into(),
        feature_count: 9,
        labs_live_enabled: labs_live_enabled(),
        kubeconfig_available: kubeconfig_available(),
        features: vec![
            feat(
                135,
                "Reference cluster runner",
                "/api/intelligence/livelabs/reference-runner",
            ),
            feat(
                136,
                "Kind Playwright fixture",
                "/api/intelligence/livelabs/kind-fixture",
            ),
            feat(
                137,
                "Labs live smoke",
                "/api/intelligence/livelabs/live-smoke",
            ),
            feat(
                138,
                "Post-deploy verify",
                "/api/intelligence/livelabs/post-deploy-verify",
            ),
            feat(
                139,
                "Kubernetes live lab",
                "/api/intelligence/livelabs/kubernetes-lab",
            ),
            feat(
                140,
                "Advanced runtime labs",
                "/api/intelligence/livelabs/advanced-runtime-labs",
            ),
            feat(
                141,
                "CI pipeline jobs",
                "/api/intelligence/livelabs/ci-pipeline",
            ),
            feat(
                142,
                "Cluster exec E2E",
                "/api/intelligence/livelabs/cluster-exec",
            ),
            feat(144, "Live labs hub", "/api/intelligence/livelabs/overview"),
        ],
    }
}

fn feat(phase: u32, name: &str, endpoint: &str) -> LiveLabsFeature {
    LiveLabsFeature {
        phase,
        name: name.into(),
        status: "ship".into(),
        endpoint: endpoint.into(),
    }
}

// ── Phase 135: Reference cluster runner ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferenceRunnerReport {
    pub status: String,
    pub generated_at: String,
    pub labs_live_enabled: bool,
    pub kubeconfig_available: bool,
    pub make_target: String,
    pub script: String,
    pub hint: String,
}

pub fn build_reference_runner_report() -> ReferenceRunnerReport {
    let ready = kubeconfig_available();
    ReferenceRunnerReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        labs_live_enabled: labs_live_enabled(),
        kubeconfig_available: ready,
        make_target: "reference-cluster-live".into(),
        script: "scripts/labs-live-smoke.sh".into(),
        hint: if ready {
            if labs_live_enabled() {
                "Live mode active — run `make reference-cluster-live` against your kubeconfig."
                    .into()
            } else {
                "Set AETHER_LABS_LIVE=1 to enable live KubeVirt deploy smoke.".into()
            }
        } else {
            "Export KUBECONFIG or place credentials at ~/.kube/config before live smoke.".into()
        },
    }
}

// ── Phase 136: Kind Playwright fixture ────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KindFixtureReport {
    pub status: String,
    pub generated_at: String,
    pub e2e_kind_enabled: bool,
    pub cluster_name: String,
    pub context: String,
    pub pod_name: String,
    pub script: String,
    pub playwright_spec: String,
}

pub fn build_kind_fixture_report() -> KindFixtureReport {
    let cluster = kind_cluster_name();
    KindFixtureReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        e2e_kind_enabled: e2e_kind_enabled(),
        cluster_name: cluster.clone(),
        context: format!("kind-{cluster}"),
        pod_name: "nginx-playwright".into(),
        script: "scripts/kind-playwright-fixture.sh".into(),
        playwright_spec: "web/dashboard/tests/cluster-exec-terminal.spec.ts".into(),
    }
}

// ── Phase 137: Labs live smoke ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSmokeStep {
    pub id: String,
    pub script: String,
    pub requires_kubeconfig: bool,
    pub requires_live_flag: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveSmokeReport {
    pub status: String,
    pub generated_at: String,
    pub labs_live_enabled: bool,
    pub kubeconfig_available: bool,
    pub steps: Vec<LiveSmokeStep>,
}

pub fn build_live_smoke_report() -> LiveSmokeReport {
    LiveSmokeReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        labs_live_enabled: labs_live_enabled(),
        kubeconfig_available: kubeconfig_available(),
        steps: vec![
            LiveSmokeStep {
                id: "reference-cluster".into(),
                script: "scripts/reference-cluster-e2e.sh".into(),
                requires_kubeconfig: true,
                requires_live_flag: true,
            },
            LiveSmokeStep {
                id: "labs-e2e".into(),
                script: "scripts/labs-e2e.sh".into(),
                requires_kubeconfig: false,
                requires_live_flag: false,
            },
        ],
    }
}

// ── Phase 138: Post-deploy verify ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDeployVerifyStep {
    pub id: String,
    pub script: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDeployVerifyReport {
    pub status: String,
    pub generated_at: String,
    pub default_api: String,
    pub steps: Vec<PostDeployVerifyStep>,
}

pub fn build_post_deploy_verify_report() -> PostDeployVerifyReport {
    PostDeployVerifyReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        default_api: std::env::var("AETHER_API").unwrap_or_else(|_| "http://127.0.0.1:5090".into()),
        steps: vec![
            PostDeployVerifyStep {
                id: "health".into(),
                script: "scripts/post-deploy-verify.sh".into(),
                endpoint: Some("/health".into()),
            },
            PostDeployVerifyStep {
                id: "remote-ux".into(),
                script: "scripts/remote-api-ux-verify.sh".into(),
                endpoint: Some("/api/server".into()),
            },
            PostDeployVerifyStep {
                id: "k8s-smoke".into(),
                script: "scripts/k8s-api-smoke.sh".into(),
                endpoint: Some("/api/cluster/workloads".into()),
            },
        ],
    }
}

// ── Phase 139: Kubernetes live lab ────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesLabSpec {
    pub name: String,
    pub spec_path: String,
    pub runtime: String,
    pub exists: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KubernetesLabReport {
    pub status: String,
    pub generated_at: String,
    pub labs_live_enabled: bool,
    pub kubeconfig_available: bool,
    pub script: String,
    pub specs: Vec<KubernetesLabSpec>,
}

pub fn build_kubernetes_lab_report() -> KubernetesLabReport {
    let specs = vec![
        lab_spec(
            "kubernetes",
            "examples/labs/kubernetes/workload.yaml",
            "kube",
        ),
        lab_spec(
            "k8s-advanced",
            "examples/workload-k8s-advanced.yaml",
            "kube",
        ),
    ];
    KubernetesLabReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        labs_live_enabled: labs_live_enabled(),
        kubeconfig_available: kubeconfig_available(),
        script: "scripts/k8s-labs-e2e.sh".into(),
        specs,
    }
}

fn lab_spec(name: &str, spec_path: &str, runtime: &str) -> KubernetesLabSpec {
    KubernetesLabSpec {
        name: name.into(),
        spec_path: spec_path.into(),
        runtime: runtime.into(),
        exists: repo_file_exists(spec_path),
    }
}

// ── Phase 140: Advanced runtime labs ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedRuntimeLabReport {
    pub status: String,
    pub generated_at: String,
    pub labs_live_enabled: bool,
    pub kubeconfig_available: bool,
    pub specs: Vec<KubernetesLabSpec>,
}

pub fn build_advanced_runtime_labs_report() -> AdvancedRuntimeLabReport {
    AdvancedRuntimeLabReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        labs_live_enabled: labs_live_enabled(),
        kubeconfig_available: kubeconfig_available(),
        specs: vec![lab_spec(
            "kubevirt",
            "examples/labs/kubevirt/workload.yaml",
            "kubevirt",
        )],
    }
}

// ── Phase 141: CI pipeline ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiPipelineJob {
    pub id: String,
    pub workflow: String,
    pub description: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiPipelineReport {
    pub status: String,
    pub generated_at: String,
    pub jobs: Vec<CiPipelineJob>,
}

pub fn build_ci_pipeline_report() -> CiPipelineReport {
    CiPipelineReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        jobs: vec![
            CiPipelineJob {
                id: "labs-e2e".into(),
                workflow: "ci.yml".into(),
                description: "Validate + dry-run lab specs".into(),
                command: "scripts/labs-e2e.sh".into(),
            },
            CiPipelineJob {
                id: "k8s-live-e2e".into(),
                workflow: "ci.yml".into(),
                description: "Kind live deploy + cascade stop".into(),
                command: "scripts/k8s-labs-e2e.sh".into(),
            },
            CiPipelineJob {
                id: "dashboard-exec-e2e".into(),
                workflow: "ci.yml".into(),
                description: "Playwright cluster exec terminal".into(),
                command: "scripts/kind-playwright-fixture.sh".into(),
            },
            CiPipelineJob {
                id: "reference-cluster-live".into(),
                workflow: "manual".into(),
                description: "KubeVirt live smoke".into(),
                command: "make reference-cluster-live".into(),
            },
        ],
    }
}

// ── Phase 142: Cluster exec E2E ───────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterExecReport {
    pub status: String,
    pub generated_at: String,
    pub e2e_kind_enabled: bool,
    pub fixture_script: String,
    pub playwright_spec: String,
    pub cluster_context: String,
    pub target_pod: String,
    pub apis: Vec<String>,
}

pub fn build_cluster_exec_report() -> ClusterExecReport {
    let cluster = kind_cluster_name();
    ClusterExecReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        e2e_kind_enabled: e2e_kind_enabled(),
        fixture_script: "scripts/kind-playwright-fixture.sh".into(),
        playwright_spec: "web/dashboard/tests/cluster-exec-terminal.spec.ts".into(),
        cluster_context: format!("kind-{cluster}"),
        target_pod: "nginx-playwright".into(),
        apis: vec![
            "/api/cluster/exec".into(),
            "/api/cluster/port-forward".into(),
            "/api/cluster/workloads".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_has_nine_features() {
        assert_eq!(build_livelabs_overview().feature_count, 9);
        assert_eq!(build_livelabs_overview().era, "N");
    }

    #[test]
    fn kubernetes_lab_lists_specs() {
        let r = build_kubernetes_lab_report();
        assert!(r.specs.iter().any(|s| s.name == "kubernetes"));
    }

    #[test]
    fn ci_pipeline_has_jobs() {
        assert!(!build_ci_pipeline_report().jobs.is_empty());
    }
}
