use chrono::{DateTime, Utc};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span, Text},
    widgets::{
        Bar, BarChart, Block, Borders, Cell, Clear, List, ListItem, ListState, Paragraph, Row,
        Scrollbar, ScrollbarOrientation, ScrollbarState, Table, TableState, Wrap,
    },
};

use crate::{
    analytics::SkillKind,
    insights::{MetricRow, Momentum},
    storage::{ScanOutcome, SourceHealth},
};

use super::{
    AnalyticsTab, App, Focus, InputMode, LibraryTab, MarketSection, MouseTarget, Setting, View,
};

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
    if app.focus() == Focus::Navigation && area.width < 120 {
        render_navigation(frame, app, area);
        return;
    }
    if app.view() == View::Settings {
        if area.width >= 120 {
            let areas =
                Layout::horizontal([Constraint::Length(22), Constraint::Fill(1)]).split(area);
            render_navigation(frame, app, areas[0]);
            render_settings(
                frame,
                app,
                areas[1],
                Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
            );
        } else {
            render_settings(frame, app, area, Borders::ALL);
        }
        return;
    }
    if matches!(app.view(), View::Scans | View::Sources) {
        render_operational_view(frame, app, area);
        return;
    }
    if app.view() == View::Analytics {
        render_analytics_view(frame, app, area);
        return;
    }
    if app.view() == View::Library {
        render_library_view(frame, app, area);
        return;
    }
    match area.width {
        120.. => render_navigation_list_details(frame, app, area),
        80..=119 => render_list_details(frame, app, area),
        _ => render_single_job_pane(frame, app, area),
    }
}

fn render_navigation_list_details(frame: &mut Frame, app: &App, area: Rect) {
    let (list, details) = app.job_panes(area).expect("wide job panes");
    render_navigation(frame, app, Rect::new(area.x, area.y, 22, area.height));
    render_job_list(
        frame,
        app,
        list,
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
    );
    render_details(
        frame,
        app,
        details,
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
    );
    render_job_divider(frame, app, list);
}

fn render_list_details(frame: &mut Frame, app: &App, area: Rect) {
    let (list, details) = app.job_panes(area).expect("medium job panes");
    render_job_list(frame, app, list, Borders::ALL);
    render_details(
        frame,
        app,
        details,
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
    );
    render_job_divider(frame, app, list);
}

fn render_job_divider(frame: &mut Frame, app: &App, list: Rect) {
    let color = if app.pressed(MouseTarget::Divider) {
        app.theme().warning
    } else if app.hovered(MouseTarget::Divider) {
        app.theme().new
    } else {
        return;
    };
    frame.render_widget(
        Block::new()
            .borders(Borders::RIGHT)
            .border_style(Style::new().fg(color)),
        Rect::new(
            list.right().saturating_sub(1),
            list.y.saturating_add(1),
            1,
            list.height.saturating_sub(2),
        ),
    );
}

fn render_analytics_view(frame: &mut Frame, app: &App, area: Rect) {
    let surface = if area.width >= 120 {
        render_navigation(frame, app, Rect::new(area.x, area.y, 22, area.height));
        Rect::new(
            area.x + 22,
            area.y,
            area.width.saturating_sub(22),
            area.height,
        )
    } else {
        area
    };
    let shell = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(surface);
    render_analytics_tabs(frame, app, shell[0]);
    render_analytics_filters(frame, app, shell[1]);

    if surface.width < 90 && app.narrow_details_visible() {
        render_analytics_evidence(frame, app, shell[2], Borders::ALL);
        return;
    }
    if surface.width >= 90 {
        let panes =
            Layout::horizontal([Constraint::Percentage(64), Constraint::Fill(1)]).split(shell[2]);
        render_analytics_tab(frame, app, panes[0]);
        render_analytics_evidence(
            frame,
            app,
            panes[1],
            Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
        );
    } else {
        render_analytics_tab(frame, app, shell[2]);
    }
}

fn render_analytics_tabs(frame: &mut Frame, app: &App, area: Rect) {
    let spans = [
        AnalyticsTab::Overview,
        AnalyticsTab::Skills,
        AnalyticsTab::Stacks,
        AnalyticsTab::Market,
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, tab)| {
        let selected = tab == app.analytics_tab();
        [
            Span::styled(
                format!(" {} {} ", index + 1, tab.label()),
                Style::new()
                    .fg(if selected {
                        app.theme().warning
                    } else {
                        app.theme().primary_text
                    })
                    .bg(if selected {
                        app.theme().selected_row
                    } else {
                        app.theme().background
                    })
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            ),
            Span::raw(" "),
        ]
    })
    .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Line::from(spans)).style(Style::new().bg(app.theme().background)),
        area,
    );
}

fn render_analytics_filters(frame: &mut Frame, app: &App, area: Rect) {
    let filters = app.analytics_filters();
    let Some(report) = app.analytics_report() else {
        let status = app
            .analytics_error()
            .unwrap_or("Preparing analytics in background…");
        frame.render_widget(
            Paragraph::new(status).style(
                Style::new()
                    .fg(app.theme().muted_text)
                    .bg(app.theme().background),
            ),
            area,
        );
        return;
    };
    let refresh = if app.data_loading() {
        "Loading data… · "
    } else if app.analytics_refreshing() {
        "Refreshing… · "
    } else {
        ""
    };
    let text = format!(
        "{refresh}{}d t/± · Company {} C · Role {} R · Level {} S · Work {} W · x clear · comparable {} firms · new {}/{} jobs",
        filters.window_days,
        filters
            .company
            .as_deref()
            .map(|company| app.company_name(company))
            .unwrap_or("All"),
        filters.role.as_deref().unwrap_or("All"),
        filters
            .seniority
            .map(crate::insights::seniority_name)
            .unwrap_or("All"),
        filters
            .work_mode
            .map(crate::insights::work_mode_name)
            .unwrap_or("All"),
        report.comparable_company_count,
        report.period_job_count,
        report.previous_job_count,
    );
    frame.render_widget(
        Paragraph::new(text).style(
            Style::new()
                .fg(app.theme().muted_text)
                .bg(app.theme().background),
        ),
        area,
    );
}

fn render_analytics_tab(frame: &mut Frame, app: &App, area: Rect) {
    match app.analytics_tab() {
        AnalyticsTab::Overview => render_overview(frame, app, area),
        AnalyticsTab::Skills => render_skills_trends(frame, app, area),
        AnalyticsTab::Stacks => render_stacks_trends(frame, app, area),
        AnalyticsTab::Market => render_market_trends(frame, app, area),
    }
}

fn render_overview(frame: &mut Frame, app: &App, area: Rect) {
    let Some(report) = app.analytics_report() else {
        render_analytics_loading(frame, app, area);
        return;
    };
    let sections = Layout::vertical([Constraint::Percentage(48), Constraint::Fill(1)]).split(area);
    let charts =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Fill(1)]).split(sections[0]);
    let hard = report
        .hard_skills
        .iter()
        .map(|skill| &skill.metric)
        .collect::<Vec<_>>();
    let roles = report.roles.iter().collect::<Vec<_>>();
    render_metric_chart(frame, app, charts[0], "Hard-skill demand", &hard);
    render_metric_chart(frame, app, charts[1], "Role demand", &roles);

    let rows = report.recommendations.iter().map(|item| {
        Row::new(vec![
            Cell::from(if item.saved { "★" } else { " " }),
            Cell::from(item.skill.clone()),
            Cell::from(item.kind.as_str()),
            Cell::from(item.demand_count.to_string()),
            Cell::from(item.target_role_count.to_string()),
            Cell::from(item.adjacent_known_count.to_string()),
            Cell::from(item.momentum.as_str()),
            Cell::from(item.confidence.as_str()),
            Cell::from(item.reason.clone()),
        ])
    });
    let title = format!(
        "Career opportunities · {} recommendations · {} active jobs · history since {}",
        report.recommendations.len(),
        report.active_job_count,
        report
            .earliest_observation
            .map(|date| date.format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "unknown".into())
    );
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Fill(2),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Length(12),
            Constraint::Length(5),
            Constraint::Fill(3),
        ],
    )
    .header(table_header([
        "",
        "Learn next",
        "Type",
        "Demand",
        "Target",
        "Beside",
        "Momentum",
        "Conf",
        "Why",
    ]))
    .block(panel(
        &title,
        app.theme().focused_border,
        app.theme().background,
        Borders::ALL,
    ))
    .row_highlight_style(Style::new().bg(app.theme().selected_row))
    .highlight_symbol("› ");
    let mut state = TableState::default().with_selected(app.selected_index());
    frame.render_stateful_widget(table, sections[1], &mut state);
    render_scrollbar(
        frame,
        app,
        sections[1],
        report.recommendations.len(),
        state.offset(),
        usize::from(sections[1].height.saturating_sub(3)).max(1),
    );
}

