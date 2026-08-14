use std::collections::HashSet;

use chrono::NaiveDateTime;
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

pub struct RecruiteeSource {
    company_id: String,
    base_url: String,
    client: Client,
}

impl RecruiteeSource {
    pub fn new(company_id: impl Into<String>, base_url: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            base_url: base_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for RecruiteeSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let raw = send_text(
            self.client
                .get(offers_url(&self.base_url, &self.company_id)?),
            "Recruitee",
        )
        .await?;
        let observations = parse_recruitee_response(&self.company_id, &raw)?;
        Ok(SourceScan::Complete { observations })
    }
}

pub fn parse_recruitee_response(
    company_id: &str,
    raw: &str,
) -> Result<Vec<ObservedJob>, SourceError> {
    let response: RecruiteeResponse = serde_json::from_str(raw).map_err(|error| {
        SourceError::schema(format!(
            "invalid Recruitee response for {company_id}: {error}"
        ))
    })?;
    let offers = response.offers.ok_or_else(|| {
        SourceError::schema(format!(
            "Recruitee response for {company_id} is missing offers"
        ))
    })?;
    let mut ids = HashSet::new();
    offers
        .into_iter()
        .map(|raw_offer| {
            let offer: RecruiteeOffer =
                serde_json::from_value(raw_offer.clone()).map_err(|error| {
                    SourceError::schema(format!(
                        "invalid Recruitee offer for {company_id}: {error}"
                    ))
                })?;
            let observation = observed_job(company_id, offer, raw_offer)?;
            if !ids.insert(observation.source_id.clone()) {
                return Err(SourceError::schema(format!(
                    "duplicate Recruitee offer {} for {company_id}",
                    observation.source_id
                )));
            }
            Ok(observation)
        })
        .collect()
}

fn offers_url(base_url: &str, company_id: &str) -> Result<Url, SourceError> {
    let mut url = Url::parse(base_url).map_err(|error| {
        SourceError::schema(format!(
            "invalid Recruitee base URL for {company_id}: {error}"
        ))
    })?;
    url.set_path("/api/offers/");
    url.set_query(None);
    Ok(url)
}

fn observed_job(
    company_id: &str,
    offer: RecruiteeOffer,
    raw_payload: serde_json::Value,
) -> Result<ObservedJob, SourceError> {
    let id = offer.id.ok_or_else(|| {
        SourceError::schema(format!("Recruitee offer for {company_id} is missing id"))
    })?;
    required(&offer.title, "title", id, company_id)?;
    required(&offer.careers_url, "official URL", id, company_id)?;
    required(&offer.careers_apply_url, "apply URL", id, company_id)?;
    let locations = offer
        .locations
        .as_ref()
        .filter(|locations| !locations.is_empty())
        .ok_or_else(|| {
            SourceError::schema(format!(
                "Recruitee offer {id} for {company_id} has no locations"
            ))
        })?;
    let mut location_names = Vec::new();
    let mut countries = Vec::new();
    for location in locations {
        required(&location.name, "location", id, company_id)?;
        required(&location.country_code, "location country", id, company_id)?;
        push_unique(&mut location_names, location.name.trim().to_owned());
        push_unique(&mut countries, location.country_code.trim().to_uppercase());
    }

    let description = html_markdown(&format!(
        "{}<br><br>{}",
        offer.description, offer.requirements
    ));
    if description.is_empty() {
        return Err(SourceError::schema(format!(
            "Recruitee offer {id} for {company_id} has an empty description"
        )));
    }
    let published_at = NaiveDateTime::parse_from_str(&offer.published_at, "%Y-%m-%d %H:%M:%S UTC")
        .map_err(|error| {
            SourceError::schema(format!(
                "Recruitee offer {id} for {company_id} has an invalid published_at: {error}"
            ))
        })?
        .and_utc();
    Ok(ObservedJob {
        source_id: id.to_string(),
        title: offer.title,
        department: non_empty(&offer.department),
        team: non_empty(&offer.category_code),
        employment_type: non_empty(&offer.employment_type_code),
        locations: location_names,
        countries,
        job_url: offer.careers_url,
        apply_url: offer.careers_apply_url,
        description,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn required(value: &str, field: &str, id: u64, company_id: &str) -> Result<(), SourceError> {
    if value.trim().is_empty() {
        return Err(SourceError::schema(format!(
            "Recruitee offer {id} for {company_id} has an empty {field}"
        )));
    }
    Ok(())
}

fn non_empty(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.trim().to_owned())
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[derive(Deserialize)]
struct RecruiteeResponse {
    offers: Option<Vec<serde_json::Value>>,
}

#[derive(Deserialize)]
struct RecruiteeOffer {
    id: Option<u64>,
    title: String,
    #[serde(default)]
    department: String,
    #[serde(default)]
    category_code: String,
    #[serde(default)]
    employment_type_code: String,
    description: String,
    requirements: String,
    careers_url: String,
    careers_apply_url: String,
    published_at: String,
    locations: Option<Vec<RecruiteeLocation>>,
}

#[derive(Deserialize)]
struct RecruiteeLocation {
    name: String,
    country_code: String,
}
