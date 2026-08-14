use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{Html, Selector};

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError, country_code_for_location,
    http::send_text,
    json_ld::{html_text, job_posting_value, parse_job_posting},
};

const PAGE_SIZE: usize = 10;

pub struct EnecoSource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl EnecoSource {
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
impl JobSource for EnecoSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let first = send_text(
            self.client
                .get(page_url(&self.listing_url, 0, &self.company_id)?),
            "Eneco",
        )
        .await?;
        let total = page_range(&first, &self.company_id)?.total;
        let page_count = total.max(1).div_ceil(PAGE_SIZE);
        let mut listings = vec![first];
        for page in 1..page_count {
            listings.push(
                send_text(
                    self.client.get(page_url(
                        &self.listing_url,
                        page * PAGE_SIZE,
                        &self.company_id,
                    )?),
                    "Eneco",
                )
                .await?,
            );
        }
        let listing_refs = listings.iter().map(String::as_str).collect::<Vec<_>>();
        let cards = parse_listings(&self.company_id, &self.listing_url, &listing_refs)?;
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "Eneco").await })
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

pub fn parse_eneco_pages(
    company_id: &str,
    listing_url: &str,
    listings: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_listings(company_id, listing_url, listings)?;
    parse_details(company_id, cards, details)
}

fn parse_listings(
    company_id: &str,
    listing_url: &str,
    pages: &[&str],
) -> Result<Vec<Card>, SourceError> {
    let base = official_listing(listing_url, company_id)?;
    let selector = Selector::parse("a.vacancy-item[href]").expect("static selector must parse");
    let title_selector = Selector::parse("h3").expect("static selector must parse");
    let mut total = None;
    let mut ids = HashSet::new();
    let mut cards = Vec::new();

    for (page, raw) in pages.iter().enumerate() {
        let range = page_range(raw, company_id)?;
        if total
            .replace(range.total)
            .is_some_and(|value| value != range.total)
        {
            return Err(schema(company_id, "listing total changed between pages"));
        }
        let expected_start = page * PAGE_SIZE + 1;
        let expected_end = (expected_start + PAGE_SIZE - 1).min(range.total);
        if range.start != expected_start || range.end != expected_end {
            return Err(schema(company_id, "listing page range drifted"));
        }
        for anchor in Html::parse_document(raw).select(&selector) {
            let href = anchor.value().attr("href").expect("selector requires href");
            let url = base
                .join(href)
                .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
            if url.scheme() != "https"
                || url.host_str() != base.host_str()
                || !url.path().starts_with("/vacatures/")
            {
                return Err(schema(
                    company_id,
                    "vacancy URL is not official Eneco HTTPS",
                ));
            }
            let id = url
                .path()
                .rsplit('-')
                .next()
                .filter(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_digit()))
                .ok_or_else(|| schema(company_id, "vacancy URL has no numeric ID"))?
                .to_owned();
            let title = anchor
                .select(&title_selector)
                .next()
                .map(|element| element.text().collect::<String>())
                .map(|title| title.trim().to_owned())
                .filter(|title| !title.is_empty())
                .ok_or_else(|| schema(company_id, format!("vacancy {id} has no title")))?;
            if !ids.insert(id.clone()) {
                return Err(schema(company_id, format!("duplicate vacancy {id}")));
            }
            cards.push(Card { id, title, url });
        }
    }

    let total = total.ok_or_else(|| schema(company_id, "returned no listing pages"))?;
    if pages.len() != total.max(1).div_ceil(PAGE_SIZE) || cards.len() != total {
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
    cards
        .into_iter()
        .zip(details)
        .map(|(card, detail)| observed_job(company_id, card, detail))
        .collect()
}

fn observed_job(company_id: &str, card: Card, detail: &str) -> Result<ObservedJob, SourceError> {
    let posting = parse_job_posting(detail, "Eneco")?;
    let raw_posting = job_posting_value(detail, "Eneco")?;
    let title = posting
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no title", card.id)))?;
    if title != card.title {
        return Err(schema(
            company_id,
            format!("detail {} title mismatch", card.id),
        ));
    }
    if posting
        .hiring_organization
        .as_ref()
        .and_then(|organization| organization.name.as_deref())
        != Some("Eneco")
    {
        return Err(schema(
            company_id,
            format!("detail {} is not an Eneco job", card.id),
        ));
    }
    let published_at = posting
        .date_posted
        .as_deref()
        .ok_or_else(|| schema(company_id, format!("detail {} has no datePosted", card.id)))
        .and_then(|date| {
            DateTime::parse_from_rfc3339(date)
                .map(|date| date.with_timezone(&Utc))
                .map_err(|error| schema(company_id, format!("invalid datePosted: {error}")))
        })?;
    let description = html_text(&posting.description);
    let mut locations = Vec::new();
    let mut countries = Vec::new();
    for place in &posting.job_location {
        let location = place
            .name
            .as_deref()
            .or(place.address.address_locality.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| schema(company_id, format!("detail {} has no location", card.id)))?;
        let raw_country = place
            .address
            .address_country
            .as_deref()
            .ok_or_else(|| schema(company_id, format!("detail {} has no country", card.id)))?;
        let country = country_code_for_location(raw_country)
            .ok_or_else(|| schema(company_id, format!("unsupported country {raw_country}")))?;
        locations.push(location.to_owned());
        if !countries.contains(&country.to_owned()) {
            countries.push(country.to_owned());
        }
    }
    if locations.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} has no locations", card.id),
        ));
    }

    Ok(ObservedJob {
        source_id: card.id,
        title: title.to_owned(),
        department: Some("Tech".to_owned()),
        team: None,
        employment_type: posting.employment_type,
        locations,
        countries,
        job_url: card.url.to_string(),
        apply_url: card.url.to_string(),
        description,
        raw_payload: serde_json::json!({"jobPosting": raw_posting}),
        published_at: Some(published_at),
    })
}

fn page_range(raw: &str, company_id: &str) -> Result<PageRange, SourceError> {
    let selector = Selector::parse(".vacancy-count").expect("static selector must parse");
    let document = Html::parse_document(raw);
    let text = document
        .select(&selector)
        .next()
        .map(|element| element.text().collect::<String>())
        .ok_or_else(|| schema(company_id, "listing has no result count"))?;
    let parts = text.split_whitespace().collect::<Vec<_>>();
    if parts.len() != 5 || parts[1] != "t/m" || parts[3] != "van" {
        return Err(schema(company_id, "invalid listing result count"));
    }
    let number = |index: usize| {
        parts[index]
            .parse::<usize>()
            .map_err(|error| schema(company_id, format!("invalid result count: {error}")))
    };
    Ok(PageRange {
        start: number(0)?,
        end: number(2)?,
        total: number(4)?,
    })
}

fn page_url(value: &str, offset: usize, company_id: &str) -> Result<Url, SourceError> {
    let mut url = official_listing(value, company_id)?;
    url.query_pairs_mut().append_pair("o", &offset.to_string());
    Ok(url)
}

fn official_listing(value: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid listing URL: {error}")))?;
    let tech_filter = url
        .query_pairs()
        .any(|(key, value)| key == "f" && value == "1270");
    if url.scheme() != "https"
        || url.host_str() != Some("www.werkenbijeneco.nl")
        || url.path() != "/vacatures"
        || !tech_filter
    {
        return Err(schema(
            company_id,
            "listing is not the official Eneco Tech URL",
        ));
    }
    Ok(url)
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("Eneco response for {company_id}: {message}"))
}

struct Card {
    id: String,
    title: String,
    url: Url,
}

struct PageRange {
    start: usize,
    end: usize,
    total: usize,
}
