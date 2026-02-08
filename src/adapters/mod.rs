//! Runtime adapters module

pub mod common;
pub mod podman;
pub mod kube;
pub mod kubevirt;
pub mod metal;

pub use podman::PodmanRuntime;
pub use kube::KubernetesRuntime;
pub use kubevirt::KubeVirtRuntime;
pub use metal::Metal3Runtime;
