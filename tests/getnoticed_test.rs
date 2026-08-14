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
        getnoticed::{GetnoticedSource, build_client, parse_getnoticed_pages},
        http::send_text,
    },
};
use serde_json::Value;

const BASE_URL: &str = "https://www.werkenbijabnamro.nl";

#[test]
fn getnoticed_builds_the_exact_abn_listing_request() {
    let source = GetnoticedSource::new(
        "abn-amro",
        BASE_URL,
        Some("Nederland".to_owned()),
        reqwest::Client::new(),
    );

    let request = source.listing_request(2).unwrap().build().unwrap();
    assert_eq!(request.method(), reqwest::Method::GET);
    assert_eq!(
        request.url().as_str(),
        "https://www.werkenbijabnamro.nl/api/vacancy/?pageNumber=2&sort=created&sortDir=DESC&filters%5BLand%5D%5B%5D=Nederland"
    );
    assert_eq!(request.headers()["X-Requested-With"], "XMLHttpRequest");
    assert!(!request.url().query().unwrap().contains("pageSize"));
}

#[test]
fn getnoticed_parses_exact_two_page_snapshot_and_full_details() {
    let jobs = parse_fixture(pages(), details()).unwrap();

    assert_eq!(
        jobs.iter()
            .map(|job| job.source_id.as_str())
            .collect::<Vec<_>>(),
        ["1001", "1002", "1003"]
    );
    assert_eq!(jobs[0].title, "Platform Engineer");
    assert_eq!(jobs[0].department.as_deref(), Some("IT Engineering"));
    assert_eq!(jobs[2].department.as_deref(), Some("Data / IT Engineering"));
    assert_eq!(jobs[0].team, None);
    assert_eq!(jobs[0].employment_type, None);
    assert_eq!(jobs[0].locations, ["Amsterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].job_url,
        "https://www.werkenbijabnamro.nl/en/vacancy/1001/platform-engineer"
    );
    assert_eq!(
        jobs[0].apply_url,
        "https://www.werkenbijabnamro.nl/vacature-solliciteren/1001"
    );
    assert_eq!(
        jobs[0].description,
        "Build reliable banking platforms.\n\n- Automate safely."
    );
    assert_eq!(
        jobs[0].published_at.unwrap().to_rfc3339(),
        "2026-08-10T07:30:00+00:00"
    );
    assert_eq!(jobs[0].raw_payload["listing"]["id"], 1001);
    assert_eq!(
        jobs[0].raw_payload["listing"]["option_values"][0]["value"],
        "Nederland"
    );
    assert_eq!(
        jobs[0].raw_payload["jobPosting"],
        first_job_posting(details()[0].as_str())
    );
    assert_eq!(
        jobs[0].raw_payload["jobPosting"]["identifier"]["value"],
        "ATS-77"
    );
    assert_eq!(
        jobs[1].raw_payload["jobPosting"]["identifier"]["value"],
        "ATS-77"
    );
}

#[test]
fn getnoticed_accepts_an_official_empty_first_page_as_complete() {
    let page = r#"{
        "meta": {
            "num_total_hits": 0,
            "pageNumber": 1,
            "maxPerPage": 8,
            "totalPageCount": 0
        },
        "vacancies": []
    }"#;

    let jobs = parse_getnoticed_pages("abn-amro", BASE_URL, &[page], &[]).unwrap();

    assert!(jobs.is_empty());
}

#[test]
fn getnoticed_rejects_listing_drift_gaps_counts_and_duplicate_ids() {
    assert_schema(parse_fixture(vec![pages().remove(0)], details()));

    for mutation in [
        "total",
        "page_number",
        "page_size",
        "page_count",
        "early_empty",
        "duplicate_id",
        "extra_row",
        "non_numeric_id",
    ] {
        let mut pages = pages();
        match mutation {
            "total" => set_json(&mut pages[1], "/meta/num_total_hits", Value::from(4)),
            "page_number" => set_json(&mut pages[1], "/meta/pageNumber", Value::from(3)),
            "page_size" => set_json(&mut pages[1], "/meta/maxPerPage", Value::from(3)),
            "page_count" => set_json(&mut pages[1], "/meta/totalPageCount", Value::from(3)),
            "early_empty" => set_json(&mut pages[0], "/vacancies", Value::Array(vec![])),
            "duplicate_id" => set_json(&mut pages[1], "/vacancies/0/id", Value::from(1001)),
            "extra_row" => {
                let mut page: Value = serde_json::from_str(&pages[1]).unwrap();
                page["vacancies"].as_array_mut().unwrap().push(
                    serde_json::json!({"id": 1004, "slug": "extra", "subtitle": {"option_values": [{"title": "IT"}]}}),
                );
                pages[1] = page.to_string();
            }
            "non_numeric_id" => set_json(&mut pages[0], "/vacancies/0/id", Value::from("1001")),
            _ => unreachable!(),
        }
        assert_schema(parse_fixture(pages, details()));
    }
}

