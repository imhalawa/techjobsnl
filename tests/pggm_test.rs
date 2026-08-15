use std::{collections::HashSet, time::Duration};

use techjobsnl::{
    domain::SourceScan,
    sources::{
        JobSource,
        pggm::{PggmSource, parse_pggm_pages},
    },
};

const LISTING_URL: &str = "https://www.werkenbijpggm.nl/vacatures";

fn listing(page: usize, last: usize, jobs: &[(&str, &str)]) -> String {
    let cards = jobs
        .iter()
        .map(|(id, title)| {
            format!(
                r#"<a class="c-card" href="/vacatures/vacature/{id}/?id={id}"><h3>{title}</h3></a>"#
            )
        })
        .collect::<String>();
    let pages = (1..=last)
        .map(|number| {
            if number == page {
                format!(r#"<li class="pagination__item pagination__item--active">{number}</li>"#)
            } else {
                format!(r#"<li class="pagination__item"><a href="?p={number}">{number}</a></li>"#)
            }
        })
        .collect::<String>();
    format!(
        r#"<div id="async-container-vacancies">{cards}<ol class="pagination__list">{pages}</ol></div>"#
    )
}

fn detail(id: &str, title: &str) -> String {
    format!(
        r#"<h1 class="hero__title">{title}</h1>
        <div class="c-specification"><div class="specification__item">
          <span class="specification__title">Vakgebied</span>
          <span class="specification__value">IT</span>
        </div></div>
        <section class="c-paragraph"><div class="s-rich-text"><p>Build reliable pension software in Zeist.</p></div></section>
        <a href="/vacatures/solliciteren/{id}/?id={id}"><span>Solliciteer nu</span></a>"#
    )
}

#[test]
fn parses_a_complete_pggm_board() {
    let first = listing(
        1,
        2,
        &[("a0wOne", ".NET Engineer"), ("a0wTwo", "Cloud Engineer")],
    );
    let second = listing(2, 2, &[("a0wThree", "Data Engineer")]);
    let sentinel = listing(3, 2, &[]);
    let details = [
        detail("a0wOne", ".NET Engineer"),
        detail("a0wTwo", "Cloud Engineer"),
        detail("a0wThree", "Data Engineer"),
    ];
    let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();
    let jobs = parse_pggm_pages(
        "pggm",
        LISTING_URL,
        &[&first, &second, &sentinel],
        &detail_refs,
        2,
    )
    .unwrap();

    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].source_id, "a0wOne");
    assert_eq!(jobs[0].title, ".NET Engineer");
    assert_eq!(jobs[0].department.as_deref(), Some("IT"));
    assert_eq!(jobs[0].locations, ["Netherlands"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert!(jobs[0].description.contains("pension software"));
    assert_eq!(
        jobs[0].apply_url,
        "https://www.werkenbijpggm.nl/vacatures/solliciteren/a0wOne/?id=a0wOne"
    );
    assert_eq!(jobs[0].published_at, None);
}

#[test]
fn rejects_missing_pages_duplicates_and_non_official_urls() {
    let first = listing(
        1,
        2,
        &[("a0wOne", ".NET Engineer"), ("a0wTwo", "Cloud Engineer")],
    );
    let second = listing(2, 2, &[("a0wOne", ".NET Engineer")]);
    let sentinel = listing(3, 2, &[]);
    let details = [
        detail("a0wOne", ".NET Engineer"),
        detail("a0wOne", ".NET Engineer"),
    ];
    let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();

    assert!(parse_pggm_pages("pggm", LISTING_URL, &[&first], &[], 2).is_err());
    assert!(
        parse_pggm_pages(
            "pggm",
            LISTING_URL,
            &[&first, &second, &sentinel],
            &detail_refs,
            2,
        )
        .is_err()
    );
    assert!(
        parse_pggm_pages(
            "pggm",
            "https://example.test/vacatures",
            &[&first, &second, &sentinel],
            &detail_refs,
            2,
        )
        .is_err()
    );
}

#[tokio::test]
#[ignore = "live external source"]
async fn pggm_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl/0.1 (+PGGM live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let jobs = match PggmSource::new("pggm", LISTING_URL, client)
        .scan()
        .await
        .unwrap()
    {
        SourceScan::Complete { observations } => observations,
        SourceScan::Incomplete { .. } => panic!("PGGM scan incomplete"),
    };

    assert!(!jobs.is_empty());
    assert!(jobs.iter().all(|job| job.countries == ["NL"]));
    assert!(jobs.iter().all(|job| job.locations == ["Netherlands"]));
    assert!(jobs.iter().all(|job| !job.description.is_empty()));
    assert_eq!(
        jobs.iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        jobs.len()
    );
    println!("PGGM: {} NL jobs", jobs.len());
}
