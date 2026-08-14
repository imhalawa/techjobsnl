use rusqlite::{Connection, Result};

pub(super) fn migrate(connection: &Connection) -> Result<()> {
    connection.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS companies (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            enabled INTEGER NOT NULL CHECK (enabled IN (0, 1)),
            latest_attempted_at TEXT,
            latest_successful_at TEXT,
            health TEXT NOT NULL DEFAULT 'unknown',
            latest_error_kind TEXT,
            latest_diagnostic TEXT
        );

        CREATE TABLE IF NOT EXISTS scans (
            id INTEGER PRIMARY KEY,
            run_id TEXT NOT NULL,
            company_id TEXT NOT NULL REFERENCES companies(id),
            started_at TEXT NOT NULL,
            completed_at TEXT NOT NULL,
            outcome TEXT NOT NULL,
            observed_count INTEGER NOT NULL,
            error_kind TEXT,
            diagnostic TEXT
        );

        CREATE TABLE IF NOT EXISTS jobs (
            company_id TEXT NOT NULL REFERENCES companies(id),
            source_id TEXT NOT NULL,
            title TEXT NOT NULL,
            department TEXT,
            team TEXT,
            employment_type TEXT,
            locations_json TEXT NOT NULL,
            countries_json TEXT NOT NULL,
            job_url TEXT NOT NULL,
            apply_url TEXT NOT NULL,
            description TEXT NOT NULL,
            published_at TEXT,
            raw_payload TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            eligible INTEGER NOT NULL CHECK (eligible IN (0, 1)),
            eligibility_reason TEXT NOT NULL,
            source_open INTEGER NOT NULL CHECK (source_open IN (0, 1)),
            is_new INTEGER NOT NULL CHECK (is_new IN (0, 1)),
            first_seen_at TEXT NOT NULL,
            last_seen_at TEXT NOT NULL,
            closed_at TEXT,
            reopened_at TEXT,
            applied_at TEXT,
            PRIMARY KEY (company_id, source_id)
        );

        CREATE TABLE IF NOT EXISTS job_snapshots (
            company_id TEXT NOT NULL REFERENCES companies(id),
            source_id TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            captured_at TEXT NOT NULL,
            title TEXT NOT NULL,
            metadata_json TEXT NOT NULL,
            locations_json TEXT NOT NULL,
            job_url TEXT NOT NULL,
            apply_url TEXT NOT NULL,
            description TEXT NOT NULL,
            raw_payload TEXT NOT NULL,
            PRIMARY KEY (company_id, source_id, content_hash),
            FOREIGN KEY (company_id, source_id) REFERENCES jobs(company_id, source_id)
        );

        CREATE TABLE IF NOT EXISTS job_analytics (
            company_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            extractor_version TEXT NOT NULL,
            facts_json TEXT NOT NULL,
            PRIMARY KEY (company_id, source_id, content_hash, extractor_version),
            FOREIGN KEY (company_id, source_id, content_hash)
                REFERENCES job_snapshots(company_id, source_id, content_hash)
        );
        "#,
    )
}
