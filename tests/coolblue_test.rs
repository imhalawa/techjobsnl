use job_watch::{
    domain::SourceScan,
    sources::{
        JobSource,
        coolblue::{CoolblueSource, parse_coolblue_pages},
    },
};

const LISTING_URL: &str = "https://www.coolblue.nl/en/vacancies/search";

fn listing(count: usize) -> String {
    format!(
        r#"<html><body><h2>{count} jobs found</h2>
        <a href="/en/vacancies/c-developer" aria-label="C# Developer"><h3>C# Developer</h3></a>
        <a href="/en/vacancies/cloud-engineer" aria-label="Cloud Engineer"><h3>Cloud Engineer</h3></a>
        </body></html>"#
    )
}

fn detail(id: &str, slug: &str, title: &str) -> String {
    format!(
        r#"<script type="application/ld+json">{{
          "@context":"https://schema.org",
          "@type":"JobPosting",
          "identifier":{{"@type":"PropertyValue","name":"Coolblue","value":"{id}"}},
          "url":"https://www.coolblue.nl/en/vacancies/{slug}",
          "title":"{title} - 36-40 hour",
          "description":"<h2>Role</h2><p>Build reliable systems.</p>",
          "employmentType":"FULL_TIME",
          "datePosted":"2026-05-12",
          "hiringOrganization":{{"@type":"Organization","name":"Coolblue"}},
          "jobLocation":{{"@type":"Place","address":{{"@type":"PostalAddress","addressLocality":"Rotterdam","addressCountry":"NL"}}}}
        }}</script>"#
    )
}

#[test]
fn parses_a_complete_coolblue_snapshot() {
    let first = detail("job-1", "c-developer", "C# Developer");
    let second = detail("job-2", "cloud-engineer", "Cloud Engineer");
    let jobs =
        parse_coolblue_pages("coolblue", LISTING_URL, &listing(2), &[&first, &second]).unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].source_id, "job-1");
    assert_eq!(jobs[0].title, "C# Developer - 36-40 hour");
    assert_eq!(jobs[0].locations, ["Rotterdam"]);
    assert_eq!(jobs[0].countries, ["NL"]);
    assert_eq!(jobs[0].employment_type.as_deref(), Some("FULL_TIME"));
    assert!(jobs[0].description.contains("Build reliable systems."));
    assert_eq!(jobs[0].job_url, jobs[0].apply_url);
}

#[test]
fn rejects_incomplete_or_untrusted_coolblue_pages() {
    let first = detail("job-1", "c-developer", "C# Developer");
    let second = detail("job-2", "cloud-engineer", "Cloud Engineer");
    assert!(
        parse_coolblue_pages("coolblue", LISTING_URL, &listing(3), &[&first, &second]).is_err()
    );
    assert!(
        parse_coolblue_pages(
            "coolblue",
            "https://example.test/en/vacancies/search",
            &listing(2),
            &[&first, &second],
        )
        .is_err()
    );
}

#[tokio::test]
#[ignore = "live external source"]
async fn coolblue_live_returns_complete_unique_netherlands_jobs() {
    let client = reqwest::Client::builder()
        .user_agent("techjobsnl-live-test/0.1")
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap();
    let scan = CoolblueSource::new("coolblue", LISTING_URL, client)
        .scan()
        .await
        .unwrap();
    let SourceScan::Complete { observations } = scan else {
        panic!("Coolblue must return a complete scan");
    };
    assert!(!observations.is_empty());
    assert!(observations.iter().all(|job| job.countries == ["NL"]));
    let unique = observations
        .iter()
        .map(|job| job.source_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(unique.len(), observations.len());
}
