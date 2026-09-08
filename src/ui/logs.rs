// Copyright 2026 ZyvorAI Labs Private Limited
// SPDX-License-Identifier: Apache-2.0

//! Log viewer screen - Orange-themed with enhanced features (matching HyperSDK)

use super::{
    app::App,
    components::{bordered_block, MUTED, PRIMARY, WARNING},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
    Frame,
};

pub fn render_logs(f: &mut Frame, app: &App, workload_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header with breadcrumb
            Constraint::Min(10),   // Logs
            Constraint::Length(3), // Footer
        ])
        .split(f.area());

    render_logs_header(f, chunks[0], workload_name, app);
    render_logs_content(f, chunks[1], app);
    render_logs_footer(f, chunks[2]);
}

fn render_logs_header(f: &mut Frame, area: Rect, workload_name: &str, app: &App) {
    let line_count = app.logs_buffer.len();
    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled(" Dashboard ", Style::default().fg(MUTED)),
        Span::styled(
            "›",
            Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
        ),
        Span::styled(" Logs ", Style::default().fg(MUTED)),
        Span::styled(
            "›",
            Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(" {} ", workload_name),
            Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  ({} lines)", line_count),
            Style::default().fg(MUTED),
        ),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(PRIMARY))
            .border_type(BorderType::Rounded),
    );

    f.render_widget(header, area);
}

fn render_logs_content(f: &mut Frame, area: Rect, app: &App) {
    if app.logs_buffer.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No logs available",
                Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Press 'r' to reload",
                Style::default().fg(WARNING),
            )),
        ])
        .alignment(Alignment::Center)
        .block(bordered_block(" Output "));

        f.render_widget(empty, area);
        return;
    }

    // Show last N lines that fit in the area
    let max_lines = (area.height as usize).saturating_sub(2).max(1);
    let start_idx = app.logs_buffer.len().saturating_sub(max_lines);

    let log_lines: Vec<Line> = app.logs_buffer[start_idx..]
        .iter()
        .enumerate()
        .map(|(i, line)| {
            // Line number prefix
            let line_num = start_idx + i + 1;
            let line_num_span =
                Span::styled(format!("{:>4} │ ", line_num), Style::default().fg(MUTED));

            // Log level coloring — shared classification with output::colorize_log_line
            let (r, g, b) = crate::output::log_level_color(line);
            let color = ratatui::style::Color::Rgb(r, g, b);

            Line::from(vec![
                line_num_span,
                Span::styled(line.clone(), Style::default().fg(color)),
            ])
        })
        .collect();

    let logs = Paragraph::new(log_lines).block(bordered_block(" Output "));

    f.render_widget(logs, area);
}

fn render_logs_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(
            "Esc/Backspace",
            Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Back  "),
        Span::styled(
            "r",
            Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Reload  "),
        Span::styled(
            "q",
            Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Quit"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(MUTED))
            .border_type(BorderType::Rounded),
    )
    .alignment(Alignment::Center);

    f.render_widget(footer, area);
}
