use std::{
    collections::{HashMap, HashSet},
    fmt::Write as _,
    path::Path,
};

use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, Result, Row, Transaction, params, types::Type};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    analytics::{self, JobFacts, SkillSuggestion, SuggestionStatus},
    config::{AnalyticsConfig, CompanyConfig},
    domain::{
        ClassifiedJob, Eligibility, JobKey, JobRecord, ObservedJob, ScanFailure, SourceErrorKind,
    },
    insights::{AnalyticsFilters, LibraryState},
};

use super::schema;

pub struct Store {
    connection: Connection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanOutcome {
    Complete,
    Incomplete,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanReadModel {
    pub run_id: String,
    pub company_id: String,
    pub company_name: String,
    pub completed_at: DateTime<Utc>,
    pub outcome: ScanOutcome,
    pub observed_count: usize,
    pub error_kind: Option<SourceErrorKind>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceHealth {
    Unknown,
    Healthy,
    Incomplete,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceReadModel {
    pub company_id: String,
    pub company_name: String,
    pub enabled: bool,
    pub latest_attempted_at: Option<DateTime<Utc>>,
    pub latest_successful_at: Option<DateTime<Utc>>,
    pub health: SourceHealth,
    pub latest_error_kind: Option<SourceErrorKind>,
    pub diagnostic: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct JobQuery(QueryKind);

#[derive(Debug, Clone, Copy)]
enum QueryKind {
    Active,
    New,
    Applied,
    History,
    Analytics,
    All,
}

#[allow(clippy::new_without_default)] // `new` means newly discovered jobs, not a constructor.
impl JobQuery {
    pub fn active() -> Self {
        Self(QueryKind::Active)
    }

    pub fn new() -> Self {
        Self(QueryKind::New)
    }

    pub fn applied() -> Self {
        Self(QueryKind::Applied)
    }

    pub fn history() -> Self {
        Self(QueryKind::History)
    }

    pub fn analytics() -> Self {
        Self(QueryKind::Analytics)
    }

    pub fn all() -> Self {
        Self(QueryKind::All)
    }
}

impl Store {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        Self::from_connection(Connection::open(path)?)
    }

    pub fn open_in_memory() -> Result<Self> {
        Self::from_connection(Connection::open_in_memory()?)
    }

    fn from_connection(connection: Connection) -> Result<Self> {
        schema::migrate(&connection)?;
        Ok(Self { connection })
    }

    pub fn sync_companies(&mut self, companies: &[CompanyConfig]) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute("UPDATE companies SET enabled = 0", [])?;
        for company in companies {
            tx.execute(
                "INSERT INTO companies (id, name, enabled)
                 VALUES (?1, ?2, ?3)
                 ON CONFLICT(id) DO UPDATE SET
                    name = excluded.name,
                    enabled = excluded.enabled",
                params![company.id, company.name, company.enabled],
            )?;
        }
        tx.commit()
    }

    pub fn record_complete_scan(
        &mut self,
        run_id: &str,
        company: &CompanyConfig,
        jobs: &[ClassifiedJob],
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        tx.execute(
            "UPDATE jobs SET is_new = 0 WHERE company_id = ?1",
            [company.id.as_str()],
        )?;
        for classified in jobs {
            upsert_observation(&tx, company, classified, completed_at)?;
            insert_snapshot_if_changed(&tx, company, classified, completed_at)?;
        }
        close_missing_source_ids(&tx, &company.id, jobs, completed_at)?;
        insert_scan_row(
            &tx,
            run_id,
            company,
            "complete",
            jobs.len(),
            None,
            None,
            started_at,
            completed_at,
        )?;
        tx.commit()
    }

    pub fn record_failed_scan(
        &mut self,
        run_id: &str,
        company: &CompanyConfig,
        error: &ScanFailure,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        let error_kind = error.kind.as_str();
        insert_scan_row(
            &tx,
            run_id,
            company,
            "failed",
            0,
            Some(error_kind),
            Some(&error.diagnostic),
            started_at,
            completed_at,
        )?;
        tx.commit()
    }

    pub fn record_incomplete_scan(
        &mut self,
        run_id: &str,
        company: &CompanyConfig,
        diagnostic: &str,
        observed_count: usize,
        started_at: DateTime<Utc>,
        completed_at: DateTime<Utc>,
    ) -> Result<()> {
        let tx = self.connection.transaction()?;
        insert_scan_row(
            &tx,
            run_id,
            company,
            "incomplete",
            observed_count,
            Some(SourceErrorKind::IncompleteResults.as_str()),
            Some(diagnostic),
            started_at,
            completed_at,
        )?;
        tx.commit()
    }

    pub fn list_jobs(&self, query: JobQuery) -> Result<Vec<JobRecord>> {
        let condition = match query.0 {
            QueryKind::Active => "c.enabled = 1 AND j.eligible = 1 AND j.source_open = 1",
            QueryKind::New => {
                "c.enabled = 1 AND j.eligible = 1 AND j.source_open = 1 AND j.is_new = 1"
            }
            QueryKind::Applied => "c.enabled = 1 AND j.eligible = 1 AND j.applied_at IS NOT NULL",
            QueryKind::History => {
                "c.enabled = 1 AND j.eligible = 1 AND (j.source_open = 0 OR j.reopened_at IS NOT NULL)"
            }
            QueryKind::Analytics => "c.enabled = 1 AND j.eligible = 1",
            QueryKind::All => "1 = 1",
        };
        let sql = format!(
            "SELECT
                j.company_id, j.source_id, j.title, j.department, j.team,
                j.employment_type, j.locations_json, j.countries_json, j.job_url,
                j.apply_url, j.description, j.raw_payload, j.published_at,
                j.eligible, j.eligibility_reason, j.source_open, j.is_new,
                j.first_seen_at, j.last_seen_at, j.closed_at, j.reopened_at,
                j.applied_at
             FROM jobs j
             JOIN companies c ON c.id = j.company_id
             WHERE {condition}
             ORDER BY j.last_seen_at DESC, j.company_id, j.source_id"
        );
        let mut statement = self.connection.prepare(&sql)?;
        statement
            .query_map([], job_record_from_row)?
            .collect::<Result<Vec<_>>>()
    }

    pub fn analytics_facts(
        &self,
        jobs: &[JobRecord],
        _config: &AnalyticsConfig,
    ) -> Result<HashMap<JobKey, JobFacts>> {
        let extractor_version = analytics::cache_version();
        let mut select = self.connection.prepare(
            "SELECT j.content_hash, a.facts_json
             FROM jobs j
             LEFT JOIN job_analytics a ON
                a.company_id = j.company_id AND
                a.source_id = j.source_id AND
                a.content_hash = j.content_hash AND
                a.extractor_version = ?3
             WHERE j.company_id = ?1 AND j.source_id = ?2",
        )?;
        let mut insert = self.connection.prepare(
            "INSERT OR IGNORE INTO job_analytics (
                company_id, source_id, content_hash, extractor_version, facts_json
             ) VALUES (?1, ?2, ?3, ?4, ?5)",
        )?;
        let mut facts_by_job = HashMap::with_capacity(jobs.len());
        for job in jobs {
            let stored = select
                .query_row(
                    params![job.key.company_id, job.key.source_id, extractor_version],
                    |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
                )
                .optional()?;
            let Some((content_hash, cached)) = stored else {
                continue;
            };
            let facts = match cached {
                Some(cached) => serde_json::from_str(&cached).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(1, Type::Text, Box::new(error))
                })?,
                None => {
                    let facts = analytics::extract(job);
                    insert.execute(params![
                        job.key.company_id,
                        job.key.source_id,
                        content_hash,
                        extractor_version,
                        json_text(&facts),
                    ])?;
                    facts
                }
            };
            facts_by_job.insert(job.key.clone(), facts);
        }
        Ok(facts_by_job)
    }

