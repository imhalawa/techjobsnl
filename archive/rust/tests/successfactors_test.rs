use std::{collections::HashSet, time::Duration};

use techjobsnl::{
    domain::SourceScan,
    sources::{
        JobSource,
        ashby::build_client,
        successfactors::{SuccessFactorsSource, parse_successfactors_pages},
    },
};

const LISTING_URL: &str = "https://jobs.flatexdegiro.com/search/?q=&locationsearch=NL";

#[test]
fn parses_a_complete_successfactors_snapshot() {
    let jobs = parse_successfactors_pages(
        "flatexdegiro",
        LISTING_URL,
        "flatexDEGIRO AG",
        &[
            include_str!("fixtures/successfactors/degiro-list.html"),
            include_str!("fixtures/successfactors/degiro-list-2.html"),
        ],
        &[
            include_str!("fixtures/successfactors/degiro-detail-1001.html"),
            include_str!("fixtures/successfactors/degiro-detail-1002.html"),
        ],
    )
    .unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "1001");
    assert_eq!(jobs[0].title, "Platform Engineer");
    assert_eq!(jobs[0].locations, ["Amsterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].description, "Build the trading platform.");
    assert_eq!(
        jobs[0].job_url,
        "https://jobs.flatexdegiro.com/job/Amsterdam-Platform-Engineer-1096-HA/1001/"
    );
    assert_eq!(
        jobs[0].apply_url,
        "https://jobs.flatexdegiro.com/talentcommunity/apply/1001/?locale=en_US"
    );
    assert!(jobs[0].published_at.is_some());
}

#[test]
fn rejects_an_incomplete_successfactors_snapshot() {
    let error = parse_successfactors_pages(
        "flatexdegiro",
        LISTING_URL,
        "flatexDEGIRO AG",
        &[
            include_str!("fixtures/successfactors/degiro-list.html"),
            include_str!("fixtures/successfactors/degiro-list-2.html"),
        ],
        &[include_str!(
            "fixtures/successfactors/degiro-detail-1001.html"
        )],
    )
    .unwrap_err();

    assert!(error.message.contains("listing/detail count mismatch"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn flatexdegiro_live_returns_every_nl_job() {
    let client = build_client(
        "techjobsnl/0.1 (+flatexDEGIRO live test)",
        Duration::from_secs(30),
    )
    .unwrap();
    let source = SuccessFactorsSource::new("flatexdegiro", LISTING_URL, "flatexDEGIRO AG", client);

    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("flatexDEGIRO scans must be complete");
    };
    assert!(!observations.is_empty());
    let mut ids = HashSet::new();
    for job in &observations {
        assert!(ids.insert(&job.source_id));
        assert_eq!(job.countries, ["NL"]);
        assert!(!job.title.trim().is_empty());
        assert!(!job.description.trim().is_empty());
        assert!(job.published_at.is_some());
        assert!(
            job.job_url
                .starts_with("https://jobs.flatexdegiro.com/job/")
        );
        assert!(
            job.apply_url
                .starts_with("https://jobs.flatexdegiro.com/talentcommunity/apply/")
        );
    }
    println!("flatexDEGIRO: {} complete NL jobs", observations.len());
}
