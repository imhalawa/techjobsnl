use std::collections::HashSet;

use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{ElementRef, Html, Selector};

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError,
    http::send_text,
    json_ld::{html_markdown, html_text},
};

pub struct ChipsoftSource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl ChipsoftSource {
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
impl JobSource for ChipsoftSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let listing = send_text(self.client.get(&self.listing_url), "ChipSoft").await?;
        let cards = parse_listing(&self.company_id, &self.listing_url, &listing)?
            .into_iter()
            .filter(|card| card.netherlands)
            .collect::<Vec<_>>();
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "ChipSoft job").await })
            .buffered(8)
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

pub fn parse_chipsoft_page(
    company_id: &str,
    listing_url: &str,
    listing: &str,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_listing(company_id, listing_url, listing)?
        .into_iter()
        .filter(|card| card.netherlands)
        .collect();
    parse_details(company_id, cards, details)
}

fn parse_listing(
    company_id: &str,
    listing_url: &str,
    listing: &str,
) -> Result<Vec<Card>, SourceError> {
    let base = official_listing_url(listing_url, company_id)?;
    let document = Html::parse_document(listing);
    let board_selector =
        Selector::parse("#results-container .vacancy-list").expect("static selector must parse");
    let boards = document.select(&board_selector).collect::<Vec<_>>();
    if boards.len() != 1 {
        return Err(schema(
            company_id,
            "board container is missing or duplicated",
        ));
    }
    let pagination_selector =
        Selector::parse(".pagination, nav[aria-label*=pagination]").expect("static selector");
    if document.select(&pagination_selector).next().is_some() {
        return Err(schema(company_id, "board unexpectedly uses pagination"));
    }
    let card_selector = Selector::parse("article.course-card").expect("static selector must parse");
    let link_selector =
        Selector::parse("a.stretched-link[href]").expect("static selector must parse");
    let title_selector = Selector::parse("h3.title").expect("static selector must parse");
    let location_selector =
        Selector::parse(".location-detail").expect("static selector must parse");
    let mut slugs = HashSet::new();
    let mut cards = Vec::new();

    for card in boards[0].select(&card_selector) {
        let link = card
            .select(&link_selector)
            .next()
            .ok_or_else(|| schema(company_id, "vacancy card has no link"))?;
        let url = base
            .join(link.value().attr("href").expect("selector requires href"))
            .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
        if url.scheme() != "https"
            || url.host_str() != base.host_str()
            || !url.path().starts_with("/nl-nl/werken-bij/vacatures/")
            || url.path() == "/nl-nl/werken-bij/vacatures/"
            || url.query().is_some()
        {
            return Err(schema(
                company_id,
                "vacancy URL left the official ChipSoft board",
            ));
        }
        let slug = url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|part| !part.is_empty()))
            .ok_or_else(|| schema(company_id, "vacancy URL has no slug"))?
            .to_owned();
        if !slugs.insert(slug.clone()) {
            return Err(schema(company_id, format!("duplicate vacancy {slug}")));
        }
        let title = required_text(card, &title_selector, company_id, "title")?;
        let location = required_text(card, &location_selector, company_id, "location")?;
        let netherlands = match location.as_str() {
            "Amsterdam" | "Heerenveen" | "Hoogeveen" | "Amsterdam Heerenveen Hoogeveen" => true,
            "Antwerpen" => false,
            _ => {
                return Err(schema(
                    company_id,
                    format!("vacancy {slug} has unknown location {location:?}"),
                ));
            }
        };
        cards.push(Card {
            slug,
            title,
            location,
            url,
            netherlands,
        });
    }
    if cards.is_empty() {
        return Err(schema(company_id, "board returned no vacancies"));
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
    let mut ids = HashSet::new();
    cards
        .into_iter()
        .zip(details)
        .map(|(card, detail)| observed_job(company_id, card, detail, &mut ids))
        .collect()
}

