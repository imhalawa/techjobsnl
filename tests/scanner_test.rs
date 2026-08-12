use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use job_watch::{
    config::{CompanyConfig, FiltersConfig, ScanConfig, SourceConfig},
    domain::{JobKey, ObservedJob, ScanEvent, SourceErrorKind, SourceScan},
    filter::EligibilityFilter,
    scanner::ScanService,
    sources::{JobSource, SourceError},
    storage::{JobQuery, Store},
};
use serde_json::json;
use tokio::sync::mpsc;

struct CompleteSource {
    company_id: String,
    observations: Vec<ObservedJob>,
}

#[async_trait::async_trait]
impl JobSource for CompleteSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        Ok(SourceScan::Complete {
            observations: self.observations.clone(),
        })
    }
}

struct FailingSource {
    company_id: String,
}

struct HangingSource {
    company_id: String,
    attempts: Arc<AtomicUsize>,
}

struct RetrySource {
    company_id: String,
    attempts: Arc<AtomicUsize>,
    failures_before_success: usize,
    error_kind: SourceErrorKind,
    http_status: Option<u16>,
    retry_after: Option<Duration>,
    retryable: bool,
    observations: Vec<ObservedJob>,
}

#[async_trait::async_trait]
impl JobSource for RetrySource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if attempt < self.failures_before_success {
            return Err(SourceError {
                kind: self.error_kind,
                message: "scripted source failure".into(),
                http_status: self.http_status,
                retry_after: self.retry_after,
                retryable: self.retryable,
            });
        }

        Ok(SourceScan::Complete {
            observations: self.observations.clone(),
        })
    }
}

struct IncompleteSource {
    company_id: String,
    observations: Vec<ObservedJob>,
}

#[async_trait::async_trait]
impl JobSource for IncompleteSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        Ok(SourceScan::Incomplete {
            observations: self.observations.clone(),
            diagnostic: "upstream response was truncated".into(),
        })
    }
}

struct ConcurrentSource {
    company_id: String,
    in_flight: Arc<AtomicUsize>,
    max_in_flight: Arc<AtomicUsize>,
}

struct StoreAccessSource {
    company_id: String,
    store: Arc<Mutex<Store>>,
}

#[async_trait::async_trait]
impl JobSource for StoreAccessSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        assert!(
            self.store.try_lock().is_ok(),
            "the store mutex must not be held while a source future is awaited"
        );
        tokio::time::sleep(Duration::from_millis(1)).await;
        Ok(SourceScan::Complete {
            observations: vec![observed("job-1", "Amsterdam", &["NL"])],
        })
    }
}

#[async_trait::async_trait]
impl JobSource for ConcurrentSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let in_flight = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_in_flight.fetch_max(in_flight, Ordering::SeqCst);
        tokio::time::sleep(Duration::from_millis(30)).await;
        self.in_flight.fetch_sub(1, Ordering::SeqCst);
        Ok(SourceScan::Complete {
            observations: vec![observed(&self.company_id, "Amsterdam", &["NL"])],
        })
    }
}

#[async_trait::async_trait]
impl JobSource for FailingSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        Err(SourceError {
            kind: SourceErrorKind::Schema,
            message: "invalid board response".into(),
            http_status: None,
            retry_after: None,
            retryable: false,
        })
    }
}

#[async_trait::async_trait]
impl JobSource for HangingSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        self.attempts.fetch_add(1, Ordering::SeqCst);
        std::future::pending().await
    }
}

#[tokio::test]
async fn a_failed_company_does_not_discard_another_companys_jobs() {
    let companies = vec![company("healthy"), company("broken")];
    let store = store_for(&companies);
    let service = service(
        vec![
            Arc::new(CompleteSource {
                company_id: "healthy".into(),
                observations: vec![observed("job-1", "Amsterdam", &["NL"])],
            }),
            Arc::new(FailingSource {
                company_id: "broken".into(),
            }),
        ],
        companies,
        Arc::clone(&store),
    );
    let (tx, mut rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;
    let events = drain_events(&mut rx);

    assert_eq!(summary.completed, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::active())
            .unwrap()
            .len(),
        1
    );
    assert!(events.iter().any(
        |event| matches!(event, ScanEvent::CompanyFailed { company_id, .. } if company_id == "broken")
    ));
}

