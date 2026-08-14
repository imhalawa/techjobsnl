use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        successfactors_api::{SuccessFactorsSource, parse_successfactors_pages},
    },
};

const SEARCH: &str = r#"{
  "totalJobs": 1,
  "jobSearchResult": [{"response": {
    "id": "304539",
    "unifiedStandardTitle": "Senior Business Engineer (F/M)",
    "unifiedUrlTitle": "Senior-Business-Engineer-%28FM%29",
    "unifiedStandardStart": "7/1/26",
    "jobLocationShort": ["Netherlands-Utrecht "],
    "jobContract": ["Permanent"],
    "custJobArea": ["Technology"],
    "custJobFamily": ["Design & Engineering"],
    "sfstd_marketingBrand_obj": ["Worldline"]
  }}]
}"#;

const DETAIL: &str = r#"
<html><head>
  <link rel="canonical" href="https://jobs.worldline.com/job/Senior-Business-Engineer-%28FM%29/304539-en_US/">
</head><body>
  <span itemprop="title">Senior Business Engineer (F/M)</span>
  <span itemprop="description"><p>Design reliable payment systems.</p></span>
  <span itemprop="description"><p>Work with an international engineering team.</p></span>
  <a class="unify-apply-now" href="/talentcommunity/apply/304539/?locale=en_US">Apply now</a>
  <script>var jobID = 304539;</script>
</body></html>
"#;

#[test]
fn parses_every_declared_successfactors_job_and_matches_its_detail() {
    let jobs = parse_successfactors_pages(
        "worldline",
        "Worldline",
        "https://jobs.worldline.com",
        &[SEARCH],
        &[DETAIL],
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "304539");
    assert_eq!(jobs[0].title, "Senior Business Engineer (F/M)");
    assert_eq!(jobs[0].department.as_deref(), Some("Technology"));
    assert_eq!(jobs[0].team.as_deref(), Some("Design & Engineering"));
    assert_eq!(jobs[0].employment_type.as_deref(), Some("Permanent"));
    assert_eq!(jobs[0].locations, ["Utrecht"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].job_url,
        "https://jobs.worldline.com/job/Senior-Business-Engineer-%28FM%29/304539-en_US/"
    );
    assert_eq!(
        jobs[0].apply_url,
        "https://jobs.worldline.com/talentcommunity/apply/304539/?locale=en_US"
    );
    assert!(
        jobs[0]
            .description
            .contains("Design reliable payment systems.")
    );
    assert!(
        jobs[0]
            .description
            .contains("international engineering team")
    );
    assert!(jobs[0].published_at.is_some());
}

#[test]
fn rejects_incomplete_successfactors_pagination() {
    let search = SEARCH.replace("\"totalJobs\": 1", "\"totalJobs\": 26");
    let error = parse_successfactors_pages(
        "worldline",
        "Worldline",
        "https://jobs.worldline.com",
        &[&search],
        &[DETAIL],
    )
    .unwrap_err();

    assert!(error.message.contains("pagination"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn worldline_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+Worldline live test)")
        .cookie_store(true)
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = SuccessFactorsSource::new(
        "worldline",
        "Worldline",
        "https://jobs.worldline.com",
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("Worldline scan incomplete"),
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
    println!("Worldline: {} NL jobs", jobs.len());
}
