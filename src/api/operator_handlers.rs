// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! REST API for the operator features in [`crate::ops`].

use super::handlers::{err_bad_request, err_internal, err_not_found, ok_json};
use super::types::AppState;
use crate::ops::{self, changes, holds, incidents, revisions};
use crate::state::{StateStore, WorkloadState};
use axum::{
    extract::{Path, Query, State as AxumState},
    http::header,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

/// A ready-made error response, boxed to keep `Result`s small.
type Reject = Box<Response>;

fn not_found(message: impl std::fmt::Display) -> Response {
    err_not_found::<()>(message.to_string()).into_response()
}

fn internal(e: anyhow::Error) -> Response {
    err_internal::<()>(format!("{e:#}")).into_response()
}

fn bad_request(e: impl std::fmt::Display) -> Response {
    err_bad_request::<()>(e).into_response()
}

/// Run blocking file work off the async workers, keeping the caller's actor.
async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> anyhow::Result<T> + Send + 'static,
) -> anyhow::Result<T> {
    tokio::task::spawn_blocking(work).await?
}

/// The managed workloads (kept in step with the state file by the auth layer).
async fn current_state(app: &AppState) -> StateStore {
    app.state.read().await.clone()
}

async fn workload(app: &AppState, name: &str) -> Result<WorkloadState, Reject> {
    if ops::check_name(name).is_err() {
        return Err(Box::new(bad_request("invalid workload name")));
    }
    current_state(app)
        .await
        .get(name)
        .cloned()
        .ok_or_else(|| Box::new(not_found(format!("Workload {name} not found"))))
}

macro_rules! try_response {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(response) => return *response,
        }
    };
}

fn respond<T: serde::Serialize>(result: anyhow::Result<T>) -> Response {
    match result {
        Ok(v) => ok_json(v).into_response(),
        Err(e) => internal(e),
    }
}

fn window(since: Option<&str>, default: &str) -> Result<chrono::DateTime<chrono::Utc>, Reject> {
    ops::parse_since(since.unwrap_or(default), chrono::Utc::now())
        .map_err(|e| Box::new(bad_request(e)))
}

// ── Revisions ────────────────────────────────────────────────────────────────

/// GET /api/workloads/{name}/revisions
pub(crate) async fn api_revisions(Path(name): Path<String>) -> Response {
    if ops::check_name(&name).is_err() {
        return bad_request("invalid workload name");
    }
    // History outlives the workload, so no state lookup here.
    respond(blocking(move || Ok(revisions::load(&ops::data_dir(), &name)?.summaries())).await)
}

/// GET /api/workloads/{name}/revisions/{id}
pub(crate) async fn api_revision(Path((name, id)): Path<(String, u64)>) -> Response {
    if ops::check_name(&name).is_err() {
        return bad_request("invalid workload name");
    }
    match revisions::load(&ops::data_dir(), &name) {
        Ok(log) => match log.view(id) {
            Ok(view) => ok_json(view).into_response(),
            Err(e) => err_not_found::<()>(format!("{e:#}")).into_response(),
        },
        Err(e) => internal(e),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct DiffQuery {
    from: u64,
    to: u64,
}

/// GET /api/workloads/{name}/revision-diff?from=&to=
pub(crate) async fn api_revision_diff(
    Path(name): Path<String>,
    Query(q): Query<DiffQuery>,
) -> Response {
    if ops::check_name(&name).is_err() {
        return bad_request("invalid workload name");
    }
    match revisions::load(&ops::data_dir(), &name) {
        Ok(log) => match log.diff(q.from, q.to) {
            Ok(plan) => ok_json(plan).into_response(),
            Err(e) => err_not_found::<()>(format!("{e:#}")).into_response(),
        },
        Err(e) => internal(e),
    }
}

// ── Changes, attention, report ───────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct ChangesQuery {
    since: Option<String>,
    workload: Option<String>,
    limit: Option<usize>,
}

/// GET /api/changes?since=24h&workload=&limit=
pub(crate) async fn api_changes(Query(q): Query<ChangesQuery>) -> Response {
    let since = try_response!(window(q.since.as_deref(), "24h"));
    let query = changes::Query::since(
        since,
        q.workload.filter(|w| !w.is_empty()),
        q.limit.unwrap_or(200).clamp(1, 1000),
    );
    respond(blocking(move || changes::collect(&ops::data_dir(), &query)).await)
}

// ── Incidents ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub(crate) struct IncidentQuery {
    workload: Option<String>,
    #[serde(default)]
    open: bool,
}

#[derive(serde::Serialize)]
struct IncidentList {
    /// Newest first.
    incidents: Vec<IncidentSummary>,
    recovery: incidents::Recovery,
    observation_threshold: u32,
}

#[derive(serde::Serialize)]
struct IncidentSummary {
    #[serde(flatten)]
    incident: incidents::Incident,
    duration_secs: i64,
}

