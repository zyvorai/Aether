//! Reusable UI components - colors derived from output::COLOR_* (single source of truth)

use crate::output;
use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

// ─── Colors derived from the shared palette in output.rs ─────────────

pub const PRIMARY: Color = Color::Rgb(output::COLOR_PRIMARY.0, output::COLOR_PRIMARY.1, output::COLOR_PRIMARY.2);
pub const INFO: Color = Color::Rgb(output::COLOR_INFO.0, output::COLOR_INFO.1, output::COLOR_INFO.2);
pub const SUCCESS: Color = Color::Rgb(output::COLOR_SUCCESS.0, output::COLOR_SUCCESS.1, output::COLOR_SUCCESS.2);
pub const WARNING: Color = Color::Rgb(output::COLOR_WARNING.0, output::COLOR_WARNING.1, output::COLOR_WARNING.2);
pub const ERROR: Color = Color::Rgb(output::COLOR_ERROR.0, output::COLOR_ERROR.1, output::COLOR_ERROR.2);
pub const MUTED: Color = Color::Rgb(output::COLOR_MUTED.0, output::COLOR_MUTED.1, output::COLOR_MUTED.2);

/// Create a standard block with orange-themed borders
pub fn bordered_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .style(Style::default().fg(PRIMARY))
}

/// Create a highlighted block
pub fn highlighted_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(title)
        .style(Style::default().fg(WARNING))
}

/// Get color for runtime — derived from the shared runtime_meta()
pub fn runtime_color(runtime: &crate::runtime::RuntimeKind) -> Color {
    let (_, _, c) = output::runtime_meta(runtime);
    Color::Rgb(c.0, c.1, c.2)
}

/// Get color for instance state
pub fn state_color(state: &crate::runtime::InstanceState) -> Color {
    match state {
        crate::runtime::InstanceState::Running => SUCCESS,
        crate::runtime::InstanceState::Pending => WARNING,
        crate::runtime::InstanceState::Stopped => MUTED,
        crate::runtime::InstanceState::Failed => ERROR,
        crate::runtime::InstanceState::Unknown => Color::DarkGray,
    }
}

/// Create status badge
pub fn status_badge(state: &crate::runtime::InstanceState, ready: bool) -> Line<'static> {
    let state_str = format!("{}", state);
    let ready_indicator = if ready { "●" } else { "○" };

    Line::from(vec![
        Span::styled(
            ready_indicator,
            Style::default()
                .fg(if ready { SUCCESS } else { ERROR })
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(
            state_str,
            Style::default()
                .fg(state_color(state))
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Create runtime badge — icon and name derived from shared runtime_meta()
pub fn runtime_badge(runtime: &crate::runtime::RuntimeKind) -> Span<'static> {
    let (icon, _, _) = output::runtime_meta(runtime);
    let name = format!("{}", runtime);

    Span::styled(
        format!("{} {}", icon, name),
        Style::default()
            .fg(runtime_color(runtime))
            .add_modifier(Modifier::BOLD),
    )
}

/// Create help text with orange-themed key highlights
pub fn help_text() -> Line<'static> {
    Line::from(vec![
        Span::styled("↑↓/jk", Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw(" Navigate  "),
        Span::styled("Enter", Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw(" Logs  "),
        Span::styled("g/G", Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw(" Top/Bottom  "),
        Span::styled("r", Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw(" Refresh  "),
        Span::styled("q", Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)),
        Span::raw(" Quit"),
    ])
}

/// Create footer with help text
pub fn footer<'a>() -> Block<'a> {
    Block::default()
        .borders(Borders::TOP)
        .style(Style::default().fg(MUTED))
}
