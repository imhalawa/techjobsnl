use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::Deserialize;

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError,
    http::send_text,
    json_ld::{html_markdown, html_text, job_posting_value, parse_job_posting},
};

pub struct AnwbSource {
    company_id: String,
    feed_url: String,
    client: Client,
}

impl AnwbSource {
    pub fn new(company_id: impl Into<String>, feed_url: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            feed_url: feed_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for AnwbSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let feed = send_text(self.client.get(&self.feed_url), "ANWB").await?;
        let cards = parse_feed(&self.company_id, &self.feed_url, &feed)?;
        let requests = cards
            .iter()
            .map(|card| (self.client.clone(), card.url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "ANWB job").await })
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

pub fn parse_anwb_feed(
    company_id: &str,
    feed_url: &str,
    feed: &str,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let cards = parse_feed(company_id, feed_url, feed)?;
    parse_details(company_id, cards, details)
}

fn parse_feed(company_id: &str, feed_url: &str, feed: &str) -> Result<Vec<Card>, SourceError> {
    let base = official_feed_url(feed_url, company_id)?;
    let rows: Vec<FeedRow> = serde_json::from_str(feed)
        .map_err(|error| schema(company_id, format!("invalid feed JSON: {error}")))?;
    if rows.is_empty() {
        return Err(schema(company_id, "feed returned no vacancies"));
    }
    let mut ids = HashSet::new();
    rows.into_iter()
        .map(|row| {
            if row.id == 0 || !ids.insert(row.id) {
                return Err(schema(company_id, "feed has an invalid or duplicate ID"));
            }
            let title = row.title.split_whitespace().collect::<Vec<_>>().join(" ");
            if title.is_empty() || row.description_plain.trim().is_empty() {
                return Err(schema(
                    company_id,
                    format!("vacancy {} is incomplete", row.id),
                ));
            }
            let url = base
                .join(&row.url)
                .map_err(|error| schema(company_id, format!("invalid vacancy URL: {error}")))?;
            let expected_prefix = format!("/vacature/{}/", row.id);
            if url.scheme() != "https"
                || url.host_str() != base.host_str()
                || !url.path().starts_with(&expected_prefix)
                || url.query().is_some()
            {
                return Err(schema(
                    company_id,
                    format!("vacancy {} has an untrusted URL", row.id),
                ));
            }
            Ok(Card {
                id: row.id,
                title,
                url,
            })
        })
        .collect()
}

fn parse_details(
    company_id: &str,
    cards: Vec<Card>,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    if cards.len() != details.len() {
        return Err(schema(company_id, "feed/detail count mismatch"));
    }
    cards
        .into_iter()
        .zip(details)
        .map(|(card, detail)| observed_job(company_id, card, detail))
        .collect::<Result<Vec<_>, _>>()
        .map(|jobs| jobs.into_iter().flatten().collect())
}

fn observed_job(
    company_id: &str,
    card: Card,
    detail: &str,
) -> Result<Option<ObservedJob>, SourceError> {
    let posting = parse_job_posting(detail, "ANWB")
        .map_err(|error| schema(company_id, format!("detail {}: {error}", card.id)))?;
    let raw_payload = job_posting_value(detail, "ANWB")?;
    let title = posting
        .title
        .as_deref()
        .map(html_text)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no title", card.id)))?;
    if title != card.title {
        return Err(schema(
            company_id,
            format!("detail {} title mismatch", card.id),
        ));
    }
    let employer = posting
        .hiring_organization
        .as_ref()
        .and_then(|organization| organization.name.as_deref())
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no employer", card.id)))?;
    let published_at = posting
        .date_posted
        .as_deref()
        .ok_or_else(|| schema(company_id, format!("detail {} has no datePosted", card.id)))
        .and_then(|date| {
            DateTime::parse_from_rfc3339(date)
                .map(|date| date.with_timezone(&Utc))
                .map_err(|error| schema(company_id, format!("invalid datePosted: {error}")))
        })?;
    let mut locations = Vec::new();
    for place in posting.job_location {
        let country = place
            .address
            .address_country
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        if country != "Nederland" {
            return Ok(None);
        }
        let location = place
            .address
            .address_locality
            .map(|location| location.trim().to_owned())
            .filter(|location| !location.is_empty())
            .ok_or_else(|| schema(company_id, format!("detail {} has no location", card.id)))?;
        if !locations.contains(&location) {
            locations.push(location);
        }
    }
    let description = html_markdown(&posting.description);
    if locations.is_empty() || description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} is incomplete", card.id),
        ));
    }
    let document = Html::parse_document(detail);
    let application_selector =
        Selector::parse("#vacancy-application-form").expect("static selector must parse");
    if document.select(&application_selector).next().is_none() {
        return Err(schema(
            company_id,
            format!("detail {} has no application form", card.id),
        ));
    }
    let job_url = card.url.to_string();

    Ok(Some(ObservedJob {
        source_id: card.id.to_string(),
        title: card.title,
        department: Some(employer.to_owned()),
        team: None,
        employment_type: posting.employment_type,
        locations,
        countries: vec!["NL".to_owned()],
        apply_url: format!("{job_url}#vacancy-application-form"),
        job_url,
        description,
        raw_payload,
        published_at: Some(published_at),
    }))
}

fn official_feed_url(raw: &str, company_id: &str) -> Result<Url, SourceError> {
    let url =
        Url::parse(raw).map_err(|error| schema(company_id, format!("invalid URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.werkenbijanwb.nl")
        || url.path() != "/fuse/vacancies.json"
        || url.query().is_some()
    {
        return Err(schema(company_id, "unexpected ANWB feed URL"));
    }
    Ok(url)
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("ANWB response for {company_id}: {message}"))
}

#[derive(Deserialize)]
struct FeedRow {
    id: u64,
    title: String,
    url: String,
    description_plain: String,
}

struct Card {
    id: u64,
    title: String,
    url: Url,
}
