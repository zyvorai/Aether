// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Embed dashboard build stamp when `AETHER_DASHBOARD_BUILD` is set during `cargo build`.

fn main() {
    let build = std::env::var("AETHER_DASHBOARD_BUILD").unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=AETHER_EMBEDDED_UI_BUILD={build}");
    println!("cargo:rerun-if-env-changed=AETHER_DASHBOARD_BUILD");
    println!("cargo:rerun-if-changed=web/dashboard/dist/index.html");

    // Bake the build date into the binary for 30-day trial enforcement.
    let output = std::process::Command::new("sh")
        .args(["-c", "date +%Y-%m-%d"])
        .output()
        .expect("failed to get build date");
    let date = String::from_utf8(output.stdout).expect("date output is not UTF-8");
    println!("cargo:rustc-env=AETHER_BUILD_DATE={}", date.trim());
    println!("cargo:rerun-if-changed=build.rs");
}
