use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use job_watch::{
    config::Config,
    domain::{SourceErrorKind, SourceScan},
    filter::EligibilityFilter,
    sources::{
        JobSource,
        ashby::{
            AshbySource, build_client, parse_ashby_response, parse_ashby_response_with_overrides,
        },
    },
};

#[test]
fn parses_and_filters_sanitized_datasnipper_board() {
    let jobs = parse_ashby_response(
        "datasnipper",
        include_str!("fixtures/ashby/datasnipper.json"),
    )
    .unwrap();
    let config = Config::load(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")).unwrap();
    let filter = EligibilityFilter::new(&config.filters).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].title, "Site Reliability Engineer");
    assert_eq!(jobs[0].locations, ["Amsterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert!(filter.classify(&jobs[0], &HashMap::new()).unwrap().eligible);
}

#[test]
fn parses_and_filters_sanitized_airwallex_board() {
    let jobs =
        parse_ashby_response("airwallex", include_str!("fixtures/ashby/airwallex.json")).unwrap();

    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].source_id, "11111111-1111-4111-8111-111111111111");
    assert_eq!(jobs[0].title, "Senior Software Engineer");
    assert_eq!(jobs[0].department.as_deref(), Some("Engineering"));
    assert_eq!(jobs[0].team.as_deref(), Some("Payments Platform"));
    assert_eq!(jobs[0].employment_type.as_deref(), Some("FullTime"));
    assert_eq!(jobs[0].locations, ["NL - Amsterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].job_url,
        "https://jobs.example.test/airwallex/11111111-1111-4111-8111-111111111111"
    );
    assert_eq!(
        jobs[0].apply_url,
        "https://jobs.example.test/airwallex/11111111-1111-4111-8111-111111111111/application"
    );
    assert_eq!(jobs[0].description, "Build reliable payment services.");
    assert!(jobs[0].published_at.is_some());
    assert_eq!(jobs[0].raw_payload["workplaceType"], "Hybrid");

    assert_eq!(jobs[1].locations, ["UK - London", "NL - Amsterdam"]);
    assert_eq!(jobs[1].countries, ["United Kingdom", "NL"]);
    assert_eq!(jobs[2].locations, ["SG - Singapore"]);
    assert_eq!(jobs[2].countries, ["Singapore"]);

    let config = Config::load(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")).unwrap();
    let filter = EligibilityFilter::new(&config.filters).unwrap();
    let results = jobs
        .iter()
        .map(|job| filter.classify(job, &HashMap::new()).unwrap())
        .collect::<Vec<_>>();
    assert!(results[0].eligible);
    assert!(results[1].eligible);
    assert_eq!(results[2].reason, "outside-configured-countries");
}

#[test]
fn parses_complete_ashby_board() {
    let raw = include_str!("fixtures/ashby/mollie.json");
    let jobs = parse_ashby_response("mollie", raw).unwrap();
    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].source_id, "platform-1");
    assert_eq!(jobs[0].countries, vec!["NL"]);
    assert_eq!(jobs[0].locations, vec!["Amsterdam"]);
    assert!(jobs[0].description.contains("CI/CD"));
    assert_eq!(jobs[1].locations, vec!["Lisbon", "Porto"]);
    assert_eq!(jobs[1].countries, vec!["Portugal"]);
    assert_eq!(jobs[0].raw_payload["workplaceType"], "Hybrid");
}

#[test]
fn resolves_an_ashby_location_when_the_optional_address_is_null() {
    let mut board = fixture_board();
    board["jobs"][0]["address"] = serde_json::Value::Null;
    board["jobs"][0]["location"] = "Amsterdam".into();

    let jobs = parse_ashby_response("mollie", &board.to_string()).unwrap();

    assert_eq!(jobs[0].countries, ["NL"]);
}

