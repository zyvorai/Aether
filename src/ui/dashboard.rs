//! Dashboard screen rendering

use super::{
    app::App,
    components::{bordered_block, help_text, runtime_badge, status_badge},
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render_dashboard(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(10),    // Main content
            Constraint::Length(3),  // Footer
        ])
        .split(f.area());

    render_header(f, chunks[0], app);
    render_workload_list(f, chunks[1], app);
    render_footer(f, chunks[2]);
}

fn render_header(f: &mut Frame, area: Rect, app: &App) {
    let title = vec![
        Line::from(vec![
            Span::styled(
                "ORCHESTR8",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled(
                "Universal Runtime Control Plane",
                Style::default().fg(Color::Gray),
            ),
        ]),
        Line::from(vec![
            Span::raw("Workloads: "),
            Span::styled(
                format!("{}", app.workloads.len()),
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" | Last refresh: "),
            Span::styled(
                format!("{:.0}s ago", app.last_refresh.elapsed().as_secs()),
                Style::default().fg(Color::DarkGray),
            ),
        ]),
    ];

    let header = Paragraph::new(title)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan)),
        )
        .alignment(Alignment::Center);

    f.render_widget(header, area);
}

fn render_workload_list(f: &mut Frame, area: Rect, app: &App) {
    if app.workloads.is_empty() {
        let empty_message = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled(
                "No workloads running",
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::ITALIC),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Run 'orchestr8 run' to deploy a workload",
                Style::default().fg(Color::Gray),
            )),
        ])
        .alignment(Alignment::Center)
        .block(bordered_block("Workloads"));

        f.render_widget(empty_message, area);
        return;
    }

    let items: Vec<ListItem> = app
        .workloads
        .iter()
        .enumerate()
        .map(|(idx, workload_info)| {
            let is_selected = idx == app.selected_index;

            let mut lines = Vec::new();

            // Line 1: Name and runtime
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {}", workload_info.state.name),
                    Style::default()
                        .fg(if is_selected { Color::Yellow } else { Color::White })
                        .add_modifier(if is_selected {
                            Modifier::BOLD
                        } else {
                            Modifier::empty()
                        }),
                ),
                Span::raw("  "),
                runtime_badge(&workload_info.state.runtime),
            ]));

            // Line 2: Status
            if let Some(ref status) = workload_info.status {
                let status_line = Line::from(vec![
                    Span::raw("   Status: "),
                    status_badge(&status.state, status.ready).spans[0].clone(),
                    Span::raw(" "),
                    status_badge(&status.state, status.ready).spans[2].clone(),
                ]);
                lines.push(status_line);

                // Line 3: Additional info
                let mut info_spans = vec![Span::raw("   ")];

                if status.restart_count > 0 {
                    info_spans.push(Span::styled(
                        format!("Restarts: {} ", status.restart_count),
                        Style::default().fg(Color::Red),
                    ));
                }

                if let Some(ref msg) = status.message {
                    info_spans.push(Span::styled(
                        format!("Message: {}", msg),
                        Style::default().fg(Color::Gray),
                    ));
                }

                if !info_spans.is_empty() && info_spans.len() > 1 {
                    lines.push(Line::from(info_spans));
                }
            } else {
                lines.push(Line::from(vec![
                    Span::raw("   Status: "),
                    Span::styled("Loading...", Style::default().fg(Color::DarkGray)),
                ]));
            }

            // Separator
            if !is_selected {
                lines.push(Line::from(""));
            }

            ListItem::new(lines).style(if is_selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            })
        })
        .collect();

    let list = List::new(items).block(bordered_block("Workloads"));

    f.render_widget(list, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let footer = Paragraph::new(help_text())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray)),
        )
        .alignment(Alignment::Center);

    f.render_widget(footer, area);
}
