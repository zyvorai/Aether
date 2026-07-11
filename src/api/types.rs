// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! API types and data structures

use crate::spec::Workload;
use crate::state::StateStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock};

/// API Server configuration
#[derive(Debug, Clone)]
pub struct ApiConfig {
    pub host: String,
    pub port: u16,
    /// Path to the workload `state.json` file (mirrored when `AETHER_STATE_DATABASE_URL` is set).
    pub state_path: PathBuf,
    /// TLS certificate file path (enables HTTPS when set with tls_key)
    pub tls_cert: Option<PathBuf>,
    /// TLS private key file path
    pub tls_key: Option<PathBuf>,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 5090,
            state_path: StateStore::default_path(),
            tls_cert: None,
            tls_key: None,
        }
    }
}

/// Server-sent event for real-time updates
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
#[allow(dead_code)]
pub enum ServerEvent {
    /// Workload state changed
    WorkloadChanged { name: String, action: String },
    /// New event emitted
    EventEmitted { message: String, severity: String },
    /// Health status update
    HealthUpdate { workload: String, status: String },
    /// Migration progress for live activity cards
    MigrationProgress {
        workload: String,
        phase: String,
        percent: u8,
        eta_secs: Option<u64>,
        message: String,
    },
}

/// Shared application state
#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) state: Arc<RwLock<StateStore>>,
    pub(crate) event_tx: tokio::sync::broadcast::Sender<String>,
    pub(crate) rbac: Arc<RwLock<crate::rbac::RbacStore>>,
    pub(crate) port_forwards: Arc<Mutex<HashMap<String, PortForwardSession>>>,
    pub(crate) shared_cache: crate::ha::SharedCache,
    pub(crate) oidc: Option<Arc<crate::oidc::OidcRuntime>>,
    pub(crate) saml: Option<Arc<crate::saml::SamlRuntime>>,
    pub(crate) ldap: Option<Arc<crate::ldap::LdapRuntime>>,
    /// True when the API is serving HTTPS (used for Secure session cookies).
    pub(crate) tls_active: bool,
    /// Path to the workload state JSON file (mirrored when Postgres is enabled).
    pub(crate) state_path: PathBuf,
    /// Optional Postgres pool for shared workload state across API replicas.
    pub(crate) workload_state_pg: Option<Arc<crate::state_postgres::WorkloadStatePool>>,
}

pub(crate) struct PortForwardSession {
    pub(crate) id: String,
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) pod: String,
    pub(crate) local_port: u16,
    pub(crate) remote_port: u16,
    pub(crate) child: Child,
}

/// API Response wrapper
#[derive(Debug, Serialize)]
pub(crate) struct ApiResponse<T> {
    pub(crate) success: bool,
    pub(crate) data: Option<T>,
    pub(crate) error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub(crate) fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub(crate) fn error(message: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(message),
        }
    }
}

/// Workload creation request
#[derive(Debug, Deserialize)]
pub(crate) struct CreateWorkloadRequest {
    pub(crate) spec: Workload,
    pub(crate) runtime: Option<String>,
}

/// Migrate workload request
#[derive(Debug, Deserialize)]
pub(crate) struct MigrateWorkloadRequest {
    /// Target runtime (podman, kubernetes, kubevirt, metal3)
    pub(crate) target_runtime: String,
    /// Migration strategy (immediate, blue-green, rolling)
    #[serde(default = "default_strategy")]
    pub(crate) strategy: String,
    /// When true, use MigrationAdvisor to pick strategy automatically.
    #[serde(default)]
    pub(crate) auto_strategy: bool,
}

pub(crate) fn default_strategy() -> String {
    "blue-green".to_string()
}

/// Validate workload request
#[derive(Debug, Deserialize)]
pub(crate) struct ValidateRequest {
    pub(crate) yaml: String,
}

/// Validation response
#[derive(Debug, Serialize)]
pub(crate) struct ValidateResponse {
    pub(crate) valid: bool,
    pub(crate) workload_name: Option<String>,
    pub(crate) errors: Vec<String>,
}

/// Build response
#[derive(Debug, Serialize)]
pub(crate) struct BuildResponse {
    pub(crate) image_name: String,
    pub(crate) image_tag: String,
    pub(crate) full_name: String,
    pub(crate) runtime: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthStatusResponse {
    pub(crate) authenticated: bool,
    pub(crate) username: String,
    pub(crate) role: String,
}

/// Query parameters for native Kubernetes workload logs.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterLogsQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
}

/// Query parameters for native Kubernetes workload detail inspection.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterResourceQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) api_version: Option<String>,
    pub(crate) plural: Option<String>,
    pub(crate) namespaced: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClusterHealthQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) api_version: Option<String>,
    pub(crate) plural: Option<String>,
    pub(crate) namespaced: Option<bool>,
}

/// Query parameters for native Kubernetes resource-related events.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterEventsQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
}

/// Query parameters for native Kubernetes exec sessions over WebSocket.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterExecQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) pod: String,
    pub(crate) command: Option<String>,
    pub(crate) container: Option<String>,
}

/// Query parameters for native Kubernetes watch sessions over WebSocket.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterWatchQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: Option<String>,
    pub(crate) kind: String,
    pub(crate) api_version: Option<String>,
    pub(crate) plural: Option<String>,
    pub(crate) namespaced: Option<bool>,
}

/// Request body for native Kubernetes actions.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterActionRequestBody {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) action: String,
    pub(crate) replicas: Option<i32>,
    pub(crate) api_version: Option<String>,
    pub(crate) plural: Option<String>,
    pub(crate) namespaced: Option<bool>,
}

/// Query parameters for native Kubernetes namespace listing.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterNamespacesQuery {
    pub(crate) cluster: String,
}

/// Query parameters for native Kubernetes resource browsing.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterBrowseQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: Option<String>,
    pub(crate) kind: String,
    pub(crate) api_version: Option<String>,
    pub(crate) plural: Option<String>,
    pub(crate) namespaced: Option<bool>,
}

/// Request body for applying an edited Kubernetes manifest.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterApplyRequestBody {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) manifest: serde_json::Value,
    pub(crate) api_version: Option<String>,
    pub(crate) plural: Option<String>,
    pub(crate) namespaced: Option<bool>,
}

/// Request body for starting a pod port-forward session.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterPortForwardRequestBody {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) target_kind: Option<String>,
    pub(crate) target_name: Option<String>,
    pub(crate) pod: Option<String>,
    pub(crate) remote_port: u16,
    pub(crate) local_port: Option<u16>,
}

/// Request body for stopping a port-forward session.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterPortForwardStopRequestBody {
    pub(crate) session_id: String,
}

