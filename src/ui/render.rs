use chrono::{DateTime, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::storage::{ScanOutcome, SourceHealth};

use super::{App, InputMode, View};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let theme = app.theme();
    frame.render_widget(Block::new().style(Style::new().bg(theme.background)), area);

    let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
    render_content(frame, app, areas[0]);
    render_footer(frame, app, areas[1]);
    if app.help_visible() {
        render_help(frame, app, areas[0]);
    }
}

fn render_content(frame: &mut Frame, app: &App, area: Rect) {
    if matches!(app.view(), View::Scans | View::Sources) {
        render_operational_view(frame, app, area);
        return;
    }
    match area.width {
        120.. => render_navigation_list_details(frame, app, area),
        80..=119 => render_list_details(frame, app, area),
        _ => render_single_job_pane(frame, app, area),
    }
}

fn render_navigation_list_details(frame: &mut Frame, app: &App, area: Rect) {
    let areas = Layout::horizontal([
        Constraint::Length(22),
        Constraint::Percentage(40),
        Constraint::Fill(1),
    ])
    .split(area);
    render_navigation(frame, app, areas[0]);
    render_job_list(
        frame,
        app,
        areas[1],
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
    );
    render_details(
        frame,
        app,
        areas[2],
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
    );
}

fn render_list_details(frame: &mut Frame, app: &App, area: Rect) {
    let areas = Layout::horizontal([Constraint::Percentage(45), Constraint::Fill(1)]).split(area);
    render_job_list(frame, app, areas[0], Borders::ALL);
    render_details(
        frame,
        app,
        areas[1],
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
    );
}

fn render_single_job_pane(frame: &mut Frame, app: &App, area: Rect) {
    if app.narrow_details_visible() {
        render_details(frame, app, area, Borders::ALL);
    } else {
        render_job_list(frame, app, area, Borders::ALL);
    }
}

fn render_operational_view(frame: &mut Frame, app: &App, area: Rect) {
    if area.width >= 120 {
        let areas = Layout::horizontal([Constraint::Length(22), Constraint::Fill(1)]).split(area);
        render_navigation(frame, app, areas[0]);
        render_operational_surface(
            frame,
            app,
            areas[1],
            Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
        );
    } else {
        render_operational_surface(frame, app, area, Borders::ALL);
    }
}

fn render_operational_surface(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    match app.view() {
        View::Scans => render_scans(frame, app, area, borders),
        View::Sources => render_sources(frame, app, area, borders),
        _ => unreachable!("operational surfaces have fixed views"),
    }
}

fn render_navigation(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let icons = app.icons();
    let items = vec![
        nav_item(icons.open, "Active", theme.open),
        nav_item(icons.new, "New", theme.new),
        nav_item(icons.applied, "Applied", theme.applied),
        nav_item(icons.history, "History", theme.muted_text),
        nav_item(icons.scanning, "Scans", theme.primary_text),
        nav_item(icons.source_failure, "Sources", theme.warning),
    ];
    let list = List::new(items)
        .block(panel(
            "Navigation",
            theme.unfocused_border,
            theme.background,
            Borders::ALL,
        ))
        .style(Style::new().fg(theme.primary_text).bg(theme.background))
        .highlight_style(Style::new().bg(theme.selected_row))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    state.select(Some(match app.view() {
        View::Active => 0,
        View::New => 1,
        View::Applied => 2,
        View::History => 3,
        View::Scans => 4,
        View::Sources => 5,
    }));
    frame.render_stateful_widget(list, area, &mut state);
}

fn nav_item(icon: &'static str, label: &'static str, color: Color) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(icon, Style::new().fg(color)),
        Span::raw(format!(" {label}")),
    ]))
}

