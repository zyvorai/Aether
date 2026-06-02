// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Security API — SBOM and signed image catalog.

use super::handlers::{err_bad_request, ok_json};
use crate::ragnarok::image::{ImageCatalog, ImageVerifyResult};
use crate::sbom;
use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Deserialize;
use std::path::PathBuf;

fn state_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".aether")
}

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
    let catalog = ImageCatalog::load(&state_dir());
    ok_json(catalog.list())
}

#[derive(Debug, Deserialize)]
pub(crate) struct ImageVerifyBody {
    pub name: Option<String>,
    pub digest: Option<String>,
}

pub(crate) async fn api_security_images_verify(
    Json(body): Json<ImageVerifyBody>,
) -> impl IntoResponse {
    let catalog = ImageCatalog::load(&state_dir());
    if let Some(ref digest) = body.digest {
        let verified = catalog.verify_digest(digest);
        return ok_json(ImageVerifyResult {
            name: body.name.unwrap_or_else(|| "digest".into()),
            verified,
            image_hash: None,
            launch_digest: Some(digest.clone()),
            message: if verified {
                "digest found in catalog".into()
            } else {
                "digest not in catalog".into()
            },
        });
    }
    if let Some(name) = body.name {
        if let Some(m) = catalog.get(&name) {
            return ok_json(ImageVerifyResult {
                name: name.clone(),
                verified: true,
                image_hash: Some(m.image_hash.clone()),
                launch_digest: m.launch_digest.clone(),
                message: "catalog entry present".into(),
            });
        }
        return (
            StatusCode::NOT_FOUND,
            Json(super::types::ApiResponse {
                success: false,
                data: None,
                error: Some(format!("image '{name}' not in catalog")),
            }),
        );
    }
    err_bad_request("provide name or digest")
}
