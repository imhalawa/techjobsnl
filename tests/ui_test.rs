use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use job_watch::{
    config::{
        CompanyConfig, Config, FiltersConfig, KeybindingsConfig, ScanConfig, SourceConfig,
        ThemeOverrides, UiConfig,
    },
    domain::{ClassifiedJob, Eligibility, JobKey, JobRecord, ObservedJob},
    ui::{App, IconSet, Theme, render},
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

fn job(title: &str, is_new: bool, applied: bool) -> JobRecord {
    let seen_at = Utc.with_ymd_and_hms(2026, 8, 11, 9, 0, 0).unwrap();
    JobRecord {
        key: JobKey::new("acme", title),
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
