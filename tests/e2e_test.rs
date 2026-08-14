use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use job_watch::{
    config::{CompanyConfig, Config, SourceConfig},
    domain::{JobKey, ObservedJob, SourceErrorKind, SourceScan},
    filter::EligibilityFilter,
    scanner::ScanService,
    sources::{JobSource, SourceError},
    storage::{JobQuery, Store},
    ui::{App, render},
};
use ratatui::{Terminal, backend::TestBackend};
use serde_json::json;
use tokio::sync::mpsc;

#[derive(Clone)]
enum FakeScan {
    Complete(Vec<ObservedJob>),
    Incomplete(Vec<ObservedJob>),
    Failed,
}

struct FakeSource {
    company_id: String,
    scans: Mutex<VecDeque<FakeScan>>,
}

#[async_trait::async_trait]
impl JobSource for FakeSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        match self.scans.lock().unwrap().pop_front().unwrap() {
            FakeScan::Complete(observations) => Ok(SourceScan::Complete { observations }),
            FakeScan::Incomplete(observations) => Ok(SourceScan::Incomplete {
                observations,
                diagnostic: "scripted incomplete response".into(),
            }),
            FakeScan::Failed => Err(SourceError {
                kind: SourceErrorKind::Transport,
                message: "scripted transport failure".into(),
                http_status: None,
                retry_after: None,
                retryable: false,
            }),
        }
    }
}

fn observed(source_id: &str, title: &str) -> ObservedJob {
    ObservedJob {
        source_id: source_id.into(),
        title: title.into(),
        department: Some("Engineering".into()),
        team: Some("Platform".into()),
        employment_type: Some("Full-time".into()),
        locations: vec!["Amsterdam".into()],
        countries: vec!["NL".into()],
        job_url: format!("https://example.test/jobs/{source_id}"),
        apply_url: format!("https://example.test/jobs/{source_id}/apply"),
        description: "Build reliable payment systems.".into(),
        raw_payload: json!({"id": source_id}),
        published_at: None,
    }
}

fn source(company_id: &str, scans: Vec<FakeScan>) -> Arc<dyn JobSource> {
    Arc::new(FakeSource {
        company_id: company_id.into(),
        scans: Mutex::new(scans.into()),
    })
}

