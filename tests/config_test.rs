use job_watch::config::{Config, SourceConfig};

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
