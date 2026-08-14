use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::{ObservedJob, SourceScan},
    sources::{
        JobSource,
        personio::{PersonioSource, parse_personio_feed},
    },
};

const FEED: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<workzag-jobs><position><id>42</id><office>NL</office><department>Engineering</department>
<recruitingCategory>Platform</recruitingCategory><name>Software Engineer</name>
<jobDescriptions><jobDescription><name>Role</name><value><![CDATA[<p>Build payment systems.</p>]]></value></jobDescription></jobDescriptions>
<employmentType>permanent</employmentType><createdAt>2026-08-01T10:00:00+00:00</createdAt></position></workzag-jobs>"#;

#[test]
fn parses_a_complete_personio_feed() {
    let jobs =
        parse_personio_feed("silverflow", "https://silverflow.jobs.personio.com", FEED).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "42");
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].description, "## Role\n\nBuild payment systems.");
    assert!(jobs[0].job_url.contains("/job/42"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn personio_live_returns_complete_unique_jobs() {
    for (id, name, base) in [
        (
            "silverflow",
            "Silverflow",
            "https://silverflow.jobs.personio.com",
        ),
        ("ohpen", "Ohpen", "https://ohpen.jobs.personio.com"),
    ] {
        let client = reqwest::Client::builder()
            .user_agent("job-watch/0.1 (+Personio live test)")
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap();
        let source = PersonioSource::new(id, base, client);
        let jobs = match source.scan().await.unwrap() {
            SourceScan::Complete { observations } => observations,
            SourceScan::Incomplete { .. } => panic!("{name} scan incomplete"),
        };
        assert_live(name, &jobs);
    }
}

fn assert_live(name: &str, jobs: &[ObservedJob]) {
    assert!(!jobs.is_empty(), "{name} returned no jobs");
    assert_eq!(
        jobs.iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        jobs.len()
    );
    println!("{name}: {} jobs", jobs.len());
}
