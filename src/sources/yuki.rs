use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::Value;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

const FEED_URL: &str = "https://jobs.yukisoftware.com/jobs.json";
const YUKI_EMPLOYER: &str = "The Yuki Company";

pub struct YukiSource {
    company_id: String,
    feed_url: String,
    expected_employer: String,
    client: Client,
}

impl YukiSource {
    pub fn new(company_id: impl Into<String>, feed_url: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            feed_url: feed_url.into(),
            expected_employer: YUKI_EMPLOYER.into(),
            client,
        }
    }

    pub fn with_employer(mut self, employer: impl Into<String>) -> Self {
        self.expected_employer = employer.into();
        self
    }
}

#[async_trait::async_trait]
impl JobSource for YukiSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let raw = send_text(self.client.get(&self.feed_url), "Yuki").await?;
        Ok(SourceScan::Complete {
            observations: parse_teamtailor_feed(
                &self.company_id,
                &raw,
                &self.feed_url,
                &self.expected_employer,
            )?,
        })
    }
}

pub fn parse_yuki_feed(company_id: &str, raw: &str) -> Result<Vec<ObservedJob>, SourceError> {
    parse_teamtailor_feed(company_id, raw, FEED_URL, YUKI_EMPLOYER)
}

pub fn parse_teamtailor_feed(
    company_id: &str,
    raw: &str,
    expected_feed_url: &str,
    expected_employer: &str,
) -> Result<Vec<ObservedJob>, SourceError> {
    let feed: Feed = serde_json::from_str(raw)
        .map_err(|error| schema(company_id, format!("invalid JSON feed: {error}")))?;
    let expected_home_url = expected_feed_url
        .strip_suffix(".json")
        .ok_or_else(|| schema(company_id, "configured feed URL must end with .json"))?;
    if feed.version != "https://jsonfeed.org/version/1.1"
        || feed.title != expected_employer
        || feed.home_page_url != expected_home_url
        || feed.feed_url != expected_feed_url
    {
        return Err(schema(company_id, "unexpected feed identity"));
    }

    let mut ids = HashSet::new();
    feed.items
        .into_iter()
        .map(|raw_item| {
            let item: Item = serde_json::from_value(raw_item.clone())
                .map_err(|error| schema(company_id, format!("invalid feed item: {error}")))?;
            let job = observed_job(
                company_id,
                item,
                raw_item,
                expected_feed_url,
                expected_employer,
            )?;
            if !ids.insert(job.source_id.clone()) {
                return Err(schema(
                    company_id,
                    format!("duplicate job ID {}", job.source_id),
                ));
            }
            Ok(job)
        })
        .collect()
}

fn observed_job(
    company_id: &str,
    item: Item,
    raw_payload: Value,
    expected_feed_url: &str,
    expected_employer: &str,
) -> Result<ObservedJob, SourceError> {
    if item.title.trim().is_empty()
        || item.title != item.posting.title
        || item.content_html != item.posting.description
        || item.date_published != item.posting.date_posted
        || item.posting.hiring_organization.name != expected_employer
    {
        return Err(schema(company_id, "feed item and JobPosting disagree"));
    }
    let source_id = match item.posting.identifier.value {
        Value::Number(value) => value.to_string(),
        Value::String(value) if !value.trim().is_empty() => value,
        _ => return Err(schema(company_id, "job has an invalid identifier")),
    };
    let job_url = official_job_url(&item.url, &source_id, expected_feed_url, company_id)?;
    if item.posting.job_location.is_empty() {
        return Err(schema(company_id, "job has no locations"));
    }
    let mut locations = Vec::new();
    let mut countries = Vec::new();
    for place in item.posting.job_location {
        push_unique(
            &mut locations,
            required(&place.address.address_locality, "location", company_id)?,
        );
        let country = required(&place.address.address_country, "country", company_id)?;
        if country.len() != 2
            || !country
                .chars()
                .all(|character| character.is_ascii_uppercase())
        {
            return Err(schema(company_id, "job has an invalid country code"));
        }
        push_unique(&mut countries, country);
    }
    let description = html_markdown(&item.content_html);
    if description.is_empty() {
        return Err(schema(company_id, "job has an empty description"));
    }
    let published_at = DateTime::parse_from_rfc3339(&item.date_published)
        .map_err(|error| schema(company_id, format!("invalid publication date: {error}")))?
        .with_timezone(&Utc);
    let mut apply_url = job_url.clone();
    apply_url.set_path(&format!("{}/applications/new", job_url.path()));

    Ok(ObservedJob {
        source_id,
        title: item.title,
        department: None,
        team: None,
        employment_type: None,
        locations,
        countries,
        job_url: job_url.to_string(),
        apply_url: apply_url.to_string(),
        description,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn official_job_url(
    raw: &str,
    id: &str,
    expected_feed_url: &str,
    company_id: &str,
) -> Result<Url, SourceError> {
    let url =
        Url::parse(raw).map_err(|error| schema(company_id, format!("invalid job URL: {error}")))?;
    let feed_url = Url::parse(expected_feed_url)
        .map_err(|error| schema(company_id, format!("invalid configured feed URL: {error}")))?;
    let jobs_path = feed_url
        .path()
        .strip_suffix(".json")
        .ok_or_else(|| schema(company_id, "configured feed URL must end with .json"))?;
    let expected_prefix = format!("{jobs_path}/{id}-");
    if url.scheme() != "https"
        || url.host_str() != feed_url.host_str()
        || !url.path().starts_with(&expected_prefix)
        || url.path().len() == expected_prefix.len()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(schema(
            company_id,
            "job URL is not official or has a mismatched ID",
        ));
    }
    Ok(url)
}

fn required(value: &str, field: &str, company_id: &str) -> Result<String, SourceError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(schema(company_id, format!("job has no {field}")));
    }
    Ok(value.to_owned())
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("Yuki response for {company_id}: {message}"))
}

#[derive(Deserialize)]
struct Feed {
    version: String,
    title: String,
    home_page_url: String,
    feed_url: String,
    items: Vec<Value>,
}

#[derive(Deserialize)]
struct Item {
    url: String,
    title: String,
    content_html: String,
    date_published: String,
    #[serde(rename = "_jobposting")]
    posting: JobPosting,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct JobPosting {
    title: String,
    description: String,
    identifier: Identifier,
    date_posted: String,
    hiring_organization: HiringOrganization,
    job_location: Vec<JobLocation>,
}

#[derive(Deserialize)]
struct Identifier {
    value: Value,
}

#[derive(Deserialize)]
struct HiringOrganization {
    name: String,
}

#[derive(Deserialize)]
struct JobLocation {
    address: Address,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Address {
    address_locality: String,
    address_country: String,
}
