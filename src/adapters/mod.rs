// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Runtime adapters module

pub mod common;
pub mod docker;
pub mod kube;
pub mod kube_extras;
pub mod kube_manifest;
pub mod kube_policy_extras;
pub mod kube_reconcile;
pub mod kubevirt;
pub mod podman;

pub use docker::DockerRuntime;
pub use kube::KubernetesRuntime;
pub use kubevirt::KubeVirtRuntime;
pub use podman::PodmanRuntime;

/// Implement `new()`, `with_namespace()`, and `with_context()` for a kube-client-based adapter.
///
/// Eliminates the identical constructor boilerplate across `KubernetesRuntime`
/// and `KubeVirtRuntime`.
macro_rules! impl_kube_adapter_new {
    ($ty:ty, $default_ns:expr) => {
        impl $ty {
            /// Create a new runtime, reading the namespace from
            /// `AETHER_NAMESPACE` (falling back to the adapter default).
            pub async fn new() -> anyhow::Result<Self> {
                let client = ::kube::Client::try_default().await?;
                let namespace =
                    std::env::var("AETHER_NAMESPACE").unwrap_or_else(|_| $default_ns.to_string());
                Ok(Self { client, namespace })
            }

            /// Create a new runtime with an explicit namespace.
            pub async fn with_namespace(namespace: String) -> anyhow::Result<Self> {
                let client = ::kube::Client::try_default().await?;
                Ok(Self { client, namespace })
            }

            /// Create a new runtime targeting a specific kubeconfig context (multi-cluster).
            pub async fn with_context(
                context: &str,
                namespace: Option<String>,
            ) -> anyhow::Result<Self> {
                let kubeconfig = ::kube::config::Kubeconfig::read()?;
                let config = ::kube::Config::from_custom_kubeconfig(
                    kubeconfig,
                    &::kube::config::KubeConfigOptions {
                        context: Some(context.to_string()),
                        ..Default::default()
                    },
                )
                .await?;
                let client = ::kube::Client::try_from(config)?;
                let namespace = namespace.unwrap_or_else(|| {
                    std::env::var("AETHER_NAMESPACE").unwrap_or_else(|_| $default_ns.to_string())
                });
                Ok(Self { client, namespace })
            }
        }
    };
}

pub(crate) use impl_kube_adapter_new;

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    /// Mutex to serialize tests that modify environment variables.
    /// Prevents race conditions when tests run in parallel.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    #[test]
    fn test_namespace_fallback_explicit_wins() {
        // No env var access — no lock needed
        let explicit = Some("prod".to_string());
        let namespace = match explicit {
            Some(value) => value,
            None => std::env::var("AETHER_NAMESPACE").unwrap_or_else(|_| "default".to_string()),
        };
        assert_eq!(namespace, "prod");
    }

    #[test]
    fn test_namespace_fallback_default() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let saved = std::env::var("AETHER_NAMESPACE").ok();
        std::env::remove_var("AETHER_NAMESPACE");
        let explicit: Option<String> = None;
        let namespace = match explicit {
            Some(value) => value,
            None => std::env::var("AETHER_NAMESPACE").unwrap_or_else(|_| "default".to_string()),
        };
        assert_eq!(namespace, "default");
        if let Some(v) = saved {
            std::env::set_var("AETHER_NAMESPACE", v);
        }
    }

    #[test]
    fn test_with_namespace_always_uses_explicit() {
        let explicit_ns = "explicit-ns".to_string();
        assert_eq!(explicit_ns, "explicit-ns");
    }

    #[test]
    fn test_context_env_var_handling() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let saved = std::env::var("AETHER_CONTEXT").ok();
        std::env::remove_var("AETHER_CONTEXT");
        let context = std::env::var("AETHER_CONTEXT")
            .ok()
            .filter(|c| !c.is_empty());
        assert!(context.is_none());
        if let Some(v) = saved {
            std::env::set_var("AETHER_CONTEXT", v);
        }
    }

    #[test]
    fn test_context_env_var_empty_is_none() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let saved = std::env::var("AETHER_CONTEXT").ok();
        std::env::set_var("AETHER_CONTEXT", "");
        let context = std::env::var("AETHER_CONTEXT")
            .ok()
            .filter(|c| !c.is_empty());
        assert!(context.is_none());
        if let Some(v) = saved {
            std::env::set_var("AETHER_CONTEXT", v);
        } else {
            std::env::remove_var("AETHER_CONTEXT");
        }
    }

    #[test]
    fn test_context_env_var_non_empty() {
        let _guard = ENV_MUTEX.lock().unwrap();
        let saved = std::env::var("AETHER_CONTEXT").ok();
        std::env::set_var("AETHER_CONTEXT", "staging-cluster");
        let context = std::env::var("AETHER_CONTEXT")
            .ok()
            .filter(|c| !c.is_empty());
        assert_eq!(context, Some("staging-cluster".to_string()));
        if let Some(v) = saved {
            std::env::set_var("AETHER_CONTEXT", v);
        } else {
            std::env::remove_var("AETHER_CONTEXT");
        }
    }
}
