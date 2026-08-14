use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

pub struct WorkableSource {
    company_id: String,
    account: String,
    country_filter: Option<String>,
    client: Client,
}

impl WorkableSource {
    pub fn new(company_id: impl Into<String>, account: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            account: account.into(),
            country_filter: None,
            client,
        }
    }

    pub fn with_country_filter(mut self, country: Option<&str>) -> Self {
        self.country_filter = country.map(str::to_owned);
        self
    }
}

#[async_trait::async_trait]
impl JobSource for WorkableSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let endpoint = format!(
            "https://apply.workable.com/api/v3/accounts/{}/jobs",
            self.account
        );
        let mut token = None;
        let mut seen_tokens = HashSet::new();
        let mut listings = Vec::new();
        let mut expected_total = None;

        loop {
            let body = token
                .as_ref()
                .map(|token| serde_json::json!({ "token": token }))
                .unwrap_or_else(|| serde_json::json!({}));
            let raw = send_text(self.client.post(&endpoint).json(&body), "Workable").await?;
            let page: WorkablePage = serde_json::from_str(&raw).map_err(|error| {
                SourceError::schema(format!(
                    "invalid Workable page for {}: {error}",
                    self.company_id
                ))
            })?;
            if expected_total
                .replace(page.total)
                .is_some_and(|total| total != page.total)
            {
                return Err(SourceError::schema(format!(
                    "Workable total changed while scanning {}",
                    self.company_id
                )));
            }
            listings.extend(page.results);
            token = page.next_page;
            match token.as_ref() {
                Some(token) if !seen_tokens.insert(token.clone()) => {
                    return Err(SourceError::schema(format!(
                        "Workable pagination repeated for {}",
                        self.company_id
                    )));
                }
                Some(_) => {}
                None => break,
            }
        }

        let mut ids = HashSet::new();
        if listings.len() != expected_total.unwrap_or_default()
            || listings.iter().any(|job| !ids.insert(job.id))
        {
            return Err(SourceError::schema(format!(
                "incomplete or duplicate Workable board for {}",
                self.company_id
            )));
        }

        let mut jobs = Vec::new();
        for listing in listings {
            if self.country_filter.as_ref().is_some_and(|country| {
                !listing
                    .locations
                    .iter()
                    .any(|location| location.country_code == *country)
            }) {
                continue;
            }
            let detail_url = format!(
                "https://apply.workable.com/api/v2/accounts/{}/jobs/{}",
                self.account, listing.shortcode
            );
            let raw = send_text(self.client.get(detail_url), "Workable job").await?;
            let raw_payload: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
                SourceError::schema(format!(
                    "invalid Workable job {} for {}: {error}",
                    listing.id, self.company_id
                ))
            })?;
            let job = parse_workable_job(&self.company_id, &self.account, raw_payload)?;
            if job.source_id != listing.id.to_string() {
                return Err(SourceError::schema(format!(
                    "Workable listing/detail mismatch for {}",
                    self.company_id
                )));
            }
            if self
                .country_filter
                .as_ref()
                .is_some_and(|country| !job.countries.contains(country))
            {
                return Err(SourceError::schema(format!(
                    "Workable listing/detail location mismatch for {}",
                    self.company_id
                )));
            }
            jobs.push(job);
        }

        Ok(SourceScan::Complete { observations: jobs })
    }
}

pub fn parse_workable_job(
    company_id: &str,
    account: &str,
    raw_payload: serde_json::Value,
) -> Result<ObservedJob, SourceError> {
    let job: WorkableJob = serde_json::from_value(raw_payload.clone()).map_err(|error| {
        SourceError::schema(format!("invalid Workable job for {company_id}: {error}"))
    })?;
    let locations = job
        .locations
        .iter()
        .map(|location| {
            [
                Some(location.city.as_str()),
                location.region.as_deref(),
                Some(location.country.as_str()),
            ]
            .into_iter()
            .flatten()
            .filter(|part| !part.trim().is_empty())
            .collect::<Vec<_>>()
            .join(", ")
        })
        .collect::<Vec<_>>();
    let countries = job
        .locations
        .iter()
        .map(|location| location.country_code.clone())
        .collect::<Vec<_>>();
    let description = html_markdown(&format!(
        "{}<h2>Requirements</h2>{}<h2>Benefits</h2>{}",
        job.description,
        job.requirements.unwrap_or_default(),
        job.benefits.unwrap_or_default()
    ));
    if job.title.trim().is_empty()
        || job.shortcode.trim().is_empty()
        || locations.is_empty()
        || countries.iter().any(|country| country.trim().is_empty())
        || description.is_empty()
    {
        return Err(SourceError::schema(format!(
            "Workable job {} for {company_id} is missing required data",
            job.id
        )));
    }
    let published_at = DateTime::parse_from_rfc3339(&job.published)
        .map_err(|error| {
            SourceError::schema(format!(
                "Workable job {} for {company_id} has invalid published date: {error}",
                job.id
            ))
        })?
        .with_timezone(&Utc);
    let job_url = format!("https://apply.workable.com/{account}/j/{}/", job.shortcode);

    Ok(ObservedJob {
        source_id: job.id.to_string(),
        title: job.title,
        department: non_empty(job.department.join(", ")),
        team: None,
        employment_type: non_empty(job.employment_type),
        locations,
        countries,
        apply_url: format!("{job_url}apply/"),
        job_url,
        description,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn non_empty(value: String) -> Option<String> {
    (!value.trim().is_empty()).then_some(value)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkablePage {
    total: usize,
    results: Vec<WorkableListing>,
    next_page: Option<String>,
}

#[derive(Deserialize)]
struct WorkableListing {
    id: u64,
    shortcode: String,
    #[serde(default)]
    locations: Vec<WorkableLocation>,
}

#[derive(Deserialize)]
struct WorkableJob {
    id: u64,
    shortcode: String,
    title: String,
    #[serde(default)]
    locations: Vec<WorkableLocation>,
    published: String,
    #[serde(rename = "type", default)]
    employment_type: String,
    #[serde(default)]
    department: Vec<String>,
    description: String,
    requirements: Option<String>,
    benefits: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkableLocation {
    country: String,
    country_code: String,
    city: String,
    region: Option<String>,
}
