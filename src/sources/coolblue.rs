use std::{collections::HashSet, str::FromStr};

use chrono::{DateTime, NaiveDate, Utc};
use futures_util::{StreamExt, stream};
use regex::Regex;
use reqwest::{Client, Url};
use scraper::{Html, Selector};

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError,
    http::send_text,
    json_ld::{html_markdown, job_posting_value, parse_job_posting},
};

const OFFICIAL_HOST: &str = "www.coolblue.nl";

pub struct CoolblueSource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl CoolblueSource {
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
impl JobSource for CoolblueSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let listing = send_text(self.client.get(&self.listing_url), "Coolblue").await?;
        let cards = parse_listing(&self.company_id, &self.listing_url, &listing)?;
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "Coolblue").await })
            .buffered(4)
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

#[derive(Debug)]
struct Card {
    listed_title: String,
    url: Url,
}

fn official_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(raw)
        .map_err(|error| schema(company_id, format!("invalid Coolblue URL: {error}")))?;
    if url.scheme() != "https" || url.host_str() != Some(OFFICIAL_HOST) {
        return Err(schema(company_id, "Coolblue URL is not official HTTPS"));
    }
    Ok(url)
}

fn parse_listing(
    company_id: &str,
    listing_url: &str,
    html: &str,
) -> Result<Vec<Card>, SourceError> {
    let base = official_url(listing_url, company_id)?;
    let document = Html::parse_document(html);
    let count_pattern = Regex::new(r"(?i)\b(\d+) jobs found\b").expect("static count regex");
    let text = document.root_element().text().collect::<Vec<_>>().join(" ");
    let totals = count_pattern
        .captures_iter(&text)
        .map(|captures| captures[1].parse::<usize>().expect("digits matched"))
        .collect::<HashSet<_>>();
    let total = match totals.iter().copied().collect::<Vec<_>>().as_slice() {
        [total] => *total,
        [] => return Err(schema(company_id, "listing has no job count")),
        _ => return Err(schema(company_id, "listing has conflicting job counts")),
    };

    let selector = Selector::parse(r#"a[aria-label][href^="/en/vacancies/"]"#)
        .expect("static Coolblue card selector");
    let mut urls = HashSet::new();
    let mut cards = Vec::new();
    for anchor in document.select(&selector) {
        let listed_title = anchor
            .value()
            .attr("aria-label")
            .map(str::trim)
            .filter(|title| !title.is_empty())
            .ok_or_else(|| schema(company_id, "listing card has no title"))?
            .to_owned();
        let url = base
            .join(anchor.value().attr("href").expect("selector requires href"))
            .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
        if url.scheme() != "https"
            || url.host_str() != Some(OFFICIAL_HOST)
            || url
                .path_segments()
                .is_none_or(|segments| segments.count() != 3)
        {
            return Err(schema(company_id, "vacancy URL is not an official job URL"));
        }
        if !urls.insert(url.clone()) {
            return Err(schema(company_id, format!("duplicate vacancy URL {url}")));
        }
        cards.push(Card { listed_title, url });
    }
    if cards.len() != total {
        return Err(schema(
            company_id,
            format!(
                "incomplete listing: expected {total} jobs, got {}",
                cards.len()
            ),
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
    let jobs = cards
        .into_iter()
        .zip(details)
        .map(|(card, detail)| observed_job(company_id, card, detail))
        .collect::<Result<Vec<_>, _>>()?;
    let mut ids = HashSet::new();
    for job in &jobs {
        if !ids.insert(&job.source_id) {
            return Err(schema(
                company_id,
                format!("duplicate vacancy ID {}", job.source_id),
            ));
        }
    }
    Ok(jobs)
}

fn observed_job(company_id: &str, card: Card, detail: &str) -> Result<ObservedJob, SourceError> {
    let posting = parse_job_posting(detail, "Coolblue")?;
    let raw_payload = job_posting_value(detail, "Coolblue")?;
    let source_id = required(
        posting.identifier.as_ref().map(|id| id.value.as_str()),
        "identifier",
        company_id,
    )?;
    let title = required(posting.title.as_deref(), "title", company_id)?;
    if !title.starts_with(&card.listed_title) {
        return Err(schema(company_id, "detail title does not match listing"));
    }
    let raw_job_url = required(posting.url.as_deref(), "url", company_id)?;
    let job_url = official_url(&raw_job_url, company_id)?;
    if job_url != card.url {
        return Err(schema(company_id, "detail URL does not match listing"));
    }
    if posting
        .hiring_organization
        .as_ref()
        .and_then(|organization| organization.name.as_deref())
        != Some("Coolblue")
    {
        return Err(schema(company_id, "unexpected hiring organization"));
    }
    if posting.job_location.is_empty() {
        return Err(schema(company_id, "detail has no locations"));
    }
    let mut locations = Vec::new();
    for location in &posting.job_location {
        if location.address.address_country.as_deref() != Some("NL") {
            return Err(schema(company_id, "detail has an unresolved country"));
        }
        let locality = required(
            location
                .name
                .as_deref()
                .or(location.address.address_locality.as_deref()),
            "location",
            company_id,
        )?;
        if !locations.iter().any(|existing| existing == &locality) {
            locations.push(locality);
        }
    }
    let published_at = parse_date(
        required(posting.date_posted.as_deref(), "datePosted", company_id)?,
        company_id,
    )?;

    Ok(ObservedJob {
        source_id,
        title,
        department: None,
        team: None,
        employment_type: posting.employment_type,
        locations,
        countries: vec!["NL".into()],
        job_url: job_url.to_string(),
        apply_url: job_url.to_string(),
        description: html_markdown(&posting.description),
        raw_payload,
        published_at: Some(published_at),
    })
}

fn parse_date(raw: String, company_id: &str) -> Result<DateTime<Utc>, SourceError> {
    DateTime::parse_from_rfc3339(&raw)
        .map(|date| date.with_timezone(&Utc))
        .or_else(|_| {
            NaiveDate::from_str(&raw).map(|date| {
                date.and_hms_opt(0, 0, 0)
                    .expect("midnight is valid")
                    .and_utc()
            })
        })
        .map_err(|error| schema(company_id, format!("invalid datePosted: {error}")))
}

fn required(value: Option<&str>, field: &str, company_id: &str) -> Result<String, SourceError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| schema(company_id, format!("detail has no {field}")))
}

fn schema(company_id: &str, message: impl Into<String>) -> SourceError {
    SourceError::schema(format!("{company_id}: {}", message.into()))
}

pub fn parse_coolblue_pages(
    company_id: &str,
    listing_url: &str,
    listing: &str,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_listing(company_id, listing_url, listing)?;
    parse_details(company_id, cards, details)
}
