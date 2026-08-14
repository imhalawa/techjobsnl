use std::collections::HashSet;

use chrono::DateTime;
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

const PAGE_SIZE: usize = 10;

pub struct MicrosoftSource {
    company_id: String,
    search_url: String,
    client: Client,
}

impl MicrosoftSource {
    pub fn new(
        company_id: impl Into<String>,
        search_url: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            company_id: company_id.into(),
            search_url: search_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for MicrosoftSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let base = official_search_url(&self.search_url, &self.company_id)?;
        let first = send_text(self.client.get(page_url(&base, 0)), "Microsoft careers").await?;
        let first_page = parse_response::<SearchData>(&first, &self.company_id, "search page")?;
        if first_page.count == 0 || first_page.positions.len() != first_page.count.min(PAGE_SIZE) {
            return Err(schema(&self.company_id, "invalid first-page result count"));
        }

        let requests = (PAGE_SIZE..first_page.count)
            .step_by(PAGE_SIZE)
            .map(|start| (self.client.clone(), page_url(&base, start)))
            .collect::<Vec<_>>();
        let remaining = stream::iter(requests)
            .map(|(client, url)| async move {
                send_text(client.get(url), "Microsoft careers page").await
            })
            .buffered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let mut pages = vec![first];
        pages.extend(remaining);

        let positions = pages
            .iter()
            .map(|page| parse_response::<SearchData>(page, &self.company_id, "search page"))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flat_map(|page| page.positions)
            .collect::<Vec<_>>();
        let detail_requests = positions
            .iter()
            .map(|position| (self.client.clone(), detail_url(&base, position.id)))
            .collect::<Vec<_>>();
        let details = stream::iter(detail_requests)
            .map(|(client, url)| async move {
                send_text(client.get(url), "Microsoft career detail").await
            })
            .buffered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        Ok(SourceScan::Complete {
            observations: parse_microsoft_pages(
                &self.company_id,
                &self.search_url,
                &pages.iter().map(String::as_str).collect::<Vec<_>>(),
                &details.iter().map(String::as_str).collect::<Vec<_>>(),
            )?,
        })
    }
}

pub fn parse_microsoft_pages(
    company_id: &str,
    search_url: &str,
    pages: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    official_search_url(search_url, company_id)?;
    if pages.is_empty() {
        return Err(schema(company_id, "search returned no pages"));
    }
    let pages = pages
        .iter()
        .map(|page| parse_response::<SearchData>(page, company_id, "search page"))
        .collect::<Result<Vec<_>, _>>()?;
    let count = pages[0].count;
    if count == 0 || pages.len() != count.div_ceil(PAGE_SIZE) {
        return Err(schema(company_id, "incomplete search pagination"));
    }

    let mut positions = Vec::with_capacity(count);
    let mut platform_ids = HashSet::new();
    let mut job_ids = HashSet::new();
    let mut paths = HashSet::new();
    for (page_index, page) in pages.into_iter().enumerate() {
        let expected = (count - page_index * PAGE_SIZE).min(PAGE_SIZE);
        if page.count != count || page.positions.len() != expected {
            return Err(schema(company_id, "inconsistent search pagination"));
        }
        for position in page.positions {
            validate_listing(&position, company_id)?;
            if !platform_ids.insert(position.id)
                || !job_ids.insert(position.display_job_id.clone())
                || !paths.insert(position.position_url.clone())
            {
                return Err(schema(company_id, "duplicate Microsoft vacancy"));
            }
            positions.push(position);
        }
    }
    if positions.len() != count || details.len() != count {
        return Err(schema(company_id, "search total does not match details"));
    }

    let details = details
        .iter()
        .map(|detail| parse_response::<Detail>(detail, company_id, "job detail"))
        .collect::<Result<Vec<_>, _>>()?;
    let mut observations = Vec::new();
    for (listing, detail) in positions.into_iter().zip(details) {
        validate_detail(&listing, &detail, company_id)?;
        let nl_locations = detail
            .locations
            .iter()
            .zip(&detail.standardized_locations)
            .filter(|(_, standardized)| is_netherlands(standardized))
            .map(|(location, _)| normalized(location))
            .collect::<Vec<_>>();
        if nl_locations.is_empty() {
            continue;
        }

        let published_at = DateTime::from_timestamp(detail.posted_ts, 0).ok_or_else(|| {
            schema(
                company_id,
                format!(
                    "vacancy {} has invalid posted timestamp",
                    detail.display_job_id
                ),
            )
        })?;
        let description = html_markdown(&detail.job_description);
        if description.is_empty() {
            return Err(schema(
                company_id,
                format!("vacancy {} has no description", detail.display_job_id),
            ));
        }
        let raw_payload = serde_json::to_value(&detail).map_err(|error| {
            schema(
                company_id,
                format!("vacancy {} payload: {error}", detail.display_job_id),
            )
        })?;

        observations.push(ObservedJob {
            source_id: detail.display_job_id,
            title: normalized(&detail.name),
            department: nonempty(detail.department),
            team: None,
            employment_type: detail
                .employment_type
                .into_iter()
                .map(|value| normalized(&value))
                .find(|value| !value.is_empty()),
            locations: nl_locations,
            countries: vec!["NL".to_owned()],
            job_url: detail.public_url.clone(),
            apply_url: detail.public_url,
            description,
            raw_payload,
            published_at: Some(published_at),
        });
    }
    Ok(observations)
}

