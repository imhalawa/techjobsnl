use std::collections::HashMap;

use chrono::{TimeZone, Utc};
use job_watch::{
    config::FiltersConfig,
    domain::ObservedJob,
    filter::{EligibilityFilter, FilterError},
};
use serde_json::json;

fn filters() -> FiltersConfig {
    FiltersConfig {
        countries: vec!["NL".into()],
        include_families: vec!["software".into(), "platform".into()],
        include_title_patterns: vec!["principal engineer".into()],
        exclude_title_patterns: vec!["manager".into()],
    }
}

fn observed(title: &str, locations: &[&str], countries: &[&str]) -> ObservedJob {
    ObservedJob {
        source_id: "job-123".into(),
        title: title.into(),
        department: Some("Engineering".into()),
        team: Some("Platform".into()),
        employment_type: Some("Full-time".into()),
        locations: locations
            .iter()
            .map(|location| (*location).into())
            .collect(),
        countries: countries.iter().map(|country| (*country).into()).collect(),
        job_url: "https://careers.example.test/jobs/job-123".into(),
        apply_url: "https://careers.example.test/jobs/job-123/apply".into(),
        description: "Build reliable systems.".into(),
        raw_payload: json!({"id": "job-123", "title": title}),
        published_at: Some(Utc.with_ymd_and_hms(2026, 8, 11, 9, 30, 0).unwrap()),
    }
}

#[test]
fn classifies_country_role_and_overrides() {
    let filter = EligibilityFilter::new(&filters()).unwrap();
    let cases: [(
        ObservedJob,
        HashMap<String, String>,
        Result<(bool, &str), ()>,
    ); 4] = [
        (
            observed("Senior Platform Engineer", &["Amsterdam"], &["NL"]),
            HashMap::new(),
            Ok((true, "eligible")),
        ),
        (
            observed("Software Engineer", &["Lisbon"], &["PT"]),
            HashMap::new(),
            Ok((false, "outside-netherlands")),
        ),
        (
            observed("Engineering Manager", &["Amsterdam"], &["NL"]),
            HashMap::new(),
            Ok((false, "excluded-title")),
        ),
        (
            observed("Software Engineer", &["Amsterdam"], &[]),
            HashMap::from([("Amsterdam".into(), "NL".into())]),
            Ok((true, "eligible")),
        ),
    ];

    for (job, overrides, expected) in cases {
        let eligibility = filter.classify(&job, &overrides).unwrap();
        assert_eq!(
            (eligibility.eligible, eligibility.reason.as_str()),
            expected.unwrap()
        );
    }
}

#[test]
fn unresolved_location_makes_the_result_incomplete() {
    let error = EligibilityFilter::new(&filters())
        .unwrap()
        .classify(
            &observed("Software Engineer", &["Hybrid"], &[]),
            &HashMap::new(),
        )
        .unwrap_err();
    assert_eq!(
        error,
        FilterError::UnresolvedLocation(vec!["Hybrid".into()])
    );
}

#[test]
fn rejects_filters_that_include_a_non_nl_country() {
    let mut config = filters();
    config.countries.push("DE".into());

    let error = EligibilityFilter::new(&config).unwrap_err();

    assert_eq!(error, FilterError::UnsupportedCountry("DE".into()));
}
