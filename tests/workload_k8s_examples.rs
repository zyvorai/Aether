//! Parse and validate Kubernetes example workload specs.

use aether::spec::Workload;
use std::path::PathBuf;

fn example_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join(name)
}

fn load_example(name: &str) -> Workload {
    let path = example_path(name);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_yaml::from_str(&content)
        .unwrap_or_else(|e| panic!("parse {}: {e}", path.display()))
}

#[test]
fn workload_full_featured_parses_and_validates() {
    let spec = load_example("workload-full-featured.yaml");
    spec.validate()
        .expect("workload-full-featured.yaml should validate");
}

#[test]
fn workload_k8s_advanced_parses_and_validates() {
    let spec = load_example("workload-k8s-advanced.yaml");
    assert!(spec.kubernetes.is_some(), "kubernetes block required");
    let k8s = spec.kubernetes.as_ref().unwrap();
    assert!(k8s.gateway.as_ref().is_some_and(|g| g.provision_gateway));
    assert!(k8s.vertical_pod_autoscaler.as_ref().is_some_and(|v| v.enabled));
    assert!(k8s.keda.as_ref().is_some_and(|k| k.enabled));
    spec.validate()
        .expect("workload-k8s-advanced.yaml should validate");
}

#[test]
fn labs_kubernetes_workload_parses_and_validates() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples/labs/kubernetes/workload.yaml");
    let content = std::fs::read_to_string(&path).expect("read labs kubernetes spec");
    let spec: Workload = serde_yaml::from_str(&content).expect("parse labs kubernetes spec");
    spec.validate().expect("labs kubernetes spec should validate");
}
