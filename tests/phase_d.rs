// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use aether::hosted::keys::TenantKeyStore;
use aether::hosted::metering::UsageMeter;
use aether::hosted::tenant::{TenantPlan, TenantStore};
use aether::intelligence::remediation::build_remediation_plan;
use aether::migration::volume::{
    execute_volume_replication, plan_volume_replication, VolumeReplicationRequest,
};
use aether::state::StateStore;

#[test]
fn tenant_key_issue_verify() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    let mut store = TenantKeyStore::load();
    let (plain, _) = store.issue("tenant-x", "ci").unwrap();
    assert!(TenantKeyStore::load().verify(&plain).is_some());
}

#[test]
fn metering_records_requests() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    UsageMeter::record("tenant-a", "/api/workloads");
    assert!(UsageMeter::load().requests_for("tenant-a") >= 1);
}

#[test]
fn tenant_upgrade_plan() {
    let dir = tempfile::tempdir().unwrap();
    std::env::set_var("HOME", dir.path());
    let mut store = TenantStore::load();
    let t = store.create("Acme", "acme", TenantPlan::Free).unwrap();
    store.upgrade_plan(&t.id, TenantPlan::Team).unwrap();
    assert_eq!(store.get(&t.id).unwrap().plan, TenantPlan::Team);
}

#[tokio::test]
async fn volume_execute_dry_run() {
    let result = execute_volume_replication(
        &VolumeReplicationRequest {
            source_cluster: "src".into(),
            target_cluster: "dst".into(),
            namespace: "default".into(),
            pvc_name: "data".into(),
            workload_name: None,
        },
        None,
        true,
    )
    .await
    .unwrap();
    assert_eq!(result.executed_steps.len(), 5);
    assert!(result.dry_run);
}

#[tokio::test]
async fn remediation_plan_empty_fleet() {
    let store = StateStore::default();
    let plan = build_remediation_plan(&store).await;
    assert!(!plan.warnings.is_empty());
}

#[test]
fn volume_plan_steps() {
    let plan = plan_volume_replication(
        &VolumeReplicationRequest {
            source_cluster: "a".into(),
            target_cluster: "b".into(),
            namespace: "default".into(),
            pvc_name: "data".into(),
            workload_name: None,
        },
        None,
    )
    .unwrap();
    assert_eq!(plan.steps.len(), 5);
}
