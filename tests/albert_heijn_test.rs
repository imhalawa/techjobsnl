use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        albert_heijn::{AlbertHeijnSource, parse_albert_heijn_pages},
        ashby::build_client,
    },
};

#[test]
fn parses_a_complete_albert_heijn_snapshot() {
    let jobs = parse_albert_heijn_pages(
        "ahold",
        "https://werk.ah.nl",
        &[include_str!("fixtures/albert-heijn/page-1.json")],
        &[
            include_str!("fixtures/albert-heijn/detail-47001.html"),
            include_str!("fixtures/albert-heijn/detail-47002.html"),
        ],
    )
    .unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "47001");
    assert_eq!(jobs[0].title, "Senior Data Platform Engineer");
    assert_eq!(jobs[0].locations, ["Zaandam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].department.as_deref(), Some("Data-science"));
    assert_eq!(jobs[0].employment_type.as_deref(), Some("Fulltime"));
    assert_eq!(jobs[0].description, "Build the retail data platform.");
    assert_eq!(
        jobs[0].job_url,
        "https://werk.ah.nl/vacature/47001/senior-data-platform-engineer"
    );
    assert_eq!(jobs[0].job_url, jobs[0].apply_url);
    assert!(jobs[0].published_at.is_some());
}

#[tokio::test]
#[ignore = "live external source"]
async fn albert_heijn_live_returns_complete_unique_tech_jobs() {
    let client = build_client(
        "techjobsnl/0.1 (+Albert Heijn live test)",
        Duration::from_secs(20),
    )
    .unwrap();
    let source = AlbertHeijnSource::new("ahold", "https://werk.ah.nl", client);

    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("Albert Heijn scans must be complete");
    };
    assert!(!observations.is_empty());
    let mut ids = HashSet::new();
    for job in &observations {
        assert!(ids.insert(&job.source_id));
        assert_eq!(job.countries, ["NL"]);
        assert!(!job.title.trim().is_empty());
        assert!(!job.description.trim().is_empty());
    }
    println!("Albert Heijn: {} complete Tech jobs", observations.len());
}
