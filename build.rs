// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Ensure dashboard assets exist for `include_str!` embedding, and stamp the UI build id.
//!
//! Prefer a real `npm run build` output under `web/dashboard/dist/`. When those files are
//! missing (fresh clone without a dashboard build), write minimal stubs so `cargo build`
//! and unit tests still compile. Production images / `make ci` run a real dashboard build.

use std::fs;
use std::path::Path;
use std::process::Command;

fn main() {
    let build = std::env::var("AETHER_DASHBOARD_BUILD").unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=AETHER_EMBEDDED_UI_BUILD={build}");
    println!("cargo:rerun-if-env-changed=AETHER_DASHBOARD_BUILD");
    println!("cargo:rerun-if-changed=web/dashboard/dist/index.html");
    println!("cargo:rerun-if-changed=web/dashboard/dist/assets/aether-dashboard.css");
    println!("cargo:rerun-if-changed=web/dashboard/dist/assets/aether-dashboard.js");
    println!("cargo:rerun-if-changed=build.rs");

    ensure_dashboard_assets();
}

fn ensure_dashboard_assets() {
    let index = Path::new("web/dashboard/dist/index.html");
    let css = Path::new("web/dashboard/dist/assets/aether-dashboard.css");
    let js = Path::new("web/dashboard/dist/assets/aether-dashboard.js");

    if index.is_file() && css.is_file() && js.is_file() {
        return;
    }

    // Attempt a real dashboard build when node/npm are available.
    if Path::new("web/dashboard/package.json").is_file() {
        let status = Command::new("npm")
            .args(["run", "build", "--silent"])
            .current_dir("web/dashboard")
            .status();
        if matches!(status, Ok(s) if s.success())
            && index.is_file()
            && css.is_file()
            && js.is_file()
        {
            return;
        }
    }

    write_dashboard_stubs();
}

fn write_dashboard_stubs() {
    let assets = Path::new("web/dashboard/dist/assets");
    if let Err(err) = fs::create_dir_all(assets) {
        panic!("failed to create web/dashboard/dist/assets: {err}");
    }

    let index = r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0" />
  <title>Aether</title>
  <link rel="stylesheet" href="/assets/aether-dashboard.css" />
</head>
<body>
  <div id="root">
    <p>Dashboard assets were not built. Run <code>cd web/dashboard &amp;&amp; npm run build</code>.</p>
  </div>
  <script type="module" src="/assets/aether-dashboard.js"></script>
</body>
</html>
"#;
    let css = "/* stub dashboard css — run npm run build in web/dashboard */\nbody{font-family:sans-serif;padding:2rem;}\n";
    let js = "console.warn('[aether] stub dashboard js — run npm run build in web/dashboard');\n";

    fs::write("web/dashboard/dist/index.html", index)
        .unwrap_or_else(|e| panic!("write stub index.html: {e}"));
    fs::write("web/dashboard/dist/assets/aether-dashboard.css", css)
        .unwrap_or_else(|e| panic!("write stub css: {e}"));
    fs::write("web/dashboard/dist/assets/aether-dashboard.js", js)
        .unwrap_or_else(|e| panic!("write stub js: {e}"));
}