#[test]
fn resolves_an_official_location_label_from_company_configuration() {
    let mut board = fixture_board();
    board["jobs"][0]["address"] = serde_json::Value::Null;
    board["jobs"][0]["location"] = "Headquarters".into();
    let overrides = HashMap::from([("Headquarters".to_owned(), "NL".to_owned())]);

    let jobs =
        parse_ashby_response_with_overrides("bitvavo", &board.to_string(), &overrides).unwrap();

    assert_eq!(jobs[0].countries, ["NL"]);
}

#[test]
fn excludes_unlisted_jobs_from_the_board() {
    let mut board = fixture_board();
    board["jobs"][1]["isListed"] = false.into();

    let jobs = parse_ashby_response("mollie", &board.to_string()).unwrap();

    assert_eq!(
        jobs.iter()
            .map(|job| job.source_id.as_str())
            .collect::<Vec<_>>(),
        vec!["platform-1", "support-1"]
    );
}

#[test]
fn preserves_timestamp_text_and_unknown_nested_job_fields() {
    let mut board = fixture_board();
    board["jobs"][1]["address"]["officeCode"] = "LIS".into();
    board["jobs"][1]["secondaryLocations"][0]["locationCode"] = "OPO".into();

    let jobs = parse_ashby_response("mollie", &board.to_string()).unwrap();

    assert_eq!(
        jobs[1].raw_payload["publishedAt"],
        "2026-08-02T10:45:00+00:00"
    );
    assert_eq!(jobs[1].raw_payload["address"]["officeCode"], "LIS");
    assert_eq!(
        jobs[1].raw_payload["secondaryLocations"][0]["locationCode"],
        "OPO"
    );
}

#[test]
fn maps_a_missing_publication_time_to_none() {
    let mut board = fixture_board();
    board["jobs"][0]
        .as_object_mut()
        .unwrap()
        .remove("publishedAt");

    let jobs = parse_ashby_response("mollie", &board.to_string()).unwrap();

    assert!(jobs[0].published_at.is_none());
}

#[test]
fn rejects_unsupported_api_versions_as_schema_errors() {
    let mut board = fixture_board();
    board["apiVersion"] = "2".into();

    assert_schema_error(board);
}

#[test]
fn rejects_a_missing_jobs_collection_as_a_schema_error() {
    let mut board = fixture_board();
    board.as_object_mut().unwrap().remove("jobs");

    assert_schema_error(board);
}

#[test]
fn rejects_empty_job_identity_and_official_urls_as_schema_errors() {
    for field in ["id", "jobUrl"] {
        let mut board = fixture_board();
        board["jobs"][0][field] = " ".into();

        assert_schema_error(board);
    }
}

#[test]
fn rejects_an_invalid_user_agent_before_scanning() {
    let error = build_client("techjobsnl\ninvalid", Duration::from_secs(20)).unwrap_err();

    assert_eq!(error.kind, SourceErrorKind::Configuration);
    assert!(!error.retryable);
    assert_eq!(error.http_status, None);
    assert_eq!(error.retry_after, None);
}

#[test]
fn job_source_contract_is_object_safe() {
    let client = build_client("techjobsnl/0.1", Duration::from_secs(20)).unwrap();
    let source: Box<dyn JobSource> = Box::new(AshbySource::new("mollie", "mollie", client));

    assert_eq!(source.company_id(), "mollie");
}

#[tokio::test]
#[ignore = "live external source"]
async fn scans_live_mollie_board_as_a_complete_unique_result() {
    let client = build_client("techjobsnl/0.1", Duration::from_secs(20)).unwrap();
    let source = AshbySource::new("mollie", "mollie", client);

    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("Ashby scans must be complete");
    };
    assert!(!observations.is_empty());

    let mut source_ids = HashSet::new();
    for job in observations {
        assert!(source_ids.insert(job.source_id.clone()));
        assert!(!job.source_id.is_empty());
        assert!(!job.title.is_empty());
        assert!(!job.locations.is_empty());
        assert!(!job.countries.is_empty());
        assert!(!job.job_url.is_empty());
        assert!(!job.apply_url.is_empty());
        assert!(!job.description.is_empty());
        assert!(job.raw_payload.is_object());
    }
}

