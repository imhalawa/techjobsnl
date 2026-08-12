use std::{
    error::Error,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Utc;
use crossterm::event::{Event, EventStream};
use futures_util::StreamExt;
use job_watch::{
    config::{Config, SourceConfig},
    domain::ScanEvent,
    filter::EligibilityFilter,
    scanner::ScanService,
    sources::{JobSource, ashby, ebay, jibe},
    storage::{JobQuery, Store},
    ui::{App, AppCommand, View, render},
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Platform {
    Windows,
    Wsl,
    Macos,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CommandSpec {
    program: &'static str,
    args: &'static [&'static str],
}

const NO_ARGS: &[&str] = &[];
const XCLIP_ARGS: &[&str] = &["-selection", "clipboard"];
const XSEL_ARGS: &[&str] = &["--clipboard", "--input"];
const WINDOWS_CLIPBOARD: &[CommandSpec] = &[CommandSpec {
    program: "clip.exe",
    args: NO_ARGS,
}];
const MACOS_CLIPBOARD: &[CommandSpec] = &[CommandSpec {
    program: "pbcopy",
    args: NO_ARGS,
}];
const LINUX_CLIPBOARD: &[CommandSpec] = &[
    CommandSpec {
        program: "wl-copy",
        args: NO_ARGS,
    },
    CommandSpec {
        program: "xclip",
        args: XCLIP_ARGS,
    },
    CommandSpec {
        program: "xsel",
        args: XSEL_ARGS,
    },
];

#[tokio::main]
async fn main() -> Result<()> {
    let Startup {
        mut app,
        store,
        scan_service,
    } = initialize(&std::env::current_dir()?)?;
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, store, scan_service).await;
    finish_with_restore(result, ratatui::restore)
}

struct Startup {
    app: App,
    store: Arc<Mutex<Store>>,
    scan_service: Arc<ScanService>,
}

fn initialize(working_directory: &Path) -> Result<Startup> {
    let config_path = std::path::absolute(working_directory.join("config.toml"))?;
    let config = Config::load(&config_path).map_err(|source| {
        std::io::Error::other(format!(
            "could not initialize from configuration {}: {source}",
            config_path.display()
        ))
    })?;
    let database_path = database_path(
        config_path
            .parent()
            .expect("config.toml always has its working-directory parent"),
        &config.database_path,
    );
    if let Some(parent) = database_path.parent() {
        fs::create_dir_all(parent).map_err(|source| {
            std::io::Error::other(format!(
                "could not create parent for database {}: {source}",
                database_path.display()
            ))
        })?;
    }
    let mut store = Store::open(&database_path).map_err(|source| {
        std::io::Error::other(format!(
            "could not open database {}: {source}",
            database_path.display()
        ))
    })?;
    store.sync_companies(&config.companies)?;
    let store = Arc::new(Mutex::new(store));
    let scan_service = Arc::new(scan_service(&config, Arc::clone(&store)).map_err(|source| {
        std::io::Error::other(format!(
            "could not initialize from configuration {}: {source}",
            config_path.display()
        ))
    })?);
    let (jobs, scans, sources) = {
        let store = store.lock().unwrap();
        (
            store.list_jobs(JobQuery::active())?,
            store.recent_scans()?,
            store.source_health()?,
        )
    };
    let mut app = App::new(config, jobs);
    app.replace_read_models(scans, sources);
    Ok(Startup {
        app,
        store,
        scan_service,
    })
}

fn database_path(working_directory: &Path, configured_path: &str) -> PathBuf {
    let path = Path::new(configured_path);
    if path.is_absolute() {
        path.into()
    } else {
        working_directory.join(path)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandEffect {
    Continue,
    StartScan,
    Quit,
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    store: Arc<Mutex<Store>>,
    scan_service: Arc<ScanService>,
) -> Result<()> {
    let mut events = EventStream::new();
    let (scan_tx, mut scan_rx) = mpsc::unbounded_channel();
    let mut active_scan = None;

    loop {
        terminal.draw(|frame| render(frame, app))?;
        tokio::select! {
            event = events.next() => {
                let Some(event) = event else { return Ok(()) };
                let event = event?;
                if let Event::Key(key) = event {
                    let width = terminal.size()?.width;
                    let command = app.handle_key_with_width(key, width);
                    let mut open_job_url = open_url;
                    let mut copy_job_url = copy_url;
                    match execute_command(
                        command,
                        &store,
                        app,
                        &mut open_job_url,
                        &mut copy_job_url,
                    )? {
                        CommandEffect::StartScan => {
                            start_scan(
                                &mut active_scan,
                                Arc::clone(&scan_service),
                                scan_tx.clone(),
                            );
                        }
                        CommandEffect::Quit => {
                            abort_scan(&mut active_scan).await;
                            break;
                        }
                        CommandEffect::Continue => {}
                    }
                }
            }
            Some(event) = scan_rx.recv() => {
                handle_runtime_scan_event(event, &store, app)?;
            }
            scan_result = async {
                active_scan.as_mut().expect("guarded by select branch").await
            }, if active_scan.is_some() => {
                finish_scan(&mut active_scan, scan_result)?;
                reload_jobs(&store, app)?;
            }
        }
    }
    Ok(())
}

fn handle_runtime_scan_event(
    event: ScanEvent,
    store: &Arc<Mutex<Store>>,
    app: &mut App,
) -> rusqlite::Result<()> {
    let company_finished = matches!(
        &event,
        ScanEvent::CompanyCompleted { .. }
            | ScanEvent::CompanyFailed { .. }
            | ScanEvent::CompanyIncomplete { .. }
    );
    app.handle_scan_event(event);
    if company_finished {
        reload_jobs(store, app)?;
    }
    Ok(())
}

fn scan_service(config: &Config, store: Arc<Mutex<Store>>) -> Result<ScanService> {
    let sources = build_sources(config)?;
    Ok(ScanService::new(
        sources,
        EligibilityFilter::new(&config.filters)?,
        config.companies.clone(),
        store,
        config.scan.clone(),
    ))
}

fn build_sources(config: &Config) -> Result<Vec<Arc<dyn JobSource>>> {
    let client = ashby::build_client(
        &config.scan.user_agent,
        Duration::from_secs(config.scan.timeout_seconds),
    )?;
    let ebay_client = ebay::build_client(
        &config.scan.user_agent,
        Duration::from_secs(config.scan.timeout_seconds),
    )?;
    config
        .companies
        .iter()
        .filter(|company| company.enabled)
        .map(|company| -> Result<Arc<dyn JobSource>> {
            match &company.source {
                SourceConfig::Ashby { board } => Ok(Arc::new(ashby::AshbySource::new(
                    &company.id,
                    board,
                    client.clone(),
                ))),
                SourceConfig::Jibe {
                    base_url,
                    client: brand,
                } => Ok(Arc::new(jibe::JibeSource::new(
                    &company.id,
                    base_url,
                    brand,
                    client.clone(),
                ))),
                SourceConfig::Ebay { listing_url } => Ok(Arc::new(ebay::EbaySource::new(
                    &company.id,
                    listing_url,
                    ebay_client.clone(),
                ))),
                _ => Err(std::io::Error::other(format!(
                    "source strategy for {} is not wired yet",
                    company.id
                ))
                .into()),
            }
        })
        .collect()
}

fn reload_jobs(store: &Arc<Mutex<Store>>, app: &mut App) -> rusqlite::Result<()> {
    let query = match app.view() {
        View::Active => JobQuery::active(),
        View::New => JobQuery::new(),
        View::Applied => JobQuery::applied(),
        View::History => JobQuery::history(),
        View::Scans | View::Sources => JobQuery::all(),
    };
    let store = store.lock().unwrap();
    let jobs = store.list_jobs(query)?;
    let active_job_count = store.list_jobs(JobQuery::active())?.len();
    let scans = store.recent_scans()?;
    let sources = store.source_health()?;
    drop(store);
    app.replace_jobs(jobs, active_job_count);
    app.replace_read_models(scans, sources);
    Ok(())
}

fn execute_command(
    command: AppCommand,
    store: &Arc<Mutex<Store>>,
    app: &mut App,
    open_url: &mut impl FnMut(&str) -> Result<()>,
    copy_url: &mut impl FnMut(&str) -> Result<()>,
) -> Result<CommandEffect> {
    match command {
        AppCommand::ToggleApplied(key) => {
            store.lock().unwrap().toggle_applied(&key, Utc::now())?;
            reload_jobs(store, app)?;
            Ok(CommandEffect::Continue)
        }
        AppCommand::OpenUrl(url) => {
            open_url(&url)?;
            Ok(CommandEffect::Continue)
        }
        AppCommand::CopyUrl(url) => {
            copy_url(&url)?;
            Ok(CommandEffect::Continue)
        }
        AppCommand::StartScan => Ok(CommandEffect::StartScan),
        AppCommand::ReloadJobs => {
            reload_jobs(store, app)?;
            Ok(CommandEffect::Continue)
        }
        AppCommand::Quit => Ok(CommandEffect::Quit),
        AppCommand::None => Ok(CommandEffect::Continue),
    }
}

fn current_platform() -> Platform {
    if cfg!(target_os = "windows") {
        Platform::Windows
    } else if cfg!(target_os = "macos") {
        Platform::Macos
    } else if is_wsl() {
        Platform::Wsl
    } else {
        Platform::Linux
    }
}

fn is_wsl() -> bool {
    std::env::var_os("WSL_DISTRO_NAME").is_some()
        || std::env::var_os("WSL_INTEROP").is_some()
        || fs::read_to_string("/proc/sys/kernel/osrelease")
            .is_ok_and(|release| release.to_ascii_lowercase().contains("microsoft"))
}

fn browser_command(platform: Platform) -> Option<CommandSpec> {
    (platform == Platform::Wsl).then_some(CommandSpec {
        program: "explorer.exe",
        args: NO_ARGS,
    })
}

fn open_url(url: &str) -> Result<()> {
    let platform = current_platform();
    let Some(command) = browser_command(platform) else {
        return opener::open_browser(url).map_err(Into::into);
    };
    let status = Command::new(command.program)
        .args(command.args)
        .arg(url)
        .status()
        .map_err(|source| {
            io::Error::new(
                source.kind(),
                format!("could not open URL with {}: {source}", command.program),
            )
        })?;
    if !status.success() {
        return Err(io::Error::other(format!(
            "could not open URL with {}: exited with {status}",
            command.program
        ))
        .into());
    }
    Ok(())
}

fn clipboard_commands(platform: Platform) -> &'static [CommandSpec] {
    match platform {
        Platform::Windows | Platform::Wsl => WINDOWS_CLIPBOARD,
        Platform::Macos => MACOS_CLIPBOARD,
        Platform::Linux => LINUX_CLIPBOARD,
    }
}

fn copy_url(url: &str) -> Result<()> {
    copy_url_with(url, current_platform(), pipe_to_command)
}

fn copy_url_with(
    url: &str,
    platform: Platform,
    mut pipe: impl FnMut(CommandSpec, &str) -> io::Result<()>,
) -> Result<()> {
    let commands = clipboard_commands(platform);
    let mut errors = Vec::with_capacity(commands.len());
    for command in commands {
        match pipe(*command, url) {
            Ok(()) => return Ok(()),
            Err(source) => errors.push(format!("{}: {source}", command.program)),
        }
    }
    Err(io::Error::other(format!(
        "could not copy URL: all clipboard utilities failed ({})",
        errors.join("; ")
    ))
    .into())
}

fn pipe_to_command(command: CommandSpec, contents: &str) -> io::Result<()> {
    let mut child = Command::new(command.program)
        .args(command.args)
        .stdin(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .expect("piped clipboard stdin must be available")
        .write_all(contents.as_bytes())?;
    let status = child.wait()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("exited with {status}")))
    }
}

