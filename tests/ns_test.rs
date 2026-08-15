use std::{collections::HashSet, time::Duration};

use techjobsnl::{
    domain::SourceScan,
    sources::{
        JobSource,
        ns::{NsSource, parse_ns_pages},
    },
};

const PAGE_1: &str = r#"<span class="vacancy-count">1 t/m 1 van 2</span><ul class="vacancy-list"><li class="vacancy-item-cell"><a class="vacancy-item" href="https://www.werkenbijns.nl/vacatures/platform-engineer-utrecht-111"><h3>Platform Engineer</h3></a></li></ul>"#;
const PAGE_2: &str = r#"<span class="vacancy-count">2 t/m 2 van 2</span><ul class="vacancy-list"><li class="vacancy-item-cell"><a class="vacancy-item" href="https://www.werkenbijns.nl/vacatures/data-engineer-amsterdam-222"><h3>Data Engineer</h3></a></li></ul>"#;
const DETAIL_1: &str = r#"<script type="application/ld+json">{"@type":"JobPosting","datePosted":"2026-08-13T14:40:31Z","description":"<p>Build reliable platforms.</p>","employmentType":["FULL_TIME"],"hiringOrganization":{"name":"NS"},"jobLocation":{"address":{"addressLocality":"Utrecht","addressCountry":"Nederland"}},"title":"Platform Engineer"}</script>"#;
const DETAIL_2: &str = r#"<script>const dataLayerObj = {"country":"Netherlands"};</script><script type="application/ld+json">{"@type":"JobPosting","datePosted":"2026-08-12T10:00:00Z","description":"<p>Build data products.</p>","hiringOrganization":{"name":"NS"},"jobLocation":null,"title":"Data Engineer"}</script>"#;

#[test]
fn parses_complete_ns_pages_and_details() {
    let jobs = parse_ns_pages(
        "ns",
        "https://www.werkenbijns.nl/vacatures",
        &[PAGE_1, PAGE_2],
        &[DETAIL_1, DETAIL_2],
        1,
    )
    .unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "111");
    assert_eq!(jobs[0].locations, ["Utrecht"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].apply_url,
        format!("{}/solliciteer", jobs[0].job_url)
    );
    assert_eq!(jobs[1].locations, ["Netherlands"]);
}

#[test]
fn rejects_incomplete_or_duplicate_ns_pages() {
    assert!(
        parse_ns_pages(
            "ns",
            "https://www.werkenbijns.nl/vacatures",
            &[PAGE_1],
            &[DETAIL_1],
            1,
        )
        .is_err()
    );
    assert!(
        parse_ns_pages(
            "ns",
            "https://www.werkenbijns.nl/vacatures",
            &[PAGE_1, PAGE_1],
            &[DETAIL_1, DETAIL_1],
            1,
        )
        .is_err()
    );
}

#[tokio::test]
#[ignore = "live external source"]
async fn ns_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+NS live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = NsSource::new("ns", "https://www.werkenbijns.nl/vacatures", client);
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("NS scan incomplete"),
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
    println!("NS: {} NL jobs", jobs.len());
}
