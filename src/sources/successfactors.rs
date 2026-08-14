use std::collections::HashSet;

use chrono::{NaiveDateTime, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{ElementRef, Html, Selector};

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

pub struct SuccessFactorsSource {
    company_id: String,
    listing_url: String,
    employer: String,
    client: Client,
}

impl SuccessFactorsSource {
    pub fn new(
        company_id: impl Into<String>,
        listing_url: impl Into<String>,
        employer: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            company_id: company_id.into(),
            listing_url: listing_url.into(),
            employer: employer.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for SuccessFactorsSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let first = send_text(self.client.get(&self.listing_url), "SAP SuccessFactors").await?;
        let (total, page_size) = listing_summary(&first, &self.company_id)?;
        let base = official_listing_url(&self.listing_url, &self.company_id)?;
        let requests = (1..total.div_ceil(page_size))
            .map(|page| {
                let mut url = base.clone();
                url.query_pairs_mut()
                    .append_pair("startrow", &(page * page_size).to_string());
                (self.client.clone(), url)
            })
            .collect::<Vec<_>>();
        let rest =
            stream::iter(requests)
                .map(|(client, url)| async move {
                    send_text(client.get(url), "SAP SuccessFactors").await
                })
                .buffered(4)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
        let mut listings = vec![first];
        listings.extend(rest);
        let listing_refs = listings.iter().map(String::as_str).collect::<Vec<_>>();
        let cards = parse_listings(&self.company_id, &self.listing_url, &listing_refs)?;
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move {
                send_text(client.get(url), "SAP SuccessFactors job").await
            })
            .buffered(8)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();

        Ok(SourceScan::Complete {
            observations: parse_details(&self.company_id, &self.employer, cards, &detail_refs)?,
        })
    }
}

pub fn parse_successfactors_pages(
    company_id: &str,
    listing_url: &str,
    employer: &str,
    listings: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_listings(company_id, listing_url, listings)?;
    parse_details(company_id, employer, cards, details)
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
    let (total, page_size) = listing_summary(pages[0], company_id)?;
    if total == 0 || pages.len() != total.div_ceil(page_size) {
        return Err(schema(company_id, "listing page count mismatch"));
    }

    let board_selector = Selector::parse("#job-tile-list").expect("static selector must parse");
    let card_selector =
        Selector::parse(":scope > li.job-tile[data-url]").expect("static selector must parse");
    let title_selector =
        Selector::parse("a.jobTitle-link[href]").expect("static selector must parse");
    let mut ids = HashSet::new();
    let mut cards = Vec::with_capacity(total);

    for (index, raw) in pages.iter().enumerate() {
        let (page_total, current_page_size) = listing_summary(raw, company_id)?;
        if page_total != total || current_page_size != page_size {
            return Err(schema(company_id, "listing summary changed between pages"));
        }
        let document = Html::parse_document(raw);
        let boards = document.select(&board_selector).collect::<Vec<_>>();
        if boards.len() != 1 {
            return Err(schema(company_id, "job list is missing or duplicated"));
        }
        let page_cards = boards[0].select(&card_selector).collect::<Vec<_>>();
        let expected = page_size.min(total - index * page_size);
        let returned = required_attr(boards[0], "data-record-returned", company_id, "page count")?
            .parse::<usize>()
            .map_err(|error| schema(company_id, format!("invalid page count: {error}")))?;
        if page_cards.len() != expected || returned != expected {
            return Err(schema(company_id, "listing card count mismatch"));
        }

        for card in page_cards {
            let href = required_attr(card, "data-url", company_id, "job URL")?;
            let url = base
                .join(href)
                .map_err(|error| schema(company_id, format!("invalid job URL: {error}")))?;
            let id = official_job_id(&url, &base, company_id)?;
            if !ids.insert(id.clone()) {
                return Err(schema(company_id, format!("duplicate vacancy {id}")));
            }
            let title = card
                .select(&title_selector)
                .next()
                .map(|element| normalized_text(element))
                .filter(|title| !title.is_empty())
                .ok_or_else(|| schema(company_id, format!("vacancy {id} has no title")))?;
            cards.push(Card { id, title, url });
        }
    }
    if cards.len() != total {
        return Err(schema(
            company_id,
            format!("incomplete listing: expected {total}, got {}", cards.len()),
        ));
    }
    Ok(cards)
}

fn parse_details(
    company_id: &str,
    employer: &str,
    cards: Vec<Card>,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    if cards.len() != details.len() {
        return Err(schema(company_id, "listing/detail count mismatch"));
    }
    cards
        .into_iter()
        .zip(details)
        .map(|(card, detail)| observed_job(company_id, employer, card, detail))
        .collect()
}

