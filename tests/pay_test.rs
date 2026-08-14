use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        pay::{PaySource, parse_pay_pages},
    },
};

const LISTING_URL: &str = "https://www.pay.nl/werk";
const LISTING: &str = r#"
<section class="section-vacancyoverview" id="vacatures">
  <ul class="list-vacancy">
    <li><div><h5>Software engineer</h5></div><div><p>Development</p></div><div><p>Enschede / Spijkenisse / hybride</p></div><div><a href="https://www.pay.nl/werk/software-engineer"><span class="btn-content">Bekijk vacature</span></a></div></li>
    <li><div><h5>Closed role</h5></div><div><p>Product</p></div><div><p>Enschede</p></div><div><a href="https://www.pay.nl/werk/closed"><span class="btn-content">Reeds vervuld</span></a></div></li>
  </ul>
</section>"#;
const DETAIL: &str = r#"
<div class="intro-vacancy"><div class="intro__title"><h2>Software Engineer (IT)</h2></div><a href="https://www.nmbrshire.com/spa/nl/public/apply?guidAssignment=0a87f51d-af7a-4528-8a00-b7302abf7390&amp;forcelocale=true">Nu solliciteren</a><div class="intro__body"><p class="labeltag"><span>Spijkenisse / Enschede, hybride</span></p></div></div>
<section class="text-module"><div class="richtext"><p>Build reliable payment software.</p></div></section>
<section class="section-vacancy-content"><div class="section__body"><p>Own the service from design through production.</p></div></section>"#;

#[test]
fn parses_every_active_pay_vacancy_and_skips_explicitly_filled_roles() {
    let jobs = parse_pay_pages("pay", LISTING_URL, LISTING, &[DETAIL]).unwrap();

    assert_eq!(jobs.len(), 1);
    assert_eq!(jobs[0].source_id, "0a87f51d-af7a-4528-8a00-b7302abf7390");
    assert_eq!(jobs[0].title, "Software Engineer (IT)");
    assert_eq!(jobs[0].department.as_deref(), Some("Development"));
    assert_eq!(jobs[0].locations, ["Enschede / Spijkenisse / hybride"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert!(
        jobs[0]
            .description
            .contains("Build reliable payment software.")
    );
    assert!(jobs[0].description.contains("Own the service"));
    assert!(jobs[0].apply_url.contains("guidAssignment="));
    assert_eq!(jobs[0].published_at, None);
}

#[test]
fn rejects_incomplete_or_untrusted_pay_pages() {
    assert!(parse_pay_pages("pay", LISTING_URL, LISTING, &[]).is_err());
    assert!(parse_pay_pages("pay", "https://example.test/werk", LISTING, &[DETAIL]).is_err());
}

#[tokio::test]
#[ignore = "live external source"]
async fn pay_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("job-watch/0.1 (+PAY. live test)")
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap();
    let scan = PaySource::new("pay", LISTING_URL, client)
        .scan()
        .await
        .unwrap();
    let SourceScan::Complete { observations } = scan else {
        panic!("PAY. scan must be complete");
    };

    assert!(!observations.is_empty());
    assert!(observations.iter().all(|job| job.countries == ["NL"]));
    assert!(observations.iter().all(|job| !job.description.is_empty()));
    assert!(observations.iter().all(|job| job.published_at.is_none()));
    assert_eq!(
        observations
            .iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        observations.len()
    );
    println!("PAY.: {} NL jobs", observations.len());
}
