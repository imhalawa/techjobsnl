use std::collections::HashSet;

use chrono::{NaiveDateTime, TimeZone, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{Html, Selector};

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError,
    http::send_text,
    json_ld::{html_markdown, html_text, job_posting_value, parse_job_posting},
};

const PAGE_SIZE: usize = 20;

pub struct ExactSource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl ExactSource {
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
impl JobSource for ExactSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let first = send_text(self.client.get(&self.listing_url), "Exact").await?;
        let last_offset = listing_last_offset(&first, &self.listing_url, &self.company_id)?;
        let mut listings = vec![first];
        for offset in (PAGE_SIZE..=last_offset).step_by(PAGE_SIZE) {
            let mut url = Url::parse(&self.listing_url).map_err(|error| {
                SourceError::schema(format!("invalid Exact listing URL: {error}"))
            })?;
            url.query_pairs_mut()
                .append_pair("start", &offset.to_string());
            listings.push(send_text(self.client.get(url), "Exact").await?);
        }
        let listing_refs = listings.iter().map(String::as_str).collect::<Vec<_>>();
        let cards = parse_listings(&self.company_id, &self.listing_url, &listing_refs)?;
        let nl_cards = cards
            .into_iter()
            .filter(|card| card.netherlands)
            .collect::<Vec<_>>();
        let requests = nl_cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "Exact job").await })
            .buffered(6)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();

        Ok(SourceScan::Complete {
            observations: parse_details(&self.company_id, nl_cards, &detail_refs)?,
        })
    }
}

pub fn parse_exact_pages(
    company_id: &str,
    listing_url: &str,
    listings: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_listings(company_id, listing_url, listings)?
        .into_iter()
        .filter(|card| card.netherlands)
        .collect();
    parse_details(company_id, cards, details)
}