    pub fn enriched_analytics_facts(
        &self,
        jobs: &[JobRecord],
        config: &AnalyticsConfig,
    ) -> Result<HashMap<JobKey, JobFacts>> {
        let mut facts = self.analytics_facts(jobs, config)?;
        let approved = self
            .skill_suggestions()?
            .into_iter()
            .filter(|item| item.status == SuggestionStatus::Approved)
            .collect::<Vec<_>>();
        analytics::apply_approved_suggestions(&mut facts, jobs, &approved);
        if config.provider == crate::config::AnalyticsProvider::Local || jobs.is_empty() {
            return Ok(facts);
        }
        let fingerprint = jobs
            .iter()
            .map(|job| {
                format!(
                    "{}/{}:{}",
                    job.key.company_id, job.key.source_id, job.last_seen_at
                )
            })
            .collect::<Vec<_>>();
        let cache_key = analytics::discovery_cache_key(config, &fingerprint);
        let seen = self
            .connection
            .query_row(
                "SELECT 1 FROM analytics_discovery WHERE cache_key = ?1",
                [&cache_key],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !seen && let Some((_, suggestions)) = analytics::discover_emerging_skills(config, jobs) {
            for suggestion in &suggestions {
                self.connection.execute(
                    "INSERT OR IGNORE INTO skill_suggestions (
                            name, aliases_json, evidence_json, status, created_at
                         ) VALUES (?1, ?2, ?3, 'pending', ?4)",
                    params![
                        suggestion.name,
                        json_text(suggestion),
                        json_text(&suggestion.evidence),
                        Utc::now().to_rfc3339(),
                    ],
                )?;
            }
            self.connection.execute(
                "INSERT INTO analytics_discovery (cache_key, provider, result_json)
                     VALUES (?1, ?2, ?3)",
                params![cache_key, config.provider.as_str(), json_text(&suggestions)],
            )?;
        }
        Ok(facts)
    }

