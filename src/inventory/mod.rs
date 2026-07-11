// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Application-level inventory: grouping, dependencies, and persisted snapshots.

pub mod application;
pub mod dependency;
pub mod snapshot;

pub use application::{group_into_applications, Application, MigrationClass};
pub use dependency::{discover_dependencies, DependencyEdges};
pub use snapshot::InventorySnapshot;
