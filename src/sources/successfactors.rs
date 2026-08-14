use std::collections::HashSet;

use chrono::NaiveDate;
use futures_util::{StreamExt, stream};
use reqwest::{Client, Url};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text, json_ld::html_markdown};

const PAGE_SIZE: usize = 25;

pub struct SuccessFactorsSource {
    company_id: String,
    employer: String,
    base_url: String,
    client: Client,
}

impl SuccessFactorsSource {
    pub fn new(
        company_id: impl Into<String>,
        employer: impl Into<String>,
        base_url: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            company_id: company_id.into(),
            employer: employer.into(),
            base_url: base_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for SuccessFactorsSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let base = official_base_url(&self.base_url, &self.company_id)?;
        let mut search_url = base.join("search/").expect("validated base URL must join");
        search_url
            .query_pairs_mut()
            .append_pair("q", "")
            .append_pair("locationsearch", "Netherlands")
            .append_pair("locale", "en_US");
        let search_html = send_text(self.client.get(search_url), &self.employer).await?;
        let csrf = csrf_token(&search_html, &self.company_id)?;
        let endpoint = base
            .join("services/recruiting/v1/jobs")
            .expect("validated base URL must join");

        let first = send_text(
            self.client
                .post(endpoint.clone())
                .header("X-CSRF-Token", &csrf)
                .json(&search_request(0)),
            &self.employer,
        )
        .await?;
        let first_page: SearchPage = serde_json::from_str(&first)
            .map_err(|error| schema(&self.company_id, format!("invalid search JSON: {error}")))?;
        if first_page.total_jobs == 0 {
            return Err(schema(
                &self.company_id,
                "search returned no Netherlands jobs",
            ));
        }

        let page_count = first_page.total_jobs.div_ceil(PAGE_SIZE);
        let mut pages = vec![first];
        for page_number in 1..page_count {
            pages.push(
                send_text(
                    self.client
                        .post(endpoint.clone())
                        .header("X-CSRF-Token", &csrf)
                        .json(&search_request(page_number)),
                    &self.employer,
                )
                .await?,
            );
        }
        let page_refs = pages.iter().map(String::as_str).collect::<Vec<_>>();
        let listings =
            parse_search_pages(&self.company_id, &self.employer, &self.base_url, &page_refs)?;
        let requests = listings
            .iter()
            .map(|job| (self.client.clone(), job.detail_url.clone()))
            .collect::<Vec<_>>();
        let details = stream::iter(requests)
            .map(|(client, url)| async move { send_text(client.get(url), "SuccessFactors").await })
            .buffered(8)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;
        let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();

        Ok(SourceScan::Complete {
            observations: parse_details(&self.company_id, listings, &detail_refs)?,
        })
    }
}

pub fn parse_successfactors_pages(
    company_id: &str,
    employer: &str,
    base_url: &str,
    search_pages: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let listings = parse_search_pages(company_id, employer, base_url, search_pages)?;
    parse_details(company_id, listings, details)
}