#[tokio::test]
async fn a_disabled_company_is_not_scheduled_or_counted() {
    let mut disabled = company("disabled");
    disabled.enabled = false;
    let companies = vec![disabled];
    let store = store_for(&companies);
    let attempts = Arc::new(AtomicUsize::new(0));
    let service = service(
        vec![Arc::new(RetrySource {
            company_id: "disabled".into(),
            attempts: Arc::clone(&attempts),
            failures_before_success: 0,
            error_kind: SourceErrorKind::Transport,
            http_status: None,
            retry_after: None,
            retryable: false,
            observations: vec![observed("job-1", "Amsterdam", &["NL"])],
        })],
        companies,
        Arc::clone(&store),
    );
    let (tx, mut rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;
    let events = drain_events(&mut rx);

    assert_eq!(attempts.load(Ordering::SeqCst), 0);
    assert_eq!(summary.completed, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.incomplete, 0);
    assert!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::all())
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        events.first(),
        Some(ScanEvent::RunStarted {
            company_count: 0,
            ..
        })
    ));
    assert!(!events.iter().any(|event| matches!(
        event,
        ScanEvent::CompanyStarted { .. }
            | ScanEvent::CompanyCompleted { .. }
            | ScanEvent::CompanyFailed { .. }
            | ScanEvent::CompanyIncomplete { .. }
    )));
    assert!(matches!(
        events.last(),
        Some(ScanEvent::RunFinished {
            completed: 0,
            failed: 0,
            incomplete: 0,
            ..
        })
    ));
}

#[tokio::test]
async fn a_disabled_scan_leaves_an_existing_applied_job_unchanged() {
    let enabled = company("disabled");
    let store = store_for(std::slice::from_ref(&enabled));
    let initial_service = service(
        vec![Arc::new(CompleteSource {
            company_id: "disabled".into(),
            observations: vec![observed("job-1", "Amsterdam", &["NL"])],
        })],
        vec![enabled.clone()],
        Arc::clone(&store),
    );
    let (tx, _rx) = mpsc::unbounded_channel();
    initial_service.run("initial", tx).await;
    store
        .lock()
        .unwrap()
        .toggle_applied(&JobKey::new("disabled", "job-1"), chrono::Utc::now())
        .unwrap();
    let before = store
        .lock()
        .unwrap()
        .list_jobs(JobQuery::all())
        .unwrap()
        .pop()
        .unwrap();

    let mut disabled = enabled;
    disabled.enabled = false;
    store
        .lock()
        .unwrap()
        .sync_companies(&[disabled.clone()])
        .unwrap();
    let attempts = Arc::new(AtomicUsize::new(0));
    let service = service(
        vec![Arc::new(RetrySource {
            company_id: "disabled".into(),
            attempts: Arc::clone(&attempts),
            failures_before_success: 0,
            error_kind: SourceErrorKind::Transport,
            http_status: None,
            retry_after: None,
            retryable: false,
            observations: Vec::new(),
        })],
        vec![disabled],
        Arc::clone(&store),
    );
    let (tx, _rx) = mpsc::unbounded_channel();

    let summary = service.run("disabled", tx).await;
    let after = store
        .lock()
        .unwrap()
        .list_jobs(JobQuery::all())
        .unwrap()
        .pop()
        .unwrap();

    assert_eq!(summary.completed, 0);
    assert_eq!(attempts.load(Ordering::SeqCst), 0);
    assert_eq!(after.key, before.key);
    assert!(after.source_open);
    assert_eq!(after.applied_at, before.applied_at);
    assert_eq!(after.last_seen_at, before.last_seen_at);
}

