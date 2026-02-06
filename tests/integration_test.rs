//! Integration tests for Orchestr8

use orchestr8::{
    engine::Engine,
    runtime::RuntimeKind,
    spec::Workload,
    state::StateStore,
};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Create a test workload spec
fn create_test_workload(name: &str, temp_dir: &TempDir) -> (Workload, PathBuf) {
    let spec_content = format!(
        r#"
apiVersion: orchestr8/v1
kind: Workload

metadata:
  name: {}
  owner: test-user
  project: test-project

build:
  context: .
  dockerfile: Dockerfile
  registry: ghcr.io/test

requirements:
  cpu: "2"
  memory: "4Gi"
  storage: "10Gi"

runtime:
  preferred: auto
  allow:
    - container
    - kube

network:
  service: true
  ports:
    - containerPort: 8080
      servicePort: 8080
      protocol: TCP

persistence:
  enabled: true
  size: "10Gi"
  storage_class: "standard"
  access_mode: ReadWriteOnce
"#,
        name
    );

    let spec_path = temp_dir.path().join(format!("{}.yaml", name));
    fs::write(&spec_path, spec_content).unwrap();

    let workload = Workload::from_file(&spec_path).unwrap();
    (workload, spec_path)
}

#[test]
fn test_workload_parsing_and_validation() {
    let temp_dir = TempDir::new().unwrap();
    let (workload, _) = create_test_workload("test-app", &temp_dir);

    assert_eq!(workload.metadata.name, "test-app");
    assert_eq!(workload.metadata.owner, "test-user");
    assert_eq!(workload.requirements.cpu, "2");
    assert_eq!(workload.requirements.memory, "4Gi");
    assert!(workload.network.service);
    assert!(workload.persistence.enabled);
}

#[test]
fn test_decision_engine_auto_selection() {
    let temp_dir = TempDir::new().unwrap();
    let (workload, _) = create_test_workload("test-app", &temp_dir);

    let engine = Engine::new();
    let runtime = engine.decide(&workload).unwrap();

    // With service and persistence, should select Kubernetes if available
    // Otherwise falls back to Podman
    assert!(
        runtime == RuntimeKind::Kubernetes || runtime == RuntimeKind::Podman,
        "Expected Kubernetes or Podman, got {:?}",
        runtime
    );
}

#[test]
fn test_state_store_operations() {
    use orchestr8::runtime::Instance;
    use orchestr8::state::WorkloadState;

    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.json");

    // Create initial state
    let mut state = StateStore::new();
    assert_eq!(state.list().len(), 0);

    // Add workload
    let workload_state = WorkloadState {
        name: "test-app".to_string(),
        runtime: RuntimeKind::Podman,
        instance: Instance {
            id: "abc123".to_string(),
            name: "test-app-container".to_string(),
            runtime: RuntimeKind::Podman,
            image: "test:latest".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        },
        spec_path: PathBuf::from("/tmp/test.yaml"),
        created_at: chrono::Utc::now().to_rfc3339(),
        updated_at: chrono::Utc::now().to_rfc3339(),
    };

    state.upsert("test-app".to_string(), workload_state);
    assert_eq!(state.list().len(), 1);

    // Save state
    state.save(&state_path).unwrap();
    assert!(state_path.exists());

    // Load state
    let loaded_state = StateStore::load(&state_path).unwrap();
    assert_eq!(loaded_state.list().len(), 1);

    let loaded = loaded_state.get("test-app").unwrap();
    assert_eq!(loaded.name, "test-app");
    assert_eq!(loaded.runtime, RuntimeKind::Podman);

    // Remove workload
    let mut state = loaded_state;
    state.remove("test-app");
    assert_eq!(state.list().len(), 0);
}

#[test]
fn test_image_name_generation() {
    let temp_dir = TempDir::new().unwrap();
    let (workload, _) = create_test_workload("test-app", &temp_dir);

    let image_name = workload.image_name();
    assert_eq!(image_name, "ghcr.io/test/test-app:latest");
}

