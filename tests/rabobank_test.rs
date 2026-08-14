use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        ashby::build_client,
        rabobank::{RabobankSource, parse_rabobank_pages},
    },
};

#[test]
fn parses_a_complete_rabobank_snapshot() {
    let jobs = parse_rabobank_pages(
        "rabobank",
        "https://rabobank.jobs",
        "nl",
        &[include_str!("fixtures/rabobank/page-1.json")],
        include_str!("fixtures/rabobank/sitemap.xml"),
    )
    .unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "JR_00150001");
    assert_eq!(jobs[0].title, "Senior Software Engineer");
    assert_eq!(jobs[0].locations, ["Utrecht"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].department.as_deref(), Some("IT"));
    assert_eq!(jobs[0].employment_type.as_deref(), Some("Fulltime"));
    assert_eq!(jobs[0].description, "Build reliable banking software.");
    assert_eq!(
        jobs[0].job_url,
        "https://rabobank.jobs/nl/vacature/senior-software-engineer/JR_00150001/"
    );
    assert!(jobs[0].published_at.is_some());
}

#[tokio::test]
#[ignore = "live external source"]
async fn rabobank_live_returns_complete_unique_netherlands_jobs() {
    let client = build_client(
        "job-watch/0.1 (+Rabobank live test)",
        Duration::from_secs(20),
    )
    .unwrap();
    let source = RabobankSource::new("rabobank", "https://rabobank.jobs", "nl", client);

    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("Rabobank scans must be complete");
    };
    assert!(!observations.is_empty());
    let mut ids = HashSet::new();
    for job in &observations {
        assert!(ids.insert(&job.source_id));
        assert_eq!(job.countries, ["NL"]);
        assert!(!job.title.trim().is_empty());
        assert!(!job.description.trim().is_empty());
    }
    println!("Rabobank: {} complete Netherlands jobs", observations.len());
}