fn render_skills_trends(frame: &mut Frame, app: &App, area: Rect) {
    let Some(report) = app.analytics_report() else {
        render_analytics_loading(frame, app, area);
        return;
    };
    let sections = Layout::vertical([Constraint::Percentage(68), Constraint::Fill(1)]).split(area);
    let tables =
        Layout::horizontal([Constraint::Percentage(50), Constraint::Fill(1)]).split(sections[0]);
    render_skill_table(
        frame,
        app,
        tables[0],
        "Hard skills",
        &report.hard_skills,
        SkillKind::Hard,
    );
    render_skill_table(
        frame,
        app,
        tables[1],
        "Soft skills",
        &report.soft_skills,
        SkillKind::Soft,
    );
    let metrics = match app.analytics_skill_kind() {
        SkillKind::Hard => report
            .hard_skills
            .iter()
            .map(|skill| &skill.metric)
            .collect::<Vec<_>>(),
        SkillKind::Soft => report
            .soft_skills
            .iter()
            .map(|skill| &skill.metric)
            .collect::<Vec<_>>(),
    };
    render_metric_chart(frame, app, sections[1], "Demand chart", &metrics);
}

fn render_skill_table(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    title: &str,
    skills: &[crate::insights::SkillTrend],
    kind: SkillKind,
) {
    let rows = skills.iter().map(|skill| {
        Row::new(vec![
            Cell::from(if skill.saved { "★" } else { " " }),
            Cell::from(skill.metric.name.clone()),
            Cell::from(format_demand(&skill.metric)),
            Cell::from(format!("{:+}", skill.metric.delta_count)),
            Cell::from(format_delta(&skill.metric)),
            Cell::from(skill.metric.momentum.as_str()),
            Cell::from(
                skill
                    .status
                    .map_or("—", crate::insights::SkillStatus::as_str),
            ),
        ])
    });
    let selected = app.analytics_skill_kind() == kind;
    let panel_title = format!("{title} · {}", skills.len());
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Fill(2),
            Constraint::Length(11),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(table_header([
        "", "Skill", "Demand", "Δ jobs", "Δ share", "Momentum", "Status",
    ]))
    .block(panel(
        &panel_title,
        if selected {
            app.theme().focused_border
        } else {
            app.theme().unfocused_border
        },
        app.theme().background,
        Borders::ALL,
    ))
    .row_highlight_style(Style::new().bg(app.theme().selected_row))
    .highlight_symbol("› ");
    let mut state = TableState::default();
    if selected && !skills.is_empty() {
        state.select(Some(app.selected_index()));
    }
    frame.render_stateful_widget(table, area, &mut state);
    render_scrollbar(
        frame,
        app,
        area,
        skills.len(),
        state.offset(),
        usize::from(area.height.saturating_sub(3)).max(1),
    );
}

fn render_stacks_trends(frame: &mut Frame, app: &App, area: Rect) {
    let Some(report) = app.analytics_report() else {
        render_analytics_loading(frame, app, area);
        return;
    };
    let sections = Layout::vertical([Constraint::Percentage(68), Constraint::Fill(1)]).split(area);
    let rows = report.stacks.iter().map(|stack| {
        Row::new(vec![
            Cell::from(if stack.saved { "★" } else { " " }),
            Cell::from(format!(
                "{} · {}",
                stack.profile.label(),
                stack.path_label()
            )),
            Cell::from(format_demand(&stack.metric)),
            Cell::from(stack.company_count.to_string()),
            Cell::from(format!(
                "{}.{:02}×",
                stack.association_bps / 100,
                stack.association_bps % 100
            )),
            Cell::from(format_delta(&stack.metric)),
            Cell::from(stack.metric.momentum.as_str()),
            Cell::from(stack.metric.confidence.as_str()),
        ])
    });
    let title = format!(
        "Technology stacks · {} paths · minimum {} jobs · 3+ architectural roles",
        report.stacks.len(),
        app.config().analytics.minimum_cooccurrence
    );
    let table = Table::new(
        rows,
        [
            Constraint::Length(2),
            Constraint::Fill(2),
            Constraint::Length(11),
            Constraint::Length(7),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(5),
        ],
    )
    .header(table_header([
        "", "Path", "Demand", "Firms", "Link", "Δ share", "Momentum", "Conf",
    ]))
    .block(panel(
        &title,
        app.theme().focused_border,
        app.theme().background,
        Borders::ALL,
    ))
    .row_highlight_style(Style::new().bg(app.theme().selected_row))
    .highlight_symbol("› ");
    let mut state = TableState::default().with_selected(app.selected_index());
    frame.render_stateful_widget(table, sections[0], &mut state);
    let Some(stack) = report.stacks.get(app.selected_index()) else {
        frame.render_widget(
            Paragraph::new("No evidence-backed paths for these filters.")
                .block(panel(
                    "Selected stack graph",
                    app.theme().unfocused_border,
                    app.theme().background,
                    Borders::ALL,
                ))
                .style(Style::new().fg(app.theme().muted_text)),
            sections[1],
        );
        return;
    };
    let mut path = Vec::new();
    for (index, skill) in stack.path.iter().enumerate() {
        if index > 0 {
            path.push(Span::styled(
                " ── ",
                Style::new().fg(app.theme().muted_text),
            ));
        }
        path.push(Span::styled(
            format!("● {skill}"),
            Style::new().fg(app.theme().focused_border),
        ));
    }
    let association = format!(
        "{} stack · {}.{:02}× association · {} jobs · {} firms · {}",
        stack.profile.label(),
        stack.association_bps / 100,
        stack.association_bps % 100,
        stack.metric.current_count,
        stack.company_count,
        stack.metric.momentum.as_str()
    );
    let graph_title = format!("Selected {} stack", stack.profile.label().to_lowercase());
    frame.render_widget(
        Paragraph::new(Text::from(vec![
            Line::from(path),
            Line::from(""),
            Line::from(association),
        ]))
        .block(panel(
            &graph_title,
            app.theme().unfocused_border,
            app.theme().background,
            Borders::ALL,
        ))
        .wrap(Wrap { trim: false }),
        sections[1],
    );
}

fn render_market_trends(frame: &mut Frame, app: &App, area: Rect) {
    if app.analytics_report().is_none() {
        render_analytics_loading(frame, app, area);
        return;
    }
    let sections = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).split(area);
    let labels = [
        MarketSection::Roles,
        MarketSection::Seniority,
        MarketSection::Experience,
        MarketSection::Work,
        MarketSection::Companies,
    ]
    .into_iter()
    .map(|section| {
        Span::styled(
            format!(" {} ", section.label()),
            Style::new()
                .fg(if section == app.market_section() {
                    app.theme().warning
                } else {
                    app.theme().primary_text
                })
                .bg(if section == app.market_section() {
                    app.theme().selected_row
                } else {
                    app.theme().background
                }),
        )
    })
    .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(labels)), sections[0]);
    let panes =
        Layout::horizontal([Constraint::Percentage(62), Constraint::Fill(1)]).split(sections[1]);
    let rows = app.market_rows();
    render_metric_table(frame, app, panes[0], app.market_section().label(), &rows);
    let refs = rows.iter().collect::<Vec<_>>();
    render_metric_chart(frame, app, panes[1], "Market shape", &refs);
}

fn render_metric_table(frame: &mut Frame, app: &App, area: Rect, title: &str, rows: &[MetricRow]) {
    let table_rows = rows.iter().map(|metric| {
        Row::new(vec![
            Cell::from(format!(
                "{}{}",
                if market_metric_saved(app, metric) {
                    "★ "
                } else {
                    ""
                },
                metric_display_name(app, metric)
            )),
            Cell::from(format_demand(metric)),
            Cell::from(format!("{:+}", metric.delta_count)),
            Cell::from(format_delta(metric)),
            Cell::from(metric.momentum.as_str()),
            Cell::from(metric.confidence.as_str()),
        ])
    });
    let panel_title = format!("{title} · {}", rows.len());
    let table = Table::new(
        table_rows,
        [
            Constraint::Fill(2),
            Constraint::Length(11),
            Constraint::Length(7),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(5),
        ],
    )
    .header(table_header([
        "Name", "Demand", "Δ jobs", "Δ share", "Momentum", "Conf",
    ]))
    .block(panel(
        &panel_title,
        app.theme().focused_border,
        app.theme().background,
        Borders::ALL,
    ))
    .row_highlight_style(Style::new().bg(app.theme().selected_row))
    .highlight_symbol("› ");
    let mut state = TableState::default().with_selected(app.selected_index());
    frame.render_stateful_widget(table, area, &mut state);
    render_scrollbar(
        frame,
        app,
        area,
        rows.len(),
        state.offset(),
        usize::from(area.height.saturating_sub(3)).max(1),
    );
}

