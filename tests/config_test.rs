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

#[test]
fn rejects_control_character_action_bindings_that_key_dispatch_cannot_reach() {
    for binding in ["\r", "\t", "\u{1b}", "\u{7f}"] {
        let mut config = valid_config();
        config.keybindings.scan = binding.into();

        let error = config.validate().unwrap_err();
        assert!(error.to_string().contains("keybindings.scan"));
        assert!(error.to_string().contains("non-control"));
    }
}

#[test]
fn parses_and_validates_every_source_strategy() {
    let cases = [
        ("strategy = \"ashby\"\nboard = \"mollie\"", true),
        ("strategy = \"greenhouse\"\nboard = \"adyen\"", true),
        (
            "strategy = \"jibe\"\nbase_url = \"https://jobs.booking.com\"\nclient = \"Booking.com\"",
            true,
        ),
        (
            "strategy = \"recruitee\"\nbase_url = \"https://jobs.funda.nl\"",
            true,
        ),
        (
            "strategy = \"bol\"\nbase_url = \"https://careers.bol.com\"",
            true,
        ),
        (
            "strategy = \"ing\"\nlisting_url = \"https://careers.ing.com/en/search-jobs\"",
            true,
        ),
        (
            "strategy = \"getnoticed\"\nbase_url = \"https://www.werkenbijabnamro.nl\"\ncountry_filter = \"Nederland\"",
            true,
        ),
        (
            "strategy = \"paged-html\"\nlisting_url = \"https://www.exact.com/careers/vacancies\"\noffset_parameter = \"limitstart\"\npage_size = 20",
            true,
        ),
        (
            "strategy = \"unsupported\"\nreason = \"official source blocks unattended requests\"",
            false,
        ),
    ];

    for (raw, enabled) in cases {
        let source: SourceConfig = toml::from_str(raw).unwrap();
        let mut config = valid_config();
        config.companies[0].enabled = enabled;
        config.companies[0].source = source;
        config.validate().unwrap();
    }
}

#[test]
fn rejects_empty_source_fields() {
    let cases = [
        SourceConfig::Ashby { board: " ".into() },
        SourceConfig::Greenhouse { board: "".into() },
        SourceConfig::Jibe {
            base_url: "https://jobs.booking.com".into(),
            client: " ".into(),
        },
        SourceConfig::Jibe {
            base_url: " ".into(),
            client: "Booking.com".into(),
        },
        SourceConfig::Recruitee {
            base_url: "".into(),
        },
        SourceConfig::Bol {
            base_url: " ".into(),
        },
        SourceConfig::Ing {
            listing_url: "".into(),
        },
        SourceConfig::Getnoticed {
            base_url: "https://www.werkenbijabnamro.nl".into(),
            country_filter: Some(" ".into()),
        },
        SourceConfig::Getnoticed {
            base_url: "".into(),
            country_filter: None,
        },
        SourceConfig::PagedHtml {
            listing_url: "https://www.exact.com/careers/vacancies".into(),
            offset_parameter: "".into(),
            page_size: 20,
        },
        SourceConfig::PagedHtml {
            listing_url: " ".into(),
            offset_parameter: "limitstart".into(),
            page_size: 20,
        },
        SourceConfig::Unsupported { reason: " ".into() },
    ];

    for source in cases {
        let mut config = valid_config();
        config.companies[0].enabled = false;
        config.companies[0].source = source;
        assert!(config.validate().is_err());
    }
}

#[test]
fn rejects_non_https_source_urls() {
    let cases = [
        SourceConfig::Jibe {
            base_url: "http://jobs.booking.com".into(),
            client: "Booking.com".into(),
        },
        SourceConfig::Recruitee {
            base_url: "jobs.funda.nl".into(),
        },
        SourceConfig::Bol {
            base_url: "http://careers.bol.com".into(),
        },
        SourceConfig::Ing {
            listing_url: "http://careers.ing.com/en/search-jobs".into(),
        },
        SourceConfig::Getnoticed {
            base_url: "http://www.werkenbijabnamro.nl".into(),
            country_filter: None,
        },
        SourceConfig::PagedHtml {
            listing_url: "http://www.exact.com/careers/vacancies".into(),
            offset_parameter: "limitstart".into(),
            page_size: 20,
        },
    ];

    for source in cases {
        let mut config = valid_config();
        config.companies[0].source = source;
        let error = config.validate().unwrap_err();
        assert!(error.to_string().contains("must be an HTTPS URL"));
    }
}

#[test]
fn rejects_zero_paged_html_page_size() {
    let mut config = valid_config();
    config.companies[0].source = SourceConfig::PagedHtml {
        listing_url: "https://www.exact.com/careers/vacancies".into(),
        offset_parameter: "limitstart".into(),
        page_size: 0,
    };

    let error = config.validate().unwrap_err();
    assert!(error.to_string().contains("source.page_size"));
}

#[test]
fn rejects_unsupported_strategy_for_enabled_company() {
    let mut config = valid_config();
    config.companies[0].source = SourceConfig::Unsupported {
        reason: "official source blocks unattended requests".into(),
    };

    let error = config.validate().unwrap_err();
    assert!(error.to_string().contains("source.strategy"));
    assert!(error.to_string().contains("disabled"));
}
