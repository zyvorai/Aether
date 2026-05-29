// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

use aether::fleet::edge::{EdgeEnqueueRequest, EdgeRegisterRequest, EdgeStore};
use std::collections::HashMap;
use std::env;

#[test]
fn edge_register_heartbeat_enqueue_poll() {
    let dir = tempfile::tempdir().unwrap();
    env::set_var("HOME", dir.path());

    let store = EdgeStore::load();
    store
        .register(EdgeRegisterRequest {
            site: "edge-dc1".into(),
            kube_context: Some("kind-test".into()),
            labels: HashMap::from([("region".into(), "us-east".into())]),
        })
        .unwrap();

    store
        .enqueue(EdgeEnqueueRequest {
            site: "edge-dc1".into(),
            action: "gitops_sync".into(),
            payload: serde_json::json!({"workload": "demo"}),
        })
        .unwrap();

    let agents = store.list_agents();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].site, "edge-dc1");

    let jobs = store.poll_queue("edge-dc1");
    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].action, "gitops_sync");
}

#[tokio::test]
async fn edge_executor_rejects_unknown_action() {
    use aether::fleet::edge::EdgeJob;
    use aether::fleet::edge_executor::execute_edge_job;

    let job = EdgeJob {
        id: "x".into(),
        site: "s".into(),
        action: "not_real".into(),
        payload: serde_json::json!({}),
        created_at: "now".into(),
    };
    assert!(execute_edge_job(&job).await.is_err());
}