fn render_metric_chart(frame: &mut Frame, app: &App, area: Rect, title: &str, rows: &[&MetricRow]) {
    let bars = rows
        .iter()
        .take(8.min(usize::from(area.height.saturating_sub(2) / 2).max(1)))
        .map(|metric| {
            Bar::with_label(
                metric_display_name(app, metric),
                metric.current_count as u64,
            )
            .text_value(String::new())
            .style(Style::new().fg(momentum_color(app, metric.momentum)))
        })
        .collect::<Vec<_>>();
    if bars.is_empty() {
        frame.render_widget(
            Paragraph::new("No data for this filter.").block(panel(
                title,
                app.theme().unfocused_border,
                app.theme().background,
                Borders::ALL,
            )),
            area,
        );
        return;
    }
    let max = rows.iter().map(|row| row.current_count).max().unwrap_or(1);
    let mut chart = BarChart::horizontal(bars)
        .block(panel(
            title,
            app.theme().unfocused_border,
            app.theme().background,
            Borders::ALL,
        ))
        .bar_width(1)
        .bar_gap(1)
        .max(max.max(1) as u64)
        .label_style(Style::new().fg(app.theme().primary_text));
    if !app.config().ui.unicode_icons {
        chart = chart.bar_set(ascii_bar_set());
    }
    frame.render_widget(chart, area);
}

fn render_analytics_evidence(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let jobs = app.analytics_evidence_jobs();
    let coverage = app.analytics_coverage();
    let items = jobs.iter().map(|job| {
        let status = if job.source_open { "OPEN" } else { "CLOSED" };
        let evidence = app.analytics_evidence_text(job);
        ListItem::new(vec![
            Line::styled(
                format!(
                    "• {} · {}",
                    app.company_name(&job.key.company_id),
                    job.classified.observed.title
                ),
                Style::new().fg(app.theme().primary_text),
            ),
            Line::styled(
                format!("  {status} — {}", truncate(&evidence, 120)),
                Style::new().fg(app.theme().muted_text),
            ),
        ])
    });
    let title = format!(
        "Evidence · {} matching jobs · descriptions {}/{} · sources {}/{} healthy",
        jobs.len(),
        coverage.descriptions,
        coverage.total,
        coverage.healthy_sources,
        coverage.enabled_sources,
    );
    let list = List::new(items)
        .block(panel(
            &title,
            app.theme().unfocused_border,
            app.theme().background,
            borders,
        ))
        .highlight_style(Style::new().bg(app.theme().selected_row))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    if !jobs.is_empty() {
        state.select(Some(app.evidence_index().min(jobs.len() - 1)));
    }
    frame.render_stateful_widget(list, area, &mut state);
    render_scrollbar(
        frame,
        app,
        area,
        jobs.len(),
        state.offset(),
        usize::from(area.height.saturating_sub(2) / 2).max(1),
    );
}

fn render_analytics_loading(frame: &mut Frame, app: &App, area: Rect) {
    let text = app
        .analytics_error()
        .unwrap_or("Calculating trends, stacks, and evidence in the background…");
    frame.render_widget(
        Paragraph::new(text)
            .block(panel(
                "Analytics",
                app.theme().focused_border,
                app.theme().background,
                Borders::ALL,
            ))
            .style(Style::new().fg(app.theme().muted_text)),
        area,
    );
}

fn render_library_view(frame: &mut Frame, app: &App, area: Rect) {
    let surface = if area.width >= 120 {
        render_navigation(frame, app, Rect::new(area.x, area.y, 22, area.height));
        Rect::new(
            area.x + 22,
            area.y,
            area.width.saturating_sub(22),
            area.height,
        )
    } else {
        area
    };
    let sections = Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).split(surface);
    let tabs = [
        LibraryTab::Jobs,
        LibraryTab::Skills,
        LibraryTab::Stacks,
        LibraryTab::Roles,
        LibraryTab::Companies,
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(index, tab)| {
        let selected = tab == app.library_tab();
        [
            Span::styled(
                format!(" {} {} ", index + 1, tab.label()),
                Style::new()
                    .fg(if selected {
                        app.theme().warning
                    } else {
                        app.theme().primary_text
                    })
                    .bg(if selected {
                        app.theme().selected_row
                    } else {
                        app.theme().background
                    }),
            ),
            Span::raw(" "),
        ]
    })
    .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Line::from(tabs)), sections[0]);
    match app.library_tab() {
        LibraryTab::Jobs => render_library_jobs(frame, app, sections[1]),
        LibraryTab::Skills => {
            let suggestions = app.pending_skill_suggestions();
            let values = app.library_skills();
            let rows = suggestions
                .iter()
                .map(|item| {
                    Row::new(vec![
                        Cell::from("?"),
                        Cell::from(item.name.clone()),
                        Cell::from(item.kind.as_str()),
                        Cell::from("review: a approve / d reject"),
                        Cell::from(item.evidence.first().cloned().unwrap_or_default()),
                    ])
                })
                .chain(values.iter().map(|(name, status)| {
                    Row::new(vec![
                        Cell::from("★"),
                        Cell::from((*name).to_owned()),
                        Cell::from("saved"),
                        Cell::from(status.map_or("—", crate::insights::SkillStatus::as_str)),
                        Cell::from(""),
                    ])
                }));
            let title = format!(
                "Skills · AI suggestions require review · {}",
                suggestions.len() + values.len()
            );
            render_library_table(
                frame,
                app,
                sections[1],
                &title,
                rows,
                [
                    Constraint::Length(2),
                    Constraint::Length(24),
                    Constraint::Length(7),
                    Constraint::Length(28),
                    Constraint::Fill(1),
                ],
                ["", "Skill", "Type", "Status", "Evidence"],
            );
        }
        LibraryTab::Stacks => {
            let values = app.library_stacks();
            let rows = values
                .iter()
                .map(|stack| Row::new(vec![Cell::from("★"), Cell::from(stack.label())]));
            let title = format!("Saved stacks · {}", values.len());
            render_library_table(
                frame,
                app,
                sections[1],
                &title,
                rows,
                [Constraint::Length(2), Constraint::Fill(1)],
                ["", "Stack"],
            );
        }
        LibraryTab::Roles => {
            let values = app.library_roles();
            let rows = values.iter().map(|(role, target)| {
                Row::new(vec![
                    Cell::from("★"),
                    Cell::from((*role).to_owned()),
                    Cell::from(if *target { "Target" } else { "Saved" }),
                ])
            });
            let title = format!("Saved roles · {}", values.len());
            render_library_table(
                frame,
                app,
                sections[1],
                &title,
                rows,
                [
                    Constraint::Length(2),
                    Constraint::Fill(1),
                    Constraint::Length(10),
                ],
                ["", "Role", "Purpose"],
            );
        }
        LibraryTab::Companies => {
            let values = app.library_companies();
            let rows = values.iter().map(|company| {
                Row::new(vec![
                    Cell::from("★"),
                    Cell::from(app.company_name(company).to_owned()),
                ])
            });
            let title = format!("Saved companies · {}", values.len());
            render_library_table(
                frame,
                app,
                sections[1],
                &title,
                rows,
                [Constraint::Length(2), Constraint::Fill(1)],
                ["", "Company"],
            );
        }
    }
}

fn render_library_jobs(frame: &mut Frame, app: &App, area: Rect) {
    let jobs = app.library_jobs();
    let items = jobs.iter().map(|job| {
        let status = if job.source_open {
            if job.reopened_at.is_some() {
                "REOPENED"
            } else {
                "OPEN"
            }
        } else {
            "CLOSED"
        };
        ListItem::new(vec![
            Line::styled(
                format!("★ {status} · {}", app.company_name(&job.key.company_id)),
                Style::new().fg(app.theme().new),
            ),
            Line::styled(
                format!("  {}", job.classified.observed.title),
                Style::new().fg(app.theme().primary_text),
            ),
        ])
    });
    let title = format!("Saved jobs · {}", jobs.len());
    let list = List::new(items)
        .block(panel(
            &title,
            app.theme().focused_border,
            app.theme().background,
            Borders::ALL,
        ))
        .highlight_style(Style::new().bg(app.theme().selected_row))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    if !jobs.is_empty() {
        state.select(Some(app.selected_index()));
    }
    frame.render_stateful_widget(list, area, &mut state);
    render_scrollbar(
        frame,
        app,
        area,
        jobs.len(),
        state.offset(),
        usize::from(area.height.saturating_sub(2) / 2).max(1),
    );
}

