use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use job_watch::{
    config::Config,
    domain::{ObservedJob, SourceErrorKind, SourceScan},
    filter::EligibilityFilter,
    sources::{
        JobSource,
        ashby::build_client,
        greenhouse::{
            GreenhouseSource, parse_greenhouse_response, parse_greenhouse_response_for_country,
        },
        jibe::{JibeSource, parse_jibe_pages},
        recruitee::{RecruiteeSource, parse_recruitee_response},
    },
};

#[test]
fn parses_and_filters_sanitized_databricks_board() {
    let jobs = parse_greenhouse_response_for_country(
        "databricks",
        include_str!("fixtures/greenhouse/databricks.json"),
        "NL",
    )
    .unwrap();
    let config = Config::load(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")).unwrap();
    let filter = EligibilityFilter::new(&config.filters).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "Software Engineer - Backend");
    assert_eq!(jobs[0].locations, ["Amsterdam, Netherlands"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert!(filter.classify(&jobs[0], &HashMap::new()).unwrap().eligible);
}

#[test]
fn parses_and_filters_sanitized_reddit_board() {
    let jobs = parse_greenhouse_response_for_country(
        "reddit",
        include_str!("fixtures/greenhouse/reddit.json"),
        "NL",
    )
    .unwrap();
    let config = Config::load(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")).unwrap();
    let filter = EligibilityFilter::new(&config.filters).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "Staff Site Reliability Engineer, Ads");
    assert_eq!(jobs[0].locations, ["Amsterdam, North Holland, Netherlands"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert!(filter.classify(&jobs[0], &HashMap::new()).unwrap().eligible);
}

#[test]
fn parses_complete_greenhouse_board() {
    let jobs =
        parse_greenhouse_response("adyen", include_str!("fixtures/greenhouse/adyen.json")).unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "101");
    assert_eq!(jobs[0].locations, ["Amsterdam", "London"]);
    assert_eq!(jobs[0].countries, ["NL", "GB"]);
    assert_eq!(jobs[0].department.as_deref(), Some("Development"));
    assert_eq!(jobs[0].description, "Build & operate payment systems.");
    assert_eq!(jobs[0].job_url, jobs[0].apply_url);
    assert_eq!(
        jobs[0].published_at.unwrap().to_rfc3339(),
        "2026-08-10T07:15:00+00:00"
    );
    let fixture = greenhouse_fixture();
    assert_eq!(jobs[0].raw_payload, fixture["jobs"][0]);
    assert_eq!(jobs[1].raw_payload, fixture["jobs"][1]);
}

#[test]
fn greenhouse_rejects_missing_required_data_and_duplicate_ids() {
    for mutation in ["jobs", "id", "url", "description", "locations", "duplicate"] {
        let mut board = greenhouse_fixture();
        match mutation {
            "jobs" => {
                board.as_object_mut().unwrap().remove("jobs");
            }
            "id" => {
                board["jobs"][0].as_object_mut().unwrap().remove("id");
            }
            "url" => board["jobs"][0]["absolute_url"] = " ".into(),
            "description" => board["jobs"][0]["content"] = "<p> </p>".into(),
            "locations" => board["jobs"][0]["offices"] = serde_json::json!([]),
            "duplicate" => board["jobs"][1]["id"] = board["jobs"][0]["id"].clone(),
            _ => unreachable!(),
        }

        assert_schema_error(parse_greenhouse_response("adyen", &board.to_string()));
    }
}

#[test]
fn parses_complete_jibe_pages() {
    let jobs = parse_jibe_pages(
        "booking-com",
        "https://jobs.booking.com",
        "Booking.com",
        &[
            include_str!("fixtures/jibe/booking-page-1.json"),
            include_str!("fixtures/jibe/booking-page-2.json"),
        ],
    )
    .unwrap();

    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].source_id, "BK-201");
    assert_eq!(
        jobs[0].locations,
        ["Amsterdam, Netherlands", "Manchester, United Kingdom"]
    );
    assert_eq!(jobs[0].countries, ["NL", "GB"]);
    assert_eq!(jobs[0].department.as_deref(), Some("Engineering"));
    assert_eq!(jobs[0].team.as_deref(), Some("Trips"));
    assert_eq!(jobs[0].employment_type.as_deref(), Some("FULL_TIME"));
    assert_eq!(jobs[0].description, "Build booking systems.");
    assert_eq!(
        jobs[0].job_url,
        "https://jobs.booking.com/booking/jobs/BK-201"
    );
    assert_eq!(
        jobs[0].apply_url,
        "https://external-workingatbooking.icims.com/jobs/201/login"
    );
    assert_eq!(
        jobs[0].published_at.unwrap().to_rfc3339(),
        "2026-08-10T09:30:00+00:00"
    );
    let fixtures = jibe_fixtures();
    assert_eq!(jobs[0].raw_payload, fixtures[0]["jobs"][0]);
    assert_eq!(jobs[1].raw_payload, fixtures[0]["jobs"][1]);
    assert_eq!(jobs[2].raw_payload, fixtures[1]["jobs"][0]);
}

#[test]
fn jibe_rejects_missing_required_data_and_duplicate_ids() {
    for mutation in ["jobs", "id", "url", "description", "locations", "duplicate"] {
        let mut pages = jibe_fixtures();
        match mutation {
            "jobs" => {
                pages[0].as_object_mut().unwrap().remove("jobs");
            }
            "id" => {
                pages[0]["jobs"][0]["data"]
                    .as_object_mut()
                    .unwrap()
                    .remove("req_id");
            }
            "url" => pages[0]["jobs"][0]["data"]["apply_url"] = " ".into(),
            "description" => pages[0]["jobs"][0]["data"]["description"] = "<p> </p>".into(),
            "locations" => pages[0]["jobs"][0]["data"]["full_location"] = "".into(),
            "duplicate" => {
                pages[1]["jobs"][0]["data"]["req_id"] =
                    pages[0]["jobs"][0]["data"]["req_id"].clone();
            }
            _ => unreachable!(),
        }
        let raw = pages
            .iter()
            .map(serde_json::Value::to_string)
            .collect::<Vec<_>>();
        let refs = raw.iter().map(String::as_str).collect::<Vec<_>>();

        assert_schema_error(parse_jibe_pages(
            "booking-com",
            "https://jobs.booking.com",
            "Booking.com",
            &refs,
        ));
    }
}

#[test]
fn jibe_rejects_incomplete_empty_and_drifting_pages() {
    let fixtures = jibe_fixtures();
    assert_schema_error(parse_jibe_pages(
        "booking-com",
        "https://jobs.booking.com",
        "Booking.com",
        &[&fixtures[0].to_string()],
    ));

    let mut empty = fixtures.clone();
    empty[1]["jobs"] = serde_json::json!([]);
    assert_jibe_fixture_error(empty);

    for field in ["count", "totalCount"] {
        let mut drifting = fixtures.clone();
        drifting[1][field] = 4.into();
        assert_jibe_fixture_error(drifting);
    }
}

#[test]
fn parses_complete_recruitee_offers() {
    let jobs =
        parse_recruitee_response("funda", include_str!("fixtures/recruitee/funda.json")).unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "301");
    assert_eq!(jobs[0].locations, ["Amsterdam", "Utrecht"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].department.as_deref(), Some("Engineering"));
    assert_eq!(jobs[0].team.as_deref(), Some("software-development"));
    assert_eq!(
        jobs[0].employment_type.as_deref(),
        Some("fulltime_permanent")
    );
    assert_eq!(
        jobs[0].description,
        "Build the housing platform.\n\nWork closely with product."
    );
    assert_eq!(jobs[0].job_url, "https://jobs.funda.nl/o/backend-engineer");
    assert_eq!(
        jobs[0].apply_url,
        "https://jobs.funda.nl/o/backend-engineer/c/new"
    );
    assert_eq!(
        jobs[0].published_at.unwrap().to_rfc3339(),
        "2026-08-09T08:30:00+00:00"
    );
    let fixture = recruitee_fixture();
    assert_eq!(jobs[0].raw_payload, fixture["offers"][0]);
    assert_eq!(jobs[1].raw_payload, fixture["offers"][1]);
}

