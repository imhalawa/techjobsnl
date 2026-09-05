use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError, country_code_for_location,
    http::send_text,
    json_ld::{html_markdown, html_text, job_posting_value, parse_job_posting},
};

pub struct DeelSource {
    company_id: String,
    board_url: String,
    client: Client,
}

impl DeelSource {
    pub fn new(
        company_id: impl Into<String>,
        board_url: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            company_id: company_id.into(),
            board_url: board_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for DeelSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let listing = send_text(self.client.get(&self.board_url), "Deel jobs").await?;
        let urls = parse_listing(&self.company_id, &self.board_url, &listing)?;
        let requests = urls
            .iter()
            .map(|url| (self.client.clone(), url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "Deel job").await })
            .buffered(12)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();

        Ok(SourceScan::Complete {
            observations: parse_details(&self.company_id, urls, &detail_refs)?,
        })
    }
}

pub fn parse_deel_board(
    company_id: &str,
    board_url: &str,
    listing: &str,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let urls = parse_listing(company_id, board_url, listing)?;
    parse_details(company_id, urls, details)
}

fn parse_listing(
    company_id: &str,
    board_url: &str,
    listing: &str,
) -> Result<Vec<Url>, SourceError> {
    let board = official_board_url(board_url, company_id)?;
    let document = Html::parse_document(listing);
    let selector = Selector::parse(r#"script[type="application/ld+json"]"#)
        .expect("static selector must parse");
    let mut item_list = None;
    for script in document.select(&selector) {
        let raw = script.text().collect::<String>();
        let value: serde_json::Value = serde_json::from_str(&raw)
            .map_err(|error| schema(company_id, format!("invalid JSON-LD: {error}")))?;
        if value.get("@type").and_then(serde_json::Value::as_str) == Some("ItemList") {
            item_list = Some(serde_json::from_value::<ItemList>(value).map_err(|error| {
                schema(company_id, format!("invalid ItemList JSON-LD: {error}"))
            })?);
            break;
        }
    }
    let items = item_list
        .ok_or_else(|| schema(company_id, "listing has no ItemList JSON-LD"))?
        .item_list_element;
    if items.is_empty() {
        return Err(schema(company_id, "listing returned no vacancies"));
    }

    let expected_prefix = format!("{}/job-details/", board.path().trim_end_matches('/'));
    let mut ids = HashSet::new();
    items
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            if item.position != index + 1 || item.kind != "ListItem" {
                return Err(schema(company_id, "listing positions are incomplete"));
            }
            let url = Url::parse(&item.url)
                .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
            let id = vacancy_id(&url, &board, &expected_prefix, company_id)?;
            if !ids.insert(id.to_owned()) {
                return Err(schema(company_id, "listing has a duplicate vacancy ID"));
            }
            Ok(url)
        })
        .collect()
}

fn parse_details(
    company_id: &str,
    urls: Vec<Url>,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    if urls.len() != details.len() {
        return Err(schema(company_id, "listing/detail count mismatch"));
    }
    urls.into_iter()
        .zip(details)
        .map(|(url, detail)| observed_job(company_id, url, detail))
        .collect::<Result<Vec<_>, _>>()
        .map(|jobs| jobs.into_iter().flatten().collect())
}

fn observed_job(
    company_id: &str,
    job_url: Url,
    detail: &str,
) -> Result<Option<ObservedJob>, SourceError> {
    let id = job_url
        .path_segments()
        .and_then(|mut segments| segments.nth_back(1))
        .ok_or_else(|| schema(company_id, "vacancy URL has no ID"))?;
    let posting = parse_job_posting(detail, "Deel")
        .map_err(|error| schema(company_id, format!("detail {id}: {error}")))?;
    let raw_payload = job_posting_value(detail, "Deel")?;
    let identifier = posting
        .identifier
        .as_ref()
        .ok_or_else(|| schema(company_id, format!("detail {id} has no identifier")))?;
    if identifier.value != id
        || identifier.name.as_deref() != Some("Deel ATS Job Posting ID")
        || posting.url.as_deref() != Some(job_url.as_str())
        || posting
            .extra
            .get("directApply")
            .and_then(|value| value.as_bool())
            != Some(true)
    {
        return Err(schema(company_id, format!("detail {id} identity mismatch")));
    }

    let title = posting
        .title
        .as_deref()
        .map(html_text)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {id} has no title")))?;
    let employer = posting
        .hiring_organization
        .as_ref()
        .and_then(|value| value.name.as_deref())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {id} has no employer")))?;
    let published_at = posting
        .date_posted
        .as_deref()
        .ok_or_else(|| schema(company_id, format!("detail {id} has no datePosted")))
        .and_then(|value| {
            DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&Utc))
                .map_err(|error| schema(company_id, format!("detail {id} date: {error}")))
        })?;
    if posting.job_location.is_empty() {
        return Err(schema(company_id, format!("detail {id} has no location")));
    }
    let mut locations = Vec::new();
    for place in posting.job_location {
        let location = place
            .address
            .address_locality
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| schema(company_id, format!("detail {id} has no locality")))?;
        if country_code_for_location(location) == Some("NL")
            && !locations.iter().any(|value| value == location)
        {
            locations.push(location.to_owned());
        }
    }
    if locations.is_empty() {
        return Ok(None);
    }
    let description = html_markdown(&posting.description);
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {id} has no description"),
        ));
    }

    Ok(Some(ObservedJob {
        source_id: id.to_owned(),
        title,
        department: Some(employer.to_owned()),
        team: None,
        employment_type: posting.employment_type,
        locations,
        countries: vec!["NL".to_owned()],
        job_url: job_url.to_string(),
        apply_url: job_url.to_string(),
        description,
        raw_payload,
        published_at: Some(published_at),
    }))
}

fn official_board_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(raw)
        .map_err(|error| schema(company_id, format!("invalid board URL: {error}")))?;
    let segments = url
        .path_segments()
        .map(Iterator::collect::<Vec<_>>)
        .unwrap_or_default();
    if url.scheme() != "https"
        || url.host_str() != Some("jobs.deel.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || segments.len() != 1
        || segments[0].is_empty()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(schema(company_id, "unexpected Deel board URL"));
    }
    Ok(url)
}

fn vacancy_id<'a>(
    url: &'a Url,
    board: &Url,
    expected_prefix: &str,
    company_id: &str,
) -> Result<&'a str, SourceError> {
    let suffix = url
        .path()
        .strip_prefix(expected_prefix)
        .and_then(|value| value.strip_suffix("/overview"));
    if url.scheme() != "https"
        || url.host_str() != board.host_str()
        || url.query().is_some()
        || url.fragment().is_some()
        || suffix.is_none_or(|value| value.is_empty() || value.contains('/'))
    {
        return Err(schema(company_id, "unexpected Deel vacancy URL"));
    }
    Ok(suffix.expect("checked above"))
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("Deel response for {company_id}: {message}"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ItemList {
    item_list_element: Vec<ListItem>,
}

#[derive(Deserialize)]
struct ListItem {
    #[serde(rename = "@type")]
    kind: String,
    position: usize,
    url: String,
}
