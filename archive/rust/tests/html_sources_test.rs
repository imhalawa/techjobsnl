use std::{collections::HashSet, time::Duration};

use techjobsnl::{
    domain::{ObservedJob, SourceErrorKind, SourceScan},
    sources::{
        JobSource,
        ashby::build_client,
        ing::{IngSource, parse_ing_pages},
    },
};

const LISTING_URL: &str =
    "https://careers.ing.com/en/location/netherlands-jobs/2618/2750405/2/en/search-jobs";

#[test]
fn ing_parses_complete_listings_and_details() {
    let jobs = parse_fixture(ing_listings(), ing_details()).unwrap();

    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].source_id, "REQ-111");
    assert_eq!(jobs[0].title, "Platform Engineer");
    assert_eq!(jobs[0].employment_type.as_deref(), Some("Full time"));
    assert_eq!(jobs[0].locations, ["Amsterdam", "Brussels office"]);
    assert_eq!(jobs[0].countries, ["NL", "BE"]);
    assert_eq!(
        jobs[0].job_url,
        "https://careers.ing.com/en/job/amsterdam/platform-engineer/3121/111"
    );
    assert_eq!(
        jobs[0].apply_url,
        "https://ing.wd3.myworkdayjobs.com/job/Platform-Engineer_REQ-111/apply"
    );
    assert_eq!(
        jobs[0].description,
        "Build reliable platforms.\n\n- Automate safely."
    );
    assert_eq!(
        jobs[0].published_at.unwrap().to_rfc3339(),
        "2026-08-10T00:00:00+00:00"
    );
    assert_eq!(
        jobs[0].raw_payload["listing"],
        serde_json::json!({
            "data-job-id": "111",
            "href": "/en/job/amsterdam/platform-engineer/3121/111"
        })
    );
    let exact_posting = json_ld_value(include_str!("fixtures/ing/detail-111.html"));
    assert_eq!(jobs[0].raw_payload["jobPosting"], exact_posting);
    assert_eq!(
        jobs[0].raw_payload["applyUrl"],
        "https://ing.wd3.myworkdayjobs.com/job/Platform-Engineer_REQ-111/apply"
    );
}