/// GET /api/incidents?workload=&open=
pub(crate) async fn api_incidents(Query(q): Query<IncidentQuery>) -> Response {
    respond(
        blocking(move || {
            let store = incidents::load(&ops::data_dir())?;
            let wanted: Vec<_> = store
                .incidents
                .iter()
                .filter(|i| {
                    q.workload
                        .as_deref()
                        .is_none_or(|w| w.is_empty() || w == i.workload)
                })
                .collect();
            let now = chrono::Utc::now();
            Ok(IncidentList {
                recovery: incidents::recovery(wanted.iter().copied()),
                incidents: wanted
                    .into_iter()
                    .rev()
                    .filter(|i| !q.open || i.is_open())
                    .map(|i| IncidentSummary {
                        duration_secs: i.duration_secs(now),
                        incident: i.clone(),
                    })
                    .collect(),
                observation_threshold: incidents::threshold(),
            })
        })
        .await,
    )
}

fn incident_detail(id: u64) -> Result<incidents::IncidentDetail, Reject> {
    let dir = ops::data_dir();
    let store = incidents::load(&dir).map_err(|e| Box::new(internal(e)))?;
    if store.get(id).is_none() {
        return Err(Box::new(not_found(format!("incident {id} not found"))));
    }
    incidents::detail(&dir, id).map_err(|e| Box::new(internal(e)))
}

/// GET /api/incidents/{id}
pub(crate) async fn api_incident(Path(id): Path<u64>) -> Response {
    ok_json(try_response!(incident_detail(id))).into_response()
}

/// GET /api/incidents/{id}/report — Markdown post-incident draft.
pub(crate) async fn api_incident_report(Path(id): Path<u64>) -> Response {
    let detail = try_response!(incident_detail(id));
    (
        [
            (
                header::CONTENT_TYPE,
                "text/markdown; charset=utf-8".to_string(),
            ),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"incident-{id}.md\""),
            ),
        ],
        incidents::markdown(&detail),
    )
        .into_response()
}

/// POST /api/incidents/{id}/ack
pub(crate) async fn api_incident_ack(Path(id): Path<u64>) -> Response {
    match incidents::acknowledge(&ops::data_dir(), id) {
        Ok(i) => ok_json(i).into_response(),
        Err(e) => not_found(format!("{e:#}")),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct NoteRequest {
    text: String,
}

/// POST /api/incidents/{id}/notes
pub(crate) async fn api_incident_note(
    Path(id): Path<u64>,
    Json(req): Json<NoteRequest>,
) -> Response {
    let dir = ops::data_dir();
    match incidents::load(&dir) {
        Ok(store) if store.get(id).is_none() => {
            return not_found(format!("incident {id} not found"))
        }
        Ok(_) => {}
        Err(e) => return internal(e),
    }
    match incidents::add_note(&dir, id, &req.text) {
        Ok(i) => ok_json(i).into_response(),
        Err(e) => bad_request(format!("{e:#}")),
    }
}

/// POST /api/observe — check every workload's status now.
pub(crate) async fn api_observe(AxumState(app): AxumState<AppState>) -> Response {
    let state = current_state(&app).await;
    let workloads: Vec<_> = state.list().into_iter().cloned().collect();
    respond(incidents::collect(&ops::data_dir(), &workloads, None).await)
}

// ── Holds ────────────────────────────────────────────────────────────────────

/// GET /api/holds
pub(crate) async fn api_holds() -> Response {
    respond(holds::active(&ops::data_dir()))
}

#[derive(Debug, Deserialize)]
pub(crate) struct HoldRequest {
    workload: String,
    reason: String,
    /// `30m`, `2h`, `7d`; omitted holds until released.
    #[serde(default)]
    duration: Option<String>,
}

/// POST /api/holds
pub(crate) async fn api_hold_set(
    AxumState(app): AxumState<AppState>,
    Json(req): Json<HoldRequest>,
) -> Response {
    try_response!(workload(&app, &req.workload).await);
    let duration = match req.duration.as_deref().filter(|d| !d.is_empty()) {
        Some(d) => match ops::parse_duration_secs(d) {
            Ok(s) => Some(s),
            Err(e) => return bad_request(e),
        },
        None => None,
    };
    match holds::set(&ops::data_dir(), &req.workload, &req.reason, duration) {
        Ok(h) => ok_json(h).into_response(),
        Err(e) => bad_request(format!("{e:#}")),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct ReleaseRequest {
    workload: String,
}

/// POST /api/holds/release
pub(crate) async fn api_hold_release(Json(req): Json<ReleaseRequest>) -> Response {
    if ops::check_name(&req.workload).is_err() {
        return bad_request("invalid workload name");
    }
    match holds::release(&ops::data_dir(), &req.workload) {
        Ok(Some(h)) => ok_json(h).into_response(),
        Ok(None) => {
            err_not_found::<()>(format!("'{}' is not on hold", req.workload)).into_response()
        }
        Err(e) => internal(e),
    }
}