    pub fn skill_suggestions(&self) -> Result<Vec<SkillSuggestion>> {
        let mut statement = self.connection.prepare(
            "SELECT aliases_json, status FROM skill_suggestions
             ORDER BY CASE status WHEN 'pending' THEN 0 WHEN 'approved' THEN 1 ELSE 2 END,
                      created_at DESC, name",
        )?;
        statement
            .query_map([], |row| {
                let json: String = row.get(0)?;
                let mut suggestion =
                    serde_json::from_str::<SkillSuggestion>(&json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(0, Type::Text, Box::new(error))
                    })?;
                suggestion.status = match row.get::<_, String>(1)?.as_str() {
                    "pending" => SuggestionStatus::Pending,
                    "approved" => SuggestionStatus::Approved,
                    "rejected" => SuggestionStatus::Rejected,
                    value => {
                        return Err(rusqlite::Error::FromSqlConversionFailure(
                            1,
                            Type::Text,
                            format!("invalid suggestion status {value}").into(),
                        ));
                    }
                };
                Ok(suggestion)
            })?
            .collect()
    }

    pub fn review_skill_suggestion(&self, name: &str, status: SuggestionStatus) -> Result<()> {
        self.connection.execute(
            "UPDATE skill_suggestions SET status = ?2 WHERE name = ?1",
            params![name, status.as_str()],
        )?;
        Ok(())
    }

    pub fn recent_scans(&self) -> Result<Vec<ScanReadModel>> {
        // ponytail: keep the latest 100; add pagination when the scan view needs older runs.
        let mut statement = self.connection.prepare(
            "SELECT
                s.run_id, s.company_id, c.name, s.completed_at, s.outcome,
                s.observed_count, s.error_kind, s.diagnostic
             FROM scans s
             JOIN companies c ON c.id = s.company_id
             ORDER BY s.completed_at DESC, s.id DESC
             LIMIT 100",
        )?;
        statement
            .query_map([], scan_read_model_from_row)?
            .collect::<Result<Vec<_>>>()
    }

    pub fn analytics_scans(&self) -> Result<Vec<ScanReadModel>> {
        let mut statement = self.connection.prepare(
            "SELECT
                s.run_id, s.company_id, c.name, s.completed_at, s.outcome,
                s.observed_count, s.error_kind, s.diagnostic
             FROM scans s
             JOIN companies c ON c.id = s.company_id
             ORDER BY s.completed_at DESC, s.id DESC",
        )?;
        statement
            .query_map([], scan_read_model_from_row)?
            .collect::<Result<Vec<_>>>()
    }

    pub fn analytics_state(&self) -> Result<(AnalyticsFilters, LibraryState)> {
        let stored = self
            .connection
            .query_row(
                "SELECT filters_json, library_json FROM analytics_state WHERE id = 1",
                [],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()?;
        stored.map_or_else(
            || Ok((AnalyticsFilters::default(), LibraryState::default())),
            |(filters, library)| {
                Ok((
                    parse_json_column(0, &filters)?,
                    parse_json_column(1, &library)?,
                ))
            },
        )
    }

    pub fn save_analytics_state(
        &self,
        filters: &AnalyticsFilters,
        library: &LibraryState,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO analytics_state (id, filters_json, library_json)
             VALUES (1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET
                filters_json = excluded.filters_json,
                library_json = excluded.library_json",
            params![json_text(filters), json_text(library)],
        )?;
        Ok(())
    }

    pub fn source_health(&self) -> Result<Vec<SourceReadModel>> {
        let mut statement = self.connection.prepare(
            "SELECT
                id, name, enabled, latest_attempted_at, latest_successful_at,
                health, latest_error_kind, latest_diagnostic
             FROM companies
             ORDER BY name COLLATE NOCASE, id",
        )?;
        statement
            .query_map([], source_read_model_from_row)?
            .collect::<Result<Vec<_>>>()
    }

    pub fn toggle_applied(&mut self, key: &JobKey, at: DateTime<Utc>) -> Result<()> {
        let changed = self.connection.execute(
            "UPDATE jobs
             SET applied_at = CASE WHEN applied_at IS NULL THEN ?3 ELSE NULL END
             WHERE company_id = ?1 AND source_id = ?2",
            params![key.company_id, key.source_id, timestamp(at)],
        )?;
        if changed == 0 {
            Err(rusqlite::Error::QueryReturnedNoRows)
        } else {
            Ok(())
        }
    }
}

