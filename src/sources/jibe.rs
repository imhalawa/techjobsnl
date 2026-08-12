use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError, country_code_for_location, http::send_text, json_ld::html_text,
};

const PAGE_SIZE: usize = 100;

pub struct JibeSource {
    company_id: String,
    base_url: String,
    client_name: String,
    client: Client,
}

impl JibeSource {
    pub fn new(
        company_id: impl Into<String>,
        base_url: impl Into<String>,
        client_name: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            company_id: company_id.into(),
            base_url: base_url.into(),
            client_name: client_name.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for JibeSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let mut collection =
            JibeCollection::new(&self.company_id, &self.base_url, &self.client_name);
        for page in 1.. {
            let raw = send_text(
                self.client.get(page_url(
                    &self.base_url,
                    &self.client_name,
                    page,
                    &self.company_id,
                )?),
                "Jibe",
            )
            .await?;
            if collection.add_page(&raw)? {
                return Ok(SourceScan::Complete {
                    observations: collection.finish()?,
                });
            }
        }
        unreachable!("Jibe pagination returns after reaching or exceeding the declared total")
    }
}

pub fn parse_jibe_pages(
    company_id: &str,
    base_url: &str,
    client_name: &str,
    pages: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let mut collection = JibeCollection::new(company_id, base_url, client_name);
    for page in pages {
        if collection.add_page(page)? {
            break;
        }
    }
    collection.finish()
}

struct JibeCollection<'a> {
    company_id: &'a str,
    base_url: &'a str,
    client_name: &'a str,
    expected_total: Option<usize>,
    expected_count: Option<usize>,
    ids: HashSet<String>,
    observations: Vec<ObservedJob>,
}

impl<'a> JibeCollection<'a> {
    fn new(company_id: &'a str, base_url: &'a str, client_name: &'a str) -> Self {
        Self {
            company_id,
            base_url,
            client_name,
            expected_total: None,
            expected_count: None,
            ids: HashSet::new(),
            observations: Vec::new(),
        }
    }

    fn add_page(&mut self, raw: &str) -> Result<bool, SourceError> {
        let page: JibePage = serde_json::from_str(raw).map_err(|error| {
            SourceError::schema(format!(
                "invalid Jibe response for {}: {error}",
                self.company_id
            ))
        })?;
        let jobs = page.jobs.ok_or_else(|| {
            SourceError::schema(format!(
                "Jibe response for {} is missing jobs",
                self.company_id
            ))
        })?;
        let expected_total = *self.expected_total.get_or_insert(page.total_count);
        let expected_count = *self.expected_count.get_or_insert(page.count);
        if page.total_count != expected_total || page.count != expected_count {
            return Err(SourceError::schema(format!(
                "Jibe pagination metadata changed for {}",
                self.company_id
            )));
        }
        if expected_count != expected_total {
            return Err(SourceError::schema(format!(
                "Jibe count metadata disagrees for {}: count {expected_count}, totalCount {expected_total}",
                self.company_id
            )));
        }
        if jobs.is_empty() && self.observations.len() < expected_total {
            return Err(SourceError::schema(format!(
                "Jibe returned an empty page before reaching {expected_total} jobs for {}",
                self.company_id
            )));
        }

        for raw_row in jobs {
            let row: JibeRow = serde_json::from_value(raw_row.clone()).map_err(|error| {
                SourceError::schema(format!("invalid Jibe job for {}: {error}", self.company_id))
            })?;
            let observation = observed_job(
                self.company_id,
                self.base_url,
                self.client_name,
                row,
                raw_row,
            )?;
            if !self.ids.insert(observation.source_id.clone()) {
                return Err(SourceError::schema(format!(
                    "duplicate Jibe job {} for {}",
                    observation.source_id, self.company_id
                )));
            }
            self.observations.push(observation);
            if self.observations.len() > expected_total {
                return Err(SourceError::schema(format!(
                    "Jibe returned more than {expected_total} jobs for {}",
                    self.company_id
                )));
            }
        }
        Ok(self.observations.len() == expected_total)
    }