#[test]
fn recruitee_accepts_null_optional_requirements() {
    let mut offers = recruitee_fixture();
    offers["offers"][0]["requirements"] = serde_json::Value::Null;
    offers["offers"][0]["department"] = serde_json::Value::Null;

    let jobs = parse_recruitee_response("funda", &offers.to_string()).unwrap();

    assert_eq!(jobs[0].department, None);

    assert_eq!(jobs[0].description, "Build the housing platform.");
}

#[test]
fn recruitee_rejects_missing_required_data_and_duplicate_ids() {
    for mutation in [
        "offers",
        "id",
        "url",
        "apply_url",
        "description",
        "locations",
        "duplicate",
    ] {
        let mut offers = recruitee_fixture();
        match mutation {
            "offers" => {
                offers.as_object_mut().unwrap().remove("offers");
            }
            "id" => {
                offers["offers"][0].as_object_mut().unwrap().remove("id");
            }
            "url" => offers["offers"][0]["careers_url"] = "".into(),
            "apply_url" => offers["offers"][0]["careers_apply_url"] = "".into(),
            "description" => {
                offers["offers"][0]["description"] = "<p> </p>".into();
                offers["offers"][0]["requirements"] = serde_json::Value::Null;
            }
            "locations" => offers["offers"][0]["locations"] = serde_json::json!([]),
            "duplicate" => offers["offers"][1]["id"] = offers["offers"][0]["id"].clone(),
            _ => unreachable!(),
        }

        assert_schema_error(parse_recruitee_response("funda", &offers.to_string()));
    }
}

