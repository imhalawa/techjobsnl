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

const OFFICIAL_HOST: &str = "www.pay.nl";

pub struct PaySource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl PaySource {
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
impl JobSource for PaySource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let listing = send_text(self.client.get(&self.listing_url), "PAY.").await?;
        let cards = parse_listing(&self.company_id, &self.listing_url, &listing)?;
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "PAY. job").await })
            .buffered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let details = details.iter().map(String::as_str).collect::<Vec<_>>();

        Ok(SourceScan::Complete {
            observations: parse_details(&self.company_id, cards, &details)?,
        })
    }
}

pub fn parse_pay_pages(
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
    let document = Html::parse_document(listing);
    let board_selector =
        Selector::parse("section.section-vacancyoverview#vacatures").expect("static selector");
    let boards = document.select(&board_selector).collect::<Vec<_>>();
    if boards.len() != 1 {
        return Err(schema(company_id, "vacancy board is missing or duplicated"));
    }
    let pagination_selector = Selector::parse(".pagination").expect("static selector");
    if boards[0].select(&pagination_selector).next().is_some() {
        return Err(schema(
            company_id,
            "vacancy board unexpectedly uses pagination",
        ));
    }
    let list_selector = Selector::parse("ul.list-vacancy").expect("static selector");
    let lists = boards[0].select(&list_selector).collect::<Vec<_>>();
    if lists.len() != 1 {
        return Err(schema(company_id, "vacancy list is missing or duplicated"));
    }
    let card_selector = Selector::parse("li").expect("static selector");
    let title_selector = Selector::parse("h5").expect("static selector");
    let field_selector = Selector::parse("p").expect("static selector");
    let link_selector = Selector::parse("a").expect("static selector");
    let status_selector = Selector::parse(".btn-content").expect("static selector");
    let mut urls = HashSet::new();
    let mut cards = Vec::new();

    for card in lists[0].select(&card_selector) {
        let title = required_text(card, &title_selector, company_id, "title")?;
        let fields = card
            .select(&field_selector)
            .map(|field| html_text(&field.inner_html()))
            .filter(|field| !field.is_empty())
            .collect::<Vec<_>>();
        let [department, location] = fields.as_slice() else {
            return Err(schema(
                company_id,
                format!("vacancy {title:?} does not have department and location"),
            ));
        };
        let status = required_text(card, &status_selector, company_id, "status")?;
        match status.as_str() {
            "Reeds vervuld" => continue,
            "Bekijk vacature" => {}
            _ => {
                return Err(schema(
                    company_id,
                    format!("vacancy {title:?} has unknown status {status:?}"),
                ));
            }
        }
        let href = card
            .select(&link_selector)
            .next()
            .and_then(|link| link.value().attr("href"))
            .ok_or_else(|| schema(company_id, format!("vacancy {title:?} has no link")))?;
        let url = base
            .join(href)
            .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
        validate_job_url(&url, company_id)?;
        if !urls.insert(url.clone()) {
            return Err(schema(company_id, format!("duplicate vacancy URL {url}")));
        }
        cards.push(Card {
            title,
            department: department.clone(),
            location: location.clone(),
            url,
        });
    }
    if cards.is_empty() {
        return Err(schema(company_id, "vacancy board returned no active jobs"));
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
    let intro_selector = Selector::parse(".intro-vacancy").expect("static selector");
    let intros = document.select(&intro_selector).collect::<Vec<_>>();
    let [intro] = intros.as_slice() else {
        return Err(schema(
            company_id,
            format!("detail {} has no unique introduction", card.url),
        ));
    };
    let title_selector = Selector::parse(".intro__title h2").expect("static selector");
    let location_selector =
        Selector::parse(".intro__body .labeltag span").expect("static selector");
    let title = required_text(*intro, &title_selector, company_id, "detail title")?;
    let location = required_text(*intro, &location_selector, company_id, "detail location")?;
    if !title_matches(&card.title, &title) || !location_matches(&card.location, &location) {
        return Err(schema(
            company_id,
            format!("detail {} does not match its listing card", card.url),
        ));
    }
    let apply_selector = Selector::parse("a[href]").expect("static selector");
    let apply_urls = intro
        .select(&apply_selector)
        .filter_map(|link| link.value().attr("href"))
        .filter_map(|href| Url::parse(href).ok())
        .filter(|url| url.host_str() == Some("www.nmbrshire.com"))
        .collect::<Vec<_>>();
    let [apply_url] = apply_urls.as_slice() else {
        return Err(schema(
            company_id,
            format!("detail {} has no unique Nmbrs apply URL", card.url),
        ));
    };
    let source_id = assignment_id(apply_url, company_id)?;
    if !ids.insert(source_id.clone()) {
        return Err(schema(
            company_id,
            format!("duplicate assignment ID {source_id}"),
        ));
    }
    let content_selector = Selector::parse(
        "section.text-module .richtext, section.section-vacancy-content .section__body",
    )
    .expect("static selector");
    let description = document
        .select(&content_selector)
        .map(|content| html_markdown(&content.inner_html()))
        .filter(|content| !content.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} has no description", card.url),
        ));
    }
    let job_url = card.url.to_string();
    let apply_url = apply_url.to_string();
    let raw_payload = serde_json::json!({
        "assignmentId": source_id,
        "title": title,
        "department": card.department,
        "location": card.location,
        "jobUrl": job_url,
        "applyUrl": apply_url,
        "description": description,
    });

    Ok(ObservedJob {
        source_id,
        title,
        department: Some(card.department),
        team: None,
        employment_type: None,
        locations: vec![card.location],
        countries: vec!["NL".into()],
        job_url,
        apply_url,
        description,
        raw_payload,
        published_at: None,
    })
}

