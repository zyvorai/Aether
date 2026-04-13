//! Pretty terminal output module - vision-optimized color scheme
//!
//! Matches HyperSDK's UX patterns: orange-themed colors, animated spinners,
//! boxed tables, styled banners, and structured output formatting.
//!
//! This is the single source of truth for the color palette. TUI components
//! in `ui/components.rs` derive their `ratatui::Color` values from these
//! constants via the `ratatui_color()` helper.

use colored::Colorize;
use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Attribute, Cell, Color as TableColor, ContentArrangement, Table};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use serde_json;

// ─── Output Mode Control ─────────────────────────────────────────────

static QUIET_MODE: AtomicBool = AtomicBool::new(false);
static JSON_MODE: AtomicBool = AtomicBool::new(false);
static YES_MODE: AtomicBool = AtomicBool::new(false);
static YAML_MODE: AtomicBool = AtomicBool::new(false);
static WIDE_MODE: AtomicBool = AtomicBool::new(false);

/// Enable quiet mode (suppress all output except errors)
pub fn set_quiet(enabled: bool) {
    QUIET_MODE.store(enabled, Ordering::Relaxed);
}

/// Enable JSON mode (machine-readable output)
pub fn set_json(enabled: bool) {
    JSON_MODE.store(enabled, Ordering::Relaxed);
}

/// Enable yes mode (auto-confirm all prompts)
pub fn set_yes(enabled: bool) {
    YES_MODE.store(enabled, Ordering::Relaxed);
}

/// Enable YAML output mode
pub fn set_yaml(enabled: bool) {
    YAML_MODE.store(enabled, Ordering::Relaxed);
}

/// Enable wide output mode (extra columns)
pub fn set_wide(enabled: bool) {
    WIDE_MODE.store(enabled, Ordering::Relaxed);
}

/// Check if quiet mode is active
pub fn is_quiet() -> bool {
    QUIET_MODE.load(Ordering::Relaxed)
}

/// Check if JSON mode is active
pub fn is_json() -> bool {
    JSON_MODE.load(Ordering::Relaxed)
}

/// Check if YAML output mode is active
pub fn is_yaml() -> bool {
    YAML_MODE.load(Ordering::Relaxed)
}

/// Check if wide output mode is active
pub fn is_wide() -> bool {
    WIDE_MODE.load(Ordering::Relaxed)
}

// ─── Vision-optimized color palette — SINGLE SOURCE OF TRUTH ─────────
//
// All display code (CLI colored output, comfy-table, ratatui TUI) must
// derive its colors from these constants rather than defining its own.

/// Dark Orange - primary accent, high contrast, no glare
pub const COLOR_PRIMARY: (u8, u8, u8) = (211, 84, 0);
/// Teal - info, directories
pub const COLOR_INFO: (u8, u8, u8) = (93, 173, 226);
/// Light Green - success
pub const COLOR_SUCCESS: (u8, u8, u8) = (163, 190, 140);
/// Orange - warnings
pub const COLOR_WARNING: (u8, u8, u8) = (243, 156, 18);
/// Red - errors
pub const COLOR_ERROR: (u8, u8, u8) = (231, 76, 60);
/// Gray - muted/secondary text
pub const COLOR_MUTED: (u8, u8, u8) = (107, 114, 128);

/// Per-runtime display metadata: (icon, name, color).
/// Used by both CLI `runtime_display()` and TUI `runtime_badge()`.
pub fn runtime_meta(runtime: &crate::runtime::RuntimeKind) -> (&'static str, &'static str, (u8, u8, u8)) {
    match runtime {
        crate::runtime::RuntimeKind::Podman => ("🐳", "Podman", (52, 152, 219)),
        crate::runtime::RuntimeKind::Docker => ("🐋", "Docker", (36, 130, 206)),
        crate::runtime::RuntimeKind::Kubernetes => ("☸️", "Kubernetes", (93, 173, 226)),
        crate::runtime::RuntimeKind::KubeVirt => ("🖥️", "KubeVirt", (155, 89, 182)),
        crate::runtime::RuntimeKind::Metal3 => ("🖧", "Metal3", (231, 76, 60)),
    }
}