fn observed_job(
    company_id: &str,
    employer: &str,
    card: Card,
    detail: &str,
) -> Result<ObservedJob, SourceError> {
    let document = Html::parse_document(detail);
    let posting_selector =
        Selector::parse(".jobDisplayShell[itemscope][itemtype='http://schema.org/JobPosting']")
            .expect("static selector must parse");
    let postings = document.select(&posting_selector).collect::<Vec<_>>();
    if postings.len() != 1 {
        return Err(schema(
            company_id,
            "detail job posting is missing or duplicated",
        ));
    }
    let posting = postings[0];
    let title_selector = Selector::parse("[itemprop='title']").expect("static selector must parse");
    let title = posting
        .select(&title_selector)
        .next()
        .map(normalized_text)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no title", card.id)))?;
    if title != card.title {
        return Err(schema(
            company_id,
            format!("detail {} title mismatch", card.id),
        ));
    }

    let employer_selector =
        Selector::parse("meta[itemprop='hiringOrganization'][content]").expect("static selector");
    let actual_employer = posting
        .select(&employer_selector)
        .next()
        .and_then(|element| element.value().attr("content"))
        .map(str::trim);
    if actual_employer != Some(employer) {
        return Err(schema(
            company_id,
            format!("detail {} employer mismatch", card.id),
        ));
    }

    let location_selector =
        Selector::parse("meta[itemprop='streetAddress'][content]").expect("static selector");
    let street_address = posting
        .select(&location_selector)
        .next()
        .and_then(|element| element.value().attr("content"))
        .map(str::trim)
        .filter(|location| !location.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no location", card.id)))?;
    let parts = street_address.split(',').map(str::trim).collect::<Vec<_>>();
    if parts.len() < 2 || parts[0].is_empty() || parts[1] != "NL" {
        return Err(schema(
            company_id,
            format!("detail {} is not in NL", card.id),
        ));
    }

    let description_selector = Selector::parse(".jobdescription").expect("static selector");
    let description = posting
        .select(&description_selector)
        .next()
        .map(|element| html_markdown(&element.inner_html()))
        .filter(|description| !description.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no description", card.id)))?;

    let date_selector =
        Selector::parse("meta[itemprop='datePosted'][content]").expect("static selector");
    let raw_date = posting
        .select(&date_selector)
        .next()
        .and_then(|element| element.value().attr("content"))
        .ok_or_else(|| schema(company_id, format!("detail {} has no datePosted", card.id)))?;
    let published_at = NaiveDateTime::parse_from_str(raw_date, "%a %b %d %H:%M:%S UTC %Y")
        .map(|date| date.and_utc().with_timezone(&Utc))
        .map_err(|error| schema(company_id, format!("invalid datePosted: {error}")))?;

    let apply_selector =
        Selector::parse("a.apply.dialogApplyBtn[href]").expect("static selector must parse");
    let apply_href = posting
        .select(&apply_selector)
        .next()
        .and_then(|element| element.value().attr("href"))
        .ok_or_else(|| schema(company_id, format!("detail {} has no apply URL", card.id)))?;
    let apply_url = card
        .url
        .join(apply_href)
        .map_err(|error| schema(company_id, format!("invalid apply URL: {error}")))?;
    let expected_path = format!("/talentcommunity/apply/{}/", card.id);
    if apply_url.scheme() != "https"
        || apply_url.host_str() != card.url.host_str()
        || apply_url.path() != expected_path
        || !apply_url
            .query_pairs()
            .any(|(key, value)| key == "locale" && value == "en_US")
    {
        return Err(schema(company_id, "apply URL left the official form"));
    }

    Ok(ObservedJob {
        source_id: card.id,
        title,
        department: None,
        team: None,
        employment_type: None,
        locations: vec![parts[0].to_owned()],
        countries: vec!["NL".to_owned()],
        job_url: card.url.to_string(),
        apply_url: apply_url.to_string(),
        description,
        raw_payload: serde_json::json!({
            "employer": employer,
            "location": street_address,
            "datePosted": raw_date,
        }),
        published_at: Some(published_at),
    })
}

fn listing_summary(raw: &str, company_id: &str) -> Result<(usize, usize), SourceError> {
    let document = Html::parse_document(raw);
    let count_selector =
        Selector::parse("#tile-search-results-label").expect("static selector must parse");
    let count = document
        .select(&count_selector)
        .next()
        .map(normalized_text)
        .ok_or_else(|| schema(company_id, "listing has no result count"))?;
    let total = count
        .split_whitespace()
        .rev()
        .nth(1)
        .and_then(|value| value.replace(',', "").parse::<usize>().ok())
        .ok_or_else(|| schema(company_id, "invalid listing result count"))?;
    let board_selector = Selector::parse("#job-tile-list").expect("static selector must parse");
    let page_size = document
        .select(&board_selector)
        .next()
        .and_then(|board| board.value().attr("data-per-page"))
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| schema(company_id, "listing has no valid page size"))?;
    Ok((total, page_size))
}

fn official_listing_url(value: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid listing URL: {error}")))?;
    let nl_filter = url
        .query_pairs()
        .any(|(key, value)| key == "locationsearch" && value == "NL");
    if url.scheme() != "https" || url.path() != "/search/" || !nl_filter {
        return Err(schema(
            company_id,
            "listing is not an official NL search URL",
        ));
    }
    Ok(url)
}

fn official_job_id(url: &Url, base: &Url, company_id: &str) -> Result<String, SourceError> {
    if url.scheme() != "https"
        || url.host_str() != base.host_str()
        || !url.path().starts_with("/job/")
    {
        return Err(schema(company_id, "job URL left the official board"));
    }
    let segments = url
        .path_segments()
        .map(|segments| segments.filter(|part| !part.is_empty()).collect::<Vec<_>>())
        .unwrap_or_default();
    let id = segments
        .last()
        .filter(|id| id.bytes().all(|byte| byte.is_ascii_digit()))
        .ok_or_else(|| schema(company_id, "job URL has no numeric ID"))?;
    Ok((*id).to_owned())
}

fn required_attr<'a>(
    element: ElementRef<'a>,
    attribute: &str,
    company_id: &str,
    label: &str,
) -> Result<&'a str, SourceError> {
    element
        .value()
        .attr(attribute)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| schema(company_id, format!("listing has no {label}")))
}

fn normalized_text(element: ElementRef<'_>) -> String {
    element
        .text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!(
        "SAP SuccessFactors response for {company_id}: {message}"
    ))
}

struct Card {
    id: String,
    title: String,
    url: Url,
}