fn render_job_list(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    let jobs = app.visible_jobs().collect::<Vec<_>>();
    let items = jobs.iter().map(|job| {
        let (primary_icon, primary_label, primary_color) = if job.source_open {
            (app.icons().open, "OPEN", theme.open)
        } else {
            (app.icons().history, "CLOSED", theme.muted_text)
        };
        let mut state = vec![Span::styled(
            format!("{primary_icon} {primary_label}"),
            Style::new().fg(primary_color),
        )];
        if job.is_new {
            state.push(Span::styled(
                format!("  {}", app.icons().new),
                Style::new().fg(theme.new),
            ));
        }
        if job.applied_at.is_some() {
            state.push(Span::styled(
                format!(" {}", app.icons().applied),
                Style::new().fg(theme.applied),
            ));
        }
        state.push(Span::styled(
            format!("  {}", compact_date(job.first_seen_at)),
            Style::new().fg(theme.muted_text),
        ));
        ListItem::new(vec![
            Line::from(state),
            Line::styled(
                format!(
                    "  {} · {}",
                    app.company_name(&job.key.company_id),
                    job.classified.observed.title
                ),
                Style::new().fg(theme.primary_text),
            ),
        ])
    });
    let title = match app.view() {
        View::Active => "Active jobs",
        View::New => "New jobs",
        View::Applied => "Applied jobs",
        View::History => "Job history",
        View::Scans | View::Sources => unreachable!("job list has fixed views"),
    };
    let list = List::new(items)
        .block(panel(
            title,
            theme.focused_border,
            theme.background,
            borders,
        ))
        .highlight_style(Style::new().bg(theme.selected_row))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    if !jobs.is_empty() {
        state.select(Some(app.selected_index()));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    let text = app.selected_job().map_or_else(
        || Text::from("No jobs in this view."),
        |job| {
            let observed = &job.classified.observed;
            let mut context = vec![app.company_name(&job.key.company_id).to_owned()];
            if !observed.locations.is_empty() {
                context.push(observed.locations.join(", "));
            }
            if let Some(department) = &observed.department {
                context.push(department.clone());
            }
            let mut lines = vec![
                Line::styled(observed.title.as_str(), Style::new().fg(theme.primary_text)),
                Line::styled(context.join(" · "), Style::new().fg(theme.muted_text)),
                Line::from(""),
            ];
            let mut status = vec![
                Span::styled("Status      ", Style::new().fg(theme.muted_text)),
                Span::styled(
                    format!(
                        "{} {}",
                        if job.source_open {
                            app.icons().open
                        } else {
                            app.icons().history
                        },
                        if job.source_open { "OPEN" } else { "CLOSED" }
                    ),
                    Style::new().fg(if job.source_open {
                        theme.open
                    } else {
                        theme.muted_text
                    }),
                ),
            ];
            if job.is_new {
                status.push(Span::styled(
                    format!(" · {} NEW", app.icons().new),
                    Style::new().fg(theme.new),
                ));
            }
            lines.push(Line::from(status));
            lines.push(metadata_line(
                "First seen  ",
                full_date(job.first_seen_at),
                theme.muted_text,
                theme.primary_text,
            ));
            lines.push(metadata_line(
                "Last seen   ",
                full_date(job.last_seen_at),
                theme.muted_text,
                theme.primary_text,
            ));
            if let Some(closed_at) = job.closed_at {
                lines.push(metadata_line(
                    "Closed      ",
                    full_date(closed_at),
                    theme.muted_text,
                    theme.muted_text,
                ));
            }
            if let Some(reopened_at) = job.reopened_at {
                lines.push(metadata_line(
                    "Reopened    ",
                    full_date(reopened_at),
                    theme.muted_text,
                    theme.open,
                ));
            }
            lines.push(metadata_line(
                "Applied     ",
                job.applied_at
                    .map(|at| format!("{} YES · {}", app.icons().applied, full_date(at)))
                    .unwrap_or_else(|| "No".to_owned()),
                theme.muted_text,
                if job.applied_at.is_some() {
                    theme.applied
                } else {
                    theme.primary_text
                },
            ));
            lines.push(Line::from(""));
            lines.push(Line::styled(
                observed.description.as_str(),
                Style::new().fg(theme.primary_text),
            ));
            Text::from(lines)
        },
    );
    let details = Paragraph::new(text)
        .block(panel(
            "Job details",
            if borders == Borders::ALL {
                theme.focused_border
            } else {
                theme.unfocused_border
            },
            theme.background,
            borders,
        ))
        .style(Style::new().bg(theme.background))
        .scroll((app.detail_scroll(), 0))
        .wrap(Wrap { trim: true });
    frame.render_widget(details, area);
}

fn metadata_line(
    label: &'static str,
    value: String,
    label_color: Color,
    value_color: Color,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(label, Style::new().fg(label_color)),
        Span::styled(value, Style::new().fg(value_color)),
    ])
}