#[tokio::test]
#[ignore = "live external source"]
async fn scans_live_airwallex_board_as_a_complete_unique_result() {
    let client = build_client(
        "techjobsnl/0.1 (+Airwallex live test)",
        Duration::from_secs(20),
    )
    .unwrap();
    let source = AshbySource::new("airwallex", "airwallex", client);

    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("Ashby scans must be complete");
    };
    assert!(!observations.is_empty());

    let mut source_ids = HashSet::new();
    for job in &observations {
        assert!(source_ids.insert(&job.source_id));
        assert!(!job.source_id.trim().is_empty());
        assert!(!job.title.trim().is_empty());
        assert!(!job.locations.is_empty());
        assert!(!job.countries.is_empty());
        assert!(!job.job_url.trim().is_empty());
        assert!(!job.apply_url.trim().is_empty());
        assert!(!job.description.trim().is_empty());
        assert!(job.raw_payload.is_object());
    }
    println!("Airwallex: {} complete jobs", observations.len());
}

#[tokio::test]
#[ignore = "live external source"]
async fn scans_live_datasnipper_board_as_a_complete_unique_result() {
    let client = build_client(
        "techjobsnl/0.1 (+DataSnipper live test)",
        Duration::from_secs(20),
    )
    .unwrap();
    let source = AshbySource::new("datasnipper", "datasnipper", client);

    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("Ashby scans must be complete");
    };
    assert!(!observations.is_empty());

    let mut source_ids = HashSet::new();
    for job in &observations {
        assert!(source_ids.insert(&job.source_id));
        assert!(!job.title.trim().is_empty());
        assert!(!job.locations.is_empty());
        assert!(!job.countries.is_empty());
        assert!(!job.job_url.trim().is_empty());
        assert!(!job.apply_url.trim().is_empty());
        assert!(!job.description.trim().is_empty());
    }
    assert!(
        observations
            .iter()
            .any(|job| job.countries.contains(&"NL".into()))
    );
    let config = Config::load(concat!(env!("CARGO_MANIFEST_DIR"), "/config.toml")).unwrap();
    let filter = EligibilityFilter::new(&config.filters).unwrap();
    assert!(observations.iter().any(|job| {
        filter
            .classify(job, &HashMap::new())
            .is_ok_and(|result| result.eligible)
    }));
    println!("DataSnipper: {} complete jobs", observations.len());
}

#[tokio::test]
#[ignore = "live external source"]
async fn scans_live_bitvavo_board_with_its_official_headquarters_mapping() {
    let client = build_client(
        "techjobsnl/0.1 (+Bitvavo live test)",
        Duration::from_secs(20),
    )
    .unwrap();
    let overrides = HashMap::from([("Headquarters".to_owned(), "NL".to_owned())]);
    let source =
        AshbySource::new("bitvavo", "bitvavo", client).with_location_country_overrides(&overrides);

    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("Ashby scans must be complete");
    };
    assert!(!observations.is_empty());
    assert!(observations.iter().all(|job| job.countries == ["NL"]));
    assert_eq!(
        observations
            .iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        observations.len()
    );
    println!("Bitvavo: {} NL jobs", observations.len());
}

fn fixture_board() -> serde_json::Value {
    serde_json::from_str(include_str!("fixtures/ashby/mollie.json")).unwrap()
}

fn assert_schema_error(board: serde_json::Value) {
    let error = parse_ashby_response("mollie", &board.to_string()).unwrap_err();
    assert_eq!(error.kind, SourceErrorKind::Schema);
    assert!(!error.retryable);
    assert_eq!(error.http_status, None);
    assert_eq!(error.retry_after, None);
}