fn upsert_observation(
    tx: &Transaction<'_>,
    company: &CompanyConfig,
    classified: &ClassifiedJob,
    observed_at: DateTime<Utc>,
) -> Result<()> {
    let observed = &classified.observed;
    let content_hash = content_hash(observed);
    tx.execute(
        "INSERT INTO jobs (
            company_id, source_id, title, department, team, employment_type,
            locations_json, countries_json, job_url, apply_url, description,
            published_at, raw_payload, content_hash, eligible, eligibility_reason,
            source_open, is_new, first_seen_at, last_seen_at
         ) VALUES (
            ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14,
            ?15, ?16, 1, 1, ?17, ?17
         )
         ON CONFLICT(company_id, source_id) DO UPDATE SET
            title = excluded.title,
            department = excluded.department,
            team = excluded.team,
            employment_type = excluded.employment_type,
            locations_json = excluded.locations_json,
            countries_json = excluded.countries_json,
            job_url = excluded.job_url,
            apply_url = excluded.apply_url,
            description = excluded.description,
            published_at = excluded.published_at,
            raw_payload = excluded.raw_payload,
            content_hash = excluded.content_hash,
            eligible = excluded.eligible,
            eligibility_reason = excluded.eligibility_reason,
            source_open = 1,
            is_new = jobs.is_new,
            last_seen_at = excluded.last_seen_at,
            closed_at = NULL,
            reopened_at = CASE
                WHEN jobs.source_open = 0 THEN excluded.last_seen_at
                ELSE jobs.reopened_at
            END",
        params![
            company.id,
            observed.source_id,
            observed.title,
            observed.department,
            observed.team,
            observed.employment_type,
            json_text(&observed.locations),
            json_text(&observed.countries),
            observed.job_url,
            observed.apply_url,
            observed.description,
            observed.published_at.map(timestamp),
            observed.raw_payload.to_string(),
            content_hash,
            classified.eligibility.eligible,
            classified.eligibility.reason,
            timestamp(observed_at),
        ],
    )?;
    Ok(())
}

fn insert_snapshot_if_changed(
    tx: &Transaction<'_>,
    company: &CompanyConfig,
    classified: &ClassifiedJob,
    captured_at: DateTime<Utc>,
) -> Result<()> {
    let observed = &classified.observed;
    tx.execute(
        "INSERT OR IGNORE INTO job_snapshots (
            company_id, source_id, content_hash, captured_at, title, metadata_json,
            locations_json, job_url, apply_url, description, raw_payload
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            company.id,
            observed.source_id,
            content_hash(observed),
            timestamp(captured_at),
            observed.title,
            metadata_json(observed),
            json_text(&observed.locations),
            observed.job_url,
            observed.apply_url,
            observed.description,
            observed.raw_payload.to_string(),
        ],
    )?;
    Ok(())
}