#[tokio::test]
#[ignore = "live external source"]
async fn greenhouse_live_returns_complete_unique_jobs() {
    let client = live_client();
    let source = GreenhouseSource::new("adyen", "adyen", client);
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("Adyen", &jobs);
}

#[tokio::test]
#[ignore = "live external source"]
async fn databricks_live_returns_complete_unique_netherlands_jobs() {
    let source = GreenhouseSource::new("databricks", "databricks", live_client())
        .with_country_filter(Some("NL"));
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("Databricks", &jobs);

    let config = Config::load(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")).unwrap();
    let filter = EligibilityFilter::new(&config.filters).unwrap();
    assert!(jobs.iter().any(|job| {
        filter
            .classify(job, &HashMap::new())
            .is_ok_and(|result| result.eligible)
    }));
}

#[tokio::test]
#[ignore = "live external source"]
async fn reddit_live_returns_complete_unique_netherlands_jobs() {
    let source =
        GreenhouseSource::new("reddit", "reddit", live_client()).with_country_filter(Some("NL"));
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("Reddit", &jobs);

    let config = Config::load(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")).unwrap();
    let filter = EligibilityFilter::new(&config.filters).unwrap();
    assert!(jobs.iter().any(|job| {
        filter
            .classify(job, &HashMap::new())
            .is_ok_and(|result| result.eligible)
    }));
}

#[tokio::test]
#[ignore = "live external source"]
async fn backbase_live_returns_complete_unique_netherlands_jobs() {
    let source = GreenhouseSource::new("backbase", "workatbackbase", live_client())
        .with_country_filter(Some("NL"));
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("Backbase", &jobs);
}

#[tokio::test]
#[ignore = "live external source"]
async fn da_vinci_live_returns_complete_unique_netherlands_jobs() {
    let source = GreenhouseSource::new("da-vinci", "davinciderivatives", live_client())
        .with_country_filter(Some("NL"));
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("Da Vinci", &jobs);
}

#[tokio::test]
#[ignore = "live external source"]
async fn maven_live_returns_complete_unique_netherlands_jobs() {
    let source = GreenhouseSource::new(
        "maven-securities",
        "mavensecuritiesholdingltd",
        live_client(),
    )
    .with_country_filter(Some("NL"));
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("Maven Securities", &jobs);
    println!("Maven Securities: {} NL jobs", jobs.len());
}

#[tokio::test]
#[ignore = "live external source"]
async fn hosted_candidate_batch_live_returns_complete_unique_netherlands_jobs() {
    let client = live_client();
    let sources: Vec<(&str, Box<dyn JobSource>)> = vec![
        (
            "IMC Trading",
            Box::new(
                GreenhouseSource::new("imc", "imc", client.clone()).with_country_filter(Some("NL")),
            ),
        ),
        (
            "Flow Traders",
            Box::new(
                GreenhouseSource::new("flow-traders", "flowtraders", client.clone())
                    .with_country_filter(Some("NL")),
            ),
        ),
        (
            "bunq",
            Box::new(RecruiteeSource::new(
                "bunq",
                "https://bunq.recruitee.com",
                client.clone(),
            )),
        ),
        (
            "DPG Media",
            Box::new(RecruiteeSource::new(
                "dpg-media",
                "https://vacatures.dpgmedia.nl",
                client.clone(),
            )),
        ),
        (
            "Miro",
            Box::new(job_watch::sources::ashby::AshbySource::new(
                "miro",
                "miro",
                client.clone(),
            )),
        ),
        (
            "Checkout.com",
            Box::new(job_watch::sources::ashby::AshbySource::new(
                "checkout-com",
                "checkout.com",
                client.clone(),
            )),
        ),
        (
            "Fourthline",
            Box::new(
                GreenhouseSource::new("fourthline", "fourthline", client.clone())
                    .with_country_filter(Some("NL")),
            ),
        ),
        (
            "Ockto",
            Box::new(RecruiteeSource::new(
                "ockto",
                "https://ockto.recruitee.com",
                client.clone(),
            )),
        ),
        (
            "DRW",
            Box::new(
                GreenhouseSource::new("drw", "drweng", client.clone())
                    .with_country_filter(Some("NL")),
            ),
        ),
        (
            "Jump Trading",
            Box::new(
                GreenhouseSource::new("jump-trading", "jumptrading", client.clone())
                    .with_country_filter(Some("NL")),
            ),
        ),
        (
            "Tower Research",
            Box::new(
                GreenhouseSource::new("tower-research", "towerresearchcapital", client.clone())
                    .with_country_filter(Some("NL")),
            ),
        ),
        (
            "WEBB Traders",
            Box::new(RecruiteeSource::new(
                "webb-traders",
                "https://webbtraders.recruitee.com",
                client.clone(),
            )),
        ),
        (
            "STX Group",
            Box::new(
                GreenhouseSource::new("stx-group", "stxgroup", client.clone())
                    .with_country_filter(Some("NL")),
            ),
        ),
        (
            "Elastic",
            Box::new(
                GreenhouseSource::new("elastic", "elastic", client.clone())
                    .with_country_filter(Some("NL")),
            ),
        ),
        (
            "MultiSafepay",
            Box::new(RecruiteeSource::new(
                "multisafepay",
                "https://careers.multisafepay.com",
                client.clone(),
            )),
        ),
        (
            "ACT Commodities",
            Box::new(
                GreenhouseSource::new("act-commodities", "testendouble", client)
                    .with_country_filter(Some("NL")),
            ),
        ),
    ];

    for (name, source) in sources {
        let jobs = complete_jobs(source.scan().await.unwrap());
        assert_live_jobs(name, &jobs);
    }
}

#[tokio::test]
#[ignore = "live external source"]
async fn jibe_live_returns_complete_unique_jobs() {
    let client = live_client();
    let source = JibeSource::new(
        "booking-com",
        "https://jobs.booking.com",
        "Booking.com",
        client,
    );
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("Booking.com", &jobs);
}

#[tokio::test]
#[ignore = "live external source"]
async fn recruitee_live_returns_complete_unique_jobs() {
    let client = live_client();
    let source = RecruiteeSource::new("funda", "https://jobs.funda.nl", client);
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("Funda", &jobs);
}

#[tokio::test]
#[ignore = "live external source"]
async fn recruitee_live_returns_complete_unique_centric_jobs() {
    let client = live_client();
    let source = RecruiteeSource::new("centric", "https://centric.recruitee.com", client);
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("Centric", &jobs);
    assert!(jobs.iter().all(|job| job.countries == ["NL"]));
    assert!(
        jobs.iter()
            .all(|job| job.job_url.starts_with("https://centric.recruitee.com/"))
    );
    println!("Centric: {} jobs", jobs.len());
}

#[tokio::test]
#[ignore = "live external source"]
async fn recruitee_live_returns_complete_unique_cmcom_jobs() {
    let client = live_client();
    let source = RecruiteeSource::new("cmcom", "https://cmcom.recruitee.com", client);
    let jobs = complete_jobs(source.scan().await.unwrap());
    assert_live_jobs("CM.com", &jobs);
    assert!(jobs.iter().any(|job| job.countries.contains(&"NL".into())));
    assert!(
        jobs.iter()
            .all(|job| job.job_url.starts_with("https://jobs.cm.com/"))
    );
    println!("CM.com: {} jobs", jobs.len());
}

fn greenhouse_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/greenhouse/adyen.json")).unwrap()
}

fn jibe_fixtures() -> Vec<serde_json::Value> {
    [
        include_str!("fixtures/jibe/booking-page-1.json"),
        include_str!("fixtures/jibe/booking-page-2.json"),
    ]
    .map(|raw| serde_json::from_str(raw).unwrap())
    .into()
}

fn recruitee_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/recruitee/funda.json")).unwrap()
}