#[test]
fn getnoticed_rejects_detail_identity_url_and_required_field_drift() {
    for mutation in [
        "missing_detail",
        "vacancy_id",
        "apply_id",
        "canonical_id",
        "canonical_host",
        "canonical_http",
        "title",
        "date",
        "description",
        "location",
        "country",
    ] {
        let mut details = details();
        match mutation {
            "missing_detail" => {
                details.pop();
            }
            "vacancy_id" => {
                details[0] =
                    details[0].replace("data-vacancy-id=\"1001\"", "data-vacancy-id=\"9999\"")
            }
            "apply_id" => {
                details[0] = details[0].replace(
                    "/en/solliciteren/1001/inline",
                    "/en/solliciteren/9999/inline",
                )
            }
            "canonical_id" => {
                details[0] =
                    details[0].replace("/1001/platform-engineer", "/9999/platform-engineer")
            }
            "canonical_host" => {
                details[1] = details[1].replace("www.werkenbijabnamro.nl", "example.com")
            }
            "canonical_http" => {
                details[1] = details[1].replace(
                    "https://www.werkenbijabnamro.nl",
                    "http://www.werkenbijabnamro.nl",
                )
            }
            "title" => mutate_job_posting(&mut details[0], |posting| posting["title"] = " ".into()),
            "date" => mutate_job_posting(&mut details[0], |posting| {
                posting["datePosted"] = "yesterday".into()
            }),
            "description" => mutate_job_posting(&mut details[0], |posting| {
                posting["description"] = "<p> </p>".into()
            }),
            "location" => mutate_job_posting(&mut details[0], |posting| {
                posting["jobLocation"] = serde_json::json!([])
            }),
            "country" => mutate_job_posting(&mut details[0], |posting| {
                posting["jobLocation"]["address"]["addressCountry"] = "Belgium".into()
            }),
            _ => unreachable!(),
        }
        assert_schema(parse_fixture(pages(), details));
    }
}

#[test]
fn getnoticed_rejects_a_wrong_primary_id_even_when_a_related_card_has_the_expected_id() {
    let mut details = details();
    details[0] = details[0]
        .replace(
            r#"<div data-component="Favorite" data-vacancy-id="1001"></div>"#,
            r#"<div data-component="Favorite" data-vacancy-id="9999"></div>"#,
        )
        .replace(
            r#"data-vacancy-id="7777" class="partial partial_vacancy_list-item""#,
            r#"data-vacancy-id="1001" class="partial partial_vacancy_list-item""#,
        );

    assert_schema(parse_fixture(pages(), details));
}

#[test]
fn getnoticed_rejects_non_official_or_non_https_bases() {
    for base in [
        "https://careers.example.com",
        "http://www.werkenbijabnamro.nl",
        "https://werkenbijabnamro.nl",
    ] {
        assert_schema(parse_getnoticed_pages(
            "abn-amro",
            base,
            &page_refs(&pages()),
            &detail_refs(&details()),
        ));
    }
}

#[tokio::test]
async fn getnoticed_client_rejects_cross_host_redirects_before_fetching_the_target() {
    let (source_url, target) = cross_host_redirect();
    let client = build_client("job-watch-test", Duration::from_secs(5)).unwrap();

    let error = send_text(client.get(source_url), "ABN AMRO")
        .await
        .unwrap_err();

    assert!(error.message.contains("redirect"));
    target.set_nonblocking(true).unwrap();
    assert!(target.accept().is_err(), "off-host target was requested");
}

#[tokio::test]
async fn getnoticed_client_rejects_same_host_http_redirects_before_fetching_the_target() {
    let (source_url, target) = same_host_http_redirect();
    let client = build_client("job-watch-test", Duration::from_secs(5)).unwrap();

    let error = send_text(client.get(source_url), "ABN AMRO")
        .await
        .unwrap_err();

    assert!(error.message.contains("redirect"));
    target.set_nonblocking(true).unwrap();
    assert!(
        target.accept().is_err(),
        "HTTP redirect target was requested"
    );
}

#[tokio::test]
async fn getnoticed_client_rejects_alternate_port_redirects_before_fetching_the_target() {
    let (source_url, target) = alternate_port_redirect();
    let client = build_client("job-watch-test", Duration::from_secs(5)).unwrap();

    let _ = send_text(client.get(source_url), "ABN AMRO").await;

    target.set_nonblocking(true).unwrap();
    assert!(
        target.accept().is_err(),
        "alternate-port redirect target was requested"
    );
}