#[test]
fn ing_rejects_incomplete_or_drifting_listings() {
    assert_schema_error(parse_fixture(vec![ing_listings().remove(0)], ing_details()));

    for mutation in [
        "total",
        "pages",
        "page_size",
        "current_page",
        "short",
        "duplicate_card",
        "card_url",
        "missing_metadata",
    ] {
        let mut listings = ing_listings();
        match mutation {
            "total" => set_listing_attr(&mut listings[1], "data-total-job-results", "4"),
            "pages" => set_listing_attr(&mut listings[1], "data-total-pages", "3"),
            "page_size" => set_listing_attr(&mut listings[1], "data-records-per-page", "3"),
            "current_page" => set_listing_attr(&mut listings[1], "data-current-page", "1"),
            "short" => {
                listings[0] = listings[0].replacen(
                    r#"<li class="search-results-item">"#,
                    r#"<li class="removed">"#,
                    1,
                )
            }
            "duplicate_card" => {
                listings[1] = listings[1]
                    .replace(r#"data-job-id="333""#, r#"data-job-id="111""#)
                    .replace("/333\"", "/111\"")
            }
            "card_url" => listings[0] = listings[0].replacen("/3121/111", "/3121/999", 1),
            "missing_metadata" => listings[0] = listings[0].replace(r#" data-total-pages="2""#, ""),
            _ => unreachable!(),
        }
        assert_schema_error(parse_fixture(listings, ing_details()));
    }
}

#[test]
fn ing_rejects_invalid_detail_identity_and_required_fields() {
    for mutation in [
        "blank_identifier",
        "duplicate_identifier",
        "canonical_url",
        "http_detail_url",
        "missing_apply",
        "http_apply",
        "title",
        "date",
        "description",
        "locations",
        "country",
    ] {
        let mut details = ing_details();
        match mutation {
            "blank_identifier" => {
                details[0] = details[0].replace(r#""identifier": "REQ-111""#, r#""identifier": " ""#)
            }
            "duplicate_identifier" => {
                details[1] = details[1].replace(r#""value": "REQ-222""#, r#""value": "REQ-111""#)
            }
            "canonical_url" => details[0] = details[0].replace("/3121/111\"", "/3121/999\""),
            "http_detail_url" => details[0] = details[0].replace(
                "https://careers.ing.com/en/job",
                "http://careers.ing.com/en/job",
            ),
            "missing_apply" => details[0] = details[0].replace(
                r#"<meta name="search-job-apply-url" content="https://ing.wd3.myworkdayjobs.com/job/Platform-Engineer_REQ-111/apply">"#,
                "",
            ),
            "http_apply" => details[0] =
                details[0].replace("https://ing.wd3.myworkdayjobs.com", "http://example.com"),
            "title" => details[0] =
                details[0].replace(r#""title": "Platform Engineer""#, r#""title": " ""#),
            "date" => {
                details[0] = details[0].replace(r#""datePosted": "2026-8-10""#, r#""datePosted": """#)
            }
            "description" => details[0] = details[0].replace(
                r#""description": "<p>Build reliable platforms.</p><ul><li>Automate safely.</li></ul>""#,
                r#""description": "<p> </p>""#,
            ),
            "locations" => mutate_json_ld(&mut details[0], |posting| {
                posting["jobLocation"] = serde_json::json!([]);
            }),
            "country" => mutate_json_ld(&mut details[0], |posting| {
                posting["jobLocation"][0]["address"]["addressCountry"] = "".into();
            }),
            _ => unreachable!(),
        }
        assert_schema_error(parse_fixture(ing_listings(), details));
    }
}

#[test]
fn ing_accepts_array_valued_jobposting_type() {
    let mut details = ing_details();
    mutate_json_ld(&mut details[0], |posting| {
        posting["@type"] = serde_json::json!(["Thing", "JobPosting"]);
    });

    assert_eq!(
        parse_fixture(ing_listings(), details).unwrap()[0].source_id,
        "REQ-111"
    );
}

#[tokio::test]
#[ignore = "live external source"]
async fn ing_live_returns_complete_unique_jobs() {
    let source = IngSource::new("ing", LISTING_URL, live_client());
    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("ING scan must be complete");
    };

    assert!(!observations.is_empty());
    let mut ids = HashSet::new();
    for job in &observations {
        assert!(ids.insert(&job.source_id));
        assert!(!job.source_id.trim().is_empty());
        assert!(!job.title.trim().is_empty());
        assert!(!job.locations.is_empty());
        assert!(!job.countries.is_empty());
        assert!(
            job.countries.iter().all(
                |country| country.len() == 2 && country.chars().all(|c| c.is_ascii_uppercase())
            )
        );
        assert!(job.job_url.starts_with("https://"));
        assert!(job.apply_url.starts_with("https://"));
        assert!(!job.description.trim().is_empty());
        assert!(job.published_at.is_some());
        assert!(
            job.raw_payload["listing"]["data-job-id"]
                .as_str()
                .is_some_and(|id| !id.trim().is_empty())
        );
        assert_eq!(
            job.raw_payload["jobPosting"]["identifier"]
                .as_str()
                .or_else(|| job.raw_payload["jobPosting"]["identifier"]["value"].as_str())
                .unwrap(),
            job.source_id
        );
    }
    println!("ING: {} jobs", observations.len());
}

fn ing_listings() -> Vec<String> {
    [
        include_str!("fixtures/ing/list-page-1.html"),
        include_str!("fixtures/ing/list-page-2.html"),
    ]
    .map(str::to_owned)
    .into()
}

fn ing_details() -> Vec<String> {
    [
        include_str!("fixtures/ing/detail-111.html"),
        include_str!("fixtures/ing/detail-222.html"),
        include_str!("fixtures/ing/detail-333.html"),
    ]
    .map(str::to_owned)
    .into()
}

fn parse_fixture(
    listings: Vec<String>,
    details: Vec<String>,
) -> Result<Vec<ObservedJob>, techjobsnl::sources::SourceError> {
    let listings = listings.iter().map(String::as_str).collect::<Vec<_>>();
    let details = details.iter().map(String::as_str).collect::<Vec<_>>();
    parse_ing_pages("ing", LISTING_URL, &listings, &details)
}

fn set_listing_attr(html: &mut String, name: &str, value: &str) {
    let marker = format!(r#"{name}=""#);
    let start = html.find(&marker).unwrap() + marker.len();
    let end = start + html[start..].find('"').unwrap();
    html.replace_range(start..end, value);
}

fn assert_schema_error(result: Result<Vec<ObservedJob>, techjobsnl::sources::SourceError>) {
    let error = result.unwrap_err();
    assert_eq!(error.kind, SourceErrorKind::Schema);
    assert!(!error.retryable);
}

fn json_ld_value(html: &str) -> serde_json::Value {
    let start = html.find("<script type=\"application/ld+json\">").unwrap()
        + "<script type=\"application/ld+json\">".len();
    let end = html[start..].find("</script>").unwrap() + start;
    serde_json::from_str(&html[start..end]).unwrap()
}

fn mutate_json_ld(html: &mut String, mutation: impl FnOnce(&mut serde_json::Value)) {
    let marker = "<script type=\"application/ld+json\">";
    let start = html.find(marker).unwrap() + marker.len();
    let end = html[start..].find("</script>").unwrap() + start;
    let mut posting: serde_json::Value = serde_json::from_str(&html[start..end]).unwrap();
    mutation(&mut posting);
    html.replace_range(start..end, &posting.to_string());
}

fn live_client() -> reqwest::Client {
    build_client("techjobsnl/0.1 (+ING live test)", Duration::from_secs(30)).unwrap()
}
