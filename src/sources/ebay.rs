use std::collections::HashSet;

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::Value;

use crate::domain::{ObservedJob, SourceScan};

use super::{
    JobSource, SourceError, country_code_for_location,
    http::send_text,
    json_ld::{html_text, job_posting_value, parse_job_posting},
};

const PAGE_SIZE: usize = 10;
const DDO_START: &str = "phApp.ddo = ";
const DDO_END: &str = "; phApp.experimentData";

pub struct EbaySource {
    company_id: String,
    listing_url: String,
    client: Client,
}

impl EbaySource {
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
impl JobSource for EbaySource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let mut collection = EbayCollection::new(&self.company_id, &self.listing_url);
        let first = send_text(self.client.get(&self.listing_url), "eBay").await?;
        let mut complete = collection.add_page(&first)?;
        let mut offset = PAGE_SIZE;
        while !complete {
            let page = send_text(
                self.client
                    .get(page_url(&self.listing_url, offset, &self.company_id)?),
                "eBay",
            )
            .await?;
            complete = collection.add_page(&page)?;
            offset += PAGE_SIZE;
        }
        let listings = collection.finish()?;

        let requests = listings
            .iter()
            .map(|listing| (self.client.clone(), listing.detail_url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "eBay").await })
            .buffered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let details = details.iter().map(String::as_str).collect::<Vec<_>>();
        Ok(SourceScan::Complete {
            observations: parse_details(&self.company_id, listings, &details)?,
        })
    }
}

pub fn parse_ebay_pages(
    company_id: &str,
    listing_url: &str,
    pages: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let mut collection = EbayCollection::new(company_id, listing_url);
    for page in pages {
        collection.add_page(page)?;
    }
    parse_details(company_id, collection.finish()?, details)
}

struct EbayCollection<'a> {
    company_id: &'a str,
    listing_url: &'a str,
    expected_total: Option<usize>,
    ids: HashSet<String>,
    listings: Vec<EbayListing>,
}

impl<'a> EbayCollection<'a> {
    fn new(company_id: &'a str, listing_url: &'a str) -> Self {
        Self {
            company_id,
            listing_url,
            expected_total: None,
            ids: HashSet::new(),
            listings: Vec::new(),
        }
    }

    fn add_page(&mut self, html: &str) -> Result<bool, SourceError> {
        let page = listing_page(html, self.company_id)?;
        if page.status != 200 {
            return Err(schema(self.company_id, "listing status is not 200"));
        }
        let expected_total = *self.expected_total.get_or_insert(page.total_hits);
        if page.total_hits != expected_total {
            return Err(schema(
                self.company_id,
                "listing totalHits changed between pages",
            ));
        }
        if self.listings.len() == expected_total {
            return Err(schema(
                self.company_id,
                "listing returned a page after reaching totalHits",
            ));
        }
        let jobs = page
            .data
            .jobs
            .ok_or_else(|| schema(self.company_id, "listing data has no jobs"))?;
        if jobs.is_empty() && self.listings.len() < expected_total {
            return Err(schema(
                self.company_id,
                "listing returned an empty page before totalHits",
            ));
        }

        for raw_payload in jobs {
            let row: EbayRow = serde_json::from_value(raw_payload.clone()).map_err(|error| {
                schema(self.company_id, format!("invalid listing job: {error}"))
            })?;
            let listing = EbayListing::new(self.company_id, self.listing_url, row, raw_payload)?;
            if !self.ids.insert(listing.source_id.clone()) {
                return Err(schema(
                    self.company_id,
                    format!("duplicate listing job {}", listing.source_id),
                ));
            }
            self.listings.push(listing);
            if self.listings.len() > expected_total {
                return Err(schema(
                    self.company_id,
                    format!("listing returned more than totalHits {expected_total}"),
                ));
            }
        }
        Ok(self.listings.len() == expected_total)
    }

    fn finish(self) -> Result<Vec<EbayListing>, SourceError> {
        let expected_total = self
            .expected_total
            .ok_or_else(|| schema(self.company_id, "listing returned no pages"))?;
        if self.listings.len() != expected_total {
            return Err(schema(
                self.company_id,
                format!(
                    "listing returned {} of {expected_total} jobs",
                    self.listings.len()
                ),
            ));
        }
        Ok(self.listings)
    }
}

impl EbayListing {
    fn new(
        company_id: &str,
        listing_url: &str,
        row: EbayRow,
        raw_payload: Value,
    ) -> Result<Self, SourceError> {
        let source_id = required(&row.req_id, "reqId", company_id)?;
        let job_id = required(&row.job_id, "jobId", company_id)?;
        if source_id != job_id {
            return Err(schema(company_id, "listing reqId does not match jobId"));
        }
        let title = required(&row.title, "title", company_id)?;
        let category = required(&row.category, "category", company_id)?;
        let employment_type = required(&row.employment_type, "type", company_id)?;
        let country = required(&row.country, "country", company_id)?;
        if country_code_for_location(&country) != Some("NL") {
            return Err(schema(
                company_id,
                format!("listing job {source_id} is not in the Netherlands"),
            ));
        }
        if row
            .multi_location
            .iter()
            .map(|location| location.trim())
            .all(str::is_empty)
        {
            return Err(schema(
                company_id,
                format!("listing job {source_id} has no locations"),
            ));
        }
        let apply_url = official_url(&row.apply_url, "apply", company_id)?;
        let published_at = DateTime::parse_from_str(&row.posted_date, "%Y-%m-%dT%H:%M:%S%.f%z")
            .map_err(|error| {
                schema(
                    company_id,
                    format!("listing job {source_id} has invalid postedDate: {error}"),
                )
            })?
            .with_timezone(&Utc);
        let detail_url = detail_url(listing_url, &source_id, &title, company_id)?;
        Ok(Self {
            source_id,
            category,
            employment_type,
            apply_url,
            published_at,
            detail_url,
            raw_payload,
        })
    }
}