#[tokio::test]
#[ignore = "live external source"]
async fn getnoticed_live_returns_complete_unique_abn_jobs() {
    let source = GetnoticedSource::new(
        "abn-amro",
        BASE_URL,
        Some("Nederland".to_owned()),
        build_client(
            "job-watch/0.1 (+ABN AMRO live test)",
            Duration::from_secs(30),
        )
        .unwrap(),
    );
    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("ABN AMRO scan must be complete");
    };

    assert!(!observations.is_empty());
    let mut ids = HashSet::new();
    for job in &observations {
        assert!(ids.insert(&job.source_id));
        assert!(
            job.source_id
                .chars()
                .all(|character| character.is_ascii_digit())
        );
        assert!(!job.title.trim().is_empty());
        assert!(!job.description.trim().is_empty());
        assert!(!job.locations.is_empty());
        assert_eq!(job.countries, ["NL"]);
        assert!(
            job.job_url
                .starts_with("https://www.werkenbijabnamro.nl/en/vacancy/")
        );
        assert_eq!(
            job.apply_url,
            format!(
                "https://www.werkenbijabnamro.nl/vacature-solliciteren/{}",
                job.source_id
            )
        );
        assert!(job.published_at.is_some());
        assert_eq!(job.employment_type, None);
        assert_eq!(job.raw_payload["listing"]["id"].to_string(), job.source_id);
    }
    println!("ABN AMRO: {} jobs", observations.len());
}

fn pages() -> Vec<String> {
    [
        include_str!("fixtures/getnoticed/abn-page-1.json"),
        include_str!("fixtures/getnoticed/abn-page-2.json"),
    ]
    .map(str::to_owned)
    .into()
}

fn details() -> Vec<String> {
    [
        include_str!("fixtures/getnoticed/abn-detail-1001.html"),
        include_str!("fixtures/getnoticed/abn-detail-1002.html"),
        include_str!("fixtures/getnoticed/abn-detail-1003.html"),
    ]
    .map(str::to_owned)
    .into()
}

fn parse_fixture(
    pages: Vec<String>,
    details: Vec<String>,
) -> Result<Vec<ObservedJob>, job_watch::sources::SourceError> {
    parse_getnoticed_pages(
        "abn-amro",
        BASE_URL,
        &page_refs(&pages),
        &detail_refs(&details),
    )
}

fn page_refs(pages: &[String]) -> Vec<&str> {
    pages.iter().map(String::as_str).collect()
}

fn detail_refs(details: &[String]) -> Vec<&str> {
    details.iter().map(String::as_str).collect()
}

fn assert_schema(result: Result<Vec<ObservedJob>, job_watch::sources::SourceError>) {
    let error = result.unwrap_err();
    assert_eq!(error.kind, SourceErrorKind::Schema);
    assert!(!error.retryable);
}

fn set_json(raw: &mut String, pointer: &str, value: Value) {
    let mut json: Value = serde_json::from_str(raw).unwrap();
    *json.pointer_mut(pointer).unwrap() = value;
    *raw = json.to_string();
}

fn first_job_posting(html: &str) -> Value {
    job_watch::sources::json_ld::job_posting_value(html, "fixture").unwrap()
}

fn mutate_job_posting(html: &mut String, mutation: impl FnOnce(&mut Value)) {
    let marker = "<script type=\"application/ld+json\">";
    let mut offset = 0;
    loop {
        let start = html[offset..].find(marker).unwrap() + offset + marker.len();
        let end = html[start..].find("</script>").unwrap() + start;
        let mut value: Value = serde_json::from_str(&html[start..end]).unwrap();
        if value.get("@type").and_then(Value::as_str) == Some("JobPosting") {
            mutation(&mut value);
            html.replace_range(start..end, &value.to_string());
            return;
        }
        offset = end + "</script>".len();
    }
}

fn cross_host_redirect() -> (String, TcpListener) {
    redirect(|port| format!("http://localhost:{port}/off-host"))
}

fn same_host_http_redirect() -> (String, TcpListener) {
    redirect(|port| format!("http://127.0.0.1:{port}/downgrade"))
}

fn alternate_port_redirect() -> (String, TcpListener) {
    redirect(|port| format!("https://127.0.0.1:{port}/alternate-port"))
}

fn redirect(location: impl FnOnce(u16) -> String + Send + 'static) -> (String, TcpListener) {
    let source = TcpListener::bind("127.0.0.1:0").unwrap();
    let source_address = source.local_addr().unwrap();
    let target = TcpListener::bind("127.0.0.1:0").unwrap();
    let target_port = target.local_addr().unwrap().port();
    thread::spawn(move || {
        let (mut stream, _) = source.accept().unwrap();
        let mut request = [0; 2048];
        let _ = stream.read(&mut request).unwrap();
        let response = format!(
            "HTTP/1.1 302 Found\r\nLocation: {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
            location(target_port)
        );
        stream.write_all(response.as_bytes()).unwrap();
    });
    (format!("http://{source_address}"), target)
}
