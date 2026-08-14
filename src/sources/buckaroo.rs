use std::collections::{HashMap, HashSet};

use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

const OFFICIAL_HOST: &str = "www.buckaroo.nl";
const VACANCY_PATH: &str = "/over-buckaroo/vacatures/";

pub struct BuckarooSource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl BuckarooSource {
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
impl JobSource for BuckarooSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let listing_url = official_url(&self.listing_url, &self.company_id)?;
        let sitemap_url = listing_url
            .join("/sitemap.xml")
            .expect("official base URL joins a static path");
        let (listing, sitemap) = tokio::try_join!(
            send_text(self.client.get(listing_url), "Buckaroo"),
            send_text(self.client.get(sitemap_url), "Buckaroo sitemap")
        )?;
        let cards = parse_listing(&self.company_id, &self.listing_url, &listing, &sitemap)?;
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "Buckaroo").await })
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
    source_id: String,
    title: String,
    url: Url,
    last_modified: String,
}

#[derive(Deserialize)]
struct Sitemap {
    #[serde(rename = "url", default)]
    entries: Vec<SitemapEntry>,
}

#[derive(Deserialize)]
struct SitemapEntry {
    loc: String,
    lastmod: String,
}

fn official_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url =
        Url::parse(raw).map_err(|error| schema(company_id, format!("invalid URL: {error}")))?;
    if url.scheme() != "https" || url.host_str() != Some(OFFICIAL_HOST) {
        return Err(schema(company_id, "URL is not official Buckaroo HTTPS"));
    }
    Ok(url)
}

fn vacancy_id(url: &Url, company_id: &str) -> Result<String, SourceError> {
    if !url.path().starts_with(VACANCY_PATH) {
        return Err(schema(company_id, "URL is not a Buckaroo vacancy"));
    }
    let id = url
        .path()
        .trim_start_matches(VACANCY_PATH)
        .trim_end_matches('/');
    if id.is_empty() || id.contains('/') {
        return Err(schema(company_id, "vacancy URL has no stable slug"));
    }
    Ok(id.to_owned())
}

fn parse_listing(
    company_id: &str,
    listing_url: &str,
    listing: &str,
    sitemap: &str,
) -> Result<Vec<Card>, SourceError> {
    let base = official_url(listing_url, company_id)?;
    let sitemap: Sitemap = quick_xml::de::from_str(sitemap)
        .map_err(|error| schema(company_id, format!("invalid sitemap: {error}")))?;
    let mut sitemap_jobs = HashMap::new();
    for entry in sitemap.entries {
        let url = official_url(entry.loc.trim(), company_id)?;
        if url.path().starts_with(VACANCY_PATH) {
            let id = vacancy_id(&url, company_id)?;
            if sitemap_jobs.insert(url, (id, entry.lastmod)).is_some() {
                return Err(schema(company_id, "sitemap contains a duplicate vacancy"));
            }
        }
    }

    let document = Html::parse_document(listing);
    let selector =
        Selector::parse(r#"h3.card__title a.link--title[href^="/over-buckaroo/vacatures/"]"#)
            .expect("static Buckaroo selector");
    let mut seen = HashSet::new();
    let mut cards = Vec::new();
    for anchor in document.select(&selector) {
        let title = anchor
            .text()
            .flat_map(str::split_whitespace)
            .collect::<Vec<_>>()
            .join(" ");
        if title.is_empty() {
            return Err(schema(company_id, "listing vacancy has no title"));
        }
        let url = base
            .join(anchor.value().attr("href").expect("selector requires href"))
            .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
        official_url(url.as_str(), company_id)?;
        if !seen.insert(url.clone()) {
            return Err(schema(
                company_id,
                format!("duplicate listing vacancy {url}"),
            ));
        }
        let (source_id, last_modified) = sitemap_jobs.get(&url).cloned().ok_or_else(|| {
            schema(
                company_id,
                format!("listing vacancy missing from sitemap: {url}"),
            )
        })?;
        cards.push(Card {
            source_id,
            title,
            url,
            last_modified,
        });
    }
    if cards.is_empty() {
        return Err(schema(company_id, "listing contains no vacancies"));
    }
    if seen.len() != sitemap_jobs.len() {
        return Err(schema(
            company_id,
            format!(
                "incomplete listing: found {} cards but {} sitemap vacancies",
                seen.len(),
                sitemap_jobs.len()
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
    let document = Html::parse_document(detail);
    let canonical_selector = Selector::parse(r#"link[rel="canonical"][href]"#).unwrap();
    let canonical = document
        .select(&canonical_selector)
        .next()
        .and_then(|element| element.value().attr("href"))
        .ok_or_else(|| {
            schema(
                company_id,
                format!("detail {} has no canonical URL", card.source_id),
            )
        })?;
    if official_url(canonical, company_id)? != card.url {
        return Err(schema(
            company_id,
            format!("detail {} URL mismatch", card.source_id),
        ));
    }

    let item_selector = Selector::parse(".band.band--spacing .item.flow").unwrap();
    let title_selector = Selector::parse("h1").unwrap();
    let location_selector = Selector::parse("h4").unwrap();
    let item = document
        .select(&item_selector)
        .find(|item| item.select(&title_selector).next().is_some())
        .ok_or_else(|| {
            schema(
                company_id,
                format!("detail {} has no vacancy body", card.source_id),
            )
        })?;
    let title = item
        .select(&title_selector)
        .next()
        .expect("item was selected by h1")
        .text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ");
    if title != card.title && !card.title.starts_with(&format!("{title} - ")) {
        return Err(schema(
            company_id,
            format!("detail {} title mismatch", card.source_id),
        ));
    }
    let location_line = item
        .select(&location_selector)
        .next()
        .map(|element| element.text().collect::<String>())
        .ok_or_else(|| {
            schema(
                company_id,
                format!("detail {} has no location", card.source_id),
            )
        })?;
    let location = location_line
        .split_once('|')
        .map(|(_, location)| location.trim())
        .filter(|location| !location.is_empty())
        .ok_or_else(|| {
            schema(
                company_id,
                format!("detail {} has invalid location", card.source_id),
            )
        })?
        .to_owned();
    let description = html_markdown(&item.inner_html());
    if description.len() < 100 {
        return Err(schema(
            company_id,
            format!("detail {} has no full description", card.source_id),
        ));
    }

    Ok(ObservedJob {
        source_id: card.source_id,
        title,
        department: None,
        team: None,
        employment_type: None,
        locations: vec![location],
        countries: vec!["NL".into()],
        job_url: card.url.to_string(),
        apply_url: card.url.to_string(),
        description,
        raw_payload: serde_json::json!({ "sitemap_last_modified": card.last_modified }),
        published_at: None,
    })
}

fn schema(company_id: &str, message: impl Into<String>) -> SourceError {
    SourceError::schema(format!("{company_id}: {}", message.into()))
}

pub fn parse_buckaroo_pages(
    company_id: &str,
    listing_url: &str,
    listing: &str,
    sitemap: &str,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_listing(company_id, listing_url, listing, sitemap)?;
    parse_details(company_id, cards, details)
}