/// Convert an `(r, g, b)` tuple to `ratatui::style::Color`.
/// Keeps ratatui out of this module's imports.
pub fn ratatui_color(c: (u8, u8, u8)) -> ratatui::style::Color {
    ratatui::style::Color::Rgb(c.0, c.1, c.2)
}

// ─── Display-width helper ─────────────────────────────────────────────

/// Approximate display width of a string. Counts ASCII as 1 column and
/// non-ASCII characters as 2 columns. Good enough for CJK and emoji
/// without pulling in `unicode-width`.
fn display_width(s: &str) -> usize {
    s.chars().map(|c| if c.is_ascii() { 1 } else { 2 }).sum()
}

// ─── ASCII Art Logo ───────────────────────────────────────────────────

/// Display the large ASCII art logo (like HyperSDK's pterm.BigText)
pub fn logo() {
    let art = r#"
   ___  ____   ____ _   _ _____ ____ _____ ____  ___
  / _ \|  _ \ / ___| | | | ____/ ___|_   _|  _ \( _ )
 | | | | |_) | |   | |_| |  _| \___ \ | | | |_) / _ \
 | |_| |  _ <| |___|  _  | |___ ___) || | |  _ < (_) |
  \___/|_| \_\\____|_| |_|_____|____/ |_| |_| \_\___/
"#;
    for line in art.lines() {
        println!(
            "{}",
            line.truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
                .bold()
        );
    }
}

// ─── Banner / Header ──────────────────────────────────────────────────

