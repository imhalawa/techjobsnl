use std::{
    collections::HashSet,
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

use job_watch::{
    domain::{ObservedJob, SourceErrorKind, SourceScan},
    sources::{
        JobSource,
        ebay::{EbaySource, build_client as build_ebay_client, parse_ebay_pages},
        http::send_text,
    },
};

const LISTING_URL: &str = "https://jobs.ebayinc.com/us/en/jobs-in-netherlands";

#[test]
fn parses_exact_listing_total_and_full_official_details() {
    let jobs = parse_fixtures().unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "R100");
    assert_eq!(jobs[0].title, "Platform Engineer");
    assert_eq!(jobs[0].department.as_deref(), Some("Engineering"));
    assert_eq!(jobs[0].employment_type.as_deref(), Some("FULL_TIME"));
    assert_eq!(jobs[0].locations, ["Amsterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].job_url,
        "https://jobs.ebayinc.com/us/en/job/R100/Platform-Engineer"
    );
    assert_eq!(jobs[0].apply_url, "https://apply.example.test/R100");
    assert_eq!(
        jobs[0].description,
        "Build reliable marketplace services. Work with product teams."
    );
    assert_eq!(
        jobs[0].published_at.unwrap().to_rfc3339(),
        "2026-08-10T00:00:00+00:00"
    );
    assert_eq!(jobs[0].raw_payload["reqId"], "R100");
    assert_eq!(
        jobs[0].raw_payload["jobPosting"]["identifier"]["value"],
        "R100"
    );
    assert!(jobs[0].raw_payload.get("detailHtml").is_none());
    assert_eq!(jobs[1].source_id, "R200");
}

#[test]
fn accepts_an_initial_complete_empty_listing() {
    let empty = mutate_listing(&listing_fixtures()[0], |ddo| {
        ddo["eagerLoadRefineSearch"]["totalHits"] = 0.into();
        ddo["eagerLoadRefineSearch"]["data"]["jobs"] = serde_json::json!([]);
    });

    let jobs = parse_ebay_pages("ebay", LISTING_URL, &[&empty], &[]).unwrap();

    assert!(jobs.is_empty());
}

#[test]
fn rejects_total_drift_early_empty_pages_duplicates_and_listing_id_mismatch() {
    let [first, second] = listing_fixtures();

    let mut drift = mutate_listing(&second, |ddo| {
        ddo["eagerLoadRefineSearch"]["totalHits"] = 3.into();
    });
    assert_listing_error(&[&first, &drift]);

    drift = mutate_listing(&second, |ddo| {
        ddo["eagerLoadRefineSearch"]["data"]["jobs"] = serde_json::json!([]);
    });
    assert_listing_error(&[&first, &drift]);

    let first_job = listing_json(&first)["eagerLoadRefineSearch"]["data"]["jobs"][0].clone();
    drift = mutate_listing(&second, |ddo| {
        ddo["eagerLoadRefineSearch"]["data"]["jobs"][0] = first_job;
    });
    assert_listing_error(&[&first, &drift]);

    drift = mutate_listing(&first, |ddo| {
        ddo["eagerLoadRefineSearch"]["data"]["jobs"][0]["jobId"] = "R999".into();
    });
    assert_listing_error(&[&drift, &second]);
}

#[test]
fn rejects_detail_id_mismatch_missing_json_ld_location_and_description() {
    let [first, second] = listing_fixtures();
    let [detail_1, detail_2] = detail_fixtures();

    for broken in [
        detail_1.replace(r#""value":"R100""#, r#""value":"R999""#),
        "<html><head></head></html>".to_owned(),
        detail_1.replace(
            r#""jobLocation":{"@type":"Place","address":{"@type":"PostalAddress","addressLocality":"Amsterdam","addressRegion":"North Holland","addressCountry":"Netherlands"}}"#,
            r#""jobLocation":[]"#,
        ),
        detail_1.replace(
            r#""description":"&lt;p&gt;Build reliable marketplace services.&lt;/p&gt;&lt;p&gt;Work with product teams.&lt;/p&gt;""#,
            r#""description":"&lt;p&gt; &lt;/p&gt;""#,
        ),
    ] {
        assert_schema_error(parse_ebay_pages(
            "ebay",
            LISTING_URL,
            &[&first, &second],
            &[&broken, &detail_2],
        ));
    }
}

#[test]
fn rejects_a_detail_without_a_netherlands_location() {
    let [first, second] = listing_fixtures();
    let [detail_1, detail_2] = detail_fixtures();
    let us_only = detail_1
        .replace("Amsterdam", "Portland")
        .replace("North Holland", "Oregon")
        .replace("Netherlands", "United States");

    assert_schema_error(parse_ebay_pages(
        "ebay",
        LISTING_URL,
        &[&first, &second],
        &[&us_only, &detail_2],
    ));
}

#[test]
fn rejects_a_listing_url_outside_the_official_ebay_host() {
    let listings = listing_fixtures();
    let details = detail_fixtures();

    assert_schema_error(parse_ebay_pages(
        "ebay",
        "https://careers.example.test/us/en/jobs-in-netherlands",
        &[&listings[0], &listings[1]],
        &[&details[0], &details[1]],
    ));
}

#[tokio::test]
async fn ebay_http_client_rejects_cross_host_redirects_before_fetching_the_target() {
    let (source_url, target) = cross_host_redirect();
    let client = build_ebay_client("job-watch-test", Duration::from_secs(5)).unwrap();

    let error = send_text(client.get(source_url), "eBay").await.unwrap_err();

    assert!(error.message.contains("redirect"));
    target.set_nonblocking(true).unwrap();
    assert!(target.accept().is_err(), "off-host target was requested");
}

#[tokio::test]
#[ignore = "live external source"]
async fn ebay_live_returns_complete_unique_netherlands_jobs() {
    let client =
        build_ebay_client("job-watch/0.1 (+eBay live test)", Duration::from_secs(20)).unwrap();
    let source = EbaySource::new("ebay", LISTING_URL, client);
    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("eBay scan must be complete");
    };

    assert!(!observations.is_empty());
    let mut ids = HashSet::new();
    for job in &observations {
        assert_live_job(job);
        assert!(ids.insert(&job.source_id));
    }
    println!("eBay: {} Netherlands jobs", observations.len());
}

fn parse_fixtures() -> Result<Vec<ObservedJob>, job_watch::sources::SourceError> {
    let listings = listing_fixtures();
    let details = detail_fixtures();
    parse_ebay_pages(
        "ebay",
        LISTING_URL,
        &[&listings[0], &listings[1]],
        &[&details[0], &details[1]],
    )
}

fn listing_fixtures() -> [String; 2] {
    [
        include_str!("fixtures/ebay/list-page-1.html").to_owned(),
        include_str!("fixtures/ebay/list-page-2.html").to_owned(),
    ]
}

fn detail_fixtures() -> [String; 2] {
    [
        include_str!("fixtures/ebay/detail-r100.html").to_owned(),
        include_str!("fixtures/ebay/detail-r200.html").to_owned(),
    ]
}

fn listing_json(raw: &str) -> serde_json::Value {
    const START: &str = "phApp.ddo = ";
    const END: &str = "; phApp.experimentData";
    let start = raw.find(START).unwrap() + START.len();
    let end = start + raw[start..].find(END).unwrap();
    serde_json::from_str(&raw[start..end]).unwrap()
}

fn mutate_listing(raw: &str, mutation: impl FnOnce(&mut serde_json::Value)) -> String {
    let mut ddo = listing_json(raw);
    mutation(&mut ddo);
    format!(
        "<script>phApp.ddo = {}; phApp.experimentData = {{}};</script>",
        ddo
    )
}

fn assert_listing_error(listings: &[&str]) {
    let details = detail_fixtures();
    assert_schema_error(parse_ebay_pages(
        "ebay",
        LISTING_URL,
        listings,
        &[&details[0], &details[1]],
    ));
}

fn assert_schema_error(result: Result<Vec<ObservedJob>, job_watch::sources::SourceError>) {
    let error = result.unwrap_err();
    assert_eq!(error.kind, SourceErrorKind::Schema);
    assert!(!error.retryable);
}

fn assert_live_job(job: &ObservedJob) {
    assert!(!job.source_id.trim().is_empty());
    assert!(!job.title.trim().is_empty());
    assert!(!job.locations.is_empty());
    assert_eq!(job.countries, ["NL"]);
    assert!(
        job.job_url
            .starts_with("https://jobs.ebayinc.com/us/en/job/")
    );
    assert!(!job.apply_url.trim().is_empty());
    assert!(!job.description.trim().is_empty());
    assert!(job.raw_payload.is_object());
    assert!(job.published_at.is_some());
}

fn cross_host_redirect() -> (String, TcpListener) {
    let source = TcpListener::bind("127.0.0.1:0").unwrap();
    let source_address = source.local_addr().unwrap();
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    let target_port = target.local_addr().unwrap().port();
    thread::spawn(move || {
        let (mut stream, _) = source.accept().unwrap();
        let mut request = [0; 2048];
        let _ = stream.read(&mut request).unwrap();
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: http://localhost:{target_port}/off-host\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    (format!("http://{source_address}"), target)
}