fn parse_listings(
    company_id: &str,
    listing_url: &str,
    pages: &[&str],
) -> Result<Vec<Card>, SourceError> {
    if pages.is_empty() {
        return Err(schema(company_id, "returned no listing pages"));
    }
    let base = official_listing_url(listing_url, company_id)?;
    let expected_last = listing_last_offset(pages[0], listing_url, company_id)?;
    if pages.len() != expected_last / PAGE_SIZE + 1 {
        return Err(schema(company_id, "listing page count mismatch"));
    }
    let card_selector = Selector::parse(".card--vacancy").expect("static selector must parse");
    let link_selector =
        Selector::parse("a[data-card-link][href]").expect("static selector must parse");
    let label_selector = Selector::parse(".label--ghost").expect("static selector must parse");
    let mut ids = HashSet::new();
    let mut cards = Vec::new();

    for (page_index, raw) in pages.iter().enumerate() {
        let document = Html::parse_document(raw);
        let page_cards = document.select(&card_selector).collect::<Vec<_>>();
        let is_last = page_index * PAGE_SIZE == expected_last;
        if page_cards.is_empty() || (!is_last && page_cards.len() != PAGE_SIZE) {
            return Err(schema(company_id, "listing page ended early"));
        }
        if page_cards.len() > PAGE_SIZE {
            return Err(schema(company_id, "listing page exceeded its page size"));
        }
        for card in page_cards {
            let link = card
                .select(&link_selector)
                .next()
                .ok_or_else(|| schema(company_id, "vacancy card has no link"))?;
            let href = link.value().attr("href").expect("selector requires href");
            let url = base
                .join(href)
                .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
            if url.scheme() != "https"
                || url.host_str() != base.host_str()
                || !url.path().starts_with("/careers/vacancies/")
                || url.query().is_some()
            {
                return Err(schema(
                    company_id,
                    "vacancy URL is not official Exact HTTPS",
                ));
            }
            let slug = url
                .path_segments()
                .and_then(Iterator::last)
                .filter(|slug| !slug.is_empty())
                .ok_or_else(|| schema(company_id, "vacancy URL has no slug"))?;
            let id = slug
                .split('-')
                .next()
                .filter(|id| !id.is_empty())
                .ok_or_else(|| schema(company_id, "vacancy URL has no ID"))?
                .to_owned();
            if !ids.insert(id.clone()) {
                return Err(schema(company_id, format!("duplicate vacancy {id}")));
            }
            let title = link.text().collect::<String>().trim().to_owned();
            if title.is_empty() {
                return Err(schema(company_id, format!("vacancy {id} has no title")));
            }
            let netherlands = card
                .select(&label_selector)
                .any(|label| label.text().collect::<String>().trim() == "Netherlands");
            cards.push(Card {
                id,
                title,
                url,
                netherlands,
            });
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
    let posting = parse_job_posting(detail, "Exact")?;
    let raw_payload = job_posting_value(detail, "Exact")?;
    let title = posting
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no title", card.id)))?;
    if html_text(title) != card.title {
        return Err(schema(
            company_id,
            format!("detail {} title mismatch", card.id),
        ));
    }
    if posting
        .hiring_organization
        .as_ref()
        .and_then(|organization| organization.name.as_deref())
        != Some("Exact")
    {
        return Err(schema(
            company_id,
            format!("detail {} is not an Exact job", card.id),
        ));
    }
    let published_at = posting
        .date_posted
        .as_deref()
        .ok_or_else(|| schema(company_id, format!("detail {} has no datePosted", card.id)))
        .and_then(|date| {
            NaiveDateTime::parse_from_str(date, "%Y-%m-%d %H:%M:%S")
                .map(|date| Utc.from_utc_datetime(&date))
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
        if !country.is_empty() && country != "NL" {
            return Err(schema(
                company_id,
                format!("detail {} is not in NL", card.id),
            ));
        }
        if let Some(location) = place
            .address
            .address_locality
            .as_deref()
            .map(str::trim)
            .filter(|location| !location.is_empty())
            .filter(|location| !locations.iter().any(|value| value == location))
        {
            locations.push(location.to_owned());
        }
    }
    if locations.is_empty() {
        locations.push("Netherlands".to_owned());
    }
    let decoded_description = html_text(&posting.description);
    let description = html_markdown(&decoded_description);
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} has no description", card.id),
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
        apply_url: format!("{job_url}/apply"),
        job_url,
        description,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn listing_last_offset(
    html: &str,
    listing_url: &str,
    company_id: &str,
) -> Result<usize, SourceError> {
    let base = official_listing_url(listing_url, company_id)?;
    let document = Html::parse_document(html);
    let selector =
        Selector::parse("nav.pagination__wrapper a[href]").expect("static selector must parse");
    let mut last = 0;
    for anchor in document.select(&selector) {
        let Some(href) = anchor.value().attr("href") else {
            continue;
        };
        let url = base
            .join(href)
            .map_err(|error| schema(company_id, format!("invalid pagination URL: {error}")))?;
        if url.host_str() != base.host_str() || url.path() != base.path() {
            return Err(schema(company_id, "pagination URL left the Exact board"));
        }
        for (key, value) in url.query_pairs() {
            if key == "start" {
                let offset = value
                    .parse::<usize>()
                    .map_err(|error| schema(company_id, format!("invalid page offset: {error}")))?;
                if offset % PAGE_SIZE != 0 {
                    return Err(schema(company_id, "page offset is not aligned"));
                }
                last = last.max(offset);
            }
        }
    }
    Ok(last)
}

fn official_listing_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(raw)
        .map_err(|error| schema(company_id, format!("invalid listing URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.exact.com")
        || url.path() != "/careers/vacancies"
        || url.query().is_some()
    {
        return Err(schema(company_id, "unexpected Exact listing URL"));
    }
    Ok(url)
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("Exact response for {company_id}: {message}"))
}

struct Card {
    id: String,
    title: String,
    url: Url,
    netherlands: bool,
}
