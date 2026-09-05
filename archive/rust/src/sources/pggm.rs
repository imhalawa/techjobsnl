use std::collections::HashSet;

use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{ElementRef, Html, Selector};

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

const OFFICIAL_HOST: &str = "www.werkenbijpggm.nl";
const PAGE_SIZE: usize = 6;

pub struct PggmSource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl PggmSource {
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
impl JobSource for PggmSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let first = send_text(self.client.get(&self.listing_url), "PGGM").await?;
        let last_page = declared_last_page(&first, &self.company_id)?;
        let base = official_url(&self.listing_url, &self.company_id)?;
        let requests = (2..=last_page + 1)
            .map(|page| {
                let mut url = base.clone();
                url.query_pairs_mut().append_pair("p", &page.to_string());
                (self.client.clone(), url)
            })
            .collect::<Vec<_>>();
        let rest = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "PGGM").await })
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
            .map(|(client, url)| async move { send_text(client.get(url), "PGGM").await })
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

#[derive(Debug)]
struct Card {
    id: String,
    title: String,
    url: Url,
}

pub fn parse_pggm_pages(
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
    let base = official_url(listing_url, company_id)?;
    let last_page = declared_last_page(pages[0], company_id)?;
    if pages.len() != last_page + 1 {
        return Err(schema(company_id, "listing page count mismatch"));
    }

    let card_selector = Selector::parse("a.c-card[href]").expect("static selector");
    let title_selector = Selector::parse("h3").expect("static selector");
    let active_selector = Selector::parse(".pagination__item--active").expect("static selector");
    let mut ids = HashSet::new();
    let mut cards = Vec::new();

    for (index, raw) in pages[..last_page].iter().enumerate() {
        let page_number = index + 1;
        let document = Html::parse_document(raw);
        let active = document
            .select(&active_selector)
            .next()
            .map(text)
            .and_then(|value| value.parse::<usize>().ok())
            .ok_or_else(|| schema(company_id, "listing has no active page number"))?;
        if active != page_number {
            return Err(schema(company_id, "listing page order drifted"));
        }
        let page_cards = document.select(&card_selector).collect::<Vec<_>>();
        let expected_full = page_number < last_page;
        if (expected_full && page_cards.len() != page_size)
            || (!expected_full && !(1..=page_size).contains(&page_cards.len()))
        {
            return Err(schema(company_id, "listing card count drifted"));
        }
        for anchor in page_cards {
            let href = anchor.value().attr("href").expect("selector requires href");
            let url = base
                .join(href)
                .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
            validate_job_url(&url, company_id, "/vacatures/vacature/")?;
            let id = query_id(&url, company_id)?;
            if !ids.insert(id.clone()) {
                return Err(schema(company_id, format!("duplicate vacancy {id}")));
            }
            let title = anchor
                .select(&title_selector)
                .next()
                .map(text)
                .filter(|value| !value.is_empty())
                .ok_or_else(|| schema(company_id, format!("vacancy {id} has no title")))?;
            cards.push(Card { id, title, url });
        }
    }

    let sentinel = Html::parse_document(pages[last_page]);
    if sentinel.select(&card_selector).next().is_some() {
        return Err(schema(
            company_id,
            "jobs exist after the declared last page",
        ));
    }
    Ok(cards)
}

fn declared_last_page(html: &str, company_id: &str) -> Result<usize, SourceError> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(".pagination__list .pagination__item").expect("static selector");
    document
        .select(&selector)
        .filter_map(|item| text(item).parse::<usize>().ok())
        .max()
        .filter(|page| *page > 0)
        .ok_or_else(|| schema(company_id, "listing has no declared pagination"))
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
    let title_selector = Selector::parse("h1.hero__title").expect("static selector");
    let title = document
        .select(&title_selector)
        .next()
        .map(text)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no title", card.id)))?;
    if title != card.title {
        return Err(schema(
            company_id,
            format!("detail {} title mismatch", card.id),
        ));
    }

    let apply_selector =
        Selector::parse(r#"a[href^="/vacatures/solliciteren/"]"#).expect("static selector");
    let apply_url = document
        .select(&apply_selector)
        .next()
        .and_then(|anchor| anchor.value().attr("href"))
        .ok_or_else(|| schema(company_id, format!("detail {} has no apply URL", card.id)))
        .and_then(|href| {
            card.url
                .join(href)
                .map_err(|error| schema(company_id, format!("invalid apply URL: {error}")))
        })?;
    validate_job_url(&apply_url, company_id, "/vacatures/solliciteren/")?;
    if query_id(&apply_url, company_id)? != card.id {
        return Err(schema(
            company_id,
            format!("detail {} apply ID mismatch", card.id),
        ));
    }

    let paragraph_selector =
        Selector::parse("section.c-paragraph .s-rich-text").expect("static selector");
    let description_html = document
        .select(&paragraph_selector)
        .map(|paragraph| paragraph.html())
        .collect::<Vec<_>>()
        .join("\n");
    let description = html_markdown(&description_html);
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} has no description", card.id),
        ));
    }
    let department = specification(&document, "Vakgebied");
    Ok(ObservedJob {
        source_id: card.id.clone(),
        title,
        department,
        team: None,
        employment_type: None,
        // PGGM's 2025 annual report states that all employees work in the Netherlands.
        locations: vec!["Netherlands".into()],
        countries: vec!["NL".into()],
        job_url: card.url.to_string(),
        apply_url: apply_url.to_string(),
        description,
        raw_payload: serde_json::json!({
            "id": card.id,
            "source": "PGGM official careers",
        }),
        // PGGM's first-party pages do not publish a posting date.
        published_at: None,
    })
}

fn specification(document: &Html, label: &str) -> Option<String> {
    let item_selector = Selector::parse(".specification__item").expect("static selector");
    let label_selector = Selector::parse(".specification__title").expect("static selector");
    let value_selector = Selector::parse(".specification__value").expect("static selector");
    document.select(&item_selector).find_map(|item| {
        (item.select(&label_selector).next().map(text).as_deref() == Some(label))
            .then(|| item.select(&value_selector).next().map(text))
            .flatten()
            .filter(|value| !value.is_empty())
    })
}

fn official_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(raw)
        .map_err(|error| schema(company_id, format!("invalid PGGM URL: {error}")))?;
    if url.scheme() != "https" || url.host_str() != Some(OFFICIAL_HOST) {
        return Err(schema(company_id, "PGGM URL is not official HTTPS"));
    }
    Ok(url)
}

fn validate_job_url(url: &Url, company_id: &str, prefix: &str) -> Result<(), SourceError> {
    if url.scheme() != "https"
        || url.host_str() != Some(OFFICIAL_HOST)
        || !url.path().starts_with(prefix)
    {
        return Err(schema(company_id, "vacancy URL left the official board"));
    }
    Ok(())
}

fn query_id(url: &Url, company_id: &str) -> Result<String, SourceError> {
    let values = url
        .query_pairs()
        .filter(|(key, _)| key == "id")
        .map(|(_, value)| value.into_owned())
        .collect::<Vec<_>>();
    match values.as_slice() {
        [id] if id.starts_with("a0w")
            && id
                .chars()
                .all(|character| character.is_ascii_alphanumeric()) =>
        {
            Ok(id.clone())
        }
        _ => Err(schema(
            company_id,
            "vacancy URL has no unique Salesforce ID",
        )),
    }
}

fn text(element: ElementRef<'_>) -> String {
    element
        .text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

fn schema(company_id: &str, message: impl Into<String>) -> SourceError {
    SourceError::schema(format!("{company_id}: {}", message.into()))
}