fn render_scans(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    let items = app.scans().iter().map(|scan| {
        let (icon, label, color) = match scan.outcome {
            ScanOutcome::Complete => (app.icons().open, "COMPLETE", theme.open),
            ScanOutcome::Incomplete => (app.icons().source_failure, "INCOMPLETE", theme.warning),
            ScanOutcome::Failed => (app.icons().source_failure, "FAILED", theme.error),
        };
        let diagnostic = match (&scan.error_kind, &scan.diagnostic) {
            (Some(kind), Some(diagnostic)) => format!("{kind} · {diagnostic}"),
            (Some(kind), None) => kind.to_string(),
            (None, Some(diagnostic)) => diagnostic.clone(),
            (None, None) => "no diagnostic".to_owned(),
        };
        ListItem::new(vec![
            Line::from(vec![
                Span::styled(format!("{icon} {label}"), Style::new().fg(color)),
                Span::styled(
                    format!(
                        "  {}  {}  {} observed",
                        scan.company_name,
                        compact_time(scan.completed_at),
                        scan.observed_count
                    ),
                    Style::new().fg(theme.primary_text),
                ),
            ]),
            Line::styled(format!("  {diagnostic}"), Style::new().fg(color)),
        ])
    });
    let list = if app.scans().is_empty() {
        List::new(vec![ListItem::new("No scan history yet.")])
    } else {
        List::new(items.collect::<Vec<_>>())
    }
    .block(panel(
        "Recent scans",
        theme.focused_border,
        theme.background,
        borders,
    ))
    .highlight_style(Style::new().bg(theme.selected_row))
    .highlight_symbol("› ");
    let mut state = ListState::default();
    if !app.scans().is_empty() {
        state.select(Some(app.selected_index()));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_sources(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    let items = app.sources().iter().map(|source| {
        let (icon, label, color) = match source.health {
            SourceHealth::Unknown => (app.icons().history, "UNKNOWN", theme.muted_text),
            SourceHealth::Healthy => (app.icons().open, "HEALTHY", theme.open),
            SourceHealth::Incomplete => (app.icons().source_failure, "INCOMPLETE", theme.warning),
            SourceHealth::Failed => (app.icons().source_failure, "FAILED", theme.error),
        };
        let state = if source.enabled {
            "enabled"
        } else {
            "disabled"
        };
        let diagnostic = match (&source.latest_error_kind, &source.diagnostic) {
            (Some(kind), Some(diagnostic)) => format!("{kind} · {diagnostic}"),
            (Some(kind), None) => kind.to_string(),
            (None, Some(diagnostic)) => diagnostic.clone(),
            (None, None) => "no diagnostic".to_owned(),
        };
        ListItem::new(vec![
            Line::from(vec![
                Span::styled(format!("{icon} {label}"), Style::new().fg(color)),
                Span::styled(
                    format!("  {} · {state}", source.company_name),
                    Style::new().fg(theme.primary_text),
                ),
            ]),
            Line::styled(
                format!(
                    "  last attempt {} · last success {}",
                    optional_time(source.latest_attempted_at),
                    optional_time(source.latest_successful_at)
                ),
                Style::new().fg(theme.muted_text),
            ),
            Line::styled(format!("  {diagnostic}"), Style::new().fg(color)),
        ])
    });
    let list = if app.sources().is_empty() {
        List::new(vec![ListItem::new("No configured sources.")])
    } else {
        List::new(items.collect::<Vec<_>>())
    }
    .block(panel(
        "Sources",
        theme.focused_border,
        theme.background,
        borders,
    ))
    .highlight_style(Style::new().bg(theme.selected_row))
    .highlight_symbol("› ");
    let mut state = ListState::default();
    if !app.sources().is_empty() {
        state.select(Some(app.selected_index()));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let keys = &app.config().keybindings;
    let status = app.footer_status();
    let (icon, status_color) = if status.starts_with("FAILED") {
        (app.icons().source_failure, theme.error)
    } else if status.starts_with("INCOMPLETE") {
        (app.icons().source_failure, theme.warning)
    } else if status.starts_with("SCANNING") {
        (app.icons().scanning, theme.warning)
    } else {
        (app.icons().open, theme.open)
    };
    let actions = if app.input_mode() == InputMode::Search {
        let limit = if area.width < 80 { 12 } else { 32 };
        let query = app.search_query().chars().take(limit).collect::<String>();
        vec![
            format!("SEARCH {query}"),
            "Enter apply".to_owned(),
            "Esc clear".to_owned(),
            format!("{} help", keys.help),
        ]
    } else if matches!(app.view(), View::Scans | View::Sources) {
        let mut actions = vec![format!("{} scan", keys.scan), "←/→ views".to_owned()];
        if area.width >= 80 {
            actions.push(format!("{} quit", keys.quit));
        }
        actions.push(format!("{} help", keys.help));
        actions
    } else if area.width < 80 {
        if app.narrow_details_visible() {
            vec![
                format!("{} open", keys.open),
                format!("{} applied", keys.toggle_applied),
                format!("{} help", keys.help),
            ]
        } else {
            vec![
                format!("{} scan", keys.scan),
                format!("{} search", keys.search),
                format!("{} help", keys.help),
            ]
        }
    } else {
        let mut actions = vec![
            format!("{} scan", keys.scan),
            format!("{} search", keys.search),
            format!("{} applied", keys.toggle_applied),
            format!("{} open", keys.open),
        ];
        if area.width >= 100 {
            let enabled = app
                .config()
                .companies
                .iter()
                .filter(|company| company.enabled)
                .count();
            actions.push(format!("{} filter {}", keys.filter, app.filter_label()));
            actions.push(format!(
                "{enabled} companies · {} active jobs",
                app.active_job_count()
            ));
        }
        if area.width >= 120 {
            actions.push(format!("{} quit", keys.quit));
        }
        actions.push(format!("{} help", keys.help));
        actions
    };
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{icon} {status}"), Style::new().fg(status_color)),
            Span::styled(
                format!("  {}", actions.join("  ")),
                Style::new().fg(theme.muted_text),
            ),
        ]))
        .style(Style::new().bg(theme.background)),
        area,
    );
}

