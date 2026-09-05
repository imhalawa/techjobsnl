use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError,
    http::send_text,
    json_ld::{html_text, job_posting_value, parse_job_posting},
};

pub struct AlbertHeijnSource {
    company_id: String,
    base_url: String,
    client: Client,
}

impl AlbertHeijnSource {
    pub fn new(company_id: impl Into<String>, base_url: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            base_url: base_url.into(),
            client,
        }
    }

    async fn page(&self, page: usize) -> Result<String, SourceError> {
        send_text(
            self.client
                .get(listing_url(&self.base_url, page, &self.company_id)?)
                .header("x-requested-with", "XMLHttpRequest"),
            "Albert Heijn",
        )
        .await
    }
}

#[async_trait::async_trait]
impl JobSource for AlbertHeijnSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let first = self.page(1).await?;
        let meta = response_meta(&first, &self.company_id)?;
        let mut pages = vec![first];
        for page in 2..=meta.total_page_count {
            pages.push(self.page(page).await?);
        }
        let page_refs = pages.iter().map(String::as_str).collect::<Vec<_>>();
        let cards = parse_pages(&self.company_id, &self.base_url, &page_refs)?;
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "Albert Heijn").await })
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

pub fn parse_albert_heijn_pages(
    company_id: &str,
    base_url: &str,
    pages: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_pages(company_id, base_url, pages)?;
    parse_details(company_id, cards, details)
}

fn parse_pages(company_id: &str, base_url: &str, pages: &[&str]) -> Result<Vec<Card>, SourceError> {
    let base = official_base(base_url, company_id)?;
    let mut expected: Option<Meta> = None;
    let mut ids = HashSet::new();
    let mut cards = Vec::new();

    for (index, raw) in pages.iter().enumerate() {
        let response: Response = serde_json::from_str(raw)
            .map_err(|error| schema(company_id, format!("invalid page {}: {error}", index + 1)))?;
        if response.meta.page_number != index + 1 {
            return Err(schema(company_id, "page number drifted"));
        }
        if let Some(meta) = &expected {
            if !meta.same_snapshot(&response.meta) {
                return Err(schema(company_id, "pagination metadata changed"));
            }
        } else {
            expected = Some(response.meta.clone());
        }
        let remaining = response.meta.num_total_hits.saturating_sub(cards.len());
        if response.vacancies.len() != remaining.min(response.meta.max_per_page) {
            return Err(schema(
                company_id,
                format!("page {} is incomplete", index + 1),
            ));
        }
        for vacancy in response.vacancies {
            validate_vacancy(company_id, &vacancy)?;
            let id = vacancy.id.to_string();
            if !ids.insert(id.clone()) {
                return Err(schema(company_id, format!("duplicate vacancy {id}")));
            }
            let mut url = base.clone();
            url.set_path(&format!("/vacature/{id}/{}", vacancy.slug));
            cards.push(Card {
                id,
                title: vacancy.title.clone(),
                department: option(&vacancy, "Vakgebied"),
                employment_type: option(&vacancy, "Contract Type"),
                location: vacancy.city.clone(),
                url,
                raw: serde_json::to_value(vacancy).map_err(|error| {
                    schema(company_id, format!("could not preserve job: {error}"))
                })?,
            });
        }
    }

    let meta = expected.ok_or_else(|| schema(company_id, "returned no pages"))?;
    if meta.max_per_page == 0
        || meta.total_page_count != meta.num_total_hits.max(1).div_ceil(meta.max_per_page)
        || pages.len() != meta.total_page_count
        || cards.len() != meta.num_total_hits
    {
        return Err(schema(company_id, "incomplete pagination"));
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
    let posting = parse_job_posting(detail, "Albert Heijn")?;
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
        != Some("Albert Heijn")
    {
        return Err(schema(
            company_id,
            format!("detail {} is not Albert Heijn", card.id),
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
    if posting.job_location.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} has no location", card.id),
        ));
    }
    let mut locations = Vec::new();
    for place in &posting.job_location {
        let location = place
            .name
            .as_deref()
            .or(place.address.address_locality.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| schema(company_id, format!("detail {} has no city", card.id)))?;
        if !matches!(
            place.address.address_country.as_deref(),
            Some("NL" | "Netherlands" | "Nederland")
        ) {
            return Err(schema(
                company_id,
                format!("detail {} is outside NL", card.id),
            ));
        }
        locations.push(location.to_owned());
    }
    if !locations.contains(&card.location) {
        return Err(schema(
            company_id,
            format!("detail {} location mismatch", card.id),
        ));
    }
    let raw_posting = job_posting_value(detail, "Albert Heijn")?;

    Ok(ObservedJob {
        source_id: card.id,
        title: card.title,
        department: card.department,
        team: None,
        employment_type: card.employment_type,
        locations,
        countries: vec!["NL".to_owned()],
        job_url: card.url.to_string(),
        apply_url: card.url.to_string(),
        description: html_text(&posting.description),
        raw_payload: serde_json::json!({"listing": card.raw, "jobPosting": raw_posting}),
        published_at: Some(published_at),
    })
}

