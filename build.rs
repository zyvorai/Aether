// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Embed dashboard build stamp when `AETHER_DASHBOARD_BUILD` is set during `cargo build`.

fn main() {
    let build = std::env::var("AETHER_DASHBOARD_BUILD").unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=AETHER_EMBEDDED_UI_BUILD={build}");
    println!("cargo:rerun-if-env-changed=AETHER_DASHBOARD_BUILD");
    println!("cargo:rerun-if-changed=web/dashboard/dist/index.html");

    println!("cargo:rerun-if-changed=build.rs");
}
