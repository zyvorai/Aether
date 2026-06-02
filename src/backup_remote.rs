// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Optional HTTP upload of backup files after creation (S3 presigned URL, MinIO, or custom receiver).
//!
//! - `AETHER_BACKUP_REMOTE_URL` — POST target (e.g. presigned S3 PUT or internal backup API)
//! - `AETHER_BACKUP_REMOTE_TOKEN` — optional `Authorization: Bearer` value

use std::path::Path;

/// Upload backup bytes when remote URL is configured (errors are logged, not propagated).
pub async fn upload_backup_file_if_configured(path: &Path) {
    let url = match std::env::var("AETHER_BACKUP_REMOTE_URL") {
        Ok(u) if !u.trim().is_empty() => u,
        _ => return,
    };
    let bytes = match tokio::fs::read(path).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!(error = %e, "backup remote upload: read file failed");
            return;
        }
    };
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "backup remote upload: http client");
            return;
        }
    };
    let mut req = client.put(&url).body(bytes);
    if let Ok(token) = std::env::var("AETHER_BACKUP_REMOTE_TOKEN") {
        if !token.trim().is_empty() {
            req = req.header("Authorization", format!("Bearer {}", token.trim()));
        }
    }
    match req.send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!(url = %url, "backup uploaded to remote target");
        }
        Ok(resp) => {
            tracing::warn!(
                status = %resp.status(),
                url = %url,
                "backup remote upload failed"
            );
        }
        Err(e) => tracing::warn!(error = %e, url = %url, "backup remote upload error"),
    }
}