fn rendered(app: &App) -> String {
    let backend = TestBackend::new(140, 40);
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

async fn scan(service: &ScanService, run_id: &str) {
    let (tx, _rx) = mpsc::unbounded_channel();
    service.run(run_id, tx).await;
}

#[tokio::test]
async fn configured_offline_scan_lifecycle_reaches_the_default_ui() {
    let mut config = Config::load(format!("{}/config.toml", env!("CARGO_MANIFEST_DIR"))).unwrap();
    assert_eq!(config.database_path, ".data/job-watch.sqlite3");
    assert_eq!(config.scan.concurrency, 4);
    assert_eq!(
        config
            .companies
            .iter()
            .map(|company| company.name.as_str())
            .collect::<Vec<_>>(),
        [
            "Mollie",
            "Booking.com",
            "eBay",
            "Airwallex",
            "Adyen",
            "Backbase",
            "Da Vinci",
            "Funda",
            "bol.com",
            "Rabobank",
            "Eneco",
            "Albert Heijn Tech",
            "ING",
            "ABN AMRO",
            "DataSnipper",
            "Databricks",
            "Coolblue",
            "Topicus",
            "Centric",
            "CM.com",
            "Yuki",
            "Reddit",
            "IMC Trading",
            "Flow Traders",
            "bunq",
            "DPG Media",
            "Miro",
            "Checkout.com",
            "Fourthline",
            "Ockto",
            "DRW",
            "Jump Trading",
            "Tower Research",
            "WEBB Traders",
            "STX Group",
            "Elastic",
            "MultiSafepay",
            "ACT Commodities",
            "Silverflow",
            "Ohpen",
            "Finom",
            "Keylane",
            "Info Support",
            "Wolters Kluwer",
            "Vanderlande",
            "Bitvavo",
            "Exact",
            "AFAS Software",
            "NS",
            "Achmea",
            "ChipSoft",
            "ANWB",
            "PostNL",
            "TomTom",
            "Amazon / AWS",
            "Uber",
            "Microsoft",
            "Klarna",
            "flatexDEGIRO",
            "Google",
            "Worldline",
            "Buckaroo",
        ]
    );

    let mollie = config.companies[0].clone();
    config.companies.truncate(1);
    let atlas = CompanyConfig {
        id: "atlas".into(),
        name: "Atlas".into(),
        industry: "Test".into(),
        scale: "Test".into(),
        enabled: true,
        location_country_overrides: Default::default(),
        source: SourceConfig::Ashby {
            board: "offline-atlas".into(),
        },
    };
    config.companies.push(atlas.clone());

    let stable = observed("stable", "Mollie Platform Engineer");
    let excluded = observed("excluded", "Product Manager");
    let new_job = observed("new", "Application Security Engineer");
    let atlas_job = observed("atlas-job", "Software Engineer");
    let sources = vec![
        source(
            "mollie",
            vec![
                FakeScan::Complete(vec![stable.clone(), excluded.clone()]),
                FakeScan::Complete(vec![stable.clone(), excluded.clone()]),
                FakeScan::Complete(vec![]),
                FakeScan::Failed,
                FakeScan::Complete(vec![stable.clone(), excluded.clone()]),
                FakeScan::Complete(vec![stable.clone(), excluded.clone(), new_job]),
            ],
        ),
        source(
            "atlas",
            vec![
                FakeScan::Complete(vec![atlas_job.clone()]),
                FakeScan::Complete(vec![atlas_job.clone()]),
                FakeScan::Complete(vec![atlas_job.clone()]),
                FakeScan::Incomplete(vec![observed("ignored", "Software Engineer")]),
                FakeScan::Complete(vec![atlas_job.clone()]),
                FakeScan::Complete(vec![atlas_job]),
            ],
        ),
    ];
    let store = Arc::new(Mutex::new(Store::open_in_memory().unwrap()));
    store
        .lock()
        .unwrap()
        .sync_companies(&config.companies)
        .unwrap();
    let service = ScanService::new(
        sources,
        EligibilityFilter::new(&config.filters).unwrap(),
        config.companies.clone(),
        Arc::clone(&store),
        config.scan.clone(),
    );

    scan(&service, "run-1").await;
    assert_eq!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::all())
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::active())
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::new())
            .unwrap()
            .len(),
        2
    );

    scan(&service, "run-2").await;
    assert!(
        store
            .lock()
            .unwrap()
            .list_jobs(JobQuery::new())
            .unwrap()
            .is_empty()
    );

    scan(&service, "run-3").await;
    store
        .lock()
        .unwrap()
        .toggle_applied(&JobKey::new("mollie", "stable"), chrono::Utc::now())
        .unwrap();
    let before_errors = store.lock().unwrap().list_jobs(JobQuery::all()).unwrap();
    assert!(
        before_errors
            .iter()
            .find(|job| job.key == JobKey::new("mollie", "stable"))
            .is_some_and(|job| !job.source_open)
    );

    scan(&service, "run-4").await;
    let after_errors = store.lock().unwrap().list_jobs(JobQuery::all()).unwrap();
    assert_eq!(after_errors.len(), before_errors.len());
    assert_eq!(
        serde_json::to_value(&after_errors).unwrap(),
        serde_json::to_value(&before_errors).unwrap(),
        "failed and incomplete company results must not mutate any persisted job field"
    );
    assert!(
        after_errors
            .iter()
            .all(|job| job.key.source_id != "ignored")
    );

    scan(&service, "run-5").await;
    let reopened = store
        .lock()
        .unwrap()
        .list_jobs(JobQuery::all())
        .unwrap()
        .into_iter()
        .find(|job| job.key == JobKey::new("mollie", "stable"))
        .unwrap();
    assert!(reopened.source_open);
    assert!(!reopened.is_new);
    assert!(reopened.reopened_at.is_some());

    scan(&service, "run-6").await;
    let all = store.lock().unwrap().list_jobs(JobQuery::all()).unwrap();
    assert_eq!(all.len(), 4);
    assert!(
        all.iter()
            .find(|job| job.key == JobKey::new("mollie", "stable"))
            .is_some_and(|job| job.applied_at.is_some() && !job.is_new)
    );
    assert!(
        all.iter()
            .find(|job| job.key == JobKey::new("mollie", "new"))
            .is_some_and(|job| job.is_new)
    );

    let mut disabled_atlas = atlas;
    disabled_atlas.enabled = false;
    store
        .lock()
        .unwrap()
        .sync_companies(&[mollie, disabled_atlas.clone()])
        .unwrap();
    config.companies[1] = disabled_atlas;
    let jobs = store.lock().unwrap().list_jobs(JobQuery::active()).unwrap();
    let stored_after_disable = store.lock().unwrap().list_jobs(JobQuery::all()).unwrap();
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().all(|job| job.key.company_id == "mollie"));
    assert!(
        stored_after_disable
            .iter()
            .any(|job| !job.classified.eligibility.eligible)
    );
    assert!(
        stored_after_disable
            .iter()
            .any(|job| job.key.company_id == "atlas")
    );

    let screen = rendered(&App::new(config, jobs));
    assert!(screen.contains("Mollie"));
    assert!(screen.contains("Active jobs"));
    assert!(screen.contains("Job details"));
}
