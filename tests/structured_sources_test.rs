use std::{collections::HashSet, time::Duration};

use futures_util::{StreamExt, stream};
use job_watch::{
    domain::{ObservedJob, SourceErrorKind, SourceScan},
    sources::{
        JobSource,
        ashby::build_client,
        bol::{BolSource, parse_bol_pages},
    },
};

#[test]
fn parses_complete_bol_pages() {
    let jobs = parse_fixture_pages(bol_fixtures()).unwrap();

    assert_eq!(jobs.len(), 3);
    assert_eq!(jobs[0].source_id, "bol-101");
    assert_eq!(jobs[0].title, "Senior Platform Engineer & Reliability");
    assert_eq!(jobs[0].locations, ["Utrecht"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].department.as_deref(), Some("Engineering"));
    assert_eq!(jobs[0].team.as_deref(), Some("Platform"));
    assert_eq!(jobs[0].employment_type.as_deref(), Some("Full-time"));
    assert_eq!(
        jobs[0].description,
        "Serve customers.\n\nSolve hard problems.\n\nBuild reliable systems.\n\nAutomate safely.\n\nBring curiosity.\n\nWork in Utrecht."
    );
    assert_eq!(
        jobs[0].job_url,
        "https://careers.bol.com/nl/vacatures/senior-platform-engineer-reliability/101/"
    );
    assert_eq!(jobs[0].apply_url, format!("{}#form", jobs[0].job_url));
    assert_eq!(
        jobs[0].published_at.unwrap().to_rfc3339(),
        "2026-08-10T08:00:00+00:00"
    );
    let fixtures = bol_fixtures();
    assert_eq!(
        jobs[0].raw_payload,
        fixtures[0]["hits"]["hits"][0]["_source"]
    );
    assert_eq!(
        jobs[1].raw_payload,
        fixtures[0]["hits"]["hits"][1]["_source"]
    );
    assert_eq!(
        jobs[2].raw_payload,
        fixtures[1]["hits"]["hits"][0]["_source"]
    );
}

#[test]
fn bol_rejects_invalid_records_and_rich_text() {
    for mutation in [
        "id",
        "title",
        "office",
        "published",
        "status",
        "internal",
        "description",
        "block",
        "span",
    ] {
        let mut pages = bol_fixtures();
        let source = &mut pages[0]["hits"]["hits"][0]["_source"];
        match mutation {
            "id" => {
                source.as_object_mut().unwrap().remove("id");
            }
            "title" => source["title"] = " ".into(),
            "office" => {
                source["office"].as_object_mut().unwrap().remove("label");
            }
            "published" => {
                source.as_object_mut().unwrap().remove("publicationDate");
            }
            "status" => source["status"] = "DRAFT".into(),
            "internal" => source["internal"] = true.into(),
            "description" => {
                for field in [
                    "howDoYouMakeOurCustomerSmile",
                    "theBiggestChallenge",
                    "whatYouWillDoAs",
                    "whyYouCanMakeADifference",
                    "whereYoullBeWorking",
                ] {
                    source[field]["content"] = serde_json::json!([]);
                }
            }
            "block" => source["whatYouWillDoAs"]["content"][0]["_type"] = "image".into(),
            "span" => {
                source["whatYouWillDoAs"]["content"][0]["children"][0]["_type"] = "link".into()
            }
            _ => unreachable!(),
        };
        assert_schema_error(parse_fixture_pages(pages));
    }
}

#[test]
fn bol_rejects_incomplete_or_inconsistent_pagination() {
    assert_schema_error(parse_fixture_pages(vec![bol_fixtures().remove(0)]));

    for mutation in [
        "gte",
        "drift",
        "empty",
        "duplicate",
        "overshoot",
        "missing_hits",
    ] {
        let mut pages = bol_fixtures();
        match mutation {
            "gte" => pages[0]["hits"]["total"]["relation"] = "gte".into(),
            "drift" => pages[1]["hits"]["total"]["value"] = 4.into(),
            "empty" => pages[1]["hits"]["hits"] = serde_json::json!([]),
            "duplicate" => pages[1]["hits"]["hits"][0]["_id"] = "bol-101".into(),
            "overshoot" => {
                pages[0]["hits"]["total"]["value"] = 1.into();
                pages[1]["hits"]["total"]["value"] = 1.into();
            }
            "missing_hits" => {
                pages[0]["hits"].as_object_mut().unwrap().remove("hits");
            }
            _ => unreachable!(),
        }
        assert_schema_error(parse_fixture_pages(pages));
    }
}

#[test]
fn bol_rejects_an_early_short_page() {
    let mut pages = bol_fixtures();
    pages[0]["hits"]["total"]["value"] = 5.into();
    pages[1]["hits"]["total"]["value"] = 5.into();
    let mut last = pages[0].clone();
    last["hits"]["total"]["value"] = 5.into();
    last["hits"]["hits"][0]["_id"] = "bol-104".into();
    last["hits"]["hits"][0]["_source"]["id"] = 104.into();
    last["hits"]["hits"][1]["_id"] = "bol-105".into();
    last["hits"]["hits"][1]["_source"]["id"] = 105.into();
    pages.push(last);
    assert_schema_error(parse_fixture_pages(pages));
}

#[tokio::test]
#[ignore = "live external source"]
async fn bol_live_returns_complete_unique_jobs_and_working_urls() {
    let client = live_client();
    let source = BolSource::new("bol", "https://careers.bol.com", client.clone());
    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("bol scan must be complete");
    };
    assert_live_jobs(&observations);

    let checks = stream::iter(observations.iter().map(|job| job.job_url.clone()))
        .map(|url| {
            let client = client.clone();
            async move {
                let response = client.get(&url).send().await.unwrap();
                assert!(
                    response.status().is_success(),
                    "{} returned {}",
                    url,
                    response.status()
                );
            }
        })
        .buffer_unordered(4)
        .collect::<Vec<_>>()
        .await;
    assert_eq!(checks.len(), observations.len());
    println!(
        "bol: {} jobs with successful detail URLs",
        observations.len()
    );
}

fn bol_fixtures() -> Vec<serde_json::Value> {
    [
        include_str!("fixtures/bol/page-1.json"),
        include_str!("fixtures/bol/page-2.json"),
    ]
    .map(|raw| serde_json::from_str(raw).unwrap())
    .into()
}

fn parse_fixture_pages(
    pages: Vec<serde_json::Value>,
) -> Result<Vec<ObservedJob>, job_watch::sources::SourceError> {
    let raw = pages
        .iter()
        .map(serde_json::Value::to_string)
        .collect::<Vec<_>>();
    let refs = raw.iter().map(String::as_str).collect::<Vec<_>>();
    parse_bol_pages("bol", "https://careers.bol.com", &refs)
}

fn assert_schema_error(result: Result<Vec<ObservedJob>, job_watch::sources::SourceError>) {
    let error = result.unwrap_err();
    assert_eq!(error.kind, SourceErrorKind::Schema);
    assert!(!error.retryable);
}

fn live_client() -> reqwest::Client {
    build_client("job-watch/0.1 (+bol live test)", Duration::from_secs(30)).unwrap()
}

fn assert_live_jobs(jobs: &[ObservedJob]) {
    assert!(!jobs.is_empty());
    let mut ids = HashSet::new();
    for job in jobs {
        assert!(ids.insert(&job.source_id));
        assert!(!job.title.trim().is_empty());
        assert!(!job.locations.is_empty());
        assert_eq!(job.countries, ["NL"]);
        assert!(!job.description.trim().is_empty());
        assert!(job.raw_payload.is_object());
        assert!(job.published_at.is_some());
    }
}