#[tokio::test]
async fn a_source_without_a_company_configuration_is_not_scheduled() {
    let companies = vec![company("configured")];
    let store = store_for(&companies);
    let attempts = Arc::new(AtomicUsize::new(0));
    let service = service(
        vec![Arc::new(RetrySource {
            company_id: "unconfigured".into(),
            attempts: Arc::clone(&attempts),
            failures_before_success: 0,
            error_kind: SourceErrorKind::Transport,
            http_status: None,
            retry_after: None,
            retryable: false,
            observations: vec![observed("job-1", "Amsterdam", &["NL"])],
        })],
        companies,
        Arc::clone(&store),
    );
    let (tx, mut rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;
    let events = drain_events(&mut rx);

    assert_eq!(attempts.load(Ordering::SeqCst), 0);
    assert_eq!(summary, Default::default());
    assert!(matches!(
        events.first(),
        Some(ScanEvent::RunStarted {
            company_count: 0,
            ..
        })
    ));
    assert!(events.iter().all(|event| !matches!(
        event,
        ScanEvent::CompanyStarted { .. }
            | ScanEvent::CompanyCompleted { .. }
            | ScanEvent::CompanyFailed { .. }
            | ScanEvent::CompanyIncomplete { .. }
    )));
    assert!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::all())
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn each_hanging_attempt_times_out_and_uses_the_timeout_retry_budget() {
    let companies = vec![company("hanging")];
    let store = store_for(&companies);
    let attempts = Arc::new(AtomicUsize::new(0));
    let service = service_with_scan_config(
        vec![Arc::new(HangingSource {
            company_id: "hanging".into(),
            attempts: Arc::clone(&attempts),
        })],
        companies,
        store,
        ScanConfig {
            concurrency: 1,
            timeout_seconds: 1,
            retry_count: 1,
            user_agent: "job-watch-test/0.1".into(),
        },
    );
    let (tx, mut rx) = mpsc::unbounded_channel();

    let summary = tokio::time::timeout(Duration::from_secs(4), service.run("run-1", tx))
        .await
        .expect("each hanging scan attempt must be bounded");
    let events = drain_events(&mut rx);

    assert_eq!(summary.failed, 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::CompanyFailed {
            company_id,
            kind: SourceErrorKind::Timeout,
            ..
        } if company_id == "hanging"
    )));
}

#[tokio::test]
async fn an_unresolved_location_makes_the_company_incomplete_without_persisting_jobs() {
    let companies = vec![company("ambiguous")];
    let store = store_for(&companies);
    let service = service(
        vec![Arc::new(CompleteSource {
            company_id: "ambiguous".into(),
            observations: vec![observed("job-1", "Hybrid", &[])],
        })],
        companies,
        Arc::clone(&store),
    );
    let (tx, mut rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;
    let events = drain_events(&mut rx);

    assert_eq!(summary.completed, 0);
    assert_eq!(summary.failed, 0);
    assert_eq!(summary.incomplete, 1);
    assert!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::all())
            .unwrap()
            .is_empty()
    );
    assert!(events.iter().any(
        |event| matches!(event, ScanEvent::CompanyIncomplete { company_id, .. } if company_id == "ambiguous")
    ));
}

