//! Reusable UI components

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, BorderType},
};

/// Create a standard block with borders
pub fn bordered_block<'a>(title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .style(Style::default().fg(Color::Cyan))
}

/// Create a highlighted block
pub fn highlighted_block<'a>(title: &'a str) -> Block<'a> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .title(title)
        .style(Style::default().fg(Color::Yellow))
}

/// Get color for runtime
pub fn runtime_color(runtime: &crate::runtime::RuntimeKind) -> Color {
    match runtime {
        crate::runtime::RuntimeKind::Podman => Color::Blue,
        crate::runtime::RuntimeKind::Kubernetes => Color::Cyan,
        crate::runtime::RuntimeKind::KubeVirt => Color::Magenta,
        crate::runtime::RuntimeKind::Metal3 => Color::Red,
    }
}

/// Get color for instance state
pub fn state_color(state: &crate::runtime::InstanceState) -> Color {
    match state {
        crate::runtime::InstanceState::Running => Color::Green,
        crate::runtime::InstanceState::Pending => Color::Yellow,
        crate::runtime::InstanceState::Stopped => Color::Gray,
        crate::runtime::InstanceState::Failed => Color::Red,
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
                .fg(if ready { Color::Green } else { Color::Red })
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

/// Create runtime badge
pub fn runtime_badge(runtime: &crate::runtime::RuntimeKind) -> Span<'static> {
    let icon = match runtime {
        crate::runtime::RuntimeKind::Podman => "🐳",
        crate::runtime::RuntimeKind::Kubernetes => "☸️",
        crate::runtime::RuntimeKind::KubeVirt => "🖥️",
        crate::runtime::RuntimeKind::Metal3 => "🖧",
    };

    let name = format!("{}", runtime);

    Span::styled(
        format!("{} {}", icon, name),
        Style::default()
            .fg(runtime_color(runtime))
            .add_modifier(Modifier::BOLD),
    )
}

/// Create help text
pub fn help_text() -> Line<'static> {
    Line::from(vec![
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(" Select | "),
        Span::styled("Enter", Style::default().fg(Color::Yellow)),
        Span::raw(" Logs | "),
        Span::styled("r", Style::default().fg(Color::Yellow)),
        Span::raw(" Refresh | "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit"),
    ])
}

/// Create footer with help text
pub fn footer<'a>() -> Block<'a> {
    Block::default()
        .borders(Borders::TOP)
        .style(Style::default().fg(Color::DarkGray))
}
