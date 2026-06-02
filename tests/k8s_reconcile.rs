//! Integration tests for Kubernetes ancillary reconcile helpers.

use aether::adapters::kube_reconcile::managed_rbac_names;
use aether::spec::Workload;
use std::path::PathBuf;

fn load_example(name: &str) -> Workload {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name);
    let content = std::fs::read_to_string(&path).expect("read example spec");
    serde_yaml::from_str(&content).expect("parse example spec")
}

#[test]
fn managed_rbac_names_from_advanced_k8s_example() {
    let spec = load_example("workload-k8s-advanced.yaml");
    let (sa, role, binding) = managed_rbac_names(&spec);
    assert_eq!(sa.as_deref(), Some("api-sa"));
    assert!(role.is_none());
    assert!(binding.is_none());
}

#[test]
fn advanced_k8s_example_has_gateway_and_autoscaling_ancillaries() {
    let spec = load_example("workload-k8s-advanced.yaml");
    let k8s = spec.kubernetes.as_ref().expect("kubernetes block");
    assert!(k8s.gateway.as_ref().is_some_and(|g| g.provision_gateway));
    assert!(k8s
        .vertical_pod_autoscaler
        .as_ref()
        .is_some_and(|v| v.enabled));
    assert!(k8s.keda.as_ref().is_some_and(|k| k.enabled));
    assert!(k8s.cert_manager.as_ref().is_some_and(|c| c.enabled));
}