#[test]
fn test_runtime_selection_with_gpu() {
    let temp_dir = TempDir::new().unwrap();

    let spec_content = r#"
apiVersion: orchestr8/v1
kind: Workload

metadata:
  name: gpu-app
  owner: test-user
  project: ml

build:
  context: .
  dockerfile: Dockerfile
  registry: ghcr.io/test

requirements:
  cpu: "8"
  memory: "32Gi"
  storage: "100Gi"
  gpu:
    vendor: nvidia
    count: 2

runtime:
  preferred: auto
  allow:
    - kubevirt
    - metal

network:
  service: true
  ports:
    - containerPort: 8080
      servicePort: 8080
      protocol: TCP

persistence:
  enabled: true
  size: "100Gi"
  storage_class: "nvme"
  access_mode: ReadWriteOnce
"#;

    let spec_path = temp_dir.path().join("gpu-app.yaml");
    fs::write(&spec_path, spec_content).unwrap();

    let workload = Workload::from_file(&spec_path).unwrap();
    let engine = Engine::new();
    let runtime = engine.decide(&workload).unwrap();

    // With GPU requirement, should select KubeVirt or Metal3
    assert!(
        runtime == RuntimeKind::KubeVirt || runtime == RuntimeKind::Metal3,
        "Expected KubeVirt or Metal3 for GPU workload, got {:?}",
        runtime
    );
}

#[test]
fn test_multiple_workloads_in_state() {
    use orchestr8::runtime::Instance;
    use orchestr8::state::WorkloadState;

    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.json");

    let mut state = StateStore::new();

    // Add multiple workloads
    for i in 1..=5 {
        let workload_state = WorkloadState {
            name: format!("app-{}", i),
            runtime: if i % 2 == 0 {
                RuntimeKind::Kubernetes
            } else {
                RuntimeKind::Podman
            },
            instance: Instance {
                id: format!("id-{}", i),
                name: format!("app-{}-instance", i),
                runtime: if i % 2 == 0 {
                    RuntimeKind::Kubernetes
                } else {
                    RuntimeKind::Podman
                },
                image: format!("app-{}:latest", i),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            spec_path: PathBuf::from(format!("/tmp/app-{}.yaml", i)),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        state.upsert(format!("app-{}", i), workload_state);
    }

    assert_eq!(state.list().len(), 5);

    // Save and reload
    state.save(&state_path).unwrap();
    let loaded_state = StateStore::load(&state_path).unwrap();
    assert_eq!(loaded_state.list().len(), 5);

    // Verify each workload
    for i in 1..=5 {
        let workload = loaded_state.get(&format!("app-{}", i)).unwrap();
        assert_eq!(workload.name, format!("app-{}", i));
    }
}

#[test]
fn test_workload_spec_validation_errors() {
    let temp_dir = TempDir::new().unwrap();

    // Invalid spec: missing required fields
    let invalid_spec = r#"
apiVersion: orchestr8/v1
kind: Workload
metadata:
  name: invalid-app
"#;

    let spec_path = temp_dir.path().join("invalid.yaml");
    fs::write(&spec_path, invalid_spec).unwrap();

    // Should fail to parse
    let result = Workload::from_file(&spec_path);
    assert!(result.is_err(), "Expected error for invalid spec");
}

#[test]
fn test_migration_plan_structure() {
    use orchestr8::migration::{MigrationPlan, MigrationStrategy};
    use std::time::Duration;

    let plan = MigrationPlan {
        workload_name: "test-app".to_string(),
        source_runtime: RuntimeKind::Podman,
        target_runtime: RuntimeKind::Kubernetes,
        strategy: MigrationStrategy::BlueGreen,
        validation_delay: Duration::from_secs(30),
        rollback_on_failure: true,
    };

    assert_eq!(plan.workload_name, "test-app");
    assert_eq!(plan.source_runtime, RuntimeKind::Podman);
    assert_eq!(plan.target_runtime, RuntimeKind::Kubernetes);
    assert_eq!(plan.strategy, MigrationStrategy::BlueGreen);
    assert!(plan.rollback_on_failure);
}
