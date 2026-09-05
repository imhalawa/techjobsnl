use techjobsnl::{
    domain::SourceScan,
    sources::{
        JobSource,
        buckaroo::{BuckarooSource, parse_buckaroo_pages},
    },
};

const LISTING_URL: &str = "https://www.buckaroo.nl/over-buckaroo/vacatures";

fn listing() -> &'static str {
    r#"<section>
      <h2>Onze vacatures</h2>
      <div class="card card--paymentplugin"><h3 class="card__title">
        <a class="link link--title" href="/over-buckaroo/vacatures/platform-engineer">Platform Engineer</a>
      </h3></div>
      <div class="card card--paymentplugin"><h3 class="card__title">
        <a class="link link--title" href="/over-buckaroo/vacatures/support-engineer">Support Engineer</a>
      </h3></div>
    </section>"#
}

fn sitemap() -> &'static str {
    r#"<?xml version="1.0" encoding="UTF-8"?>
    <urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">
      <url><loc>https://www.buckaroo.nl/over-buckaroo/vacatures</loc><lastmod>2026-08-01</lastmod></url>
      <url><loc>https://www.buckaroo.nl/over-buckaroo/vacatures/platform-engineer</loc><lastmod>2026-08-12</lastmod></url>
      <url><loc>https://www.buckaroo.nl/over-buckaroo/vacatures/support-engineer</loc><lastmod>2026-08-13</lastmod></url>
    </urlset>"#
}

fn detail(title: &str, location: &str, slug: &str) -> String {
    format!(
        r#"<html><head>
          <link rel="canonical" href="https://www.buckaroo.nl/over-buckaroo/vacatures/{slug}">
        </head><body><div class="band band--spacing"><div class="band__content">
          <div class="item flow"><div class="rte flow">
            <h1>{title}</h1><h4>32 - 40 uur | {location}</h4>
            <p>Build and maintain reliable payment systems. Work with product and operations to improve secure integrations for customers across the Netherlands.</p>
          </div></div>
        </div></div></body></html>"#
    )
}

#[test]
fn parses_only_a_complete_official_buckaroo_snapshot() {
    let first = detail("Platform Engineer", "Utrecht", "platform-engineer");
    let second = detail("Support Engineer", "Den Haag", "support-engineer");
    let jobs = parse_buckaroo_pages(
        "buckaroo",
        LISTING_URL,
        listing(),
        sitemap(),
        &[&first, &second],
    )
    .unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "platform-engineer");
    assert_eq!(jobs[0].title, "Platform Engineer");
    assert_eq!(jobs[0].locations, ["Utrecht"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert!(jobs[0].description.contains("reliable payment systems"));
    assert_eq!(jobs[0].job_url, jobs[0].apply_url);
    assert!(jobs[0].published_at.is_none());

    let incomplete_sitemap = sitemap().replace(
        "<url><loc>https://www.buckaroo.nl/over-buckaroo/vacatures/support-engineer</loc><lastmod>2026-08-13</lastmod></url>",
        "",
    );
    assert!(
        parse_buckaroo_pages(
            "buckaroo",
            LISTING_URL,
            listing(),
            &incomplete_sitemap,
            &[&first, &second],
        )
        .is_err()
    );
}

#[tokio::test]
#[ignore = "live external source"]
async fn buckaroo_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl-live-test/0.1")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();
    let scan = BuckarooSource::new("buckaroo", LISTING_URL, client)
        .scan()
        .await
        .unwrap();
    let SourceScan::Complete { observations } = scan else {
        panic!("Buckaroo must return a complete scan");
    };
    assert_eq!(observations.len(), 5);
    assert!(observations.iter().all(|job| job.countries == ["NL"]));
    assert!(observations.iter().all(|job| !job.description.is_empty()));
    let unique = observations
        .iter()
        .map(|job| job.source_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(unique.len(), observations.len());
}
