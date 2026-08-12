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
    ui::{App, AppCommand, IconSet, InputMode, Theme, View, render},
};
use ratatui::{Terminal, backend::TestBackend, style::Color};
use serde_json::json;

fn rendered(app: &App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|frame| render(frame, app)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
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
            help: "?".into(),
            quit: "q".into(),
        },
    }
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
    assert_eq!(app.handle_key(key('v')), AppCommand::ReloadJobs);
    assert_eq!(app.view(), View::New);
    assert_eq!(app.handle_key(key('y')), AppCommand::ReloadJobs);
    assert_eq!(app.view(), View::History);
    assert_eq!(app.handle_key(key('i')), AppCommand::None);
    assert!(app.help_visible());
    assert_eq!(app.handle_key(key('e')), AppCommand::Quit);
}

#[test]
fn search_accepts_input_filters_by_title_or_company_and_escape_cancels() {
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
    let mut app = App::new(
        configured,
        vec![
            job("Backend Engineer", false, false),
            job_for("beta", "Platform Engineer", false, false),
        ],
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
    assert_eq!(app.handle_key(key('h')), AppCommand::ReloadJobs);
    assert_eq!(app.view(), View::Active);
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
    let scanning = rendered(&app, 140, 30);
    assert!(scanning.contains("SCANNING 0/1"));
    assert!(scanning.contains("acme scanning"));
    assert_eq!(app.selected_index(), 1);

    app.handle_scan_event(ScanEvent::CompanyFailed {
        company_id: "acme".into(),
        kind: SourceErrorKind::Timeout,
        diagnostic: "timed out".into(),
    });
    let failed = rendered(&app, 140, 30);
    assert!(failed.contains("acme timeout"));
    assert_eq!(app.selected_index(), 1);

    app.handle_scan_event(ScanEvent::RunFinished {
        run_id: "run-1".into(),
        completed: 0,
        failed: 1,
        incomplete: 0,
    });
    assert!(rendered(&app, 140, 30).contains("FAILED 1"));
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
    assert!(wide.contains("● OPEN") && wide.contains("✦ NEW") && wide.contains("✓ APPLIED"));
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
fn footer_uses_configured_key_hints_and_job_counts() {
    let mut configured = config();
    configured.keybindings.scan = "s".into();
    configured.keybindings.search = "?".into();
    let app = App::new(configured, vec![job("Backend Engineer", false, false)]);

    let screen = rendered(&app, 70, 25);
    assert!(screen.contains("s scan  ? search"));
    assert!(screen.contains("1 companies  1 active jobs"));
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
fn navigation_renders_history_scan_and_source_failure_status_icons() {
    let app = App::new(config(), vec![]);

    let screen = rendered(&app, 140, 40);
    assert!(screen.contains("◷ History"));
    assert!(screen.contains("↻ Scans"));
    assert!(screen.contains("⚠ Sources"));
}