fn parse_search_pages(
    company_id: &str,
    employer: &str,
    base_url: &str,
    pages: &[&str],
) -> Result<Vec<ListingJob>, SourceError> {
    let base = official_base_url(base_url, company_id)?;
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
    let total = parsed[0].total_jobs;
    if total == 0 || parsed.len() != total.div_ceil(PAGE_SIZE) {
        return Err(schema(company_id, "incomplete search pagination"));
    }

    let mut ids = HashSet::new();
    let mut jobs = Vec::with_capacity(total);
    for (page_number, page) in parsed.into_iter().enumerate() {
        let expected = (total - page_number * PAGE_SIZE).min(PAGE_SIZE);
        if page.total_jobs != total || page.results.len() != expected {
            return Err(schema(company_id, "inconsistent search pagination"));
        }
        for result in page.results {
            let response = result.response;
            let id = normalized(&response.id);
            let title = normalized(&response.title);
            let slug = normalized(&response.slug);
            if id.is_empty()
                || !id.bytes().all(|byte| byte.is_ascii_digit())
                || title.is_empty()
                || slug.is_empty()
                || slug.contains('/')
                || response.brand.as_slice() != [employer]
                || !ids.insert(id.clone())
            {
                return Err(schema(
                    company_id,
                    "vacancy has invalid or duplicate identity",
                ));
            }
            let locations = response
                .locations
                .iter()
                .filter_map(|location| location.trim().strip_prefix("Netherlands-"))
                .map(normalized)
                .filter(|location| !location.is_empty())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            if locations.is_empty() {
                return Err(schema(
                    company_id,
                    format!("vacancy {id} has no explicit Netherlands location"),
                ));
            }
            let published_at = NaiveDate::parse_from_str(response.start_date.trim(), "%m/%d/%y")
                .map_err(|error| schema(company_id, format!("vacancy {id} date: {error}")))?
                .and_hms_opt(0, 0, 0)
                .expect("midnight must be valid")
                .and_utc();
            let detail_url = base
                .join(&format!("job/{slug}/{id}-en_US/"))
                .map_err(|error| schema(company_id, format!("vacancy {id} URL: {error}")))?;
            let raw_payload = serde_json::to_value(&response).map_err(|error| {
                schema(
                    company_id,
                    format!("could not preserve vacancy payload: {error}"),
                )
            })?;
            jobs.push(ListingJob {
                id,
                title,
                locations,
                department: first(response.job_area),
                team: first(response.job_family),
                employment_type: first(response.contract),
                published_at,
                detail_url,
                raw_payload,
            });
        }
    }
    if jobs.len() != total {
        return Err(schema(company_id, "search total does not match vacancies"));
    }
    Ok(jobs)
}

fn parse_details(
    company_id: &str,
    listings: Vec<ListingJob>,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    if listings.len() != details.len() {
        return Err(schema(company_id, "listing/detail count mismatch"));
    }
    listings
        .into_iter()
        .zip(details)
        .map(|(listing, detail)| observed_job(company_id, listing, detail))
        .collect()
}

fn observed_job(
    company_id: &str,
    listing: ListingJob,
    detail: &str,
) -> Result<ObservedJob, SourceError> {
    let document = Html::parse_document(detail);
    let canonical = element_attr(&document, "link[rel=canonical][href]", "href", company_id)?;
    let job_url = Url::parse(&canonical).map_err(|error| {
        schema(
            company_id,
            format!("vacancy {} canonical URL: {error}", listing.id),
        )
    })?;
    if job_url.scheme() != "https"
        || job_url.host_str() != listing.detail_url.host_str()
        || job_url.path() != listing.detail_url.path()
        || job_url.query().is_some()
        || job_url.fragment().is_some()
    {
        return Err(schema(
            company_id,
            format!("vacancy {} canonical URL mismatch", listing.id),
        ));
    }
    let title = element_text(&document, "[itemprop=title]", company_id)?;
    if title != listing.title {
        return Err(schema(
            company_id,
            format!("vacancy {} title mismatch", listing.id),
        ));
    }
    let description_selector =
        Selector::parse("[itemprop=description]").expect("static selector must parse");
    let description = document
        .select(&description_selector)
        .map(|element| html_markdown(&element.inner_html()))
        .filter(|section| !section.is_empty())
        .collect::<Vec<_>>()
        .join("\n\n");
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("vacancy {} has no description", listing.id),
        ));
    }
    let apply = element_attr(&document, "a.unify-apply-now[href]", "href", company_id)?;
    let apply_url = job_url.join(&apply).map_err(|error| {
        schema(
            company_id,
            format!("vacancy {} apply URL: {error}", listing.id),
        )
    })?;
    let expected_path = format!("/talentcommunity/apply/{}/", listing.id);
    let query_ok = apply_url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .eq([("locale".to_owned(), "en_US".to_owned())]);
    if apply_url.scheme() != "https"
        || apply_url.host_str() != job_url.host_str()
        || apply_url.path() != expected_path
        || !query_ok
        || apply_url.fragment().is_some()
    {
        return Err(schema(
            company_id,
            format!("vacancy {} has invalid apply URL", listing.id),
        ));
    }

    Ok(ObservedJob {
        source_id: listing.id,
        title,
        department: listing.department,
        team: listing.team,
        employment_type: listing.employment_type,
        locations: listing.locations,
        countries: vec!["NL".to_owned()],
        job_url: job_url.to_string(),
        apply_url: apply_url.to_string(),
        description,
        raw_payload: json!({"listing": listing.raw_payload}),
        published_at: Some(listing.published_at),
    })
}

