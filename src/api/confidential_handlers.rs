//! Ragnarok confidential computing API handlers.

use super::handlers::{err_bad_request, err_internal, err_not_found, ok_json};
use super::types::AppState;
use crate::ragnarok::{
    attestation::{AttestationReport, AttestationService, AttestationVerdict},
    guestkit::{inspect, GuestKitRequest, InspectionMode},
    image::{attestation_digest_gate, ImageCatalog, ImageManifest, ImageVerifyResult},
    migration::plan_confidential_migration,
    secrets::{BrokerRequest, SecretBroker, SecretBrokerProvider},
    sovereign::SovereignConfig,
    tee::{probe_host_tee, TeeCapabilities},
    trust::fleet_trust_scores,
    isolation::{self, IsolationPolicy, IsolationVerdict},
};
use crate::spec::Workload;
use axum::{
    extract::{Path, State as AxumState},
    response::IntoResponse,
    Json,
};
use std::path::PathBuf;
use std::sync::Arc;

fn state_dir(app_state: &AppState) -> PathBuf {
    app_state
        .state_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".aether")
        })
}

fn attestation_service(app_state: &AppState) -> AttestationService {
    AttestationService::new(state_dir(app_state))
}

fn secret_broker(app_state: &AppState) -> SecretBroker {
    SecretBroker::new(
        Arc::new(attestation_service(app_state)),
        &state_dir(app_state),
    )
}

fn image_catalog(app_state: &AppState) -> ImageCatalog {
    ImageCatalog::load(&state_dir(app_state))
}

async fn try_workload_spec(app_state: &AppState, name: &str) -> Option<Workload> {
    let store = app_state.state.read().await;
    let ws = store.get(name)?;
    Workload::from_file(&ws.spec_path).ok()
}