fn observed_job(
    company_id: &str,
    card: Card,
    detail: &str,
    ids: &mut HashSet<String>,
) -> Result<ObservedJob, SourceError> {
    let document = Html::parse_document(detail);
    let content_selector = Selector::parse("#vacancy-content").expect("static selector must parse");
    let content = document
        .select(&content_selector)
        .next()
        .ok_or_else(|| schema(company_id, format!("detail {} has no content", card.slug)))?;
    let title_selector = Selector::parse("h1.title").expect("static selector must parse");
    let location_selector =
        Selector::parse(".location-detail").expect("static selector must parse");
    let time_selector = Selector::parse(".time-detail").expect("static selector must parse");
    let description_selector = Selector::parse(".pe-lg-4").expect("static selector must parse");
    let title = required_text(content, &title_selector, company_id, "title")?;
    let location = required_text(content, &location_selector, company_id, "location")?;
    if title != card.title || location != card.location {
        return Err(schema(
            company_id,
            format!("detail {} does not match its listing card", card.slug),
        ));
    }
    let employment_type = required_text(content, &time_selector, company_id, "hours")?;
    let description = content
        .select(&description_selector)
        .next()
        .map(|element| html_markdown(&element.inner_html()))
        .filter(|description| !description.is_empty())
        .ok_or_else(|| {
            schema(
                company_id,
                format!("detail {} has no description", card.slug),
            )
        })?;
    let apply_selector = Selector::parse("main a[href*='/werken-bij/solliciteren/?vacancyId=']")
        .expect("static selector must parse");
    let apply = document
        .select(&apply_selector)
        .next()
        .ok_or_else(|| schema(company_id, format!("detail {} has no apply URL", card.slug)))?;
    let apply_url = card
        .url
        .join(apply.value().attr("href").expect("selector requires href"))
        .map_err(|error| schema(company_id, format!("invalid apply URL: {error}")))?;
    if apply_url.scheme() != "https"
        || apply_url.host_str() != card.url.host_str()
        || apply_url.path() != "/nl-nl/werken-bij/solliciteren/"
    {
        return Err(schema(
            company_id,
            "apply URL left the official ChipSoft form",
        ));
    }
    let query = apply_url.query_pairs().collect::<Vec<_>>();
    let id = match query.as_slice() {
        [(key, value)] if key == "vacancyId" && value.chars().all(|ch| ch.is_ascii_digit()) => {
            value.to_string()
        }
        _ => {
            return Err(schema(
                company_id,
                format!("detail {} apply URL has no stable vacancy ID", card.slug),
            ));
        }
    };
    if !ids.insert(id.clone()) {
        return Err(schema(company_id, format!("duplicate vacancy ID {id}")));
    }
    let job_url = card.url.to_string();
    let raw_payload = serde_json::json!({
        "slug": card.slug,
        "title": card.title,
        "location": card.location,
        "hours": employment_type,
        "description": description,
        "applyUrl": apply_url.to_string(),
    });

    Ok(ObservedJob {
        source_id: id,
        title: card.title,
        department: None,
        team: None,
        employment_type: Some(employment_type),
        locations: vec![card.location],
        countries: vec!["NL".to_owned()],
        apply_url: apply_url.to_string(),
        job_url,
        description,
        raw_payload,
        published_at: None,
    })
}

fn required_text(
    root: ElementRef<'_>,
    selector: &Selector,
    company_id: &str,
    field: &str,
) -> Result<String, SourceError> {
    root.select(selector)
        .next()
        .map(|element| html_text(&element.inner_html()))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| schema(company_id, format!("vacancy has no {field}")))
}

fn official_listing_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(raw)
        .map_err(|error| schema(company_id, format!("invalid listing URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.chipsoft.com")
        || !url
            .path()
            .trim_end_matches('/')
            .eq_ignore_ascii_case("/nl-nl/werken-bij/vacatures")
        || url.query().is_some()
    {
        return Err(schema(company_id, "unexpected ChipSoft listing URL"));
    }
    Ok(url)
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("ChipSoft response for {company_id}: {message}"))
}

struct Card {
    slug: String,
    title: String,
    location: String,
    url: Url,
    netherlands: bool,
}