fn assignment_id(url: &Url, company_id: &str) -> Result<String, SourceError> {
    if url.scheme() != "https"
        || !matches!(url.path(), "/spa/public/apply" | "/spa/nl/public/apply")
        || url.fragment().is_some()
    {
        return Err(schema(company_id, "unexpected Nmbrs apply URL"));
    }
    let query = url.query_pairs().collect::<Vec<_>>();
    let value = match query.as_slice() {
        [(key, value)] if key == "guidAssignment" && url.path() == "/spa/public/apply" => value,
        [(key, value), (locale_key, locale_value)]
            if key == "guidAssignment"
                && locale_key == "forcelocale"
                && locale_value == "true"
                && url.path() == "/spa/nl/public/apply" =>
        {
            value
        }
        _ => return Err(schema(company_id, "apply URL has unexpected parameters")),
    };
    if !valid_guid(value) {
        return Err(schema(
            company_id,
            "apply URL has no stable assignment GUID",
        ));
    }
    Ok(value.to_string())
}

fn location_matches(listing: &str, detail: &str) -> bool {
    let concrete = |value: &str| {
        let mut parts = value
            .split(['/', ','])
            .map(str::trim)
            .filter(|part| !matches!(part.to_ascii_lowercase().as_str(), "hybrid" | "hybride"))
            .map(str::to_owned)
            .collect::<Vec<_>>();
        parts.sort_unstable();
        parts
    };
    concrete(listing) == concrete(detail)
}

fn title_matches(listing: &str, detail: &str) -> bool {
    let listing = listing.to_ascii_lowercase();
    let detail = detail.to_ascii_lowercase();
    detail == listing
        || detail
            .strip_prefix(&listing)
            .is_some_and(|suffix| suffix.starts_with(" (") && suffix.ends_with(')'))
}

fn valid_guid(value: &str) -> bool {
    value.len() == 36
        && value.chars().enumerate().all(|(index, ch)| {
            if [8, 13, 18, 23].contains(&index) {
                ch == '-'
            } else {
                ch.is_ascii_hexdigit()
            }
        })
}

fn official_listing_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(raw)
        .map_err(|error| schema(company_id, format!("invalid listing URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some(OFFICIAL_HOST)
        || url.path().trim_end_matches('/') != "/werk"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(schema(company_id, "unexpected PAY. listing URL"));
    }
    Ok(url)
}

fn validate_job_url(url: &Url, company_id: &str) -> Result<(), SourceError> {
    if url.scheme() != "https"
        || url.host_str() != Some(OFFICIAL_HOST)
        || url.path() == "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(schema(company_id, "vacancy URL is not official PAY. HTTPS"));
    }
    Ok(())
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

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("PAY. response for {company_id}: {message}"))
}

struct Card {
    title: String,
    department: String,
    location: String,
    url: Url,
}
