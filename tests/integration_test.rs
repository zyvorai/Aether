//! Integration tests for Orchestr8

use orchestr8::{
    engine::Engine,
    runtime::RuntimeKind,
    spec::Workload,
    state::StateStore,
};
use std::collections::HashMap;
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

#[test]
fn test_backup_create_list_restore() {
    use orchestr8::backup::BackupManager;
    use orchestr8::runtime::Instance;
    use orchestr8::state::WorkloadState;

    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.json");
    let backup_dir = temp_dir.path().join("backups");

    // Create state with workloads
    let mut state = StateStore::new();
    state.upsert(
        "backup-test".to_string(),
        WorkloadState {
            name: "backup-test".to_string(),
            runtime: RuntimeKind::Podman,
            instance: Instance {
                id: "bkp-123".to_string(),
                name: "backup-test-container".to_string(),
                runtime: RuntimeKind::Podman,
                image: "test:latest".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            spec_path: PathBuf::from("/tmp/test.yaml"),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    );
    state.save(&state_path).unwrap();

    // Create backup
    let manager = BackupManager::new(backup_dir.clone());
    let backup_path = manager
        .create_backup(
            &state,
            Some("test-backup".to_string()),
            Some("Integration test backup".to_string()),
        )
        .unwrap();
    assert!(backup_path.exists());

    // List backups
    let backups = manager.list_backups().unwrap();
    assert_eq!(backups.len(), 1);

    // Get backup info
    let info = manager.get_backup_info(&backups[0]).unwrap();
    assert_eq!(info.workload_count, 1);
    assert_eq!(info.description, Some("Integration test backup".to_string()));

    // Restore to a new state path
    let restore_path = temp_dir.path().join("restored_state.json");
    let backup = orchestr8::backup::Backup::load(&backup_path).unwrap();
    backup.restore(&restore_path).unwrap();

    // Verify restored state
    let restored = StateStore::load(&restore_path).unwrap();
    assert_eq!(restored.list().len(), 1);
    let w = restored.get("backup-test").unwrap();
    assert_eq!(w.instance.id, "bkp-123");
}

#[test]
fn test_cost_estimation_for_workload() {
    use orchestr8::cost::{estimate_cost, estimate_all_providers, CloudProvider, CostComparison};

    let temp_dir = TempDir::new().unwrap();
    let (workload, _) = create_test_workload("cost-test", &temp_dir);

    // Test single provider estimate
    let estimate = estimate_cost(&workload, CloudProvider::AWS).unwrap();
    assert!(estimate.total_monthly > 0.0);
    assert!(estimate.total_hourly > 0.0);
    assert_eq!(estimate.provider, CloudProvider::AWS);

    // Test all-provider estimate
    let all_estimates = estimate_all_providers(&workload).unwrap();
    assert_eq!(all_estimates.len(), 5); // AWS, Azure, GCP, DigitalOcean, Linode

    // Test cost comparison
    let comparison = CostComparison::for_workload(&workload).unwrap();
    let display = comparison.display();
    assert!(display.contains("AWS"));
    assert!(display.contains("Azure"));
    assert!(display.contains("GCP"));
}

#[test]
fn test_backup_merge() {
    use orchestr8::backup::BackupManager;
    use orchestr8::runtime::Instance;
    use orchestr8::state::WorkloadState;

    let temp_dir = TempDir::new().unwrap();
    let state_path = temp_dir.path().join("state.json");
    let backup_dir = temp_dir.path().join("backups");

    // Create initial state with one workload
    let mut state = StateStore::new();
    state.upsert(
        "app-existing".to_string(),
        WorkloadState {
            name: "app-existing".to_string(),
            runtime: RuntimeKind::Kubernetes,
            instance: Instance {
                id: "existing-id".to_string(),
                name: "app-existing-pod".to_string(),
                runtime: RuntimeKind::Kubernetes,
                image: "existing:latest".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            spec_path: PathBuf::from("/tmp/existing.yaml"),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    );
    state.save(&state_path).unwrap();

    // Create a backup with a different workload
    let mut backup_state = StateStore::new();
    backup_state.upsert(
        "app-from-backup".to_string(),
        WorkloadState {
            name: "app-from-backup".to_string(),
            runtime: RuntimeKind::Podman,
            instance: Instance {
                id: "backup-id".to_string(),
                name: "app-from-backup-container".to_string(),
                runtime: RuntimeKind::Podman,
                image: "backup:latest".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
            },
            spec_path: PathBuf::from("/tmp/backup.yaml"),
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        },
    );

    let manager = BackupManager::new(backup_dir);
    let backup_path = manager
        .create_backup(&backup_state, Some("merge-test".to_string()), None)
        .unwrap();

    // Merge backup into existing state
    let backup = orchestr8::backup::Backup::load(&backup_path).unwrap();
    backup.merge(&state_path).unwrap();

    // Verify merged state has both workloads
    let merged = StateStore::load(&state_path).unwrap();
    assert_eq!(merged.list().len(), 2);
    assert!(merged.get("app-existing").is_some());
    assert!(merged.get("app-from-backup").is_some());
}

// ========== Secrets module tests ==========

#[test]
fn test_secrets_crud_and_encryption() {
    use orchestr8::secrets::SecretStore;

    let temp_dir = TempDir::new().unwrap();
    let store_path = temp_dir.path().join("secrets.json");

    let mut store = SecretStore::new();

    // Create secrets
    store.create_secret("db-creds", "production");
    store.create_secret("api-keys", "staging");

    // Set values
    store.set("db-creds", "username", "admin").unwrap();
    store.set("db-creds", "password", "s3cret!123").unwrap();
    store.set("api-keys", "stripe", "sk_test_abc123").unwrap();

    // Read back and verify decryption
    assert_eq!(store.get("db-creds", "username").unwrap(), "admin");
    assert_eq!(store.get("db-creds", "password").unwrap(), "s3cret!123");
    assert_eq!(store.get("api-keys", "stripe").unwrap(), "sk_test_abc123");

    // List
    let list = store.list();
    assert_eq!(list.len(), 2);

    // Delete a key
    store.delete_key("db-creds", "username").unwrap();
    assert!(store.get("db-creds", "username").is_err());
    assert_eq!(store.get("db-creds", "password").unwrap(), "s3cret!123");

    // Rotate
    store.rotate("db-creds", "password", "new_p@ss").unwrap();
    assert_eq!(store.get("db-creds", "password").unwrap(), "new_p@ss");

    // Verify version incremented
    let secret = store.get_secret("db-creds").unwrap();
    assert!(secret.data["password"].version >= 2);

    // Audit access log
    assert!(!secret.access_log.is_empty());

    // Save and reload
    store.save(&store_path).unwrap();
    let mut loaded = SecretStore::load(&store_path).unwrap();
    assert_eq!(loaded.list().len(), 2);
    assert_eq!(loaded.get("db-creds", "password").unwrap(), "new_p@ss");

    // Delete entire secret
    assert!(loaded.delete_secret("api-keys").is_some());
    assert_eq!(loaded.list().len(), 1);
}

// ========== Events module tests ==========

#[test]
fn test_events_emit_filter_acknowledge() {
    use orchestr8::events::{EventBus, EventCategory, EventSeverity};

    let temp_dir = TempDir::new().unwrap();
    let events_path = temp_dir.path().join("events.json");

    let mut bus = EventBus::new();

    // Emit events
    let id1 = bus.emit_simple(
        EventSeverity::Info,
        EventCategory::Deployment,
        "engine",
        Some("web-app"),
        "Deployed web-app",
        "Successfully deployed to kubernetes",
    );
    let id2 = bus.emit_simple(
        EventSeverity::Critical,
        EventCategory::SlaViolation,
        "sla-monitor",
        Some("api-server"),
        "SLA breach",
        "Uptime dropped below 99.9%",
    );
    bus.emit_simple(
        EventSeverity::Warning,
        EventCategory::DriftDetected,
        "drift-detector",
        None,
        "Config drift",
        "Runtime mismatch detected",
    );

    assert_eq!(bus.events().len(), 3);

    // Filter by category
    assert_eq!(bus.events_by_category(&EventCategory::Deployment).len(), 1);
    assert_eq!(bus.events_by_category(&EventCategory::SlaViolation).len(), 1);

    // Filter by severity
    assert_eq!(bus.events_by_severity(&EventSeverity::Critical).len(), 1);
    assert_eq!(bus.events_by_severity(&EventSeverity::Warning).len(), 2); // Warning + Critical

    // Filter by workload
    assert_eq!(bus.events_for_workload("web-app").len(), 1);
    assert_eq!(bus.events_for_workload("api-server").len(), 1);

    // Acknowledge
    assert_eq!(bus.unacknowledged().len(), 3);
    assert!(bus.acknowledge(id1));
    assert_eq!(bus.unacknowledged().len(), 2);
    assert!(bus.acknowledge(id2));

    // Summary
    let summary = bus.summary();
    assert_eq!(summary.total_events, 3);
    assert_eq!(summary.critical_unacked, 0);

    // Prune
    bus.prune(2);
    assert_eq!(bus.events().len(), 2);

    // Save and reload
    bus.save(&events_path).unwrap();
    let loaded = EventBus::load(&events_path).unwrap();
    assert_eq!(loaded.events().len(), 2);
}

// ========== Scheduler module tests ==========

#[test]
fn test_scheduler_placement_and_release() {
    use orchestr8::scheduler::{
        Priority, ScheduleConstraint, ScheduleRequest, ScheduleStrategy, Scheduler,
    };

    let mut scheduler = Scheduler::new();

    // Basic placement
    let req = ScheduleRequest {
        workload_name: "frontend".to_string(),
        cpu_required: 2.0,
        memory_required_mb: 2048,
        storage_required_mb: 10240,
        gpu_required: 0,
        preferred_runtime: None,
        constraints: vec![],
        priority: Priority::Normal,
    };
    let decision = scheduler.schedule(&req).unwrap();
    assert!(!decision.workload_name.is_empty());
    assert!(decision.score > 0.0);
    assert!(decision.estimated_cost_per_day > 0.0);
    assert_eq!(scheduler.placements().len(), 1);

    // Place with constraint
    let req2 = ScheduleRequest {
        workload_name: "backend".to_string(),
        cpu_required: 4.0,
        memory_required_mb: 4096,
        storage_required_mb: 20480,
        gpu_required: 0,
        preferred_runtime: None,
        constraints: vec![ScheduleConstraint::RequireRuntime(RuntimeKind::Kubernetes)],
        priority: Priority::High,
    };
    let decision2 = scheduler.schedule(&req2).unwrap();
    assert_eq!(decision2.selected_runtime, RuntimeKind::Kubernetes);
    assert_eq!(scheduler.placements().len(), 2);

    // Co-location
    let req3 = ScheduleRequest {
        workload_name: "cache".to_string(),
        cpu_required: 1.0,
        memory_required_mb: 1024,
        storage_required_mb: 5120,
        gpu_required: 0,
        preferred_runtime: None,
        constraints: vec![ScheduleConstraint::CoLocate("backend".to_string())],
        priority: Priority::Normal,
    };
    let decision3 = scheduler.schedule(&req3).unwrap();
    assert_eq!(decision3.selected_runtime, RuntimeKind::Kubernetes);

    // Utilization summary
    let utils = scheduler.utilization_summary();
    assert_eq!(utils.len(), 4);

    // Optimization suggestions (may or may not have suggestions depending on state)
    let _suggestions = scheduler.optimize();

    // Release and verify capacity restore
    scheduler.release("frontend");
    assert_eq!(scheduler.placements().len(), 2);

    // Cost-optimized strategy
    let mut cost_scheduler = Scheduler::with_strategy(ScheduleStrategy::CostOptimized);
    let cheap_req = ScheduleRequest {
        workload_name: "worker".to_string(),
        cpu_required: 1.0,
        memory_required_mb: 512,
        storage_required_mb: 1024,
        gpu_required: 0,
        preferred_runtime: None,
        constraints: vec![],
        priority: Priority::Low,
    };
    let cost_decision = cost_scheduler.schedule(&cheap_req).unwrap();
    assert_eq!(cost_decision.selected_runtime, RuntimeKind::Podman);

    // Infeasible request
    let huge_req = ScheduleRequest {
        workload_name: "giant".to_string(),
        cpu_required: 99999.0,
        memory_required_mb: 99999999,
        storage_required_mb: 99999999,
        gpu_required: 0,
        preferred_runtime: None,
        constraints: vec![],
        priority: Priority::Normal,
    };
    assert!(scheduler.schedule(&huge_req).is_err());
}

// ========== Orchestrator module tests ==========

#[test]
fn test_orchestrator_health_lifecycle() {
    use orchestr8::orchestrator::{
        CheckResult, CircuitState, HealthCheck, HealthConfig, HealthStatus, Orchestrator,
        OrchestratorAction, UpdatePhase,
    };

    let mut orch = Orchestrator::new();

    // Register workloads
    orch.register("web-app", RuntimeKind::Kubernetes, None);
    let config = HealthConfig {
        failure_threshold: 2,
        success_threshold: 2,
        max_restarts: 3,
        ..Default::default()
    };
    orch.register("api-server", RuntimeKind::Kubernetes, Some(config));

    assert_eq!(orch.list_workloads().len(), 2);

    // Healthy checks -> status becomes Healthy
    for _ in 0..3 {
        orch.process_health_check(HealthCheck {
            workload: "web-app".to_string(),
            status: HealthStatus::Healthy,
            checks: vec![CheckResult {
                name: "http".to_string(),
                passed: true,
                message: "OK".to_string(),
                latency_ms: Some(25.0),
            }],
            timestamp: chrono::Utc::now().to_rfc3339(),
            consecutive_failures: 0,
        });
    }
    assert_eq!(
        orch.get_workload("web-app").unwrap().current_health,
        HealthStatus::Healthy
    );

    // Failing checks -> triggers restarts
    let mut restart_count = 0;
    for _ in 0..6 {
        let actions = orch.process_health_check(HealthCheck {
            workload: "api-server".to_string(),
            status: HealthStatus::Unhealthy,
            checks: vec![CheckResult {
                name: "http".to_string(),
                passed: false,
                message: "Connection refused".to_string(),
                latency_ms: None,
            }],
            timestamp: chrono::Utc::now().to_rfc3339(),
            consecutive_failures: 0,
        });
        restart_count += actions
            .iter()
            .filter(|a| matches!(a, OrchestratorAction::Restart { .. }))
            .count();
    }
    assert!(restart_count > 0);

    // Eventually circuit should open
    let w = orch.get_workload("api-server").unwrap();
    assert!(w.restart_count > 0 || w.circuit == CircuitState::Open);

    // Health summary
    let summary = orch.health_summary();
    assert_eq!(summary.total_workloads, 2);
    assert!(summary.healthy >= 1);

    // Manual circuit reset
    orch.reset_circuit("api-server");
    assert_eq!(
        orch.get_workload("api-server").unwrap().circuit,
        CircuitState::Closed
    );

    // Rolling update simulation
    let statuses = orch.rolling_update("web-app", 5, None);
    assert!(!statuses.is_empty());
    assert_eq!(statuses.last().unwrap().phase, UpdatePhase::Completed);
    assert_eq!(statuses.last().unwrap().total_replicas, 5);

    // Unregister
    assert!(orch.unregister("web-app").is_some());
    assert_eq!(orch.list_workloads().len(), 1);
}

// ========== Environments module tests ==========

#[test]
fn test_environment_promotion_and_parity() {
    use orchestr8::environments::{
        EnvTier, EnvWorkload, EnvironmentManager, PromotionRequest, PromotionStrategy,
    };

    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join("environments.json");

    let mut mgr = EnvironmentManager::new();

    // Create environments
    mgr.create_env("dev", EnvTier::Development);
    mgr.create_env("staging", EnvTier::Staging);
    mgr.create_env("prod", EnvTier::Production);

    assert_eq!(mgr.list_envs().len(), 3);

    // Add workloads to dev
    let web = EnvWorkload {
        name: "web-app".to_string(),
        spec_path: PathBuf::from("web-app.yaml"),
        runtime_override: None,
        replicas: Some(1),
        cpu_override: Some("1".to_string()),
        memory_override: Some("512Mi".to_string()),
        env_vars: HashMap::from([
            ("NODE_ENV".to_string(), "development".to_string()),
            ("LOG_LEVEL".to_string(), "debug".to_string()),
        ]),
        deployed: true,
        version: "1.0.0".to_string(),
    };
    mgr.add_workload("dev", web).unwrap();

    let api = EnvWorkload {
        name: "api-server".to_string(),
        spec_path: PathBuf::from("api.yaml"),
        runtime_override: None,
        replicas: Some(1),
        cpu_override: None,
        memory_override: None,
        env_vars: HashMap::new(),
        deployed: true,
        version: "2.0.0".to_string(),
    };
    mgr.add_workload("dev", api).unwrap();

    assert_eq!(mgr.get_env("dev").unwrap().workloads.len(), 2);

    // Promote web-app from dev to staging (direct)
    let result = mgr
        .promote(&PromotionRequest {
            workload: "web-app".to_string(),
            from_env: "dev".to_string(),
            to_env: "staging".to_string(),
            strategy: PromotionStrategy::Direct,
            require_approval: false,
        })
        .unwrap();
    assert!(result.success);
    assert!(mgr.get_env("staging").unwrap().workloads.contains_key("web-app"));

    // Promote web-app from staging to prod (tier-adjusted)
    let result = mgr
        .promote(&PromotionRequest {
            workload: "web-app".to_string(),
            from_env: "staging".to_string(),
            to_env: "prod".to_string(),
            strategy: PromotionStrategy::TierAdjusted,
            require_approval: false,
        })
        .unwrap();
    assert!(result.success);
    // Replicas should be scaled up for production
    let prod_wl = &mgr.get_env("prod").unwrap().workloads["web-app"];
    assert!(prod_wl.replicas.unwrap_or(0) >= 2);

    // Check parity between staging and prod
    let parity = mgr.check_parity("staging", "prod", "web-app").unwrap();
    assert!(!parity.in_sync); // Different replicas, version
    assert!(!parity.diffs.is_empty());

    // Check parity for missing workload
    let parity2 = mgr.check_parity("dev", "prod", "api-server").unwrap();
    assert!(!parity2.in_sync);

    // Set environment variables
    mgr.set_var("dev", "DATABASE_URL", "localhost:5432").unwrap();
    assert_eq!(
        mgr.get_env("dev").unwrap().variables["DATABASE_URL"],
        "localhost:5432"
    );

    // Save and reload
    mgr.save(&env_path).unwrap();
    let loaded = EnvironmentManager::load(&env_path).unwrap();
    assert_eq!(loaded.list_envs().len(), 3);
    assert!(loaded.get_env("prod").unwrap().workloads.contains_key("web-app"));
}

// ========== Affinity module tests ==========

#[test]
fn test_affinity_learning_and_recommendation() {
    use orchestr8::ai::affinity::{AffinityEngine, DeploymentOutcome, WorkloadClass};

    let temp_dir = TempDir::new().unwrap();
    let affinity_path = temp_dir.path().join("affinity.json");

    let mut engine = AffinityEngine::new();

    // Record successful Kubernetes web service deployments
    for i in 0..10 {
        engine.record(DeploymentOutcome {
            workload_name: format!("web-{}", i),
            workload_class: WorkloadClass::WebService,
            runtime: RuntimeKind::Kubernetes,
            success: true,
            uptime_pct: Some(99.9),
            avg_latency_ms: Some(45.0),
            error_rate_pct: Some(0.1),
            restarts: 0,
            cost_per_day: Some(12.0),
            timestamp: chrono::Utc::now().to_rfc3339(),
            failure_reason: None,
        });
    }

    // Record some Podman failures for databases
    for i in 0..3 {
        engine.record(DeploymentOutcome {
            workload_name: format!("db-{}", i),
            workload_class: WorkloadClass::Database,
            runtime: RuntimeKind::Podman,
            success: false,
            uptime_pct: Some(50.0),
            avg_latency_ms: Some(500.0),
            error_rate_pct: Some(10.0),
            restarts: 5,
            cost_per_day: Some(5.0),
            timestamp: chrono::Utc::now().to_rfc3339(),
            failure_reason: Some("Persistent storage issues".to_string()),
        });
    }

    // Record successful Kubernetes database deployments
    for i in 0..5 {
        engine.record(DeploymentOutcome {
            workload_name: format!("db-k8s-{}", i),
            workload_class: WorkloadClass::Database,
            runtime: RuntimeKind::Kubernetes,
            success: true,
            uptime_pct: Some(99.5),
            avg_latency_ms: Some(10.0),
            error_rate_pct: Some(0.2),
            restarts: 0,
            cost_per_day: Some(20.0),
            timestamp: chrono::Utc::now().to_rfc3339(),
            failure_reason: None,
        });
    }

    // Recommendations: Kubernetes should rank highest for web services
    let web_recs = engine.recommend(&WorkloadClass::WebService);
    assert_eq!(web_recs.len(), 4);
    assert_eq!(web_recs[0].runtime, RuntimeKind::Kubernetes);
    assert!(web_recs[0].confidence > 0.0);

    // Recommendations: Kubernetes should rank above Podman for databases
    let db_recs = engine.recommend(&WorkloadClass::Database);
    let k8s_pos = db_recs.iter().position(|r| r.runtime == RuntimeKind::Kubernetes).unwrap();
    let podman_pos = db_recs.iter().position(|r| r.runtime == RuntimeKind::Podman).unwrap();
    assert!(k8s_pos < podman_pos);

    // Incompatibilities tracked
    let incompat = engine.incompatibilities();
    assert_eq!(incompat.len(), 1);
    assert_eq!(incompat[0].workload_class, WorkloadClass::Database);
    assert_eq!(incompat[0].runtime, RuntimeKind::Podman);
    assert_eq!(incompat[0].failure_count, 3);

    // Compatibility matrix
    let matrix = engine.compatibility_matrix();
    assert_eq!(matrix.len(), 32); // 8 classes x 4 runtimes

    // Learning stats
    let stats = engine.stats();
    assert_eq!(stats.total_outcomes, 18);
    assert_eq!(stats.successes, 15);
    assert_eq!(stats.failures, 3);

    // Heuristic-only recommendations (no data for ML Training)
    let ml_recs = engine.recommend(&WorkloadClass::MlTraining);
    assert!(ml_recs.iter().all(|r| r.confidence == 0.0));

    // Save and reload
    engine.save(&affinity_path).unwrap();
    let loaded = AffinityEngine::load(&affinity_path).unwrap();
    assert_eq!(loaded.stats().total_outcomes, 18);
    let loaded_recs = loaded.recommend(&WorkloadClass::WebService);
    assert_eq!(loaded_recs[0].runtime, RuntimeKind::Kubernetes);
}

// ========== Scheduler persistence tests ==========

#[test]
fn test_scheduler_save_load() {
    use orchestr8::scheduler::{Priority, ScheduleRequest, Scheduler};

    let temp_dir = TempDir::new().unwrap();
    let sched_path = temp_dir.path().join("scheduler.json");

    let mut scheduler = Scheduler::new();
    let req = ScheduleRequest {
        workload_name: "persist-test".to_string(),
        cpu_required: 2.0,
        memory_required_mb: 2048,
        storage_required_mb: 10240,
        gpu_required: 0,
        preferred_runtime: None,
        constraints: vec![],
        priority: Priority::Normal,
    };
    scheduler.schedule(&req).unwrap();

    scheduler.save(&sched_path).unwrap();
    let loaded = Scheduler::load(&sched_path).unwrap();
    assert_eq!(loaded.placements().len(), 1);
    assert_eq!(loaded.placements()[0].workload_name, "persist-test");
}

// ========== Orchestrator persistence tests ==========

#[test]
fn test_orchestrator_save_load() {
    use orchestr8::orchestrator::Orchestrator;

    let temp_dir = TempDir::new().unwrap();
    let orch_path = temp_dir.path().join("orchestrator.json");

    let mut orch = Orchestrator::new();
    orch.register("app-1", RuntimeKind::Kubernetes, None);
    orch.register("app-2", RuntimeKind::Podman, None);

    orch.save(&orch_path).unwrap();
    let loaded = Orchestrator::load(&orch_path).unwrap();
    assert_eq!(loaded.list_workloads().len(), 2);
}
