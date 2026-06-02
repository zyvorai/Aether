// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

//! Migration API — volume replication and fleet migration plans.

use super::handlers::{err_bad_request, ok_json};
use super::types::AppState;
use crate::migration::fleet::{plan_fleet_migration, FleetMigrationRequest};
use crate::migration::volume::{
    execute_volume_replication, plan_volume_replication, VolumeReplicationRequest,
};
use crate::spec::Workload;
use axum::{extract::State as AxumState, response::IntoResponse, Json};

pub(crate) async fn api_migration_volume_plan(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<VolumeReplicationRequest>,
) -> impl IntoResponse {
    let spec = if let Some(ref name) = body.workload_name {
        let store = app_state.state.read().await;
        let ws = match store.get(name) {
            Some(w) => w,
            None => return err_bad_request(format!("workload '{name}' not found")),
        };
        Workload::from_file(&ws.spec_path).ok()
    } else {
        None
    };
    match plan_volume_replication(&body, spec.as_ref()) {
        Ok(plan) => ok_json(plan),
        Err(e) => err_bad_request(e),
    }
}

#[derive(Debug, serde::Deserialize)]
pub(crate) struct VolumeExecuteBody {
    #[serde(flatten)]
    pub request: VolumeReplicationRequest,
    pub dry_run: Option<bool>,
}

pub(crate) async fn api_migration_volume_execute(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<VolumeExecuteBody>,
) -> impl IntoResponse {
    let spec = if let Some(ref name) = body.request.workload_name {
        let store = app_state.state.read().await;
        let ws = match store.get(name) {
            Some(w) => w,
            None => return err_bad_request(format!("workload '{name}' not found")),
        };
        Workload::from_file(&ws.spec_path).ok()
    } else {
        None
    };
    let dry_run = body.dry_run.unwrap_or(true);
    match execute_volume_replication(&body.request, spec.as_ref(), dry_run).await {
        Ok(result) => ok_json(result),
        Err(e) => err_bad_request(e),
    }
}

pub(crate) async fn api_migration_fleet_plan(
    AxumState(app_state): AxumState<AppState>,
    Json(body): Json<FleetMigrationRequest>,
) -> impl IntoResponse {
    let store = app_state.state.read().await;
    let mut specs = Vec::new();
    for name in &body.workloads {
        let Some(ws) = store.get(name) else {
            return err_bad_request(format!("workload '{name}' not found"));
        };
        let path = ws.spec_path.clone();
        let runtime = ws.runtime;
        match Workload::from_file(&path) {
            Ok(s) => specs.push((name.clone(), s, runtime)),
            Err(e) => return err_bad_request(e),
        }
    }
    drop(store);
    match plan_fleet_migration(&body, &specs) {
        Ok(plan) => ok_json(plan),
        Err(e) => err_bad_request(e),
    }
}
