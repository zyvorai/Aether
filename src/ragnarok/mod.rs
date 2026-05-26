//! Ragnarok — confidential computing platform (attestation, measured images, sovereign runtime).
//!
//! Integrates with Aether for KubeVirt/Kata confidential workloads.

pub mod attestation;
pub mod client;
pub mod envelope;
pub mod guestkit;
pub mod image;
pub mod inject;
pub mod intelligence;
pub mod isolation;
pub mod kata;
pub mod kubevirt;
pub mod kbs;
pub mod migration;
pub mod network;
pub mod scheduling;
pub mod secrets;
pub mod sovereign;
pub mod tee;
pub mod trust;
pub mod vault;

pub use attestation::{AttestationService, AttestationVerdict, ExplainReport};
pub use client::{attestation_gate_for_workload, RagnarokClient};
pub use image::{ImageCatalog, ImageManifest};
pub use isolation::{IsolationPolicy, IsolationVerdict};
pub use tee::{probe_host_tee, TeeCapabilities, TeeKind};