/// API response for a running port-forward session.
#[derive(Debug, Serialize)]
pub(crate) struct ClusterPortForwardResponse {
    pub(crate) session_id: String,
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) target_kind: String,
    pub(crate) target_name: String,
    pub(crate) local_port: u16,
    pub(crate) remote_port: u16,
    pub(crate) local_url: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClusterTopQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClusterMetricsSummaryQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClusterCiliumStatusQuery {
    pub(crate) cluster: Option<String>,
    pub(crate) namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ObservabilitySummaryQuery {
    pub(crate) cluster: Option<String>,
    pub(crate) namespace: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct PrometheusQueryParams {
    pub(crate) query: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClusterDiffRequestBody {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) draft_manifest: serde_json::Value,
    pub(crate) api_version: Option<String>,
    pub(crate) plural: Option<String>,
    pub(crate) namespaced: Option<bool>,
}

/// Query parameters for rollout inspection.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterRolloutQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
}

/// Rollout action request body.
#[derive(Debug, Deserialize)]
pub(crate) struct ClusterRolloutActionRequestBody {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) action: String,
    pub(crate) revision: Option<String>,
}

/// Rollout status/history response.
#[derive(Debug, Serialize)]
pub(crate) struct ClusterRolloutResponse {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) kind: String,
    pub(crate) name: String,
    pub(crate) status: String,
    pub(crate) history: Vec<ClusterRolloutRevision>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ClusterRolloutRevision {
    pub(crate) revision: String,
    pub(crate) change_cause: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct ClusterMetricsSummaryResponse {
    pub(crate) scope: String,
    pub(crate) pod_count: usize,
    pub(crate) total_cpu_millicores: i64,
    pub(crate) total_memory_mib: i64,
    pub(crate) pods: Vec<crate::kubecluster::ClusterTopMetric>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ClusterDiffLine {
    pub(crate) kind: String,
    pub(crate) text: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClusterHelmHistoryQuery {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) release: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ClusterHelmActionRequestBody {
    pub(crate) cluster: String,
    pub(crate) namespace: String,
    pub(crate) release: String,
    pub(crate) action: String,
    pub(crate) chart: Option<String>,
    pub(crate) values_yaml: Option<String>,
    pub(crate) revision: Option<String>,
}

/// Secret metadata response (no raw values exposed)
#[derive(Debug, Serialize)]
pub(crate) struct SecretMetadataResponse {
    pub(crate) name: String,
    pub(crate) namespace: String,
    pub(crate) key_count: usize,
    pub(crate) keys: Vec<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    pub(crate) needs_rotation: bool,
    pub(crate) rotation_policy: Option<SecretRotationInfo>,
}

/// Rotation info for API response
#[derive(Debug, Serialize)]
pub(crate) struct SecretRotationInfo {
    pub(crate) interval_days: u32,
    pub(crate) max_age_days: u32,
    pub(crate) notify_before_days: u32,
    /// Whether Aether owns this value and may auto-generate a fresh credential
    /// on rotation (vs. an externally-managed secret that is only alerted).
    pub(crate) generate: bool,
}

/// Workload response
#[derive(Debug, Serialize)]
pub(crate) struct WorkloadResponse {
    pub(crate) name: String,
    pub(crate) runtime: String,
    pub(crate) image: String,
    pub(crate) status: String,
    pub(crate) created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cluster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) namespace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
}

/// Health check response
#[derive(Debug, Serialize)]
pub(crate) struct HealthResponse {
    pub(crate) status: String,
    pub(crate) version: String,
}

/// Migration advice response
#[derive(Debug, Serialize)]
pub(crate) struct MigrationAdviceResponse {
    pub(crate) workload_name: String,
    pub(crate) source_runtime: String,
    pub(crate) target_runtime: String,
    pub(crate) recommended_strategy: String,
    pub(crate) estimated_downtime_secs: u64,
    pub(crate) risk_level: String,
    pub(crate) reasons: Vec<String>,
    pub(crate) warnings: Vec<String>,
    pub(crate) timing: TimingAdviceResponse,
    pub(crate) canary_config: CanaryConfigResponse,
}

/// Timing advice in migration response
#[derive(Debug, Serialize)]
pub(crate) struct TimingAdviceResponse {
    pub(crate) recommendation: String,
    pub(crate) preferred_window: String,
    pub(crate) avoid_times: Vec<String>,
}

/// Canary config in migration response
#[derive(Debug, Serialize)]
pub(crate) struct CanaryConfigResponse {
    pub(crate) steps: Vec<u32>,
    pub(crate) step_interval_secs: u64,
    pub(crate) error_threshold: f64,
    pub(crate) latency_threshold_pct: f64,
    pub(crate) min_observation_secs: u64,
}

/// Scaling advice response
#[derive(Debug, Serialize)]
pub(crate) struct ScalingAdviceResponse {
    pub(crate) action: String,
    pub(crate) current_replicas: u32,
    pub(crate) recommended_replicas: u32,
    pub(crate) reason: String,
    pub(crate) confidence: f64,
    pub(crate) forecast: ForecastResponse,
    pub(crate) cost_impact: CostImpactResponse,
}

/// Forecast in scaling response
#[derive(Debug, Serialize)]
pub(crate) struct ForecastResponse {
    pub(crate) trend: String,
    pub(crate) predicted_value: f64,
    pub(crate) lower_bound: f64,
    pub(crate) upper_bound: f64,
    pub(crate) horizon_minutes: u64,
}

/// Cost impact in scaling response
#[derive(Debug, Serialize)]
pub(crate) struct CostImpactResponse {
    pub(crate) current_hourly: f64,
    pub(crate) projected_hourly: f64,
    pub(crate) delta_hourly: f64,
    pub(crate) delta_monthly: f64,
}

/// Backup creation request
#[derive(Debug, Deserialize)]
pub(crate) struct CreateBackupRequest {
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
}

/// Policy check request (used by policy API tests; wire to handler when exposing POST /policy/check).
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub(crate) struct PolicyCheckRequest {
    pub(crate) spec: Workload,
    pub(crate) policy_set: Option<String>,
}

/// Add dependency request (also used for DELETE body)
#[derive(Debug, Deserialize)]
pub(crate) struct AddDependencyRequest {
    pub(crate) workload: String,
    pub(crate) dependency: String,
}

/// Create secret via API (metadata + plaintext keys; values encrypted at rest).
#[derive(Debug, Deserialize)]
pub(crate) struct CreateSecretRequest {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) namespace: Option<String>,
    pub(crate) keys: std::collections::HashMap<String, String>,
}

/// Restore state from a backup file in the default backup directory.
#[derive(Debug, Deserialize)]
pub(crate) struct RestoreBackupRequest {
    /// Backup file stem (`foo`) or filename (`foo.json`).
    pub(crate) name: String,
    /// When true, merge workloads that are not already present (default: false).
    #[serde(default)]
    pub(crate) merge: Option<bool>,
}

/// Append an audit event (admin only).
#[derive(Debug, Deserialize)]
pub(crate) struct AuditAppendRequest {
    pub(crate) action: String,
    pub(crate) workload: String,
    pub(crate) message: String,
    pub(crate) result: String,
    #[serde(default)]
    pub(crate) details: Option<String>,
    #[serde(default)]
    pub(crate) runtime: Option<String>,
}

/// Optional pagination for list endpoints (`limit` default varies by handler).
#[derive(Debug, Default, Deserialize)]
pub(crate) struct PaginationQuery {
    pub(crate) limit: Option<usize>,
    pub(crate) offset: Option<usize>,
}

/// GET /api/cost/chargeback query parameters.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct CostChargebackQuery {
    pub(crate) provider: Option<String>,
}

