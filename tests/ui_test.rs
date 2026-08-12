use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use job_watch::{
    config::{
        CompanyConfig, Config, FiltersConfig, KeybindingsConfig, ScanConfig, SourceConfig,
        ThemeOverrides, UiConfig,
    },
    domain::{
        ClassifiedJob, Eligibility, JobKey, JobRecord, ObservedJob, ScanEvent, SourceErrorKind,
    },
    storage::{ScanOutcome, ScanReadModel, SourceHealth, SourceReadModel},
    ui::{App, AppCommand, IconSet, InputMode, Theme, View, render},
};
use ratatui::{Terminal, backend::TestBackend, buffer::Buffer, style::Color};
use serde_json::json;

fn rendered(app: &App, width: u16, height: u16) -> String {
    rendered_buffer(app, width, height)
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

fn rendered_buffer(app: &App, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| render(frame, app)).unwrap();
    terminal.backend().buffer().clone()
}

fn row(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer.cell((x, y)).unwrap().symbol())
        .collect()
}

fn symbol_x(buffer: &Buffer, y: u16, start_x: u16, symbol: &str) -> u16 {
    (start_x..buffer.area.width)
        .find(|x| buffer.cell((*x, y)).unwrap().symbol() == symbol)
        .unwrap_or_else(|| panic!("missing {symbol:?} on row {y}"))
}

