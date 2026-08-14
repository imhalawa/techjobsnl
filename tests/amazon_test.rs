use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        amazon::{AmazonSource, parse_amazon_pages},
    },
};

const PAGE_1: &str = r#"{"hits":2,"jobs":[{"id":"feed-1","id_icims":"1001","title":"Cloud Engineer","country_code":"NLD","city":"Amsterdam","location":"NL, Amsterdam","normalized_location":"Amsterdam, North Holland, NLD","job_path":"/en/jobs/1001/cloud-engineer","url_next_step":"https://account.amazon.jobs/jobs/1001/apply","posted_date":"August 13, 2026","job_category":"Software Development","job_family":"Software Development","job_schedule_type":"full-time","company_name":"Amazon Web Services EMEA SARL, Dutch Branch","description":"<p>Build reliable cloud services.</p>","basic_qualifications":"<p>Production experience.</p>","preferred_qualifications":"<p>Rust experience.</p>"}]}"#;
const PAGE_2: &str = r#"{"hits":2,"jobs":[{"id":"feed-2","id_icims":"1002","title":"Data Engineer","country_code":"NLD","city":"Den Haag","location":"NL, Den Haag","normalized_location":"The Hague, South Holland, NLD","job_path":"/en/jobs/1002/data-engineer","url_next_step":"https://account.amazon.com/jobs/1002/apply","posted_date":"August 12, 2026","job_category":"Data Science","job_family":"Data Engineering","job_schedule_type":"full-time","company_name":"Amazon Development Center (Netherlands) B.V.","description":"<p>Build data products.</p>","basic_qualifications":"<p>SQL experience.</p>","preferred_qualifications":null}]}"#;

#[test]
fn parses_every_declared_amazon_page_into_unique_netherlands_jobs() {
    let jobs = parse_amazon_pages(
        "amazon",
        "https://www.amazon.jobs/en/search.json?normalized_country_code%5B%5D=NLD&offset=0&result_limit=1",
        &[PAGE_1, PAGE_2],
    )
    .unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "1001");
    assert_eq!(jobs[0].title, "Cloud Engineer");
    assert_eq!(jobs[0].department.as_deref(), Some("Software Development"));
    assert_eq!(jobs[0].team.as_deref(), Some("Software Development"));
    assert_eq!(jobs[0].locations, ["Amsterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].job_url,
        "https://www.amazon.jobs/en/jobs/1001/cloud-engineer"
    );
    assert_eq!(
        jobs[0].apply_url,
        "https://account.amazon.jobs/jobs/1001/apply"
    );
    assert!(
        jobs[0]
            .description
            .contains("Build reliable cloud services.")
    );
    assert!(jobs[0].description.contains("Production experience."));
    assert!(jobs[0].description.contains("Rust experience."));
    assert_eq!(
        jobs.iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        jobs.len()
    );
}

#[test]
fn rejects_incomplete_amazon_pagination() {
    let error = parse_amazon_pages(
        "amazon",
        "https://www.amazon.jobs/en/search.json?normalized_country_code%5B%5D=NLD&offset=0&result_limit=1",
        &[PAGE_1],
    )
    .unwrap_err();

    assert!(error.message.contains("incomplete search pagination"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn amazon_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+Amazon live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = AmazonSource::new(
        "amazon",
        "https://www.amazon.jobs/en/search.json?normalized_country_code%5B%5D=NLD&offset=0&result_limit=100",
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Amazon scan incomplete"),
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
    println!("Amazon / AWS: {} NL jobs", jobs.len());
}
