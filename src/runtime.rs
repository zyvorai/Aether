//! Runtime trait and common types
//!
//! All runtime adapters (Podman, Kubernetes, KubeVirt, Metal3)
//! must implement this trait.

use crate::spec::Workload;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Universal runtime trait
#[async_trait]
pub trait Runtime: Send + Sync {
    /// Build an image from the workload spec
    async fn build(&self, spec: &Workload) -> crate::Result<Image>;

    /// Run a workload instance
    async fn run(&self, image: &Image, spec: &Workload) -> crate::Result<Instance>;

    /// Stop a running instance
    async fn stop(&self, instance: &Instance) -> crate::Result<()>;

    /// Get status of an instance
    async fn status(&self, instance: &Instance) -> crate::Result<Status>;

    /// Get logs from an instance
    async fn logs(&self, instance: &Instance, follow: bool) -> crate::Result<String>;

    /// Delete an instance
    async fn delete(&self, instance: &Instance) -> crate::Result<()>;

    /// List all instances managed by this runtime
    async fn list(&self) -> crate::Result<Vec<Instance>>;
}

/// Built image reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Image {
    pub name: String,
    pub tag: String,
    pub digest: Option<String>,
    pub runtime: RuntimeKind,
}

impl Image {
    pub fn full_name(&self) -> String {
        format!("{}:{}", self.name, self.tag)
    }
}

/// Running instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub runtime: RuntimeKind,
    pub image: String,
    pub created_at: String,
}

/// Instance status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Status {
    pub state: InstanceState,
    pub ready: bool,
    pub message: Option<String>,
    pub restart_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum InstanceState {
    Pending,
    Running,
    Stopped,
    Failed,
    Unknown,
}

impl fmt::Display for InstanceState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InstanceState::Pending => write!(f, "pending"),
            InstanceState::Running => write!(f, "running"),
            InstanceState::Stopped => write!(f, "stopped"),
            InstanceState::Failed => write!(f, "failed"),
            InstanceState::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RuntimeKind {
    Podman,
    Kubernetes,
    KubeVirt,
    Metal3,
}

impl fmt::Display for RuntimeKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            RuntimeKind::Podman => write!(f, "podman"),
            RuntimeKind::Kubernetes => write!(f, "kubernetes"),
            RuntimeKind::KubeVirt => write!(f, "kubevirt"),
            RuntimeKind::Metal3 => write!(f, "metal3"),
        }
    }
}
