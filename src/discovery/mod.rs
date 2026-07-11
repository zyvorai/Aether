// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Cloud Kubernetes discovery: saved connections and metadata-preserving reads.

pub mod connection;
pub mod kubernetes;

pub use connection::{Connection, ConnectionKind, ConnectionStore};
pub use kubernetes::{discover, RawInventory};
