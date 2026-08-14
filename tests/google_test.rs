use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        google::{GoogleSource, parse_google_pages},
    },
};
use serde_json::{Value, json};

const SEARCH_URL: &str = "https://www.google.com/about/careers/applications/jobs/results/?company=Google&location=Netherlands&sort_by=date";

fn job(id: u64, locations: Value) -> Value {
    json!([
        id.to_string(),
        format!("Cloud Engineer {id}"),
        format!("https://www.google.com/about/careers/applications/signin?jobId={id}"),
        [null, "<ul><li>Build reliable systems.</li></ul>"],
        [
            null,
            "<h3>Minimum qualifications:</h3><p>Engineering experience.</p>"
        ],
        "projects/gweb-careers-proto/tenants/google/companies/google",
        null,
        "Google",
        "en-US",
        locations,
        [null, "<p>Build cloud products for customers.</p>"],
        [2],
        [1786468482_i64, 0],
        [1786468482_i64, 0],
        [1786468482_i64, 0],
        [null, ""],
        null,
        null,
        [null, ""],
        [null, "<p>Engineering experience.</p>"],
        2
    ])
}

fn page(jobs: Vec<Value>, total: usize) -> String {
    format!(
        "<script>AF_initDataCallback({{key: 'ds:1', hash: '2', data:{}, sideChannel: {{}}}});</script>",
        json!([jobs, null, total, jobs.len()])
    )
}

#[test]
fn parses_complete_google_results_and_keeps_only_explicit_nl_locations() {
    let html = page(
        vec![
            job(
                101,
                json!([
                    [
                        "Amsterdam, Netherlands",
                        ["Amsterdam, Netherlands"],
                        "Amsterdam",
                        null,
                        "NH",
                        "NL"
                    ],
                    [
                        "Dublin, Ireland",
                        ["Dublin, Ireland"],
                        "Dublin",
                        null,
                        "D",
                        "IE"
                    ]
                ]),
            ),
            job(
                102,
                json!([[
                    "Brussels, Belgium",
                    ["Brussels, Belgium"],
                    "Brussels",
                    null,
                    "BRU",
                    "BE"
                ]]),
            ),
        ],
        2,
    );
    let jobs = parse_google_pages("google", SEARCH_URL, &[html.as_str()]).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "101");
    assert_eq!(jobs[0].title, "Cloud Engineer 101");
    assert_eq!(jobs[0].locations, ["Amsterdam, Netherlands"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].job_url,
        "https://www.google.com/about/careers/applications/jobs/results/101"
    );
    assert!(jobs[0].apply_url.starts_with("https://www.google.com/"));
    assert!(jobs[0].description.contains("Build cloud products"));
    assert!(jobs[0].description.contains("Minimum qualifications"));
    assert!(jobs[0].published_at.is_some());
}

#[test]
fn rejects_incomplete_google_pagination() {
    let jobs = (1..=20)
        .map(|id| {
            job(
                id,
                json!([[
                    "Amsterdam, Netherlands",
                    ["Amsterdam, Netherlands"],
                    "Amsterdam",
                    null,
                    "NH",
                    "NL"
                ]]),
            )
        })
        .collect();
    let html = page(jobs, 21);
    let error = parse_google_pages("google", SEARCH_URL, &[html.as_str()]).unwrap_err();

    assert!(error.message.contains("incomplete search pagination"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn google_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("job-watch/0.1 (+Google live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = GoogleSource::new("google", SEARCH_URL, client);
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Google scan incomplete"),
    };

    assert!(!jobs.is_empty());
    assert!(jobs.iter().all(|job| job.countries == ["NL"]));
    assert!(jobs.iter().all(|job| job.published_at.is_some()));
    assert!(jobs.iter().all(|job| !job.description.is_empty()));
    assert!(jobs.iter().all(|job| {
        job.job_url
            .starts_with("https://www.google.com/about/careers/applications/jobs/results/")
            && job
                .apply_url
                .starts_with("https://www.google.com/about/careers/applications/signin?")
    }));
    assert_eq!(
        jobs.iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        jobs.len()
    );
    println!("Google: {} NL jobs", jobs.len());
}
