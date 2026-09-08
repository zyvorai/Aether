// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

pub mod adapters;
pub mod registry;

pub use registry::{
    build_provider_status, delete_provider, list_providers_public, load_registry,
    provider_from_registry, save_registry, test_provider, upsert_provider, ZyraProviderConfig,
    ZyraProviderKind, ZyraProviderRegistry, ZyraProviderStatusReport,
};
