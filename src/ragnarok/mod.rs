//! Ragnarok — confidential computing platform (attestation, measured images, sovereign runtime).
//!
//! Integrates with Aether for KubeVirt/Kata confidential workloads.

pub mod attestation;
pub mod client;
pub mod guestkit;
pub mod image;
pub mod isolation;
pub mod kata;
pub mod kubevirt;
pub mod migration;
pub mod network;
pub mod secrets;
pub mod sovereign;
pub mod tee;
pub mod trust;

pub use attestation::{AttestationService, AttestationVerdict, ExplainReport};
pub use client::{attestation_gate_for_workload, RagnarokClient};
pub use image::{ImageCatalog, ImageManifest};
pub use tee::{probe_host_tee, TeeCapabilities, TeeKind};
