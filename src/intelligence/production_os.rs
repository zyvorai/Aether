// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Production & Trust Layer — Era M (phases 125–134).

use crate::fleet::edge::EdgeStore;
use crate::hosted::tenant::TenantStore;
use serde::{Deserialize, Serialize};

fn env_set(key: &str) -> bool {
    std::env::var(key)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .is_some()
}

// ── Overview ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionFeature {
    pub phase: u32,
    pub name: String,
    pub status: String,
    pub endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionOverview {
    pub generated_at: String,
    pub era: String,
    pub feature_count: u32,
    pub features: Vec<ProductionFeature>,
}

pub fn build_production_overview() -> ProductionOverview {
    ProductionOverview {
        generated_at: crate::resources::now_rfc3339(),
        era: "M".into(),
        feature_count: 10,
        features: vec![
            feat(125, "Production scorecard", "/api/intelligence/production/scorecard"),
            feat(126, "Auth plane status", "/api/intelligence/production/auth-plane"),
            feat(127, "OPA enforcement plane", "/api/intelligence/production/opa-plane"),
            feat(128, "HA backends plane", "/api/intelligence/production/ha-plane"),
            feat(129, "Durability plane", "/api/intelligence/production/durability-plane"),
            feat(130, "Hosted SaaS plane", "/api/intelligence/production/hosted-plane"),
            feat(131, "Edge fleet plane", "/api/intelligence/production/edge-fleet"),
            feat(132, "Post-deploy verify manifest", "/api/intelligence/production/post-deploy-manifest"),
            feat(133, "CI smoke manifest", "/api/intelligence/production/ci-smoke-manifest"),
            feat(134, "Production trust hub", "/api/intelligence/production/overview"),
        ],
    }
}

fn feat(phase: u32, name: &str, endpoint: &str) -> ProductionFeature {
    ProductionFeature {
        phase,
        name: name.into(),
        status: "ship".into(),
        endpoint: endpoint.into(),
    }
}

// ── Phase 125: Scorecard ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionCheck {
    pub id: String,
    pub label: String,
    pub ok: bool,
    pub severity: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductionScorecard {
    pub status: String,
    pub generated_at: String,
    pub readiness_pct: f64,
    pub production_ready: bool,
    pub checks: Vec<ProductionCheck>,
}

pub struct ProductionRuntimeSnapshot {
    pub postgres_required: bool,
    pub postgres_ok: bool,
    pub redis_required: bool,
    pub redis_ok: bool,
    pub tls_active: bool,
}

pub fn build_production_scorecard(snap: &ProductionRuntimeSnapshot) -> ProductionScorecard {
    let mut checks = vec![
        check(
            "api-key-or-oidc",
            "API authentication",
            auth_hardened(),
            if auth_hardened() { "info" } else { "warn" },
            "AETHER_API_KEY, OIDC, or mutation confirm",
        ),
        check(
            "opa",
            "OPA policy engine",
            crate::opa::configured(),
            "info",
            "AETHER_OPA_URL",
        ),
        check(
            "postgres",
            "PostgreSQL workload state",
            !snap.postgres_required || snap.postgres_ok,
            if snap.postgres_required && !snap.postgres_ok {
                "critical"
            } else {
                "info"
            },
            "AETHER_STATE_DATABASE_URL",
        ),
        check(
            "redis",
            "Redis OIDC sessions",
            !snap.redis_required || snap.redis_ok,
            if snap.redis_required && !snap.redis_ok {
                "critical"
            } else {
                "info"
            },
            "AETHER_REDIS_URL",
        ),
        check(
            "backup-remote",
            "Remote backup target",
            env_set("AETHER_BACKUP_REMOTE_URL"),
            "info",
            "AETHER_BACKUP_REMOTE_URL",
        ),
        check(
            "audit-webhook",
            "Audit webhook",
            env_set("AETHER_AUDIT_WEBHOOK_URL"),
            "info",
            "AETHER_AUDIT_WEBHOOK_URL",
        ),
        check(
            "tls",
            "TLS termination",
            snap.tls_active,
            "info",
            "AETHER_EXPOSE / ingress TLS",
        ),
    ];

    let passed = checks.iter().filter(|c| c.ok).count();
    let readiness_pct = (passed as f64 / checks.len() as f64) * 100.0;
    let production_ready = checks
        .iter()
        .all(|c| c.ok || c.severity != "critical");

    ProductionScorecard {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        readiness_pct,
        production_ready,
        checks,
    }
}

