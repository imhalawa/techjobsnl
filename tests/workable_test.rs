use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        workable::{WorkableSource, parse_workable_job},
    },
};

const JOB: &str = r#"{"id":4024494,"shortcode":"97D3AEF605","title":"Implementation Engineer","locations":[{"country":"Netherlands","countryCode":"NL","city":"Utrecht","region":"Utrecht","hidden":false}],"published":"2026-07-30T00:00:00.000Z","type":"full","department":["Life & Pension"],"description":"<p>Build reliable software.</p>","requirements":"<p>Know APIs.</p>","benefits":"<p>Hybrid work.</p>"}"#;

#[test]
fn parses_a_complete_workable_job() {
    let raw: serde_json::Value = serde_json::from_str(JOB).unwrap();
    let job = parse_workable_job("keylane", "keylane", raw).unwrap();

    assert_eq!(job.source_id, "4024494");
    assert_eq!(job.countries, ["NL"]);
    assert_eq!(job.locations, ["Utrecht, Utrecht, Netherlands"]);
    assert_eq!(job.department.as_deref(), Some("Life & Pension"));
    assert!(job.description.contains("Build reliable software."));
    assert_eq!(
        job.job_url,
        "https://apply.workable.com/keylane/j/97D3AEF605/"
    );
}

#[tokio::test]
#[ignore = "live external source"]
async fn keylane_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("job-watch/0.1 (+Workable live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = WorkableSource::new("keylane", "keylane", client).with_country_filter(Some("NL"));
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Keylane scan incomplete"),
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
    println!("Keylane: {} NL jobs", jobs.len());
}
