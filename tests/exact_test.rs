use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        exact::{ExactSource, parse_exact_pages},
    },
};

const LISTING: &str = r#"<html><body><div class="card card--vacancy"><div class="card__body"><h2><a data-card-link href="/careers/vacancies/a0t123-platform-engineer">Platform Engineer</a></h2><div class="label label--ghost">Technology</div><div class="label label--ghost">Netherlands</div></div></div></body></html>"#;
const DETAIL: &str = r#"<html><script type="application/ld+json">{"@type":"JobPosting","title":"Platform Engineer","description":"&lt;p&gt;Build &lt;strong&gt;reliable&lt;/strong&gt; software.&lt;/p&gt;","datePosted":"2026-08-01 00:00:00","employmentType":"FULL_TIME","hiringOrganization":{"name":"Exact"},"jobLocation":{"address":{"addressLocality":"Delft","addressCountry":"NL"}}}</script></html>"#;

#[test]
fn parses_a_complete_exact_board_and_detail() {
    let jobs = parse_exact_pages(
        "exact",
        "https://www.exact.com/careers/vacancies",
        &[LISTING],
        &[DETAIL],
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "a0t123");
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].locations, ["Delft"]);
    assert!(jobs[0].description.contains("reliable"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn exact_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+Exact live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = ExactSource::new("exact", "https://www.exact.com/careers/vacancies", client);
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Exact scan incomplete"),
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
    println!("Exact: {} NL jobs", jobs.len());
}