fn check(id: &str, label: &str, ok: bool, severity: &str, detail: &str) -> ProductionCheck {
    ProductionCheck {
        id: id.into(),
        label: label.into(),
        ok,
        severity: severity.into(),
        detail: detail.into(),
    }
}

fn auth_hardened() -> bool {
    let oidc = env_set("AETHER_OIDC_ISSUER")
        && env_set("AETHER_OIDC_CLIENT_ID")
        && env_set("AETHER_SESSION_SECRET");
    let api_key = env_set("AETHER_API_KEY");
    let mutation = std::env::var("AETHER_REQUIRE_MUTATION_CONFIRM")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    oidc || api_key || mutation
}

// ── Phase 126: Auth plane ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthPlaneReport {
    pub status: String,
    pub generated_at: String,
    pub api_key_configured: bool,
    pub oidc_configured: bool,
    pub saml_available: bool,
    pub mock_idp: bool,
    pub rbac_keys_path: String,
    pub mutation_confirm_required: bool,
}

pub fn build_auth_plane_report(saml_enabled: bool) -> AuthPlaneReport {
    AuthPlaneReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        api_key_configured: env_set("AETHER_API_KEY"),
        oidc_configured: env_set("AETHER_OIDC_ISSUER") && env_set("AETHER_OIDC_CLIENT_ID"),
        saml_available: saml_enabled,
        mock_idp: env_set("AETHER_MOCK_IDP"),
        rbac_keys_path: crate::resources::aether_path("rbac-keys.json").display().to_string(),
        mutation_confirm_required: std::env::var("AETHER_REQUIRE_MUTATION_CONFIRM")
            .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
            .unwrap_or(false),
    }
}

// ── Phase 127: OPA plane ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpaPlaneReport {
    pub status: String,
    pub generated_at: String,
    pub configured: bool,
    pub enforce_enabled: bool,
    pub package: Option<String>,
    pub enforced_paths: Vec<String>,
}

pub fn build_opa_plane_report() -> OpaPlaneReport {
    let cfg = crate::opa::OpaConfig::from_env();
    OpaPlaneReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        configured: crate::opa::configured(),
        enforce_enabled: crate::opa::enforce_enabled(),
        package: cfg.as_ref().map(|c| c.package.clone()),
        enforced_paths: vec![
            "POST /api/workloads".into(),
            "PUT /api/workloads/:name".into(),
            "POST /api/cluster/apply".into(),
        ],
    }
}

// ── Phase 128: HA plane ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HaPlaneReport {
    pub status: String,
    pub generated_at: String,
    pub workload_state_backend: String,
    pub postgres_required: bool,
    pub postgres_ok: bool,
    pub redis_required: bool,
    pub redis_ok: bool,
    pub ha_ready: bool,
}

pub fn build_ha_plane_report(snap: &ProductionRuntimeSnapshot) -> HaPlaneReport {
    let ha_ready = (!snap.postgres_required || snap.postgres_ok)
        && (!snap.redis_required || snap.redis_ok);
    HaPlaneReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        workload_state_backend: if snap.postgres_required {
            "postgresql".into()
        } else {
            "local-json".into()
        },
        postgres_required: snap.postgres_required,
        postgres_ok: snap.postgres_ok,
        redis_required: snap.redis_required,
        redis_ok: snap.redis_ok,
        ha_ready,
    }
}

// ── Phase 129: Durability plane ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DurabilityPlaneReport {
    pub status: String,
    pub generated_at: String,
    pub backup_remote_configured: bool,
    pub audit_webhook_configured: bool,
    pub state_file: String,
    pub backup_api: String,
}

