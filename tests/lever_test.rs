use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        lever::{LeverSource, parse_lever_response},
    },
};

const BOARD: &str = r#"[{"id":"job-1","text":"Platform Engineer","categories":{"location":"Amsterdam, The Netherlands","allLocations":["Amsterdam, The Netherlands"],"department":"Engineering","team":"Platform","commitment":"Full Time"},"descriptionPlain":"Build reliable systems.","lists":[],"hostedUrl":"https://jobs.eu.lever.co/acme/job-1","applyUrl":"https://jobs.eu.lever.co/acme/job-1/apply","createdAt":1786382005187},{"id":"job-2","text":"Engineer","categories":{"location":"Berlin","allLocations":["Berlin"]},"descriptionPlain":"Build systems.","lists":[],"hostedUrl":"https://jobs.eu.lever.co/acme/job-2","applyUrl":"https://jobs.eu.lever.co/acme/job-2/apply","createdAt":1786382005187}]"#;

#[test]
fn parses_and_filters_a_complete_lever_board() {
    let jobs = parse_lever_response("finom", BOARD, Some("NL")).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "job-1");
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].department.as_deref(), Some("Engineering"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn finom_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("job-watch/0.1 (+Lever live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = LeverSource::new(
        "finom",
        "https://api.eu.lever.co/v0/postings/pnlfin",
        client,
    )
    .with_country_filter(Some("NL"));
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Finom scan incomplete"),
    };

    assert!(!jobs.is_empty());
    assert!(jobs.iter().all(|job| job.countries.contains(&"NL".into())));
    assert_eq!(
        jobs.iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        jobs.len()
    );
    println!("Finom: {} NL jobs", jobs.len());
}

#[tokio::test]
#[ignore = "live external source"]
async fn tomtom_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("job-watch/0.1 (+Lever live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = LeverSource::new(
        "tomtom",
        "https://api.eu.lever.co/v0/postings/tomtom",
        client,
    )
    .with_country_filter(Some("NL"));
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("TomTom scan incomplete"),
    };

    assert!(!jobs.is_empty());
    assert!(jobs.iter().all(|job| job.countries.contains(&"NL".into())));
    assert_eq!(
        jobs.iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        jobs.len()
    );
    println!("TomTom: {} NL jobs", jobs.len());
}
