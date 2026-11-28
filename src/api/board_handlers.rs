// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Read-only routes behind the live status board and the workload timeline.

use super::handlers::{err_bad_request, err_internal, err_not_found, ok_json};
use super::types::AppState;
use crate::ops::{self, board, timeline};
use axum::{
    extract::{Path, Query, State as AxumState},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

const DEFAULT_WINDOW: &str = "24h";
const DEFAULT_SINCE: &str = "7d";
const DEFAULT_LIMIT: usize = 200;
const MAX_LIMIT: usize = 1_000;

#[derive(Debug, Deserialize)]
pub(crate) struct BoardQuery {
    #[serde(default)]
    pub window: Option<String>,
}

/// GET /api/board?window=1h|6h|24h|7d|30d — one tile per managed workload.
pub(crate) async fn api_board(
    AxumState(app_state): AxumState<AppState>,
    Query(q): Query<BoardQuery>,
) -> Response {
    let window = match board::parse_window(q.window.as_deref().unwrap_or(DEFAULT_WINDOW)) {
        Ok(w) => w,
        Err(e) => return err_bad_request::<serde_json::Value>(e).into_response(),
    };
    let state = app_state.state.read().await.clone();
    let observe_secs = board::observe_secs_from_env();
    // Health history can be large; keep the parsing off the async workers.
    let built = tokio::task::spawn_blocking(move || {
        board::build(
            &ops::data_dir(),
            &state,
            window,
            observe_secs,
            chrono::Utc::now(),
        )
    })
    .await;
    match built {
        Ok(Ok(b)) => ok_json(b).into_response(),
        Ok(Err(e)) => err_internal::<serde_json::Value>(format!("{e:#}")).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub(crate) struct TimelineQuery {
    #[serde(default)]
    pub since: Option<String>,
    /// Comma separated: changes, health, incidents, notes, holds.
    #[serde(default)]
    pub only: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

/// GET /api/workloads/{name}/timeline?since=7d&only=health,notes&limit=200
///
/// History outlives a workload, so the name need not be managed any more.
pub(crate) async fn api_timeline(
    AxumState(app_state): AxumState<AppState>,
    Path(name): Path<String>,
    Query(q): Query<TimelineQuery>,
) -> Response {
    let bad = |e: anyhow::Error| err_bad_request::<serde_json::Value>(e).into_response();
    if let Err(e) = ops::check_name(&name) {
        return bad(e);
    }
    let now = chrono::Utc::now();
    let since = match ops::parse_since(q.since.as_deref().unwrap_or(DEFAULT_SINCE), now) {
        Ok(s) => s,
        Err(e) => return bad(e),
    };
    let only = match timeline::parse_categories(q.only.as_deref().unwrap_or_default()) {
        Ok(o) => o,
        Err(e) => return bad(e),
    };
    let runtime = app_state.state.read().await.get(&name).map(|w| w.runtime);
    let query = timeline::Query {
        workload: name,
        runtime,
        since,
        now,
        only,
        limit: q.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT),
        gap_secs: board::stale_after_secs(board::observe_secs_from_env()),
    };
    let built =
        tokio::task::spawn_blocking(move || timeline::collect(&ops::data_dir(), &query)).await;
    match built {
        Ok(Ok(t)) if !t.known && t.unavailable.is_empty() => err_not_found::<serde_json::Value>(
            format!("nothing is recorded about '{}'", t.workload),
        )
        .into_response(),
        Ok(Ok(t)) => ok_json(t).into_response(),
        Ok(Err(e)) => err_internal::<serde_json::Value>(format!("{e:#}")).into_response(),
        Err(e) => err_internal::<serde_json::Value>(e).into_response(),
    }
}