fn render_library_table<'a, const W: usize, const H: usize>(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    panel_title: &str,
    rows: impl Iterator<Item = Row<'a>>,
    widths: [Constraint; W],
    headers: [&str; H],
) {
    let rows = rows.collect::<Vec<_>>();
    let count = rows.len();
    let table = Table::new(rows, widths)
        .header(table_header(headers))
        .block(panel(
            panel_title,
            app.theme().focused_border,
            app.theme().background,
            Borders::ALL,
        ))
        .row_highlight_style(Style::new().bg(app.theme().selected_row))
        .highlight_symbol("› ");
    let mut state = TableState::default();
    if count > 0 {
        state.select(Some(app.selected_index()));
    }
    frame.render_stateful_widget(table, area, &mut state);
    render_scrollbar(
        frame,
        app,
        area,
        count,
        state.offset(),
        usize::from(area.height.saturating_sub(3)).max(1),
    );
}

fn table_header<const N: usize>(labels: [&str; N]) -> Row<'static> {
    Row::new(
        labels
            .into_iter()
            .map(|label| Cell::from(label.to_owned()))
            .collect::<Vec<_>>(),
    )
    .style(Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD))
}

fn format_demand(metric: &MetricRow) -> String {
    format!(
        "{} · {}%",
        metric.current_count,
        metric.current_share_per_mille / 10
    )
}

fn format_delta(metric: &MetricRow) -> String {
    format!("{:+.1}pp", metric.delta_share_per_mille as f64 / 10.0)
}

fn momentum_color(app: &App, momentum: Momentum) -> Color {
    match momentum {
        Momentum::New | Momentum::Rising => app.theme().new,
        Momentum::Stable => app.theme().open,
        Momentum::Falling => app.theme().error,
        Momentum::LowConfidence => app.theme().muted_text,
    }
}

fn metric_display_name(app: &App, metric: &MetricRow) -> String {
    if app.analytics_tab() == AnalyticsTab::Market
        && app.market_section() == MarketSection::Companies
    {
        app.company_name(&metric.name).to_owned()
    } else {
        metric.name.clone()
    }
}

fn market_metric_saved(app: &App, metric: &MetricRow) -> bool {
    app.analytics_tab() == AnalyticsTab::Market
        && match app.market_section() {
            MarketSection::Roles => app.library().roles.contains_key(&metric.name),
            MarketSection::Companies => app.library().companies.contains(&metric.name),
            MarketSection::Seniority | MarketSection::Experience | MarketSection::Work => false,
        }
}

fn truncate(value: &str, maximum: usize) -> String {
    let mut characters = value.chars();
    let text = characters.by_ref().take(maximum).collect::<String>();
    if characters.next().is_some() {
        format!("{text}…")
    } else {
        text
    }
}

#[cfg(any())]
fn render_analytics_list(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let [hard, soft] = analytics_skill_panes(area);
    render_skill_pane(frame, app, hard, borders, SkillKind::Hard);
    render_skill_pane(
        frame,
        app,
        soft,
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
        SkillKind::Soft,
    );
}

