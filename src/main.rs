use std::{
    collections::{HashMap, HashSet},
    env,
    error::Error,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Utc;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event, EventStream},
    execute,
};
use futures_util::StreamExt;
use job_watch::{
    analytics::{JobFacts, SkillSuggestion},
    config::{Config, FiltersConfig, SourceConfig},
    domain::{JobKey, JobRecord, ScanEvent},
    filter::EligibilityFilter,
    insights::{AnalyticsFilters, AnalyticsResult, LibraryState},
    scanner::ScanService,
    sources::{
        JobSource, albert_heijn, ashby, bol, coolblue, ebay, eneco, getnoticed, greenhouse, ing,
        jibe, lever, personio, rabobank, recruitee, workable, yuki,
    },
    storage::{JobQuery, ScanReadModel, SourceReadModel, Store},
    ui::{App, AppCommand, View, render},
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

const DEFAULT_CONFIG: &str = include_str!("../config.toml");

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
    let config_path = user_config_path()?;
    ensure_default_config(&config_path)?;
    sync_default_companies(&config_path)?;
    let Startup {
        mut app,
        store,
        scan_service,
        ..
    } = initialize(&config_path)?;
    let mut terminal = ratatui::init();
    if let Err(source) = execute!(io::stdout(), EnableMouseCapture) {
        ratatui::restore();
        return Err(source.into());
    }
    let result = run(&mut terminal, &mut app, store, scan_service, config_path).await;
    finish_with_restore(result, || {
        let _ = execute!(io::stdout(), DisableMouseCapture);
        ratatui::restore();
    })
}

struct Startup {
    app: App,
    store: Arc<Mutex<Store>>,
    scan_service: Arc<ScanService>,
}

fn initialize(config_path: &Path) -> Result<Startup> {
    let config_path = std::path::absolute(config_path)?;
    let config = Config::load(&config_path).map_err(|source| {
        std::io::Error::other(format!(
            "could not initialize from configuration {}: {source}",
            config_path.display()
        ))
    })?;
    let database_path = database_path(
        config_path
            .parent()
            .expect("config.toml always has a user-config parent"),
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
    let (
        jobs,
        facts,
        scans,
        sources,
        analytics_filters,
        library,
        analytics_scans,
        skill_suggestions,
    ) = {
        let store = store.lock().unwrap();
        let jobs = store.list_jobs(JobQuery::active())?;
        let facts = store.analytics_facts(&jobs, &config.analytics)?;
        let (analytics_filters, library) = store.analytics_state()?;
        (
            jobs,
            facts,
            store.recent_scans()?,
            store.source_health()?,
            analytics_filters,
            library,
            store.analytics_scans()?,
            store.skill_suggestions()?,
        )
    };
    let mut app = App::new_with_facts(config, jobs, facts);
    app.replace_read_models(scans, sources);
    app.replace_analytics_state(analytics_filters, library, analytics_scans);
    app.replace_skill_suggestions(skill_suggestions);
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

fn user_config_path() -> Result<PathBuf> {
    let home = env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map(PathBuf::from);
    let xdg = env::var_os("XDG_CONFIG_HOME").map(PathBuf::from);
    let appdata = env::var_os("APPDATA").map(PathBuf::from);
    Ok(config_path_for(
        current_platform(),
        home.as_deref(),
        xdg.as_deref(),
        appdata.as_deref(),
    )?)
}

fn config_path_for(
    platform: Platform,
    home: Option<&Path>,
    xdg: Option<&Path>,
    appdata: Option<&Path>,
) -> io::Result<PathBuf> {
    let base = match platform {
        Platform::Windows => appdata
            .map(Path::to_path_buf)
            .or_else(|| home.map(|home| home.join("AppData/Roaming"))),
        Platform::Macos => home.map(|home| home.join("Library/Application Support")),
        Platform::Linux | Platform::Wsl => xdg
            .map(Path::to_path_buf)
            .or_else(|| home.map(|home| home.join(".config"))),
    }
    .ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "user config directory is unavailable",
        )
    })?;
    Ok(base.join("job-watch/config.toml"))
}

fn ensure_default_config(path: &Path) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("config path has no parent directory"))?;
    fs::create_dir_all(parent)?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
    {
        Ok(mut file) => file.write_all(DEFAULT_CONFIG.as_bytes()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Ok(()),
        Err(error) => Err(error),
    }
}