/// GET /api/audit query parameters.
#[derive(Debug, Deserialize)]
pub(crate) struct AuditListQuery {
    pub(crate) limit: Option<usize>,
    pub(crate) offset: Option<usize>,
    /// Substring match on audit `workload` field (e.g. pod or cluster ref).
    pub(crate) workload: Option<String>,
}

/// Trigger a webhook test by channel name (same as CLI `aether webhook test`).
#[derive(Debug, Deserialize)]
pub(crate) struct WebhookTestRequest {
    pub(crate) channel: String,
}

/// Create a webhook notification channel.
#[derive(Debug, Deserialize)]
pub(crate) struct WebhookChannelCreateRequest {
    pub(crate) name: String,
    pub(crate) url: String,
    #[serde(default = "default_webhook_method")]
    pub(crate) method: String,
    #[serde(default = "default_webhook_severity")]
    pub(crate) severity: String,
}

fn default_webhook_method() -> String {
    "POST".to_string()
}

fn default_webhook_severity() -> String {
    "warning".to_string()
}

/// GET /api/events query parameters.
#[derive(Debug, Default, Deserialize)]
pub(crate) struct EventsListQuery {
    pub(crate) limit: Option<usize>,
    pub(crate) offset: Option<usize>,
    /// Event category filter: drift, intent-violation, policy, etc.
    pub(crate) category: Option<String>,
    pub(crate) workload: Option<String>,
}

/// Template generation request
#[derive(Debug, Deserialize)]
pub(crate) struct TemplateRequest {
    pub(crate) workload_name: Option<String>,
    pub(crate) owner: Option<String>,
    pub(crate) project: Option<String>,
    pub(crate) registry: Option<String>,
    pub(crate) cpu: Option<String>,
    pub(crate) memory: Option<String>,
    pub(crate) port: Option<u16>,
    pub(crate) replicas: Option<u32>,
}

/// Request to create a new RBAC API key
#[derive(Debug, Deserialize)]
pub(crate) struct CreateApiKeyRequest {
    /// Human-readable name for the key
    pub(crate) name: String,
    /// Role: "admin", "operator", or "viewer"
    pub(crate) role: String,
}

/// Response after creating a new RBAC API key
#[derive(Debug, Serialize)]
pub(crate) struct CreateApiKeyResponse {
    pub(crate) name: String,
    pub(crate) role: String,
    /// Plaintext API key (shown only once)
    pub(crate) key: String,
}

/// Request to revoke an RBAC API key
#[derive(Debug, Deserialize)]
pub(crate) struct RevokeApiKeyRequest {
    /// Name of the key to revoke
    pub(crate) name: String,
}