fn start_scan(
    active_scan: &mut Option<JoinHandle<()>>,
    scan_service: Arc<ScanService>,
    scan_tx: mpsc::UnboundedSender<ScanEvent>,
) -> bool {
    if active_scan.is_some() {
        return false;
    }
    let run_id = format!("scan-{}", Utc::now().timestamp_micros());
    *active_scan = Some(tokio::spawn(async move {
        scan_service.run(run_id, scan_tx).await;
    }));
    true
}

async fn abort_scan(active_scan: &mut Option<JoinHandle<()>>) {
    if let Some(handle) = active_scan.take() {
        handle.abort();
        let _ = handle.await;
    }
}

fn finish_scan(
    active_scan: &mut Option<JoinHandle<()>>,
    result: std::result::Result<(), tokio::task::JoinError>,
) -> Result<()> {
    *active_scan = None;
    result?;
    Ok(())
}

fn finish_with_restore<T>(result: Result<T>, restore: impl FnOnce()) -> Result<T> {
    restore();
    result
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use chrono::{TimeZone, Utc};
    use job_watch::{
        config::{
            CompanyConfig, Config, FiltersConfig, KeybindingsConfig, ScanConfig, SourceConfig,
            UiConfig,
        },
        domain::{
            ClassifiedJob, Eligibility, JobKey, ObservedJob, ScanEvent, ScanFailure,
            SourceErrorKind, SourceScan,
        },
        filter::EligibilityFilter,
        scanner::ScanService,
        sources::{JobSource, SourceError},
        storage::{JobQuery, Store},
        ui::{App, AppCommand},
    };
    use serde_json::json;
    use tokio::sync::mpsc;

    use super::{
        CommandEffect, Platform, abort_scan, browser_command, build_sources, copy_url_with,
        execute_command, finish_scan, finish_with_restore, handle_runtime_scan_event, initialize,
        start_scan,
    };

    #[test]
    fn production_config_keeps_rabobank_disabled_and_unschedulable() {
        let config = Config::load(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")).unwrap();
        let rabobank = config
            .companies
            .iter()
            .filter(|company| company.id == "rabobank")
            .collect::<Vec<_>>();

        assert_eq!(rabobank.len(), 1);
        assert!(!rabobank[0].enabled);
        assert!(matches!(
            &rabobank[0].source,
            SourceConfig::Unsupported { reason }
                if reason == "official sources reject unattended access (HTTP 403)"
        ));

        let enabled_ids = config
            .companies
            .iter()
            .filter(|company| company.enabled)
            .map(|company| company.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(enabled_ids, ["mollie", "booking-com", "ebay", "airwallex"]);

        let source_ids = build_sources(&config)
            .unwrap()
            .into_iter()
            .map(|source| source.company_id().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(source_ids, enabled_ids);
    }

    struct CompleteSource;

    #[async_trait::async_trait]
    impl JobSource for CompleteSource {
        fn company_id(&self) -> &str {
            "acme"
        }

        async fn scan(&self) -> std::result::Result<SourceScan, SourceError> {
            Ok(SourceScan::Complete {
                observations: vec![observed("job-1")],
            })
        }
    }

    struct HangingSource;

    #[async_trait::async_trait]
    impl JobSource for HangingSource {
        fn company_id(&self) -> &str {
            "acme"
        }

        async fn scan(&self) -> std::result::Result<SourceScan, SourceError> {
            std::future::pending().await
        }
    }

    fn company() -> CompanyConfig {
        CompanyConfig {
            id: "acme".into(),
            name: "Acme".into(),
            enabled: true,
            location_country_overrides: HashMap::new(),
            source: SourceConfig::Ashby {
                board: "acme".into(),
            },
        }
    }

    fn config() -> Config {
        Config {
            schema_version: 1,
            database_path: ":memory:".into(),
            companies: vec![company()],
            filters: FiltersConfig {
                countries: vec!["NL".into()],
                include_families: vec!["software".into()],
                include_title_patterns: vec![],
                exclude_title_patterns: vec![],
            },
            scan: ScanConfig {
                concurrency: 1,
                timeout_seconds: 30,
                retry_count: 0,
                user_agent: "job-watch-test".into(),
            },
            ui: UiConfig {
                theme: "clean-dark".into(),
                unicode_icons: false,
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

    fn observed(source_id: &str) -> ObservedJob {
        ObservedJob {
            source_id: source_id.into(),
            title: "Software Engineer".into(),
            department: None,
            team: None,
            employment_type: None,
            locations: vec!["Amsterdam".into()],
            countries: vec!["NL".into()],
            job_url: "https://example.test/job".into(),
            apply_url: "https://example.test/apply".into(),
            description: "Build systems.".into(),
            raw_payload: json!({}),
            published_at: None,
        }
    }

    fn store_with_job() -> Arc<Mutex<Store>> {
        let configured = company();
        let mut store = Store::open_in_memory().unwrap();
        store
            .sync_companies(std::slice::from_ref(&configured))
            .unwrap();
        let at = Utc.with_ymd_and_hms(2026, 8, 12, 9, 0, 0).unwrap();
        store
            .record_complete_scan(
                "setup",
                &configured,
                &[ClassifiedJob {
                    observed: observed("job-1"),
                    eligibility: Eligibility {
                        eligible: true,
                        reason: "eligible".into(),
                    },
                }],
                at,
                at,
            )
            .unwrap();
        Arc::new(Mutex::new(store))
    }

    fn service(source: Arc<dyn JobSource>, store: Arc<Mutex<Store>>) -> Arc<ScanService> {
        let configured = config();
        Arc::new(ScanService::new(
            vec![source],
            EligibilityFilter::new(&configured.filters).unwrap(),
            configured.companies,
            store,
            configured.scan,
        ))
    }

    #[test]
    fn applied_command_persists_and_reloads_the_real_store() {
        let store = store_with_job();
        let jobs = store.lock().unwrap().list_jobs(JobQuery::active()).unwrap();
        let mut app = App::new(config(), jobs);
        let mut opened = |_: &str| -> super::Result<()> { Ok(()) };
        let mut copied = |_: &str| -> super::Result<()> { Ok(()) };

        let effect = execute_command(
            AppCommand::ToggleApplied(JobKey::new("acme", "job-1")),
            &store,
            &mut app,
            &mut opened,
            &mut copied,
        )
        .unwrap();

        assert_eq!(effect, CommandEffect::Continue);
        assert!(app.selected_job().unwrap().applied_at.is_some());
        assert_eq!(
            store
                .lock()
                .unwrap()
                .list_jobs(JobQuery::applied())
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn duplicate_scan_guard_releases_only_after_actual_completion() {
        let store = store_with_job();
        let scan_service = service(Arc::new(CompleteSource), store);
        let (tx, mut rx) = mpsc::unbounded_channel();
        let mut active = None;

        assert!(start_scan(
            &mut active,
            Arc::clone(&scan_service),
            tx.clone()
        ));
        assert!(!start_scan(
            &mut active,
            Arc::clone(&scan_service),
            tx.clone()
        ));
        active.as_mut().unwrap().await.unwrap();
        active = None;
        assert!(active.is_none());
        let mut finished = false;
        while let Ok(event) = rx.try_recv() {
            finished |= matches!(event, job_watch::domain::ScanEvent::RunFinished { .. });
        }
        assert!(finished);
        assert!(start_scan(&mut active, scan_service, tx));
        abort_scan(&mut active).await;
    }

    #[tokio::test]
    async fn aborting_an_active_scan_does_not_wait_for_a_hanging_source() {
        let store = store_with_job();
        let scan_service = service(Arc::new(HangingSource), store);
        let (tx, _rx) = mpsc::unbounded_channel();
        let mut active = None;
        assert!(start_scan(&mut active, scan_service, tx));

        tokio::time::timeout(Duration::from_millis(100), abort_scan(&mut active))
            .await
            .unwrap();
        assert!(active.is_none());
    }

    #[tokio::test]
    async fn scan_panic_releases_the_guard_and_surfaces_an_error() {
        let mut active = Some(tokio::spawn(async { panic!("scan task failed") }));
        let join_result = active.as_mut().unwrap().await;

        let result = finish_scan(&mut active, join_result);

        assert!(result.is_err());
        assert!(active.is_none());
    }

    #[test]
    fn terminal_restore_runs_before_an_event_loop_error_is_returned() {
        let store = store_with_job();
        let jobs = store.lock().unwrap().list_jobs(JobQuery::active()).unwrap();
        let mut app = App::new(config(), jobs);
        let mut opened = |_: &str| -> super::Result<()> { Ok(()) };
        let mut copied = |_: &str| -> super::Result<()> { Ok(()) };
        let restored = std::cell::Cell::new(false);
        let result = execute_command(
            AppCommand::ToggleApplied(JobKey::new("acme", "missing")),
            &store,
            &mut app,
            &mut opened,
            &mut copied,
        )
        .map(|_| ());

        let returned = finish_with_restore(result, || restored.set(true));

        assert!(returned.is_err());
        assert!(
            returned
                .unwrap_err()
                .to_string()
                .contains("Query returned no rows")
        );
        assert!(restored.get());
    }

    #[test]
    fn startup_reports_the_absolute_working_directory_config_path() {
        let directory = tempfile::tempdir().unwrap();

        let error = match initialize(directory.path()) {
            Ok(_) => panic!("startup unexpectedly succeeded without config.toml"),
            Err(error) => error,
        };

        assert!(
            error
                .to_string()
                .contains(&directory.path().join("config.toml").display().to_string())
        );
    }

    #[test]
    fn startup_invalid_configuration_reports_the_absolute_config_path() {
        let directory = tempfile::tempdir().unwrap();
        let config =
            include_str!("../config.toml").replace("schema_version = 1", "schema_version = 2");
        std::fs::write(directory.path().join("config.toml"), config).unwrap();

        let error = match initialize(directory.path()) {
            Ok(_) => panic!("startup unexpectedly accepted an invalid configuration"),
            Err(error) => error,
        };
        let diagnostic = error.to_string();

        assert!(diagnostic.contains("schema_version"));
        assert!(diagnostic.contains(&directory.path().join("config.toml").display().to_string()));
    }

    #[test]
    fn startup_config_derived_failure_reports_the_absolute_config_path() {
        let directory = tempfile::tempdir().unwrap();
        let config = include_str!("../config.toml").replace(
            "include_families = [\"software\", \"platform\", \"sre\", \"data\", \"ml\", \"application-security\"]",
            "include_families = [\"unknown-family\"]",
        );
        std::fs::write(directory.path().join("config.toml"), config).unwrap();

        let error = match initialize(directory.path()) {
            Ok(_) => panic!("startup unexpectedly accepted an unknown filter family"),
            Err(error) => error,
        };
        let diagnostic = error.to_string();

        assert!(diagnostic.contains("unknown included family"));
        assert!(diagnostic.contains(&directory.path().join("config.toml").display().to_string()));
    }

    #[test]
    fn startup_database_parent_failure_reports_the_absolute_database_path() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("config.toml"),
            include_str!("../config.toml"),
        )
        .unwrap();
        std::fs::write(directory.path().join(".data"), "not a directory").unwrap();

        let error = match initialize(directory.path()) {
            Ok(_) => panic!("startup unexpectedly created a database below a file"),
            Err(error) => error,
        };

        assert!(
            error.to_string().contains(
                &directory
                    .path()
                    .join(".data/job-watch.sqlite3")
                    .display()
                    .to_string()
            )
        );
    }

    #[test]
    fn startup_database_open_failure_reports_the_absolute_database_path() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("config.toml"),
            include_str!("../config.toml"),
        )
        .unwrap();
        std::fs::create_dir_all(directory.path().join(".data/job-watch.sqlite3")).unwrap();

        let error = match initialize(directory.path()) {
            Ok(_) => panic!("startup unexpectedly opened a directory as a database"),
            Err(error) => error,
        };

        assert!(
            error.to_string().contains(
                &directory
                    .path()
                    .join(".data/job-watch.sqlite3")
                    .display()
                    .to_string()
            )
        );
    }

    #[test]
    fn startup_creates_the_database_parent_and_loads_stored_active_jobs_without_scanning() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("config.toml"),
            include_str!("../config.toml"),
        )
        .unwrap();

        let startup = initialize(directory.path()).unwrap();
        assert!(directory.path().join(".data").is_dir());
        let company = startup.app.config().companies[0].clone();
        let at = Utc.with_ymd_and_hms(2026, 8, 12, 10, 0, 0).unwrap();
        startup
            .store
            .lock()
            .unwrap()
            .record_complete_scan(
                "seed",
                &company,
                &[ClassifiedJob {
                    observed: observed("stored"),
                    eligibility: Eligibility {
                        eligible: true,
                        reason: "eligible".into(),
                    },
                }],
                at,
                at,
            )
            .unwrap();
        drop(startup);

        let startup = initialize(directory.path()).unwrap();
        let stored = startup.app.selected_job().unwrap();
        assert_eq!(stored.key, JobKey::new("mollie", "stored"));
        assert!(stored.is_new, "startup must not perform an implicit scan");
    }

    #[test]
    fn company_completion_event_refreshes_durable_scan_and_source_read_models() {
        let store = store_with_job();
        let jobs = store.lock().unwrap().list_jobs(JobQuery::active()).unwrap();
        let mut app = App::new(config(), jobs);
        let configured = company();
        store
            .lock()
            .unwrap()
            .record_failed_scan(
                "run-failed",
                &configured,
                &ScanFailure {
                    kind: SourceErrorKind::Transport,
                    diagnostic: "connection reset".into(),
                },
                Utc.with_ymd_and_hms(2026, 8, 12, 10, 0, 0).unwrap(),
                Utc.with_ymd_and_hms(2026, 8, 12, 11, 0, 0).unwrap(),
            )
            .unwrap();

        handle_runtime_scan_event(
            ScanEvent::CompanyFailed {
                company_id: "acme".into(),
                kind: SourceErrorKind::Transport,
                diagnostic: "connection reset".into(),
            },
            &store,
            &mut app,
        )
        .unwrap();

        assert_eq!(app.scans().len(), 2);
        assert_eq!(app.scans()[0].company_name, "Acme");
        assert_eq!(app.scans()[0].error_kind, Some(SourceErrorKind::Transport));
        assert_eq!(app.sources().len(), 1);
        assert_eq!(app.sources()[0].company_name, "Acme");
        assert_eq!(
            app.sources()[0].diagnostic.as_deref(),
            Some("connection reset")
        );
    }

    #[test]
    fn copy_command_sends_the_official_url_to_the_clipboard_action() {
        let store = store_with_job();
        let jobs = store.lock().unwrap().list_jobs(JobQuery::active()).unwrap();
        let mut app = App::new(config(), jobs);
        let mut opened = |_: &str| -> super::Result<()> { Ok(()) };
        let mut copied_url = None;
        let mut copied = |url: &str| -> super::Result<()> {
            copied_url = Some(url.to_owned());
            Ok(())
        };

        let effect = execute_command(
            AppCommand::CopyUrl("https://example.test/job".into()),
            &store,
            &mut app,
            &mut opened,
            &mut copied,
        )
        .unwrap();

        assert_eq!(effect, CommandEffect::Continue);
        assert_eq!(copied_url.as_deref(), Some("https://example.test/job"));
    }

    #[test]
    fn wsl_browser_uses_windows_explorer_instead_of_a_linux_opener() {
        let command = browser_command(Platform::Wsl).unwrap();

        assert_eq!(command.program, "explorer.exe");
        assert!(command.args.is_empty());
    }

    #[test]
    fn linux_clipboard_falls_back_in_order() {
        let mut tried = Vec::new();

        copy_url_with(
            "https://example.test/job",
            Platform::Linux,
            |command, contents| {
                assert_eq!(contents, "https://example.test/job");
                tried.push((command.program, command.args));
                if command.program == "xclip" {
                    Ok(())
                } else {
                    Err(std::io::Error::from(std::io::ErrorKind::NotFound))
                }
            },
        )
        .unwrap();

        assert_eq!(
            tried,
            vec![
                ("wl-copy", &[][..]),
                ("xclip", &["-selection", "clipboard"][..])
            ]
        );
    }

    #[test]
    fn linux_clipboard_falls_back_after_an_installed_utility_fails() {
        let mut tried = Vec::new();

        copy_url_with("https://example.test/job", Platform::Linux, |command, _| {
            tried.push(command.program);
            if command.program == "xclip" {
                Ok(())
            } else {
                Err(std::io::Error::other("no Wayland display"))
            }
        })
        .unwrap();

        assert_eq!(tried, vec!["wl-copy", "xclip"]);
    }

    #[test]
    fn missing_clipboard_tools_report_every_supported_linux_fallback() {
        let error = copy_url_with("https://example.test/job", Platform::Linux, |_, _| {
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("wl-copy"));
        assert!(error.contains("xclip"));
        assert!(error.contains("xsel"));
    }

    #[test]
    fn failed_clipboard_tools_aggregate_every_diagnostic() {
        let error = copy_url_with("https://example.test/job", Platform::Linux, |command, _| {
            Err(std::io::Error::other(format!(
                "{} unavailable",
                command.program
            )))
        })
        .unwrap_err()
        .to_string();

        assert!(error.contains("wl-copy unavailable"));
        assert!(error.contains("xclip unavailable"));
        assert!(error.contains("xsel unavailable"));
    }
}
