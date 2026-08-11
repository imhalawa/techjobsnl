use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct JobKey {
    pub company_id: String,
    pub source_id: String,
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
    pub first_seen_at: DateTime<Utc>,
    pub last_seen_at: DateTime<Utc>,
    pub applied_at: Option<DateTime<Utc>>,
}