/// Summary of an API key (without the plaintext key)
#[derive(Debug, Serialize)]
pub(crate) struct ApiKeySummary {
    pub(crate) name: String,
    pub(crate) role: String,
    pub(crate) created_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    // ---------------------------------------------------------------
    // ApiConfig defaults
    // ---------------------------------------------------------------

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 5090);
    }

    #[test]
    fn test_api_config_default_state_path() {
        let config = ApiConfig::default();
        assert_eq!(config.state_path, StateStore::default_path());
    }

    #[test]
    fn test_api_config_custom_values() {
        let config = ApiConfig {
            host: "0.0.0.0".to_string(),
            port: 3000,
            state_path: PathBuf::from("/tmp/test-state.json"),
            tls_cert: None,
            tls_key: None,
        };
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 3000);
        assert_eq!(config.state_path, PathBuf::from("/tmp/test-state.json"));
    }

    // ---------------------------------------------------------------
    // ApiResponse construction
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_success() {
        let response = ApiResponse::success("hello".to_string());
        assert!(response.success);
        assert_eq!(response.data, Some("hello".to_string()));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error() {
        let response = ApiResponse::<String>::error("something went wrong".to_string());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some("something went wrong".to_string()));
    }

    #[test]
    fn test_api_response_success_with_integer() {
        let response = ApiResponse::success(42u64);
        assert!(response.success);
        assert_eq!(response.data, Some(42u64));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_success_with_vec() {
        let data = vec!["a".to_string(), "b".to_string()];
        let response = ApiResponse::success(data.clone());
        assert!(response.success);
        assert_eq!(response.data, Some(data));
        assert!(response.error.is_none());
    }

    #[test]
    fn test_api_response_error_with_empty_message() {
        let response = ApiResponse::<String>::error(String::new());
        assert!(!response.success);
        assert!(response.data.is_none());
        assert_eq!(response.error, Some(String::new()));
    }

    // ---------------------------------------------------------------
    // ApiResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_success_serialization() {
        let response = ApiResponse::success("data here".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"], "data here");
        assert!(json["error"].is_null());
    }

    #[test]
    fn test_api_response_error_serialization() {
        let response = ApiResponse::<String>::error("bad request".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "bad request");
    }

    #[test]
    fn test_api_response_success_serialization_has_all_fields() {
        let response = ApiResponse::success("ok".to_string());
        let json_str = serde_json::to_string(&response).unwrap();

        assert!(json_str.contains("\"success\""));
        assert!(json_str.contains("\"data\""));
        assert!(json_str.contains("\"error\""));
    }

    #[test]
    fn test_api_response_nested_struct_serialization() {
        let health = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
        };
        let response = ApiResponse::success(health);
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["status"], "ok");
        assert_eq!(json["data"]["version"], "1.0.0");
        assert!(json["error"].is_null());
    }

    // ---------------------------------------------------------------
    // HealthResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "0.2.0".to_string(),
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"status\":\"ok\""));
        assert!(json.contains("\"version\":\"0.2.0\""));
    }

    #[test]
    fn test_health_response_serialization_roundtrip_via_value() {
        let response = HealthResponse {
            status: "healthy".to_string(),
            version: "3.1.4".to_string(),
        };
        let value = serde_json::to_value(&response).unwrap();

        assert_eq!(value["status"], "healthy");
        assert_eq!(value["version"], "3.1.4");
    }

    #[test]
    fn test_health_response_contains_exactly_two_fields() {
        let response = HealthResponse {
            status: "ok".to_string(),
            version: "1.0.0".to_string(),
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("status"));
        assert!(obj.contains_key("version"));
    }

    // ---------------------------------------------------------------
    // WorkloadResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_workload_response_serialization() {
        let response = WorkloadResponse {
            name: "my-app".to_string(),
            runtime: "Podman".to_string(),
            image: "my-app:latest".to_string(),
            status: "running".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            source: None,
            cluster: None,
            namespace: None,
            kind: None,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("my-app"));
        assert!(json.contains("Podman"));
    }

    #[test]
    fn test_workload_response_all_fields_present() {
        let response = WorkloadResponse {
            name: "web-server".to_string(),
            runtime: "Kubernetes".to_string(),
            image: "registry.io/web:v2".to_string(),
            status: "deployed (kubernetes)".to_string(),
            created_at: "2026-06-15T12:30:00Z".to_string(),
            source: None,
            cluster: None,
            namespace: None,
            kind: None,
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();

        assert_eq!(obj.len(), 5);
        assert_eq!(value["name"], "web-server");
        assert_eq!(value["runtime"], "Kubernetes");
        assert_eq!(value["image"], "registry.io/web:v2");
        assert_eq!(value["status"], "deployed (kubernetes)");
        assert_eq!(value["created_at"], "2026-06-15T12:30:00Z");
    }

    #[test]
    fn test_workload_response_serialization_produces_valid_json() {
        let response = WorkloadResponse {
            name: "test".to_string(),
            runtime: "Metal3".to_string(),
            image: "img:latest".to_string(),
            status: "deployed".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            source: None,
            cluster: None,
            namespace: None,
            kind: None,
        };
        let json_str = serde_json::to_string(&response).unwrap();
        let reparsed: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        assert_eq!(reparsed["name"], "test");
    }

    // ---------------------------------------------------------------
    // BuildResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_build_response_serialization() {
        let response = BuildResponse {
            image_name: "my-app".to_string(),
            image_tag: "v1.0".to_string(),
            full_name: "my-app:v1.0".to_string(),
            runtime: "podman".to_string(),
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["image_name"], "my-app");
        assert_eq!(value["image_tag"], "v1.0");
        assert_eq!(value["full_name"], "my-app:v1.0");
        assert_eq!(value["runtime"], "podman");
    }

    #[test]
    fn test_build_response_has_four_fields() {
        let response = BuildResponse {
            image_name: "svc".to_string(),
            image_tag: "latest".to_string(),
            full_name: "svc:latest".to_string(),
            runtime: "kubernetes".to_string(),
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 4);
    }

    // ---------------------------------------------------------------
    // ValidateRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_request_deserialize() {
        let json = serde_json::json!({
            "yaml": "apiVersion: aether/v1\nkind: Workload\n"
        });
        let request: ValidateRequest = serde_json::from_value(json).unwrap();
        assert!(request.yaml.contains("aether/v1"));
    }

    #[test]
    fn test_validate_request_missing_yaml_fails() {
        let json = serde_json::json!({});
        let result = serde_json::from_value::<ValidateRequest>(json);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // ValidateResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_validate_response_valid_serialization() {
        let response = ValidateResponse {
            valid: true,
            workload_name: Some("my-app".to_string()),
            errors: vec![],
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["valid"], true);
        assert_eq!(value["workload_name"], "my-app");
        assert_eq!(value["errors"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_validate_response_invalid_serialization() {
        let response = ValidateResponse {
            valid: false,
            workload_name: None,
            errors: vec!["YAML parse error: unexpected token".to_string()],
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["valid"], false);
        assert!(value["workload_name"].is_null());
        let errors = value["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 1);
        assert!(errors[0].as_str().unwrap().contains("YAML parse error"));
    }

    #[test]
    fn test_validate_response_multiple_errors() {
        let response = ValidateResponse {
            valid: false,
            workload_name: Some("bad-spec".to_string()),
            errors: vec![
                "Invalid apiVersion".to_string(),
                "Missing metadata.name".to_string(),
            ],
        };
        let value = serde_json::to_value(&response).unwrap();
        let errors = value["errors"].as_array().unwrap();
        assert_eq!(errors.len(), 2);
    }

    // ---------------------------------------------------------------
    // SecretMetadataResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_secret_metadata_response_serialization() {
        let response = SecretMetadataResponse {
            name: "db-credentials".to_string(),
            namespace: "production".to_string(),
            key_count: 2,
            keys: vec!["username".to_string(), "password".to_string()],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-15T00:00:00Z".to_string(),
            needs_rotation: false,
            rotation_policy: None,
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["name"], "db-credentials");
        assert_eq!(value["namespace"], "production");
        assert_eq!(value["key_count"], 2);
        assert_eq!(value["keys"].as_array().unwrap().len(), 2);
        assert_eq!(value["needs_rotation"], false);
        assert!(value["rotation_policy"].is_null());
    }

    #[test]
    fn test_secret_metadata_response_with_rotation_policy() {
        let response = SecretMetadataResponse {
            name: "api-key".to_string(),
            namespace: "default".to_string(),
            key_count: 1,
            keys: vec!["token".to_string()],
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            needs_rotation: true,
            rotation_policy: Some(SecretRotationInfo {
                interval_days: 30,
                max_age_days: 90,
                notify_before_days: 7,
                generate: false,
            }),
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["needs_rotation"], true);
        assert_eq!(value["rotation_policy"]["interval_days"], 30);
        assert_eq!(value["rotation_policy"]["max_age_days"], 90);
        assert_eq!(value["rotation_policy"]["notify_before_days"], 7);
        assert_eq!(value["rotation_policy"]["generate"], false);
    }

    // ---------------------------------------------------------------
    // SecretRotationInfo serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_secret_rotation_info_serialization() {
        let info = SecretRotationInfo {
            interval_days: 60,
            max_age_days: 180,
            notify_before_days: 14,
            generate: true,
        };
        let value = serde_json::to_value(&info).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 4);
        assert_eq!(value["interval_days"], 60);
        assert_eq!(value["max_age_days"], 180);
        assert_eq!(value["notify_before_days"], 14);
        assert_eq!(value["generate"], true);
    }

    // ---------------------------------------------------------------
    // CreateWorkloadRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_create_workload_request_deserialize_with_runtime() {
        let json = serde_json::json!({
            "spec": {
                "apiVersion": "aether/v1",
                "kind": "Workload",
                "metadata": {
                    "name": "test-app",
                    "owner": "dev",
                    "project": "demo"
                },
                "build": {
                    "context": ".",
                    "dockerfile": "Dockerfile",
                    "registry": "ghcr.io/test"
                },
                "requirements": {
                    "cpu": "1",
                    "memory": "512Mi",
                    "storage": "1Gi"
                },
                "runtime": {
                    "preferred": "auto",
                    "allow": ["container"]
                }
            },
            "runtime": "podman"
        });

        let request: CreateWorkloadRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.runtime, Some("podman".to_string()));
        assert_eq!(request.spec.metadata.name, "test-app");
    }

    #[test]
    fn test_create_workload_request_deserialize_without_runtime() {
        let json = serde_json::json!({
            "spec": {
                "apiVersion": "aether/v1",
                "kind": "Workload",
                "metadata": {
                    "name": "auto-app",
                    "owner": "dev",
                    "project": "demo"
                },
                "build": {
                    "context": ".",
                    "dockerfile": "Dockerfile",
                    "registry": "ghcr.io/test"
                },
                "requirements": {
                    "cpu": "2",
                    "memory": "4Gi",
                    "storage": "10Gi"
                },
                "runtime": {
                    "preferred": "auto",
                    "allow": ["container", "kube"]
                }
            }
        });

        let request: CreateWorkloadRequest = serde_json::from_value(json).unwrap();
        assert!(request.runtime.is_none());
        assert_eq!(request.spec.metadata.name, "auto-app");
    }

    // ---------------------------------------------------------------
    // MigrateWorkloadRequest deserialization and default_strategy
    // ---------------------------------------------------------------

    #[test]
    fn test_default_strategy_value() {
        assert_eq!(default_strategy(), "blue-green");
    }

    #[test]
    fn test_migrate_workload_request_deserialize_with_strategy() {
        let json = serde_json::json!({
            "target_runtime": "kubernetes",
            "strategy": "rolling"
        });

        let request: MigrateWorkloadRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.target_runtime, "kubernetes");
        assert_eq!(request.strategy, "rolling");
    }

    #[test]
    fn test_migrate_workload_request_deserialize_default_strategy() {
        let json = serde_json::json!({
            "target_runtime": "podman"
        });

        let request: MigrateWorkloadRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.target_runtime, "podman");
        assert_eq!(request.strategy, "blue-green");
    }

    #[test]
    fn test_migrate_workload_request_all_strategy_values() {
        for strategy in &["immediate", "blue-green", "rolling"] {
            let json = serde_json::json!({
                "target_runtime": "kubernetes",
                "strategy": strategy
            });
            let request: MigrateWorkloadRequest = serde_json::from_value(json).unwrap();
            assert_eq!(request.strategy, *strategy);
        }
    }

    #[test]
    fn test_migrate_workload_request_missing_target_runtime_fails() {
        let json = serde_json::json!({
            "strategy": "rolling"
        });

        let result = serde_json::from_value::<MigrateWorkloadRequest>(json);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // CreateBackupRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_create_backup_request_all_fields() {
        let json = serde_json::json!({
            "name": "my-backup",
            "description": "Pre-migration backup"
        });

        let request: CreateBackupRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.name, Some("my-backup".to_string()));
        assert_eq!(
            request.description,
            Some("Pre-migration backup".to_string())
        );
    }

    #[test]
    fn test_create_backup_request_empty_object() {
        let json = serde_json::json!({});

        let request: CreateBackupRequest = serde_json::from_value(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.description.is_none());
    }

    #[test]
    fn test_create_backup_request_name_only() {
        let json = serde_json::json!({
            "name": "quick-backup"
        });

        let request: CreateBackupRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.name, Some("quick-backup".to_string()));
        assert!(request.description.is_none());
    }

    // ---------------------------------------------------------------
    // PolicyCheckRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_policy_check_request_with_policy_set() {
        let json = serde_json::json!({
            "spec": {
                "apiVersion": "aether/v1",
                "kind": "Workload",
                "metadata": {
                    "name": "policy-test",
                    "owner": "dev",
                    "project": "demo"
                },
                "build": {
                    "context": ".",
                    "dockerfile": "Dockerfile",
                    "registry": "ghcr.io/test"
                },
                "requirements": {
                    "cpu": "1",
                    "memory": "1Gi",
                    "storage": "5Gi"
                },
                "runtime": {
                    "preferred": "auto",
                    "allow": ["container"]
                }
            },
            "policy_set": "development"
        });

        let request: PolicyCheckRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.policy_set, Some("development".to_string()));
        assert_eq!(request.spec.metadata.name, "policy-test");
    }

    #[test]
    fn test_policy_check_request_without_policy_set() {
        let json = serde_json::json!({
            "spec": {
                "apiVersion": "aether/v1",
                "kind": "Workload",
                "metadata": {
                    "name": "policy-test",
                    "owner": "dev",
                    "project": "demo"
                },
                "build": {
                    "context": ".",
                    "dockerfile": "Dockerfile",
                    "registry": "ghcr.io/test"
                },
                "requirements": {
                    "cpu": "1",
                    "memory": "1Gi",
                    "storage": "5Gi"
                },
                "runtime": {
                    "preferred": "auto",
                    "allow": ["container"]
                }
            }
        });

        let request: PolicyCheckRequest = serde_json::from_value(json).unwrap();
        assert!(request.policy_set.is_none());
    }

    // ---------------------------------------------------------------
    // AddDependencyRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_add_dependency_request_deserialize() {
        let json = serde_json::json!({
            "workload": "frontend",
            "dependency": "backend-api"
        });

        let request: AddDependencyRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.workload, "frontend");
        assert_eq!(request.dependency, "backend-api");
    }

    #[test]
    fn test_add_dependency_request_missing_workload_fails() {
        let json = serde_json::json!({
            "dependency": "backend"
        });
        assert!(serde_json::from_value::<AddDependencyRequest>(json).is_err());
    }

    #[test]
    fn test_add_dependency_request_missing_dependency_fails() {
        let json = serde_json::json!({
            "workload": "frontend"
        });
        assert!(serde_json::from_value::<AddDependencyRequest>(json).is_err());
    }

    // ---------------------------------------------------------------
    // TemplateRequest deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_template_request_all_fields() {
        let json = serde_json::json!({
            "workload_name": "my-web-app",
            "owner": "platform-team",
            "project": "main-site",
            "registry": "docker.io/myorg",
            "cpu": "4",
            "memory": "8Gi",
            "port": 8080,
            "replicas": 3
        });

        let request: TemplateRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.workload_name, Some("my-web-app".to_string()));
        assert_eq!(request.owner, Some("platform-team".to_string()));
        assert_eq!(request.project, Some("main-site".to_string()));
        assert_eq!(request.registry, Some("docker.io/myorg".to_string()));
        assert_eq!(request.cpu, Some("4".to_string()));
        assert_eq!(request.memory, Some("8Gi".to_string()));
        assert_eq!(request.port, Some(8080));
        assert_eq!(request.replicas, Some(3));
    }

    #[test]
    fn test_template_request_empty_object() {
        let json = serde_json::json!({});

        let request: TemplateRequest = serde_json::from_value(json).unwrap();
        assert!(request.workload_name.is_none());
        assert!(request.owner.is_none());
        assert!(request.project.is_none());
        assert!(request.registry.is_none());
        assert!(request.cpu.is_none());
        assert!(request.memory.is_none());
        assert!(request.port.is_none());
        assert!(request.replicas.is_none());
    }

    #[test]
    fn test_template_request_partial_fields() {
        let json = serde_json::json!({
            "workload_name": "svc",
            "port": 3000
        });

        let request: TemplateRequest = serde_json::from_value(json).unwrap();
        assert_eq!(request.workload_name, Some("svc".to_string()));
        assert_eq!(request.port, Some(3000));
        assert!(request.owner.is_none());
        assert!(request.replicas.is_none());
    }

    #[test]
    fn test_template_request_invalid_port_type_fails() {
        let json = serde_json::json!({
            "port": "not-a-number"
        });
        let result = serde_json::from_value::<TemplateRequest>(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_template_request_invalid_replicas_type_fails() {
        let json = serde_json::json!({
            "replicas": -1
        });
        let result = serde_json::from_value::<TemplateRequest>(json);
        assert!(result.is_err());
    }

    // ---------------------------------------------------------------
    // Full round-trip: ApiResponse wrapping HealthResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_health_response_json() {
        let health = HealthResponse {
            status: "ok".to_string(),
            version: "0.3.0".to_string(),
        };
        let api_resp = ApiResponse::success(health);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert!(json["error"].is_null());
        assert_eq!(json["data"]["status"], "ok");
        assert_eq!(json["data"]["version"], "0.3.0");
    }

    // ---------------------------------------------------------------
    // Full round-trip: ApiResponse wrapping WorkloadResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_workload_response_json() {
        let workload = WorkloadResponse {
            name: "api-svc".to_string(),
            runtime: "KubeVirt".to_string(),
            image: "registry.io/api-svc:v1".to_string(),
            status: "deployed (kubevirt)".to_string(),
            created_at: "2026-03-10T08:00:00Z".to_string(),
            source: None,
            cluster: None,
            namespace: None,
            kind: None,
        };
        let api_resp = ApiResponse::success(workload);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["name"], "api-svc");
        assert_eq!(json["data"]["runtime"], "KubeVirt");
        assert_eq!(json["data"]["image"], "registry.io/api-svc:v1");
        assert_eq!(json["data"]["status"], "deployed (kubevirt)");
        assert_eq!(json["data"]["created_at"], "2026-03-10T08:00:00Z");
    }

    // ---------------------------------------------------------------
    // Full round-trip: ApiResponse wrapping Vec<WorkloadResponse>
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_workload_list_json() {
        let workloads = vec![
            WorkloadResponse {
                name: "app-a".to_string(),
                runtime: "Podman".to_string(),
                image: "app-a:latest".to_string(),
                status: "running".to_string(),
                created_at: "2026-01-01T00:00:00Z".to_string(),
                source: None,
                cluster: None,
                namespace: None,
                kind: None,
            },
            WorkloadResponse {
                name: "app-b".to_string(),
                runtime: "Kubernetes".to_string(),
                image: "app-b:v2".to_string(),
                status: "deployed".to_string(),
                created_at: "2026-02-01T00:00:00Z".to_string(),
                source: None,
                cluster: None,
                namespace: None,
                kind: None,
            },
        ];
        let api_resp = ApiResponse::success(workloads);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        let data = json["data"].as_array().unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0]["name"], "app-a");
        assert_eq!(data[1]["name"], "app-b");
    }

    // ---------------------------------------------------------------
    // ApiResponse error wrapping for various types
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_error_typed_workload_response() {
        let response = ApiResponse::<WorkloadResponse>::error("not found".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "not found");
    }

    #[test]
    fn test_api_response_error_typed_vec() {
        let response = ApiResponse::<Vec<String>>::error("internal error".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "internal error");
    }

    #[test]
    fn test_api_response_error_typed_build_response() {
        let response = ApiResponse::<BuildResponse>::error("build failed".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "build failed");
    }

    // ---------------------------------------------------------------
    // Edge cases for deserialization
    // ---------------------------------------------------------------

    #[test]
    fn test_migrate_workload_request_extra_fields_ignored() {
        let json = serde_json::json!({
            "target_runtime": "metal3",
            "strategy": "immediate",
            "unknown_field": "should be ignored"
        });
        let result = serde_json::from_value::<MigrateWorkloadRequest>(json);
        assert!(result.is_ok());
        let request = result.unwrap();
        assert_eq!(request.target_runtime, "metal3");
        assert_eq!(request.strategy, "immediate");
    }

    #[test]
    fn test_create_backup_request_null_fields() {
        let json = serde_json::json!({
            "name": null,
            "description": null
        });
        let request: CreateBackupRequest = serde_json::from_value(json).unwrap();
        assert!(request.name.is_none());
        assert!(request.description.is_none());
    }

    // ---------------------------------------------------------------
    // Full ApiResponse wrapping BuildResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_build_response_json() {
        let build = BuildResponse {
            image_name: "my-svc".to_string(),
            image_tag: "abc123".to_string(),
            full_name: "my-svc:abc123".to_string(),
            runtime: "podman".to_string(),
        };
        let api_resp = ApiResponse::success(build);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["image_name"], "my-svc");
        assert_eq!(json["data"]["image_tag"], "abc123");
        assert_eq!(json["data"]["full_name"], "my-svc:abc123");
        assert_eq!(json["data"]["runtime"], "podman");
    }

    // ---------------------------------------------------------------
    // Full ApiResponse wrapping ValidateResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_validate_response_json() {
        let validate = ValidateResponse {
            valid: true,
            workload_name: Some("good-app".to_string()),
            errors: vec![],
        };
        let api_resp = ApiResponse::success(validate);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["valid"], true);
        assert_eq!(json["data"]["workload_name"], "good-app");
        assert_eq!(json["data"]["errors"].as_array().unwrap().len(), 0);
    }

    // ---------------------------------------------------------------
    // MigrationAdviceResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_migration_advice_response_serialization() {
        let response = MigrationAdviceResponse {
            workload_name: "my-app".to_string(),
            source_runtime: "Podman".to_string(),
            target_runtime: "Kubernetes".to_string(),
            recommended_strategy: "BlueGreen".to_string(),
            estimated_downtime_secs: 30,
            risk_level: "Medium".to_string(),
            reasons: vec!["Better scaling".to_string()],
            warnings: vec!["Requires PVC migration".to_string()],
            timing: TimingAdviceResponse {
                recommendation: "Off-peak hours".to_string(),
                preferred_window: "02:00-06:00 UTC".to_string(),
                avoid_times: vec!["Peak hours".to_string()],
            },
            canary_config: CanaryConfigResponse {
                steps: vec![10, 25, 50, 100],
                step_interval_secs: 300,
                error_threshold: 0.05,
                latency_threshold_pct: 10.0,
                min_observation_secs: 120,
            },
        };
        let value = serde_json::to_value(&response).unwrap();

        assert_eq!(value["workload_name"], "my-app");
        assert_eq!(value["source_runtime"], "Podman");
        assert_eq!(value["target_runtime"], "Kubernetes");
        assert_eq!(value["recommended_strategy"], "BlueGreen");
        assert_eq!(value["estimated_downtime_secs"], 30);
        assert_eq!(value["risk_level"], "Medium");
        assert_eq!(value["reasons"].as_array().unwrap().len(), 1);
        assert_eq!(value["warnings"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_migration_advice_response_has_all_fields() {
        let response = MigrationAdviceResponse {
            workload_name: "svc".to_string(),
            source_runtime: "Podman".to_string(),
            target_runtime: "Kubernetes".to_string(),
            recommended_strategy: "Immediate".to_string(),
            estimated_downtime_secs: 0,
            risk_level: "Low".to_string(),
            reasons: vec![],
            warnings: vec![],
            timing: TimingAdviceResponse {
                recommendation: "Anytime".to_string(),
                preferred_window: "any".to_string(),
                avoid_times: vec![],
            },
            canary_config: CanaryConfigResponse {
                steps: vec![100],
                step_interval_secs: 60,
                error_threshold: 0.01,
                latency_threshold_pct: 5.0,
                min_observation_secs: 60,
            },
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();

        assert!(obj.contains_key("workload_name"));
        assert!(obj.contains_key("source_runtime"));
        assert!(obj.contains_key("target_runtime"));
        assert!(obj.contains_key("recommended_strategy"));
        assert!(obj.contains_key("estimated_downtime_secs"));
        assert!(obj.contains_key("risk_level"));
        assert!(obj.contains_key("reasons"));
        assert!(obj.contains_key("warnings"));
        assert!(obj.contains_key("timing"));
        assert!(obj.contains_key("canary_config"));
        assert_eq!(obj.len(), 10);
    }

    #[test]
    fn test_migration_advice_response_empty_reasons_and_warnings() {
        let response = MigrationAdviceResponse {
            workload_name: "test".to_string(),
            source_runtime: "Kubernetes".to_string(),
            target_runtime: "KubeVirt".to_string(),
            recommended_strategy: "Rolling".to_string(),
            estimated_downtime_secs: 60,
            risk_level: "High".to_string(),
            reasons: vec![],
            warnings: vec![],
            timing: TimingAdviceResponse {
                recommendation: "Maintenance window".to_string(),
                preferred_window: "Sunday 02:00".to_string(),
                avoid_times: vec![],
            },
            canary_config: CanaryConfigResponse {
                steps: vec![10, 50, 100],
                step_interval_secs: 600,
                error_threshold: 0.02,
                latency_threshold_pct: 15.0,
                min_observation_secs: 180,
            },
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["reasons"].as_array().unwrap().len(), 0);
        assert_eq!(value["warnings"].as_array().unwrap().len(), 0);
    }

    // ---------------------------------------------------------------
    // TimingAdviceResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_timing_advice_response_serialization() {
        let response = TimingAdviceResponse {
            recommendation: "Off-peak hours recommended".to_string(),
            preferred_window: "02:00-06:00 UTC".to_string(),
            avoid_times: vec![
                "Monday 09:00-12:00".to_string(),
                "Friday 15:00-18:00".to_string(),
            ],
        };
        let value = serde_json::to_value(&response).unwrap();

        assert_eq!(value["recommendation"], "Off-peak hours recommended");
        assert_eq!(value["preferred_window"], "02:00-06:00 UTC");
        let avoid = value["avoid_times"].as_array().unwrap();
        assert_eq!(avoid.len(), 2);
        assert_eq!(avoid[0], "Monday 09:00-12:00");
    }

    #[test]
    fn test_timing_advice_response_has_three_fields() {
        let response = TimingAdviceResponse {
            recommendation: "Now".to_string(),
            preferred_window: "anytime".to_string(),
            avoid_times: vec![],
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 3);
    }

    // ---------------------------------------------------------------
    // CanaryConfigResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_canary_config_response_serialization() {
        let response = CanaryConfigResponse {
            steps: vec![10, 25, 50, 75, 100],
            step_interval_secs: 300,
            error_threshold: 0.05,
            latency_threshold_pct: 10.0,
            min_observation_secs: 120,
        };
        let value = serde_json::to_value(&response).unwrap();

        let steps = value["steps"].as_array().unwrap();
        assert_eq!(steps.len(), 5);
        assert_eq!(steps[0], 10);
        assert_eq!(steps[4], 100);
        assert_eq!(value["step_interval_secs"], 300);
        assert_eq!(value["error_threshold"], 0.05);
        assert_eq!(value["latency_threshold_pct"], 10.0);
        assert_eq!(value["min_observation_secs"], 120);
    }

    #[test]
    fn test_canary_config_response_has_five_fields() {
        let response = CanaryConfigResponse {
            steps: vec![100],
            step_interval_secs: 60,
            error_threshold: 0.01,
            latency_threshold_pct: 5.0,
            min_observation_secs: 30,
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 5);
    }

    // ---------------------------------------------------------------
    // ScalingAdviceResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_scaling_advice_response_serialization() {
        let response = ScalingAdviceResponse {
            action: "ScaleUp".to_string(),
            current_replicas: 3,
            recommended_replicas: 5,
            reason: "CPU utilization trending upward".to_string(),
            confidence: 0.85,
            forecast: ForecastResponse {
                trend: "Increasing".to_string(),
                predicted_value: 0.78,
                lower_bound: 0.65,
                upper_bound: 0.92,
                horizon_minutes: 30,
            },
            cost_impact: CostImpactResponse {
                current_hourly: 1.50,
                projected_hourly: 2.50,
                delta_hourly: 1.00,
                delta_monthly: 720.0,
            },
        };
        let value = serde_json::to_value(&response).unwrap();

        assert_eq!(value["action"], "ScaleUp");
        assert_eq!(value["current_replicas"], 3);
        assert_eq!(value["recommended_replicas"], 5);
        assert_eq!(value["reason"], "CPU utilization trending upward");
        assert_eq!(value["confidence"], 0.85);
    }

    #[test]
    fn test_scaling_advice_response_has_all_fields() {
        let response = ScalingAdviceResponse {
            action: "NoOp".to_string(),
            current_replicas: 2,
            recommended_replicas: 2,
            reason: "Stable".to_string(),
            confidence: 0.95,
            forecast: ForecastResponse {
                trend: "Stable".to_string(),
                predicted_value: 0.45,
                lower_bound: 0.40,
                upper_bound: 0.50,
                horizon_minutes: 15,
            },
            cost_impact: CostImpactResponse {
                current_hourly: 1.0,
                projected_hourly: 1.0,
                delta_hourly: 0.0,
                delta_monthly: 0.0,
            },
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();

        assert!(obj.contains_key("action"));
        assert!(obj.contains_key("current_replicas"));
        assert!(obj.contains_key("recommended_replicas"));
        assert!(obj.contains_key("reason"));
        assert!(obj.contains_key("confidence"));
        assert!(obj.contains_key("forecast"));
        assert!(obj.contains_key("cost_impact"));
        assert_eq!(obj.len(), 7);
    }

    #[test]
    fn test_scaling_advice_response_scale_down() {
        let response = ScalingAdviceResponse {
            action: "ScaleDown".to_string(),
            current_replicas: 10,
            recommended_replicas: 5,
            reason: "Low utilization detected".to_string(),
            confidence: 0.72,
            forecast: ForecastResponse {
                trend: "Decreasing".to_string(),
                predicted_value: 0.20,
                lower_bound: 0.10,
                upper_bound: 0.30,
                horizon_minutes: 60,
            },
            cost_impact: CostImpactResponse {
                current_hourly: 5.0,
                projected_hourly: 2.5,
                delta_hourly: -2.5,
                delta_monthly: -1800.0,
            },
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["action"], "ScaleDown");
        assert_eq!(value["cost_impact"]["delta_hourly"], -2.5);
        assert_eq!(value["cost_impact"]["delta_monthly"], -1800.0);
    }

    // ---------------------------------------------------------------
    // ForecastResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_forecast_response_serialization() {
        let response = ForecastResponse {
            trend: "Increasing".to_string(),
            predicted_value: 0.82,
            lower_bound: 0.70,
            upper_bound: 0.95,
            horizon_minutes: 30,
        };
        let value = serde_json::to_value(&response).unwrap();

        assert_eq!(value["trend"], "Increasing");
        assert_eq!(value["predicted_value"], 0.82);
        assert_eq!(value["lower_bound"], 0.70);
        assert_eq!(value["upper_bound"], 0.95);
        assert_eq!(value["horizon_minutes"], 30);
    }

    #[test]
    fn test_forecast_response_has_five_fields() {
        let response = ForecastResponse {
            trend: "Stable".to_string(),
            predicted_value: 0.5,
            lower_bound: 0.4,
            upper_bound: 0.6,
            horizon_minutes: 15,
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 5);
    }

    // ---------------------------------------------------------------
    // CostImpactResponse serialization
    // ---------------------------------------------------------------

    #[test]
    fn test_cost_impact_response_serialization() {
        let response = CostImpactResponse {
            current_hourly: 2.50,
            projected_hourly: 3.75,
            delta_hourly: 1.25,
            delta_monthly: 900.0,
        };
        let value = serde_json::to_value(&response).unwrap();

        assert_eq!(value["current_hourly"], 2.50);
        assert_eq!(value["projected_hourly"], 3.75);
        assert_eq!(value["delta_hourly"], 1.25);
        assert_eq!(value["delta_monthly"], 900.0);
    }

    #[test]
    fn test_cost_impact_response_has_four_fields() {
        let response = CostImpactResponse {
            current_hourly: 1.0,
            projected_hourly: 1.0,
            delta_hourly: 0.0,
            delta_monthly: 0.0,
        };
        let value = serde_json::to_value(&response).unwrap();
        let obj = value.as_object().unwrap();
        assert_eq!(obj.len(), 4);
    }

    #[test]
    fn test_cost_impact_response_negative_delta() {
        let response = CostImpactResponse {
            current_hourly: 5.0,
            projected_hourly: 2.0,
            delta_hourly: -3.0,
            delta_monthly: -2160.0,
        };
        let value = serde_json::to_value(&response).unwrap();
        assert_eq!(value["delta_hourly"], -3.0);
        assert_eq!(value["delta_monthly"], -2160.0);
    }

    // ---------------------------------------------------------------
    // Full ApiResponse wrapping MigrationAdviceResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_migration_advice_json() {
        let advice = MigrationAdviceResponse {
            workload_name: "api-svc".to_string(),
            source_runtime: "Podman".to_string(),
            target_runtime: "Kubernetes".to_string(),
            recommended_strategy: "BlueGreen".to_string(),
            estimated_downtime_secs: 15,
            risk_level: "Low".to_string(),
            reasons: vec!["Scaling needs".to_string()],
            warnings: vec![],
            timing: TimingAdviceResponse {
                recommendation: "Anytime".to_string(),
                preferred_window: "any".to_string(),
                avoid_times: vec![],
            },
            canary_config: CanaryConfigResponse {
                steps: vec![50, 100],
                step_interval_secs: 120,
                error_threshold: 0.03,
                latency_threshold_pct: 8.0,
                min_observation_secs: 90,
            },
        };
        let api_resp = ApiResponse::success(advice);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["workload_name"], "api-svc");
        assert_eq!(json["data"]["timing"]["recommendation"], "Anytime");
        assert_eq!(
            json["data"]["canary_config"]["steps"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
    }

    // ---------------------------------------------------------------
    // Full ApiResponse wrapping ScalingAdviceResponse
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_wrapping_scaling_advice_json() {
        let scaling = ScalingAdviceResponse {
            action: "ScaleUp".to_string(),
            current_replicas: 2,
            recommended_replicas: 4,
            reason: "High CPU".to_string(),
            confidence: 0.90,
            forecast: ForecastResponse {
                trend: "Increasing".to_string(),
                predicted_value: 0.85,
                lower_bound: 0.70,
                upper_bound: 0.95,
                horizon_minutes: 30,
            },
            cost_impact: CostImpactResponse {
                current_hourly: 1.0,
                projected_hourly: 2.0,
                delta_hourly: 1.0,
                delta_monthly: 720.0,
            },
        };
        let api_resp = ApiResponse::success(scaling);
        let json = serde_json::to_value(&api_resp).unwrap();

        assert_eq!(json["success"], true);
        assert_eq!(json["data"]["action"], "ScaleUp");
        assert_eq!(json["data"]["forecast"]["trend"], "Increasing");
        assert_eq!(json["data"]["cost_impact"]["delta_monthly"], 720.0);
    }

    // ---------------------------------------------------------------
    // ApiResponse error wrapping for new types
    // ---------------------------------------------------------------

    #[test]
    fn test_api_response_error_typed_migration_advice() {
        let response = ApiResponse::<MigrationAdviceResponse>::error("not found".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "not found");
    }

    #[test]
    fn test_api_response_error_typed_scaling_advice() {
        let response = ApiResponse::<ScalingAdviceResponse>::error("unavailable".to_string());
        let json = serde_json::to_value(&response).unwrap();

        assert_eq!(json["success"], false);
        assert!(json["data"].is_null());
        assert_eq!(json["error"], "unavailable");
    }
}
