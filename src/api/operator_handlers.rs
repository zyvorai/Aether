// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! REST API for the operator features in [`crate::ops`].

use super::handlers::{err_bad_request, err_internal, err_not_found, ok_json};
use super::types::AppState;
use crate::ops::{self, holds, revisions};
use crate::state::{StateStore, WorkloadState};
use axum::{
    extract::{Path, Query, State as AxumState},
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