fn render_help(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let keys = &app.config().keybindings;
    let margin = Margin {
        horizontal: (area.width / 10).max(2),
        vertical: (area.height / 8).max(1),
    };
    let overlay = area.inner(margin);
    let text = format!(
        "←/→ views  Active/New/Applied/History/Scans/Sources\n\
         ↑/↓ or j/k select\n\
         J/K scroll details\n\n\
         {} scan  {} search jobs  {} filter company/New/Applied\n\
         {} applied  {} open\n\
         Enter narrow: details; otherwise: open\n\
         Search: title, company, posting; Enter accept; Esc clear\n\
         Esc close help  {} toggle help  {} quit",
        keys.scan, keys.search, keys.filter, keys.toggle_applied, keys.open, keys.help, keys.quit,
    );
    frame.render_widget(Clear, overlay);
    frame.render_widget(
        Paragraph::new(text)
            .block(panel(
                "Help",
                theme.focused_border,
                theme.background,
                Borders::ALL,
            ))
            .style(Style::new().fg(theme.primary_text).bg(theme.background))
            .wrap(Wrap { trim: true }),
        overlay,
    );
}

fn compact_date(value: DateTime<Utc>) -> String {
    value.format("%d %b").to_string()
}

fn full_date(value: DateTime<Utc>) -> String {
    value.format("%d %b %Y %H:%M UTC").to_string()
}

fn compact_time(value: DateTime<Utc>) -> String {
    value.format("%d %b %H:%M").to_string()
}

fn optional_time(value: Option<DateTime<Utc>>) -> String {
    value.map_or_else(|| "never".to_owned(), compact_time)
}

fn panel(title: &str, border: Color, background: Color, borders: Borders) -> Block<'_> {
    Block::new()
        .title(title)
        .borders(borders)
        .border_style(Style::new().fg(border))
        .style(Style::new().bg(background))
}