fn assert_jibe_fixture_error(pages: Vec<serde_json::Value>) {
    let raw = pages
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>();
    let refs = raw.iter().map(String::as_str).collect::<Vec<_>>();
    assert_schema_error(parse_jibe_pages(
        "booking-com",
        "https://jobs.booking.com",
        "Booking.com",
        &refs,
    ));
}

fn assert_schema_error(result: Result<Vec<ObservedJob>, job_watch::sources::SourceError>) {
    let error = result.unwrap_err();
    assert_eq!(error.kind, SourceErrorKind::Schema);
    assert!(!error.retryable);
}

fn live_client() -> reqwest::Client {
    build_client(
        "job-watch/0.1 (+hosted ATS live test)",
        Duration::from_secs(20),
    )
    .unwrap()
}

fn complete_jobs(scan: SourceScan) -> Vec<ObservedJob> {
    let SourceScan::Complete { observations } = scan else {
        panic!("hosted ATS scans must be complete");
    };
    observations
}

fn assert_live_jobs(source: &str, jobs: &[ObservedJob]) {
    assert!(!jobs.is_empty());
    let mut ids = HashSet::new();
    for job in jobs {
        assert!(ids.insert(&job.source_id));
        assert!(!job.source_id.trim().is_empty());
        assert!(!job.title.trim().is_empty());
        assert!(!job.locations.is_empty());
        assert!(!job.countries.is_empty());
        assert!(!job.job_url.trim().is_empty());
        assert!(!job.apply_url.trim().is_empty());
        assert!(!job.description.trim().is_empty());
        assert!(job.raw_payload.is_object());
        assert!(job.published_at.is_some());
    }
    println!("{source}: {} jobs", jobs.len());
}
