//! Runtime adapters module

pub mod common;
pub mod docker;
pub mod podman;
pub mod kube;
pub mod kubevirt;
pub mod metal;

pub use docker::DockerRuntime;
pub use podman::PodmanRuntime;
pub use kube::KubernetesRuntime;
pub use kubevirt::KubeVirtRuntime;
pub use metal::Metal3Runtime;

/// Implement `new()`, `with_namespace()`, and `with_context()` for a kube-client-based adapter.
///
/// Eliminates the identical constructor boilerplate across `KubernetesRuntime`,
/// `KubeVirtRuntime`, and `Metal3Runtime`.
macro_rules! impl_kube_adapter_new {
    ($ty:ty, $default_ns:expr) => {
        impl $ty {
            /// Create a new runtime, reading the namespace from
            /// `AETHER_NAMESPACE` (falling back to the adapter default).
            pub async fn new() -> anyhow::Result<Self> {
                let client = ::kube::Client::try_default().await?;
                let namespace = std::env::var("AETHER_NAMESPACE")
                    .unwrap_or_else(|_| $default_ns.to_string());
                Ok(Self { client, namespace })
            }

            /// Create a new runtime with an explicit namespace.
            pub async fn with_namespace(namespace: String) -> anyhow::Result<Self> {
                let client = ::kube::Client::try_default().await?;
                Ok(Self { client, namespace })
            }

            /// Create a new runtime targeting a specific kubeconfig context (multi-cluster).
            pub async fn with_context(context: &str, namespace: Option<String>) -> anyhow::Result<Self> {
                let kubeconfig = ::kube::config::Kubeconfig::read()?;
                let config = ::kube::Config::from_custom_kubeconfig(
                    kubeconfig,
                    &::kube::config::KubeConfigOptions {
                        context: Some(context.to_string()),
                        ..Default::default()
                    },
                ).await?;
                let client = ::kube::Client::try_from(config)?;
                let namespace = namespace.unwrap_or_else(|| {
                    std::env::var("AETHER_NAMESPACE")
                        .unwrap_or_else(|_| $default_ns.to_string())
                });
                Ok(Self { client, namespace })
            }
        }
    };
}

pub(crate) use impl_kube_adapter_new;
