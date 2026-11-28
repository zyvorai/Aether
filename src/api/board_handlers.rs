// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-2.0

//! Read-only routes behind the live status board.

use super::handlers::{err_bad_request, err_internal, ok_json};
use super::types::AppState;
use crate::ops::{self, board};
use axum::{
    extract::{Query, State as AxumState},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

const DEFAULT_WINDOW: &str = "24h";

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
    let observe_secs = super::observe_interval_secs();
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