fn close_missing_source_ids(
    tx: &Transaction<'_>,
    company_id: &str,
    jobs: &[ClassifiedJob],
    closed_at: DateTime<Utc>,
) -> Result<()> {
    let observed_ids = jobs
        .iter()
        .map(|job| job.observed.source_id.as_str())
        .collect::<HashSet<_>>();
    let existing_ids = {
        let mut statement =
            tx.prepare("SELECT source_id FROM jobs WHERE company_id = ?1 AND source_open = 1")?;
        statement
            .query_map([company_id], |row| row.get::<_, String>(0))?
            .collect::<Result<Vec<_>>>()?
    };
    for source_id in existing_ids {
        if !observed_ids.contains(source_id.as_str()) {
            tx.execute(
                "UPDATE jobs SET source_open = 0, is_new = 0, closed_at = ?3
                 WHERE company_id = ?1 AND source_id = ?2 AND source_open = 1",
                params![company_id, source_id, timestamp(closed_at)],
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn insert_scan_row(
    tx: &Transaction<'_>,
    run_id: &str,
    company: &CompanyConfig,
    outcome: &str,
    observed_count: usize,
    error_kind: Option<&str>,
    diagnostic: Option<&str>,
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
) -> Result<()> {
    let observed_count = i64::try_from(observed_count)
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
    tx.execute(
        "INSERT INTO scans (
            run_id, company_id, started_at, completed_at, outcome, observed_count,
            error_kind, diagnostic
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            run_id,
            company.id,
            timestamp(started_at),
            timestamp(completed_at),
            outcome,
            observed_count,
            error_kind,
            diagnostic,
        ],
    )?;
    match outcome {
        "complete" => tx.execute(
            "UPDATE companies SET
                latest_attempted_at = ?2,
                latest_successful_at = ?2,
                health = 'healthy',
                latest_error_kind = NULL,
                latest_diagnostic = NULL
             WHERE id = ?1",
            params![company.id, timestamp(completed_at)],
        )?,
        "failed" | "incomplete" => tx.execute(
            "UPDATE companies SET
                latest_attempted_at = ?2,
                health = ?3,
                latest_error_kind = ?4,
                latest_diagnostic = ?5
             WHERE id = ?1",
            params![
                company.id,
                timestamp(completed_at),
                outcome,
                error_kind,
                diagnostic
            ],
        )?,
        _ => unreachable!("scan outcomes are fixed by Store methods"),
    };
    Ok(())
}

fn job_record_from_row(row: &Row<'_>) -> Result<JobRecord> {
    let published_at = optional_timestamp(row, 12)?;
    Ok(JobRecord {
        key: JobKey::new(row.get::<_, String>(0)?, row.get::<_, String>(1)?),
        classified: ClassifiedJob {
            observed: ObservedJob {
                source_id: row.get(1)?,
                title: row.get(2)?,
                department: row.get(3)?,
                team: row.get(4)?,
                employment_type: row.get(5)?,
                locations: json_from_row(row, 6)?,
                countries: json_from_row(row, 7)?,
                job_url: row.get(8)?,
                apply_url: row.get(9)?,
                description: row.get(10)?,
                raw_payload: json_from_row(row, 11)?,
                published_at,
            },
            eligibility: Eligibility {
                eligible: row.get(13)?,
                reason: row.get(14)?,
            },
        },
        source_open: row.get(15)?,
        is_new: row.get(16)?,
        first_seen_at: required_timestamp(row, 17)?,
        last_seen_at: required_timestamp(row, 18)?,
        closed_at: optional_timestamp(row, 19)?,
        reopened_at: optional_timestamp(row, 20)?,
        applied_at: optional_timestamp(row, 21)?,
    })
}

fn scan_read_model_from_row(row: &Row<'_>) -> Result<ScanReadModel> {
    let outcome = match row.get::<_, String>(4)?.as_str() {
        "complete" => ScanOutcome::Complete,
        "incomplete" => ScanOutcome::Incomplete,
        "failed" => ScanOutcome::Failed,
        value => return Err(invalid_text(4, "scan outcome", value)),
    };
    Ok(ScanReadModel {
        run_id: row.get(0)?,
        company_id: row.get(1)?,
        company_name: row.get(2)?,
        completed_at: required_timestamp(row, 3)?,
        outcome,
        observed_count: usize_from_row(row, 5)?,
        error_kind: optional_error_kind(row, 6)?,
        diagnostic: row.get(7)?,
    })
}

fn source_read_model_from_row(row: &Row<'_>) -> Result<SourceReadModel> {
    let health = match row.get::<_, String>(5)?.as_str() {
        "unknown" => SourceHealth::Unknown,
        "healthy" => SourceHealth::Healthy,
        "incomplete" => SourceHealth::Incomplete,
        "failed" => SourceHealth::Failed,
        value => return Err(invalid_text(5, "source health", value)),
    };
    Ok(SourceReadModel {
        company_id: row.get(0)?,
        company_name: row.get(1)?,
        enabled: row.get(2)?,
        latest_attempted_at: optional_timestamp(row, 3)?,
        latest_successful_at: optional_timestamp(row, 4)?,
        health,
        latest_error_kind: optional_error_kind(row, 6)?,
        diagnostic: row.get(7)?,
    })
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn required_timestamp(row: &Row<'_>, index: usize) -> Result<DateTime<Utc>> {
    let value = row.get::<_, String>(index)?;
    parse_timestamp(index, value)
}

fn optional_timestamp(row: &Row<'_>, index: usize) -> Result<Option<DateTime<Utc>>> {
    row.get::<_, Option<String>>(index)?
        .map(|value| parse_timestamp(index, value))
        .transpose()
}

fn parse_timestamp(index: usize, value: String) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(&value)
        .map(|parsed| parsed.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
        })
}

fn usize_from_row(row: &Row<'_>, index: usize) -> Result<usize> {
    let value = row.get::<_, i64>(index)?;
    usize::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Integer, Box::new(error))
    })
}

