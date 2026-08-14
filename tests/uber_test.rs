use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        uber::{UberSource, parse_uber_responses},
    },
};

const PAGE: &str = r#"{"items":[{"TotalJobsCount":2,"Limit":100,"Offset":0,"requisitionList":[{"Id":"159438","Title":"Senior People Partner","PostedDate":"2026-08-10","PrimaryLocationCountry":"NL","PrimaryLocation":"Amsterdam, Noord-Holland, Netherlands"},{"Id":"300713","Title":"Deployment Manager","PostedDate":"2026-07-29","PrimaryLocationCountry":"GB","PrimaryLocation":"London, United Kingdom"}]}]}"#;
const NL_DETAIL: &str = r#"{"items":[{"Id":"159438","Title":"Senior People Partner","Category":"People","ExternalPostedStartDate":"2026-08-10T11:53:04+00:00","JobSchedule":"Full time","ExternalDescriptionStr":"<p>Support Uber employees.</p>","CorporateDescriptionStr":"<p>Ready to ride?</p>","PrimaryLocation":"Amsterdam, Noord-Holland, Netherlands","PrimaryLocationCountry":"NL","secondaryLocations":[]}]}"#;
const MULTI_DETAIL: &str = r#"{"items":[{"Id":"300713","Title":"Deployment Manager","Category":"Operations","ExternalPostedStartDate":"2026-07-29T08:00:00+00:00","JobSchedule":"Full time","ExternalDescriptionStr":"<p>Deploy infrastructure.</p>","CorporateDescriptionStr":"","PrimaryLocation":"London, United Kingdom","PrimaryLocationCountry":"GB","secondaryLocations":[{"Name":"Amsterdam, Noord-Holland, Netherlands","CountryCode":"NL"},{"Name":"Berlin, Germany","CountryCode":"DE"}]}]}"#;

#[test]
fn parses_a_complete_uber_page_and_validates_netherlands_locations() {
    let jobs = parse_uber_responses(
        "uber",
        "https://iaziqy.fa.ocs.oraclecloud.com/hcmRestApi/resources/latest/",
        &[PAGE],
        &[NL_DETAIL, MULTI_DETAIL],
    )
    .unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "159438");
    assert_eq!(jobs[0].department.as_deref(), Some("People"));
    assert_eq!(jobs[0].employment_type.as_deref(), Some("Full time"));
    assert_eq!(jobs[0].locations, ["Amsterdam, Noord-Holland, Netherlands"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].job_url, "https://jobs.uber.com/en/jobs/159438/");
    assert_eq!(
        jobs[0].apply_url,
        "https://iaziqy.fa.ocs.oraclecloud.com/hcmUI/CandidateExperience/en/sites/UberCareers/job/159438"
    );
    assert_eq!(
        jobs[1].locations,
        [
            "London, United Kingdom",
            "Amsterdam, Noord-Holland, Netherlands",
            "Berlin, Germany"
        ]
    );
    assert_eq!(jobs[1].countries, ["GB", "NL", "DE"]);
}

#[test]
fn rejects_an_incomplete_uber_listing() {
    let incomplete = PAGE.replace("\"TotalJobsCount\":2", "\"TotalJobsCount\":3");
    let error = parse_uber_responses(
        "uber",
        "https://iaziqy.fa.ocs.oraclecloud.com/hcmRestApi/resources/latest/",
        &[&incomplete],
        &[NL_DETAIL, MULTI_DETAIL],
    )
    .unwrap_err();

    assert!(error.to_string().contains("incomplete pagination"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn uber_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+Uber live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = UberSource::new(
        "uber",
        "https://iaziqy.fa.ocs.oraclecloud.com/hcmRestApi/resources/latest/",
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Uber scan incomplete"),
    };

    assert!(!jobs.is_empty());
    assert!(
        jobs.iter()
            .all(|job| job.countries.iter().any(|code| code == "NL"))
    );
    assert!(jobs.iter().all(|job| {
        job.job_url.starts_with("https://jobs.uber.com/en/jobs/")
            && job
                .apply_url
                .starts_with("https://iaziqy.fa.ocs.oraclecloud.com/")
    }));
    assert_eq!(
        jobs.iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        jobs.len()
    );
    println!("Uber: {} NL jobs", jobs.len());
}
