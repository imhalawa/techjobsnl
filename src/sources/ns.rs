use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use regex::Regex;
use reqwest::{Client, Url};
use scraper::{Html, Selector};

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError,
    http::send_text,
    json_ld::{html_markdown, html_text, job_posting_value, parse_job_posting},
};

const PAGE_SIZE: usize = 10;

pub struct NsSource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl NsSource {
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
impl JobSource for NsSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let first = send_text(self.client.get(&self.listing_url), "NS").await?;
        let (_, _, total) = listing_range(&first, &self.company_id)?;
        let page_count = total.div_ceil(PAGE_SIZE);
        let base = official_listing_url(&self.listing_url, &self.company_id)?;
        let requests = (1..page_count)
            .map(|page| {
                let mut url = base.clone();
                url.query_pairs_mut()
                    .append_pair("o", &(page * PAGE_SIZE).to_string());
                (self.client.clone(), url)
            })
            .collect::<Vec<_>>();
        let rest = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "NS").await })
            .buffered(6)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let mut listings = vec![first];
        listings.extend(rest);
        let listing_refs = listings.iter().map(String::as_str).collect::<Vec<_>>();
        let cards = parse_listings(
            &self.company_id,
            &self.listing_url,
            &listing_refs,
            PAGE_SIZE,
        )?;
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "NS job").await })
            .buffered(12)
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

pub fn parse_ns_pages(
    company_id: &str,
    listing_url: &str,
    listings: &[&str],
    details: &[&str],
    page_size: usize,
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_listings(company_id, listing_url, listings, page_size)?;
    parse_details(company_id, cards, details)
}

fn parse_listings(
    company_id: &str,
    listing_url: &str,
    pages: &[&str],
    page_size: usize,
) -> Result<Vec<Card>, SourceError> {
    if pages.is_empty() || page_size == 0 {
        return Err(schema(company_id, "returned no valid listing pages"));
    }
    let base = official_listing_url(listing_url, company_id)?;
    let (_, _, total) = listing_range(pages[0], company_id)?;
    if total == 0 || pages.len() != total.div_ceil(page_size) {
        return Err(schema(company_id, "listing page count mismatch"));
    }
    let card_selector = Selector::parse(".vacancy-item-cell").expect("static selector must parse");
    let link_selector =
        Selector::parse("a.vacancy-item[href]").expect("static selector must parse");
    let title_selector = Selector::parse("h3").expect("static selector must parse");
    let mut ids = HashSet::new();
    let mut cards = Vec::with_capacity(total);

    for (index, raw) in pages.iter().enumerate() {
        let expected_start = index * page_size + 1;
        let expected_end = total.min(expected_start + page_size - 1);
        let (start, end, page_total) = listing_range(raw, company_id)?;
        if (start, end, page_total) != (expected_start, expected_end, total) {
            return Err(schema(company_id, "listing range or total drifted"));
        }
        let document = Html::parse_document(raw);
        let page_cards = document.select(&card_selector).collect::<Vec<_>>();
        if page_cards.len() != end - start + 1 {
            return Err(schema(company_id, "listing card count mismatch"));
        }
        for card in page_cards {
            let link = card
                .select(&link_selector)
                .next()
                .ok_or_else(|| schema(company_id, "vacancy card has no link"))?;
            let url = base
                .join(link.value().attr("href").expect("selector requires href"))
                .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
            if url.scheme() != "https"
                || url.host_str() != base.host_str()
                || !url.path().starts_with("/vacatures/")
                || url.query().is_some()
            {
                return Err(schema(company_id, "vacancy URL left the official NS board"));
            }
            let slug = url
                .path_segments()
                .and_then(Iterator::last)
                .filter(|slug| !slug.is_empty())
                .ok_or_else(|| schema(company_id, "vacancy URL has no slug"))?;
            let id = slug
                .rsplit('-')
                .next()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| schema(company_id, "vacancy URL has no ID"))?
                .to_owned();
            if !ids.insert(id.clone()) {
                return Err(schema(company_id, format!("duplicate vacancy {id}")));
            }
            let title = card
                .select(&title_selector)
                .next()
                .map(|title| title.text().collect::<String>())
                .map(|title| title.split_whitespace().collect::<Vec<_>>().join(" "))
                .filter(|title| !title.is_empty())
                .ok_or_else(|| schema(company_id, format!("vacancy {id} has no title")))?;
            cards.push(Card { id, title, url });
        }
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
    let posting = parse_job_posting(detail, "NS")
        .map_err(|error| schema(company_id, format!("detail {}: {error}", card.id)))?;
    let raw_payload = job_posting_value(detail, "NS")?;
    let title = posting
        .title
        .as_deref()
        .map(html_text)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no title", card.id)))?;
    if title != card.title {
        return Err(schema(
            company_id,
            format!(
                "detail {} title mismatch: listing {:?}, detail {:?}",
                card.id, card.title, title
            ),
        ));
    }
    if posting
        .hiring_organization
        .as_ref()
        .and_then(|organization| organization.name.as_deref())
        != Some("NS")
    {
        return Err(schema(
            company_id,
            format!("detail {} is not an NS job", card.id),
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
    let mut locations = Vec::new();
    for place in posting.job_location {
        let country = place
            .address
            .address_country
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        if !country.is_empty() && country != "Nederland" {
            return Err(schema(
                company_id,
                format!("detail {} is not in the Netherlands", card.id),
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
    if locations.is_empty() {
        let country_pattern = Regex::new(r#""country"\s*:\s*"Netherlands""#)
            .expect("static NS country pattern must compile");
        if !country_pattern.is_match(detail) {
            return Err(schema(
                company_id,
                format!("detail {} has no Netherlands location", card.id),
            ));
        }
        locations.push("Netherlands".to_owned());
    }
    let description = html_markdown(&posting.description);
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} is incomplete", card.id),
        ));
    }
    let job_url = card.url.to_string();

    Ok(ObservedJob {
        source_id: card.id,
        title: card.title,
        department: None,
        team: None,
        employment_type: posting.employment_type,
        locations,
        countries: vec!["NL".to_owned()],
        apply_url: format!("{job_url}/solliciteer"),
        job_url,
        description,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn listing_range(html: &str, company_id: &str) -> Result<(usize, usize, usize), SourceError> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(".vacancy-count").expect("static selector must parse");
    let text = document
        .select(&selector)
        .next()
        .map(|element| element.text().collect::<String>())
        .ok_or_else(|| schema(company_id, "listing has no result count"))?;
    let pattern = Regex::new(r"([0-9]+)\s+t/m\s+([0-9]+)\s+van\s+([0-9]+)")
        .expect("static NS count pattern must compile");
    let captures = pattern
        .captures(&text)
        .ok_or_else(|| schema(company_id, "listing has an invalid result count"))?;
    let number = |index: usize| {
        captures[index]
            .parse::<usize>()
            .map_err(|error| schema(company_id, format!("invalid result count: {error}")))
    };
    Ok((number(1)?, number(2)?, number(3)?))
}

fn official_listing_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(raw)
        .map_err(|error| schema(company_id, format!("invalid listing URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.werkenbijns.nl")
        || url.path() != "/vacatures"
        || url.query().is_some()
    {
        return Err(schema(company_id, "unexpected NS listing URL"));
    }
    Ok(url)
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("NS response for {company_id}: {message}"))
}

struct Card {
    id: String,
    title: String,
    url: Url,
}
