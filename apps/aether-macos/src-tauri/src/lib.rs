// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager, RunEvent,
};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

#[derive(serde::Deserialize)]
struct TrayHealthPayload {
    fleet_health_pct: f64,
    issues: u32,
    #[serde(default)]
    sparkline: Option<String>,
}

#[derive(serde::Deserialize)]
struct CriticalAlertPayload {
    title: String,
    detail: String,
    count: u32,
}

#[derive(serde::Deserialize)]
struct DockBadgePayload {
    issue_count: u32,
}

#[derive(serde::Deserialize)]
struct NativeNotificationPayload {
    title: String,
    body: String,
}

#[derive(serde::Deserialize)]
struct LiveActivityPayload {
    workload: String,
    progress_pct: u32,
    detail: String,
}

#[tauri::command]
fn tray_health_update(app: tauri::AppHandle, payload: TrayHealthPayload) {
    update_tray_tooltip(
        &app,
        payload.fleet_health_pct,
        payload.issues,
        payload.sparkline.as_deref(),
    );
}

#[tauri::command]
fn tray_sparkline_update(app: tauri::AppHandle, sparkline: String, health: f64) {
    update_tray_tooltip(&app, health, 0, Some(&sparkline));
}

#[tauri::command]
fn tray_critical_alert(app: tauri::AppHandle, payload: CriticalAlertPayload) {
    if let Some(tray) = app.tray_by_id("aether-tray") {
        let tooltip = format!(
            "Aether — {} critical: {} ({})",
            payload.count, payload.title, payload.detail
        );
        let _ = tray.set_tooltip(Some(tooltip));
    }
    show_native_notification(&payload.title, &payload.detail);
}

#[tauri::command]
fn dock_badge_update(app: tauri::AppHandle, payload: DockBadgePayload) {
    let title = if payload.issue_count > 0 {
        format!("Aether ({})", payload.issue_count)
    } else {
        "Aether".into()
    };
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_title(&title);
    }
}

#[tauri::command]
fn native_notification_show(payload: NativeNotificationPayload) {
    show_native_notification(&payload.title, &payload.body);
}

#[tauri::command]
fn live_activity_update(app: tauri::AppHandle, payload: LiveActivityPayload) {
    if let Some(tray) = app.tray_by_id("aether-tray") {
        let tooltip = format!(
            "Migration {} — {}% · {}",
            payload.workload, payload.progress_pct, payload.detail
        );
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

fn show_native_notification(title: &str, body: &str) {
    #[cfg(target_os = "macos")]
    {
        let script = format!(
            "display notification \"{}\" with title \"Aether — {}\"",
            body.replace('\\', "\\\\").replace('"', "\\\""),
            title.replace('\\', "\\\\").replace('"', "\\\"")
        );
        let _ = std::process::Command::new("osascript")
            .arg("-e")
            .arg(script)
            .spawn();
    }
}

fn update_tray_tooltip(app: &tauri::AppHandle, health: f64, issues: u32, sparkline: Option<&str>) {
    if let Some(tray) = app.tray_by_id("aether-tray") {
        let spark = sparkline.unwrap_or("");
        let tooltip = if spark.is_empty() {
            format!(
                "Aether — Fleet {:.0}% healthy · {issues} issue{}",
                health,
                if issues == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "Aether {spark} · {:.0}% · {issues} issue{}",
                health,
                if issues == 1 { "" } else { "s" }
            )
        };
        let _ = tray.set_tooltip(Some(tooltip));
    }
}

fn open_command_palette(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.eval("window.dispatchEvent(new CustomEvent('aether-open-command-palette'));");
    }
}

fn open_spotlight(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
        let _ = window.eval("window.dispatchEvent(new CustomEvent('aether-open-spotlight'));");
    }
}

fn show_main_window(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn navigate_dashboard(app: &tauri::AppHandle, path: &str) {
    show_main_window(app);
    if let Some(window) = app.get_webview_window("main") {
        let escaped = path.replace('\\', "\\\\").replace('\'', "\\'");
        let _ = window.eval(&format!(
            "window.dispatchEvent(new CustomEvent('aether-navigate', {{ detail: {{ path: '{escaped}' }} }}));"
        ));
    }
}

fn handle_deep_link(app: &tauri::AppHandle, url: String) {
    show_main_window(app);
    if let Some(window) = app.get_webview_window("main") {
        let escaped = url.replace('\\', "\\\\").replace('\'', "\\'");
        let _ = window.eval(&format!(
            "window.dispatchEvent(new CustomEvent('aether-deep-link', {{ detail: {{ url: '{escaped}' }} }}));"
        ));
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    if event.state != ShortcutState::Pressed {
                        return;
                    }
                    let spotlight =
                        Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space);
                    if shortcut == &spotlight {
                        open_spotlight(app);
                    } else {
                        open_command_palette(app);
                    }
                })
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            tray_health_update,
            tray_sparkline_update,
            tray_critical_alert,
            dock_badge_update,
            native_notification_show,
            live_activity_update,
        ])
        .setup(|app| {
            let open_item = MenuItem::with_id(app, "open", "Open Aether", true, None::<&str>)?;
            let command_center_item = MenuItem::with_id(
                app,
                "command-center",
                "Command Center",
                true,
                None::<&str>,
            )?;
            let fleet_item = MenuItem::with_id(app, "fleet", "Fleet Intelligence", true, None::<&str>)?;
            let fabric_item = MenuItem::with_id(app, "fabric", "Runtime Fabric", true, None::<&str>)?;
            let palette_item =
                MenuItem::with_id(app, "palette", "Command Palette (⌘K)", true, None::<&str>)?;
            let agents_item =
                MenuItem::with_id(app, "agents", "Agent toggles", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "Quit Aether", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &open_item,
                    &command_center_item,
                    &fleet_item,
                    &fabric_item,
                    &palette_item,
                    &agents_item,
                    &PredefinedMenuItem::separator(app)?,
                    &quit_item,
                ],
            )?;

            let _tray = TrayIconBuilder::with_id("aether-tray")
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .tooltip("Aether — AI Infrastructure OS")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "open" => show_main_window(app),
                    "command-center" => navigate_dashboard(app, "/"),
                    "fleet" => navigate_dashboard(app, "/fleet"),
                    "fabric" => navigate_dashboard(app, "/fabric"),
                    "palette" => open_command_palette(app),
                    "agents" => {
                        handle_deep_link(app, "aether://settings".into());
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            #[cfg(target_os = "macos")]
            {
                let cmd_k = Shortcut::new(Some(Modifiers::SUPER), Code::KeyK);
                app.global_shortcut().register(cmd_k)?;
                let spotlight =
                    Shortcut::new(Some(Modifiers::SUPER | Modifiers::SHIFT), Code::Space);
                app.global_shortcut().register(spotlight)?;
            }

            if let Ok(url) = std::env::var("AETHER_DASHBOARD_URL") {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.eval(&format!("window.location.replace('{url}');"));
                }
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building Aether macOS shell")
        .run(|app_handle, event| {
            match event {
                RunEvent::Reopen { .. } => show_main_window(app_handle),
                RunEvent::Opened { urls } => {
                    for url in urls {
                        handle_deep_link(app_handle, url.to_string());
                    }
                }
                _ => {}
            }
        });
}
