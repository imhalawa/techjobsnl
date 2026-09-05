use std::collections::HashSet;

use chrono::{NaiveDate, TimeZone, Utc};
use futures_util::{StreamExt, stream};
use regex::Regex;
use reqwest::{Client, Url};
use scraper::{Html, Selector};

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError,
    http::send_text,
    json_ld::{JobPosting, html_markdown, job_posting_value},
};

pub struct AfasSource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl AfasSource {
    pub fn new(
        company_id: impl Into<String>,
        listing_url: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            company_id: company_id.into(),
            listing_url: listing_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for AfasSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let listing = send_text(self.client.get(&self.listing_url), "AFAS").await?;
        let cards = parse_listing(&self.company_id, &self.listing_url, &listing)?;
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "AFAS job").await })
            .buffered(6)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();

        Ok(SourceScan::Complete {
            observations: parse_details(&self.company_id, cards, &detail_refs)?,
        })
    }
}

pub fn parse_afas_page(
    company_id: &str,
    listing_url: &str,
    listing: &str,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_listing(company_id, listing_url, listing)?;
    parse_details(company_id, cards, details)
}

fn parse_listing(
    company_id: &str,
    listing_url: &str,
    listing: &str,
) -> Result<Vec<Card>, SourceError> {
    let base = official_listing_url(listing_url, company_id)?;
    let state_pattern =
        Regex::new(r#""firstPage":(true|false),"lastPage":(true|false),"pageSize":([0-9]+)"#)
            .expect("static AFAS state pattern must compile");
    let states = state_pattern.captures_iter(listing).collect::<Vec<_>>();
    if states.is_empty()
        || states.iter().any(|state| {
            state.get(1).map(|value| value.as_str()) != Some("true")
                || state.get(2).map(|value| value.as_str()) != Some("true")
                || state.get(3).map(|value| value.as_str()) != Some("75")
        })
    {
        return Err(schema(
            company_id,
            "board is not a complete one-page result",
        ));
    }

    let link_pattern =
        Regex::new(r#""link":"\\/job\\/([^"\\]+)"#).expect("static AFAS link pattern must compile");
    let mut ids = HashSet::new();
    let mut cards = Vec::new();
    for captures in link_pattern.captures_iter(listing) {
        let id = captures
            .get(1)
            .expect("capture group must exist")
            .as_str()
            .to_owned();
        if !ids.insert(id.clone()) {
            continue;
        }
        let url = base
            .join(&format!("/job/{id}"))
            .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
        cards.push(Card { id, url });
    }
    if cards.is_empty() || cards.len() > 75 {
        return Err(schema(
            company_id,
            "board returned an invalid vacancy count",
        ));
    }
    Ok(cards)
}

fn parse_details(
    company_id: &str,
    cards: Vec<Card>,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    if cards.len() != details.len() {
        return Err(schema(company_id, "listing/detail count mismatch"));
    }
    cards
        .into_iter()
        .zip(details)
        .map(|(card, detail)| observed_job(company_id, card, detail))
        .collect::<Result<Vec<_>, _>>()
        .map(|jobs| jobs.into_iter().flatten().collect())
}

fn observed_job(
    company_id: &str,
    card: Card,
    detail: &str,
) -> Result<Option<ObservedJob>, SourceError> {
    let clean_detail = detail.replace("//<![CDATA[", "").replace("//]]>", "");
    let mut raw_payload = job_posting_value(&clean_detail, "AFAS")?;
    let raw_locations = match raw_payload.get("jobLocation") {
        Some(serde_json::Value::Array(locations)) => locations.iter().collect::<Vec<_>>(),
        Some(location @ serde_json::Value::Object(_)) => vec![location],
        _ => {
            return Err(schema(
                company_id,
                format!("detail {} has no location", card.id),
            ));
        }
    };
    let countries = raw_locations
        .iter()
        .map(|location| {
            location
                .pointer("/address/addressCountry")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| schema(company_id, format!("detail {} has no country", card.id)))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if !countries.contains(&"NL") {
        return Ok(None);
    }
    if raw_payload
        .get("description")
        .is_none_or(serde_json::Value::is_null)
    {
        raw_payload["description"] = serde_json::Value::String(visible_description(&clean_detail));
    }
    let posting: JobPosting = serde_json::from_value(raw_payload.clone()).map_err(|error| {
        schema(
            company_id,
            format!("detail {} has invalid JobPosting: {error}", card.id),
        )
    })?;
    if posting
        .hiring_organization
        .as_ref()
        .and_then(|organization| organization.name.as_deref())
        != Some("AFAS Software B.V.")
    {
        return Err(schema(
            company_id,
            format!("detail {} is not an AFAS job", card.id),
        ));
    }
    let title = posting
        .title
        .map(|title| title.trim().to_owned())
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no title", card.id)))?;
    let published_at = posting
        .date_posted
        .as_deref()
        .ok_or_else(|| schema(company_id, format!("detail {} has no datePosted", card.id)))
        .and_then(|date| {
            NaiveDate::parse_from_str(date, "%Y-%m-%d")
                .map(|date| {
                    Utc.from_utc_datetime(
                        &date.and_hms_opt(0, 0, 0).expect("midnight must be valid"),
                    )
                })
                .map_err(|error| schema(company_id, format!("invalid datePosted: {error}")))
        })?;
    let mut locations = Vec::new();
    for place in posting.job_location {
        if place.address.address_country.as_deref() != Some("NL") {
            return Err(schema(
                company_id,
                format!("detail {} is not in NL", card.id),
            ));
        }
        let location = place
            .address
            .address_locality
            .map(|location| location.trim().to_owned())
            .filter(|location| !location.is_empty())
            .ok_or_else(|| schema(company_id, format!("detail {} has no location", card.id)))?;
        if !locations.contains(&location) {
            locations.push(location);
        }
    }
    if locations.is_empty() || posting.description.trim().is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} is incomplete", card.id),
        ));
    }
    let job_url = card.url.to_string();

    Ok(Some(ObservedJob {
        source_id: card.id,
        title,
        department: None,
        team: None,
        employment_type: posting.employment_type,
        locations,
        countries: vec!["NL".to_owned()],
        apply_url: job_url.clone(),
        job_url,
        description: posting.description,
        raw_payload,
        published_at: Some(published_at),
    }))
}

fn visible_description(detail: &str) -> String {
    let document = Html::parse_document(detail);
    let selector = Selector::parse("main#P_mastercontent .freehtml")
        .expect("static AFAS description selector must compile");
    html_markdown(
        &document
            .select(&selector)
            .map(|element| element.inner_html())
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

fn official_listing_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(raw)
        .map_err(|error| schema(company_id, format!("invalid listing URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.werkenbijafas.nl")
        || url.path() != "/alle-vacatures"
        || url.query().is_some()
    {
        return Err(schema(company_id, "unexpected AFAS listing URL"));
    }
    Ok(url)
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("AFAS response for {company_id}: {message}"))
}

struct Card {
    id: String,
    url: Url,
}
