//! Optional platform setup items surfaced on the dashboard Platform page (not inline banners).

use super::types::AppState;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PlatformRecommendation {
    pub id: String,
    pub category: String,
    /// `info`, `warn`, or `critical`
    pub severity: String,
    pub title: String,
    pub detail: String,
    pub action: String,
}

pub async fn collect(app_state: &AppState) -> Vec<PlatformRecommendation> {
    let mut items = Vec::new();

    if let Some(err) = crate::kubecluster::kubeconfig_inventory_fallback_error().await {
        items.push(PlatformRecommendation {
            id: "kubeconfig-inventory".into(),
            category: "kubernetes".into(),
            severity: "info".into(),
            title: "Multi-context kubeconfig inventory unavailable".into(),
            detail: err,
            action: "Ensure kubeconfig exists and is readable (check $KUBECONFIG and file permissions). \
                      Workload discovery still uses your active default cluster."
                .into(),
        });
    }

    let postgres_configured = app_state.workload_state_pg.is_some();
    let redis_configured = app_state.shared_cache.uses_redis();

    if !postgres_configured {
        items.push(PlatformRecommendation {
            id: "ha-state-database".into(),
            category: "high-availability".into(),
            severity: "info".into(),
            title: "Shared workload state for multiple API replicas".into(),
            detail: "Workload state is stored in a local JSON file on each API pod.".into(),
            action: "Set AETHER_STATE_DATABASE_URL (or enable Helm postgresql.enabled) before running multiple API replicas."
                .into(),
        });
    } else if !redis_configured {
        items.push(PlatformRecommendation {
            id: "ha-redis-sessions".into(),
            category: "high-availability".into(),
            severity: "info".into(),
            title: "Shared OIDC session cache".into(),
            detail: "PostgreSQL backs workload state; OIDC sessions are still per-pod without Redis.".into(),
            action: "Set AETHER_REDIS_URL for shared SSE/OIDC sessions across API replicas.".into(),
        });
    }

    let postgres_ok = if let Some(ref pg) = app_state.workload_state_pg {
        pg.ping_ok().await
    } else {
        true
    };
    let redis_ok = app_state.shared_cache.redis_ping_ok().await;

    if postgres_configured && !postgres_ok {
        items.push(PlatformRecommendation {
            id: "postgres-unreachable".into(),
            category: "high-availability".into(),
            severity: "critical".into(),
            title: "PostgreSQL workload state unreachable".into(),
            detail: "AETHER_STATE_DATABASE_URL is set but the database did not respond.".into(),
            action: "Verify Postgres is running, reachable from API pods, and credentials in AETHER_STATE_DATABASE_URL are correct."
                .into(),
        });
    }

    if redis_configured && !redis_ok {
        items.push(PlatformRecommendation {
            id: "redis-unreachable".into(),
            category: "high-availability".into(),
            severity: "critical".into(),
            title: "Redis session cache unreachable".into(),
            detail: "AETHER_REDIS_URL is set but Redis did not respond.".into(),
            action: "Start Redis and confirm AETHER_REDIS_URL from every API replica.".into(),
        });
    }

    let oidc_ready = std::env::var("AETHER_OIDC_ISSUER").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_OIDC_CLIENT_ID").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_OIDC_REDIRECT_URI").ok().filter(|s| !s.is_empty()).is_some()
        && std::env::var("AETHER_SESSION_SECRET").ok().filter(|s| !s.is_empty()).is_some();
    let mutation_confirm = std::env::var("AETHER_REQUIRE_MUTATION_CONFIRM")
        .map(|s| s == "1" || s.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let api_key_set = std::env::var("AETHER_API_KEY").ok().filter(|s| !s.is_empty()).is_some();

    if !oidc_ready && !mutation_confirm && !api_key_set {
        items.push(PlatformRecommendation {
            id: "auth-hardening".into(),
            category: "security".into(),
            severity: "warn".into(),
            title: "API authentication not hardened".into(),
            detail: "Neither OIDC, AETHER_API_KEY, nor mutation confirmation is enabled.".into(),
            action: "For production: set AETHER_API_KEY or configure OIDC (AETHER_OIDC_*), or enable AETHER_REQUIRE_MUTATION_CONFIRM."
                .into(),
        });
    }

    if std::env::var("AETHER_BACKUP_REMOTE_URL").ok().filter(|s| !s.is_empty()).is_none() {
        items.push(PlatformRecommendation {
            id: "backup-remote".into(),
            category: "durability".into(),
            severity: "info".into(),
            title: "Off-host backup target".into(),
            detail: "Backups are stored locally under ~/.aether unless a remote target is configured.".into(),
            action: "Set AETHER_BACKUP_REMOTE_URL and schedule POST /api/backups for disaster recovery.".into(),
        });
    }

    items
}