#[tokio::test]
async fn retryable_timeouts_use_the_configured_retry_budget() {
    let companies = vec![company("eventual")];
    let store = store_for(&companies);
    let attempts = Arc::new(AtomicUsize::new(0));
    let service = service_with_scan_config(
        vec![Arc::new(RetrySource {
            company_id: "eventual".into(),
            attempts: Arc::clone(&attempts),
            failures_before_success: 2,
            error_kind: SourceErrorKind::Timeout,
            http_status: None,
            retry_after: None,
            retryable: true,
            observations: vec![observed("job-1", "Amsterdam", &["NL"])],
        })],
        companies,
        Arc::clone(&store),
        ScanConfig {
            concurrency: 1,
            timeout_seconds: 20,
            retry_count: 2,
            user_agent: "job-watch-test/0.1".into(),
        },
    );
    let (tx, _rx) = mpsc::unbounded_channel();
    let started_at = tokio::time::Instant::now();

    let summary = service.run("run-1", tx).await;

    assert_eq!(summary.completed, 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 3);
    assert!(started_at.elapsed() >= Duration::from_millis(750));
    assert_eq!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::active())
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn retries_rate_limits_and_server_errors_but_not_client_errors() {
    let companies = vec![company("limited"), company("server"), company("client")];
    let store = store_for(&companies);
    let limited_attempts = Arc::new(AtomicUsize::new(0));
    let server_attempts = Arc::new(AtomicUsize::new(0));
    let client_attempts = Arc::new(AtomicUsize::new(0));
    let sources: Vec<Arc<dyn JobSource>> = vec![
        retry_source(
            "limited",
            Arc::clone(&limited_attempts),
            SourceErrorKind::RateLimit,
            Some(429),
        ),
        retry_source(
            "server",
            Arc::clone(&server_attempts),
            SourceErrorKind::Transport,
            Some(503),
        ),
        retry_source(
            "client",
            Arc::clone(&client_attempts),
            SourceErrorKind::Transport,
            Some(404),
        ),
    ];
    let service = service_with_scan_config(
        sources,
        companies,
        Arc::clone(&store),
        ScanConfig {
            concurrency: 3,
            timeout_seconds: 20,
            retry_count: 1,
            user_agent: "job-watch-test/0.1".into(),
        },
    );
    let (tx, _rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;

    assert_eq!(summary.completed, 2);
    assert_eq!(summary.failed, 1);
    assert_eq!(limited_attempts.load(Ordering::SeqCst), 2);
    assert_eq!(server_attempts.load(Ordering::SeqCst), 2);
    assert_eq!(client_attempts.load(Ordering::SeqCst), 1);
    assert_eq!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::active())
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn retry_after_overrides_the_fallback_backoff() {
    let companies = vec![company("limited")];
    let store = store_for(&companies);
    let attempts = Arc::new(AtomicUsize::new(0));
    let service = service_with_scan_config(
        vec![Arc::new(RetrySource {
            company_id: "limited".into(),
            attempts: Arc::clone(&attempts),
            failures_before_success: 1,
            error_kind: SourceErrorKind::RateLimit,
            http_status: Some(429),
            retry_after: Some(Duration::from_millis(600)),
            retryable: true,
            observations: vec![observed("job-1", "Amsterdam", &["NL"])],
        })],
        companies,
        store,
        ScanConfig {
            concurrency: 1,
            timeout_seconds: 20,
            retry_count: 1,
            user_agent: "job-watch-test/0.1".into(),
        },
    );
    let (tx, _rx) = mpsc::unbounded_channel();
    let started_at = tokio::time::Instant::now();

    let summary = service.run("run-1", tx).await;

    assert_eq!(summary.completed, 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert!(started_at.elapsed() >= Duration::from_millis(600));
}

#[tokio::test]
async fn a_statusless_non_timeout_transport_error_is_not_retried() {
    let companies = vec![company("connection")];
    let store = store_for(&companies);
    let attempts = Arc::new(AtomicUsize::new(0));
    let service = service_with_scan_config(
        vec![Arc::new(RetrySource {
            company_id: "connection".into(),
            attempts: Arc::clone(&attempts),
            failures_before_success: 1,
            error_kind: SourceErrorKind::Transport,
            http_status: None,
            retry_after: Some(Duration::ZERO),
            retryable: true,
            observations: vec![observed("job-1", "Amsterdam", &["NL"])],
        })],
        companies,
        store,
        ScanConfig {
            concurrency: 1,
            timeout_seconds: 20,
            retry_count: 1,
            user_agent: "job-watch-test/0.1".into(),
        },
    );
    let (tx, _rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;

    assert_eq!(summary.failed, 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_source_declared_incomplete_scan_does_not_persist_observations() {
    let companies = vec![company("partial")];
    let store = store_for(&companies);
    let service = service(
        vec![Arc::new(IncompleteSource {
            company_id: "partial".into(),
            observations: vec![observed("job-1", "Amsterdam", &["NL"])],
        })],
        companies,
        Arc::clone(&store),
    );
    let (tx, mut rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;
    let events = drain_events(&mut rx);

    assert_eq!(summary.incomplete, 1);
    assert!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::all())
            .unwrap()
            .is_empty()
    );
    assert!(events.iter().any(|event| matches!(
        event,
        ScanEvent::CompanyIncomplete { diagnostic, .. }
            if diagnostic.contains("upstream response was truncated")
    )));
}

#[tokio::test]
async fn source_fetches_respect_the_concurrency_limit_and_event_lifecycle() {
    let companies = ["one", "two", "three", "four"]
        .into_iter()
        .map(company)
        .collect::<Vec<_>>();
    let store = store_for(&companies);
    let in_flight = Arc::new(AtomicUsize::new(0));
    let max_in_flight = Arc::new(AtomicUsize::new(0));
    let sources = companies
        .iter()
        .map(|company| {
            Arc::new(ConcurrentSource {
                company_id: company.id.clone(),
                in_flight: Arc::clone(&in_flight),
                max_in_flight: Arc::clone(&max_in_flight),
            }) as Arc<dyn JobSource>
        })
        .collect();
    let service = service_with_scan_config(
        sources,
        companies,
        store,
        ScanConfig {
            concurrency: 2,
            timeout_seconds: 20,
            retry_count: 0,
            user_agent: "job-watch-test/0.1".into(),
        },
    );
    let (tx, mut rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;
    let events = drain_events(&mut rx);

    assert_eq!(summary.completed, 4);
    assert_eq!(max_in_flight.load(Ordering::SeqCst), 2);
    assert!(matches!(
        events.first(),
        Some(ScanEvent::RunStarted { run_id, .. }) if run_id == "run-1"
    ));
    assert!(matches!(
        events.last(),
        Some(ScanEvent::RunFinished { run_id, completed: 4, .. }) if run_id == "run-1"
    ));
    for company_id in ["one", "two", "three", "four"] {
        let started = event_position(
            &events,
            |event| matches!(event, ScanEvent::CompanyStarted { company_id: id } if id == company_id),
        );
        let completed = event_position(
            &events,
            |event| matches!(event, ScanEvent::CompanyCompleted { company_id: id, .. } if id == company_id),
        );
        assert!(started < completed);
    }
}

#[tokio::test]
async fn source_fetches_do_not_run_under_the_store_mutex() {
    let companies = vec![company("probe")];
    let store = store_for(&companies);
    let service = service(
        vec![Arc::new(StoreAccessSource {
            company_id: "probe".into(),
            store: Arc::clone(&store),
        })],
        companies,
        store,
    );
    let (tx, _rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;

    assert_eq!(summary.completed, 1);
}

#[tokio::test]
async fn a_storage_failure_is_isolated_and_emits_one_failed_terminal_event() {
    let database = tempfile::NamedTempFile::new().unwrap();
    let companies = vec![company("healthy"), company("broken-store")];
    let mut raw_store = Store::open(database.path()).unwrap();
    raw_store.sync_companies(&companies).unwrap();
    rusqlite::Connection::open(database.path())
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_broken_scan BEFORE INSERT ON scans
             WHEN NEW.company_id = 'broken-store'
             BEGIN SELECT RAISE(ABORT, 'forced scan failure'); END;",
        )
        .unwrap();
    let store = Arc::new(Mutex::new(raw_store));
    let service = service(
        vec![
            Arc::new(CompleteSource {
                company_id: "healthy".into(),
                observations: vec![observed("healthy-job", "Amsterdam", &["NL"])],
            }),
            Arc::new(CompleteSource {
                company_id: "broken-store".into(),
                observations: vec![observed("broken-job", "Amsterdam", &["NL"])],
            }),
        ],
        companies,
        Arc::clone(&store),
    );
    let (tx, mut rx) = mpsc::unbounded_channel();

    let summary = service.run("run-1", tx).await;
    let events = drain_events(&mut rx);

    assert_eq!(summary.completed, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::active())
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        events
            .iter()
            .filter(|event| matches!(
                event,
                ScanEvent::CompanyFailed {
                    company_id,
                    kind: SourceErrorKind::Storage,
                    ..
                } if company_id == "broken-store"
            ))
            .count(),
        1
    );
    assert!(!events.iter().any(|event| matches!(
        event,
        ScanEvent::CompanyCompleted { company_id, .. } if company_id == "broken-store"
    )));
}

fn retry_source(
    company_id: &str,
    attempts: Arc<AtomicUsize>,
    error_kind: SourceErrorKind,
    http_status: Option<u16>,
) -> Arc<dyn JobSource> {
    Arc::new(RetrySource {
        company_id: company_id.into(),
        attempts,
        failures_before_success: 1,
        error_kind,
        http_status,
        retry_after: Some(Duration::ZERO),
        retryable: true,
        observations: vec![observed("job-1", "Amsterdam", &["NL"])],
    })
}

fn service(
    sources: Vec<Arc<dyn JobSource>>,
    companies: Vec<CompanyConfig>,
    store: Arc<Mutex<Store>>,
) -> ScanService {
    service_with_scan_config(
        sources,
        companies,
        store,
        ScanConfig {
            concurrency: 2,
            timeout_seconds: 20,
            retry_count: 0,
            user_agent: "job-watch-test/0.1".into(),
        },
    )
}

fn service_with_scan_config(
    sources: Vec<Arc<dyn JobSource>>,
    companies: Vec<CompanyConfig>,
    store: Arc<Mutex<Store>>,
    scan_config: ScanConfig,
) -> ScanService {
    ScanService::new(
        sources,
        EligibilityFilter::new(&filters()).unwrap(),
        companies,
        store,
        scan_config,
    )
}

fn store_for(companies: &[CompanyConfig]) -> Arc<Mutex<Store>> {
    let mut store = Store::open_in_memory().unwrap();
    store.sync_companies(companies).unwrap();
    Arc::new(Mutex::new(store))
}

fn company(id: &str) -> CompanyConfig {
    CompanyConfig {
        id: id.into(),
        name: id.into(),
        enabled: true,
        location_country_overrides: HashMap::new(),
        source: SourceConfig::Ashby { board: id.into() },
    }
}

fn filters() -> FiltersConfig {
    FiltersConfig {
        countries: vec!["NL".into()],
        include_families: vec!["software".into()],
        include_title_patterns: Vec::new(),
        exclude_title_patterns: Vec::new(),
    }
}

fn observed(source_id: &str, location: &str, countries: &[&str]) -> ObservedJob {
    ObservedJob {
        source_id: source_id.into(),
        title: "Software Engineer".into(),
        department: Some("Engineering".into()),
        team: None,
        employment_type: Some("Full-time".into()),
        locations: vec![location.into()],
        countries: countries.iter().map(|country| (*country).into()).collect(),
        job_url: format!("https://careers.example.test/jobs/{source_id}"),
        apply_url: format!("https://careers.example.test/jobs/{source_id}/apply"),
        description: "Build reliable systems.".into(),
        raw_payload: json!({"id": source_id}),
        published_at: None,
    }
}

fn drain_events(rx: &mut mpsc::UnboundedReceiver<ScanEvent>) -> Vec<ScanEvent> {
    let mut events = Vec::new();
    while let Ok(event) = rx.try_recv() {
        events.push(event);
    }
    events
}

fn event_position(events: &[ScanEvent], predicate: impl Fn(&ScanEvent) -> bool) -> usize {
    events.iter().position(predicate).unwrap()
}