fn sync_default_companies(path: &Path) -> Result<()> {
    let mut config: toml::Value = toml::from_str(&fs::read_to_string(path)?)?;
    let defaults: toml::Value = toml::from_str(DEFAULT_CONFIG)?;
    let default_companies = defaults
        .get("companies")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| io::Error::other("built-in config has no company catalog"))?;
    let default_ids = default_companies
        .iter()
        .filter_map(|company| company.get("id").and_then(toml::Value::as_str))
        .collect::<std::collections::HashSet<_>>();
    let current_companies = config
        .get("companies")
        .and_then(toml::Value::as_array)
        .ok_or_else(|| io::Error::other("user config has no company catalog"))?;
    let mut companies = default_companies.clone();
    companies.extend(
        current_companies
            .iter()
            .filter(|company| {
                company
                    .get("id")
                    .and_then(toml::Value::as_str)
                    .is_none_or(|id| !default_ids.contains(id))
            })
            .cloned(),
    );

    if current_companies == &companies {
        return Ok(());
    }
    config["companies"] = toml::Value::Array(companies);
    let temporary_path = path.with_extension("toml.tmp");
    fs::write(&temporary_path, toml::to_string_pretty(&config)?)?;
    fs::rename(temporary_path, path)?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandEffect {
    Continue,
    StartScan,
    FiltersChanged,
    ReloadJobs,
    Quit,
}

