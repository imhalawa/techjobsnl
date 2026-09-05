use std::{collections::HashSet, time::Duration};

use techjobsnl::{
    domain::SourceScan,
    sources::{
        JobSource,
        postnl::{PostnlSource, parse_postnl_responses},
    },
};

const PAGE: &str = r#"{"overview":[{"id":60068,"referenceId":59609,"fancyUrl":"data-engineer-60068","isProfessional":true,"description":"Build data products.","jobTitle":"Data Engineer","city":"Den Haag"}],"paging":{"totalResult":1,"currentPage":1,"pages":1,"pageSize":10}}"#;
const DETAIL: &str = r#"{"publicationPeriod":{"startDate":"2026-08-14T10:36:42Z","endDate":null},"workLocation":{"city":"Den Haag"},"contractType":"Reguliere baan","discipline":"Data and analytics","contentFields":{"challenges":"<p>Build reliable data products.</p>","whatWeAsk":"<p>Strong SQL.</p>"},"levels":["Medior"],"id":60068,"referenceId":59609,"fancyUrl":"data-engineer-60068","isProfessional":true,"description":"Build data products.","jobTitle":"Data Engineer"}"#;

#[test]
fn parses_a_complete_postnl_page_and_detail() {
    let jobs = parse_postnl_responses(
        "postnl",
        "https://vacatures-website.postnl.nl/vacatures-widget/api/",
        &[PAGE],
        &[DETAIL],
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "60068");
    assert_eq!(jobs[0].title, "Data Engineer");
    assert_eq!(jobs[0].department.as_deref(), Some("Data and analytics"));
    assert_eq!(jobs[0].team.as_deref(), Some("Medior"));
    assert_eq!(jobs[0].locations, ["Den Haag"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].job_url,
        "https://www.postnl.nl/werkenbij/vacatures-voor-hbo-en-wo/data-engineer-60068/"
    );
    assert_eq!(
        jobs[0].apply_url,
        "https://www.postnl.nl/werkenbij/sollicitatie/?id=60068"
    );
}

#[tokio::test]
#[ignore = "live external source"]
async fn postnl_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+PostNL live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = PostnlSource::new(
        "postnl",
        "https://vacatures-website.postnl.nl/vacatures-widget/api/",
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("PostNL scan incomplete"),
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
    println!("PostNL: {} NL professional jobs", jobs.len());
}