fn parse_details(
    company_id: &str,
    listings: Vec<EbayListing>,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    if listings.len() != details.len() {
        return Err(schema(
            company_id,
            format!(
                "received {} details for {} listing jobs",
                details.len(),
                listings.len()
            ),
        ));
    }
    listings
        .into_iter()
        .zip(details)
        .map(|(listing, detail)| observed_job(company_id, listing, detail))
        .collect()
}

fn observed_job(
    company_id: &str,
    mut listing: EbayListing,
    detail: &str,
) -> Result<ObservedJob, SourceError> {
    let posting = parse_job_posting(detail, "eBay")?;
    let raw_posting = job_posting_value(detail, "eBay")?;
    let detail_id = posting
        .identifier
        .as_ref()
        .map(|identifier| identifier.value.trim())
        .filter(|identifier| !identifier.is_empty())
        .ok_or_else(|| schema(company_id, "detail has no identifier"))?;
    if detail_id != listing.source_id {
        return Err(schema(
            company_id,
            format!(
                "listing job {} does not match detail identifier {detail_id}",
                listing.source_id
            ),
        ));
    }
    let title = posting
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {detail_id} has no title")))?
        .to_owned();
    let description = html_text(&html_text(&posting.description));
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {detail_id} has an empty description"),
        ));
    }
    if posting.job_location.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {detail_id} has no locations"),
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
                    format!("detail {detail_id} has unnamed location"),
                )
            })?;
        let country = place
            .address
            .address_country
            .as_deref()
            .map(str::trim)
            .filter(|country| !country.is_empty())
            .and_then(country_code_for_location)
            .ok_or_else(|| {
                schema(
                    company_id,
                    format!("detail {detail_id} has unresolved country"),
                )
            })?;
        push_unique(&mut locations, location.to_owned());
        push_unique(&mut countries, country.to_owned());
    }

    listing
        .raw_payload
        .as_object_mut()
        .expect("validated eBay listing payload is an object")
        .insert("jobPosting".into(), raw_posting);
    Ok(ObservedJob {
        source_id: listing.source_id,
        title,
        department: Some(listing.category),
        team: None,
        employment_type: posting
            .employment_type
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or(Some(listing.employment_type)),
        locations,
        countries,
        job_url: listing.detail_url.to_string(),
        apply_url: listing.apply_url.to_string(),
        description,
        raw_payload: listing.raw_payload,
        published_at: Some(listing.published_at),
    })
}

fn listing_page(html: &str, company_id: &str) -> Result<EbayPage, SourceError> {
    let start = html
        .find(DDO_START)
        .map(|index| index + DDO_START.len())
        .ok_or_else(|| schema(company_id, "listing has no phApp.ddo assignment"))?;
    let end = html[start..]
        .find(DDO_END)
        .map(|index| start + index)
        .ok_or_else(|| schema(company_id, "listing has no phApp.ddo terminator"))?;
    let ddo: EbayDdo = serde_json::from_str(&html[start..end]).map_err(|error| {
        schema(
            company_id,
            format!("invalid listing phApp.ddo JSON: {error}"),
        )
    })?;
    Ok(ddo.refine)
}

fn page_url(listing_url: &str, offset: usize, company_id: &str) -> Result<Url, SourceError> {
    let mut url = official_url(listing_url, "listing", company_id)?;
    url.set_query(None);
    url.query_pairs_mut()
        .append_pair("from", &offset.to_string())
        .append_pair("s", "1");
    Ok(url)
}

fn detail_url(
    listing_url: &str,
    source_id: &str,
    title: &str,
    company_id: &str,
) -> Result<Url, SourceError> {
    let mut url = official_url(listing_url, "listing", company_id)?;
    url.set_path(&format!("/us/en/job/{source_id}/{}", slug(title)));
    url.set_query(None);
    Ok(url)
}

fn official_url(value: &str, kind: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid {kind} URL: {error}")))?;
    if url.scheme() != "https" || url.host_str().is_none() {
        return Err(schema(company_id, format!("{kind} URL is not HTTPS")));
    }
    Ok(url)
}

fn required(value: &str, field: &str, company_id: &str) -> Result<String, SourceError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(schema(company_id, format!("listing job has empty {field}")));
    }
    Ok(value.to_owned())
}

fn slug(title: &str) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for character in title.chars() {
        if character.is_ascii_alphanumeric() {
            if separator && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(character);
            separator = false;
        } else {
            separator = true;
        }
    }
    slug
}

fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.contains(&value) {
        values.push(value);
    }
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("eBay response for {company_id}: {message}"))
}

#[derive(Deserialize)]
struct EbayDdo {
    #[serde(rename = "eagerLoadRefineSearch")]
    refine: EbayPage,
}

#[derive(Deserialize)]
struct EbayPage {
    status: u16,
    #[serde(rename = "totalHits")]
    total_hits: usize,
    data: EbayData,
}

#[derive(Deserialize)]
struct EbayData {
    jobs: Option<Vec<Value>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EbayRow {
    req_id: String,
    job_id: String,
    title: String,
    #[serde(rename = "type")]
    employment_type: String,
    #[serde(rename = "multi_location")]
    multi_location: Vec<String>,
    country: String,
    category: String,
    apply_url: String,
    posted_date: String,
}

struct EbayListing {
    source_id: String,
    category: String,
    employment_type: String,
    apply_url: Url,
    published_at: DateTime<Utc>,
    detail_url: Url,
    raw_payload: Value,
}