/// Display a styled banner with title and subtitle
pub fn banner(title: &str, subtitle: &str) {
    let width = 50;
    let border = "═".repeat(width);
    let pad = |s: &str| {
        let w = display_width(s);
        let space = width.saturating_sub(w);
        let left = space / 2;
        let right = space - left;
        format!("{}{}{}", " ".repeat(left), s, " ".repeat(right))
    };

    println!();
    println!(
        "{}",
        format!("╔{}╗", border)
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
    );
    println!(
        "{}",
        format!("║{}║", pad(title))
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .bold()
    );
    println!(
        "{}",
        format!("║{}║", pad(subtitle))
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
    println!(
        "{}",
        format!("║{}║", pad(&format!("v{}", env!("CARGO_PKG_VERSION"))))
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
    );
    println!(
        "{}",
        format!("╚{}╝", border)
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
    );
    println!();
}

/// Display a compact header for subcommands
pub fn header(icon: &str, title: &str) {
    println!(
        "\n{} {} {}",
        icon,
        title
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .bold(),
        format!("v{}", env!("CARGO_PKG_VERSION"))
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2),
    );
    println!(
        "{}",
        "─".repeat(display_width(title) + 8)
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
}

// ─── Spinners ─────────────────────────────────────────────────────────

/// Create an orange-themed spinner with braille animation
pub fn spinner(message: &str) -> ProgressBar {
    if is_quiet() || is_json() {
        return ProgressBar::hidden();
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.208} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]),
    );
    pb.set_message(
        message
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .to_string(),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

/// Finish spinner with success message
pub fn spinner_success(pb: &ProgressBar, message: &str) {
    pb.finish_with_message(
        message
            .truecolor(COLOR_SUCCESS.0, COLOR_SUCCESS.1, COLOR_SUCCESS.2)
            .to_string(),
    );
}

/// Finish spinner with failure message
pub fn spinner_fail(pb: &ProgressBar, message: &str) {
    pb.finish_with_message(
        message
            .truecolor(COLOR_ERROR.0, COLOR_ERROR.1, COLOR_ERROR.2)
            .to_string(),
    );
}

// ─── Status Messages ──────────────────────────────────────────────────

/// Emit a JSON status line with properly escaped message
fn json_status(status: &str, message: &str) {
    let json = serde_json::json!({ "status": status, "message": message });
    println!("{}", json);
}

/// Print a success message with green checkmark
pub fn success(message: &str) {
    if is_quiet() {
        return;
    }
    if is_json() {
        json_status("success", message);
        return;
    }
    println!(
        "{} {}",
        "✅".truecolor(COLOR_SUCCESS.0, COLOR_SUCCESS.1, COLOR_SUCCESS.2),
        message.truecolor(COLOR_SUCCESS.0, COLOR_SUCCESS.1, COLOR_SUCCESS.2)
    );
}

/// Print an error message with red cross (always shown, even in quiet mode)
pub fn error(message: &str) {
    if is_json() {
        let json = serde_json::json!({ "status": "error", "message": message });
        eprintln!("{}", json);
        return;
    }
    eprintln!(
        "{} {}",
        "❌".truecolor(COLOR_ERROR.0, COLOR_ERROR.1, COLOR_ERROR.2),
        message.truecolor(COLOR_ERROR.0, COLOR_ERROR.1, COLOR_ERROR.2)
    );
}

/// Print a warning message
pub fn warning(message: &str) {
    if is_quiet() {
        return;
    }
    if is_json() {
        json_status("warning", message);
        return;
    }
    println!(
        "{} {}",
        "⚠️ ".truecolor(COLOR_WARNING.0, COLOR_WARNING.1, COLOR_WARNING.2),
        message.truecolor(COLOR_WARNING.0, COLOR_WARNING.1, COLOR_WARNING.2)
    );
}

/// Print an info message
pub fn info(message: &str) {
    if is_quiet() {
        return;
    }
    if is_json() {
        json_status("info", message);
        return;
    }
    println!(
        "{} {}",
        "ℹ️ ".truecolor(COLOR_INFO.0, COLOR_INFO.1, COLOR_INFO.2),
        message.truecolor(COLOR_INFO.0, COLOR_INFO.1, COLOR_INFO.2)
    );
}

/// Print a muted/secondary message
pub fn muted(message: &str) {
    if is_quiet() || is_json() {
        return;
    }
    println!(
        "{}",
        message.truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
}

// ─── Section Headers ──────────────────────────────────────────────────

/// Print a section header with an orange underline
pub fn section(title: &str) {
    println!();
    println!(
        "{}",
        title
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .bold()
    );
    println!(
        "{}",
        "─".repeat(display_width(title))
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
}

/// Print a section header with emoji prefix
pub fn section_with_icon(icon: &str, title: &str) {
    println!();
    println!(
        "{} {}",
        icon,
        title
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .bold()
    );
    // emoji is ~2 display cols + 1 space + title width
    let underline_len = display_width(icon) + 1 + display_width(title);
    println!(
        "{}",
        "─".repeat(underline_len)
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
}

// ─── Key-Value Display ────────────────────────────────────────────────

/// Print a key-value pair with colored key
pub fn kv(key: &str, value: &str) {
    println!(
        "  {}: {}",
        key.truecolor(COLOR_INFO.0, COLOR_INFO.1, COLOR_INFO.2),
        value
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .bold()
    );
}

/// Print a key-value pair with tree branch prefix
pub fn kv_tree(key: &str, value: &str, last: bool) {
    let branch = if last { "└─" } else { "├─" };
    println!(
        "{} {}: {}",
        branch
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2),
        key.truecolor(COLOR_INFO.0, COLOR_INFO.1, COLOR_INFO.2),
        value
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .bold()
    );
}

/// Print an indented detail line
pub fn detail(message: &str) {
    println!(
        "  {}",
        message.truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
}

// ─── Tree-Style Bullet Items ──────────────────────────────────────────

/// Format a tree-style bullet item: `  ├─ icon text\n`
/// Use in format_* functions that build a String.
pub fn tree_bullet(icon: &str, text: &str) -> String {
    format!("  ├─ {} {}\n", icon, text)
}

// ─── Tables ───────────────────────────────────────────────────────────

/// Create a base table with standard preset
fn base_table() -> Table {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_content_arrangement(ContentArrangement::Dynamic);
    table
}

/// Style a header cell with the primary color
fn header_cell(text: &str) -> Cell {
    Cell::new(text)
        .fg(TableColor::Rgb {
            r: COLOR_PRIMARY.0,
            g: COLOR_PRIMARY.1,
            b: COLOR_PRIMARY.2,
        })
        .add_attribute(Attribute::Bold)
}

/// Create a boxed table with headers (HyperSDK style)
pub fn table(headers: &[&str], rows: Vec<Vec<String>>) -> String {
    let mut table = base_table();
    table.set_header(headers.iter().map(|h| header_cell(h)).collect::<Vec<_>>());

    for row in rows {
        table.add_row(row);
    }

    format!("{table}")
}

/// Create a property-value table wrapped with leading/trailing newlines.
/// Convenience for the common `format!("\n{}\n", property_table(...))` pattern.
pub fn property_section(pairs: &[(&str, String)]) -> String {
    format!("\n{}\n", property_table(pairs))
}

/// Create a simple property-value table (like HyperSDK's VM info display)
pub fn property_table(pairs: &[(&str, String)]) -> String {
    let mut table = base_table();
    table.set_header(vec![header_cell("Property"), header_cell("Value")]);

    for (key, value) in pairs {
        table.add_row(vec![
            Cell::new(key).fg(TableColor::Rgb {
                r: COLOR_INFO.0,
                g: COLOR_INFO.1,
                b: COLOR_INFO.2,
            }),
            Cell::new(value).add_attribute(Attribute::Bold),
        ]);
    }

    format!("{table}")
}

// ─── Box / Panel ──────────────────────────────────────────────────────

/// Display content in a bordered box (like pterm.DefaultBox)
pub fn boxed(title: &str, content: &str) {
    let lines: Vec<&str> = content.lines().collect();
    let max_width = lines
        .iter()
        .map(|l| display_width(l))
        .max()
        .unwrap_or(0)
        .max(display_width(title))
        + 4;

    let border = "─".repeat(max_width);
    println!(
        "{}",
        format!("┌─{}─┐", border)
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
    );
    let title_pad = max_width.saturating_sub(display_width(title));
    println!(
        "{} {}{} {}",
        "│".truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2),
        title
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .bold(),
        " ".repeat(title_pad + 1),
        "│".truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
    );
    println!(
        "{}",
        format!("├─{}─┤", border)
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
    for line in &lines {
        let pad = max_width.saturating_sub(display_width(line));
        println!(
            "{} {}{} {}",
            "│".truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2),
            line,
            " ".repeat(pad + 1),
            "│".truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
        );
    }
    println!(
        "{}",
        format!("└─{}─┘", border)
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
}

// ─── Bullet Lists ─────────────────────────────────────────────────────

/// Print a bullet list with colored bullets
pub fn bullet_list(items: &[&str]) {
    for item in items {
        println!(
            "  {} {}",
            "•".truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2),
            item
        );
    }
}

// ─── Progress Bars ────────────────────────────────────────────────────

/// Create a progress bar with orange theme
pub fn progress_bar(total: u64, message: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{msg}\n{spinner:.208} [{bar:40.208/237}] {pos}/{len} ({eta})",
        )
        .unwrap()
        .progress_chars("███░"),
    );
    pb.set_message(message.to_string());
    pb
}

// ─── Runtime Icons ────────────────────────────────────────────────────

/// Get colored runtime display string for CLI output
pub fn runtime_display(runtime: &crate::runtime::RuntimeKind) -> String {
    let (icon, name, color) = runtime_meta(runtime);
    format!(
        "{} {}",
        icon,
        name.truecolor(color.0, color.1, color.2).bold()
    )
}

// ─── Formatted output for "serve" endpoint listing ────────────────────

/// Print an API endpoint line
pub fn endpoint(method: &str, path: &str) {
    let method_color = match method {
        "GET" => COLOR_SUCCESS,
        "POST" => COLOR_WARNING,
        "DELETE" => COLOR_ERROR,
        "PUT" | "PATCH" => COLOR_INFO,
        _ => COLOR_MUTED,
    };
    println!(
        "     {} {}",
        method
            .truecolor(method_color.0, method_color.1, method_color.2)
            .bold(),
        path.truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
}

/// Print a category label for endpoint grouping
pub fn endpoint_category(name: &str) {
    println!(
        "\n   {}:",
        name.truecolor(COLOR_INFO.0, COLOR_INFO.1, COLOR_INFO.2)
            .bold()
    );
}

// ─── Summary Box ──────────────────────────────────────────────────────

/// Internal helper for summary boxes with configurable color/icon
fn summary_box(title: &str, items: &[(&str, String)], color: (u8, u8, u8), icon: &str) {
    if is_quiet() {
        return;
    }
    let max_key = items.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    let max_val = items.iter().map(|(_, v)| v.len()).max().unwrap_or(0);
    let inner_width = (max_key + max_val + 5).max(display_width(title) + 4);
    let border = "─".repeat(inner_width);

    println!();
    println!(
        "{}",
        format!("┌─{}─┐", border).truecolor(color.0, color.1, color.2)
    );
    let title_pad = inner_width.saturating_sub(display_width(title) + 2);
    println!(
        "{} {} {}{}",
        "│".truecolor(color.0, color.1, color.2),
        format!("{} {}", icon, title)
            .truecolor(color.0, color.1, color.2)
            .bold(),
        " ".repeat(title_pad),
        "│".truecolor(color.0, color.1, color.2)
    );
    println!(
        "{}",
        format!("├─{}─┤", border)
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
    for (key, value) in items {
        let line = format!("{:>width$}: {}", key, value, width = max_key);
        let pad = inner_width.saturating_sub(line.len());
        println!(
            "{} {}{} {}",
            "│".truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2),
            line,
            " ".repeat(pad),
            "│".truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
        );
    }
    println!(
        "{}",
        format!("└─{}─┘", border)
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
}

/// Display a success summary box (used after major operations)
pub fn summary_success(title: &str, items: &[(&str, String)]) {
    summary_box(title, items, COLOR_SUCCESS, "✅");
}

/// Display an error summary box
pub fn summary_error(title: &str, items: &[(&str, String)]) {
    summary_box(title, items, COLOR_ERROR, "❌");
}

// ─── Multi-Progress ───────────────────────────────────────────────────

/// Create a multi-progress group for parallel operations
pub fn multi_progress() -> MultiProgress {
    MultiProgress::new()
}

/// Add a spinner to a multi-progress group
pub fn multi_spinner(mp: &MultiProgress, message: &str) -> ProgressBar {
    let pb = mp.add(ProgressBar::new_spinner());
    pb.set_style(
        ProgressStyle::with_template("{spinner:.208} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✓"]),
    );
    pb.set_message(
        message
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .to_string(),
    );
    pb.enable_steady_tick(Duration::from_millis(80));
    pb
}

// ─── Log Level Coloring ───────────────────────────────────────────────

/// Case-insensitive substring check without allocating
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .as_bytes()
        .windows(needle.len())
        .any(|w| w.eq_ignore_ascii_case(needle.as_bytes()))
}

/// Determine the display color for a log line based on its level.
/// Returns the RGB color tuple. Usable by both CLI and TUI rendering.
pub fn log_level_color(line: &str) -> (u8, u8, u8) {
    if contains_ci(line, "error") || contains_ci(line, "fatal") || contains_ci(line, "panic") {
        COLOR_ERROR
    } else if contains_ci(line, "warn") {
        COLOR_WARNING
    } else if contains_ci(line, "info") {
        COLOR_SUCCESS
    } else if contains_ci(line, "debug") || contains_ci(line, "trace") {
        COLOR_MUTED
    } else {
        (255, 255, 255) // default white
    }
}

/// Colorize a log line based on its log level
pub fn colorize_log_line(line: &str) -> String {
    let c = log_level_color(line);
    line.truecolor(c.0, c.1, c.2).to_string()
}

// ─── Confirmation Prompt ──────────────────────────────────────────────

/// Display a styled confirmation prompt and return true if confirmed.
/// Auto-confirms (returns true) only with `--yes`.
/// In quiet/json mode, defaults to rejecting (safe by default) unless `--yes` is also set.
pub fn confirm(message: &str) -> bool {
    if YES_MODE.load(Ordering::Relaxed) {
        return true; // explicit --yes: auto-confirm
    }
    if is_quiet() || is_json() {
        return false; // non-interactive without --yes: safe default is to reject
    }
    eprint!(
        "\n{} {} {} ",
        "?".truecolor(COLOR_WARNING.0, COLOR_WARNING.1, COLOR_WARNING.2)
            .bold(),
        message.truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2),
        "[y/N]"
            .truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2),
    );
    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    } else {
        false
    }
}