struct ReloadData {
    jobs: Vec<JobRecord>,
    facts: HashMap<JobKey, JobFacts>,
    active_job_count: usize,
    scans: Vec<ScanReadModel>,
    sources: Vec<SourceReadModel>,
    analytics_filters: AnalyticsFilters,
    library: LibraryState,
    analytics_scans: Vec<ScanReadModel>,
    skill_suggestions: Vec<SkillSuggestion>,
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    store: Arc<Mutex<Store>>,
    scan_service: Arc<ScanService>,
    config_path: PathBuf,
) -> Result<()> {
    let mut events = EventStream::new();
    let (scan_tx, mut scan_rx) = mpsc::unbounded_channel();
    let (analytics_tx, mut analytics_rx) =
        mpsc::unbounded_channel::<std::result::Result<AnalyticsResult, String>>();
    let (discovery_tx, mut discovery_rx) =
        mpsc::unbounded_channel::<std::result::Result<Option<Vec<SkillSuggestion>>, String>>();
    let (reload_tx, mut reload_rx) =
        mpsc::unbounded_channel::<std::result::Result<ReloadData, String>>();
    let mut active_scan = None;
    let mut active_discovery = false;
    let mut reload_in_flight = false;
    let mut reload_requested = false;
    let mut attempted_discoveries = HashSet::new();

    loop {
        if reload_requested && !reload_in_flight {
            reload_requested = false;
            reload_in_flight = true;
            let reload_tx = reload_tx.clone();
            let store = Arc::clone(&store);
            let query = job_query(app.view());
            let analytics_config = app.config().analytics.clone();
            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || {
                    load_jobs(&store, query, &analytics_config).map_err(|error| error.to_string())
                })
                .await
                .map_err(|error| format!("reload worker failed: {error}"))
                .and_then(|result| result);
                let _ = reload_tx.send(result);
            });
        }
        if let Some(work) = app.start_analytics_work() {
            let analytics_tx = analytics_tx.clone();
            tokio::spawn(async move {
                let result = tokio::task::spawn_blocking(move || work.compute())
                    .await
                    .map_err(|error| format!("analytics worker failed: {error}"));
                let _ = analytics_tx.send(result);
            });
        }
        if !active_discovery
            && let Some(work) = app.emerging_discovery_work()
            && attempted_discoveries.insert(work.cache_key().to_owned())
        {
            active_discovery = true;
            let discovery_tx = discovery_tx.clone();
            let store = Arc::clone(&store);
            tokio::spawn(async move {
                let result =
                    tokio::task::spawn_blocking(move || -> std::result::Result<_, String> {
                        if store
                            .lock()
                            .unwrap()
                            .has_analytics_discovery(work.cache_key())
                            .map_err(|error| error.to_string())?
                        {
                            return Ok(None);
                        }
                        let Some(result) = work.compute() else {
                            return Ok(None);
                        };
                        let store = store.lock().unwrap();
                        store
                            .save_emerging_discovery(&result)
                            .map_err(|error| error.to_string())?;
                        store
                            .skill_suggestions()
                            .map(Some)
                            .map_err(|error| error.to_string())
                    })
                    .await
                    .map_err(|error| format!("analytics discovery worker failed: {error}"))
                    .and_then(|result| result);
                let _ = discovery_tx.send(result);
            });
        }
        terminal.draw(|frame| render(frame, app))?;
        tokio::select! {
            event = events.next() => {
                let Some(event) = event else { return Ok(()) };
                let event = event?;
                let size = terminal.size()?;
                let command = match event {
                    Event::Key(key) => Some(app.handle_key_with_width(key, size.width)),
                    Event::Mouse(mouse) => Some(app.handle_mouse(mouse, size.width, size.height)),
                    Event::FocusLost => {
                        app.clear_mouse_state();
                        None
                    }
                    _ => None,
                };
                if let Some(command) = command {
                    let mut open_job_url = open_url;
                    let mut copy_job_url = copy_url;
                    let mut save_filters = |filters| save_filters(&config_path, &filters);
                    match execute_command(
                        command,
                        &store,
                        app,
                        &mut open_job_url,
                        &mut copy_job_url,
                        &mut save_filters,
                    )? {
                        CommandEffect::StartScan => {
                            start_scan(
                                &mut active_scan,
                                Arc::clone(&scan_service),
                                scan_tx.clone(),
                            );
                        }
                        CommandEffect::FiltersChanged => {
                            scan_service.update_filter(EligibilityFilter::new(
                                &app.config().filters,
                            )?);
                        }
                        CommandEffect::ReloadJobs => {
                            reload_requested = true;
                            app.set_data_loading(true);
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
                if handle_runtime_scan_event(event, app) {
                    reload_requested = true;
                    app.set_data_loading(true);
                }
            }
            Some(result) = analytics_rx.recv() => {
                match result {
                    Ok(result) => app.finish_analytics_work(result),
                    Err(error) => app.fail_analytics_work(error),
                }
            }
            Some(result) = discovery_rx.recv() => {
                active_discovery = false;
                if let Ok(Some(suggestions)) = result {
                    app.replace_skill_suggestions(suggestions);
                }
            }
            Some(result) = reload_rx.recv() => {
                reload_in_flight = false;
                if !reload_requested {
                    apply_reload(app, result.map_err(io::Error::other)?);
                    app.set_data_loading(false);
                }
            }
            scan_result = async {
                active_scan.as_mut().expect("guarded by select branch").await
            }, if active_scan.is_some() => {
                finish_scan(&mut active_scan, scan_result)?;
                reload_requested = true;
                app.set_data_loading(true);
            }
        }
    }
    Ok(())
}

fn handle_runtime_scan_event(event: ScanEvent, app: &mut App) -> bool {
    let company_finished = matches!(
        &event,
        ScanEvent::CompanyCompleted { .. }
            | ScanEvent::CompanyFailed { .. }
            | ScanEvent::CompanyIncomplete { .. }
    );
    app.handle_scan_event(event);
    company_finished
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
    let getnoticed_client = getnoticed::build_client(
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
                SourceConfig::Greenhouse {
                    board,
                    country_filter,
                } => Ok(Arc::new(
                    greenhouse::GreenhouseSource::new(&company.id, board, client.clone())
                        .with_country_filter(country_filter.as_deref()),
                )),
                SourceConfig::Jibe {
                    base_url,
                    client: brand,
                } => Ok(Arc::new(jibe::JibeSource::new(
                    &company.id,
                    base_url,
                    brand,
                    client.clone(),
                ))),
                SourceConfig::Recruitee { base_url } => Ok(Arc::new(
                    recruitee::RecruiteeSource::new(&company.id, base_url, client.clone()),
                )),
                SourceConfig::Personio { base_url } => Ok(Arc::new(personio::PersonioSource::new(
                    &company.id,
                    base_url,
                    client.clone(),
                ))),
                SourceConfig::Lever {
                    api_url,
                    country_filter,
                } => Ok(Arc::new(
                    lever::LeverSource::new(&company.id, api_url, client.clone())
                        .with_country_filter(country_filter.as_deref()),
                )),
                SourceConfig::Workable {
                    account,
                    country_filter,
                } => Ok(Arc::new(
                    workable::WorkableSource::new(&company.id, account, client.clone())
                        .with_country_filter(country_filter.as_deref()),
                )),
                SourceConfig::Yuki { feed_url } => Ok(Arc::new(yuki::YukiSource::new(
                    &company.id,
                    feed_url,
                    client.clone(),
                ))),
                SourceConfig::Teamtailor { feed_url, employer } => Ok(Arc::new(
                    yuki::YukiSource::new(&company.id, feed_url, client.clone())
                        .with_employer(employer),
                )),
                SourceConfig::Bol { base_url } => Ok(Arc::new(bol::BolSource::new(
                    &company.id,
                    base_url,
                    client.clone(),
                ))),
                SourceConfig::Coolblue { listing_url } => Ok(Arc::new(
                    coolblue::CoolblueSource::new(&company.id, listing_url, client.clone()),
                )),
                SourceConfig::Rabobank { base_url, country } => Ok(Arc::new(
                    rabobank::RabobankSource::new(&company.id, base_url, country, client.clone()),
                )),
                SourceConfig::Eneco { listing_url } => Ok(Arc::new(eneco::EnecoSource::new(
                    &company.id,
                    listing_url,
                    client.clone(),
                ))),
                SourceConfig::AlbertHeijn { base_url } => Ok(Arc::new(
                    albert_heijn::AlbertHeijnSource::new(&company.id, base_url, client.clone()),
                )),
                SourceConfig::Ing { listing_url } => Ok(Arc::new(ing::IngSource::new(
                    &company.id,
                    listing_url,
                    client.clone(),
                ))),
                SourceConfig::Ebay { listing_url } => Ok(Arc::new(ebay::EbaySource::new(
                    &company.id,
                    listing_url,
                    ebay_client.clone(),
                ))),
                SourceConfig::Getnoticed {
                    base_url,
                    country_filter,
                } => Ok(Arc::new(getnoticed::GetnoticedSource::new(
                    &company.id,
                    base_url,
                    country_filter.clone(),
                    getnoticed_client.clone(),
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

fn job_query(view: View) -> JobQuery {
    match view {
        View::Active => JobQuery::active(),
        View::New => JobQuery::active(),
        View::Analytics => JobQuery::analytics(),
        View::Library => JobQuery::all(),
        View::Applied => JobQuery::applied(),
        View::History => JobQuery::history(),
        View::Scans | View::Sources | View::Settings => JobQuery::all(),
    }
}

fn load_jobs(
    store: &Arc<Mutex<Store>>,
    query: JobQuery,
    analytics_config: &job_watch::config::AnalyticsConfig,
) -> rusqlite::Result<ReloadData> {
    let store = store.lock().unwrap();
    let jobs = store.list_jobs(query)?;
    let facts = store.analytics_facts(&jobs, analytics_config)?;
    let (analytics_filters, library) = store.analytics_state()?;
    Ok(ReloadData {
        jobs,
        facts,
        active_job_count: store.list_jobs(JobQuery::active())?.len(),
        scans: store.recent_scans()?,
        sources: store.source_health()?,
        analytics_filters,
        library,
        analytics_scans: store.analytics_scans()?,
        skill_suggestions: store.skill_suggestions()?,
    })
}

fn apply_reload(app: &mut App, data: ReloadData) {
    app.replace_jobs_with_facts(data.jobs, data.active_job_count, data.facts);
    app.replace_read_models(data.scans, data.sources);
    app.replace_analytics_state(data.analytics_filters, data.library, data.analytics_scans);
    app.replace_skill_suggestions(data.skill_suggestions);
}

fn execute_command(
    command: AppCommand,
    store: &Arc<Mutex<Store>>,
    app: &mut App,
    open_url: &mut impl FnMut(&str) -> Result<()>,
    copy_url: &mut impl FnMut(&str) -> Result<()>,
    save_filters: &mut impl FnMut(FiltersConfig) -> Result<()>,
) -> Result<CommandEffect> {
    match command {
        AppCommand::ToggleApplied(key) => {
            store.lock().unwrap().toggle_applied(&key, Utc::now())?;
            Ok(CommandEffect::ReloadJobs)
        }
        AppCommand::OpenUrl(url) => {
            open_url(&url)?;
            Ok(CommandEffect::Continue)
        }
        AppCommand::CopyUrl(url) => {
            copy_url(&url)?;
            Ok(CommandEffect::Continue)
        }
        AppCommand::SaveFilters(filters) => {
            save_filters(filters.clone())?;
            app.apply_filters(filters);
            Ok(CommandEffect::FiltersChanged)
        }
        AppCommand::SaveAnalyticsState(filters, library) => {
            store
                .lock()
                .unwrap()
                .save_analytics_state(&filters, &library)?;
            Ok(CommandEffect::Continue)
        }
        AppCommand::ReviewSkillSuggestion(name, status) => {
            store
                .lock()
                .unwrap()
                .review_skill_suggestion(&name, status)?;
            Ok(CommandEffect::ReloadJobs)
        }
        AppCommand::StartScan => Ok(CommandEffect::StartScan),
        AppCommand::ReloadJobs => Ok(CommandEffect::ReloadJobs),
        AppCommand::Quit => Ok(CommandEffect::Quit),
        AppCommand::None => Ok(CommandEffect::Continue),
    }
}

fn save_filters(config_path: &Path, values: &FiltersConfig) -> Result<()> {
    values.validate()?;
    let contents = fs::read_to_string(config_path)?;
    let mut document: toml::Value = toml::from_str(&contents)?;
    let filters = document
        .get_mut("filters")
        .and_then(toml::Value::as_table_mut)
        .ok_or_else(|| io::Error::other("configuration has no [filters] table"))?;
    filters.insert("countries".into(), toml_strings(&values.countries));
    filters.insert(
        "new_job_max_age_days".into(),
        toml::Value::Integer(i64::from(values.new_job_max_age_days)),
    );
    filters.insert(
        "include_title_patterns".into(),
        toml_strings(&values.include_title_patterns),
    );
    filters.insert(
        "exclude_title_patterns".into(),
        toml_strings(&values.exclude_title_patterns),
    );
    let updated = toml::to_string_pretty(&document)?;
    toml::from_str::<Config>(&updated)?.validate()?;
    let temporary_path = config_path.with_extension("toml.tmp");
    fs::write(&temporary_path, updated)?;
    fs::rename(&temporary_path, config_path)?;
    Ok(())
}

fn toml_strings(values: &[String]) -> toml::Value {
    toml::Value::Array(values.iter().cloned().map(toml::Value::String).collect())
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
        sync::{Arc, Barrier, Mutex},
        thread,
        time::{Duration, Instant},
    };

    use chrono::{TimeZone, Utc};
    use job_watch::{
        config::{
            AnalyticsConfig, CompanyConfig, Config, FiltersConfig, KeybindingsConfig, ScanConfig,
            SourceConfig, UiConfig,
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
        CommandEffect, Platform, abort_scan, apply_reload, browser_command, build_sources,
        config_path_for, copy_url_with, ensure_default_config, execute_command, finish_scan,
        finish_with_restore, handle_runtime_scan_event, initialize, load_jobs, save_filters,
        start_scan, sync_default_companies,
    };

    #[test]
    fn config_uses_each_platforms_user_config_directory() {
        let home = std::path::Path::new("/users/alex");
        let xdg = std::path::Path::new("/custom-config");
        let appdata = std::path::Path::new("C:/Users/Alex/AppData/Roaming");

        assert_eq!(
            config_path_for(Platform::Linux, Some(home), Some(xdg), None).unwrap(),
            xdg.join("job-watch/config.toml")
        );
        assert_eq!(
            config_path_for(Platform::Macos, Some(home), None, None).unwrap(),
            home.join("Library/Application Support/job-watch/config.toml")
        );
        assert_eq!(
            config_path_for(Platform::Windows, Some(home), None, Some(appdata)).unwrap(),
            appdata.join("job-watch/config.toml")
        );
    }

    #[test]
    fn first_start_creates_default_config_without_overwriting_it_later() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("job-watch/config.toml");

        ensure_default_config(&path).unwrap();
        assert!(Config::load(&path).is_ok());
        std::fs::write(&path, "user changes").unwrap();
        ensure_default_config(&path).unwrap();

        assert_eq!(std::fs::read_to_string(path).unwrap(), "user changes");
    }

    #[test]
    fn startup_refreshes_the_company_catalog_without_losing_user_configuration() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        let mut old: toml::Value = toml::from_str(include_str!("../config.toml")).unwrap();
        old["filters"]["new_job_max_age_days"] = toml::Value::Integer(14);
        let companies = old["companies"].as_array_mut().unwrap();
        companies.truncate(15);
        for company in companies.iter_mut() {
            company.as_table_mut().unwrap().remove("industry");
            company.as_table_mut().unwrap().remove("scale");
        }
        companies[0]["enabled"] = toml::Value::Boolean(false);
        let mut custom = companies[0].clone();
        custom["id"] = toml::Value::String("custom-company".into());
        custom["name"] = toml::Value::String("Custom Company".into());
        companies.push(custom);
        std::fs::write(&path, toml::to_string(&old).unwrap()).unwrap();

        sync_default_companies(&path).unwrap();

        let migrated = Config::load(&path).unwrap();
        assert_eq!(migrated.filters.new_job_max_age_days, 14);
        assert_eq!(migrated.companies.len(), 44);
        assert!(
            migrated
                .companies
                .iter()
                .take(20)
                .all(|company| company.industry != "Unknown")
        );
        assert!(
            migrated
                .companies
                .iter()
                .any(|company| company.id == "custom-company")
        );
        assert!(
            migrated
                .companies
                .iter()
                .find(|company| company.id == "mollie")
                .unwrap()
                .enabled
        );
    }

    #[test]
    fn settings_save_filters_to_the_existing_config() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");
        std::fs::write(&path, include_str!("../config.toml")).unwrap();
        let mut filters = Config::load(&path).unwrap().filters;
        filters.new_job_max_age_days = 14;
        filters.include_title_patterns.clear();

        save_filters(&path, &filters).unwrap();

        assert_eq!(Config::load(path).unwrap().filters, filters);
    }

    #[test]
    fn production_config_keeps_disabled_companies_unschedulable() {
        let config = Config::load(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")).unwrap();
        let adyen = config
            .companies
            .iter()
            .filter(|company| company.id == "adyen")
            .collect::<Vec<_>>();
        let funda = config
            .companies
            .iter()
            .filter(|company| company.id == "funda")
            .collect::<Vec<_>>();
        let bol = config
            .companies
            .iter()
            .filter(|company| company.id == "bol")
            .collect::<Vec<_>>();
        let ing = config
            .companies
            .iter()
            .filter(|company| company.id == "ing")
            .collect::<Vec<_>>();
        let abn_amro = config
            .companies
            .iter()
            .filter(|company| company.id == "abn-amro")
            .collect::<Vec<_>>();
        let rabobank = config
            .companies
            .iter()
            .filter(|company| company.id == "rabobank")
            .collect::<Vec<_>>();
        let eneco = config
            .companies
            .iter()
            .filter(|company| company.id == "eneco")
            .collect::<Vec<_>>();
        let ahold = config
            .companies
            .iter()
            .filter(|company| company.id == "ahold")
            .collect::<Vec<_>>();
        let datasnipper = config
            .companies
            .iter()
            .filter(|company| company.id == "datasnipper")
            .collect::<Vec<_>>();
        let databricks = config
            .companies
            .iter()
            .filter(|company| company.id == "databricks")
            .collect::<Vec<_>>();
        let reddit = config
            .companies
            .iter()
            .filter(|company| company.id == "reddit")
            .collect::<Vec<_>>();
        let coolblue = config
            .companies
            .iter()
            .filter(|company| company.id == "coolblue")
            .collect::<Vec<_>>();
        let topicus = config
            .companies
            .iter()
            .filter(|company| company.id == "topicus")
            .collect::<Vec<_>>();
        let centric = config
            .companies
            .iter()
            .filter(|company| company.id == "centric")
            .collect::<Vec<_>>();
        let cmcom = config
            .companies
            .iter()
            .filter(|company| company.id == "cmcom")
            .collect::<Vec<_>>();
        let yuki = config
            .companies
            .iter()
            .filter(|company| company.id == "yuki")
            .collect::<Vec<_>>();

        assert_eq!(adyen.len(), 1);
        assert!(adyen[0].enabled);
        assert!(matches!(
            &adyen[0].source,
            SourceConfig::Greenhouse {
                board,
                country_filter: None,
            } if board == "adyen"
        ));

        assert_eq!(funda.len(), 1);
        assert!(funda[0].enabled);
        assert!(matches!(
            &funda[0].source,
            SourceConfig::Recruitee { base_url } if base_url == "https://jobs.funda.nl"
        ));

        assert_eq!(bol.len(), 1);
        assert!(bol[0].enabled);
        assert!(matches!(
            &bol[0].source,
            SourceConfig::Bol { base_url } if base_url == "https://careers.bol.com"
        ));

        assert_eq!(ing.len(), 1);
        assert_eq!(ing[0].name, "ING");
        assert!(ing[0].enabled);
        assert!(matches!(
            &ing[0].source,
            SourceConfig::Ing { listing_url }
                if listing_url == "https://careers.ing.com/en/location/netherlands-jobs/2618/2750405/2/en/search-jobs"
        ));

        assert_eq!(abn_amro.len(), 1);
        assert_eq!(abn_amro[0].name, "ABN AMRO");
        assert!(abn_amro[0].enabled);
        assert!(matches!(
            &abn_amro[0].source,
            SourceConfig::Getnoticed {
                base_url,
                country_filter: Some(country_filter),
            } if base_url == "https://www.werkenbijabnamro.nl" && country_filter == "Nederland"
        ));

        assert_eq!(rabobank.len(), 1);
        assert!(rabobank[0].enabled);
        assert!(matches!(
            &rabobank[0].source,
            SourceConfig::Rabobank { base_url, country }
                if base_url == "https://rabobank.jobs" && country == "NL"
        ));

        assert_eq!(eneco.len(), 1);
        assert!(eneco[0].enabled);
        assert!(matches!(
            &eneco[0].source,
            SourceConfig::Eneco { listing_url }
                if listing_url == "https://www.werkenbijeneco.nl/vacatures?f=1270"
        ));

        assert_eq!(ahold.len(), 1);
        assert_eq!(ahold[0].name, "Albert Heijn Tech");
        assert!(ahold[0].enabled);
        assert!(matches!(
            &ahold[0].source,
            SourceConfig::AlbertHeijn { base_url } if base_url == "https://werk.ah.nl"
        ));

        assert_eq!(datasnipper.len(), 1);
        assert!(datasnipper[0].enabled);
        assert!(matches!(
            &datasnipper[0].source,
            SourceConfig::Ashby { board } if board == "datasnipper"
        ));

        assert_eq!(databricks.len(), 1);
        assert!(databricks[0].enabled);
        assert!(matches!(
            &databricks[0].source,
            SourceConfig::Greenhouse {
                board,
                country_filter: Some(country_filter),
            } if board == "databricks" && country_filter == "NL"
        ));

        assert_eq!(reddit.len(), 1);
        assert!(reddit[0].enabled);
        assert!(matches!(
            &reddit[0].source,
            SourceConfig::Greenhouse {
                board,
                country_filter: Some(country_filter),
            } if board == "reddit" && country_filter == "NL"
        ));

        assert_eq!(coolblue.len(), 1);
        assert!(coolblue[0].enabled);
        assert_eq!(
            coolblue[0].industry,
            "E-commerce, Retail, Logistics, Energy"
        );
        assert!(matches!(
            &coolblue[0].source,
            SourceConfig::Coolblue { listing_url }
                if listing_url == "https://www.coolblue.nl/en/vacancies/search"
        ));

        assert_eq!(topicus.len(), 1);
        assert!(topicus[0].enabled);
        assert_eq!(
            topicus[0].industry,
            "Financial software, Healthcare software, Education software, Public-sector software"
        );
        assert!(matches!(
            &topicus[0].source,
            SourceConfig::Getnoticed {
                base_url,
                country_filter: None,
            } if base_url == "https://www.werkenbijtopicus.nl"
        ));

        assert_eq!(centric.len(), 1);
        assert!(centric[0].enabled);
        assert_eq!(
            centric[0].industry,
            "IT services, Software, Cloud, Consulting"
        );
        assert!(matches!(
            &centric[0].source,
            SourceConfig::Recruitee { base_url }
                if base_url == "https://centric.recruitee.com"
        ));

        assert_eq!(cmcom.len(), 1);
        assert!(cmcom[0].enabled);
        assert_eq!(
            cmcom[0].industry,
            "Communications platform, Payments, Customer engagement, SaaS"
        );
        assert!(matches!(
            &cmcom[0].source,
            SourceConfig::Recruitee { base_url }
                if base_url == "https://cmcom.recruitee.com"
        ));

        assert_eq!(yuki.len(), 1);
        assert!(yuki[0].enabled);
        assert_eq!(yuki[0].industry, "Accounting software, Fintech, B2B SaaS");
        assert!(matches!(
            &yuki[0].source,
            SourceConfig::Yuki { feed_url }
                if feed_url == "https://jobs.yukisoftware.com/jobs.json"
        ));

        let enabled_ids = config
            .companies
            .iter()
            .filter(|company| company.enabled)
            .map(|company| company.id.as_str())
            .collect::<Vec<_>>();
        let scale_labels = [
            "50–249 · Medium-sized company",
            "500–999 · Large company",
            "1,000–1,999 · Large company",
            "2,000+ · Large company",
            "200+ · Medium or large company",
            "1,000+ · Large company",
        ];
        assert!(
            config
                .companies
                .iter()
                .all(|company| scale_labels.contains(&company.scale.as_str()))
        );
        assert_eq!(
            enabled_ids,
            [
                "mollie",
                "booking-com",
                "ebay",
                "airwallex",
                "adyen",
                "backbase",
                "da-vinci",
                "funda",
                "bol",
                "rabobank",
                "eneco",
                "ahold",
                "ing",
                "abn-amro",
                "datasnipper",
                "databricks",
                "coolblue",
                "topicus",
                "centric",
                "cmcom",
                "yuki",
                "reddit",
                "imc",
                "flow-traders",
                "bunq",
                "dpg-media",
                "miro",
                "checkout-com",
                "fourthline",
                "ockto",
                "drw",
                "jump-trading",
                "tower-research",
                "webb-traders",
                "stx-group",
                "elastic",
                "multisafepay",
                "act-commodities",
                "silverflow",
                "ohpen",
                "finom",
                "keylane",
                "info-support"
            ]
        );

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
            industry: "Test".into(),
            scale: "Test".into(),
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
                new_job_max_age_days: 7,
                include_title_patterns: vec!["software engineer".into()],
                exclude_title_patterns: vec![],
            },
            scan: ScanConfig {
                concurrency: 1,
                timeout_seconds: 30,
                retry_count: 0,
                user_agent: "job-watch-test".into(),
            },
            analytics: AnalyticsConfig::default(),
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
    fn applied_command_persists_and_requests_a_background_reload() {
        let store = store_with_job();
        let jobs = store.lock().unwrap().list_jobs(JobQuery::active()).unwrap();
        let mut app = App::new(config(), jobs);
        let mut opened = |_: &str| -> super::Result<()> { Ok(()) };
        let mut copied = |_: &str| -> super::Result<()> { Ok(()) };
        let mut saved = |_: FiltersConfig| -> super::Result<()> { Ok(()) };

        let effect = execute_command(
            AppCommand::ToggleApplied(JobKey::new("acme", "job-1")),
            &store,
            &mut app,
            &mut opened,
            &mut copied,
            &mut saved,
        )
        .unwrap();

        assert_eq!(effect, CommandEffect::ReloadJobs);
        assert!(app.selected_job().unwrap().applied_at.is_none());
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

    #[test]
    fn reload_command_does_not_wait_for_the_store_on_the_ui_thread() {
        let store = store_with_job();
        let jobs = store.lock().unwrap().list_jobs(JobQuery::active()).unwrap();
        let mut app = App::new(config(), jobs);
        let barrier = Arc::new(Barrier::new(2));
        let held_store = Arc::clone(&store);
        let held_barrier = Arc::clone(&barrier);
        let holder = thread::spawn(move || {
            let _guard = held_store.lock().unwrap();
            held_barrier.wait();
            thread::sleep(Duration::from_millis(200));
        });
        barrier.wait();
        let mut opened = |_: &str| -> super::Result<()> { Ok(()) };
        let mut copied = |_: &str| -> super::Result<()> { Ok(()) };
        let mut saved = |_: FiltersConfig| -> super::Result<()> { Ok(()) };

        let started = Instant::now();
        let effect = execute_command(
            AppCommand::ReloadJobs,
            &store,
            &mut app,
            &mut opened,
            &mut copied,
            &mut saved,
        )
        .unwrap();
        let elapsed = started.elapsed();
        holder.join().unwrap();

        assert!(
            elapsed < Duration::from_millis(50),
            "reload blocked the UI thread for {elapsed:?}"
        );
        assert_eq!(effect, CommandEffect::ReloadJobs);
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
        let mut saved = |_: FiltersConfig| -> super::Result<()> { Ok(()) };
        let restored = std::cell::Cell::new(false);
        let result = execute_command(
            AppCommand::ToggleApplied(JobKey::new("acme", "missing")),
            &store,
            &mut app,
            &mut opened,
            &mut copied,
            &mut saved,
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
    fn startup_reports_the_absolute_config_path() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("config.toml");

        let error = match initialize(&path) {
            Ok(_) => panic!("startup unexpectedly succeeded without config.toml"),
            Err(error) => error,
        };

        assert!(error.to_string().contains(&path.display().to_string()));
    }

    #[test]
    fn startup_invalid_configuration_reports_the_absolute_config_path() {
        let directory = tempfile::tempdir().unwrap();
        let config =
            include_str!("../config.toml").replace("schema_version = 1", "schema_version = 2");
        std::fs::write(directory.path().join("config.toml"), config).unwrap();

        let error = match initialize(&directory.path().join("config.toml")) {
            Ok(_) => panic!("startup unexpectedly accepted an invalid configuration"),
            Err(error) => error,
        };
        let diagnostic = error.to_string();

        assert!(diagnostic.contains("schema_version"));
        assert!(diagnostic.contains(&directory.path().join("config.toml").display().to_string()));
    }

    #[test]
    fn startup_filter_failure_reports_the_absolute_config_path() {
        let directory = tempfile::tempdir().unwrap();
        let config = include_str!("../config.toml")
            .replace("new_job_max_age_days = 7", "new_job_max_age_days = 0");
        std::fs::write(directory.path().join("config.toml"), config).unwrap();

        let error = match initialize(&directory.path().join("config.toml")) {
            Ok(_) => panic!("startup unexpectedly accepted an invalid new-job age"),
            Err(error) => error,
        };
        let diagnostic = error.to_string();

        assert!(diagnostic.contains("filters.new_job_max_age_days"));
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

        let error = match initialize(&directory.path().join("config.toml")) {
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

        let error = match initialize(&directory.path().join("config.toml")) {
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

        let startup = initialize(&directory.path().join("config.toml")).unwrap();
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

        let startup = initialize(&directory.path().join("config.toml")).unwrap();
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

        assert!(handle_runtime_scan_event(
            ScanEvent::CompanyFailed {
                company_id: "acme".into(),
                kind: SourceErrorKind::Transport,
                diagnostic: "connection reset".into(),
            },
            &mut app,
        ));
        let data = load_jobs(&store, JobQuery::all(), &config().analytics).unwrap();
        apply_reload(&mut app, data);

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
        let mut saved = |_: FiltersConfig| -> super::Result<()> { Ok(()) };

        let effect = execute_command(
            AppCommand::CopyUrl("https://example.test/job".into()),
            &store,
            &mut app,
            &mut opened,
            &mut copied,
            &mut saved,
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
