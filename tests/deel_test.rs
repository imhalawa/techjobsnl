use std::time::Duration;

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        deel::{DeelSource, parse_deel_board},
    },
};

const LISTING: &str = r#"
<script type="application/ld+json">
{"@context":"https://schema.org","@type":"ItemList","itemListElement":[
  {"@type":"ListItem","position":1,"url":"https://jobs.deel.com/klarna/job-details/11111111-1111-4111-8111-111111111111/overview"},
  {"@type":"ListItem","position":2,"url":"https://jobs.deel.com/klarna/job-details/22222222-2222-4222-8222-222222222222/overview"}
]}
</script>
"#;

const AMSTERDAM: &str = r#"
<script type="application/ld+json">
{"@context":"https://schema.org/","@type":"JobPosting","title":"Software Engineer","description":"Build payment systems.","datePosted":"2026-08-12T09:00:00Z","employmentType":["FULL_TIME"],"hiringOrganization":{"@type":"Organization","name":"Klarna"},"jobLocation":[{"@type":"Place","address":{"@type":"PostalAddress","addressLocality":"Amsterdam"}}],"identifier":{"@type":"PropertyValue","name":"Deel ATS Job Posting ID","value":"11111111-1111-4111-8111-111111111111"},"directApply":true,"url":"https://jobs.deel.com/klarna/job-details/11111111-1111-4111-8111-111111111111/overview"}
</script>
"#;

const STOCKHOLM: &str = r#"
<script type="application/ld+json">
{"@context":"https://schema.org/","@type":"JobPosting","title":"Risk Analyst","description":"Manage risk.","datePosted":"2026-08-11T09:00:00Z","employmentType":"FULL_TIME","hiringOrganization":{"@type":"Organization","name":"Klarna"},"jobLocation":{"@type":"Place","address":{"@type":"PostalAddress","addressLocality":"Stockholm"}},"identifier":{"@type":"PropertyValue","name":"Deel ATS Job Posting ID","value":"22222222-2222-4222-8222-222222222222"},"directApply":true,"url":"https://jobs.deel.com/klarna/job-details/22222222-2222-4222-8222-222222222222/overview"}
</script>
"#;

#[test]
fn complete_deel_board_keeps_only_netherlands_jobs() {
    let jobs = parse_deel_board(
        "klarna",
        "https://jobs.deel.com/klarna",
        LISTING,
        &[AMSTERDAM, STOCKHOLM],
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "11111111-1111-4111-8111-111111111111");
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].locations, ["Amsterdam"]);
    assert_eq!(jobs[0].employment_type.as_deref(), Some("FULL_TIME"));
    assert_eq!(jobs[0].department.as_deref(), Some("Klarna"));
    assert_eq!(jobs[0].apply_url, jobs[0].job_url);
}

#[test]
fn deel_board_rejects_missing_details() {
    let error = parse_deel_board(
        "klarna",
        "https://jobs.deel.com/klarna",
        LISTING,
        &[AMSTERDAM],
    )
    .unwrap_err();

    assert!(error.message.contains("listing/detail count mismatch"));
}

#[tokio::test]
#[ignore = "live official source"]
async fn klarna_live_board_is_complete_and_has_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("job-watch/0.1 (+Deel live test)")
        .timeout(Duration::from_secs(60))
        .build()
        .unwrap();
    let source = DeelSource::new("klarna", "https://jobs.deel.com/klarna", client);

    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("Deel scans must be complete");
    };
    assert!(!observations.is_empty());
    assert!(observations.iter().all(|job| job.countries == ["NL"]));
}
