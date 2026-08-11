use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::{SourceErrorKind, SourceScan},
    sources::{
        JobSource,
        ashby::{AshbySource, build_client, parse_ashby_response},
    },
};

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
    let error = build_client("job-watch\ninvalid", Duration::from_secs(20)).unwrap_err();

    assert_eq!(error.kind, SourceErrorKind::Configuration);
    assert!(!error.retryable);
    assert_eq!(error.http_status, None);
    assert_eq!(error.retry_after, None);
}

#[test]
fn job_source_contract_is_object_safe() {
    let client = build_client("job-watch/0.1", Duration::from_secs(20)).unwrap();
    let source: Box<dyn JobSource> = Box::new(AshbySource::new("mollie", "mollie", client));

    assert_eq!(source.company_id(), "mollie");
}

#[tokio::test]
#[ignore = "live external source"]
async fn scans_live_mollie_board_as_a_complete_unique_result() {
    let client = build_client("job-watch/0.1", Duration::from_secs(20)).unwrap();
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
