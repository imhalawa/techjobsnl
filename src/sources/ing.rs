use std::collections::HashSet;

use chrono::{NaiveDate, TimeZone, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde_json::Value;

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError,
    http::send_text,
    json_ld::{html_markdown, parse_job_posting},
};

pub struct IngSource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl IngSource {
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
impl JobSource for IngSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let snapshot = Utc::now().timestamp_millis().to_string();
        let first_url = cache_busted(
            official_url(&self.listing_url, "listing", &self.company_id)?,
            &snapshot,
        );
        let first = send_text(self.client.get(first_url), "ING").await?;
        let first_meta = listing_meta(&first, &self.company_id)?;
        let mut listings = vec![first];
        for page in 2..=first_meta.total_pages {
            listings.push(
                send_text(
                    self.client.get(cache_busted(
                        page_url(&self.listing_url, page, &self.company_id)?,
                        &snapshot,
                    )),
                    "ING",
                )
                .await?,
            );
        }
        let listing_refs = listings.iter().map(String::as_str).collect::<Vec<_>>();
        let cards = parse_listing_pages(&self.company_id, &self.listing_url, &listing_refs)?;

        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "ING").await })
            .buffered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();
        let observations = parse_details(&self.company_id, cards, &detail_refs)?;
        Ok(SourceScan::Complete { observations })
    }
}

pub fn parse_ing_pages(
    company_id: &str,
    listing_url: &str,
    listings: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_listing_pages(company_id, listing_url, listings)?;
    parse_details(company_id, cards, details)
}

fn parse_listing_pages(
    company_id: &str,
    listing_url: &str,
    pages: &[&str],
) -> Result<Vec<IngCard>, SourceError> {
    let base = official_url(listing_url, "listing", company_id)?;
    let mut expected: Option<ListingMeta> = None;
    let mut ids = HashSet::new();
    let mut cards = Vec::new();

    for (index, raw) in pages.iter().enumerate() {
        let requested_page = index + 1;
        let document = Html::parse_document(raw);
        let meta = listing_meta_from_document(&document, company_id)?;
        if meta.current_page != requested_page {
            return Err(schema(company_id, "listing current-page metadata drifted"));
        }
        if let Some(expected) = &expected {
            if expected.total != meta.total
                || expected.total_pages != meta.total_pages
                || expected.records_per_page != meta.records_per_page
            {
                return Err(schema(company_id, "listing metadata changed between pages"));
            }
        } else {
            expected = Some(meta.clone());
        }
        if requested_page > meta.total_pages {
            return Err(schema(
                company_id,
                "returned more listing pages than declared",
            ));
        }

        let card_selector =
            Selector::parse("li.search-results-item").expect("static ING card selector");
        let anchor_selector =
            Selector::parse("a[data-job-id][href]").expect("static ING anchor selector");
        let mut page_cards = Vec::new();
        for item in document.select(&card_selector) {
            let anchor = item
                .select(&anchor_selector)
                .next()
                .ok_or_else(|| schema(company_id, "listing card has no ID/detail link"))?;
            let id = anchor
                .value()
                .attr("data-job-id")
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .ok_or_else(|| schema(company_id, "listing card has an empty ID"))?;
            let href = anchor
                .value()
                .attr("href")
                .map(str::trim)
                .filter(|href| !href.is_empty())
                .ok_or_else(|| schema(company_id, "listing card has an empty detail link"))?;
            let url = base
                .join(href)
                .map_err(|error| schema(company_id, format!("invalid card URL: {error}")))?;
            require_same_official_origin(&base, &url, "detail", company_id)?;
            if url
                .path_segments()
                .and_then(|mut segments| segments.next_back())
                != Some(id)
            {
                return Err(schema(
                    company_id,
                    format!("listing card ID {id} does not match its detail URL"),
                ));
            }
            if !ids.insert(id.to_owned()) {
                return Err(schema(
                    company_id,
                    format!("duplicate listing card ID {id}"),
                ));
            }
            page_cards.push(IngCard {
                id: id.to_owned(),
                href: href.to_owned(),
                url,
            });
        }

        let prior = cards.len();
        let remaining = meta.total.saturating_sub(prior);
        let expected_on_page = remaining.min(meta.records_per_page);
        if page_cards.len() != expected_on_page {
            return Err(schema(
                company_id,
                format!(
                    "listing page {requested_page} returned {} of {expected_on_page} expected cards",
                    page_cards.len()
                ),
            ));
        }
        cards.extend(page_cards);
    }

    let meta = expected.ok_or_else(|| schema(company_id, "returned no listing pages"))?;
    if pages.len() != meta.total_pages || cards.len() != meta.total {
        return Err(schema(
            company_id,
            format!(
                "listing returned {} pages and {} cards; expected {} pages and {} cards",
                pages.len(),
                cards.len(),
                meta.total_pages,
                meta.total
            ),
        ));
    }
    Ok(cards)
}