fn normalized_interior(buffer: &Buffer) -> String {
    (1..buffer.area.height.saturating_sub(1))
        .flat_map(|y| {
            (1..buffer.area.width.saturating_sub(1))
                .map(move |x| buffer.cell((x, y)).unwrap().symbol())
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn config() -> Config {
    Config {
        schema_version: 1,
        database_path: ":memory:".into(),
        companies: vec![CompanyConfig {
            id: "acme".into(),
            name: "Acme".into(),
            enabled: true,
            location_country_overrides: HashMap::new(),
            source: SourceConfig::Ashby {
                board: "acme".into(),
            },
        }],
        filters: FiltersConfig {
            countries: vec!["NL".into()],
            include_families: vec![],
            include_title_patterns: vec![],
            exclude_title_patterns: vec![],
        },
        scan: ScanConfig {
            concurrency: 1,
            timeout_seconds: 30,
            retry_count: 0,
            user_agent: "ui-test".into(),
        },
        ui: UiConfig {
            theme: "clean-dark".into(),
            unicode_icons: true,
            theme_overrides: Default::default(),
        },
        keybindings: KeybindingsConfig {
            scan: "r".into(),
            search: "/".into(),
            filter: "f".into(),
            toggle_applied: "a".into(),
            history: "h".into(),
            open: "o".into(),
            copy: "c".into(),
            help: "?".into(),
            quit: "q".into(),
        },
    }
}

fn config_with_two_companies() -> Config {
    let mut configured = config();
    configured.companies.push(CompanyConfig {
        id: "beta".into(),
        name: "Beta Labs".into(),
        enabled: true,
        location_country_overrides: HashMap::new(),
        source: SourceConfig::Ashby {
            board: "beta".into(),
        },
    });
    configured
}

fn key(character: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE)
}

fn special(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn job(title: &str, is_new: bool, applied: bool) -> JobRecord {
    job_for("acme", title, is_new, applied)
}

fn job_for(company_id: &str, title: &str, is_new: bool, applied: bool) -> JobRecord {
    let seen_at = Utc.with_ymd_and_hms(2026, 8, 11, 9, 0, 0).unwrap();
    JobRecord {
        key: JobKey::new(company_id, title),
        classified: ClassifiedJob {
            observed: ObservedJob {
                source_id: title.into(),
                title: title.into(),
                department: Some("Engineering".into()),
                team: None,
                employment_type: Some("Full-time".into()),
                locations: vec!["Amsterdam".into()],
                countries: vec!["NL".into()],
                job_url: "https://example.test/job".into(),
                apply_url: "https://example.test/apply".into(),
                description: "Build reliable systems.".into(),
                raw_payload: json!({}),
                published_at: Some(seen_at),
            },
            eligibility: Eligibility {
                eligible: true,
                reason: "eligible".into(),
            },
        },
        source_open: true,
        is_new,
        first_seen_at: seen_at,
        last_seen_at: seen_at,
        closed_at: None,
        reopened_at: None,
        applied_at: applied.then_some(seen_at),
    }
}

#[test]
fn default_keys_emit_commands_and_move_the_selection() {
    let mut app = App::new(
        config(),
        vec![
            job("Backend Engineer", false, false),
            job("Platform Engineer", false, false),
        ],
    );

    assert_eq!(app.handle_key(key('j')), AppCommand::None);
    assert_eq!(app.selected_index(), 1);
    assert!(matches!(
        app.handle_key(key('a')),
        AppCommand::ToggleApplied(_)
    ));
    assert!(matches!(app.handle_key(key('o')), AppCommand::OpenUrl(_)));
    assert_eq!(
        app.handle_key(key('c')),
        AppCommand::CopyUrl("https://example.test/job".into())
    );
    assert_eq!(app.handle_key(key('r')), AppCommand::StartScan);
    assert_eq!(app.handle_key(key('h')), AppCommand::ReloadJobs);
    assert_eq!(app.view(), View::History);
    assert_eq!(app.handle_key(key('q')), AppCommand::Quit);
}

#[test]
fn configured_actions_replace_only_the_configurable_keys() {
    let mut configured = config();
    configured.keybindings.scan = "s".into();
    configured.keybindings.search = "z".into();
    configured.keybindings.filter = "v".into();
    configured.keybindings.toggle_applied = "x".into();
    configured.keybindings.history = "y".into();
    configured.keybindings.open = "u".into();
    configured.keybindings.copy = "w".into();
    configured.keybindings.help = "i".into();
    configured.keybindings.quit = "e".into();
    let mut app = App::new(configured, vec![job("Backend Engineer", false, false)]);

    assert_eq!(app.handle_key(key('s')), AppCommand::StartScan);
    assert!(matches!(
        app.handle_key(key('x')),
        AppCommand::ToggleApplied(_)
    ));
    assert_eq!(app.handle_key(key('r')), AppCommand::None);
    assert_eq!(app.handle_key(key('a')), AppCommand::None);
    assert_eq!(app.handle_key(key('z')), AppCommand::None);
    assert_eq!(app.input_mode(), InputMode::Search);
    assert_eq!(app.handle_key(special(KeyCode::Esc)), AppCommand::None);
    assert!(matches!(app.handle_key(key('u')), AppCommand::OpenUrl(_)));
    assert_eq!(
        app.handle_key(key('w')),
        AppCommand::CopyUrl("https://example.test/job".into())
    );
    assert_eq!(app.handle_key(key('c')), AppCommand::None);
    assert_eq!(app.handle_key(key('v')), AppCommand::ReloadJobs);
    assert_eq!(app.company_filter(), Some("acme"));
    assert_eq!(app.handle_key(key('y')), AppCommand::ReloadJobs);
    assert_eq!(app.view(), View::History);
    assert_eq!(app.handle_key(key('i')), AppCommand::None);
    assert!(app.help_visible());
    assert_eq!(app.handle_key(key('e')), AppCommand::Quit);
}

#[test]
fn search_filters_by_title_or_company_but_not_posting_text_and_escape_cancels() {
    let configured = config_with_two_companies();
    let mut backend = job("Backend Engineer", false, false);
    backend.classified.observed.description = "Maintain payment ledgers.".into();
    let mut app = App::new(
        configured,
        vec![backend, job_for("beta", "Platform Engineer", false, false)],
    );

    app.handle_key(key('/'));
    app.handle_key(key('P'));
    app.handle_key(key('l'));
    assert_eq!(app.input_mode(), InputMode::Search);
    assert_eq!(app.search_query(), "Pl");
    assert_eq!(app.visible_jobs().count(), 1);
    assert_eq!(app.selected_job().unwrap().key.company_id, "beta");

    app.handle_key(special(KeyCode::Esc));
    assert_eq!(app.input_mode(), InputMode::Normal);
    assert_eq!(app.search_query(), "");
    assert_eq!(app.visible_jobs().count(), 2);

    app.handle_key(key('/'));
    for character in "beta".chars() {
        app.handle_key(key(character));
    }
    assert_eq!(app.visible_jobs().count(), 1);
    assert_eq!(app.selected_job().unwrap().key.company_id, "beta");

    app.handle_key(special(KeyCode::Esc));
    app.handle_key(key('/'));
    for character in "ledgers".chars() {
        app.handle_key(key(character));
    }
    assert_eq!(app.visible_jobs().count(), 0);
}

#[test]
fn configured_filter_cycles_companies_new_applied_and_clears() {
    let mut app = App::new(
        config_with_two_companies(),
        vec![
            job("Backend Engineer", false, false),
            job_for("beta", "Platform Engineer", false, false),
        ],
    );

    assert_eq!(app.handle_key(key('f')), AppCommand::ReloadJobs);
    assert_eq!(app.company_filter(), Some("acme"));
    assert_eq!(app.visible_jobs().count(), 1);
    assert!(rendered(&app, 100, 25).contains("filter Acme"));

    app.handle_key(key('f'));
    assert_eq!(app.company_filter(), Some("beta"));
    assert_eq!(app.visible_jobs().count(), 1);

    app.handle_key(key('f'));
    assert_eq!(app.company_filter(), None);
    assert_eq!(app.view(), View::New);
    assert!(rendered(&app, 100, 25).contains("filter New"));

    app.handle_key(key('f'));
    assert_eq!(app.view(), View::Applied);
    assert!(rendered(&app, 100, 25).contains("filter Applied"));

    app.handle_key(key('f'));
    assert_eq!(app.company_filter(), None);
    assert_eq!(app.view(), View::Active);
    assert_eq!(app.visible_jobs().count(), 2);
    assert!(rendered(&app, 100, 25).contains("filter All"));
}

#[test]
fn replace_jobs_preserves_selected_identity_across_reorder_and_falls_back_if_removed() {
    let backend = job("Backend Engineer", false, false);
    let platform = job("Platform Engineer", false, false);
    let data = job("Data Engineer", false, false);
    let mut app = App::new(
        config(),
        vec![backend.clone(), platform.clone(), data.clone()],
    );
    app.handle_key(key('j'));

    app.replace_jobs(vec![data.clone(), platform.clone(), backend], 3);
    assert_eq!(app.selected_job().unwrap().key, platform.key);

    app.replace_jobs(vec![data], 1);
    assert_eq!(app.selected_index(), 0);
    assert_eq!(
        app.selected_job().unwrap().classified.observed.title,
        "Data Engineer"
    );
}

#[test]
fn explicit_view_change_keeps_first_row_after_reload_instead_of_stale_identity() {
    let active = job("Active Engineer", false, false);
    let mut reopened = job("Reopened Engineer", false, false);
    reopened.reopened_at = Some(reopened.last_seen_at);
    let mut closed = job("Closed Engineer", false, false);
    closed.source_open = false;
    closed.closed_at = Some(closed.last_seen_at);
    let mut app = App::new(config(), vec![active, reopened.clone()]);
    app.handle_key(key('j'));

    assert_eq!(app.handle_key(key('h')), AppCommand::ReloadJobs);
    app.replace_jobs(vec![closed.clone(), reopened], 1);

    assert_eq!(app.selected_index(), 0);
    assert_eq!(app.selected_job().unwrap().key, closed.key);
}

#[test]
fn fixed_navigation_controls_move_jobs_scroll_details_and_switch_views() {
    let mut app = App::new(
        config(),
        vec![
            job("Backend Engineer", false, false),
            job("Platform Engineer", true, false),
            job("Applied Engineer", false, true),
        ],
    );

    assert_eq!(app.handle_key(special(KeyCode::Down)), AppCommand::None);
    assert_eq!(app.selected_index(), 1);
    assert_eq!(app.handle_key(key('k')), AppCommand::None);
    assert_eq!(app.selected_index(), 0);
    assert_eq!(app.handle_key(key('J')), AppCommand::None);
    assert_eq!(app.detail_scroll(), 1);
    assert_eq!(app.handle_key(key('K')), AppCommand::None);
    assert_eq!(app.detail_scroll(), 0);

    assert_eq!(
        app.handle_key(special(KeyCode::Right)),
        AppCommand::ReloadJobs
    );
    assert_eq!(app.view(), View::New);
    assert_eq!(app.visible_jobs().count(), 1);
    assert_eq!(
        app.handle_key(special(KeyCode::Right)),
        AppCommand::ReloadJobs
    );
    assert_eq!(app.view(), View::Applied);
    assert_eq!(app.visible_jobs().count(), 1);
    assert_eq!(
        app.handle_key(special(KeyCode::Left)),
        AppCommand::ReloadJobs
    );
    assert_eq!(app.view(), View::New);

    assert_eq!(app.handle_key(key('h')), AppCommand::ReloadJobs);
    assert_eq!(app.view(), View::History);
    assert_eq!(
        app.handle_key(special(KeyCode::Right)),
        AppCommand::ReloadJobs
    );
    assert_eq!(app.view(), View::Scans);
    assert_eq!(
        app.handle_key(special(KeyCode::Right)),
        AppCommand::ReloadJobs
    );
    assert_eq!(app.view(), View::Sources);
    assert_eq!(
        app.handle_key(special(KeyCode::Right)),
        AppCommand::ReloadJobs
    );
    assert_eq!(app.view(), View::Active);
}

#[test]
fn fixed_navigation_keys_win_even_if_an_unvalidated_config_reuses_them() {
    let mut configured = config();
    configured.keybindings.scan = "j".into();
    let mut app = App::new(
        configured,
        vec![
            job("Backend Engineer", false, false),
            job("Platform Engineer", false, false),
        ],
    );

    assert_eq!(app.handle_key(key('j')), AppCommand::None);
    assert_eq!(app.selected_index(), 1);
}

#[test]
fn enter_toggles_narrow_details_but_opens_the_official_url_on_wide_terminals() {
    let mut app = App::new(config(), vec![job("Backend Engineer", false, false)]);

    assert_eq!(
        app.handle_key_with_width(special(KeyCode::Enter), 79),
        AppCommand::None
    );
    assert!(app.narrow_details_visible());
    assert!(rendered(&app, 70, 25).contains("Job details"));
    assert_eq!(
        app.handle_key_with_width(special(KeyCode::Esc), 79),
        AppCommand::None
    );
    assert!(!app.narrow_details_visible());
    assert_eq!(
        app.handle_key_with_width(special(KeyCode::Enter), 80),
        AppCommand::OpenUrl("https://example.test/job".into())
    );
}

#[test]
fn scan_events_update_progress_and_health_without_moving_selection() {
    let mut app = App::new(
        config(),
        vec![
            job("Backend Engineer", false, false),
            job("Platform Engineer", false, false),
        ],
    );
    app.handle_key(key('j'));

    app.handle_scan_event(ScanEvent::RunStarted {
        run_id: "run-1".into(),
        company_count: 1,
    });
    app.handle_scan_event(ScanEvent::CompanyStarted {
        company_id: "acme".into(),
    });
    let scanning_buffer = rendered_buffer(&app, 140, 30);
    let scanning: String = scanning_buffer
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect();
    assert!(scanning.contains("SCANNING 0/1"));
    assert!(!scanning.contains("acme scanning"));
    let scanning_x = symbol_x(&scanning_buffer, 29, 0, "↻");
    assert_eq!(
        scanning_buffer.cell((scanning_x, 29)).unwrap().fg,
        Theme::clean_dark().warning
    );
    assert_eq!(app.selected_index(), 1);

    app.handle_scan_event(ScanEvent::CompanyFailed {
        company_id: "acme".into(),
        kind: SourceErrorKind::Timeout,
        diagnostic: "timed out".into(),
    });
    let failed = rendered(&app, 140, 30);
    assert!(!failed.contains("acme timeout"));
    assert_eq!(app.selected_index(), 1);

    app.handle_scan_event(ScanEvent::RunFinished {
        run_id: "run-1".into(),
        completed: 0,
        failed: 1,
        incomplete: 0,
    });
    let failed_buffer = rendered_buffer(&app, 140, 30);
    assert!(row(&failed_buffer, 29).contains("FAILED 1"));
    let failed_x = symbol_x(&failed_buffer, 29, 0, "⚠");
    assert_eq!(
        failed_buffer.cell((failed_x, 29)).unwrap().fg,
        Theme::clean_dark().error
    );
    assert_eq!(app.selected_index(), 1);
}

#[test]
fn renderer_uses_the_specified_responsive_job_layouts_and_status_icons() {
    let app = App::new(
        config(),
        vec![
            job("Backend Engineer", false, false),
            job("Platform Engineer", true, false),
            job("Site Reliability Engineer", false, true),
        ],
    );

    let wide = rendered(&app, 140, 40);
    assert!(
        wide.contains("Navigation") && wide.contains("Active jobs") && wide.contains("Job details")
    );
    let medium = rendered(&app, 100, 30);
    assert!(
        !medium.contains("Navigation")
            && medium.contains("Active jobs")
            && medium.contains("Job details")
    );
    let narrow = rendered(&app, 70, 25);
    assert!(
        !narrow.contains("Navigation")
            && narrow.contains("Active jobs")
            && !narrow.contains("Job details")
    );
    assert!(wide.contains("● OPEN") && wide.contains("✦") && wide.contains("✓"));
}

#[test]
fn theme_overrides_and_ascii_icons_preserve_the_configured_semantics() {
    let overrides = ThemeOverrides {
        background: Some("#010203".into()),
        focused_border: Some("red".into()),
        unfocused_border: Some("green".into()),
        selected_row: Some("blue".into()),
        primary_text: Some("yellow".into()),
        muted_text: Some("magenta".into()),
        open: Some("cyan".into()),
        new: Some("light-red".into()),
        applied: Some("light-green".into()),
        warning: Some("light-yellow".into()),
        error: Some("light-blue".into()),
    };

    let theme = Theme::from_config("clean-light", &overrides);
    assert_eq!(theme.background, Color::Rgb(1, 2, 3));
    assert_eq!(theme.focused_border, Color::Red);
    assert_eq!(theme.unfocused_border, Color::Green);
    assert_eq!(theme.selected_row, Color::Blue);
    assert_eq!(theme.primary_text, Color::Yellow);
    assert_eq!(theme.muted_text, Color::Magenta);
    assert_eq!(theme.open, Color::Cyan);
    assert_eq!(theme.new, Color::LightRed);
    assert_eq!(theme.applied, Color::LightGreen);
    assert_eq!(theme.warning, Color::LightYellow);
    assert_eq!(theme.error, Color::LightBlue);
    assert_eq!(
        IconSet::ascii(),
        IconSet {
            open: "O",
            new: "*",
            applied: "A",
            history: "H",
            scanning: "R",
            source_failure: "!",
        }
    );
}

#[test]
fn footer_keeps_configured_help_as_the_final_visible_hint_at_supported_widths() {
    let mut configured = config();
    configured.keybindings.scan = "s".into();
    configured.keybindings.search = "z".into();
    configured.keybindings.help = "i".into();
    let app = App::new(configured, vec![job("Backend Engineer", false, false)]);

    for width in [70, 100, 140] {
        let buffer = rendered_buffer(&app, width, 25);
        let footer = row(&buffer, 24);
        assert!(footer.contains("s scan"));
        assert!(footer.trim_end().ends_with("i help"), "{footer:?}");
    }
}

#[test]
fn footer_shows_the_configured_copy_fallback_for_job_details() {
    let mut configured = config();
    configured.keybindings.copy = "u".into();
    let mut app = App::new(configured, vec![job("Backend Engineer", false, false)]);

    assert!(rendered(&app, 100, 25).contains("u copy"));
    app.handle_key_with_width(special(KeyCode::Enter), 79);
    assert!(rendered(&app, 79, 25).contains("u copy"));
}

#[test]
fn footer_reserves_the_configured_help_hint_with_a_long_filter_label() {
    let mut configured = config();
    configured.companies[0].name =
        "A company name deliberately longer than the complete footer width".into();
    configured.keybindings.help = "i".into();
    let mut app = App::new(configured, vec![job("Backend Engineer", false, false)]);
    app.handle_key(key('f'));

    for width in [70, 100, 140] {
        let buffer = rendered_buffer(&app, width, 25);
        assert!(row(&buffer, 24).trim_end().ends_with("i help"));
    }
}

#[test]
fn footer_reserves_terminal_cell_width_for_a_wide_help_key() {
    let mut configured = config();
    configured.companies[0].name =
        "A company name deliberately longer than the complete footer width".into();
    configured.keybindings.help = "界".into();
    let mut app = App::new(configured, vec![job("Backend Engineer", false, false)]);
    app.handle_key(key('f'));

    for width in [70, 100, 140] {
        let buffer = rendered_buffer(&app, width, 25);
        assert_eq!(buffer.cell((width - 7, 24)).unwrap().symbol(), "界");
        let suffix = (width - 5..width)
            .map(|x| buffer.cell((x, 24)).unwrap().symbol())
            .collect::<String>();
        assert_eq!(suffix, " help");
    }
}

#[test]
fn active_view_hides_closed_records_supplied_with_open_records() {
    let open = job("Open role", false, false);
    let mut closed = job("Closed role", false, false);
    closed.source_open = false;
    closed.closed_at = Some(closed.last_seen_at);
    let app = App::new(config(), vec![open, closed]);

    let screen = rendered(&app, 140, 40);
    assert!(screen.contains("Open role"));
    assert!(!screen.contains("Closed role"));
}

#[test]
fn history_renders_closed_rows_as_closed_instead_of_open() {
    let mut closed = job("Closed role", false, false);
    closed.source_open = false;
    closed.closed_at = Some(closed.last_seen_at);
    let mut app = App::new(config(), vec![closed]);
    app.handle_key(key('h'));

    let screen = rendered(&app, 100, 25);
    assert!(screen.contains("CLOSED"));
    assert!(!screen.contains("OPEN  Closed role"));
}

#[test]
fn footer_keeps_total_active_count_when_the_loaded_view_is_a_subset() {
    let mut app = App::new(
        config(),
        vec![
            job("Backend Engineer", false, false),
            job("Platform Engineer", true, false),
            job("Applied Engineer", false, true),
        ],
    );

    app.handle_key(special(KeyCode::Right));
    app.replace_jobs(vec![job("Platform Engineer", true, false)], 3);
    assert!(rendered(&app, 100, 25).contains("3 active jobs"));

    app.handle_key(special(KeyCode::Right));
    app.replace_jobs(vec![job("Applied Engineer", false, true)], 3);
    assert!(rendered(&app, 100, 25).contains("3 active jobs"));

    app.handle_key(key('h'));
    app.replace_jobs(vec![], 3);
    assert!(rendered(&app, 100, 25).contains("3 active jobs"));
}

#[test]
fn renderer_shows_active_search_mode_and_query() {
    let mut app = App::new(config(), vec![job("Backend Engineer", false, false)]);
    app.handle_key(key('/'));
    for character in "back".chars() {
        app.handle_key(key(character));
    }

    assert!(rendered(&app, 100, 25).contains("SEARCH back"));
}

#[test]
fn navigation_renders_history_scan_and_source_failure_status_icons() {
    let app = App::new(config(), vec![]);

    let screen = rendered(&app, 140, 40);
    assert!(screen.contains("◷ History"));
    assert!(screen.contains("↻ Scans"));
    assert!(screen.contains("⚠ Sources"));
}

#[test]
fn production_job_buffers_preserve_geometry_styles_and_truth_at_all_breakpoints_and_themes() {
    for theme_name in ["clean-dark", "clean-light"] {
        let mut configured = config();
        configured.ui.theme = theme_name.into();
        let theme = Theme::from_config(theme_name, &Default::default());
        let mut app = App::new(configured, vec![job("Backend Engineer", true, true)]);

        for width in [120, 80, 79] {
            let buffer = rendered_buffer(&app, width, 24);
            let screen: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
            assert!(screen.contains("Acme · Backend Engineer"));
            assert!(!screen.contains("acme"));
            assert!(screen.contains("11 Aug"));
            assert!(row(&buffer, 23).trim_end().ends_with("? help"));

            let list_x = if width >= 120 { 22 } else { 0 };
            let open_x = symbol_x(&buffer, 1, list_x, "●");
            assert_eq!(buffer.cell((open_x, 1)).unwrap().fg, theme.open);
            assert_eq!(buffer.cell((open_x, 1)).unwrap().bg, theme.selected_row);
            let new_x = symbol_x(&buffer, 1, open_x + 1, "✦");
            let applied_x = symbol_x(&buffer, 1, new_x + 1, "✓");
            assert_eq!(buffer.cell((new_x, 1)).unwrap().fg, theme.new);
            assert_eq!(buffer.cell((applied_x, 1)).unwrap().fg, theme.applied);

            match width {
                120 => {
                    assert!(screen.contains("● Active"));
                    assert!(screen.contains("✦ New"));
                    assert!(screen.contains("✓ Applied"));
                    assert_eq!(buffer.cell((18, 0)).unwrap().fg, theme.unfocused_border);
                    assert_eq!(buffer.cell((40, 0)).unwrap().fg, theme.focused_border);
                    assert_eq!(buffer.cell((100, 0)).unwrap().fg, theme.unfocused_border);
                    assert_eq!(buffer.cell((21, 5)).unwrap().symbol(), "│");
                    assert_ne!(buffer.cell((22, 5)).unwrap().symbol(), "│");
                    assert_eq!(buffer.cell((69, 5)).unwrap().symbol(), "│");
                    assert_ne!(buffer.cell((70, 5)).unwrap().symbol(), "│");
                    assert!(screen.contains("Job details"));
                    assert!(screen.contains("Engineering"));
                    assert!(screen.contains("Status"));
                    assert!(screen.contains("First seen"));
                    assert!(screen.contains("Last seen"));
                    assert!(screen.contains("Applied"));
                    assert!(screen.contains("Build reliable systems."));
                }
                80 => {
                    assert!(!screen.contains("Navigation"));
                    assert_eq!(buffer.cell((25, 0)).unwrap().fg, theme.focused_border);
                    assert_eq!(buffer.cell((60, 0)).unwrap().fg, theme.unfocused_border);
                    assert_eq!(buffer.cell((35, 5)).unwrap().symbol(), "│");
                    assert_ne!(buffer.cell((36, 5)).unwrap().symbol(), "│");
                    assert!(screen.contains("Job details"));
                }
                79 => {
                    assert!(!screen.contains("Job details"));
                    assert_eq!(buffer.cell((25, 0)).unwrap().fg, theme.focused_border);
                    app.handle_key_with_width(special(KeyCode::Enter), 79);
                    let details = rendered_buffer(&app, 79, 24);
                    let details_screen: String =
                        details.content().iter().map(|cell| cell.symbol()).collect();
                    assert!(details_screen.contains("Job details"));
                    assert!(details_screen.contains("Acme · Amsterdam · Engineering"));
                    assert_eq!(details.cell((25, 0)).unwrap().fg, theme.focused_border);
                    app.handle_key_with_width(special(KeyCode::Esc), 79);
                }
                _ => unreachable!(),
            }
        }
    }
}

#[test]
fn scans_and_sources_render_durable_semantic_states_at_all_breakpoints_and_themes() {
    let completed_at = Utc.with_ymd_and_hms(2026, 8, 11, 11, 0, 0).unwrap();
    for theme_name in ["clean-dark", "clean-light"] {
        let mut configured = config();
        configured.ui.theme = theme_name.into();
        let theme = Theme::from_config(theme_name, &Default::default());
        let mut app = App::new(configured, vec![]);
        app.replace_read_models(
            vec![ScanReadModel {
                run_id: "run-2".into(),
                company_id: "acme".into(),
                company_name: "Acme".into(),
                completed_at,
                outcome: ScanOutcome::Failed,
                observed_count: 0,
                error_kind: Some(SourceErrorKind::Timeout),
                diagnostic: Some("timed out".into()),
            }],
            vec![SourceReadModel {
                company_id: "acme".into(),
                company_name: "Acme".into(),
                enabled: true,
                latest_attempted_at: Some(completed_at),
                latest_successful_at: None,
                health: SourceHealth::Incomplete,
                latest_error_kind: Some(SourceErrorKind::IncompleteResults),
                diagnostic: Some("unresolved location".into()),
            }],
        );

        for _ in 0..4 {
            assert_eq!(
                app.handle_key(special(KeyCode::Right)),
                AppCommand::ReloadJobs
            );
        }
        assert_eq!(app.view(), View::Scans);
        for width in [120, 80, 79] {
            let buffer = rendered_buffer(&app, width, 24);
            let screen: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
            assert!(screen.contains("FAILED"));
            assert!(screen.contains("Acme"));
            assert!(screen.contains("11 Aug 11:00"));
            assert!(screen.contains("0 observed"));
            assert!(screen.contains("timeout · timed out"));
            let start_x = if width >= 120 { 22 } else { 0 };
            let status_x = symbol_x(&buffer, 1, start_x, "⚠");
            assert_eq!(buffer.cell((status_x, 1)).unwrap().fg, theme.error);
            assert_eq!(
                buffer.cell((25.max(start_x), 0)).unwrap().fg,
                theme.focused_border
            );
        }

        assert_eq!(
            app.handle_key(special(KeyCode::Right)),
            AppCommand::ReloadJobs
        );
        assert_eq!(app.view(), View::Sources);
        for width in [120, 80, 79] {
            let buffer = rendered_buffer(&app, width, 24);
            let screen: String = buffer.content().iter().map(|cell| cell.symbol()).collect();
            assert!(screen.contains("INCOMPLETE"));
            assert!(screen.contains("Acme · enabled"));
            assert!(screen.contains("last attempt 11 Aug 11:00"));
            assert!(screen.contains("last success never"));
            assert!(screen.contains("incomplete-results · unresolved location"));
            let start_x = if width >= 120 { 22 } else { 0 };
            let status_x = symbol_x(&buffer, 1, start_x, "⚠");
            assert_eq!(buffer.cell((status_x, 1)).unwrap().fg, theme.warning);
        }
    }
}

#[test]
fn configured_help_opens_an_overlay_with_fixed_and_contextual_controls() {
    let mut configured = config();
    configured.keybindings.scan = "s".into();
    configured.keybindings.search = "z".into();
    configured.keybindings.filter = "v".into();
    configured.keybindings.toggle_applied = "x".into();
    configured.keybindings.open = "u".into();
    configured.keybindings.copy = "w".into();
    configured.keybindings.history = "y".into();
    configured.keybindings.help = "i".into();
    configured.keybindings.quit = "e".into();
    let mut app = App::new(configured, vec![job("Backend Engineer", false, false)]);

    app.handle_key(key('i'));
    let screen = rendered(&app, 79, 28);
    assert!(screen.contains("←/→ views"));
    assert!(screen.contains("↑/↓ or j/k select"));
    assert!(screen.contains("J/K scroll details"));
    assert!(screen.contains("s scan"));
    assert!(screen.contains("z search jobs"));
    assert!(screen.contains("v filter company/New/Applied"));
    assert!(screen.contains("x applied"));
    assert!(screen.contains("u open"));
    assert!(screen.contains("w copy"));
    assert!(screen.contains("y history"));
    assert!(screen.contains("Enter narrow: details; otherwise: open"));
    assert!(screen.contains("Esc close help"));
    assert!(screen.contains("e quit"));

    app.handle_key(special(KeyCode::Esc));
    assert!(!app.help_visible());
    assert!(rendered(&app, 79, 28).contains("Active jobs"));
}

#[test]
fn operational_rows_remain_keyboard_reachable_when_the_view_exceeds_the_terminal() {
    let mut app = App::new(config(), vec![]);
    let sources = (0..12)
        .map(|index| SourceReadModel {
            company_id: format!("source-{index}"),
            company_name: format!("Source {index}"),
            enabled: true,
            latest_attempted_at: None,
            latest_successful_at: None,
            health: SourceHealth::Unknown,
            latest_error_kind: None,
            diagnostic: None,
        })
        .collect();
    app.replace_read_models(vec![], sources);
    for _ in 0..5 {
        app.handle_key(special(KeyCode::Right));
    }
    for _ in 0..11 {
        app.handle_key(key('j'));
    }

    assert_eq!(app.view(), View::Sources);
    assert_eq!(app.selected_index(), 11);
    let screen = rendered(&app, 79, 12);
    assert!(screen.contains("Source 11"));
    assert!(!screen.contains("Source 0 ·"));
}

#[test]
fn narrow_operational_details_recover_long_required_fields() {
    let at = Utc.with_ymd_and_hms(2026, 8, 11, 11, 0, 0).unwrap();
    let company = "A Company Display Name That Does Not Fit On One Narrow Row";
    let diagnostic =
        "the source returned a diagnostic long enough to require wrapping across terminal rows";
    let mut app = App::new(config(), vec![]);
    app.replace_read_models(
        vec![ScanReadModel {
            run_id: "run-long".into(),
            company_id: "acme".into(),
            company_name: company.into(),
            completed_at: at,
            outcome: ScanOutcome::Incomplete,
            observed_count: 1234,
            error_kind: Some(SourceErrorKind::IncompleteResults),
            diagnostic: Some(diagnostic.into()),
        }],
        vec![SourceReadModel {
            company_id: "acme".into(),
            company_name: company.into(),
            enabled: true,
            latest_attempted_at: Some(at),
            latest_successful_at: None,
            health: SourceHealth::Incomplete,
            latest_error_kind: Some(SourceErrorKind::IncompleteResults),
            diagnostic: Some(diagnostic.into()),
        }],
    );

    for _ in 0..4 {
        app.handle_key(special(KeyCode::Right));
    }
    app.handle_key_with_width(special(KeyCode::Enter), 79);
    let scans = normalized_interior(&rendered_buffer(&app, 79, 20));
    assert!(scans.contains(company));
    assert!(scans.contains("1234 observed"));
    assert!(scans.contains(diagnostic));

    app.handle_key(special(KeyCode::Esc));
    app.handle_key(special(KeyCode::Right));
    app.handle_key_with_width(special(KeyCode::Enter), 79);
    let sources = normalized_interior(&rendered_buffer(&app, 79, 20));
    assert!(sources.contains(company));
    assert!(sources.contains("last attempt 11 Aug 11:00"));
    assert!(sources.contains("last success never"));
    assert!(sources.contains(diagnostic));
}

#[test]
fn idle_footer_uses_durable_enabled_source_health() {
    let mut app = App::new(config(), vec![]);
    app.replace_read_models(
        vec![],
        vec![
            SourceReadModel {
                company_id: "acme".into(),
                company_name: "Acme".into(),
                enabled: true,
                latest_attempted_at: None,
                latest_successful_at: None,
                health: SourceHealth::Incomplete,
                latest_error_kind: Some(SourceErrorKind::IncompleteResults),
                diagnostic: Some("partial".into()),
            },
            SourceReadModel {
                company_id: "removed".into(),
                company_name: "Removed".into(),
                enabled: false,
                latest_attempted_at: None,
                latest_successful_at: None,
                health: SourceHealth::Failed,
                latest_error_kind: Some(SourceErrorKind::Transport),
                diagnostic: Some("offline".into()),
            },
        ],
    );

    assert!(row(&rendered_buffer(&app, 100, 20), 19).contains("INCOMPLETE 1"));
}

#[test]
fn scan_selection_follows_identity_when_newer_rows_are_prepended() {
    let at = Utc.with_ymd_and_hms(2026, 8, 11, 11, 0, 0).unwrap();
    let scan = |run_id: &str| ScanReadModel {
        run_id: run_id.into(),
        company_id: "acme".into(),
        company_name: "Acme".into(),
        completed_at: at,
        outcome: ScanOutcome::Complete,
        observed_count: 1,
        error_kind: None,
        diagnostic: None,
    };
    let mut app = App::new(config(), vec![]);
    app.replace_read_models(vec![scan("new"), scan("selected")], vec![]);
    for _ in 0..4 {
        app.handle_key(special(KeyCode::Right));
    }
    app.handle_key(key('j'));

    app.replace_read_models(vec![scan("newest"), scan("new"), scan("selected")], vec![]);

    assert_eq!(app.scans()[app.selected_index()].run_id, "selected");
}

#[test]
fn lifecycle_details_show_closed_reopened_and_applied_dates() {
    let mut closed = job("Closed Engineer", false, true);
    closed.source_open = false;
    closed.closed_at = Some(Utc.with_ymd_and_hms(2026, 8, 11, 10, 0, 0).unwrap());
    let mut reopened = job("Reopened Engineer", false, true);
    reopened.reopened_at = Some(Utc.with_ymd_and_hms(2026, 8, 11, 11, 0, 0).unwrap());
    let mut app = App::new(config(), vec![closed, reopened]);
    app.handle_key(key('h'));

    let closed_screen = rendered(&app, 120, 24);
    assert!(closed_screen.contains("Closed"));
    assert!(closed_screen.contains("11 Aug 2026 10:00 UTC"));
    assert!(closed_screen.contains("✓ YES · 11 Aug 2026 09:00 UTC"));

    app.handle_key(key('j'));
    let reopened_screen = rendered(&app, 120, 24);
    assert!(reopened_screen.contains("Reopened"));
    assert!(reopened_screen.contains("11 Aug 2026 11:00 UTC"));
}
