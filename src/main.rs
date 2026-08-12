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
    ratatui::restore();
    result
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    store: Arc<Mutex<Store>>,
    scan_service: Arc<ScanService>,
) -> Result<()> {
    let mut events = EventStream::new();
    let (scan_tx, mut scan_rx) = mpsc::unbounded_channel();
    let mut scan_active = false;

    loop {
        terminal.draw(|frame| render(frame, app))?;
        tokio::select! {
            event = events.next() => {
                let Some(event) = event else { return Ok(()) };
                let event = event?;
                if let Event::Key(key) = event {
                    let width = terminal.size()?.width;
                    match app.handle_key_with_width(key, width) {
                        AppCommand::ToggleApplied(key) => {
                            store.lock().unwrap().toggle_applied(&key, Utc::now())?;
                            app.replace_jobs(load_current_view(&store, app)?);
                        }
                        AppCommand::OpenUrl(url) => opener::open_browser(url)?,
                        AppCommand::StartScan if mark_scan_started(&mut scan_active) => {
                            spawn_scan(Arc::clone(&scan_service), scan_tx.clone());
                        }
                        AppCommand::ReloadJobs => {
                            app.replace_jobs(load_current_view(&store, app)?);
                        }
                        AppCommand::Quit => break,
                        AppCommand::None | AppCommand::StartScan => {}
                    }
                }
            }
            Some(event) = scan_rx.recv() => {
                let finished = matches!(event, ScanEvent::RunFinished { .. });
                app.handle_scan_event(event);
                if finished {
                    scan_active = false;
                    app.replace_jobs(load_current_view(&store, app)?);
                }
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

fn load_current_view(
    store: &Arc<Mutex<Store>>,
    app: &App,
) -> rusqlite::Result<Vec<job_watch::domain::JobRecord>> {
    let query = match app.view() {
        View::Active => JobQuery::active(),
        View::New => JobQuery::new(),
        View::Applied => JobQuery::applied(),
        View::History => JobQuery::history(),
        View::Scans | View::Sources => JobQuery::all(),
    };
    store.lock().unwrap().list_jobs(query)
}

fn spawn_scan(scan_service: Arc<ScanService>, scan_tx: mpsc::UnboundedSender<ScanEvent>) {
    let run_id = format!("scan-{}", Utc::now().timestamp_micros());
    tokio::task::spawn_blocking(move || {
        tokio::runtime::Handle::current().block_on(scan_service.run(run_id, scan_tx));
    });
}

fn mark_scan_started(active: &mut bool) -> bool {
    if *active {
        false
    } else {
        *active = true;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::mark_scan_started;

    #[test]
    fn starts_only_one_scan_until_the_active_run_finishes() {
        let mut active = false;

        assert!(mark_scan_started(&mut active));
        assert!(!mark_scan_started(&mut active));
        active = false;
        assert!(mark_scan_started(&mut active));
    }
}
