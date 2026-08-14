use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

const SITE: &str = "UberCareers";
const PAGE_SIZE: usize = 100;

pub struct UberSource {
    company_id: String,
    api_url: String,
    client: Client,
}

impl UberSource {
    pub fn new(company_id: impl Into<String>, api_url: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            api_url: api_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for UberSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let base = official_api_url(&self.api_url, &self.company_id)?;
        let first = send_text(
            self.client.get(listing_url(&base, 0)),
            "Uber Oracle HCM listing",
        )
        .await?;
        let first_page = parse_page(&self.company_id, &first)?;
        if first_page.total_jobs_count == 0 || first_page.limit != PAGE_SIZE {
            return Err(schema(&self.company_id, "invalid first-page metadata"));
        }

        let mut pages = vec![first];
        let requests = (PAGE_SIZE..first_page.total_jobs_count)
            .step_by(PAGE_SIZE)
            .map(|offset| (self.client.clone(), listing_url(&base, offset)))
            .collect::<Vec<_>>();
        let remaining = stream::iter(requests)
            .map(|(client, url)| async move {
                send_text(client.get(url), "Uber Oracle HCM listing page").await
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
            .map(|card| (self.client.clone(), detail_url(&base, &card.id)))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move {
                send_text(client.get(url), "Uber Oracle HCM detail").await
            })
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

pub fn parse_uber_responses(
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
        return Err(schema(company_id, "listing returned no pages"));
    }
    let parsed = pages
        .iter()
        .map(|page| parse_page(company_id, page))
        .collect::<Result<Vec<_>, _>>()?;
    let total = parsed[0].total_jobs_count;
    let limit = parsed[0].limit;
    if total == 0 || limit == 0 || parsed.len() != total.div_ceil(limit) {
        return Err(schema(company_id, "incomplete pagination"));
    }

    let mut ids = HashSet::new();
    let mut cards = Vec::new();
    for (index, page) in parsed.into_iter().enumerate() {
        let expected_offset = index * limit;
        let expected_count = (total - expected_offset).min(limit);
        if page.total_jobs_count != total
            || page.limit != limit
            || page.offset != expected_offset
            || page.requisition_list.len() != expected_count
        {
            return Err(schema(company_id, "incomplete pagination"));
        }
        for row in page.requisition_list {
            if row.id.is_empty()
                || !row.id.bytes().all(|byte| byte.is_ascii_digit())
                || !ids.insert(row.id.clone())
                || normalized(&row.title).is_empty()
                || normalized(&row.primary_location).is_empty()
                || row.primary_location_country.len() != 2
                || row.posted_date.len() != 10
            {
                return Err(schema(company_id, "invalid or duplicate listing row"));
            }
            cards.push(row);
        }
    }
    if cards.len() != total {
        return Err(schema(company_id, "listing total mismatch"));
    }
    Ok(cards)
}

fn parse_page(company_id: &str, raw: &str) -> Result<ListingPage, SourceError> {
    let root: ListingRoot = serde_json::from_str(raw)
        .map_err(|error| schema(company_id, format!("invalid listing JSON: {error}")))?;
    if root.items.len() != 1 {
        return Err(schema(
            company_id,
            "listing must contain exactly one result",
        ));
    }
    Ok(root.items.into_iter().next().expect("length checked"))
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
        .map(|(card, raw)| observed_job(company_id, card, raw))
        .collect()
}

fn observed_job(company_id: &str, card: Card, raw: &str) -> Result<ObservedJob, SourceError> {
    let root: DetailRoot = serde_json::from_str(raw)
        .map_err(|error| schema(company_id, format!("invalid detail {}: {error}", card.id)))?;
    if root.items.len() != 1 {
        return Err(schema(
            company_id,
            format!("detail {} must contain exactly one result", card.id),
        ));
    }
    let detail = root.items.into_iter().next().expect("length checked");
    if detail.id != card.id
        || normalized(&detail.title) != normalized(&card.title)
        || normalized(&detail.primary_location) != normalized(&card.primary_location)
        || detail.primary_location_country != card.primary_location_country
        || !detail
            .external_posted_start_date
            .starts_with(&card.posted_date)
    {
        return Err(schema(company_id, format!("detail {} mismatch", card.id)));
    }

    let mut locations = vec![normalized(&detail.primary_location)];
    let mut countries = vec![detail.primary_location_country.clone()];
    for location in detail.secondary_locations {
        let name = normalized(&location.name);
        if name.is_empty() || location.country_code.len() != 2 {
            return Err(schema(
                company_id,
                format!("detail {} has an invalid location", card.id),
            ));
        }
        if !locations.contains(&name) {
            locations.push(name);
        }
        if !countries.contains(&location.country_code) {
            countries.push(location.country_code);
        }
    }
    if !countries.iter().any(|country| country == "NL") {
        return Err(schema(
            company_id,
            format!("detail {} has no Netherlands location", card.id),
        ));
    }

    let description = [
        html_markdown(&detail.external_description_str),
        html_markdown(&detail.corporate_description_str),
    ]
    .into_iter()
    .filter(|section| !section.is_empty())
    .collect::<Vec<_>>()
    .join("\n\n");
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} has no description", card.id),
        ));
    }
    let published_at = DateTime::parse_from_rfc3339(&detail.external_posted_start_date)
        .map(|date| date.with_timezone(&Utc))
        .map_err(|error| schema(company_id, format!("detail {} date: {error}", card.id)))?;
    let job_url = format!("https://jobs.uber.com/en/jobs/{}/", card.id);

    Ok(ObservedJob {
        source_id: card.id.clone(),
        title: normalized(&card.title),
        department: nonempty(detail.category),
        team: None,
        employment_type: nonempty(detail.job_schedule),
        locations,
        countries,
        apply_url: format!(
            "https://iaziqy.fa.ocs.oraclecloud.com/hcmUI/CandidateExperience/en/sites/{SITE}/job/{}",
            card.id
        ),
        job_url,
        description,
        raw_payload: serde_json::from_str(raw)
            .map_err(|error| schema(company_id, format!("invalid detail JSON: {error}")))?,
        published_at: Some(published_at),
    })
}

