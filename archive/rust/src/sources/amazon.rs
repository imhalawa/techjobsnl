use std::collections::HashSet;

use chrono::NaiveDate;
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

pub struct AmazonSource {
    company_id: String,
    search_url: String,
    client: Client,
}

impl AmazonSource {
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
impl JobSource for AmazonSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let (base, limit) = official_search_url(&self.search_url, &self.company_id)?;
        let first = send_text(self.client.get(page_url(&base, 0, limit)), "Amazon jobs").await?;
        let first_page: SearchPage = serde_json::from_str(&first)
            .map_err(|error| schema(&self.company_id, format!("invalid search JSON: {error}")))?;
        if first_page.hits == 0 || first_page.jobs.len() != first_page.hits.min(limit) {
            return Err(schema(&self.company_id, "invalid first-page result count"));
        }

        let requests = (limit..first_page.hits)
            .step_by(limit)
            .map(|offset| (self.client.clone(), page_url(&base, offset, limit)))
            .collect::<Vec<_>>();
        let remaining = stream::iter(requests)
            .map(
                |(client, url)| async move { send_text(client.get(url), "Amazon jobs page").await },
            )
            .buffered(4)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let mut pages = vec![first];
        pages.extend(remaining);
        let page_refs = pages.iter().map(String::as_str).collect::<Vec<_>>();

        Ok(SourceScan::Complete {
            observations: parse_amazon_pages(&self.company_id, &self.search_url, &page_refs)?,
        })
    }
}

