// Copyright (c) 2026 ZyvorAI Labs Private Limited. All rights reserved.
// Proprietary software — see LICENSE in the repository root.
// https://zyvor.dev · info@zyvor.dev

//! Dashboard screen rendering - Orange-themed with detail panel (matching HyperSDK)

use super::{
    app::App,
    components::{
        bordered_block, help_text, runtime_badge, state_color, status_badge, ERROR, INFO, MUTED,
        PRIMARY, SUCCESS, WARNING,
    },
};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, Paragraph},
    Frame,
};

pub fn render_dashboard(f: &mut Frame, app: &App) {
    let area = f.area();
    // Guard against very small terminals (need at least header + 1 row + footer = 7 lines)
    if area.height < 7 {
        let msg = Paragraph::new("Terminal too small").alignment(Alignment::Center);
        f.render_widget(msg, area);
        return;
    }
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Length(3), // Stats bar
            Constraint::Min(1),    // Main content (list + detail)
            Constraint::Length(3), // Footer
        ])
        .split(area);

    render_header(f, chunks[0], app);
    render_stats_bar(f, chunks[1], app);
    render_main_content(f, chunks[2], app);
    render_footer(f, app, chunks[3]);
}

fn render_header(f: &mut Frame, area: Rect, _app: &App) {
    let title = vec![Line::from(vec![
        Span::styled(
            " ⚡ AETHER ",
            Style::default()
                .fg(Color::Black)
                .bg(PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " Universal Runtime Control Plane ",
            Style::default().fg(MUTED),
        ),
        Span::styled(
            format!(" v{} ", env!("CARGO_PKG_VERSION")),
            Style::default().fg(Color::Black).bg(MUTED),
        ),
    ])];

    let header = Paragraph::new(title)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(PRIMARY))
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Center);

    f.render_widget(header, area);
}

fn render_stats_bar(f: &mut Frame, area: Rect, app: &App) {
    let total = app.workloads.len();
    // Single pass over workloads for all stats
    let (mut running, mut failed, mut restarts) = (0usize, 0usize, 0u32);
    for w in &app.workloads {
        if let Some(ref s) = w.status {
            if s.ready {
                running += 1;
            }
            if matches!(s.state, crate::runtime::InstanceState::Failed) {
                failed += 1;
            }
            restarts = restarts.saturating_add(s.restart_count);
        }
    }

    let stats = vec![Line::from(vec![
        Span::styled(" Total: ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{}", total),
            Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  Ready: ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{}", running),
            Style::default().fg(SUCCESS).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  Failed: ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{}", failed),
            Style::default()
                .fg(if failed > 0 { ERROR } else { MUTED })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  Restarts: ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{}", restarts),
            Style::default()
                .fg(if restarts > 0 { WARNING } else { MUTED })
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  │  Refresh: ", Style::default().fg(MUTED)),
        Span::styled(
            format!("{}s ago", app.last_refresh.elapsed().as_secs()),
            Style::default().fg(MUTED),
        ),
    ])];

    let bar = Paragraph::new(stats)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED))
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Center);

    f.render_widget(bar, area);
}

fn render_main_content(f: &mut Frame, area: Rect, app: &App) {
    let filtered = app.filtered_workloads();
    if filtered.is_empty() {
        if !app.search_filter.is_empty() {
            // Show "no matches" when filter is active
            let msg = Paragraph::new(vec![
                Line::from(""),
                Line::from(Span::styled(
                    format!("No workloads matching '{}'", app.search_filter),
                    Style::default().fg(WARNING).add_modifier(Modifier::ITALIC),
                )),
                Line::from(Span::styled(
                    "Press Esc to clear filter",
                    Style::default().fg(MUTED),
                )),
            ])
            .alignment(Alignment::Center)
            .block(bordered_block(" Workloads "));
            f.render_widget(msg, area);
        } else {
            render_empty_state(f, area);
        }
        return;
    }

    // Split into list (left) and detail panel (right)
    let has_detail_space = area.width > 60;
    if has_detail_space {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        render_workload_list(f, chunks[0], app);
        render_detail_panel(f, chunks[1], app);
    } else {
        render_workload_list(f, area, app);
    }
}

