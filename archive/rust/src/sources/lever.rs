use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError, country_code_for_location, http::send_text, json_ld::html_markdown,
};

pub struct LeverSource {
    company_id: String,
    api_url: String,
    country_filter: Option<String>,
    client: Client,
}

impl LeverSource {
    pub fn new(company_id: impl Into<String>, api_url: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            api_url: api_url.into(),
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
impl JobSource for LeverSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }
    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let mut url = Url::parse(&self.api_url).map_err(|error| {
            SourceError::schema(format!(
                "invalid Lever API URL for {}: {error}",
                self.company_id
            ))
        })?;
        url.query_pairs_mut().append_pair("mode", "json");
        let raw = send_text(self.client.get(url), "Lever").await?;
        Ok(SourceScan::Complete {
            observations: parse_lever_response(
                &self.company_id,
                &raw,
                self.country_filter.as_deref(),
            )?,
        })
    }
}

pub fn parse_lever_response(
    company_id: &str,
    raw: &str,
    country_filter: Option<&str>,
) -> Result<Vec<ObservedJob>, SourceError> {
    let postings: Vec<serde_json::Value> = serde_json::from_str(raw).map_err(|error| {
        SourceError::schema(format!("invalid Lever response for {company_id}: {error}"))
    })?;
    let mut ids = HashSet::new();
    let mut jobs = Vec::new();
    for raw_payload in postings {
        let posting: LeverPosting =
            serde_json::from_value(raw_payload.clone()).map_err(|error| {
                SourceError::schema(format!("invalid Lever posting for {company_id}: {error}"))
            })?;
        if posting.id.trim().is_empty() || !ids.insert(posting.id.clone()) {
            return Err(SourceError::schema(format!(
                "empty or duplicate Lever job id for {company_id}"
            )));
        }
        let locations = if posting.categories.all_locations.is_empty() {
            vec![posting.categories.location.clone()]
        } else {
            posting.categories.all_locations.clone()
        };
        let locations = if let Some(filter) = country_filter {
            locations
                .into_iter()
                .filter(|location| country_code_for_location(location) == Some(filter))
                .collect::<Vec<_>>()
        } else {
            locations
        };
        if locations.is_empty() {
            continue;
        }
        let countries = locations
            .iter()
            .map(|location| {
                country_code_for_location(location)
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        SourceError::schema(format!(
                            "Lever job {} for {company_id} has unresolved location {location:?}",
                            posting.id
                        ))
                    })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let description = html_markdown(&format!(
            "{}{}",
            posting.description_plain,
            posting
                .lists
                .iter()
                .map(|list| format!("<h2>{}</h2>{}", list.text, list.content))
                .collect::<String>()
        ));
        if posting.text.trim().is_empty()
            || posting.hosted_url.trim().is_empty()
            || posting.apply_url.trim().is_empty()
            || description.is_empty()
        {
            return Err(SourceError::schema(format!(
                "Lever job {} for {company_id} is missing required data",
                posting.id
            )));
        }
        let published_at =
            DateTime::<Utc>::from_timestamp_millis(posting.created_at).ok_or_else(|| {
                SourceError::schema(format!(
                    "Lever job {} for {company_id} has invalid createdAt",
                    posting.id
                ))
            })?;
        jobs.push(ObservedJob {
            source_id: posting.id,
            title: posting.text,
            department: non_empty(posting.categories.department),
            team: non_empty(posting.categories.team),
            employment_type: non_empty(posting.categories.commitment),
            locations,
            countries,
            job_url: posting.hosted_url,
            apply_url: posting.apply_url,
            description,
            raw_payload,
            published_at: Some(published_at),
        });
    }
    Ok(jobs)
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeverPosting {
    id: String,
    text: String,
    categories: LeverCategories,
    description_plain: String,
    #[serde(default)]
    lists: Vec<LeverList>,
    hosted_url: String,
    apply_url: String,
    created_at: i64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeverCategories {
    location: String,
    #[serde(default)]
    all_locations: Vec<String>,
    department: Option<String>,
    team: Option<String>,
    commitment: Option<String>,
}

#[derive(Deserialize)]
struct LeverList {
    text: String,
    content: String,
}
