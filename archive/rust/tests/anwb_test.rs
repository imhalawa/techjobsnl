use std::{collections::HashSet, time::Duration};

use techjobsnl::{
    domain::SourceScan,
    sources::{
        JobSource,
        anwb::{AnwbSource, parse_anwb_feed},
    },
};

const FEED: &str = r#"[{"id":2228,"title":"Back-end Engineer","url":"/vacature/2228/back-end-engineer","description_plain":"{\"introInformation\":\"Build energy services.\"}","synonyms_plain":"backend|developer"}]"#;
const DETAIL: &str = r##"<script type="application/ld+json">{"@type":"JobPosting","datePosted":"2026-07-20T08:11:25+02:00","description":"<p>Build reliable energy services.</p>","hiringOrganization":{"name":"ANWB Energie"},"jobLocation":{"address":{"addressLocality":"Den Haag ","addressCountry":"Nederland"}},"title":"Back-end Engineer","identifier":{"value":"10020"}}</script><a href="#vacancy-application-form">Solliciteer direct</a><div id="vacancy-application-form"></div>"##;

#[test]
fn parses_a_complete_anwb_feed_and_detail() {
    let jobs = parse_anwb_feed(
        "anwb",
        "https://www.werkenbijanwb.nl/fuse/vacancies.json",
        FEED,
        &[DETAIL],
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "2228");
    assert_eq!(jobs[0].title, "Back-end Engineer");
    assert_eq!(jobs[0].locations, ["Den Haag"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert!(jobs[0].apply_url.ends_with("#vacancy-application-form"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn anwb_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+ANWB live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = AnwbSource::new(
        "anwb",
        "https://www.werkenbijanwb.nl/fuse/vacancies.json",
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("ANWB scan incomplete"),
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
    println!("ANWB: {} NL jobs", jobs.len());
}
