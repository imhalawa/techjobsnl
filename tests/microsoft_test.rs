use std::{collections::HashSet, time::Duration};

use serde_json::{Value, json};
use techjobsnl::{
    domain::SourceScan,
    sources::{
        JobSource,
        microsoft::{MicrosoftSource, parse_microsoft_pages},
    },
};

const SEARCH_URL: &str = "https://apply.careers.microsoft.com/api/pcsx/search?domain=microsoft.com&query=&location=Netherlands&start=0&hl=en";

fn listing(id: u64, job_id: u64, country: &str) -> Value {
    json!({
        "id": id,
        "displayJobId": job_id.to_string(),
        "name": format!("Engineer {job_id}"),
        "locations": [format!("{country}, Noord-Holland, Amsterdam")],
        "standardizedLocations": [if country == "Netherlands" { "Amsterdam, NH, NL" } else { "Brussels, Brussels, BE" }],
        "postedTs": 1786006383_i64,
        "department": "Software Engineering",
        "atsJobId": job_id.to_string(),
        "positionUrl": format!("/careers/job/{id}")
    })
}

fn detail(id: u64, job_id: u64, country: &str) -> String {
    json!({
        "status": 200,
        "data": {
            "id": id,
            "displayJobId": job_id.to_string(),
            "name": format!("Engineer {job_id}"),
            "locations": [format!("{country}, Noord-Holland, Amsterdam")],
            "standardizedLocations": [if country == "Netherlands" { "Amsterdam, NH, NL" } else { "Brussels, Brussels, BE" }],
            "postedTs": 1786006383_i64,
            "department": "Software Engineering",
            "atsJobId": job_id.to_string(),
            "positionUrl": format!("/careers/job/{id}"),
            "publicUrl": format!("https://apply.careers.microsoft.com/careers/job/{id}"),
            "jobDescription": "<p>Build reliable cloud software.</p>",
            "efcustomTextEmploymentType": ["Full-Time"]
        }
    })
    .to_string()
}

fn page(count: usize, positions: Vec<Value>) -> String {
    json!({"status": 200, "data": {"count": count, "positions": positions}}).to_string()
}

fn complete_fixture() -> (Vec<String>, Vec<String>) {
    let listings = (1..=11)
        .map(|id| {
            listing(
                id,
                200_000_000 + id,
                if id == 11 { "Belgium" } else { "Netherlands" },
            )
        })
        .collect::<Vec<_>>();
    let pages = vec![
        page(11, listings[..10].to_vec()),
        page(11, listings[10..].to_vec()),
    ];
    let details = (1..=11)
        .map(|id| {
            detail(
                id,
                200_000_000 + id,
                if id == 11 { "Belgium" } else { "Netherlands" },
            )
        })
        .collect();
    (pages, details)
}

#[test]
fn parses_complete_microsoft_pagination_and_keeps_only_explicit_nl_locations() {
    let (pages, details) = complete_fixture();
    let page_refs = pages.iter().map(String::as_str).collect::<Vec<_>>();
    let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();
    let jobs = parse_microsoft_pages("microsoft", SEARCH_URL, &page_refs, &detail_refs).unwrap();

    assert_eq!(jobs.len(), 10);
    assert_eq!(jobs[0].source_id, "200000001");
    assert_eq!(jobs[0].title, "Engineer 200000001");
    assert_eq!(jobs[0].department.as_deref(), Some("Software Engineering"));
    assert_eq!(jobs[0].employment_type.as_deref(), Some("Full-Time"));
    assert_eq!(jobs[0].locations, ["Netherlands, Noord-Holland, Amsterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].job_url,
        "https://apply.careers.microsoft.com/careers/job/1"
    );
    assert_eq!(jobs[0].apply_url, jobs[0].job_url);
    assert!(
        jobs[0]
            .description
            .contains("Build reliable cloud software.")
    );
    assert_eq!(
        jobs.iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        jobs.len()
    );
}

#[test]
fn rejects_incomplete_microsoft_pagination() {
    let (pages, details) = complete_fixture();
    let error = parse_microsoft_pages(
        "microsoft",
        SEARCH_URL,
        &[pages[0].as_str()],
        &details.iter().map(String::as_str).collect::<Vec<_>>(),
    )
    .unwrap_err();

    assert!(error.message.contains("incomplete search pagination"));
}

#[test]
fn rejects_listing_detail_identity_drift() {
    let (pages, mut details) = complete_fixture();
    let mut changed: Value = serde_json::from_str(&details[0]).unwrap();
    changed["data"]["displayJobId"] = "different".into();
    details[0] = changed.to_string();
    let error = parse_microsoft_pages(
        "microsoft",
        SEARCH_URL,
        &pages.iter().map(String::as_str).collect::<Vec<_>>(),
        &details.iter().map(String::as_str).collect::<Vec<_>>(),
    )
    .unwrap_err();

    assert!(error.message.contains("listing/detail mismatch"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn microsoft_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+Microsoft live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = MicrosoftSource::new("microsoft", SEARCH_URL, client);
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Microsoft scan incomplete"),
    };

    assert!(!jobs.is_empty());
    assert!(jobs.iter().all(|job| job.countries == ["NL"]));
    assert_eq!(
        jobs.iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        jobs.len()
    );
    println!("Microsoft: {} NL jobs", jobs.len());
}
