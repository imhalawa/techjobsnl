use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::Style,
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use super::App;

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let theme = app.theme();
    frame.render_widget(Block::new().style(Style::new().bg(theme.background)), area);

    let areas = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
    let content = areas[0];
    let footer = areas[1];
    match content.width {
        120.. => render_navigation_list_details(frame, app, content),
        80..=119 => render_list_details(frame, app, content),
        _ => render_single_pane(frame, app, content),
    }
    render_footer(frame, app, footer);
}

fn render_navigation_list_details(frame: &mut Frame, app: &App, area: Rect) {
    let areas = Layout::horizontal([
        Constraint::Length(22),
        Constraint::Percentage(40),
        Constraint::Min(30),
    ])
    .split(area);
    render_navigation(frame, app, areas[0]);
    render_job_list(frame, app, areas[1]);
    render_details(frame, app, areas[2]);
}

fn render_list_details(frame: &mut Frame, app: &App, area: Rect) {
    let areas = Layout::horizontal([Constraint::Percentage(45), Constraint::Min(30)]).split(area);
    render_job_list(frame, app, areas[0]);
    render_details(frame, app, areas[1]);
}

fn render_single_pane(frame: &mut Frame, app: &App, area: Rect) {
    render_job_list(frame, app, area);
}

fn render_navigation(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let icons = app.icons();
    let items = [
        "Active".to_owned(),
        "New".to_owned(),
        "Applied".to_owned(),
        format!("{} History", icons.history),
        format!("{} Scans", icons.scanning),
        format!("{} Sources", icons.source_failure),
    ];
    let list = List::new(items)
        .block(panel(
            "Navigation",
            theme.unfocused_border,
            theme.background,
        ))
        .style(Style::new().fg(theme.primary_text).bg(theme.background));
    frame.render_widget(list, area);
}

fn render_job_list(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let jobs = app.visible_jobs().collect::<Vec<_>>();
    let items = jobs.iter().map(|job| {
        let mut spans = vec![Span::styled(
            format!("{} OPEN ", app.icons().open),
            Style::new().fg(theme.open),
        )];
        if job.is_new {
            spans.push(Span::styled(
                format!("{} NEW ", app.icons().new),
                Style::new().fg(theme.new),
            ));
        }
        if job.applied_at.is_some() {
            spans.push(Span::styled(
                format!("{} APPLIED ", app.icons().applied),
                Style::new().fg(theme.applied),
            ));
        }
        spans.push(Span::styled(
            job.classified.observed.title.as_str(),
            Style::new().fg(theme.primary_text),
        ));
        ListItem::new(Line::from(spans))
    });
    let list = List::new(items)
        .block(panel("Active jobs", theme.focused_border, theme.background))
        .highlight_style(Style::new().bg(theme.selected_row))
        .highlight_symbol("› ");
    let mut state = ListState::default();
    if !jobs.is_empty() {
        state.select(Some(app.selected_index()));
    }
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_details(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let text = app.selected_job().map_or_else(
        || Text::from("No jobs in this view."),
        |job| {
            Text::from(vec![
                Line::styled(
                    job.classified.observed.title.as_str(),
                    Style::new().fg(theme.primary_text),
                ),
                Line::styled(
                    format!(
                        "{} · {}",
                        job.key.company_id,
                        job.classified.observed.locations.join(", ")
                    ),
                    Style::new().fg(theme.muted_text),
                ),
                Line::from(""),
                Line::styled(
                    job.classified.observed.description.as_str(),
                    Style::new().fg(theme.primary_text),
                ),
            ])
        },
    );
    let details = Paragraph::new(text)
        .block(panel(
            "Job details",
            theme.unfocused_border,
            theme.background,
        ))
        .style(Style::new().bg(theme.background))
        .wrap(Wrap { trim: true });
    frame.render_widget(details, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let theme = app.theme();
    let enabled = app
        .config()
        .companies
        .iter()
        .filter(|company| company.enabled)
        .count();
    let keys = &app.config().keybindings;
    let text = format!(
        "OK  {} scan  {} search  {} applied  {} quit  {enabled} companies  {} active jobs",
        keys.scan,
        keys.search,
        keys.toggle_applied,
        keys.quit,
        app.jobs().iter().filter(|job| job.source_open).count()
    );
    frame.render_widget(
        Paragraph::new(text).style(Style::new().fg(theme.muted_text).bg(theme.background)),
        area,
    );
}

fn panel(
    title: &str,
    border: ratatui::style::Color,
    background: ratatui::style::Color,
) -> Block<'_> {
    Block::new()
        .title(title)
        .borders(Borders::ALL)
        .border_style(Style::new().fg(border))
        .style(Style::new().bg(background))
}
