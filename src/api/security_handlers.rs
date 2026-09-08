// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Security API — SBOM and signed image catalog.

use super::handlers::{err_bad_request, ok_json};
use crate::sbom;
use axum::{response::IntoResponse, Json};

pub(crate) async fn api_security_sbom() -> impl IntoResponse {
    let bom = sbom::load_cached()
        .unwrap_or_else(|| sbom::generate_cyclonedx(None).unwrap_or(serde_json::json!({})));
    let meta = sbom::sbom_metadata(&bom);
    ok_json(serde_json::json!({
        "metadata": meta,
        "bom": bom
    }))
}

pub(crate) async fn api_security_images() -> impl IntoResponse {
    ok_json(serde_json::json!([]))
}

pub(crate) async fn api_security_images_verify(
    Json(_body): Json<serde_json::Value>,
) -> impl IntoResponse {
    err_bad_request::<serde_json::Value>("image catalog requires Ragnarok (separate product)")
}
