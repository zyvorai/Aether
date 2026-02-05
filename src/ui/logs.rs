//! Log viewer screen

use super::{app::App, components::bordered_block};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub fn render_logs(f: &mut Frame, app: &App, workload_name: &str) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Logs
            Constraint::Length(3),  // Footer
        ])
        .split(f.area());

    render_logs_header(f, chunks[0], workload_name);
    render_logs_content(f, chunks[1], app);
    render_logs_footer(f, chunks[2]);
}

fn render_logs_header(f: &mut Frame, area: Rect, workload_name: &str) {
    let header = Paragraph::new(vec![Line::from(vec![
        Span::styled(
            "Logs: ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            workload_name,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::Cyan)),
    )
    .alignment(Alignment::Center);

    f.render_widget(header, area);
}

fn render_logs_content(f: &mut Frame, area: Rect, app: &App) {
    if app.logs_buffer.is_empty() {
        let empty = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No logs available",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
        ])
        .alignment(Alignment::Center)
        .block(bordered_block("Output"));

        f.render_widget(empty, area);
        return;
    }

    // Show last N lines that fit in the area
    let max_lines = (area.height as usize).saturating_sub(2);
    let start_idx = app.logs_buffer.len().saturating_sub(max_lines);

    let log_lines: Vec<Line> = app.logs_buffer[start_idx..]
        .iter()
        .map(|line| {
            // Simple log level coloring
            let color = if line.contains("ERROR") || line.contains("error") {
                Color::Red
            } else if line.contains("WARN") || line.contains("warn") {
                Color::Yellow
            } else if line.contains("INFO") || line.contains("info") {
                Color::Green
            } else {
                Color::White
            };

            Line::from(Span::styled(line.clone(), Style::default().fg(color)))
        })
        .collect();

    let logs = Paragraph::new(log_lines).block(bordered_block("Output"));

    f.render_widget(logs, area);
}

fn render_logs_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(Line::from(vec![
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(" Back | "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit"),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::DarkGray)),
    )
    .alignment(Alignment::Center);

    f.render_widget(footer, area);
}
