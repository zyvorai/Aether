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
