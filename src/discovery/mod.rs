// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Cloud Kubernetes discovery: saved connections and metadata-preserving reads.

pub mod connection;
pub mod kubernetes;

pub use connection::{Connection, ConnectionKind, ConnectionStore};
pub use kubernetes::{discover, RawInventory};