pub(crate) async fn api_confidential_capabilities(
    AxumState(_app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let host = probe_host_tee();
    let integration = match std::env::var("RAGNAROK_URL") {
        Ok(url) if !url.trim().is_empty() => crate::ragnarok::tee::RagnarokIntegration {
            mode: "composite".into(),
            remote_url: Some(url.trim().to_string()),
        },
        _ => crate::ragnarok::tee::RagnarokIntegration {
            mode: "embedded".into(),
            remote_url: None,
        },
    };
    let caps = TeeCapabilities {
        host,
        clusters: vec![],
        integration,
    };
    ok_json(caps)
}

pub(crate) async fn api_attestation_verify(
    AxumState(app_state): AxumState<AppState>,
    Json(report): Json<AttestationReport>,
) -> impl IntoResponse {
    let catalog = image_catalog(&app_state);
    if let Err(e) = attestation_digest_gate(&report, &catalog) {
        return err_bad_request::<serde_json::Value>(e).into_response();
    }

    let svc = attestation_service(&app_state);
    match svc.verify(&report) {
        Ok(resp) => {
            let broker = secret_broker(&app_state);
            match resp.verdict {
                AttestationVerdict::Pass => {
                    if let Some(spec) = try_workload_spec(&app_state, &report.vm_id).await {
                        let provider = spec
                            .confidential
                            .as_ref()
                            .map(|c| SecretBrokerProvider::from(c.secrets.provider.clone()))
                            .unwrap_or(SecretBrokerProvider::Vault);
                        match broker.release_all_pending(&report.vm_id, provider).await {
                            Ok(tokens) if !tokens.is_empty() => {
                                tracing::info!(
                                    vm_id = %report.vm_id,
                                    count = tokens.len(),
                                    "attestation-gated secrets released"
                                );
                                let payload = serde_json::json!({
                                    "type": "secrets.released",
                                    "vm_id": report.vm_id,
                                    "count": tokens.len(),
                                });
                                let _ = app_state.event_tx.send(payload.to_string());
                            }
                            Ok(_) => {}
                            Err(e) => tracing::warn!(
                                vm_id = %report.vm_id,
                                error = %e,
                                "secret release after attestation failed"
                            ),
                        }
                    }
                }
                AttestationVerdict::Fail => {
                    broker.on_attestation_failure(&report.vm_id, AttestationVerdict::Fail);
                    emit_attestation_event(&app_state, &report.vm_id, AttestationVerdict::Fail);
                }
                AttestationVerdict::Pending => {}
            }
            ok_json(resp).into_response()
        }
        Err(e) => err_bad_request::<serde_json::Value>(e).into_response(),
    }
}

pub(crate) async fn api_attestation_status(
    AxumState(app_state): AxumState<AppState>,
    Path(vm_id): Path<String>,
) -> impl IntoResponse {
    let svc = attestation_service(&app_state);
    match svc.status(&vm_id) {
        Some(status) => ok_json(status).into_response(),
        None => err_not_found::<serde_json::Value>(format!("no attestation record for {vm_id}"))
            .into_response(),
    }
}

pub(crate) async fn api_attestation_explain(
    AxumState(app_state): AxumState<AppState>,
    Path(vm_id): Path<String>,
) -> impl IntoResponse {
    let svc = attestation_service(&app_state);
    match svc.explain(&vm_id) {
        Ok(explain) => ok_json(explain).into_response(),
        Err(e) => err_not_found::<serde_json::Value>(e.to_string()).into_response(),
    }
}

pub(crate) async fn api_confidential_trust_score(
    AxumState(app_state): AxumState<AppState>,
    Path(workload): Path<String>,
) -> impl IntoResponse {
    let svc = attestation_service(&app_state);
    let store = app_state.state.read().await;
    let ws = match store.get(&workload) {
        Some(w) => w,
        None => {
            return err_not_found::<serde_json::Value>(format!("workload {workload} not found"))
                .into_response();
        }
    };
    let spec = match Workload::from_file(&ws.spec_path) {
        Ok(s) => s,
        Err(e) => return err_internal::<serde_json::Value>(e).into_response(),
    };
    let debug = spec
        .confidential
        .as_ref()
        .map(|c| c.isolation.debug_allowed)
        .unwrap_or(false);
    let digest = spec
        .confidential
        .as_ref()
        .and_then(|c| c.image_digest.as_deref());
    let score = crate::ragnarok::network::compute_trust_score(
        &workload,
        svc.passed(&workload),
        if spec.confidential.as_ref().is_some_and(|c| c.enabled) {
            1
        } else {
            0
        },
        debug,
        digest,
    );
    ok_json(score).into_response()
}

pub(crate) async fn api_confidential_trust_fleet(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    let svc = attestation_service(&app_state);
    let store = app_state.state.read().await;
    let mut pairs = Vec::new();
    for w in store.list() {
        if let Ok(spec) = Workload::from_file(&w.spec_path) {
            if spec.confidential.as_ref().is_some_and(|c| c.enabled) {
                pairs.push((w.name.clone(), spec));
            }
        }
    }
    let refs: Vec<(&str, &Workload)> = pairs.iter().map(|(n, s)| (n.as_str(), s)).collect();
    ok_json(fleet_trust_scores(&refs, &svc))
}

#[derive(serde::Deserialize)]
pub(crate) struct SecretReleaseRequest {
    pub vm_id: String,
    pub secret_name: String,
    #[serde(default = "default_broker_provider")]
    pub provider: String,
}

fn default_broker_provider() -> String {
    "vault".into()
}

pub(crate) async fn api_confidential_secret_release(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<SecretReleaseRequest>,
) -> impl IntoResponse {
    let broker = secret_broker(&app_state);
    let provider = match req.provider.as_str() {
        "kbs" => SecretBrokerProvider::Kbs,
        "aws-kms" => SecretBrokerProvider::AwsKms,
        "azure-kv" => SecretBrokerProvider::AzureKv,
        _ => SecretBrokerProvider::Vault,
    };
    match broker
        .request_release(&BrokerRequest {
            vm_id: req.vm_id,
            secret_name: req.secret_name,
            provider,
        })
        .await
    {
        Ok(token) => ok_json(token).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(e).into_response(),
    }
}

pub(crate) async fn api_confidential_secret_status(
    AxumState(app_state): AxumState<AppState>,
    Path(workload): Path<String>,
) -> impl IntoResponse {
    let broker = secret_broker(&app_state);
    ok_json(broker.status_for_vm(&workload)).into_response()
}

pub(crate) async fn api_confidential_migration_plan(
    AxumState(app_state): AxumState<AppState>,
    Path((name, _target)): Path<(String, String)>,
) -> impl IntoResponse {
    let store = app_state.state.read().await;
    let ws = match store.get(&name) {
        Some(w) => w,
        None => {
            return err_not_found::<serde_json::Value>(format!("workload {name} not found"))
                .into_response();
        }
    };
    let spec = match Workload::from_file(&ws.spec_path) {
        Ok(s) => s,
        Err(e) => return err_internal::<serde_json::Value>(e).into_response(),
    };
    let host = probe_host_tee();
    ok_json(plan_confidential_migration(&spec, host.sev_snp, host.sev_snp)).into_response()
}

#[derive(serde::Deserialize)]
pub(crate) struct GuestKitInspectRequest {
    pub vm_id: String,
    pub image_path: Option<String>,
    #[serde(default = "default_guestkit_mode")]
    pub mode: String,
}

fn default_guestkit_mode() -> String {
    "pre-launch".into()
}

pub(crate) async fn api_guestkit_inspect(
    Json(req): Json<GuestKitInspectRequest>,
) -> impl IntoResponse {
    let mode = match req.mode.as_str() {
        "offline-policy" => InspectionMode::OfflinePolicy,
        "post-shutdown" => InspectionMode::PostShutdown,
        "attested-repair" => InspectionMode::AttestedRepair,
        _ => InspectionMode::PreLaunch,
    };
    ok_json(inspect(&GuestKitRequest {
        vm_id: req.vm_id,
        image_path: req.image_path,
        mode,
        policy_manifest: None,
    }))
}

pub(crate) async fn api_confidential_sovereign_status() -> impl IntoResponse {
    ok_json(SovereignConfig::from_env())
}

pub(crate) async fn api_confidential_image_catalog(
    AxumState(app_state): AxumState<AppState>,
) -> impl IntoResponse {
    ok_json(image_catalog(&app_state).list())
}

#[derive(serde::Deserialize)]
pub(crate) struct ImageSignRequest {
    pub name: String,
    pub path: String,
    #[serde(default = "default_signing_key")]
    pub signing_key_id: String,
}

fn default_signing_key() -> String {
    "cosign://aether".into()
}

pub(crate) async fn api_confidential_image_sign(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<ImageSignRequest>,
) -> impl IntoResponse {
    let catalog = image_catalog(&app_state);
    match catalog.sign(&req.name, std::path::Path::new(&req.path), &req.signing_key_id) {
        Ok(m) => ok_json(m).into_response(),
        Err(e) => err_bad_request::<serde_json::Value>(e).into_response(),
    }
}

#[derive(serde::Deserialize)]
pub(crate) struct ImageVerifyRequest {
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub digest: Option<String>,
}

pub(crate) async fn api_confidential_image_verify(
    AxumState(app_state): AxumState<AppState>,
    Json(req): Json<ImageVerifyRequest>,
) -> impl IntoResponse {
    let catalog = image_catalog(&app_state);
    if let Some(ref path) = req.path {
        match catalog.verify(&req.name, std::path::Path::new(path)) {
            Ok(true) => {
                let m: Option<ImageManifest> = catalog.get(&req.name);
                return ok_json(ImageVerifyResult {
                    name: req.name,
                    verified: true,
                    image_hash: m.as_ref().map(|x| x.image_hash.clone()),
                    launch_digest: m.and_then(|x| x.launch_digest),
                    message: "image file matches catalog".into(),
                })
                .into_response();
            }
            Ok(false) => {
                return ok_json(ImageVerifyResult {
                    name: req.name,
                    verified: false,
                    image_hash: None,
                    launch_digest: None,
                    message: "image hash mismatch".into(),
                })
                .into_response();
            }
            Err(e) => return err_bad_request::<serde_json::Value>(e).into_response(),
        }
    }
    if let Some(ref digest) = req.digest {
        let verified = catalog.verify_digest(digest);
        return ok_json(ImageVerifyResult {
            name: req.name,
            verified,
            image_hash: None,
            launch_digest: Some(digest.clone()),
            message: if verified {
                "digest found in catalog".into()
            } else {
                "digest not in catalog".into()
            },
        })
        .into_response();
    }
    err_bad_request::<serde_json::Value>("provide path or digest").into_response()
}

pub(crate) async fn api_confidential_isolation(
    AxumState(app_state): AxumState<AppState>,
    Path(workload): Path<String>,
) -> impl IntoResponse {
    let Some(spec) = try_workload_spec(&app_state, &workload).await else {
        return err_not_found::<IsolationVerdict>(format!("workload {workload} not found"))
            .into_response();
    };
    let policy = IsolationPolicy::from_env();
    ok_json(isolation::evaluate(&spec, &policy)).into_response()
}

/// Record attestation failure event for audit consumers.
pub fn emit_attestation_event(
    app_state: &AppState,
    vm_id: &str,
    verdict: AttestationVerdict,
) {
    if verdict == AttestationVerdict::Fail {
        let payload = serde_json::json!({
            "type": "attestation.failed",
            "vm_id": vm_id,
            "verdict": "fail",
        });
        let _ = app_state.event_tx.send(payload.to_string());
    }
}