pub fn build_durability_plane_report() -> DurabilityPlaneReport {
    DurabilityPlaneReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        backup_remote_configured: env_set("AETHER_BACKUP_REMOTE_URL"),
        audit_webhook_configured: env_set("AETHER_AUDIT_WEBHOOK_URL"),
        state_file: crate::resources::aether_path("state.json").display().to_string(),
        backup_api: "/api/backups".into(),
    }
}

// ── Phase 130: Hosted plane ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostedPlaneReport {
    pub status: String,
    pub generated_at: String,
    pub tenant_count: usize,
    pub stripe_configured: bool,
    pub metering_enabled: bool,
    pub endpoints: Vec<String>,
}

pub fn build_hosted_plane_report() -> HostedPlaneReport {
    let tenants = TenantStore::load();
    HostedPlaneReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        tenant_count: tenants.list().len(),
        stripe_configured: env_set("AETHER_STRIPE_SECRET_KEY"),
        metering_enabled: true,
        endpoints: vec![
            "/api/hosted/tenants".into(),
            "/api/hosted/billing/usage".into(),
            "/api/hosted/billing/metering".into(),
        ],
    }
}

// ── Phase 131: Edge fleet ───────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeFleetPlaneReport {
    pub status: String,
    pub generated_at: String,
    pub agent_count: u32,
    pub online_count: u32,
    pub total_queue_depth: u32,
}

pub fn build_edge_fleet_plane_report() -> EdgeFleetPlaneReport {
    let store = EdgeStore::load();
    let agents = store.list_agents();
    let online = agents.iter().filter(|a| a.online).count() as u32;
    let queue: u32 = agents.iter().map(|a| a.queue_depth as u32).sum();
    EdgeFleetPlaneReport {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        agent_count: agents.len() as u32,
        online_count: online,
        total_queue_depth: queue,
    }
}

// ── Phase 132: Post-deploy manifest ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDeployStep {
    pub id: String,
    pub script: String,
    pub endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostDeployManifest {
    pub status: String,
    pub generated_at: String,
    pub steps: Vec<PostDeployStep>,
}

pub fn build_post_deploy_manifest() -> PostDeployManifest {
    PostDeployManifest {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        steps: vec![
            PostDeployStep {
                id: "health".into(),
                script: "scripts/post-deploy-verify.sh".into(),
                endpoint: Some("/health".into()),
            },
            PostDeployStep {
                id: "remote-ux".into(),
                script: "scripts/remote-api-ux-verify.sh".into(),
                endpoint: Some("/api/server".into()),
            },
            PostDeployStep {
                id: "k8s-smoke".into(),
                script: "scripts/k8s-api-smoke.sh".into(),
                endpoint: Some("/api/cluster/workloads".into()),
            },
        ],
    }
}

// ── Phase 133: CI smoke manifest ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiSmokeJob {
    pub id: String,
    pub description: String,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiSmokeManifest {
    pub status: String,
    pub generated_at: String,
    pub jobs: Vec<CiSmokeJob>,
}

pub fn build_ci_smoke_manifest() -> CiSmokeManifest {
    CiSmokeManifest {
        status: "ship".into(),
        generated_at: crate::resources::now_rfc3339(),
        jobs: vec![
            CiSmokeJob {
                id: "labs-e2e".into(),
                description: "Validate + dry-run lab specs".into(),
                command: "scripts/labs-e2e.sh".into(),
            },
            CiSmokeJob {
                id: "dashboard-e2e".into(),
                description: "Playwright API/UI smoke".into(),
                command: "cd web/dashboard && npm run test:e2e".into(),
            },
            CiSmokeJob {
                id: "k8s-live".into(),
                description: "Kind live deploy E2E".into(),
                command: "scripts/kind-playwright-fixture.sh".into(),
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overview_has_ten_features() {
        assert_eq!(build_production_overview().feature_count, 10);
    }

    #[test]
    fn scorecard_computes_pct() {
        let snap = ProductionRuntimeSnapshot {
            postgres_required: false,
            postgres_ok: true,
            redis_required: false,
            redis_ok: true,
            tls_active: false,
        };
        let s = build_production_scorecard(&snap);
        assert!(s.readiness_pct > 0.0);
        assert_eq!(s.status, "ship");
    }
}