fn optional_error_kind(row: &Row<'_>, index: usize) -> Result<Option<SourceErrorKind>> {
    row.get::<_, Option<String>>(index)?
        .map(|value| {
            SourceErrorKind::from_str(&value)
                .ok_or_else(|| invalid_text(index, "source error kind", &value))
        })
        .transpose()
}

fn invalid_text(index: usize, field: &str, value: &str) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        index,
        Type::Text,
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("unknown {field} `{value}`"),
        )
        .into(),
    )
}

fn json_from_row<T: serde::de::DeserializeOwned>(row: &Row<'_>, index: usize) -> Result<T> {
    let value = row.get::<_, String>(index)?;
    serde_json::from_str(&value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

fn parse_json_column<T: serde::de::DeserializeOwned>(index: usize, value: &str) -> Result<T> {
    serde_json::from_str(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(index, Type::Text, Box::new(error))
    })
}

fn json_text<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("serializing strings to JSON cannot fail")
}

fn content_hash(observed: &ObservedJob) -> String {
    let mut locations = observed
        .locations
        .iter()
        .map(|location| normalize_text(location))
        .collect::<Vec<_>>();
    locations.sort();
    locations.dedup();
    let normalized = json!({
        "title": normalize_text(&observed.title),
        "metadata": normalized_metadata(observed),
        "locations": locations,
        "job_url": observed.job_url.trim(),
        "apply_url": observed.apply_url.trim(),
        "description": normalize_text(&observed.description),
    });
    Sha256::digest(normalized.to_string().as_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut encoded, byte| {
            write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
            encoded
        })
}

fn metadata_json(observed: &ObservedJob) -> String {
    normalized_metadata(observed).to_string()
}

fn normalized_metadata(observed: &ObservedJob) -> serde_json::Value {
    let mut countries = observed.countries.clone();
    countries.sort();
    countries.dedup();
    json!({
        "department": observed.department.as_deref().map(normalize_text),
        "team": observed.team.as_deref().map(normalize_text),
        "employment_type": observed.employment_type.as_deref().map(normalize_text),
        "countries": countries,
        "published_at": observed.published_at.map(timestamp),
    })
}

fn normalize_text(value: &str) -> String {
    value.replace("\r\n", "\n").trim().to_owned()
}
