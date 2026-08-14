use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        afas::{AfasSource, parse_afas_page},
    },
};

const LISTING: &str = r#"<script>{"rows":[{"link":"\/job\/software-engineer"}],"meta":{"state":{"firstPage":true,"lastPage":true,"pageSize":75}}}</script>"#;
const DETAIL: &str = r#"<script type="application/ld+json">//<![CDATA[
{"@type":"JobPosting","datePosted":"2026-08-01","description":"Build reliable software.","hiringOrganization":{"name":"AFAS Software B.V."},"jobLocation":{"address":{"addressLocality":"Leusden","addressCountry":"NL"}},"title":"Software engineer"}
//]]></script>"#;
const BELGIAN_DETAIL: &str = r#"<script type="application/ld+json">//<![CDATA[
{"@type":"JobPosting","datePosted":"2026-07-01","description":null,"hiringOrganization":{"name":"AFAS Software B.V."},"jobLocation":{"address":{"addressLocality":"Kontich","addressCountry":"B"}},"title":"First Class Specialist"}
//]]></script>"#;

#[test]
fn parses_a_complete_afas_board_and_detail() {
    let jobs = parse_afas_page(
        "afas",
        "https://www.werkenbijafas.nl/alle-vacatures",
        LISTING,
        &[DETAIL],
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "software-engineer");
    assert_eq!(jobs[0].locations, ["Leusden"]);
    assert_eq!(jobs[0].countries, ["NL"]);
}

#[test]
fn skips_non_netherlands_jobs_before_validating_the_nl_job_schema() {
    let listing = LISTING.replace("]", r#",{"link":"\/job\/first-class-specialist"}]"#);
    let jobs = parse_afas_page(
        "afas",
        "https://www.werkenbijafas.nl/alle-vacatures",
        &listing,
        &[DETAIL, BELGIAN_DETAIL],
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "software-engineer");
}

#[test]
fn uses_the_visible_job_body_when_afas_omits_the_json_ld_description() {
    let detail = BELGIAN_DETAIL
        .replace("Kontich", "Leusden")
        .replace("\"B\"", "\"NL\"")
        .replace(
            "</script>",
            "</script><main id=\"P_mastercontent\"><div class=\"freehtml\"><p>Serve strategic customers.</p></div></main>",
        );
    let jobs = parse_afas_page(
        "afas",
        "https://www.werkenbijafas.nl/alle-vacatures",
        LISTING,
        &[&detail],
    )
    .unwrap();

    assert_eq!(jobs[0].description, "Serve strategic customers.");
}

#[tokio::test]
#[ignore = "live external source"]
async fn afas_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+AFAS live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = AfasSource::new(
        "afas",
        "https://www.werkenbijafas.nl/alle-vacatures",
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("AFAS scan incomplete"),
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
    println!("AFAS: {} NL jobs", jobs.len());
}