fn render_empty_state(f: &mut Frame, area: Rect) {
    let empty_message = Paragraph::new(vec![
        Line::from(""),
        Line::from(Span::styled(
            "No workloads running",
            Style::default().fg(MUTED).add_modifier(Modifier::ITALIC),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Run 'aether run' to deploy a workload",
            Style::default().fg(WARNING),
        )),
    ])
    .alignment(Alignment::Center)
    .block(bordered_block(" Workloads "));

    f.render_widget(empty_message, area);
}

fn render_workload_list(f: &mut Frame, area: Rect, app: &App) {
    let filtered = app.filtered_workloads();
    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(idx, workload_info)| {
            let is_selected = idx == app.selected_index;

            let mut lines = Vec::new();

            // Line 1: Name and runtime
            lines.push(Line::from(vec![
                Span::styled(
                    if is_selected { " ▶ " } else { "   " },
                    Style::default().fg(PRIMARY),
                ),
                Span::styled(
                    workload_info.state.name.clone(),
                    Style::default()
                        .fg(if is_selected { PRIMARY } else { Color::White })
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
                let badge = status_badge(&status.state, status.ready);
                let badge_icon = badge
                    .spans
                    .first()
                    .cloned()
                    .unwrap_or_else(|| Span::raw("?"));
                let badge_text = badge
                    .spans
                    .get(2)
                    .cloned()
                    .unwrap_or_else(|| Span::raw("unknown"));
                let mut status_spans =
                    vec![Span::raw("     "), badge_icon, Span::raw(" "), badge_text];

                if status.restart_count > 0 {
                    status_spans.push(Span::styled(
                        format!("  ↻{}", status.restart_count),
                        Style::default().fg(ERROR),
                    ));
                }

                lines.push(Line::from(status_spans));
            } else {
                lines.push(Line::from(vec![
                    Span::raw("     "),
                    Span::styled("⠿ Loading...", Style::default().fg(MUTED)),
                ]));
            }

            ListItem::new(lines).style(if is_selected {
                Style::default().bg(Color::Rgb(30, 30, 30))
            } else {
                Style::default()
            })
        })
        .collect();

    let list = List::new(items).block(bordered_block(" Workloads "));

    f.render_widget(list, area);
}

/// Detail panel showing selected workload info (like HyperSDK's VM info panel)
fn render_detail_panel(f: &mut Frame, area: Rect, app: &App) {
    let selected = match app.selected_workload() {
        Some(w) => w,
        None => {
            let empty = Paragraph::new("Select a workload")
                .style(Style::default().fg(MUTED))
                .alignment(Alignment::Center)
                .block(bordered_block(" Details "));
            f.render_widget(empty, area);
            return;
        }
    };

    let ws = &selected.state;
    let mut lines: Vec<Line> = vec![
        // Name
        Line::from(vec![
            Span::styled("  Name:     ", Style::default().fg(INFO)),
            Span::styled(
                ws.name.clone(),
                Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
            ),
        ]),
        // Runtime
        Line::from(vec![
            Span::styled("  Runtime:  ", Style::default().fg(INFO)),
            runtime_badge(&ws.runtime),
        ]),
        // Instance ID
        Line::from(vec![
            Span::styled("  Instance: ", Style::default().fg(INFO)),
            Span::styled(
                ws.instance.id.chars().take(12).collect::<String>(),
                Style::default().fg(Color::White),
            ),
        ]),
        // Image
        Line::from(vec![
            Span::styled("  Image:    ", Style::default().fg(INFO)),
            Span::styled(ws.instance.image.clone(), Style::default().fg(MUTED)),
        ]),
        // Created
        Line::from(vec![
            Span::styled("  Created:  ", Style::default().fg(INFO)),
            Span::styled(
                ws.created_at.chars().take(19).collect::<String>(),
                Style::default().fg(MUTED),
            ),
        ]),
        // Spec path
        Line::from(vec![
            Span::styled("  Spec:     ", Style::default().fg(INFO)),
            Span::styled(
                ws.spec_path.display().to_string(),
                Style::default().fg(MUTED),
            ),
        ]),
        Line::from(""),
    ];

    // Status details
    if let Some(ref status) = selected.status {
        lines.push(Line::from(vec![Span::styled(
            "  ─── Status ───",
            Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
        )]));

        lines.push(Line::from(vec![
            Span::styled("  State:    ", Style::default().fg(INFO)),
            Span::styled(
                format!("{}", status.state),
                Style::default()
                    .fg(state_color(&status.state))
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        lines.push(Line::from(vec![
            Span::styled("  Ready:    ", Style::default().fg(INFO)),
            Span::styled(
                if status.ready { "● Yes" } else { "○ No" },
                Style::default().fg(if status.ready { SUCCESS } else { ERROR }),
            ),
        ]));

        if status.restart_count > 0 {
            lines.push(Line::from(vec![
                Span::styled("  Restarts: ", Style::default().fg(INFO)),
                Span::styled(
                    format!("{}", status.restart_count),
                    Style::default().fg(ERROR).add_modifier(Modifier::BOLD),
                ),
            ]));
        }

        if let Some(ref msg) = status.message {
            lines.push(Line::from(vec![
                Span::styled("  Message:  ", Style::default().fg(INFO)),
                Span::styled(msg.clone(), Style::default().fg(WARNING)),
            ]));
        }

        // Resource requirements from spec (if loadable)
        if let Ok(spec) = crate::spec::Workload::from_file(&ws.spec_path) {
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                "  ─── Resources ───",
                Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(vec![
                Span::styled("  CPU:      ", Style::default().fg(INFO)),
                Span::styled(
                    spec.requirements.cpu.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Memory:   ", Style::default().fg(INFO)),
                Span::styled(
                    spec.requirements.memory.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::from(vec![
                Span::styled("  Storage:  ", Style::default().fg(INFO)),
                Span::styled(
                    spec.requirements.storage.clone(),
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                ),
            ]));
            if let Some(ref gpu) = spec.requirements.gpu {
                lines.push(Line::from(vec![
                    Span::styled("  GPU:      ", Style::default().fg(INFO)),
                    Span::styled(
                        format!("{}x {}", gpu.count, gpu.vendor),
                        Style::default().fg(WARNING).add_modifier(Modifier::BOLD),
                    ),
                ]));
            }
        }

        // Health bar — considers state, readiness, and restart count
        lines.push(Line::from(""));
        let health_pct: usize = match status.state {
            crate::runtime::InstanceState::Running if status.ready => {
                // Deduct 10% per restart, floor at 50%
                100usize
                    .saturating_sub(status.restart_count as usize * 10)
                    .max(50)
            }
            crate::runtime::InstanceState::Running => 40, // running but not ready
            crate::runtime::InstanceState::Pending => 20,
            crate::runtime::InstanceState::Failed => 0,
            _ => 10,
        };
        let bar_width: usize = 20;
        let filled = bar_width * health_pct / 100;
        let empty = bar_width - filled;
        let bar_color = if health_pct >= 80 {
            SUCCESS
        } else if health_pct >= 40 {
            WARNING
        } else {
            ERROR
        };
        lines.push(Line::from(vec![
            Span::styled("  Health:   ", Style::default().fg(INFO)),
            Span::styled("█".repeat(filled), Style::default().fg(bar_color)),
            Span::styled("░".repeat(empty), Style::default().fg(MUTED)),
            Span::styled(
                format!(" {}%", health_pct),
                Style::default().fg(bar_color).add_modifier(Modifier::BOLD),
            ),
        ]));
    }

    let detail = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(PRIMARY))
            .title(Span::styled(
                " Details ",
                Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD),
            )),
    );

    f.render_widget(detail, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let content = if app.search_active {
        Line::from(vec![
            Span::styled(
                " Search: ",
                Style::default().fg(WARNING).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                app.search_filter.clone(),
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("▌", Style::default().fg(WARNING)),
            Span::styled(
                "  [Enter] confirm  [Esc] cancel",
                Style::default().fg(MUTED),
            ),
        ])
    } else if !app.search_filter.is_empty() {
        Line::from(vec![
            Span::styled(
                format!(" Filter: {} ", app.search_filter),
                Style::default().fg(WARNING),
            ),
            Span::styled("│ [Esc] clear  ", Style::default().fg(MUTED)),
        ])
    } else {
        let mut spans = vec![Span::styled("[/] search  ", Style::default().fg(MUTED))];
        spans.extend(help_text().spans);
        Line::from(spans)
    };

    let footer = Paragraph::new(content)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(MUTED))
                .border_type(BorderType::Rounded),
        )
        .alignment(Alignment::Center);

    f.render_widget(footer, area);
}
