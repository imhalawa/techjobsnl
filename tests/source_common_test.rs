use std::{
    io::{Read, Write},
    net::TcpListener,
    thread,
    time::Duration,
};

use job_watch::{
    domain::SourceErrorKind,
    sources::{
        http::send_text,
        json_ld::{html_text, parse_job_posting},
    },
};

#[tokio::test]
async fn returns_successful_response_text() {
    let url = serve_once(200, &[], "official response");

    let body = send_text(reqwest::Client::new().get(url), "test source")
        .await
        .unwrap();

    assert_eq!(body, "official response");
}

#[tokio::test]
async fn classifies_retryable_http_statuses() {
    for (status, kind) in [
        (429, SourceErrorKind::RateLimit),
        (408, SourceErrorKind::Timeout),
        (500, SourceErrorKind::Transport),
        (503, SourceErrorKind::Transport),
    ] {
        let headers = if status == 429 {
            vec![("Retry-After", "9")]
        } else {
            vec![]
        };
        let url = serve_once(status, &headers, "unavailable");

        let source = if status == 429 {
            "Ashby"
        } else {
            "test source"
        };
        let error = send_text(reqwest::Client::new().get(url), source)
            .await
            .unwrap_err();

        assert_eq!(error.kind, kind, "status {status}");
        assert!(error.retryable, "status {status}");
        assert_eq!(error.http_status, Some(status), "status {status}");
        if status == 429 {
            assert_eq!(error.retry_after, Some(Duration::from_secs(9)));
            assert_eq!(error.message, "Ashby rate limit exceeded");
        }
    }
}

#[tokio::test]
async fn classifies_authentication_and_forbidden_statuses_as_non_retryable() {
    for status in [401, 403] {
        let url = serve_once(status, &[], "denied");

        let error = send_text(reqwest::Client::new().get(url), "test source")
            .await
            .unwrap_err();

        assert_eq!(error.kind, SourceErrorKind::Transport, "status {status}");
        assert!(!error.retryable, "status {status}");
        assert_eq!(error.http_status, Some(status), "status {status}");
    }
}

#[test]
fn extracts_job_posting_from_graph_and_normalises_description_html() {
    let posting = parse_job_posting(
        include_str!("fixtures/common/job-posting.html"),
        "fixture source",
    )
    .unwrap();

    assert_eq!(posting.identifier.unwrap().value, "REQ-42");
    assert_eq!(posting.title.as_deref(), Some("Platform Engineer"));
    assert_eq!(
        posting.job_location[0].address.address_country.as_deref(),
        Some("NL")
    );
    assert_eq!(
        html_text(&posting.description),
        "Build & ship reliable systems. Work with product."
    );
}

#[test]
fn reports_json_ld_parser_failures_as_non_retryable_schema_errors() {
    let error = parse_job_posting(
        r#"<script type="application/ld+json">{"@type":"JobPosting"</script>"#,
        "broken fixture",
    )
    .unwrap_err();

    assert_eq!(error.kind, SourceErrorKind::Schema);
    assert!(!error.retryable);
    assert_eq!(error.http_status, None);
    assert_eq!(error.retry_after, None);
}

#[test]
fn rejects_a_page_when_malformed_json_ld_precedes_a_valid_job_posting() {
    let html = format!(
        r#"<script type="application/ld+json">{{"@type":"BreadcrumbList"</script>{}"#,
        include_str!("fixtures/common/job-posting.html")
    );

    let error = parse_job_posting(&html, "mixed fixture").unwrap_err();

    assert_eq!(error.kind, SourceErrorKind::Schema);
    assert!(!error.retryable);
    assert!(error.message.contains("invalid JSON-LD"));
}

fn serve_once(status: u16, headers: &[(&str, &str)], body: &str) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let headers = headers
        .iter()
        .map(|(name, value)| format!("{name}: {value}\r\n"))
        .collect::<String>();
    let response = format!(
        "HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n{headers}\r\n{body}",
        body.len()
    );

    let _server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        let mut request = [0; 2048];
        let _ = stream.read(&mut request).unwrap();
        stream.write_all(response.as_bytes()).unwrap();
    });

    format!("http://{address}")
}
