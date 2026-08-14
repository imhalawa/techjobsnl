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

use crate::storage::{ScanOutcome, SourceHealth};

use super::{App, Focus, InputMode, MouseTarget, Setting, View};

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
    if area.width < 80 {
        if app.narrow_details_visible() {
            render_analytics_details(frame, app, area, Borders::ALL);
        } else {
            render_analytics_list(frame, app, area, Borders::ALL);
        }
        return;
    }
    let (list, details) = app.job_panes(area).expect("analytics panes");
    if area.width >= 120 {
        render_navigation(frame, app, Rect::new(area.x, area.y, 22, area.height));
        render_analytics_list(
            frame,
            app,
            list,
            Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
        );
    } else {
        render_analytics_list(frame, app, list, Borders::ALL);
    }
    render_analytics_details(
        frame,
        app,
        details,
        Borders::TOP | Borders::RIGHT | Borders::BOTTOM,
    );
    render_job_divider(frame, app, list);
}

fn render_analytics_list(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    let stats = app.skill_stats();
    let total = app.analytics_job_count();
    let coverage = app.analytics_coverage();
    let skill_label = if stats.len() == 1 { "skill" } else { "skills" };
    let title = format!(
        "Observed skills · {} {skill_label} · {total} jobs · {}/{} descriptions",
        stats.len(),
        coverage.descriptions,
        coverage.total
    );
    let block = panel(
        &title,
        if app.focus() == Focus::Content {
            theme.focused_border
        } else {
            theme.unfocused_border
        },
        theme.background,
        borders,
    );
    if stats.is_empty() {
        frame.render_widget(
            Paragraph::new("No configured skills found in observed job descriptions.")
                .block(block)
                .style(Style::new().bg(theme.background)),
            area,
        );
        return;
    }

    let visible_count = usize::from(area.height.saturating_sub(2) / 2).max(1);
    let first = app
        .selected_index()
        .saturating_sub(visible_count.saturating_sub(1));
    let bars = stats
        .iter()
        .enumerate()
        .skip(first)
        .take(visible_count)
        .map(|(index, stat)| {
            let percent = stat.job_count * 100 / total.max(1);
            let selected = index == app.selected_index();
            let style = if selected {
                Style::new().fg(theme.warning).bg(theme.selected_row)
            } else {
                Style::new()
                    .fg(theme.new)
                    .patch(mouse_style(app, MouseTarget::Item(index)))
            };
            Bar::with_label(stat.name.clone(), stat.job_count as u64)
                .text_value(format!("{} jobs · {percent}%", stat.job_count))
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
        let target = MouseTarget::Item(index);
        let background = if app.pressed(target) || index == app.selected_index() {
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

fn render_analytics_details(frame: &mut Frame, app: &App, area: Rect, borders: Borders) {
    let theme = app.theme();
    let total = app.analytics_job_count();
    let stats = app.skill_stats();
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
            Bar::with_label(skill.name.clone(), skill.jaccard_per_mille as u64)
                .text_value(format!(
                    "{:.1}% · {} jobs",
                    skill.jaccard_per_mille as f64 / 10.0,
                    skill.job_count
                ))
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
        nav_item(app, 7, "=", "Settings", theme.primary_text),
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
        View::Scans | View::Sources | View::Analytics | View::Settings => {
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
            Setting::NewJobAge => ("Edit new job age", "Enter a positive number of days."),
            Setting::Countries => (
                "Edit countries",
                "Use two-letter country codes separated by commas, for example NL, DE.",
            ),
            Setting::IncludedTitles => (
                "Edit included titles",
                "Separate regular expressions with ; or leave empty to include every job title.",
            ),
            Setting::ExcludedTitles => (
                "Edit excluded titles",
                "Separate regular expressions with ; or leave empty to exclude no job titles.",
            ),
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
    let included = if filters.include_title_patterns.is_empty() {
        "All titles".to_owned()
    } else {
        format!("{} patterns", filters.include_title_patterns.len())
    };
    let excluded = if filters.exclude_title_patterns.is_empty() {
        "None".to_owned()
    } else {
        format!("{} patterns", filters.exclude_title_patterns.len())
    };
    let items = vec![
        setting_item(
            app,
            0,
            "New job age",
            format!("{} days", filters.new_job_max_age_days),
            "Controls which published jobs appear in New.",
        ),
        setting_item(
            app,
            1,
            "Countries",
            filters.countries.join(", "),
            "Only jobs in these countries are eligible.",
        ),
        setting_item(
            app,
            2,
            "Included titles",
            included,
            "Clear this setting to include engineering and non-engineering jobs.",
        ),
        setting_item(
            app,
            3,
            "Excluded titles",
            excluded,
            "Matching titles remain hidden; clear this setting to exclude none.",
        ),
    ];
    let list = List::new(items)
        .block(panel(
            "Settings · 4 · Enter to edit",
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
        4,
        state.offset(),
        usize::from(area.height.saturating_sub(2) / 2).max(1),
    );
}

fn setting_item(
    app: &App,
    index: usize,
    label: &'static str,
    value: String,
    help: &'static str,
) -> ListItem<'static> {
    let theme = app.theme();
    ListItem::new(vec![
        Line::from(vec![
            Span::styled(format!("{label}  "), Style::new().fg(theme.primary_text)),
            Span::styled(value, Style::new().fg(theme.new)),
        ]),
        Line::styled(format!("  {help}"), Style::new().fg(theme.muted_text)),
    ])
    .style(mouse_style(app, MouseTarget::Setting(index)))
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
                Constraint::Length(16),
                Constraint::Length(20),
                Constraint::Length(20),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Length(13),
                Constraint::Length(13),
                Constraint::Fill(1),
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
                Constraint::Length(16),
                Constraint::Length(12),
                Constraint::Length(8),
                Constraint::Length(13),
                Constraint::Length(13),
                Constraint::Fill(1),
            ],
        )
    } else {
        (
            vec!["Health", "Company", "Adapter", "Last attempt"],
            vec![
                Constraint::Length(11),
                Constraint::Percentage(34),
                Constraint::Percentage(26),
                Constraint::Fill(1),
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
            "↑/↓ skills".to_owned(),
            "J/K matches".to_owned(),
            "Enter open".to_owned(),
            "Tab navigation".to_owned(),
        ];
        if area.width >= 80 {
            actions.push(format!("{} quit", keys.quit));
        }
        actions
    } else if matches!(app.view(), View::Scans | View::Sources | View::Settings) {
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
         Analytics: ↑/↓ skills; J/K matches; Enter/open key opens match\n\
         Analytics uses cached active job descriptions\n\n\
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
