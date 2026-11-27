// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! REST API for the operator features in [`crate::ops`].

use super::handlers::{err_bad_request, err_internal, err_not_found, ok_json};
use crate::ops::{self, revisions};
use axum::{
    extract::{Path, Query},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

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
