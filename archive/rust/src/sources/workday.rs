use std::collections::HashSet;

use chrono::NaiveDate;
use reqwest::Client;
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

const PAGE_SIZE: usize = 20;

pub struct WorkdaySource {
    company_id: String,
    base_url: String,
    tenant: String,
    site: String,
    country: String,
    country_code: String,
    client: Client,
}

impl WorkdaySource {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        company_id: impl Into<String>,
        base_url: impl Into<String>,
        tenant: impl Into<String>,
        site: impl Into<String>,
        country: impl Into<String>,
        country_code: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            company_id: company_id.into(),
            base_url: base_url.into(),
            tenant: tenant.into(),
            site: site.into(),
            country: country.into(),
            country_code: country_code.into(),
            client,
        }
    }

    fn endpoint(&self) -> String {
        format!(
            "{}/wday/cxs/{}/{}/jobs",
            self.base_url, self.tenant, self.site
        )
    }
}

#[async_trait::async_trait]
impl JobSource for WorkdaySource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let endpoint = self.endpoint();
        let discovery = fetch_page(&self.client, &endpoint, None, 0).await?;
        let facet_id = find_country_facet(&discovery, &self.country).ok_or_else(|| {
            SourceError::schema(format!(
                "Workday country facet {:?} is missing for {}",
                self.country, self.company_id
            ))
        })?;

        let mut offset = 0;
        let mut expected_total = None;
        let mut listings = Vec::new();
        loop {
            let raw = fetch_page(&self.client, &endpoint, Some(&facet_id), offset).await?;
            let page: WorkdayPage = serde_json::from_value(raw).map_err(|error| {
                SourceError::schema(format!(
                    "invalid Workday page for {}: {error}",
                    self.company_id
                ))
            })?;
            if offset == 0 && page.total == 0 {
                return Err(SourceError::schema(format!(
                    "Workday returned no total for {}",
                    self.company_id
                )));
            }
            if page.total > 0
                && expected_total
                    .replace(page.total)
                    .is_some_and(|total| total != page.total)
            {
                return Err(SourceError::schema(format!(
                    "Workday total changed while scanning {}",
                    self.company_id
                )));
            }
            let total = expected_total.unwrap_or_default();
            if page.job_postings.is_empty() && listings.len() < total {
                return Err(SourceError::schema(format!(
                    "Workday pagination ended early for {}",
                    self.company_id
                )));
            }
            listings.extend(page.job_postings);
            if listings.len() >= total {
                break;
            }
            offset += PAGE_SIZE;
        }

        let mut paths = HashSet::new();
        if listings.len() != expected_total.unwrap_or_default()
            || listings
                .iter()
                .any(|listing| !paths.insert(listing.external_path.clone()))
        {
            return Err(SourceError::schema(format!(
                "incomplete or duplicate Workday board for {}",
                self.company_id
            )));
        }

        let mut jobs = Vec::with_capacity(listings.len());
        for listing in listings {
            if !listing.external_path.starts_with("/job/") {
                return Err(SourceError::schema(format!(
                    "invalid Workday job path for {}",
                    self.company_id
                )));
            }
            let detail_url = format!(
                "{}/wday/cxs/{}/{}{}",
                self.base_url, self.tenant, self.site, listing.external_path
            );
            let raw = send_text(self.client.get(detail_url), "Workday job").await?;
            let raw_payload = serde_json::from_str(&raw).map_err(|error| {
                SourceError::schema(format!(
                    "invalid Workday job for {}: {error}",
                    self.company_id
                ))
            })?;
            jobs.push(parse_workday_job(
                &self.company_id,
                raw_payload,
                &self.country_code,
            )?);
        }

        Ok(SourceScan::Complete { observations: jobs })
    }
}

async fn fetch_page(
    client: &Client,
    endpoint: &str,
    country_facet_id: Option<&str>,
    offset: usize,
) -> Result<serde_json::Value, SourceError> {
    let facets = country_facet_id
        .map(|id| serde_json::json!({ "locationCountry": [id] }))
        .unwrap_or_else(|| serde_json::json!({}));
    let raw = send_text(
        client.post(endpoint).json(&serde_json::json!({
            "appliedFacets": facets,
            "limit": PAGE_SIZE,
            "offset": offset,
            "searchText": ""
        })),
        "Workday",
    )
    .await?;
    serde_json::from_str(&raw)
        .map_err(|error| SourceError::schema(format!("invalid Workday response: {error}")))
}

fn find_country_facet(value: &serde_json::Value, country: &str) -> Option<String> {
    match value {
        serde_json::Value::Object(object) => {
            if object
                .get("facetParameter")
                .and_then(|value| value.as_str())
                == Some("locationCountry")
            {
                return object.get("values")?.as_array()?.iter().find_map(|value| {
                    (value.get("descriptor")?.as_str()? == country)
                        .then(|| value.get("id")?.as_str().map(str::to_owned))
                        .flatten()
                });
            }
            object
                .values()
                .find_map(|value| find_country_facet(value, country))
        }
        serde_json::Value::Array(values) => values
            .iter()
            .find_map(|value| find_country_facet(value, country)),
        _ => None,
    }
}

pub fn parse_workday_job(
    company_id: &str,
    raw_payload: serde_json::Value,
    country_code: &str,
) -> Result<ObservedJob, SourceError> {
    let detail: WorkdayDetail = serde_json::from_value(raw_payload.clone()).map_err(|error| {
        SourceError::schema(format!("invalid Workday job for {company_id}: {error}"))
    })?;
    let job = detail.job_posting_info;
    let mut locations = vec![job.location];
    locations.extend(job.additional_locations);
    locations.retain(|location| !location.trim().is_empty());
    locations.dedup();
    let description = html_markdown(&job.job_description);
    if job.job_req_id.trim().is_empty()
        || job.title.trim().is_empty()
        || locations.is_empty()
        || description.is_empty()
        || job.external_url.trim().is_empty()
    {
        return Err(SourceError::schema(format!(
            "Workday job for {company_id} is missing required data"
        )));
    }
    let published_at = NaiveDate::parse_from_str(&job.start_date, "%Y-%m-%d")
        .map_err(|error| {
            SourceError::schema(format!(
                "Workday job {} for {company_id} has invalid start date: {error}",
                job.job_req_id
            ))
        })?
        .and_hms_opt(0, 0, 0)
        .expect("midnight is valid")
        .and_utc();
    let job_url = job.external_url;

    Ok(ObservedJob {
        source_id: job.job_req_id,
        title: job.title,
        department: None,
        team: None,
        employment_type: non_empty(job.time_type),
        locations,
        countries: vec![country_code.to_owned()],
        apply_url: format!("{job_url}/apply"),
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
struct WorkdayPage {
    total: usize,
    job_postings: Vec<WorkdayListing>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkdayListing {
    external_path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkdayDetail {
    job_posting_info: WorkdayJob,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkdayJob {
    title: String,
    job_description: String,
    location: String,
    #[serde(default)]
    additional_locations: Vec<String>,
    start_date: String,
    time_type: String,
    job_req_id: String,
    external_url: String,
}
