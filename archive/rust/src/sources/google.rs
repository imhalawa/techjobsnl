use std::collections::HashSet;

use chrono::DateTime;
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use serde_json::Value;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

const PAGE_SIZE: usize = 20;
const DATA_MARKER: &str = "AF_initDataCallback({key: 'ds:1'";
const DATA_END: &str = ", sideChannel:";

pub struct GoogleSource {
    company_id: String,
    search_url: String,
    client: Client,
}

impl GoogleSource {
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
impl JobSource for GoogleSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let base = official_search_url(&self.search_url, &self.company_id)?;
        let first = send_text(self.client.get(page_url(&base, 1)), "Google Careers").await?;
        let first_page = parse_page(&first, &self.company_id)?;
        if first_page.total == 0 || first_page.jobs.len() != first_page.total.min(PAGE_SIZE) {
            return Err(schema(&self.company_id, "invalid first-page result count"));
        }

        let requests = (2..=first_page.total.div_ceil(PAGE_SIZE))
            .map(|page| (self.client.clone(), page_url(&base, page)))
            .collect::<Vec<_>>();
        let remaining =
            stream::iter(requests)
                .map(|(client, url)| async move {
                    send_text(client.get(url), "Google Careers page").await
                })
                .buffered(4)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
        let mut pages = vec![first];
        pages.extend(remaining);

        Ok(SourceScan::Complete {
            observations: parse_google_pages(
                &self.company_id,
                &self.search_url,
                &pages.iter().map(String::as_str).collect::<Vec<_>>(),
            )?,
        })
    }
}

pub fn parse_google_pages(
    company_id: &str,
    search_url: &str,
    pages: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    official_search_url(search_url, company_id)?;
    if pages.is_empty() {
        return Err(schema(company_id, "search returned no pages"));
    }
    let pages = pages
        .iter()
        .map(|page| parse_page(page, company_id))
        .collect::<Result<Vec<_>, _>>()?;
    let total = pages[0].total;
    if total == 0 || pages.len() != total.div_ceil(PAGE_SIZE) {
        return Err(schema(company_id, "incomplete search pagination"));
    }

    let mut ids = HashSet::new();
    let mut apply_urls = HashSet::new();
    let mut observations = Vec::with_capacity(total);
    for (page_index, page) in pages.into_iter().enumerate() {
        let expected = (total - page_index * PAGE_SIZE).min(PAGE_SIZE);
        if page.total != total || page.jobs.len() != expected || page.reported_count != expected {
            return Err(schema(company_id, "inconsistent search pagination"));
        }
        for job in page.jobs {
            let id = required_string(&job, 0, company_id, "ID")?.to_owned();
            if !id.bytes().all(|byte| byte.is_ascii_digit()) || !ids.insert(id.clone()) {
                return Err(schema(company_id, "invalid or duplicate Google vacancy ID"));
            }
            let apply_url = required_string(&job, 2, company_id, "application URL")?.to_owned();
            validate_apply_url(&apply_url, company_id)?;
            if !apply_urls.insert(apply_url.clone()) {
                return Err(schema(company_id, "duplicate Google application URL"));
            }
            if let Some(observation) = observed_job(company_id, &id, &apply_url, job)? {
                observations.push(observation);
            }
        }
    }
    if ids.len() != total {
        return Err(schema(company_id, "search total does not match jobs"));
    }
    Ok(observations)
}

fn observed_job(
    company_id: &str,
    id: &str,
    apply_url: &str,
    job: Value,
) -> Result<Option<ObservedJob>, SourceError> {
    let fields = job
        .as_array()
        .filter(|fields| fields.len() == 21)
        .ok_or_else(|| schema(company_id, format!("vacancy {id} field layout drifted")))?;
    let title = normalized(required_string(&job, 1, company_id, "title")?);
    if title.is_empty()
        || fields.get(7).and_then(Value::as_str) != Some("Google")
        || fields.get(8).and_then(Value::as_str) != Some("en-US")
    {
        return Err(schema(
            company_id,
            format!("vacancy {id} has invalid identity"),
        ));
    }

    let locations = fields
        .get(9)
        .and_then(Value::as_array)
        .ok_or_else(|| schema(company_id, format!("vacancy {id} has no locations")))?;
    if locations.is_empty() {
        return Err(schema(company_id, format!("vacancy {id} has no locations")));
    }
    let mut nl_locations = Vec::new();
    for location in locations {
        let values = location
            .as_array()
            .filter(|values| values.len() >= 6)
            .ok_or_else(|| schema(company_id, format!("vacancy {id} location layout drifted")))?;
        let label = values[0]
            .as_str()
            .map(normalized)
            .filter(|label| !label.is_empty())
            .ok_or_else(|| schema(company_id, format!("vacancy {id} has invalid location")))?;
        let country = values[5]
            .as_str()
            .filter(|country| country.len() == 2)
            .ok_or_else(|| schema(company_id, format!("vacancy {id} has invalid country")))?;
        if country == "NL" {
            nl_locations.push(label);
        }
    }
    if nl_locations.is_empty() {
        return Ok(None);
    }

    let timestamp = fields
        .get(12)
        .and_then(Value::as_array)
        .and_then(|timestamp| timestamp.first())
        .and_then(Value::as_i64)
        .filter(|timestamp| *timestamp > 0)
        .ok_or_else(|| schema(company_id, format!("vacancy {id} has invalid date")))?;
    let published_at = DateTime::from_timestamp(timestamp, 0)
        .ok_or_else(|| schema(company_id, format!("vacancy {id} has invalid date")))?;
    let description = [
        section(&job, 10, None),
        section(&job, 4, Some("Qualifications")),
        section(&job, 3, Some("Responsibilities")),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n\n");
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("vacancy {id} has no description"),
        ));
    }

    Ok(Some(ObservedJob {
        source_id: id.to_owned(),
        title,
        department: None,
        team: None,
        employment_type: None,
        locations: nl_locations,
        countries: vec!["NL".to_owned()],
        job_url: format!("https://www.google.com/about/careers/applications/jobs/results/{id}"),
        apply_url: apply_url.to_owned(),
        description,
        raw_payload: job,
        published_at: Some(published_at),
    }))
}

