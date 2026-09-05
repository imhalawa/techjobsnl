use std::{collections::HashSet, time::Duration};

use techjobsnl::{
    domain::SourceScan,
    sources::{
        JobSource,
        ashby::build_client,
        eneco::{EnecoSource, parse_eneco_pages},
    },
};

#[test]
fn parses_a_complete_eneco_snapshot() {
    let jobs = parse_eneco_pages(
        "eneco",
        "https://www.werkenbijeneco.nl/vacatures?f=1270",
        &[include_str!("fixtures/eneco/list.html")],
        &[
            include_str!("fixtures/eneco/detail-1001.html"),
            include_str!("fixtures/eneco/detail-1002.html"),
        ],
    )
    .unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "1001");
    assert_eq!(jobs[0].title, "Platform Engineer");
    assert_eq!(jobs[0].locations, ["Rotterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].description, "Build clean energy platforms.");
    assert_eq!(
        jobs[0].job_url,
        "https://www.werkenbijeneco.nl/vacatures/platform-engineer-1001"
    );
    assert_eq!(jobs[0].job_url, jobs[0].apply_url);
    assert!(jobs[0].published_at.is_some());
}

#[tokio::test]
#[ignore = "live external source"]
async fn eneco_live_returns_complete_unique_tech_jobs() {
    let client =
        build_client("techjobsnl/0.1 (+Eneco live test)", Duration::from_secs(20)).unwrap();
    let source = EnecoSource::new(
        "eneco",
        "https://www.werkenbijeneco.nl/vacatures?f=1270",
        client,
    );

    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("Eneco scans must be complete");
    };
    assert!(!observations.is_empty());
    let mut ids = HashSet::new();
    for job in &observations {
        assert!(ids.insert(&job.source_id));
        assert_eq!(job.countries, ["NL"]);
        assert!(!job.title.trim().is_empty());
        assert!(!job.description.trim().is_empty());
    }
    println!("Eneco: {} complete Tech jobs", observations.len());
}