fn parse_details(
    company_id: &str,
    cards: Vec<IngCard>,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    if cards.len() != details.len() {
        return Err(schema(
            company_id,
            format!(
                "received {} details for {} listing cards",
                details.len(),
                cards.len()
            ),
        ));
    }
    let mut source_ids = HashSet::new();
    cards
        .into_iter()
        .zip(details)
        .map(|(card, detail)| {
            let observation = observed_job(company_id, card, detail)?;
            if !source_ids.insert(observation.source_id.clone()) {
                return Err(schema(
                    company_id,
                    format!("duplicate detail identifier {}", observation.source_id),
                ));
            }
            Ok(observation)
        })
        .collect()
}

fn observed_job(company_id: &str, card: IngCard, detail: &str) -> Result<ObservedJob, SourceError> {
    let posting = parse_job_posting(detail, "ING")?;
    let raw_posting = raw_job_posting(detail, company_id)?;
    let source_id = posting
        .identifier
        .as_ref()
        .map(|identifier| identifier.value.trim())
        .filter(|identifier| !identifier.is_empty())
        .ok_or_else(|| schema(company_id, "detail has a blank identifier"))?
        .to_owned();
    let canonical = posting.url.as_deref().ok_or_else(|| {
        schema(
            company_id,
            format!("detail {source_id} has no canonical URL"),
        )
    })?;
    let canonical = official_url(canonical, "detail", company_id)?;
    let listing_base = official_url(card.url.as_ref(), "detail", company_id)?;
    require_same_official_origin(&listing_base, &canonical, "detail", company_id)?;
    if canonical != card.url {
        return Err(schema(
            company_id,
            format!("detail {source_id} canonical URL does not match its listing card"),
        ));
    }
    let apply_url = detail_meta(detail, "search-job-apply-url", company_id)?;
    let apply = official_url(&apply_url, "apply", company_id)?;

    let title = posting
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {source_id} has an empty title")))?
        .to_owned();
    let description = html_markdown(&posting.description);
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {source_id} has an empty description"),
        ));
    }
    let date = posting
        .date_posted
        .as_deref()
        .ok_or_else(|| schema(company_id, format!("detail {source_id} has no datePosted")))?;
    let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").map_err(|error| {
        schema(
            company_id,
            format!("detail {source_id} has invalid datePosted: {error}"),
        )
    })?;
    let published_at = Utc
        .from_local_datetime(&date.and_hms_opt(0, 0, 0).expect("midnight is valid"))
        .single()
        .expect("UTC local datetime is unique");

    if posting.job_location.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {source_id} has no locations"),
        ));
    }
    let mut locations = Vec::new();
    let mut countries = Vec::new();
    for place in &posting.job_location {
        let location = place
            .name
            .as_deref()
            .or(place.address.address_locality.as_deref())
            .map(str::trim)
            .filter(|location| !location.is_empty())
            .ok_or_else(|| {
                schema(
                    company_id,
                    format!("detail {source_id} has an unnamed location"),
                )
            })?;
        let country = place
            .address
            .address_country
            .as_deref()
            .map(str::trim)
            .filter(|country| !country.is_empty())
            .ok_or_else(|| {
                schema(
                    company_id,
                    format!("detail {source_id} has a location without country"),
                )
            })?;
        let country = normalized_country(country).ok_or_else(|| {
            schema(
                company_id,
                format!("detail {source_id} has unsupported country {country:?}"),
            )
        })?;
        locations.push(location.to_owned());
        if !countries.iter().any(|existing| existing == country) {
            countries.push(country.to_owned());
        }
    }

    let raw_payload = serde_json::json!({
        "listing": {
            "data-job-id": card.id,
            "href": card.href,
        },
        "jobPosting": raw_posting,
        "applyUrl": apply_url,
    });
    Ok(ObservedJob {
        source_id,
        title,
        department: None,
        team: None,
        employment_type: posting
            .employment_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned),
        locations,
        countries,
        job_url: canonical.to_string(),
        apply_url: apply.to_string(),
        description,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn normalized_country(country: &str) -> Option<&'static str> {
    match country {
        "NL" | "NLD" | "Netherlands" => Some("NL"),
        "BE" | "BEL" | "Belgium" => Some("BE"),
        _ => None,
    }
}

