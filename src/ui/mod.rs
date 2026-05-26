// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! TUI module for interactive dashboard

pub mod app;
pub mod components;
pub mod dashboard;
pub mod events;
pub mod logs;

pub use app::{App, AppState, Screen};
pub use dashboard::render_dashboard;
pub use events::handle_events;
pub use logs::render_logs;
