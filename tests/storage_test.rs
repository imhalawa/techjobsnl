use std::collections::HashMap;

use chrono::{DateTime, TimeZone, Utc};
use job_watch::{
    config::{CompanyConfig, SourceConfig},
    domain::{ClassifiedJob, Eligibility, JobKey, ObservedJob, ScanFailure, SourceErrorKind},
    storage::{JobQuery, Store},
};
use rusqlite::Connection;
use serde_json::json;

fn at(hour: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 11, hour, 0, 0).unwrap()
}

fn company(id: &str, enabled: bool) -> CompanyConfig {
    CompanyConfig {
        id: id.into(),
        name: id.to_uppercase(),
        enabled,
        location_country_overrides: HashMap::new(),
        source: SourceConfig::Ashby { board: id.into() },
    }
}

fn mollie_config() -> CompanyConfig {
    company("mollie", true)
}

fn job(source_id: &str) -> ClassifiedJob {
    ClassifiedJob {
        observed: ObservedJob {
            source_id: source_id.into(),
            title: format!("Software Engineer {source_id}"),
            department: Some("Engineering".into()),
            team: Some("Platform".into()),
            employment_type: Some("Full-time".into()),
            locations: vec!["Amsterdam".into()],
            countries: vec!["NL".into()],
            job_url: format!("https://careers.example.test/jobs/{source_id}"),
            apply_url: format!("https://careers.example.test/jobs/{source_id}/apply"),
            description: "Build reliable systems.".into(),
            raw_payload: json!({"id": source_id, "request_id": "first"}),
            published_at: Some(at(8)),
        },
        eligibility: Eligibility {
            eligible: true,
            reason: "eligible".into(),
        },
    }
}

#[test]
fn complete_scans_advance_lifecycle_and_preserve_applied_state() {
    let (t0, t1, t2) = (at(9), at(10), at(11));
    let config = mollie_config();
    let mut store = Store::open_in_memory().unwrap();
    store.sync_companies(std::slice::from_ref(&config)).unwrap();

    store
        .record_complete_scan("run-1", &config, &[job("1"), job("2")], t0, t1)
        .unwrap();
    assert_eq!(store.list_jobs(JobQuery::active()).unwrap().len(), 2);
    assert!(
        store
            .list_jobs(JobQuery::new())
            .unwrap()
            .iter()
            .all(|job| job.is_new)
    );

    store
        .toggle_applied(&JobKey::new("mollie", "1"), t1)
        .unwrap();
    store
        .record_complete_scan("run-2", &config, &[job("1")], t1, t2)
        .unwrap();

    let active = store.list_jobs(JobQuery::active()).unwrap();
    assert_eq!(active.len(), 1);
    assert!(!active[0].is_new);
    assert!(active[0].applied_at.is_some());
    let applied = store.list_jobs(JobQuery::applied()).unwrap();
    assert_eq!(applied.len(), 1);
    assert_eq!(applied[0].key, JobKey::new("mollie", "1"));
    let history = store.list_jobs(JobQuery::history()).unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].key, JobKey::new("mollie", "2"));
    assert!(!history[0].source_open);
    assert_eq!(history[0].closed_at, Some(t2));

    store
        .toggle_applied(&JobKey::new("mollie", "1"), t2)
        .unwrap();
    assert!(store.list_jobs(JobQuery::applied()).unwrap().is_empty());
    assert_eq!(
        store.list_jobs(JobQuery::active()).unwrap()[0].applied_at,
        None
    );
}

#[test]
fn duplicate_observations_of_a_first_identity_remain_new() {
    let config = mollie_config();
    let mut duplicate = job("1");
    duplicate.observed.raw_payload = json!({"id": "1", "request_id": "duplicate"});
    let mut store = Store::open_in_memory().unwrap();
    store.sync_companies(std::slice::from_ref(&config)).unwrap();

    store
        .record_complete_scan("run-1", &config, &[job("1"), duplicate], at(9), at(10))
        .unwrap();

    let all = store.list_jobs(JobQuery::all()).unwrap();
    assert_eq!(all.len(), 1);
    assert!(all[0].is_new);
    assert_eq!(store.list_jobs(JobQuery::new()).unwrap().len(), 1);
}