// ─── Capability / Feature Detection Display ───────────────────────────

/// Display a capabilities/features table (like HyperSDK's capability detection)
pub fn capabilities(items: &[(&str, bool, &str)]) {
    let rows: Vec<Vec<String>> = items
        .iter()
        .map(|(name, available, detail)| {
            vec![
                if *available {
                    "✅".to_string()
                } else {
                    "❌".to_string()
                },
                name.to_string(),
                detail.to_string(),
            ]
        })
        .collect();
    println!("{}", table(&["", "Capability", "Details"], rows));
}

// ─── Hierarchical Bullet List ─────────────────────────────────────────

/// Print a hierarchical bullet list with indent levels
pub fn tree_list(items: &[(&str, usize)]) {
    for (item, level) in items {
        let indent = "  ".repeat(*level);
        let bullet = if *level == 0 { "●" } else { "○" };
        println!(
            "{}{} {}",
            indent,
            bullet.truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2),
            item,
        );
    }
}

// ─── Diff / Change Display ────────────────────────────────────────────

/// Display a before/after change (like git diff style)
pub fn change(field: &str, old_val: &str, new_val: &str) {
    println!(
        "  {} {}",
        field
            .truecolor(COLOR_INFO.0, COLOR_INFO.1, COLOR_INFO.2)
            .bold(),
        ":".truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
    println!(
        "    {} {}",
        "-".truecolor(COLOR_ERROR.0, COLOR_ERROR.1, COLOR_ERROR.2),
        old_val.truecolor(COLOR_ERROR.0, COLOR_ERROR.1, COLOR_ERROR.2)
    );
    println!(
        "    {} {}",
        "+".truecolor(COLOR_SUCCESS.0, COLOR_SUCCESS.1, COLOR_SUCCESS.2),
        new_val.truecolor(COLOR_SUCCESS.0, COLOR_SUCCESS.1, COLOR_SUCCESS.2)
    );
}

// ─── Interactive Selector ─────────────────────────────────────────────

/// Display an interactive runtime selector with descriptions.
/// Returns the selected index, or None if cancelled.
/// Falls back to `None` in quiet/json/non-interactive modes.
pub fn select_runtime(options: &[(&str, &str, &str)]) -> Option<usize> {
    if is_quiet() || is_json() || YES_MODE.load(Ordering::Relaxed) {
        return None; // non-interactive: let auto-select decide
    }

    println!();
    println!(
        "{}",
        "  Select a runtime:"
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .bold()
    );
    println!();

    for (i, (icon, name, description)) in options.iter().enumerate() {
        println!(
            "  {} {} {}  {}",
            format!("[{}]", i + 1)
                .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
                .bold(),
            icon,
            name.truecolor(COLOR_INFO.0, COLOR_INFO.1, COLOR_INFO.2)
                .bold(),
            description.truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2),
        );
    }

    eprint!(
        "\n  {} ",
        format!("Choice (1-{}, or Enter for auto):", options.len())
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
    );

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok() {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return None; // auto-select
        }
        if let Ok(n) = trimmed.parse::<usize>() {
            if (1..=options.len()).contains(&n) {
                return Some(n - 1);
            }
        }
    }
    None
}

// ─── Step / Wizard Display ────────────────────────────────────────────

/// Display a step indicator (like HyperSDK's "Step 1/3: VM Selection")
pub fn step(current: usize, total: usize, title: &str) {
    println!(
        "\n{} {}",
        format!("Step {}/{}", current, total)
            .truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2)
            .bold(),
        title
            .truecolor(COLOR_INFO.0, COLOR_INFO.1, COLOR_INFO.2),
    );
    // Progress dots
    let filled = "●".repeat(current);
    let empty = "○".repeat(total.saturating_sub(current));
    println!(
        "  {}{}",
        filled.truecolor(COLOR_PRIMARY.0, COLOR_PRIMARY.1, COLOR_PRIMARY.2),
        empty.truecolor(COLOR_MUTED.0, COLOR_MUTED.1, COLOR_MUTED.2)
    );
}
