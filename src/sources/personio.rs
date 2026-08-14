use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError, country_code_for_location, http::send_text, json_ld::html_markdown,
};

pub struct PersonioSource {
    company_id: String,
    base_url: String,
    client: Client,
}

impl PersonioSource {
    pub fn new(company_id: impl Into<String>, base_url: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            base_url: base_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for PersonioSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let raw = send_text(
            self.client.get(format!(
                "{}/xml?language=en",
                self.base_url.trim_end_matches('/')
            )),
            "Personio",
        )
        .await?;
        Ok(SourceScan::Complete {
            observations: parse_personio_feed(&self.company_id, &self.base_url, &raw)?,
        })
    }
}

pub fn parse_personio_feed(
    company_id: &str,
    base_url: &str,
    raw: &str,
) -> Result<Vec<ObservedJob>, SourceError> {
    let feed: PersonioFeed = quick_xml::de::from_str(raw).map_err(|error| {
        SourceError::schema(format!("invalid Personio feed for {company_id}: {error}"))
    })?;
    let base = Url::parse(base_url).map_err(|error| {
        SourceError::schema(format!(
            "invalid Personio base URL for {company_id}: {error}"
        ))
    })?;
    let mut ids = HashSet::new();
    feed.positions
        .into_iter()
        .map(|position| {
            if position.id.trim().is_empty() || !ids.insert(position.id.clone()) {
                return Err(SourceError::schema(format!(
                    "empty or duplicate Personio job id for {company_id}"
                )));
            }
            let raw_payload = serde_json::to_value(&position).map_err(|error| {
                SourceError::schema(format!(
                    "could not preserve Personio job {} for {company_id}: {error}",
                    position.id
                ))
            })?;
            let title = required(position.name, "title", &position.id, company_id)?;
            let location = required(position.office, "office", &position.id, company_id)?;
            let country = country_code_for_location(&location)
                .or_else(|| (location == "NL" || location.starts_with("Amsterdam")).then_some("NL"))
                .or_else(|| location.starts_with("Barcelona").then_some("ES"))
                .or_else(|| {
                    (location.starts_with("Bratislava") || location.starts_with("Zilina"))
                        .then_some("SK")
                })
                .ok_or_else(|| {
                    SourceError::schema(format!(
                        "Personio job {} for {company_id} has unresolved office {location:?}",
                        position.id
                    ))
                })?;
            let description = position
                .job_descriptions
                .items
                .into_iter()
                .map(|item| format!("<h2>{}</h2>{}", item.name, item.value))
                .collect::<String>();
            let description = html_markdown(&description);
            if description.is_empty() {
                return Err(SourceError::schema(format!(
                    "Personio job {} for {company_id} has an empty description",
                    position.id
                )));
            }
            let published_at = DateTime::parse_from_rfc3339(&position.created_at)
                .map_err(|error| {
                    SourceError::schema(format!(
                        "Personio job {} for {company_id} has invalid createdAt: {error}",
                        position.id
                    ))
                })?
                .with_timezone(&Utc);
            let job_url = base
                .join(&format!("job/{}?language=en", position.id))
                .map_err(|error| {
                    SourceError::schema(format!(
                        "invalid Personio job URL for {company_id}: {error}"
                    ))
                })?;
            Ok(ObservedJob {
                source_id: position.id,
                title,
                department: non_empty(position.department),
                team: non_empty(position.recruiting_category),
                employment_type: non_empty(position.employment_type),
                locations: vec![location],
                countries: vec![country.into()],
                job_url: job_url.to_string(),
                apply_url: format!("{job_url}#apply"),
                description,
                raw_payload,
                published_at: Some(published_at),
            })
        })
        .collect()
}

fn required(value: String, field: &str, id: &str, company_id: &str) -> Result<String, SourceError> {
    if value.trim().is_empty() {
        Err(SourceError::schema(format!(
            "Personio job {id} for {company_id} has an empty {field}"
        )))
    } else {
        Ok(value.trim().to_owned())
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

#[derive(Deserialize, Serialize)]
struct PersonioFeed {
    #[serde(rename = "position", default)]
    positions: Vec<PersonioPosition>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PersonioPosition {
    id: String,
    office: String,
    department: Option<String>,
    recruiting_category: Option<String>,
    name: String,
    job_descriptions: PersonioDescriptions,
    employment_type: Option<String>,
    created_at: String,
}

#[derive(Deserialize, Serialize)]
struct PersonioDescriptions {
    #[serde(rename = "jobDescription", default)]
    items: Vec<PersonioDescription>,
}

#[derive(Deserialize, Serialize)]
struct PersonioDescription {
    name: String,
    value: String,
}
