// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

pub mod adapters;
pub mod registry;

pub use registry::{
    build_provider_status, delete_provider, list_providers_public, load_registry,
    provider_from_registry, save_registry, test_provider, upsert_provider, ZeusProviderConfig,
    ZeusProviderKind, ZeusProviderRegistry, ZeusProviderStatusReport,
};
