use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError, country_code_for_location,
    http::send_text,
    json_ld::{html_markdown, html_text},
};

const BOARD_ENDPOINT: &str = "https://boards-api.greenhouse.io/v1/boards";

pub struct GreenhouseSource {
    company_id: String,
    board: String,
    country_filter: Option<String>,
    client: Client,
}

impl GreenhouseSource {
    pub fn new(company_id: impl Into<String>, board: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            board: board.into(),
            country_filter: None,
            client,
        }
    }

    pub fn with_country_filter(mut self, country_filter: Option<&str>) -> Self {
        self.country_filter = country_filter.map(str::to_owned);
        self
    }
}

#[async_trait::async_trait]
impl JobSource for GreenhouseSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let raw = send_text(self.client.get(board_url(&self.board)), "Greenhouse").await?;
        let observations = match self.country_filter.as_deref() {
            Some(country) => {
                parse_greenhouse_response_for_country(&self.company_id, &raw, country)?
            }
            None => parse_greenhouse_response(&self.company_id, &raw)?,
        };
        Ok(SourceScan::Complete { observations })
    }
}

pub fn parse_greenhouse_response(
    company_id: &str,
    raw: &str,
) -> Result<Vec<ObservedJob>, SourceError> {
    parse_greenhouse_response_with_country(company_id, raw, None)
}

pub fn parse_greenhouse_response_for_country(
    company_id: &str,
    raw: &str,
    country: &str,
) -> Result<Vec<ObservedJob>, SourceError> {
    parse_greenhouse_response_with_country(company_id, raw, Some(country))
}

fn parse_greenhouse_response_with_country(
    company_id: &str,
    raw: &str,
    country_filter: Option<&str>,
) -> Result<Vec<ObservedJob>, SourceError> {
    let response: GreenhouseResponse = serde_json::from_str(raw).map_err(|error| {
        SourceError::schema(format!(
            "invalid Greenhouse response for {company_id}: {error}"
        ))
    })?;
    let jobs = response.jobs.ok_or_else(|| {
        SourceError::schema(format!(
            "Greenhouse response for {company_id} is missing jobs"
        ))
    })?;
    let total = response.meta.map(|meta| meta.total).ok_or_else(|| {
        SourceError::schema(format!(
            "Greenhouse response for {company_id} is missing total metadata"
        ))
    })?;
    if jobs.len() != total {
        return Err(SourceError::schema(format!(
            "Greenhouse response for {company_id} declared {total} jobs but returned {}",
            jobs.len()
        )));
    }

    let mut ids = HashSet::new();
    let mut observations = Vec::new();
    for raw_job in jobs {
        let job: GreenhouseJob = serde_json::from_value(raw_job.clone()).map_err(|error| {
            SourceError::schema(format!("invalid Greenhouse job for {company_id}: {error}"))
        })?;
        let id = job.id.ok_or_else(|| {
            SourceError::schema(format!("Greenhouse job for {company_id} is missing id"))
        })?;
        if !ids.insert(id) {
            return Err(SourceError::schema(format!(
                "duplicate Greenhouse job {id} for {company_id}"
            )));
        }
        if country_filter.is_some_and(|country| !job_has_country(&job, country)) {
            continue;
        }
        observations.push(observed_job(company_id, job, raw_job, country_filter)?);
    }
    Ok(observations)
}

fn job_has_country(job: &GreenhouseJob, country: &str) -> bool {
    job.offices.as_deref().is_some_and(|offices| {
        offices.iter().any(|office| {
            country_code_for_location(&office.name).or_else(|| {
                office
                    .location
                    .as_deref()
                    .and_then(country_code_for_location)
            }) == Some(country)
        })
    })
}

fn board_url(board: &str) -> Url {
    let mut url = Url::parse(BOARD_ENDPOINT).expect("Greenhouse endpoint constant must be valid");
    url.path_segments_mut()
        .expect("Greenhouse endpoint must be a base URL")
        .push(board)
        .push("jobs");
    url.query_pairs_mut().append_pair("content", "true");
    url
}

fn observed_job(
    company_id: &str,
    job: GreenhouseJob,
    raw_payload: serde_json::Value,
    country_filter: Option<&str>,
) -> Result<ObservedJob, SourceError> {
    let id = job.id.ok_or_else(|| {
        SourceError::schema(format!("Greenhouse job for {company_id} is missing id"))
    })?;
    required(&job.title, "title", id, company_id)?;
    required(&job.absolute_url, "official URL", id, company_id)?;

    let offices = job
        .offices
        .as_ref()
        .filter(|offices| !offices.is_empty())
        .ok_or_else(|| {
            SourceError::schema(format!(
                "Greenhouse job {id} for {company_id} has no offices"
            ))
        })?;
    let mut locations = Vec::new();
    let mut countries = Vec::new();
    for office in offices {
        let location = office
            .location
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or(&office.name);
        let country =
            country_code_for_location(&office.name).or_else(|| country_code_for_location(location));
        let Some(country) = country else {
            if country_filter.is_some() {
                continue;
            }
            return Err(SourceError::schema(format!(
                "Greenhouse job {id} for {company_id} has unresolved office {:?}",
                office.name
            )));
        };
        if country_filter.is_some_and(|filter| country != filter) {
            continue;
        }
        push_unique(&mut locations, location.trim().to_owned());
        push_unique(&mut countries, country.to_owned());
    }

    let description = html_markdown(&html_text(&job.content));
    if description.is_empty() {
        return Err(SourceError::schema(format!(
            "Greenhouse job {id} for {company_id} has an empty description"
        )));
    }
    let published_at = DateTime::parse_from_rfc3339(&job.updated_at)
        .map_err(|error| {
            SourceError::schema(format!(
                "Greenhouse job {id} for {company_id} has an invalid updated_at: {error}"
            ))
        })?
        .with_timezone(&Utc);
    let department = joined_names(job.departments.as_deref().unwrap_or_default());
    Ok(ObservedJob {
        source_id: id.to_string(),
        title: job.title,
        department,
        team: None,
        employment_type: None,
        locations,
        countries,
        job_url: job.absolute_url.clone(),
        apply_url: job.absolute_url,
        description,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn required(value: &str, field: &str, id: u64, company_id: &str) -> Result<(), SourceError> {
    if value.trim().is_empty() {
        return Err(SourceError::schema(format!(
            "Greenhouse job {id} for {company_id} has an empty {field}"
        )));
    }
    Ok(())
}

fn joined_names(values: &[GreenhouseName]) -> Option<String> {
    let names = values
        .iter()
        .map(|value| value.name.trim())
        .filter(|name| !name.is_empty())
        .collect::<Vec<_>>();
    (!names.is_empty()).then(|| names.join(" / "))
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[derive(Deserialize)]
struct GreenhouseResponse {
    jobs: Option<Vec<serde_json::Value>>,
    meta: Option<GreenhouseMeta>,
}

#[derive(Deserialize)]
struct GreenhouseMeta {
    total: usize,
}

#[derive(Deserialize)]
struct GreenhouseJob {
    id: Option<u64>,
    title: String,
    updated_at: String,
    absolute_url: String,
    content: String,
    offices: Option<Vec<GreenhouseOffice>>,
    departments: Option<Vec<GreenhouseName>>,
}

#[derive(Deserialize)]
struct GreenhouseOffice {
    name: String,
    location: Option<String>,
}

#[derive(Deserialize)]
struct GreenhouseName {
    name: String,
}