fn validate_listing(position: &Position, company_id: &str) -> Result<(), SourceError> {
    let expected_path = format!("/careers/job/{}", position.id);
    if position.id == 0
        || normalized(&position.display_job_id).is_empty()
        || position.display_job_id != position.ats_job_id
        || normalized(&position.name).is_empty()
        || position.locations.is_empty()
        || position.locations.len() != position.standardized_locations.len()
        || position.posted_ts <= 0
        || position.position_url != expected_path
    {
        return Err(schema(
            company_id,
            "Microsoft listing has invalid required data",
        ));
    }
    Ok(())
}

fn validate_detail(
    listing: &Position,
    detail: &Detail,
    company_id: &str,
) -> Result<(), SourceError> {
    let expected_path = format!("/careers/job/{}", detail.id);
    let expected_url = format!("https://apply.careers.microsoft.com{expected_path}");
    if listing.id != detail.id
        || listing.display_job_id != detail.display_job_id
        || listing.ats_job_id != detail.ats_job_id
        || listing.name != detail.name
        || listing.locations != detail.locations
        || listing.standardized_locations != detail.standardized_locations
        || listing.posted_ts != detail.posted_ts
        || listing.position_url != detail.position_url
    {
        return Err(schema(
            company_id,
            format!("vacancy {} listing/detail mismatch", listing.display_job_id),
        ));
    }
    if detail.position_url != expected_path
        || detail.public_url != expected_url
        || detail.locations.is_empty()
        || detail.locations.len() != detail.standardized_locations.len()
        || normalized(&detail.name).is_empty()
        || detail.job_description.trim().is_empty()
    {
        return Err(schema(
            company_id,
            format!("vacancy {} has invalid detail data", detail.display_job_id),
        ));
    }
    Ok(())
}

fn parse_response<T: for<'de> Deserialize<'de>>(
    raw: &str,
    company_id: &str,
    label: &str,
) -> Result<T, SourceError> {
    let response: ApiResponse<T> = serde_json::from_str(raw)
        .map_err(|error| schema(company_id, format!("invalid {label} JSON: {error}")))?;
    if response.status != 200 {
        return Err(schema(company_id, format!("{label} status is not 200")));
    }
    Ok(response.data)
}

fn official_search_url(value: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid Microsoft search URL: {error}")))?;
    let pairs = url.query_pairs().collect::<Vec<_>>();
    let expected = [
        ("domain", "microsoft.com"),
        ("query", ""),
        ("location", "Netherlands"),
        ("start", "0"),
        ("hl", "en"),
    ];
    if url.scheme() != "https"
        || url.host_str() != Some("apply.careers.microsoft.com")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/api/pcsx/search"
        || url.fragment().is_some()
        || pairs.len() != expected.len()
        || !expected.iter().all(|(key, value)| {
            pairs
                .iter()
                .filter(|(candidate, _)| candidate == key)
                .map(|(_, candidate)| candidate.as_ref())
                .eq([*value])
        })
    {
        return Err(schema(
            company_id,
            "must use the exact official Microsoft Netherlands search API",
        ));
    }
    Ok(url)
}

fn page_url(base: &Url, start: usize) -> Url {
    let mut url = base.clone();
    url.query_pairs_mut()
        .clear()
        .append_pair("domain", "microsoft.com")
        .append_pair("query", "")
        .append_pair("location", "Netherlands")
        .append_pair("start", &start.to_string())
        .append_pair("hl", "en");
    url
}

fn detail_url(base: &Url, id: u64) -> Url {
    let mut url = base.clone();
    url.set_path("/api/pcsx/position_details");
    url.query_pairs_mut()
        .clear()
        .append_pair("position_id", &id.to_string())
        .append_pair("domain", "microsoft.com")
        .append_pair("hl", "en")
        .append_pair("queried_location", "Netherlands");
    url
}

fn normalized(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn nonempty(value: Option<String>) -> Option<String> {
    value
        .map(|value| normalized(&value))
        .filter(|value| !value.is_empty())
}

fn is_netherlands(value: &str) -> bool {
    value.trim() == "NL" || value.trim().ends_with(", NL")
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("{message} for {company_id}"))
}

#[derive(Deserialize)]
struct ApiResponse<T> {
    status: u16,
    data: T,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchData {
    count: usize,
    positions: Vec<Position>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Position {
    id: u64,
    display_job_id: String,
    name: String,
    locations: Vec<String>,
    standardized_locations: Vec<String>,
    posted_ts: i64,
    ats_job_id: String,
    position_url: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Detail {
    id: u64,
    display_job_id: String,
    name: String,
    locations: Vec<String>,
    standardized_locations: Vec<String>,
    posted_ts: i64,
    department: Option<String>,
    ats_job_id: String,
    position_url: String,
    public_url: String,
    job_description: String,
    #[serde(default, rename = "efcustomTextEmploymentType")]
    employment_type: Vec<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, Value>,
}
