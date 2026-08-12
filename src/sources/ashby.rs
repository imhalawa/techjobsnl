use std::{collections::HashSet, time::Duration};

use chrono::{DateTime, Utc};
use reqwest::{Client, Url, redirect::Policy};
use serde::{Deserialize, Serialize};

use crate::domain::{ObservedJob, SourceErrorKind, SourceScan};

use super::{JobSource, SourceError, http::send_text};

const ASHBY_BOARD_ENDPOINT: &str = "https://api.ashbyhq.com/posting-api/job-board";
const REDIRECT_LIMIT: usize = 5;

pub struct AshbySource {
    company_id: String,
    board: String,
    client: Client,
}

impl AshbySource {
    pub fn new(company_id: impl Into<String>, board: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            board: board.into(),
            client,
        }
    }
}

pub fn build_client(user_agent: &str, timeout: Duration) -> Result<Client, SourceError> {
    Client::builder()
        .user_agent(user_agent)
        .timeout(timeout)
        .cookie_store(true)
        .redirect(Policy::limited(REDIRECT_LIMIT))
        .build()
        .map_err(|error| SourceError {
            kind: SourceErrorKind::Configuration,
            message: format!("could not configure HTTP client: {error}"),
            http_status: None,
            retry_after: None,
            retryable: false,
        })
}

#[async_trait::async_trait]
impl JobSource for AshbySource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let raw_json = send_text(self.client.get(board_url(&self.board)), "Ashby").await?;
        let observations = parse_ashby_response(&self.company_id, &raw_json)?;
        Ok(SourceScan::Complete { observations })
    }
}

fn board_url(board: &str) -> Url {
    let mut url = Url::parse(ASHBY_BOARD_ENDPOINT).expect("Ashby endpoint constant must be valid");
    url.path_segments_mut()
        .expect("Ashby endpoint must be a base URL")
        .push(board);
    url
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AshbyResponse {
    jobs: Option<Vec<AshbyJob>>,
    api_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AshbyJob {
    id: String,
    title: String,
    department: Option<String>,
    team: Option<String>,
    employment_type: Option<String>,
    location: String,
    secondary_locations: Vec<AshbyLocation>,
    published_at: Option<String>,
    is_listed: bool,
    address: AshbyAddress,
    job_url: String,
    apply_url: String,
    description_plain: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AshbyLocation {
    location: String,
    address: AshbyAddress,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AshbyAddress {
    #[serde(rename = "postalAddress")]
    postal_address: AshbyPostalAddress,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AshbyPostalAddress {
    address_country: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

pub fn parse_ashby_response(
    company_id: &str,
    raw_json: &str,
) -> Result<Vec<ObservedJob>, SourceError> {
    let response: AshbyResponse = serde_json::from_str(raw_json).map_err(|error| {
        SourceError::schema(format!("invalid Ashby response for {company_id}: {error}"))
    })?;
    if response.api_version != "1" {
        return Err(SourceError::schema(format!(
            "unsupported Ashby API version {:?} for {company_id}",
            response.api_version
        )));
    }
    let jobs = response.jobs.ok_or_else(|| {
        SourceError::schema(format!("Ashby response for {company_id} is missing jobs"))
    })?;

    jobs.into_iter()
        .filter(|job| job.is_listed)
        .map(|job| observed_job(company_id, job))
        .collect()
}

fn observed_job(company_id: &str, job: AshbyJob) -> Result<ObservedJob, SourceError> {
    if job.id.trim().is_empty() {
        return Err(SourceError::schema(format!(
            "Ashby job for {company_id} has an empty id"
        )));
    }
    if job.job_url.trim().is_empty() {
        return Err(SourceError::schema(format!(
            "Ashby job {} for {company_id} has an empty official URL",
            job.id
        )));
    }
    if job.apply_url.trim().is_empty() {
        return Err(SourceError::schema(format!(
            "Ashby job {} for {company_id} has an empty apply URL",
            job.id
        )));
    }
    let published_at = job
        .published_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|error| {
            SourceError::schema(format!(
                "Ashby job {} for {company_id} has an invalid publication time: {error}",
                job.id
            ))
        })?
        .map(|published_at| published_at.with_timezone(&Utc));

    let mut locations = Vec::with_capacity(1 + job.secondary_locations.len());
    let mut countries = Vec::with_capacity(1 + job.secondary_locations.len());
    let mut seen_locations = HashSet::new();
    let mut seen_countries = HashSet::new();
    push_unique(&mut locations, &mut seen_locations, job.location.clone());
    push_unique(
        &mut countries,
        &mut seen_countries,
        normalise_country(&job.address.postal_address.address_country),
    );
    for secondary in &job.secondary_locations {
        push_unique(
            &mut locations,
            &mut seen_locations,
            secondary.location.clone(),
        );
        push_unique(
            &mut countries,
            &mut seen_countries,
            normalise_country(&secondary.address.postal_address.address_country),
        );
    }

    let raw_payload = serde_json::to_value(&job).map_err(|error| {
        SourceError::schema(format!(
            "could not preserve Ashby job {} for {company_id}: {error}",
            job.id
        ))
    })?;

    Ok(ObservedJob {
        source_id: job.id,
        title: job.title,
        department: job.department,
        team: job.team,
        employment_type: job.employment_type,
        locations,
        countries,
        job_url: job.job_url,
        apply_url: job.apply_url,
        description: job.description_plain,
        raw_payload,
        published_at,
    })
}

fn normalise_country(country: &str) -> String {
    match country {
        "Netherlands" => "NL".to_owned(),
        country => country.to_owned(),
    }
}

fn push_unique(values: &mut Vec<String>, seen: &mut HashSet<String>, value: String) {
    if seen.insert(value.clone()) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::board_url;

    #[test]
    fn encodes_board_as_one_url_path_segment() {
        let url = board_url("mollie/preview?region=nl#jobs");

        assert_eq!(
            url.as_str(),
            "https://api.ashbyhq.com/posting-api/job-board/mollie%2Fpreview%3Fregion=nl%23jobs"
        );
    }
}