fn listing_url(base: &Url, offset: usize) -> Url {
    let mut url = base
        .join("recruitingCEJobRequisitions")
        .expect("validated API base must join");
    url.query_pairs_mut()
        .append_pair(
            "finder",
            &format!(
                "findReqs;siteNumber={SITE},limit={PAGE_SIZE},offset={offset},location=Netherlands"
            ),
        )
        .append_pair("onlyData", "true")
        .append_pair("expand", "requisitionList");
    url
}

fn detail_url(base: &Url, id: &str) -> Url {
    let mut url = base
        .join("recruitingCEJobRequisitionDetails")
        .expect("validated API base must join");
    url.query_pairs_mut()
        .append_pair("finder", &format!("ById;Id={id},siteNumber={SITE}"))
        .append_pair("onlyData", "true")
        .append_pair("expand", "all");
    url
}

fn official_api_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url =
        Url::parse(raw).map_err(|error| schema(company_id, format!("invalid API URL: {error}")))?;
    if url.as_str() != "https://iaziqy.fa.ocs.oraclecloud.com/hcmRestApi/resources/latest/" {
        return Err(schema(company_id, "unexpected Uber Oracle HCM API URL"));
    }
    Ok(url)
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
    SourceError::schema(format!("Uber response for {company_id}: {message}"))
}

#[derive(Deserialize)]
struct ListingRoot {
    items: Vec<ListingPage>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct ListingPage {
    total_jobs_count: usize,
    limit: usize,
    offset: usize,
    #[serde(default, rename = "requisitionList")]
    requisition_list: Vec<Card>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Card {
    id: String,
    title: String,
    posted_date: String,
    primary_location_country: String,
    primary_location: String,
}

#[derive(Deserialize)]
struct DetailRoot {
    items: Vec<Detail>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Detail {
    id: String,
    title: String,
    category: Option<String>,
    external_posted_start_date: String,
    job_schedule: Option<String>,
    external_description_str: String,
    #[serde(default)]
    corporate_description_str: String,
    primary_location: String,
    primary_location_country: String,
    #[serde(default, rename = "secondaryLocations")]
    secondary_locations: Vec<SecondaryLocation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct SecondaryLocation {
    name: String,
    country_code: String,
}
