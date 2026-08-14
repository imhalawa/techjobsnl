use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use reqwest::{Client, Url, header::USER_AGENT};
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::json;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text};

const PAGE_SIZE: usize = 100;

pub struct RabobankSource {
    company_id: String,
    base_url: String,
    country: String,
    client: Client,
}

impl RabobankSource {
    pub fn new(
        company_id: impl Into<String>,
        base_url: impl Into<String>,
        country: impl Into<String>,
        client: Client,
    ) -> Self {
        let country = country.into().to_ascii_lowercase();
        Self {
            company_id: company_id.into(),
            base_url: base_url.into(),
            country,
            client,
        }
    }

    async fn page(&self, page: usize) -> Result<String, SourceError> {
        let endpoint = endpoint(&self.base_url, &self.company_id)?;
        send_text(
            self.client
                .post(endpoint)
                .header(USER_AGENT, "curl/8.0")
                .json(&json!({
                    "filters": {},
                    "language": "nl",
                    "page": page,
                    "size": PAGE_SIZE,
                })),
            "Rabobank",
        )
        .await
    }

    async fn sitemap(&self) -> Result<String, SourceError> {
        let mut endpoint = official_base(&self.base_url, &self.company_id)?;
        endpoint.set_path("/api/sitemap/");
        send_text(
            self.client.get(endpoint).header(USER_AGENT, "curl/8.0"),
            "Rabobank",
        )
        .await
    }
}

#[async_trait::async_trait]
impl JobSource for RabobankSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let first = self.page(1).await?;
        let total = response_meta(&first, &self.company_id)?.total.value;
        let page_count = total.max(1).div_ceil(PAGE_SIZE);
        let mut pages = vec![first];
        for page in 2..=page_count {
            pages.push(self.page(page).await?);
        }
        let sitemap = self.sitemap().await?;
        let refs = pages.iter().map(String::as_str).collect::<Vec<_>>();
        Ok(SourceScan::Complete {
            observations: parse_rabobank_pages(
                &self.company_id,
                &self.base_url,
                &self.country,
                &refs,
                &sitemap,
            )?,
        })
    }
}

pub fn parse_rabobank_pages(
    company_id: &str,
    base_url: &str,
    country: &str,
    pages: &[&str],
    sitemap: &str,
) -> Result<Vec<ObservedJob>, SourceError> {
    let base = official_base(base_url, company_id)?;
    let urls = parse_sitemap(company_id, &base, sitemap)?;
    let mut expected_total = None;
    let mut ids = HashSet::new();
    let mut observations = Vec::new();

    for (index, raw) in pages.iter().enumerate() {
        let response: RabobankResponse = serde_json::from_str(raw)
            .map_err(|error| schema(company_id, format!("invalid page {}: {error}", index + 1)))?;
        if response.hits.total.relation != "eq" {
            return Err(schema(company_id, "total relation is not exact"));
        }
        if expected_total
            .replace(response.hits.total.value)
            .is_some_and(|total| total != response.hits.total.value)
        {
            return Err(schema(company_id, "total changed between pages"));
        }
        for hit in response.hits.hits {
            let job_url = urls.get(&hit.id).ok_or_else(|| {
                schema(
                    company_id,
                    format!("job {} is missing from sitemap", hit.id),
                )
            })?;
            let observation = observed_job(company_id, job_url, hit)?;
            if !ids.insert(observation.source_id.clone()) {
                return Err(schema(
                    company_id,
                    format!("duplicate job {}", observation.source_id),
                ));
            }
            observations.push(observation);
        }
    }

    let total = expected_total.ok_or_else(|| schema(company_id, "returned no pages"))?;
    let expected_pages = total.max(1).div_ceil(PAGE_SIZE);
    if pages.len() != expected_pages || observations.len() != total {
        return Err(schema(
            company_id,
            format!(
                "incomplete result: expected {total} jobs across {expected_pages} pages, got {} jobs across {} pages",
                observations.len(),
                pages.len()
            ),
        ));
    }
    if urls.len() != total || urls.keys().any(|id| !ids.contains(id)) {
        return Err(schema(
            company_id,
            format!(
                "sitemap mismatch: expected {total} jobs, got {} Dutch vacancy URLs",
                urls.len()
            ),
        ));
    }
    Ok(observations
        .into_iter()
        .filter(|job| job.countries.contains(&country.to_ascii_uppercase()))
        .collect())
}

fn response_meta(raw: &str, company_id: &str) -> Result<RabobankHits, SourceError> {
    serde_json::from_str::<RabobankResponse>(raw)
        .map(|response| response.hits)
        .map_err(|error| schema(company_id, format!("invalid first page: {error}")))
}

