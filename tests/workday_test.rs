use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        workday::{WorkdaySource, parse_workday_job},
    },
};

const JOB: &str = r#"{"hiringOrganization":{"name":"Acme"},"jobPostingInfo":{"id":"internal-1","title":"Platform Engineer","jobDescription":"<p>Build reliable software.</p>","location":"NLD - Amsterdam","additionalLocations":[],"startDate":"2026-08-13","timeType":"Full time","jobReqId":"R123","jobPostingId":"Platform-Engineer_R123","country":{"descriptor":"Netherlands","id":"country-id"},"jobRequisitionLocation":{"descriptor":"NLD - Amsterdam","country":{"descriptor":"Netherlands","id":"country-id","alpha2Code":"NL"}},"externalUrl":"https://acme.wd3.myworkdayjobs.com/External/job/Platform-Engineer_R123"}}"#;

#[test]
fn parses_a_complete_workday_job() {
    let raw: serde_json::Value = serde_json::from_str(JOB).unwrap();
    let job = parse_workday_job("acme", raw, "NL").unwrap();

    assert_eq!(job.source_id, "R123");
    assert_eq!(job.title, "Platform Engineer");
    assert_eq!(job.locations, ["NLD - Amsterdam"]);
    assert_eq!(job.countries, ["NL"]);
    assert_eq!(job.employment_type.as_deref(), Some("Full time"));
    assert!(job.description.contains("Build reliable software."));
}

#[tokio::test]
#[ignore = "live external source"]
async fn wolters_kluwer_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("job-watch/0.1 (+Workday live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = WorkdaySource::new(
        "wolters-kluwer",
        "https://wk.wd3.myworkdayjobs.com",
        "wk",
        "External",
        "Netherlands",
        "NL",
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Wolters Kluwer scan incomplete"),
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
    println!("Wolters Kluwer: {} NL jobs", jobs.len());
}

#[tokio::test]
#[ignore = "live external source"]
async fn vanderlande_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("job-watch/0.1 (+Workday live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = WorkdaySource::new(
        "vanderlande",
        "https://vanderlande.wd3.myworkdayjobs.com",
        "vanderlande",
        "careers",
        "Netherlands",
        "NL",
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Vanderlande scan incomplete"),
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
    println!("Vanderlande: {} NL jobs", jobs.len());
}