    fn finish(self) -> Result<Vec<ObservedJob>, SourceError> {
        let expected = self.expected_total.unwrap_or_default();
        if self.observations.len() != expected {
            return Err(SourceError::schema(format!(
                "Jibe returned {} of {expected} jobs for {}",
                self.observations.len(),
                self.company_id
            )));
        }
        Ok(self.observations)
    }
}

fn page_url(
    base_url: &str,
    client_name: &str,
    page: usize,
    company_id: &str,
) -> Result<Url, SourceError> {
    let mut url = Url::parse(base_url).map_err(|error| {
        SourceError::schema(format!("invalid Jibe base URL for {company_id}: {error}"))
    })?;
    url.set_path("/api/jobs");
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("limit", &PAGE_SIZE.to_string())
        .append_pair("page", &page.to_string())
        .append_pair("brand", client_name);
    Ok(url)
}

fn observed_job(
    company_id: &str,
    base_url: &str,
    client_name: &str,
    row: JibeRow,
    raw_payload: serde_json::Value,
) -> Result<ObservedJob, SourceError> {
    let job = &row.data;
    required(&job.req_id, "id", company_id)?;
    required(&job.title, "title", company_id)?;
    required(&job.apply_url, "apply URL", company_id)?;
    required(&job.full_location, "location", company_id)?;
    let description = html_text(&job.description);
    if description.is_empty() {
        return Err(SourceError::schema(format!(
            "Jibe job {} for {company_id} has an empty description",
            job.req_id
        )));
    }

    let locations = job
        .full_location
        .split(';')
        .map(str::trim)
        .filter(|location| !location.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if locations.is_empty() {
        return Err(SourceError::schema(format!(
            "Jibe job {} for {company_id} has no locations",
            job.req_id
        )));
    }
    let mut countries = Vec::new();
    for location in &locations {
        let country = country_code_for_location(location).ok_or_else(|| {
            SourceError::schema(format!(
                "Jibe job {} for {company_id} has unresolved location {location:?}",
                job.req_id
            ))
        })?;
        push_unique(&mut countries, country.to_owned());
    }

    let published_at = DateTime::parse_from_str(&job.posted_date, "%Y-%m-%dT%H:%M:%S%z")
        .map_err(|error| {
            SourceError::schema(format!(
                "Jibe job {} for {company_id} has an invalid posted_date: {error}",
                job.req_id
            ))
        })?
        .with_timezone(&Utc);
    let department = joined(&job.category);
    let team = non_empty(&job.department);
    let mut job_url = Url::parse(base_url).map_err(|error| {
        SourceError::schema(format!("invalid Jibe base URL for {company_id}: {error}"))
    })?;
    job_url.set_path(&format!(
        "/{}/jobs/{}",
        client_name
            .split('.')
            .next()
            .unwrap_or(client_name)
            .to_ascii_lowercase(),
        job.req_id
    ));
    job_url.set_query(None);

    Ok(ObservedJob {
        source_id: job.req_id.clone(),
        title: job.title.clone(),
        department,
        team,
        employment_type: non_empty(&job.employment_type),
        locations,
        countries,
        job_url: job_url.to_string(),
        apply_url: job.apply_url.clone(),
        description,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn required(value: &str, field: &str, company_id: &str) -> Result<(), SourceError> {
    if value.trim().is_empty() {
        return Err(SourceError::schema(format!(
            "Jibe job for {company_id} has an empty {field}"
        )));
    }
    Ok(())
}

fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}

fn joined(values: &[String]) -> Option<String> {
    let values = values
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    (!values.is_empty()).then(|| values.join(" / "))
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JibePage {
    count: usize,
    total_count: usize,
    jobs: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
struct JibeRow {
    data: JibeJob,
}

#[derive(Deserialize)]
struct JibeJob {
    req_id: String,
    title: String,
    description: String,
    apply_url: String,
    posted_date: String,
    #[serde(default)]
    category: Vec<String>,
    #[serde(default)]
    department: String,
    #[serde(default)]
    employment_type: String,
    full_location: String,
}