fn official_base_url(value: &str, company_id: &str) -> Result<Url, SourceError> {
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid SuccessFactors URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.path() != "/"
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(schema(company_id, "invalid SuccessFactors base URL"));
    }
    Ok(url)
}

fn csrf_token(html: &str, company_id: &str) -> Result<String, SourceError> {
    html.split_once("var CSRFToken = \"")
        .and_then(|(_, rest)| rest.split_once('"'))
        .map(|(token, _)| token.to_owned())
        .filter(|token| !token.is_empty())
        .ok_or_else(|| schema(company_id, "search page has no CSRF token"))
}

fn search_request(page_number: usize) -> serde_json::Value {
    json!({
        "keywords": "",
        "locale": "en_US",
        "location": "Netherlands",
        "pageNumber": page_number,
        "sortBy": "recent"
    })
}

fn element_attr(
    document: &Html,
    selector: &str,
    attribute: &str,
    company_id: &str,
) -> Result<String, SourceError> {
    let selector = Selector::parse(selector).expect("static selector must parse");
    document
        .select(&selector)
        .next()
        .and_then(|element| element.value().attr(attribute))
        .map(normalized)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail has no {attribute}")))
}

fn element_text(document: &Html, selector: &str, company_id: &str) -> Result<String, SourceError> {
    let selector = Selector::parse(selector).expect("static selector must parse");
    document
        .select(&selector)
        .next()
        .map(|element| normalized(&element.text().collect::<String>()))
        .filter(|value| !value.is_empty())
        .ok_or_else(|| schema(company_id, "detail has no title"))
}

fn first(values: Vec<String>) -> Option<String> {
    values
        .into_iter()
        .map(|value| normalized(&value))
        .find(|value| !value.is_empty())
}

fn normalized(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn schema(company_id: &str, message: impl Into<String>) -> SourceError {
    SourceError::schema(format!("{company_id}: {}", message.into()))
}

#[derive(Deserialize)]
struct SearchPage {
    #[serde(rename = "totalJobs")]
    total_jobs: usize,
    #[serde(rename = "jobSearchResult", default)]
    results: Vec<SearchResult>,
}

#[derive(Deserialize)]
struct SearchResult {
    response: SearchJob,
}

#[derive(Deserialize, Serialize)]
struct SearchJob {
    id: String,
    #[serde(rename = "unifiedStandardTitle")]
    title: String,
    #[serde(rename = "unifiedUrlTitle")]
    slug: String,
    #[serde(rename = "unifiedStandardStart")]
    start_date: String,
    #[serde(rename = "jobLocationShort")]
    locations: Vec<String>,
    #[serde(rename = "jobContract", default)]
    contract: Vec<String>,
    #[serde(rename = "custJobArea", default)]
    job_area: Vec<String>,
    #[serde(rename = "custJobFamily", default)]
    job_family: Vec<String>,
    #[serde(rename = "sfstd_marketingBrand_obj")]
    brand: Vec<String>,
}

struct ListingJob {
    id: String,
    title: String,
    locations: Vec<String>,
    department: Option<String>,
    team: Option<String>,
    employment_type: Option<String>,
    published_at: chrono::DateTime<chrono::Utc>,
    detail_url: Url,
    raw_payload: serde_json::Value,
}