fn validate_vacancy(company_id: &str, vacancy: &Vacancy) -> Result<(), SourceError> {
    if vacancy.id == 0
        || vacancy.title.trim().is_empty()
        || vacancy.slug.trim().is_empty()
        || vacancy.city.trim().is_empty()
        || vacancy.company.name != "Albert Heijn"
        || option(vacancy, "Bedrijfsonderdeel").as_deref() != Some("Hoofdkantoor")
        || !matches!(
            option(vacancy, "Vakgebied").as_deref(),
            Some("IT" | "Data-science")
        )
    {
        return Err(schema(
            company_id,
            "vacancy violates the Albert Heijn Tech filter",
        ));
    }
    DateTime::parse_from_rfc3339(&vacancy.created)
        .map_err(|error| schema(company_id, format!("invalid created date: {error}")))?;
    Ok(())
}

fn option(vacancy: &Vacancy, title: &str) -> Option<String> {
    vacancy
        .option_values
        .iter()
        .find(|value| value.option.title == title)
        .map(|value| value.value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn response_meta(raw: &str, company_id: &str) -> Result<Meta, SourceError> {
    serde_json::from_str::<Response>(raw)
        .map(|response| response.meta)
        .map_err(|error| schema(company_id, format!("invalid first page: {error}")))
}

fn listing_url(base_url: &str, page: usize, company_id: &str) -> Result<Url, SourceError> {
    let mut url = official_base(base_url, company_id)?;
    url.set_path("/api/vacancy/");
    url.query_pairs_mut()
        .append_pair("filters[Bedrijfsonderdeel][]", "Hoofdkantoor")
        .append_pair("filters[Vakgebied][]", "IT")
        .append_pair("filters[Vakgebied][]", "Data-science")
        .append_pair("sort", "date")
        .append_pair("sortDir", "desc")
        .append_pair("pageNumber", &page.to_string());
    Ok(url)
}

fn official_base(value: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid base URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some("werk.ah.nl")
        || url.port().is_some()
        || url.path() != "/"
    {
        return Err(schema(
            company_id,
            "base URL is not official Albert Heijn HTTPS",
        ));
    }
    Ok(url)
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("Albert Heijn response for {company_id}: {message}"))
}

struct Card {
    id: String,
    title: String,
    department: Option<String>,
    employment_type: Option<String>,
    location: String,
    url: Url,
    raw: serde_json::Value,
}

#[derive(Deserialize)]
struct Response {
    vacancies: Vec<Vacancy>,
    meta: Meta,
}

#[derive(Clone, Deserialize)]
struct Meta {
    num_total_hits: usize,
    #[serde(rename = "pageNumber")]
    page_number: usize,
    #[serde(rename = "maxPerPage")]
    max_per_page: usize,
    #[serde(rename = "totalPageCount")]
    total_page_count: usize,
}

impl Meta {
    fn same_snapshot(&self, other: &Self) -> bool {
        self.num_total_hits == other.num_total_hits
            && self.max_per_page == other.max_per_page
            && self.total_page_count == other.total_page_count
    }
}

#[derive(Deserialize, Serialize)]
struct Vacancy {
    id: u64,
    created: String,
    slug: String,
    title: String,
    city: String,
    option_values: Vec<OptionValue>,
    company: Company,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct OptionValue {
    value: String,
    option: OptionDefinition,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct OptionDefinition {
    title: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Deserialize, Serialize)]
struct Company {
    name: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}
