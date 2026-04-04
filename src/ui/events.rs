//! Event handling for TUI

use super::app::{App, Screen};
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

pub async fn handle_events(app: &mut App) -> anyhow::Result<()> {
    if event::poll(Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            handle_key_event(app, key).await?;
        }
    }

    // Auto-refresh every 5 seconds
    if app.should_refresh() {
        app.refresh_workloads().await?;
    }

    Ok(())
}

async fn handle_key_event(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    let screen = app.screen.clone();
    match screen {
        Screen::Dashboard => handle_dashboard_keys(app, key).await,
        Screen::Logs(name) => handle_logs_keys(app, key, &name).await,
    }
}

async fn handle_dashboard_keys(app: &mut App, key: KeyEvent) -> anyhow::Result<()> {
    // Ctrl+C to quit
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            app.set_status_message("Refreshing...".to_string());
            app.refresh_workloads().await?;
            app.clear_status_message();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.select_next();
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.select_previous();
        }
        KeyCode::Home | KeyCode::Char('g') => {
            // Jump to first workload
            if !app.workloads.is_empty() {
                app.selected_index = 0;
            }
        }
        KeyCode::End | KeyCode::Char('G') => {
            // Jump to last workload
            if !app.workloads.is_empty() {
                app.selected_index = app.workloads.len().saturating_sub(1);
            }
        }
        KeyCode::Enter | KeyCode::Char('l') => {
            if let Some(workload) = app.selected_workload() {
                let name = workload.state.name.clone();
                app.screen = Screen::Logs(name.clone());
                app.load_logs(&name).await?;
            }
        }
        _ => {}
    }

    Ok(())
}

async fn handle_logs_keys(app: &mut App, key: KeyEvent, name: &str) -> anyhow::Result<()> {
    // Ctrl+C to quit
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        app.should_quit = true;
        return Ok(());
    }

    match key.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => {
            app.should_quit = true;
        }
        KeyCode::Esc | KeyCode::Backspace => {
            app.screen = Screen::Dashboard;
            app.logs_buffer.clear();
        }
        KeyCode::Char('r') | KeyCode::Char('R') => {
            // Reload logs
            app.load_logs(name).await?;
        }
        _ => {}
    }

    Ok(())
}
