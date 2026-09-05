use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct JobKey {
    pub company_id: String,
    pub source_id: String,
}

impl JobKey {
    pub fn new(company_id: impl Into<String>, source_id: impl Into<String>) -> Self {
        Self {
            company_id: company_id.into(),
            source_id: source_id.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObservedJob {
    pub source_id: String,
    pub title: String,
    pub department: Option<String>,
    pub team: Option<String>,
    pub employment_type: Option<String>,
    pub locations: Vec<String>,
    pub countries: Vec<String>,
    pub job_url: String,
    pub apply_url: String,
    pub description: String,
    pub raw_payload: serde_json::Value,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassifiedJob {
    pub observed: ObservedJob,
    pub eligibility: Eligibility,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Eligibility {
    pub eligible: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRecord {
    pub key: JobKey,
    pub classified: ClassifiedJob,
    pub source_open: bool,
    pub is_new: bool,
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub reopened_at: Option<DateTime<Utc>>,
    pub applied_at: Option<DateTime<Utc>>,
}
