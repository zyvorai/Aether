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

    if let Ok(cilium) = crate::kubecluster::cilium::cilium_status(None, None).await {
        if !cilium.crds.ciliumnetworkpolicies && !cilium.crds.ciliumclusterwidenetworkpolicies {
            items.push(PlatformRecommendation {
                id: "cilium-not-detected".into(),
                category: "kubernetes".into(),
                severity: "info".into(),
                title: "Cilium CNI not detected".into(),
                detail: "No Cilium CRDs found on the active cluster. Aether can deploy workloads with standard NetworkPolicy, but Cilium-specific policies require Cilium.".into(),
                action: "Install Cilium on your cluster or run cluster/install-cluster.sh from the customer bundle with Cilium bootstrap enabled. See docs/guides/kubernetes/CILIUM.md.".into(),
            });
        } else {
            let bootstrap_ok = cilium
                .managed_policies
                .iter()
                .any(|p| p.name == "allow-aether-egress" && p.exists)
                || cilium
                    .managed_policies
                    .iter()
                    .any(|p| p.name == "allow-aether-egress-strict" && p.exists);
            if !bootstrap_ok {
                items.push(PlatformRecommendation {
                    id: "cilium-bootstrap-missing".into(),
                    category: "kubernetes".into(),
                    severity: "warn".into(),
                    title: "Aether Cilium bootstrap policies missing".into(),
                    detail: "Cilium CRDs are present but expected Aether egress policies (allow-aether-egress or strict variants) were not found.".into(),
                    action: "Re-run cluster deploy with Cilium bootstrap, or apply deploy/k8s/bootstrap/ manifests. Set AETHER_CILIUM_EGRESS_STRICT=1 for strict mode.".into(),
                });
            }
        }

        if !cilium.metrics_server {
            items.push(PlatformRecommendation {
                id: "metrics-server-missing".into(),
                category: "kubernetes".into(),
                severity: "info".into(),
                title: "metrics-server not available".into(),
                detail: "kubectl top nodes failed — cluster CPU/memory metrics in the Clusters page require metrics-server.".into(),
                action: "Install metrics-server (set AETHER_INSTALL_METRICS_SERVER=1 on deploy or cluster/install-cluster-prereqs.sh).".into(),
            });
        }

        if cilium.connectivity_check == "failed" {
            items.push(PlatformRecommendation {
                id: "cilium-connectivity-failed".into(),
                category: "kubernetes".into(),
                severity: "warn".into(),
                title: "Cilium connectivity check failed".into(),
                detail: "The Cilium agent daemonset has no ready replicas or the post-deploy probe recorded failure.".into(),
                action: "Check kube-system/cilium daemonset, re-run deploy with Cilium bootstrap, or inspect ConfigMap aether-cilium-connectivity.".into(),
            });
        }
    }

    if std::env::var("AETHER_PROMETHEUS_URL").ok().filter(|s| !s.is_empty()).is_none() {
        items.push(PlatformRecommendation {
            id: "prometheus-not-linked".into(),
            category: "observability".into(),
            severity: "info".into(),
            title: "Prometheus not linked".into(),
            detail: "AETHER_PROMETHEUS_URL is unset — the dashboard cannot proxy whitelisted PromQL or show external scrape targets.".into(),
            action: "Set AETHER_PROMETHEUS_URL to your Prometheus server and import grafana/dashboard.json (see scripts/import-grafana-dashboard.sh).".into(),
        });
    }

    items
}