fn listing_meta(raw: &str, company_id: &str) -> Result<ListingMeta, SourceError> {
    listing_meta_from_document(&Html::parse_document(raw), company_id)
}

fn listing_meta_from_document(
    document: &Html,
    company_id: &str,
) -> Result<ListingMeta, SourceError> {
    let selector = Selector::parse("#search-results").expect("static ING metadata selector");
    let element = document
        .select(&selector)
        .next()
        .ok_or_else(|| schema(company_id, "listing has no search-results metadata"))?;
    let value = element.value();
    let meta = ListingMeta {
        total: usize_attr(
            value.attr("data-total-job-results"),
            "total results",
            company_id,
        )?,
        total_pages: usize_attr(value.attr("data-total-pages"), "total pages", company_id)?,
        current_page: usize_attr(value.attr("data-current-page"), "current page", company_id)?,
        records_per_page: usize_attr(
            value.attr("data-records-per-page"),
            "records per page",
            company_id,
        )?,
    };
    if meta.total_pages == 0 || meta.records_per_page == 0 {
        return Err(schema(company_id, "listing has zero page metadata"));
    }
    let calculated_pages = meta.total.div_ceil(meta.records_per_page).max(1);
    if meta.total_pages != calculated_pages {
        return Err(schema(
            company_id,
            "listing total-page metadata disagrees with total results",
        ));
    }
    Ok(meta)
}

fn usize_attr(value: Option<&str>, name: &str, company_id: &str) -> Result<usize, SourceError> {
    value
        .ok_or_else(|| schema(company_id, format!("listing is missing {name} metadata")))?
        .parse()
        .map_err(|error| schema(company_id, format!("invalid {name} metadata: {error}")))
}

fn page_url(listing_url: &str, page: usize, company_id: &str) -> Result<Url, SourceError> {
    let mut url = official_url(listing_url, "listing", company_id)?;
    url.path_segments_mut()
        .map_err(|()| schema(company_id, "listing URL cannot be a base"))?
        .pop()
        .pop()
        .push(&page.to_string());
    Ok(url)
}

fn cache_busted(mut url: Url, snapshot: &str) -> Url {
    url.query_pairs_mut().append_pair("_", snapshot);
    url
}

fn detail_meta(html: &str, name: &str, company_id: &str) -> Result<String, SourceError> {
    let document = Html::parse_document(html);
    let selector =
        Selector::parse(&format!(r#"meta[name="{name}"]"#)).expect("validated static meta name");
    document
        .select(&selector)
        .next()
        .and_then(|meta| meta.value().attr("content"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .ok_or_else(|| schema(company_id, format!("detail is missing {name}")))
}

fn raw_job_posting(html: &str, company_id: &str) -> Result<Value, SourceError> {
    let document = Html::parse_document(html);
    let selector =
        Selector::parse(r#"script[type="application/ld+json"]"#).expect("static JSON-LD selector");
    for script in document.select(&selector) {
        let raw = script.text().collect::<String>();
        let value: Value = serde_json::from_str(&raw)
            .map_err(|error| schema(company_id, format!("invalid detail JSON-LD: {error}")))?;
        if let Some(posting) = find_job_posting(&value) {
            return Ok(posting.clone());
        }
    }
    Err(schema(company_id, "detail has no JobPosting JSON-LD"))
}

fn find_job_posting(value: &Value) -> Option<&Value> {
    match value {
        Value::Object(object) if object.get("@type").is_some_and(is_job_posting_type) => {
            Some(value)
        }
        Value::Object(object) => object.get("@graph").and_then(find_job_posting),
        Value::Array(values) => values.iter().find_map(find_job_posting),
        _ => None,
    }
}

fn is_job_posting_type(value: &Value) -> bool {
    match value {
        Value::String(value) => value == "JobPosting",
        Value::Array(values) => values.iter().any(is_job_posting_type),
        _ => false,
    }
}

fn official_url(value: &str, kind: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid {kind} URL: {error}")))?;
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err(schema(company_id, format!("{kind} URL is not HTTPS")));
    }
    Ok(url)
}

fn require_same_official_origin(
    expected: &Url,
    actual: &Url,
    kind: &str,
    company_id: &str,
) -> Result<(), SourceError> {
    if expected.origin() != actual.origin() {
        return Err(schema(
            company_id,
            format!("{kind} URL is not on the configured official origin"),
        ));
    }
    Ok(())
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("ING response for {company_id}: {message}"))
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ListingMeta {
    total: usize,
    total_pages: usize,
    current_page: usize,
    records_per_page: usize,
}

struct IngCard {
    id: String,
    href: String,
    url: Url,
}