fn observed_job(
    company_id: &str,
    job_url: &Url,
    hit: RabobankHit,
) -> Result<ObservedJob, SourceError> {
    let job = hit.source;
    required(&job.job_id, "job_id", company_id)?;
    required(&job.job_title, "job_title", company_id)?;
    required(&job.city, "city", company_id)?;
    required(
        &job.job_description_plain,
        "job_description_plain",
        company_id,
    )?;
    if hit.id != job.job_id {
        return Err(schema(company_id, "hit and job IDs differ"));
    }
    if job.status != "open" {
        return Err(schema(
            company_id,
            format!("job {} is not open", job.job_id),
        ));
    }
    let countries = job
        .country
        .iter()
        .map(|country| country.code.as_str())
        .collect::<Vec<_>>();
    if countries.is_empty() {
        return Err(schema(
            company_id,
            format!("job {} has no country", job.job_id),
        ));
    }
    let published_at = DateTime::<Utc>::from_timestamp_millis(job.date_start).ok_or_else(|| {
        schema(
            company_id,
            format!("job {} has invalid date_start", job.job_id),
        )
    })?;
    let raw_payload = serde_json::to_value(&job)
        .map_err(|error| schema(company_id, format!("could not preserve job: {error}")))?;

    Ok(ObservedJob {
        source_id: job.job_id,
        title: job.job_title,
        department: first_label(&job.job_branch),
        team: None,
        employment_type: first_label(&job.contract_type),
        locations: vec![job.city],
        countries: countries
            .iter()
            .map(|country| country.to_uppercase())
            .collect(),
        job_url: job_url.to_string(),
        apply_url: job_url.to_string(),
        description: job.job_description_plain,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn parse_sitemap(
    company_id: &str,
    base: &Url,
    raw: &str,
) -> Result<HashMap<String, Url>, SourceError> {
    let selector = Selector::parse("loc").expect("static selector must parse");
    let mut urls = HashMap::new();
    for location in Html::parse_document(raw).select(&selector) {
        let value = location.text().collect::<String>();
        let url = Url::parse(value.trim())
            .map_err(|error| schema(company_id, format!("invalid sitemap URL: {error}")))?;
        if url.host_str() != base.host_str() || !url.path().starts_with("/nl/vacature/") {
            continue;
        }
        let id = url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|part| !part.is_empty()))
            .ok_or_else(|| schema(company_id, "vacancy sitemap URL has no job ID"))?
            .to_owned();
        if urls.insert(id.clone(), url).is_some() {
            return Err(schema(company_id, format!("duplicate sitemap job {id}")));
        }
    }
    Ok(urls)
}

fn first_label(values: &[RabobankLabel]) -> Option<String> {
    values.first().and_then(|value| {
        let label = value.label_nl.trim();
        (!label.is_empty()).then(|| label.to_owned())
    })
}

fn required(value: &str, field: &str, company_id: &str) -> Result<(), SourceError> {
    if value.trim().is_empty() {
        return Err(schema(company_id, format!("job has empty {field}")));
    }
    Ok(())
}

fn endpoint(base_url: &str, company_id: &str) -> Result<Url, SourceError> {
    let mut url = official_base(base_url, company_id)?;
    url.set_path("/api/v1/jobs/");
    Ok(url)
}

fn official_base(value: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid base URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some("rabobank.jobs")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(schema(
            company_id,
            "base URL is not official Rabobank HTTPS",
        ));
    }
    Ok(url)
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("Rabobank response for {company_id}: {message}"))
}

#[derive(Deserialize)]
struct RabobankResponse {
    hits: RabobankHits,
}

#[derive(Deserialize)]
struct RabobankHits {
    total: RabobankTotal,
    hits: Vec<RabobankHit>,
}

#[derive(Deserialize)]
struct RabobankTotal {
    value: usize,
    relation: String,
}

#[derive(Deserialize)]
struct RabobankHit {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "_source")]
    source: RabobankJob,
}

#[derive(Deserialize, serde::Serialize)]
struct RabobankJob {
    job_id: String,
    job_title: String,
    city: String,
    country: Vec<RabobankCountry>,
    date_start: i64,
    job_description_plain: String,
    status: String,
    #[serde(default)]
    contract_type: Vec<RabobankLabel>,
    #[serde(default)]
    job_branch: Vec<RabobankLabel>,
}

#[derive(Deserialize, serde::Serialize)]
struct RabobankCountry {
    code: String,
}

#[derive(Deserialize, serde::Serialize)]
struct RabobankLabel {
    #[serde(default)]
    label_nl: String,
}