#[test]
fn failed_and_incomplete_scans_record_health_without_mutating_jobs() {
    let path = tempfile::NamedTempFile::new().unwrap();
    let config = mollie_config();
    let mut store = Store::open(path.path()).unwrap();
    store.sync_companies(std::slice::from_ref(&config)).unwrap();
    store
        .record_complete_scan("run-1", &config, &[job("1")], at(9), at(10))
        .unwrap();

    store
        .record_failed_scan(
            "run-2",
            &config,
            &ScanFailure {
                kind: SourceErrorKind::Transport,
                diagnostic: "connection reset".into(),
            },
            at(10),
            at(11),
        )
        .unwrap();
    store
        .record_incomplete_scan("run-3", &config, "unresolved location", 7, at(11), at(12))
        .unwrap();

    let jobs = store.list_jobs(JobQuery::all()).unwrap();
    assert_eq!(jobs.len(), 1);
    assert!(jobs[0].source_open);
    assert!(jobs[0].is_new);
    assert_eq!(jobs[0].last_seen_at, at(10));

    let connection = Connection::open(path.path()).unwrap();
    let outcomes = connection
        .prepare("SELECT outcome FROM scans ORDER BY completed_at")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(outcomes, ["complete", "failed", "incomplete"]);
    let failed: (String, String) = connection
        .query_row(
            "SELECT error_kind, diagnostic FROM scans WHERE outcome = 'failed'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!(failed, ("transport".into(), "connection reset".into()));
    let health: (String, Option<String>, Option<String>) = connection
        .query_row(
            "SELECT health, latest_error_kind, latest_diagnostic FROM companies WHERE id = 'mollie'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        health,
        (
            "incomplete".into(),
            None,
            Some("unresolved location".into())
        )
    );
}

#[test]
fn a_closed_identity_reopens_without_becoming_new_and_remains_in_history() {
    let config = mollie_config();
    let mut store = Store::open_in_memory().unwrap();
    store.sync_companies(std::slice::from_ref(&config)).unwrap();
    store
        .record_complete_scan("run-1", &config, &[job("1")], at(9), at(10))
        .unwrap();
    store
        .record_complete_scan("run-2", &config, &[], at(10), at(11))
        .unwrap();
    store
        .record_complete_scan("run-3", &config, &[job("1")], at(11), at(12))
        .unwrap();

    let active = store.list_jobs(JobQuery::active()).unwrap();
    assert_eq!(active.len(), 1);
    assert!(!active[0].is_new);
    assert!(active[0].source_open);
    assert_eq!(active[0].closed_at, None);
    assert_eq!(active[0].reopened_at, Some(at(12)));
    assert_eq!(
        store.list_jobs(JobQuery::history()).unwrap()[0].key,
        JobKey::new("mollie", "1")
    );
}

#[test]
fn snapshots_ignore_raw_payload_churn_and_capture_changed_descriptions() {
    let path = tempfile::NamedTempFile::new().unwrap();
    let config = mollie_config();
    let mut store = Store::open(path.path()).unwrap();
    store.sync_companies(std::slice::from_ref(&config)).unwrap();
    store
        .record_complete_scan("run-1", &config, &[job("1")], at(9), at(10))
        .unwrap();

    let mut raw_changed = job("1");
    raw_changed.observed.raw_payload = json!({"id": "1", "request_id": "second"});
    store
        .record_complete_scan("run-2", &config, &[raw_changed], at(10), at(11))
        .unwrap();

    let mut description_changed = job("1");
    description_changed.observed.description = "Build reliable payment systems.".into();
    store
        .record_complete_scan("run-3", &config, &[description_changed], at(11), at(12))
        .unwrap();

    let snapshot_count: i64 = Connection::open(path.path())
        .unwrap()
        .query_row("SELECT COUNT(*) FROM job_snapshots", [], |row| row.get(0))
        .unwrap();
    assert_eq!(snapshot_count, 2);
}

#[test]
fn all_keeps_disabled_and_ineligible_jobs_that_user_facing_queries_hide() {
    let enabled = mollie_config();
    let disabled = company("disabled", false);
    let mut ineligible = job("ineligible");
    ineligible.eligibility = Eligibility {
        eligible: false,
        reason: "excluded-title".into(),
    };
    let mut store = Store::open_in_memory().unwrap();
    store
        .sync_companies(&[enabled.clone(), disabled.clone()])
        .unwrap();
    store
        .record_complete_scan(
            "run-1",
            &enabled,
            &[job("eligible"), ineligible],
            at(9),
            at(10),
        )
        .unwrap();
    store
        .record_complete_scan("run-1", &disabled, &[job("disabled")], at(9), at(10))
        .unwrap();

    assert_eq!(store.list_jobs(JobQuery::all()).unwrap().len(), 3);
    assert_eq!(store.list_jobs(JobQuery::active()).unwrap().len(), 1);
    assert_eq!(store.list_jobs(JobQuery::new()).unwrap().len(), 1);
}

#[test]
fn complete_scan_rolls_back_every_job_change_when_scan_recording_fails() {
    let path = tempfile::NamedTempFile::new().unwrap();
    let config = mollie_config();
    let mut store = Store::open(path.path()).unwrap();
    store.sync_companies(std::slice::from_ref(&config)).unwrap();
    store
        .record_complete_scan("run-1", &config, &[job("1"), job("2")], at(9), at(10))
        .unwrap();
    Connection::open(path.path())
        .unwrap()
        .execute_batch(
            "CREATE TRIGGER reject_scan BEFORE INSERT ON scans
             WHEN NEW.run_id = 'run-rollback'
             BEGIN SELECT RAISE(ABORT, 'forced scan failure'); END;",
        )
        .unwrap();

    let error = store
        .record_complete_scan("run-rollback", &config, &[job("1")], at(10), at(11))
        .unwrap_err();
    assert!(error.to_string().contains("forced scan failure"));

    let jobs = store.list_jobs(JobQuery::all()).unwrap();
    assert_eq!(jobs.len(), 2);
    assert!(jobs.iter().all(|job| job.source_open && job.is_new));
    assert!(jobs.iter().all(|job| job.last_seen_at == at(10)));
}
