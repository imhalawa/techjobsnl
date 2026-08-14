use std::{collections::HashSet, time::Duration};

use job_watch::{
    domain::{SourceErrorKind, SourceScan},
    sources::{
        JobSource,
        yuki::{YukiSource, parse_yuki_feed},
    },
};

#[test]
fn yuki_parses_complete_official_json_feed() {
    let jobs = parse_yuki_feed("yuki", &fixture()).unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "8166605");
    assert_eq!(jobs[0].title, "Sr. Back-end Developer");
    assert_eq!(jobs[0].locations, ["Rotterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(
        jobs[0].job_url,
        "https://jobs.yukisoftware.com/jobs/8166605-sr-back-end-developer"
    );
    assert_eq!(
        jobs[0].apply_url,
        "https://jobs.yukisoftware.com/jobs/8166605-sr-back-end-developer/applications/new"
    );
    assert_eq!(
        jobs[0].description,
        "Build accounting software with **.NET**."
    );
    assert_eq!(
        jobs[0].published_at.unwrap().to_rfc3339(),
        "2026-08-04T08:20:25+00:00"
    );
}

#[test]
fn yuki_rejects_feed_identity_duplicates_and_job_drift() {
    for mutation in ["feed", "duplicate", "title", "employer", "url"] {
        let mut feed: serde_json::Value = serde_json::from_str(&fixture()).unwrap();
        match mutation {
            "feed" => feed["feed_url"] = "https://example.com/jobs.json".into(),
            "duplicate" => feed["items"][1]["_jobposting"]["identifier"]["value"] = 8166605.into(),
            "title" => feed["items"][0]["_jobposting"]["title"] = "Other".into(),
            "employer" => {
                feed["items"][0]["_jobposting"]["hiringOrganization"]["name"] = "Other".into()
            }
            "url" => feed["items"][0]["url"] = "https://example.com/jobs/8166605-role".into(),
            _ => unreachable!(),
        }
        let error = parse_yuki_feed("yuki", &feed.to_string()).unwrap_err();
        assert_eq!(error.kind, SourceErrorKind::Schema);
    }
}

#[tokio::test]
#[ignore = "live external source"]
async fn yuki_live_returns_complete_unique_jobs() {
    let source = YukiSource::new(
        "yuki",
        "https://jobs.yukisoftware.com/jobs.json",
        reqwest::Client::builder()
            .user_agent("job-watch/0.1 (+Yuki live test)")
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap(),
    );
    let SourceScan::Complete { observations } = source.scan().await.unwrap() else {
        panic!("Yuki scan must be complete");
    };
    assert!(!observations.is_empty());
    assert!(
        observations
            .iter()
            .any(|job| job.countries.contains(&"NL".into()))
    );
    assert_eq!(
        observations
            .iter()
            .map(|job| &job.source_id)
            .collect::<HashSet<_>>()
            .len(),
        observations.len()
    );
    println!("Yuki: {} jobs", observations.len());
}

fn fixture() -> String {
    let item = |id: u64, slug: &str, title: &str, country: &str, city: &str| {
        let url = format!("https://jobs.yukisoftware.com/jobs/{id}-{slug}");
        serde_json::json!({
            "id": format!("feed-{id}"),
            "url": url,
            "title": title,
            "content_html": "<p>Build accounting software with <strong>.NET</strong>.</p>",
            "date_published": "2026-08-04T10:20:25+02:00",
            "_jobposting": {
                "@type": "JobPosting",
                "title": title,
                "description": "<p>Build accounting software with <strong>.NET</strong>.</p>",
                "identifier": {"value": id},
                "datePosted": "2026-08-04T10:20:25+02:00",
                "hiringOrganization": {"name": "The Yuki Company"},
                "jobLocation": [{"address": {"addressLocality": city, "addressCountry": country}}]
            }
        })
    };
    serde_json::json!({
        "version": "https://jsonfeed.org/version/1.1",
        "title": "The Yuki Company",
        "home_page_url": "https://jobs.yukisoftware.com/jobs",
        "feed_url": "https://jobs.yukisoftware.com/jobs.json",
        "items": [
            item(8166605, "sr-back-end-developer", "Sr. Back-end Developer", "NL", "Rotterdam"),
            item(8165815, "sr-cloud-engineer", "Sr. Cloud Engineer", "ES", "Barcelona")
        ]
    })
    .to_string()
}
