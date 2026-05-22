//! Embed dashboard build stamp when `AETHER_DASHBOARD_BUILD` is set during `cargo build`.

fn main() {
    let build = std::env::var("AETHER_DASHBOARD_BUILD").unwrap_or_else(|_| "dev".to_string());
    println!("cargo:rustc-env=AETHER_EMBEDDED_UI_BUILD={build}");
    println!("cargo:rerun-if-env-changed=AETHER_DASHBOARD_BUILD");
    println!("cargo:rerun-if-changed=web/dashboard/dist/index.html");
}
