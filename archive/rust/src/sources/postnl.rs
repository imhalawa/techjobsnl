use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

pub struct PostnlSource {
    company_id: String,
    api_url: String,
    client: Client,
}

impl PostnlSource {
    pub fn new(company_id: impl Into<String>, api_url: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            api_url: api_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for PostnlSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let base = official_api_url(&self.api_url, &self.company_id)?;
        let first = send_text(self.client.get(overview_url(&base, 1)), "PostNL overview").await?;
        let first_page: OverviewPage = serde_json::from_str(&first)
            .map_err(|error| schema(&self.company_id, format!("invalid overview JSON: {error}")))?;
        if first_page.paging.current_page != 1 || first_page.paging.pages == 0 {
            return Err(schema(&self.company_id, "invalid first-page metadata"));
        }

        let mut pages = vec![first];
        let requests = (2..=first_page.paging.pages)
            .map(|page| (self.client.clone(), overview_url(&base, page)))
            .collect::<Vec<_>>();
        let remaining =
            stream::iter(requests)
                .map(|(client, url)| async move {
                    send_text(client.get(url), "PostNL overview page").await
                })
                .buffered(6)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
        pages.extend(remaining);

        let page_refs = pages.iter().map(String::as_str).collect::<Vec<_>>();
        let cards = parse_pages(&self.company_id, &page_refs)?;
        let requests = cards
            .iter()
            .map(|card| {
                (
                    self.client.clone(),
                    base.join(&format!("vacancies/id/{}", card.id))
                        .expect("validated base URL must join"),
                )
            })
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "PostNL detail").await })
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

pub fn parse_postnl_responses(
    company_id: &str,
    api_url: &str,
    pages: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    official_api_url(api_url, company_id)?;
    let cards = parse_pages(company_id, pages)?;
    parse_details(company_id, cards, details)
}

fn parse_pages(company_id: &str, pages: &[&str]) -> Result<Vec<Card>, SourceError> {
    if pages.is_empty() {
        return Err(schema(company_id, "overview returned no pages"));
    }
    let parsed = pages
        .iter()
        .map(|page| {
            serde_json::from_str::<OverviewPage>(page)
                .map_err(|error| schema(company_id, format!("invalid overview JSON: {error}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let expected = parsed[0].paging;
    if expected.total_result == 0
        || expected.pages == 0
        || expected.page_size == 0
        || parsed.len() != expected.pages
    {
        return Err(schema(company_id, "incomplete overview pagination"));
    }

    let mut ids = HashSet::new();
    let mut cards = Vec::new();
    for (index, page) in parsed.into_iter().enumerate() {
        if page.paging.total_result != expected.total_result
            || page.paging.pages != expected.pages
            || page.paging.page_size != expected.page_size
            || page.paging.current_page != index + 1
        {
            return Err(schema(company_id, "inconsistent overview pagination"));
        }
        for row in page.overview {
            let title = normalized(&row.job_title);
            let city = normalized(&row.city);
            if row.id == 0
                || row.reference_id == 0
                || !ids.insert(row.id)
                || !row.is_professional
                || title.is_empty()
                || city.is_empty()
                || normalized(&row.description).is_empty()
                || !valid_slug(&row.fancy_url)
            {
                return Err(schema(
                    company_id,
                    format!("overview vacancy {} is invalid", row.id),
                ));
            }
            cards.push(Card {
                id: row.id,
                reference_id: row.reference_id,
                fancy_url: row.fancy_url,
                title,
                city,
            });
        }
    }
    if cards.len() != expected.total_result {
        return Err(schema(
            company_id,
            "overview total does not match pagination",
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
        return Err(schema(company_id, "overview/detail count mismatch"));
    }
    cards
        .into_iter()
        .zip(details)
        .map(|(card, raw)| observed_job(company_id, card, raw))
        .collect()
}

fn observed_job(company_id: &str, card: Card, raw: &str) -> Result<ObservedJob, SourceError> {
    let detail: Detail = serde_json::from_str(raw)
        .map_err(|error| schema(company_id, format!("invalid detail {}: {error}", card.id)))?;
    if detail.id != card.id
        || detail.reference_id != card.reference_id
        || detail.fancy_url != card.fancy_url
        || !detail.is_professional
        || normalized(&detail.job_title) != card.title
        || normalized(&detail.work_location.city) != card.city
    {
        return Err(schema(company_id, format!("detail {} mismatch", card.id)));
    }
    let published_at = DateTime::parse_from_rfc3339(&detail.publication_period.start_date)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|error| schema(company_id, format!("detail {} date: {error}", card.id)))?;
    let mut sections = detail
        .content_fields
        .values()
        .filter_map(serde_json::Value::as_str)
        .map(html_markdown)
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>();
    if sections.is_empty() {
        sections.push(normalized(&detail.description));
    }
    let description = sections.join("\n\n");
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} has no content", card.id),
        ));
    }
    let job_url = format!(
        "https://www.postnl.nl/werkenbij/vacatures-voor-hbo-en-wo/{}/",
        card.fancy_url
    );

    Ok(ObservedJob {
        source_id: card.id.to_string(),
        title: card.title,
        department: nonempty(detail.discipline),
        team: (!detail.levels.is_empty()).then(|| detail.levels.join(", ")),
        employment_type: nonempty(detail.contract_type),
        locations: vec![card.city],
        countries: vec!["NL".to_owned()],
        apply_url: format!(
            "https://www.postnl.nl/werkenbij/sollicitatie/?id={}",
            card.id
        ),
        job_url,
        description,
        raw_payload: serde_json::from_str(raw)
            .map_err(|error| schema(company_id, format!("invalid detail JSON: {error}")))?,
        published_at: Some(published_at),
    })
}

fn overview_url(base: &Url, page: usize) -> Url {
    let mut url = base
        .join("vacanciesoverview")
        .expect("validated base URL must join");
    url.query_pairs_mut()
        .append_pair("isProfessional", "true")
        .append_pair("distance", "-1")
        .append_pair("page", &page.to_string());
    url
}

fn official_api_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url =
        Url::parse(raw).map_err(|error| schema(company_id, format!("invalid API URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some("vacatures-website.postnl.nl")
        || url.path() != "/vacatures-widget/api/"
        || url.query().is_some()
    {
        return Err(schema(company_id, "unexpected PostNL API URL"));
    }
    Ok(url)
}

fn valid_slug(slug: &str) -> bool {
    !slug.is_empty()
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn normalized(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn nonempty(value: Option<String>) -> Option<String> {
    value
        .map(|value| normalized(&value))
        .filter(|value| !value.is_empty())
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("PostNL response for {company_id}: {message}"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverviewPage {
    overview: Vec<OverviewRow>,
    paging: Paging,
}

#[derive(Clone, Copy, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Paging {
    total_result: usize,
    current_page: usize,
    pages: usize,
    page_size: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverviewRow {
    id: u64,
    reference_id: u64,
    fancy_url: String,
    is_professional: bool,
    description: String,
    job_title: String,
    city: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Detail {
    publication_period: PublicationPeriod,
    work_location: WorkLocation,
    contract_type: Option<String>,
    discipline: Option<String>,
    #[serde(default)]
    content_fields: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    levels: Vec<String>,
    id: u64,
    reference_id: u64,
    fancy_url: String,
    is_professional: bool,
    description: String,
    job_title: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicationPeriod {
    start_date: String,
}

#[derive(Deserialize)]
struct WorkLocation {
    city: String,
}

struct Card {
    id: u64,
    reference_id: u64,
    fancy_url: String,
    title: String,
    city: String,
}
