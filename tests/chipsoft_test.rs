use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        chipsoft::{ChipsoftSource, parse_chipsoft_page},
    },
};

const LISTING: &str = r#"<div id="results-container"><div class="vacancy-list"><article class="course-card"><h3 class="title">.NET Developer</h3><span class="detail location-detail">Amsterdam</span><a class="stretched-link" href="/nl-nl/werken-bij/vacatures/net-developer/">Bekijk vacature</a></article><article class="course-card"><h3 class="title">Consultant</h3><span class="detail location-detail">Antwerpen</span><a class="stretched-link" href="/nl-nl/werken-bij/vacatures/consultant-antwerpen/">Bekijk vacature</a></article></div></div>"#;
const DETAIL: &str = r#"<main><div id="vacancy-content"><h1 class="title">.NET Developer</h1><span class="detail time-detail">40 uur per week</span><span class="detail location-detail">Amsterdam</span><div class="pe-lg-4"><p>Build healthcare software.</p></div></div><a href="/nl-nl/werken-bij/solliciteren/?vacancyId=47">Solliciteer nu</a></main>"#;

#[test]
fn parses_the_complete_chipsoft_board_and_keeps_only_nl_jobs() {
    let jobs = parse_chipsoft_page(
        "chipsoft",
        "https://www.chipsoft.com/nl-NL/werken-bij/vacatures",
        LISTING,
        &[DETAIL],
    )
    .unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "47");
    assert_eq!(jobs[0].title, ".NET Developer");
    assert_eq!(jobs[0].locations, ["Amsterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert!(jobs[0].apply_url.ends_with("?vacancyId=47"));
}

#[tokio::test]
#[ignore = "live external source"]
async fn chipsoft_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+ChipSoft live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let source = ChipsoftSource::new(
        "chipsoft",
        "https://www.chipsoft.com/nl-NL/werken-bij/vacatures",
        client,
    );
    let jobs = match source.scan().await.unwrap() {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("ChipSoft scan incomplete"),
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
    println!("ChipSoft: {} NL jobs", jobs.len());
}