fn section(job: &Value, index: usize, heading: Option<&str>) -> Option<String> {
    let text = job
        .get(index)?
        .as_array()?
        .get(1)?
        .as_str()
        .map(html_markdown)
        .filter(|text| !text.is_empty())?;
    Some(match heading {
        Some(heading) => format!("## {heading}\n\n{text}"),
        None => text,
    })
}

struct SearchPage {
    jobs: Vec<Value>,
    total: usize,
    reported_count: usize,
}

fn parse_page(html: &str, company_id: &str) -> Result<SearchPage, SourceError> {
    let callback = html
        .find(DATA_MARKER)
        .ok_or_else(|| schema(company_id, "Google search data callback is missing"))?;
    let data_start = html[callback..]
        .find("data:")
        .map(|offset| callback + offset + "data:".len())
        .ok_or_else(|| schema(company_id, "Google search data is missing"))?;
    let data_end = html[data_start..]
        .find(DATA_END)
        .map(|offset| data_start + offset)
        .ok_or_else(|| schema(company_id, "Google search data end is missing"))?;
    let data: Value = serde_json::from_str(&html[data_start..data_end])
        .map_err(|error| schema(company_id, format!("invalid Google search data: {error}")))?;
    let values = data
        .as_array()
        .filter(|values| values.len() == 4 && values[1].is_null())
        .ok_or_else(|| schema(company_id, "Google search result layout drifted"))?;
    let jobs = values[0]
        .as_array()
        .cloned()
        .ok_or_else(|| schema(company_id, "Google search jobs are missing"))?;
    let total = usize::try_from(
        values[2]
            .as_u64()
            .ok_or_else(|| schema(company_id, "Google search total is missing"))?,
    )
    .map_err(|_| schema(company_id, "Google search total is too large"))?;
    let reported_count = usize::try_from(
        values[3]
            .as_u64()
            .ok_or_else(|| schema(company_id, "Google page count is missing"))?,
    )
    .map_err(|_| schema(company_id, "Google page count is too large"))?;
    Ok(SearchPage {
        jobs,
        total,
        reported_count,
    })
}

fn official_search_url(search_url: &str, company_id: &str) -> Result<Url, SourceError> {
    const OFFICIAL: &str = "https://www.google.com/about/careers/applications/jobs/results/?company=Google&location=Netherlands&sort_by=date";
    if search_url != OFFICIAL {
        return Err(schema(
            company_id,
            "must use the exact official Google Netherlands search URL",
        ));
    }
    Url::parse(search_url)
        .map_err(|error| schema(company_id, format!("invalid search URL: {error}")))
}

fn page_url(base: &Url, page: usize) -> Url {
    let mut url = base.clone();
    if page > 1 {
        url.query_pairs_mut().append_pair("page", &page.to_string());
    }
    url
}

fn validate_apply_url(value: &str, company_id: &str) -> Result<(), SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid application URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some("www.google.com")
        || url.path() != "/about/careers/applications/signin"
        || !url
            .query_pairs()
            .any(|(key, value)| key == "jobId" && !value.is_empty())
    {
        return Err(schema(company_id, "application URL left Google Careers"));
    }
    Ok(())
}

fn required_string<'a>(
    job: &'a Value,
    index: usize,
    company_id: &str,
    field: &str,
) -> Result<&'a str, SourceError> {
    job.get(index)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| schema(company_id, format!("Google vacancy has no {field}")))
}

fn normalized(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn schema(company_id: &str, message: impl Into<String>) -> SourceError {
    SourceError::schema(format!("{company_id}: {}", message.into()))
}
