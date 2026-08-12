use std::collections::HashMap;

use job_watch::config::{
    CompanyConfig, Config, FiltersConfig, KeybindingsConfig, ScanConfig, SourceConfig,
    ThemeOverrides, UiConfig,
};

fn valid_config() -> Config {
    Config {
        schema_version: 1,
        database_path: ":memory:".into(),
        companies: vec![CompanyConfig {
            id: "mollie".into(),
            name: "Mollie".into(),
            enabled: true,
            location_country_overrides: HashMap::new(),
            source: SourceConfig::Ashby {
                board: "mollie".into(),
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
            timeout_seconds: 20,
            retry_count: 0,
            user_agent: "job-watch-test".into(),
        },
        ui: UiConfig {
            theme: "clean-dark".into(),
            unicode_icons: true,
            theme_overrides: ThemeOverrides::default(),
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

#[test]
fn loads_and_validates_an_ashby_company() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
schema_version = 1
database_path = ".data/jobs.sqlite3"
[scan]
concurrency = 2
timeout_seconds = 20
retry_count = 2
user_agent = "job-watch-test"
[filters]
countries = ["NL"]
include_families = ["software", "platform"]
include_title_patterns = []
exclude_title_patterns = ["manager"]
[ui]
theme = "clean-dark"
unicode_icons = true
[ui.theme_overrides]
focused_border = "green"
[keybindings]
scan = "r"
search = "/"
filter = "f"
toggle_applied = "a"
history = "h"
open = "o"
help = "?"
quit = "q"
[[companies]]
id = "mollie"
name = "Mollie"
enabled = true
[companies.source]
strategy = "ashby"
board = "mollie"
"#,
    )
    .unwrap();

    let config = Config::load(&path).unwrap();
    assert_eq!(config.companies.len(), 1);
    assert!(matches!(
        config.companies[0].source,
        SourceConfig::Ashby { .. }
    ));
}

#[test]
fn accepts_ratatui_named_ansi_colour_variants() {
    let dir = tempfile::tempdir().unwrap();

    for (index, colour) in [
        "darkgray",
        "LightRed",
        "bright-red",
        "dark-grey",
        "light_green",
    ]
    .iter()
    .enumerate()
    {
        let path = dir.path().join(format!("{index}.toml"));
        std::fs::write(
            &path,
            format!(
                r#"
schema_version = 1
database_path = ".data/jobs.sqlite3"
[scan]
concurrency = 2
timeout_seconds = 20
retry_count = 2
user_agent = "job-watch-test"
[filters]
countries = ["NL"]
include_families = []
include_title_patterns = []
exclude_title_patterns = []
[ui]
theme = "clean-dark"
unicode_icons = true
[ui.theme_overrides]
focused_border = "{colour}"
[keybindings]
scan = "r"
search = "/"
filter = "f"
toggle_applied = "a"
history = "h"
open = "o"
help = "?"
quit = "q"
[[companies]]
id = "mollie"
name = "Mollie"
enabled = true
[companies.source]
strategy = "ashby"
board = "mollie"
"#,
            ),
        )
        .unwrap();

        Config::load(&path).unwrap();
    }
}

#[test]
fn rejects_indexed_theme_colours_with_their_field_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        r#"
schema_version = 1
database_path = ".data/jobs.sqlite3"
[scan]
concurrency = 2
timeout_seconds = 20
retry_count = 2
user_agent = "job-watch-test"
[filters]
countries = ["NL"]
include_families = []
include_title_patterns = []
exclude_title_patterns = []
[ui]
theme = "clean-dark"
unicode_icons = true
[ui.theme_overrides]
focused_border = "10"
[keybindings]
scan = "r"
search = "/"
filter = "f"
toggle_applied = "a"
history = "h"
open = "o"
help = "?"
quit = "q"
[[companies]]
id = "mollie"
name = "Mollie"
enabled = true
[companies.source]
strategy = "ashby"
board = "mollie"
"#,
    )
    .unwrap();

    let error = Config::load(&path).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("ui.theme_overrides.focused_border")
    );
}

#[test]
fn rejects_action_bindings_that_are_not_exactly_one_character() {
    for binding in ["", "scan"] {
        let mut config = valid_config();
        config.keybindings.scan = binding.into();

        let error = config.validate().unwrap_err();
        assert!(error.to_string().contains("keybindings.scan"));
        assert!(error.to_string().contains("exactly one character"));
    }
}

#[test]
fn rejects_action_bindings_that_collide_with_fixed_navigation_keys() {
    for binding in ["j", "k", "J", "K"] {
        let mut config = valid_config();
        config.keybindings.scan = binding.into();

        let error = config.validate().unwrap_err();
        assert!(error.to_string().contains("keybindings.scan"));
        assert!(error.to_string().contains("fixed navigation"));
    }
}