pub fn parse_amazon_pages(
    company_id: &str,
    search_url: &str,
    pages: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let (_, limit) = official_search_url(search_url, company_id)?;
    if pages.is_empty() {
        return Err(schema(company_id, "search returned no pages"));
    }
    let parsed = pages
        .iter()
        .map(|page| {
            serde_json::from_str::<SearchPage>(page)
                .map_err(|error| schema(company_id, format!("invalid search JSON: {error}")))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let hits = parsed[0].hits;
    if hits == 0 || parsed.len() != hits.div_ceil(limit) {
        return Err(schema(company_id, "incomplete search pagination"));
    }

    let mut feed_ids = HashSet::new();
    let mut source_ids = HashSet::new();
    let mut paths = HashSet::new();
    let mut observations = Vec::with_capacity(hits);
    for (page_index, page) in parsed.into_iter().enumerate() {
        let offset = page_index * limit;
        let expected_count = (hits - offset).min(limit);
        if page.hits != hits || page.jobs.len() != expected_count {
            return Err(schema(company_id, "inconsistent search pagination"));
        }
        for job in page.jobs {
            if !feed_ids.insert(job.id.clone())
                || !source_ids.insert(job.id_icims.clone())
                || !paths.insert(job.job_path.clone())
            {
                return Err(schema(company_id, "duplicate Amazon vacancy"));
            }
            observations.push(observed_job(company_id, job)?);
        }
    }
    if observations.len() != hits {
        return Err(schema(company_id, "search total does not match jobs"));
    }
    Ok(observations)
}

fn observed_job(company_id: &str, job: AmazonJob) -> Result<ObservedJob, SourceError> {
    let title = normalized(&job.title);
    let city = normalized(&job.city);
    let company_name = normalized(&job.company_name);
    let expected_prefix = format!("/en/jobs/{}/", job.id_icims);
    if job.id.trim().is_empty()
        || job.id_icims.trim().is_empty()
        || title.is_empty()
        || company_name.is_empty()
        || city.is_empty()
        || job.country_code != "NLD"
        || !job.location.starts_with("NL,")
        || !job.normalized_location.ends_with(", NLD")
        || !job.job_path.starts_with(&expected_prefix)
        || job.job_path.len() == expected_prefix.len()
    {
        return Err(schema(
            company_id,
            format!("vacancy {} has invalid identity or location", job.id_icims),
        ));
    }
    validate_apply_url(&job, company_id)?;

    let date = normalized(&job.posted_date);
    let published_at = NaiveDate::parse_from_str(&date, "%B %-d, %Y")
        .map_err(|error| {
            schema(
                company_id,
                format!("vacancy {} date: {error}", job.id_icims),
            )
        })?
        .and_hms_opt(0, 0, 0)
        .expect("midnight must be valid")
        .and_utc();
    let mut sections = vec![html_markdown(&job.description)];
    for (heading, content) in [
        ("Basic qualifications", job.basic_qualifications.as_deref()),
        (
            "Preferred qualifications",
            job.preferred_qualifications.as_deref(),
        ),
    ] {
        if let Some(content) = content.map(html_markdown).filter(|text| !text.is_empty()) {
            sections.push(format!("## {heading}\n\n{content}"));
        }
    }
    if sections[0].is_empty() {
        return Err(schema(
            company_id,
            format!("vacancy {} has no description", job.id_icims),
        ));
    }
    let raw_payload = serde_json::to_value(&job).map_err(|error| {
        schema(
            company_id,
            format!("vacancy {} payload: {error}", job.id_icims),
        )
    })?;

    Ok(ObservedJob {
        source_id: job.id_icims,
        title,
        department: nonempty(job.job_category),
        team: nonempty(job.job_family),
        employment_type: nonempty(job.job_schedule_type),
        locations: vec![city],
        countries: vec!["NL".to_owned()],
        job_url: format!("https://www.amazon.jobs{}", job.job_path),
        apply_url: job.url_next_step,
        description: sections.join("\n\n"),
        raw_payload,
        published_at: Some(published_at),
    })
}

fn validate_apply_url(job: &AmazonJob, company_id: &str) -> Result<(), SourceError> {
    let url = Url::parse(&job.url_next_step).map_err(|error| {
        schema(
            company_id,
            format!("vacancy {} apply URL: {error}", job.id_icims),
        )
    })?;
    let standard = matches!(
        url.host_str(),
        Some("account.amazon.jobs" | "account.amazon.com")
    ) && url.path() == format!("/jobs/{}/apply", job.id_icims);
    let salesforce = url.host_str() == Some("hvr-amazon.my.site.com")
        && url.path() == "/JobDetails"
        && url
            .query_pairs()
            .any(|(key, value)| key == "reqid" && value == job.id)
        && url
            .query_pairs()
            .any(|(key, value)| key == "isapply" && value == "1");
    if url.scheme() != "https"
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || (!standard && !salesforce)
    {
        return Err(schema(
            company_id,
            format!("vacancy {} has invalid apply URL", job.id_icims),
        ));
    }
    Ok(())
}

fn official_search_url(value: &str, company_id: &str) -> Result<(Url, usize), SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid Amazon search URL: {error}")))?;
    let pairs = url.query_pairs().collect::<Vec<_>>();
    let country_ok = pairs
        .iter()
        .filter(|(key, _)| key == "normalized_country_code[]")
        .map(|(_, value)| value.as_ref())
        .eq(["NLD"]);
    let offset_ok = pairs
        .iter()
        .filter(|(key, _)| key == "offset")
        .map(|(_, value)| value.as_ref())
        .eq(["0"]);
    let limits = pairs
        .iter()
        .filter(|(key, _)| key == "result_limit")
        .filter_map(|(_, value)| value.parse::<usize>().ok())
        .collect::<Vec<_>>();
    if url.scheme() != "https"
        || url.host_str() != Some("www.amazon.jobs")
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/en/search.json"
        || url.fragment().is_some()
        || !country_ok
        || !offset_ok
        || limits.len() != 1
        || !(1..=100).contains(&limits[0])
        || pairs.len() != 3
    {
        return Err(schema(
            company_id,
            "must use the official Amazon NL search API",
        ));
    }
    Ok((url, limits[0]))
}

fn page_url(base: &Url, offset: usize, limit: usize) -> Url {
    let mut url = base.clone();
    url.query_pairs_mut()
        .clear()
        .append_pair("normalized_country_code[]", "NLD")
        .append_pair("offset", &offset.to_string())
        .append_pair("result_limit", &limit.to_string());
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

fn schema(company_id: &str, message: impl Into<String>) -> SourceError {
    SourceError::schema(format!("{company_id}: {}", message.into()))
}

#[derive(Deserialize)]
struct SearchPage {
    hits: usize,
    jobs: Vec<AmazonJob>,
}

#[derive(Deserialize, Serialize)]
struct AmazonJob {
    id: String,
    id_icims: String,
    title: String,
    country_code: String,
    city: String,
    location: String,
    normalized_location: String,
    job_path: String,
    url_next_step: String,
    posted_date: String,
    job_category: Option<String>,
    job_family: Option<String>,
    job_schedule_type: Option<String>,
    company_name: String,
    description: String,
    basic_qualifications: Option<String>,
    preferred_qualifications: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}