#[cfg(any())]
fn render_skill_pane(frame: &mut Frame, app: &App, area: Rect, borders: Borders, kind: SkillKind) {
    let theme = app.theme();
    let stats = app.skill_stats_for(kind);
    let total = app.analytics_job_count();
    let matched_jobs = app.analytics_skill_job_count(kind);
    let coverage = app.analytics_coverage();
    let heading = match kind {
        SkillKind::Hard => "Hard skills",
        SkillKind::Soft => "Soft skills",
    };
    let title = if area.width >= 32 {
        format!("{heading} · {} · jobs {matched_jobs}/{total}", stats.len())
    } else {
        let heading = match kind {
            SkillKind::Hard => "Hard",
            SkillKind::Soft => "Soft",
        };
        format!("{heading} · {} · {matched_jobs}/{total} jobs", stats.len())
    };
    let block = panel(
        &title,
        if app.focus() == Focus::Content && app.analytics_skill_kind() == kind {
            theme.focused_border
        } else {
            theme.unfocused_border
        },
        theme.background,
        borders,
    )
    .title_bottom(format!(
        "descriptions {}/{}",
        coverage.descriptions, coverage.total
    ));
    if stats.is_empty() {
        frame.render_widget(
            Paragraph::new("No banked skills found in observed job descriptions.")
                .block(block)
                .style(Style::new().bg(theme.background)),
            area,
        );
        return;
    }

    let visible_count = usize::from(area.height.saturating_sub(2) / 2).max(1);
    let selected_index = app.analytics_skill_index(kind);
    let first = selected_index.saturating_sub(visible_count.saturating_sub(1));
    let bars = stats
        .iter()
        .enumerate()
        .skip(first)
        .take(visible_count)
        .map(|(index, stat)| {
            let percent = stat.job_count * 100 / total.max(1);
            let selected = index == selected_index && app.analytics_skill_kind() == kind;
            let target = match kind {
                SkillKind::Hard => MouseTarget::HardSkill(index),
                SkillKind::Soft => MouseTarget::SoftSkill(index),
            };
            let style = if selected {
                Style::new().fg(theme.warning).bg(theme.selected_row)
            } else {
                Style::new().fg(theme.new).patch(mouse_style(app, target))
            };
            Bar::with_label(
                format!("{}  {} jobs · {percent}%", stat.name, stat.job_count),
                stat.job_count as u64,
            )
            .text_value(String::new())
            .style(style)
            .value_style(
                Style::new()
                    .fg(theme.primary_text)
                    .add_modifier(if selected {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )
        })
        .collect::<Vec<_>>();
    let inner = area.inner(Margin::new(1, 1));
    for (offset, index) in (first..first + bars.len()).enumerate() {
        let target = match kind {
            SkillKind::Hard => MouseTarget::HardSkill(index),
            SkillKind::Soft => MouseTarget::SoftSkill(index),
        };
        let background = if app.pressed(target)
            || (index == selected_index && app.analytics_skill_kind() == kind)
        {
            Some(theme.selected_row)
        } else if app.hovered(target) {
            Some(theme.hovered_row)
        } else {
            None
        };
        if let Some(background) = background {
            frame.render_widget(
                Block::new().style(Style::new().bg(background)),
                Rect::new(
                    inner.x,
                    inner.y.saturating_add(offset as u16 * 2),
                    inner.width,
                    1,
                ),
            );
        }
    }
    let mut chart = BarChart::horizontal(bars)
        .block(block)
        .bar_width(1)
        .bar_gap(1)
        .max(total.max(1) as u64)
        .label_style(Style::new().fg(theme.primary_text))
        .value_style(Style::new().fg(theme.primary_text));
    if !app.config().ui.unicode_icons {
        chart = chart.bar_set(ascii_bar_set());
    }
    frame.render_widget(chart, area);
    render_scrollbar(frame, app, area, stats.len(), first, visible_count);
}

#[cfg(any())]
fn render_analytics_details(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    let total = app.analytics_job_count();
    let stats = app.skill_stats_for(app.analytics_skill_kind());
    let Some(skill) = stats.get(app.selected_index()) else {
        frame.render_widget(
            Paragraph::new("No skill selected.").block(panel(
                "Analytics details",
                theme.focused_border,
                theme.background,
                borders,
            )),
            area,
        );
        return;
    };
    let sections = Layout::vertical([
        Constraint::Length(8.min(area.height.saturating_sub(2))),
        Constraint::Length(8.min(area.height.saturating_sub(10))),
        Constraint::Fill(1),
    ])
    .split(area);
    render_analytics_summary(frame, app, skill, total, sections[0], borders);
    render_related_skills(frame, app, sections[1]);
    render_skill_evidence(frame, app, sections[2]);
}

#[cfg(any())]
fn render_analytics_summary(
    frame: &mut Frame,
    app: &App,
    skill: &super::SkillStat,
    total: usize,
    area: Rect,
    borders: Borders,
) {
    let theme = app.theme();
    let coverage = app.analytics_coverage();
    let work_modes = app.work_mode_stats();
    let seniority = app.seniority_stats();
    let percent = skill.job_count * 100 / total.max(1);
    let count = |stats: &[super::CategoryStat], name: &str| {
        stats
            .iter()
            .find(|stat| stat.name == name)
            .map_or(0, |stat| stat.job_count)
    };
    let lines = vec![
        Line::styled(
            skill.name.clone(),
            Style::new()
                .fg(theme.primary_text)
                .add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!(
                "Observed in {} of {total} descriptions ({percent}%).",
                skill.job_count
            ),
            Style::new().fg(theme.muted_text),
        ),
        Line::styled(
            format!(
                "Coverage  descriptions {}/{} · dates {}/{} · sources {}/{} healthy",
                coverage.descriptions,
                coverage.total,
                coverage.published_dates,
                coverage.total,
                coverage.healthy_sources,
                coverage.enabled_sources
            ),
            Style::new().fg(theme.muted_text),
        ),
        Line::styled(
            format!(
                "Facts  work mode {} · seniority {} · experience {} · education {} · employment {}",
                coverage.work_mode,
                coverage.seniority,
                coverage.experience,
                coverage.education,
                coverage.employment_type
            ),
            Style::new().fg(theme.muted_text),
        ),
        Line::styled(
            format!(
                "Work mode  remote {} · hybrid {} · on-site {} · unknown {}",
                count(&work_modes, "Remote"),
                count(&work_modes, "Hybrid"),
                count(&work_modes, "On-site"),
                count(&work_modes, "Unknown")
            ),
            Style::new().fg(theme.primary_text),
        ),
        Line::styled(
            format!(
                "Seniority  junior {} · senior {} · lead {} · unknown {}",
                count(&seniority, "Junior"),
                count(&seniority, "Senior"),
                count(&seniority, "Lead"),
                count(&seniority, "Unknown")
            ),
            Style::new().fg(theme.primary_text),
        ),
    ];
    frame.render_widget(
        Paragraph::new(lines)
            .block(panel(
                "Observed posting analytics",
                if borders == Borders::ALL || app.hovered(MouseTarget::Details) {
                    theme.focused_border
                } else {
                    theme.unfocused_border
                },
                theme.background,
                borders,
            ))
            .style(Style::new().bg(theme.background)),
        area,
    );
}

#[cfg(any())]
fn render_related_skills(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let related = app.related_skill_stats();
    let title = format!(
        "Related skills · {} · minimum {} shared jobs",
        related.len(),
        app.config().analytics.minimum_cooccurrence
    );
    let block = panel(
        &title,
        theme.unfocused_border,
        theme.background,
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
    );
    if related.is_empty() {
        frame.render_widget(
            Paragraph::new("No related skill reaches the configured volume.")
                .block(block)
                .style(Style::new().fg(theme.muted_text).bg(theme.background)),
            area,
        );
        return;
    }
    let bars = related
        .iter()
        .take(usize::from(area.height.saturating_sub(2) / 2).max(1))
        .map(|skill| {
            Bar::with_label(
                format!(
                    "{}  {:.1}% · {} jobs",
                    skill.name,
                    skill.jaccard_per_mille as f64 / 10.0,
                    skill.job_count
                ),
                skill.jaccard_per_mille as u64,
            )
            .text_value(String::new())
            .style(Style::new().fg(theme.open))
        })
        .collect::<Vec<_>>();
    let mut chart = BarChart::horizontal(bars)
        .block(block)
        .bar_width(1)
        .bar_gap(1)
        .max(1_000)
        .label_style(Style::new().fg(theme.primary_text))
        .value_style(Style::new().fg(theme.primary_text));
    if !app.config().ui.unicode_icons {
        chart = chart.bar_set(ascii_bar_set());
    }
    frame.render_widget(chart, area);
}

#[cfg(any())]
fn render_skill_evidence(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let evidence = app.selected_skill_evidence();
    let items = evidence
        .iter()
        .enumerate()
        .map(|(index, (job, evidence))| {
            ListItem::new(vec![
                Line::styled(
                    format!(
                        "• {} · {}",
                        app.company_name(&job.key.company_id),
                        job.classified.observed.title
                    ),
                    Style::new().fg(theme.primary_text),
                ),
                Line::styled(
                    format!(
                        "  matched `{}` — {}",
                        evidence.matched_alias, evidence.context
                    ),
                    Style::new().fg(theme.muted_text),
                ),
            ])
            .style(mouse_style(app, MouseTarget::Evidence(index)))
        })
        .collect::<Vec<_>>();
    let title = format!("Evidence and matching jobs · {}", evidence.len());
    let list = List::new(items)
        .block(panel(
            &title,
            theme.unfocused_border,
            theme.background,
            Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
        ))
        .style(Style::new().bg(theme.background))
        .highlight_style(Style::new().bg(theme.selected_row))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    if !evidence.is_empty() {
        state.select(Some(app.evidence_index()));
    }
    frame.render_stateful_widget(list, area, &mut state);
    render_scrollbar(
        frame,
        app,
        area,
        evidence.len(),
        state.offset(),
        usize::from(area.height.saturating_sub(2) / 2).max(1),
    );
}

fn render_scrollbar(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    content_length: usize,
    position: usize,
    viewport_length: usize,
) {
    if content_length <= viewport_length || area.height < 3 {
        return;
    }
    let scroll_positions = content_length.saturating_sub(viewport_length) + 1;
    let mut state = ScrollbarState::new(scroll_positions)
        .position(position)
        .viewport_content_length(viewport_length);
    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
        .track_style(Style::new().fg(app.theme().unfocused_border))
        .thumb_style(Style::new().fg(app.theme().focused_border));
    frame.render_stateful_widget(scrollbar, area.inner(Margin::new(0, 1)), &mut state);
}

fn render_scrollable_paragraph(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    paragraph: Paragraph<'_>,
    borders: Borders,
) {
    let horizontal_borders =
        u16::from(borders.contains(Borders::LEFT)) + u16::from(borders.contains(Borders::RIGHT));
    let content_length = paragraph.line_count(area.width.saturating_sub(horizontal_borders));
    let viewport_length = usize::from(area.height);
    app.set_detail_scroll_max(
        content_length
            .saturating_sub(viewport_length)
            .min(usize::from(u16::MAX)) as u16,
    );
    let position = usize::from(app.detail_scroll());
    frame.render_widget(paragraph.scroll((app.detail_scroll(), 0)), area);
    render_scrollbar(frame, app, area, content_length, position, viewport_length);
}

fn ascii_bar_set() -> symbols::bar::Set<'static> {
    symbols::bar::Set {
        full: "#",
        seven_eighths: "#",
        three_quarters: "#",
        five_eighths: "#",
        half: "#",
        three_eighths: "#",
        one_quarter: "#",
        one_eighth: "#",
        empty: " ",
    }
}

fn render_single_job_pane(frame: &mut Frame, app: &App, area: Rect) {
    if app.narrow_details_visible() {
        render_details(frame, app, area, Borders::ALL);
    } else {
        render_job_list(frame, app, area, Borders::ALL);
    }
}

fn render_operational_view(frame: &mut Frame, app: &App, area: Rect) {
    if area.width < 80 && app.narrow_details_visible() {
        render_operational_details(frame, app, area);
        return;
    }
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

fn render_operational_details(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let (title, text, color) = match app.view() {
        View::Scans => app.scans().get(app.selected_index()).map_or_else(
            || {
                (
                    "Scan details",
                    Text::from("No scan selected."),
                    theme.muted_text,
                )
            },
            |scan| {
                let (status, color) = match scan.outcome {
                    ScanOutcome::Complete => ("COMPLETE", theme.open),
                    ScanOutcome::Incomplete => ("INCOMPLETE", theme.warning),
                    ScanOutcome::Failed => ("FAILED", theme.error),
                };
                (
                    "Scan details",
                    Text::from(vec![
                        metadata_line("Status      ", status.into(), theme.muted_text, color),
                        metadata_line(
                            "Company     ",
                            scan.company_name.clone(),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        metadata_line(
                            "Completed   ",
                            compact_time(scan.completed_at),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        metadata_line(
                            "Observed    ",
                            format!("{} observed", scan.observed_count),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        Line::from(""),
                        Line::styled(
                            diagnostic(&scan.error_kind, &scan.diagnostic),
                            Style::new().fg(color),
                        ),
                    ]),
                    color,
                )
            },
        ),
        View::Sources => app.sources().get(app.selected_index()).map_or_else(
            || {
                (
                    "Source details",
                    Text::from("No source selected."),
                    theme.muted_text,
                )
            },
            |source| {
                let (status, color) = match source.health {
                    SourceHealth::Unknown => ("UNKNOWN", theme.muted_text),
                    SourceHealth::Healthy => ("HEALTHY", theme.open),
                    SourceHealth::Incomplete => ("INCOMPLETE", theme.warning),
                    SourceHealth::Failed => ("FAILED", theme.error),
                };
                let configured = app
                    .config()
                    .companies
                    .iter()
                    .find(|company| company.id == source.company_id);
                let adapter = configured
                    .map(|company| company.source.strategy_name())
                    .unwrap_or("Unknown");
                let reference = configured
                    .map(|company| company.source.reference())
                    .unwrap_or("unknown");
                (
                    "Source details",
                    Text::from(vec![
                        metadata_line("Health       ", status.into(), theme.muted_text, color),
                        metadata_line(
                            "Company      ",
                            source.company_name.clone(),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        metadata_line(
                            "Industry     ",
                            configured
                                .map(|company| company.industry.clone())
                                .unwrap_or_else(|| "Unknown".into()),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        metadata_line(
                            "Scale        ",
                            configured
                                .map(|company| company.scale.clone())
                                .unwrap_or_else(|| "Unknown".into()),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        metadata_line(
                            "Adapter      ",
                            adapter.into(),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        metadata_line(
                            "Reference    ",
                            reference.into(),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        metadata_line(
                            "State        ",
                            if source.enabled {
                                "enabled"
                            } else {
                                "disabled"
                            }
                            .into(),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        metadata_line(
                            "last attempt ",
                            optional_time(source.latest_attempted_at),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        metadata_line(
                            "last success ",
                            optional_time(source.latest_successful_at),
                            theme.muted_text,
                            theme.primary_text,
                        ),
                        Line::from(""),
                        Line::styled(
                            diagnostic(&source.latest_error_kind, &source.diagnostic),
                            Style::new().fg(color),
                        ),
                    ]),
                    color,
                )
            },
        ),
        _ => unreachable!("operational details have fixed views"),
    };
    render_scrollable_paragraph(
        frame,
        app,
        area,
        Paragraph::new(text)
            .block(panel(
                title,
                theme.focused_border,
                theme.background,
                Borders::ALL,
            ))
            .style(Style::new().fg(color).bg(theme.background))
            .wrap(Wrap { trim: true }),
        Borders::ALL,
    );
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
        nav_item(app, 0, icons.open, "Active", theme.open),
        nav_item(app, 1, icons.new, "New", theme.new),
        nav_item(app, 2, icons.applied, "Applied", theme.applied),
        nav_item(app, 3, icons.history, "History", theme.muted_text),
        nav_item(app, 4, icons.scanning, "Scans", theme.primary_text),
        nav_item(app, 5, icons.source_failure, "Sources", theme.warning),
        nav_item(app, 6, "%", "Analytics", theme.new),
        nav_item(app, 7, "*", "Library", theme.warning),
        nav_item(app, 8, "=", "Settings", theme.primary_text),
    ];
    let list = List::new(items)
        .block(panel(
            "Navigation",
            if app.focus() == Focus::Navigation
                || app.hovered(MouseTarget::Navigation(app.navigation_index()))
            {
                theme.focused_border
            } else {
                theme.unfocused_border
            },
            theme.background,
            Borders::ALL,
        ))
        .style(Style::new().fg(theme.primary_text).bg(theme.background))
        .highlight_style(Style::new().bg(theme.selected_row))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    state.select(Some(app.navigation_index()));
    frame.render_stateful_widget(list, area, &mut state);
    render_scrollbar(
        frame,
        app,
        area,
        8,
        state.offset(),
        usize::from(area.height.saturating_sub(2)).max(1),
    );
}

fn nav_item(
    app: &App,
    index: usize,
    icon: &'static str,
    label: &'static str,
    color: Color,
) -> ListItem<'static> {
    ListItem::new(Line::from(vec![
        Span::styled(icon, Style::new().fg(color)),
        Span::raw(format!(" {label}")),
    ]))
    .style(mouse_style(app, MouseTarget::Navigation(index)))
}

fn render_job_list(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    let jobs = app.visible_jobs().collect::<Vec<_>>();
    let items = jobs.iter().enumerate().map(|(index, job)| {
        let (primary_icon, primary_label, primary_color) = if job.source_open {
            (app.icons().open, "OPEN", theme.open)
        } else {
            (app.icons().history, "CLOSED", theme.muted_text)
        };
        let mut state = vec![Span::styled(
            format!("{primary_icon} {primary_label}"),
            Style::new().fg(primary_color),
        )];
        if app.is_job_new(job) {
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
        .style(mouse_style(app, MouseTarget::Item(index)))
    });
    let title = match app.view() {
        View::Active => "Active jobs",
        View::New => "New jobs",
        View::Applied => "Applied jobs",
        View::History => "Job history",
        View::Scans | View::Sources | View::Analytics | View::Library | View::Settings => {
            unreachable!("job list has fixed views")
        }
    };
    let title = format!("{title} · {}", jobs.len());
    let list = List::new(items)
        .block(panel(
            &title,
            if app.focus() == Focus::Content {
                theme.focused_border
            } else {
                theme.unfocused_border
            },
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
    render_scrollbar(
        frame,
        app,
        area,
        jobs.len(),
        state.offset(),
        usize::from(area.height.saturating_sub(2) / 2).max(1),
    );
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
            if app.is_job_new(job) {
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
            lines.extend(markdown_lines(&observed.description, theme));
            Text::from(lines)
        },
    );
    let details = Paragraph::new(text)
        .block(panel(
            "Job details",
            if borders == Borders::ALL || app.hovered(MouseTarget::Details) {
                theme.focused_border
            } else {
                theme.unfocused_border
            },
            theme.background,
            borders,
        ))
        .style(Style::new().bg(theme.background))
        .wrap(Wrap { trim: true });
    render_scrollable_paragraph(frame, app, area, details, borders);
}

fn markdown_lines(markdown: &str, theme: super::Theme) -> Vec<Line<'static>> {
    markdown
        .lines()
        .map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return Line::from("");
            }
            if let Some(heading) = trimmed.strip_prefix('#') {
                return Line::styled(
                    heading.trim_start_matches('#').trim().to_owned(),
                    Style::new()
                        .fg(theme.primary_text)
                        .add_modifier(Modifier::BOLD),
                );
            }
            if let Some(item) = trimmed
                .strip_prefix("- ")
                .or_else(|| trimmed.strip_prefix("* "))
            {
                let mut spans = vec![Span::styled("• ", Style::new().fg(theme.new))];
                spans.extend(inline_markdown(item, theme));
                return Line::from(spans);
            }
            Line::from(inline_markdown(trimmed, theme))
        })
        .collect()
}

fn inline_markdown(markdown: &str, theme: super::Theme) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
    let mut remaining = markdown;
    while !remaining.is_empty() {
        let (delimiter, modifier) = if remaining.starts_with("**") {
            ("**", Modifier::BOLD)
        } else if remaining.starts_with('`') {
            ("`", Modifier::REVERSED)
        } else if remaining.starts_with('*') {
            ("*", Modifier::ITALIC)
        } else {
            let next = ["**", "`", "*"]
                .iter()
                .filter_map(|delimiter| remaining.find(delimiter))
                .min()
                .unwrap_or(remaining.len());
            if next == 0 {
                let character = remaining.chars().next().expect("remaining is not empty");
                spans.push(Span::styled(
                    character.to_string(),
                    Style::new().fg(theme.primary_text),
                ));
                remaining = &remaining[character.len_utf8()..];
            } else {
                spans.push(Span::styled(
                    remaining[..next].to_owned(),
                    Style::new().fg(theme.primary_text),
                ));
                remaining = &remaining[next..];
            }
            continue;
        };

        let content = &remaining[delimiter.len()..];
        if let Some(end) = content.find(delimiter) {
            spans.push(Span::styled(
                content[..end].to_owned(),
                Style::new().fg(theme.primary_text).add_modifier(modifier),
            ));
            remaining = &content[end + delimiter.len()..];
        } else {
            spans.push(Span::styled(
                delimiter.to_owned(),
                Style::new().fg(theme.primary_text),
            ));
            remaining = content;
        }
    }
    spans
}

fn render_settings(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    if app.input_mode() == InputMode::Setting {
        let (title, hint) = match app.setting() {
            Setting::NewJobAge => ("How recent?", "Enter a positive number of days."),
            Setting::Countries => (
                "Where?",
                "Enter countries separated by commas, for example NL, DE.",
            ),
            Setting::IncludedTitles => (
                "Advanced · jobs to include",
                "Separate regular expressions with ; or leave empty to include every job title.",
            ),
            Setting::ExcludedTitles => (
                "Advanced · jobs to hide",
                "Separate regular expressions with ; or leave empty to exclude no job titles.",
            ),
            Setting::AdvancedFilters | Setting::SimpleSettings => unreachable!(),
        };
        let mut lines = vec![
            Line::styled(
                format!("{}▏", app.setting_input()),
                Style::new().fg(theme.primary_text).bg(theme.hovered_row),
            ),
            Line::from(""),
            Line::styled(hint, Style::new().fg(theme.muted_text)),
            Line::styled(
                "Enter saves; Esc cancels. Scan again to apply country or title changes.",
                Style::new().fg(theme.muted_text),
            ),
        ];
        if let Some(error) = app.setting_error() {
            lines.push(Line::from(""));
            lines.push(Line::styled(error, Style::new().fg(theme.error)));
        }
        frame.render_widget(
            Paragraph::new(lines)
                .block(panel(
                    title,
                    theme.focused_border,
                    theme.background,
                    borders,
                ))
                .style(Style::new().bg(theme.background))
                .wrap(Wrap { trim: true }),
            area,
        );
        return;
    }

    let filters = &app.config().filters;
    let (title, items) = if app.advanced_settings() {
        (
            "Advanced title rules · 3 · Enter to change",
            vec![
                setting_item(app, 0, "Simple settings", "Back".to_owned()),
                setting_item(
                    app,
                    1,
                    "Include rules",
                    rule_count(filters.include_title_patterns.len()),
                ),
                setting_item(
                    app,
                    2,
                    "Hide rules",
                    rule_count(filters.exclude_title_patterns.len()),
                ),
            ],
        )
    } else {
        (
            "Settings · 5 · Enter to change",
            vec![
                setting_item(
                    app,
                    0,
                    "New jobs",
                    format!("Last {} days", filters.new_job_max_age_days),
                ),
                setting_item(
                    app,
                    1,
                    "Locations",
                    filters
                        .countries
                        .iter()
                        .map(|country| country_name(country))
                        .collect::<Vec<_>>()
                        .join(", "),
                ),
                setting_item(
                    app,
                    2,
                    "Job types",
                    job_type_summary(&filters.include_title_patterns),
                ),
                setting_item(
                    app,
                    3,
                    "Hide jobs",
                    hidden_title_summary(&filters.exclude_title_patterns),
                ),
                setting_item(app, 4, "Advanced", "Custom title rules".to_owned()),
            ],
        )
    };
    let list = List::new(items)
        .block(panel(
            title,
            if app.focus() == Focus::Content {
                theme.focused_border
            } else {
                theme.unfocused_border
            },
            theme.background,
            borders,
        ))
        .highlight_style(Style::new().bg(theme.selected_row))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    state.select(Some(app.selected_index()));
    frame.render_stateful_widget(list, area, &mut state);
    render_scrollbar(
        frame,
        app,
        area,
        if app.advanced_settings() { 3 } else { 5 },
        state.offset(),
        usize::from(area.height.saturating_sub(2)).max(1),
    );
}

fn setting_item(app: &App, index: usize, label: &'static str, value: String) -> ListItem<'static> {
    let theme = app.theme();
    ListItem::new(Line::from(vec![
        Span::styled(format!("{label:<14}"), Style::new().fg(theme.primary_text)),
        Span::styled(value, Style::new().fg(theme.new)),
    ]))
    .style(mouse_style(app, MouseTarget::Setting(index)))
}

fn rule_count(count: usize) -> String {
    match count {
        0 => "None".to_owned(),
        1 => "1 regex rule".to_owned(),
        count => format!("{count} regex rules"),
    }
}

fn country_name(code: &str) -> String {
    match code.to_ascii_uppercase().as_str() {
        "NL" => "Netherlands".to_owned(),
        "BE" => "Belgium".to_owned(),
        "DE" => "Germany".to_owned(),
        "FR" => "France".to_owned(),
        "GB" | "UK" => "United Kingdom".to_owned(),
        "IE" => "Ireland".to_owned(),
        "ES" => "Spain".to_owned(),
        "PT" => "Portugal".to_owned(),
        "IT" => "Italy".to_owned(),
        "CH" => "Switzerland".to_owned(),
        "AT" => "Austria".to_owned(),
        "PL" => "Poland".to_owned(),
        "DK" => "Denmark".to_owned(),
        "SE" => "Sweden".to_owned(),
        "NO" => "Norway".to_owned(),
        "FI" => "Finland".to_owned(),
        _ => code.to_owned(),
    }
}

fn job_type_summary(patterns: &[String]) -> String {
    if patterns.is_empty() {
        return "All job types".to_owned();
    }
    let rules = patterns.join("|").to_ascii_lowercase();
    let groups = [
        (
            "Software",
            ["software", "backend", "front", "full", "mobile"].as_slice(),
        ),
        (
            "Platform",
            ["platform", "devops", "cloud", "reliability", "sre"].as_slice(),
        ),
        ("Data", ["data engineer", "analytics engineer"].as_slice()),
        (
            "AI",
            ["machine learning", "ml engineer", "ai engineer"].as_slice(),
        ),
        ("Security", ["security"].as_slice()),
    ]
    .into_iter()
    .filter(|(_, words)| words.iter().any(|word| rules.contains(word)))
    .map(|(label, _)| label)
    .collect::<Vec<_>>();
    if groups.is_empty() {
        "Custom rules".to_owned()
    } else {
        groups.join(", ")
    }
}

fn hidden_title_summary(patterns: &[String]) -> String {
    if patterns.is_empty() {
        return "Nothing".to_owned();
    }
    if patterns.iter().any(|pattern| {
        pattern
            .chars()
            .any(|character| !character.is_alphanumeric() && character != ' ' && character != '-')
    }) {
        return "Custom rules".to_owned();
    }
    patterns
        .iter()
        .map(|pattern| {
            let mut characters = pattern.chars();
            characters.next().map_or_else(String::new, |first| {
                first.to_uppercase().collect::<String>() + characters.as_str()
            })
        })
        .collect::<Vec<_>>()
        .join(", ")
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
    let items = app.scans().iter().enumerate().map(|(index, scan)| {
        let (icon, label, color) = match scan.outcome {
            ScanOutcome::Complete => (app.icons().open, "COMPLETE", theme.open),
            ScanOutcome::Incomplete => (app.icons().source_failure, "INCOMPLETE", theme.warning),
            ScanOutcome::Failed => (app.icons().source_failure, "FAILED", theme.error),
        };
        let diagnostic = diagnostic(&scan.error_kind, &scan.diagnostic);
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
        .style(mouse_style(app, MouseTarget::Item(index)))
    });
    let title = format!("Recent scans · {}", app.scans().len());
    let list = if app.scans().is_empty() {
        List::new(vec![ListItem::new("No scan history yet.")])
    } else {
        List::new(items.collect::<Vec<_>>())
    }
    .block(panel(
        &title,
        if app.focus() == Focus::Content {
            theme.focused_border
        } else {
            theme.unfocused_border
        },
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
    render_scrollbar(
        frame,
        app,
        area,
        app.scans().len(),
        state.offset(),
        usize::from(area.height.saturating_sub(2) / 2).max(1),
    );
}

fn render_sources(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    let rows = app.sources().iter().enumerate().map(|(index, source)| {
        let (label, color) = match source.health {
            SourceHealth::Unknown => ("UNKNOWN", theme.muted_text),
            SourceHealth::Healthy => ("HEALTHY", theme.open),
            SourceHealth::Incomplete => ("INCOMPLETE", theme.warning),
            SourceHealth::Failed => ("FAILED", theme.error),
        };
        let state = if source.enabled {
            "enabled"
        } else {
            "disabled"
        };
        let configured = app
            .config()
            .companies
            .iter()
            .find(|company| company.id == source.company_id);
        let adapter = configured
            .map(|company| company.source.strategy_name())
            .unwrap_or("Unknown");
        let values = if area.width >= 140 {
            vec![
                Cell::from(label).style(Style::new().fg(color)),
                Cell::from(source.company_name.clone()),
                Cell::from(
                    configured
                        .map(|company| company.industry.as_str())
                        .unwrap_or("Unknown"),
                ),
                Cell::from(
                    configured
                        .map(|company| company.scale.as_str())
                        .unwrap_or("Unknown"),
                ),
                Cell::from(adapter),
                Cell::from(state),
                Cell::from(optional_time(source.latest_attempted_at)),
                Cell::from(optional_time(source.latest_successful_at)),
                Cell::from(diagnostic(&source.latest_error_kind, &source.diagnostic))
                    .style(Style::new().fg(color)),
            ]
        } else if area.width >= 90 {
            vec![
                Cell::from(label).style(Style::new().fg(color)),
                Cell::from(source.company_name.clone()),
                Cell::from(adapter),
                Cell::from(state),
                Cell::from(optional_time(source.latest_attempted_at)),
                Cell::from(optional_time(source.latest_successful_at)),
                Cell::from(diagnostic(&source.latest_error_kind, &source.diagnostic))
                    .style(Style::new().fg(color)),
            ]
        } else {
            vec![
                Cell::from(label).style(Style::new().fg(color)),
                Cell::from(source.company_name.clone()),
                Cell::from(adapter),
                Cell::from(optional_time(source.latest_attempted_at)),
            ]
        };
        Row::new(values).style(
            Style::new()
                .fg(theme.primary_text)
                .patch(mouse_style(app, MouseTarget::Item(index))),
        )
    });
    let title = format!("Sources · {}", app.sources().len());
    if app.sources().is_empty() {
        frame.render_widget(
            Paragraph::new("No configured sources.").block(panel(
                &title,
                theme.focused_border,
                theme.background,
                borders,
            )),
            area,
        );
        return;
    }
    let (headers, widths) = if area.width >= 140 {
        (
            vec![
                "Health",
                "Company",
                "Industry",
                "Scale",
                "Adapter",
                "State",
                "Last attempt",
                "Last success",
                "Diagnostic",
            ],
            vec![
                Constraint::Length(10),
                Constraint::Fill(3),
                Constraint::Fill(5),
                Constraint::Fill(4),
                Constraint::Fill(3),
                Constraint::Length(8),
                Constraint::Length(13),
                Constraint::Length(13),
                Constraint::Fill(4),
            ],
        )
    } else if area.width >= 90 {
        (
            vec![
                "Health",
                "Company",
                "Adapter",
                "State",
                "Last attempt",
                "Last success",
                "Diagnostic",
            ],
            vec![
                Constraint::Length(10),
                Constraint::Fill(2),
                Constraint::Fill(2),
                Constraint::Length(8),
                Constraint::Length(13),
                Constraint::Length(13),
                Constraint::Fill(3),
            ],
        )
    } else {
        (
            vec!["Health", "Company", "Adapter", "Last attempt"],
            vec![
                Constraint::Length(11),
                Constraint::Fill(3),
                Constraint::Fill(2),
                Constraint::Length(13),
            ],
        )
    };
    let table = Table::new(rows, widths)
        .header(
            Row::new(headers)
                .style(
                    Style::new()
                        .fg(theme.muted_text)
                        .add_modifier(Modifier::BOLD),
                )
                .bottom_margin(1),
        )
        .block(panel(
            &title,
            if app.focus() == Focus::Content {
                theme.focused_border
            } else {
                theme.unfocused_border
            },
            theme.background,
            borders,
        ))
        .row_highlight_style(Style::new().bg(theme.selected_row))
        .highlight_symbol("› ");
    let mut state = TableState::default().with_selected(app.selected_index());
    frame.render_stateful_widget(table, area, &mut state);
    render_scrollbar(
        frame,
        app,
        area,
        app.sources().len(),
        state.offset(),
        usize::from(area.height.saturating_sub(4)).max(1),
    );
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let keys = &app.config().keybindings;
    let status = app.footer_status();
    let (icon, status_color) = if status.starts_with("FAILED") {
        (app.icons().source_failure, theme.error)
    } else if status.starts_with("INCOMPLETE") {
        (app.icons().source_failure, theme.warning)
    } else if status.starts_with("SCANNING") || status == "LOADING" {
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
        ]
    } else if app.input_mode() == InputMode::Setting {
        vec!["Enter save".to_owned(), "Esc cancel".to_owned()]
    } else if app.focus() == Focus::Navigation {
        vec![
            "↑/↓ select tab".to_owned(),
            "Enter open".to_owned(),
            "Esc return".to_owned(),
        ]
    } else if app.view() == View::Analytics {
        let mut actions = vec![
            "[/] section".to_owned(),
            "↑/↓ rows".to_owned(),
            "J/K matches".to_owned(),
            "t/± window".to_owned(),
            "C/R/S/W filter · x clear".to_owned(),
            "* save · m status".to_owned(),
            "Tab navigation".to_owned(),
        ];
        if area.width >= 80 {
            actions.push(format!("{} quit", keys.quit));
        }
        actions
    } else if app.view() == View::Library {
        let mut actions = vec![
            "[/] section".to_owned(),
            "↑/↓ rows".to_owned(),
            "* remove · m status/target".to_owned(),
            "Tab navigation".to_owned(),
        ];
        if area.width >= 80 {
            actions.push(format!("{} quit", keys.quit));
        }
        actions
    } else if app.view() == View::Settings {
        let mut actions = vec![
            "↑/↓ settings".to_owned(),
            "Enter change".to_owned(),
            "Tab navigation".to_owned(),
        ];
        if area.width >= 80 {
            actions.push(format!("{} quit", keys.quit));
        }
        actions
    } else if matches!(app.view(), View::Scans | View::Sources) {
        let mut actions = vec![format!("{} scan", keys.scan), "Tab navigation".to_owned()];
        if area.width >= 80 {
            actions.push(format!("{} quit", keys.quit));
        }
        actions
    } else if area.width < 80 {
        if app.narrow_details_visible() {
            vec![
                format!("{} open", keys.open),
                format!("{} copy", keys.copy),
                format!("{} applied", keys.toggle_applied),
            ]
        } else {
            vec![
                format!("{} scan", keys.scan),
                format!("{} search", keys.search),
            ]
        }
    } else {
        let mut actions = vec![
            format!("{} scan", keys.scan),
            format!("{} search", keys.search),
            format!("{} applied", keys.toggle_applied),
            format!("{} open", keys.open),
            format!("{} copy", keys.copy),
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
        actions
    };
    let help = Line::from(format!("{} help", keys.help));
    let footer = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(u16::try_from(help.width()).unwrap_or(u16::MAX)),
    ])
    .split(area);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("{icon} {status}"), Style::new().fg(status_color)),
            Span::styled(
                format!("  {}", actions.join(" ")),
                Style::new().fg(theme.muted_text),
            ),
        ]))
        .style(Style::new().bg(theme.background)),
        footer[0],
    );
    frame.render_widget(
        Paragraph::new(help).style(
            Style::new()
                .fg(theme.muted_text)
                .bg(theme.background)
                .patch(mouse_style(app, MouseTarget::Help)),
        ),
        footer[1],
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
        "Tab/Esc focus navigation; ↑/↓ choose; Enter open\n\
         Inside a tab: ↑/↓ or j/k select\n\
         J/K scroll details\n\
         Mouse: hover; click tabs/rows; drag job divider; wheel select or scroll details\n\
         Click Settings to edit; click Help to close\n\n\
         Analytics: [/] or 1-4 sections; arrows rows/type; J/K evidence; Enter/open\n\
         Filters: t/± time; C/R/S/W factors; x clear; * save; m skill status\n\
         Library: [/] or 1-5; * remove; a/d optional AI review; never auto-approved\n\n\
         {} scan  {} search jobs  {} filter company/New/Applied\n\
         {} applied  {} history  {} open  {} copy\n\
         Enter narrow: details; otherwise: open\n\
         Search: title or company; Enter accept; Esc clear\n\
         Esc close help  {} toggle help  {} quit",
        keys.scan,
        keys.search,
        keys.filter,
        keys.toggle_applied,
        keys.history,
        keys.open,
        keys.copy,
        keys.help,
        keys.quit,
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

fn diagnostic(
    error_kind: &Option<crate::domain::SourceErrorKind>,
    diagnostic: &Option<String>,
) -> String {
    match (error_kind, diagnostic) {
        (Some(kind), Some(diagnostic)) => format!("{kind} · {diagnostic}"),
        (Some(kind), None) => kind.to_string(),
        (None, Some(diagnostic)) => diagnostic.clone(),
        (None, None) => "no diagnostic".to_owned(),
    }
}

fn panel(title: &str, border: Color, background: Color, borders: Borders) -> Block<'_> {
    Block::new()
        .title(title)
        .borders(borders)
        .border_style(Style::new().fg(border))
        .style(Style::new().bg(background))
}

fn mouse_style(app: &App, target: MouseTarget) -> Style {
    let mut style = Style::new();
    if app.hovered(target) {
        style = style.bg(app.theme().hovered_row);
    }
    if app.pressed(target) {
        style = style
            .bg(app.theme().selected_row)
            .add_modifier(Modifier::BOLD);
    }
    style
}
