use std::{
    error::Error,
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
    sources::{JobSource, ashby},
    storage::{JobQuery, Store},
    ui::{App, AppCommand, View, render},
};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[tokio::main]
async fn main() -> Result<()> {
    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "config.toml".to_owned());
    let config = Config::load(config_path)?;
    let mut store = Store::open(&config.database_path)?;
    store.sync_companies(&config.companies)?;
    let store = Arc::new(Mutex::new(store));
    let scan_service = Arc::new(scan_service(&config, Arc::clone(&store))?);
    let jobs = store.lock().unwrap().list_jobs(JobQuery::active())?;
    let mut app = App::new(config, jobs);
    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app, store, scan_service).await;
    finish_with_restore(result, ratatui::restore)
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
                    let mut open_url = |url: &str| opener::open_browser(url).map_err(Into::into);
                    match execute_command(command, &store, app, &mut open_url)? {
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
                app.handle_scan_event(event);
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

fn scan_service(config: &Config, store: Arc<Mutex<Store>>) -> Result<ScanService> {
    let client = ashby::build_client(
        &config.scan.user_agent,
        Duration::from_secs(config.scan.timeout_seconds),
    )?;
    let sources = config
        .companies
        .iter()
        .map(|company| match &company.source {
            SourceConfig::Ashby { board } => {
                Arc::new(ashby::AshbySource::new(&company.id, board, client.clone()))
                    as Arc<dyn JobSource>
            }
        })
        .collect();
    Ok(ScanService::new(
        sources,
        EligibilityFilter::new(&config.filters)?,
        config.companies.clone(),
        store,
        config.scan.clone(),
    ))
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
    drop(store);
    app.replace_jobs(jobs, active_job_count);
    Ok(())
}

fn execute_command(
    command: AppCommand,
    store: &Arc<Mutex<Store>>,
    app: &mut App,
    open_url: &mut impl FnMut(&str) -> Result<()>,
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
        AppCommand::StartScan => Ok(CommandEffect::StartScan),
        AppCommand::ReloadJobs => {
            reload_jobs(store, app)?;
            Ok(CommandEffect::Continue)
        }
        AppCommand::Quit => Ok(CommandEffect::Quit),
        AppCommand::None => Ok(CommandEffect::Continue),
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
        domain::{ClassifiedJob, Eligibility, JobKey, ObservedJob, SourceScan},
        filter::EligibilityFilter,
        scanner::ScanService,
        sources::{JobSource, SourceError},
        storage::{JobQuery, Store},
        ui::{App, AppCommand},
    };
    use serde_json::json;
    use tokio::sync::mpsc;

    use super::{
        CommandEffect, abort_scan, execute_command, finish_scan, finish_with_restore, start_scan,
    };

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

        let effect = execute_command(
            AppCommand::ToggleApplied(JobKey::new("acme", "job-1")),
            &store,
            &mut app,
            &mut opened,
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
        let restored = std::cell::Cell::new(false);
        let result = execute_command(
            AppCommand::ToggleApplied(JobKey::new("acme", "missing")),
            &store,
            &mut app,
            &mut opened,
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
}
