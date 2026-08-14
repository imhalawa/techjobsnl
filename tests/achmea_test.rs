use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        ns::{ACHMEA_PROFILE, NsSource, parse_hamilton_pages},
    },
};

const LISTING: &str = r#"<span class="vacancy-count">1 t/m 1 van 1</span><ul class="vacancy-list"><li class="vacancy-item-cell"><a class="vacancy-item" href="https://www.werkenbijachmea.nl/vacatures/software-engineer-apeldoorn-a0wqs00000abc123"><h3>Software Engineer</h3></a></li></ul>"#;
const DETAIL: &str = r#"<script type="application/ld+json">{"@type":"JobPosting","datePosted":"2026-08-13T14:40:31Z","description":"<p>Build insurance software.</p>","employmentType":["FULL_TIME"],"hiringOrganization":{"name":"Achmea"},"jobLocation":{"address":{"addressLocality":"Apeldoorn","addressCountry":"Nederland"}},"title":"Software Engineer"}</script>"#;

#[test]
fn parses_complete_achmea_pages_and_details() {
    let jobs = parse_hamilton_pages(
        "achmea",
        "https://www.werkenbijachmea.nl/vacatures",
        ACHMEA_PROFILE,
        &[LISTING],
        &[DETAIL],
        10,
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "a0wqs00000abc123");
    assert_eq!(jobs[0].locations, ["Apeldoorn"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].apply_url,
        "https://www.werkenbijachmea.nl/vacatures/software-engineer-apeldoorn-a0wqs00000abc123/solliciteer"
    );
}

#[tokio::test]
#[ignore = "live external source"]
async fn achmea_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+Achmea live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = NsSource::with_profile(
        "achmea",
        "https://www.werkenbijachmea.nl/vacatures",
        ACHMEA_PROFILE,
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Achmea scan incomplete"),
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
    println!("Achmea: {} NL jobs", jobs.len());
}
