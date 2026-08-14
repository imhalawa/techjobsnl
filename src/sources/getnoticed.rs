use std::{collections::HashSet, future::Future};

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use reqwest::{Client, RequestBuilder, Url, redirect::Policy};
use scraper::{Html, Selector};
use serde::Deserialize;
use serde_json::Value;

use crate::domain::{ObservedJob, SourceErrorKind, SourceScan};

use super::{
    JobSource, SourceError,
    http::send_text,
    json_ld::{html_markdown, job_posting_value, parse_job_posting},
};

const REDIRECT_LIMIT: usize = 5;
const MAX_PAGE_COUNT: usize = 100;

#[derive(Clone, Copy)]
struct SiteProfile {
    host: &'static str,
    source_name: &'static str,
    hiring_organization: Option<&'static str>,
    detail_prefix: &'static [&'static str],
}

fn site_profile(company_id: &str) -> Result<SiteProfile, SourceError> {
    match company_id {
        "abn-amro" => Ok(SiteProfile {
            host: "www.werkenbijabnamro.nl",
            source_name: "ABN AMRO",
            hiring_organization: Some("ABN AMRO"),
            detail_prefix: &["en", "vacancy"],
        }),
        "topicus" => Ok(SiteProfile {
            host: "www.werkenbijtopicus.nl",
            source_name: "Topicus",
            hiring_organization: None,
            detail_prefix: &["vacature"],
        }),
        "brand-new-day" => Ok(SiteProfile {
            host: "werkenbij.brandnewday.nl",
            source_name: "Brand New Day",
            hiring_organization: Some("Brand New Day"),
            detail_prefix: &["vacature"],
        }),
        _ => Err(schema(company_id, "unsupported Getnoticed site")),
    }
}

pub struct GetnoticedSource {
    company_id: String,
    base_url: String,
    country_filter: Option<String>,
    client: Client,
}

impl GetnoticedSource {
    pub fn new(
        company_id: impl Into<String>,
        base_url: impl Into<String>,
        country_filter: Option<String>,
        client: Client,
    ) -> Self {
        Self {
            company_id: company_id.into(),
            base_url: base_url.into(),
            country_filter,
            client,
        }
    }

    pub fn listing_request(&self, page: usize) -> Result<RequestBuilder, SourceError> {
        if page == 0 {
            return Err(schema(&self.company_id, "listing page must be at least 1"));
        }
        let mut url = official_base(&self.base_url, &self.company_id)?;
        url.set_path("/api/vacancy/");
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("pageNumber", &page.to_string())
                .append_pair("sort", "created")
                .append_pair("sortDir", "DESC");
            if let Some(country) = self.country_filter.as_deref() {
                query.append_pair("filters[Land][]", country);
            }
        }
        Ok(self
            .client
            .get(url)
            .header("X-Requested-With", "XMLHttpRequest"))
    }

    async fn fetch_listing_pages<F, Fut>(&self, mut fetch: F) -> Result<Vec<String>, SourceError>
    where
        F: FnMut(RequestBuilder) -> Fut,
        Fut: Future<Output = Result<String, SourceError>>,
    {
        let first = fetch(self.listing_request(1)?).await?;
        let first_page = listing_page(&first, &self.company_id)?;
        validate_listing_page(&first_page, 1, 0, None, &self.company_id)?;
        let page_count = first_page.meta.total_page_count;
        let mut pages = vec![first];
        for page in 2..=page_count {
            pages.push(fetch(self.listing_request(page)?).await?);
        }
        Ok(pages)
    }
}

pub fn build_client(user_agent: &str, timeout: std::time::Duration) -> Result<Client, SourceError> {
    Client::builder()
        .user_agent(user_agent)
        .timeout(timeout)
        .redirect(Policy::custom(|attempt| {
            if attempt.previous().len() >= REDIRECT_LIMIT {
                return attempt.error("too many Getnoticed redirects");
            }
            let same_origin = attempt
                .previous()
                .first()
                .is_some_and(|initial| attempt.url().origin() == initial.origin());
            if attempt.url().scheme() == "https" && same_origin {
                attempt.follow()
            } else {
                attempt.error("Getnoticed non-HTTPS or cross-host redirect blocked")
            }
        }))
        .build()
        .map_err(|error| SourceError {
            kind: SourceErrorKind::Configuration,
            message: format!("could not configure Getnoticed HTTP client: {error}"),
            http_status: None,
            retry_after: None,
            retryable: false,
        })
}

#[async_trait::async_trait]
impl JobSource for GetnoticedSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let profile = site_profile(&self.company_id)?;
        let pages = self
            .fetch_listing_pages(|request| send_text(request, profile.source_name))
            .await?;
        let page_refs = pages.iter().map(String::as_str).collect::<Vec<_>>();
        let listings = parse_listings(&self.company_id, &self.base_url, &page_refs)?;

        let requests = listings
            .iter()
            .map(|listing| (self.client.clone(), listing.detail_url.clone()))
            .collect::<Vec<_>>();
        let details =
            stream::iter(requests)
                .map(|(client, url)| async move {
                    send_text(client.get(url), profile.source_name).await
                })
                .buffered(4)
                .collect::<Vec<_>>()
                .await
                .into_iter()
                .collect::<Result<Vec<_>, _>>()?;
        let detail_refs = details.iter().map(String::as_str).collect::<Vec<_>>();

        Ok(SourceScan::Complete {
            observations: parse_details(&self.company_id, &self.base_url, listings, &detail_refs)?,
        })
    }
}

pub fn parse_getnoticed_pages(
    company_id: &str,
    base_url: &str,
    pages: &[&str],
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let listings = parse_listings(company_id, base_url, pages)?;
    parse_details(company_id, base_url, listings, details)
}

fn parse_listings(
    company_id: &str,
    base_url: &str,
    pages: &[&str],
) -> Result<Vec<Listing>, SourceError> {
    let base = official_base(base_url, company_id)?;
    let mut expected_meta = None;
    let mut ids = HashSet::new();
    let mut listings = Vec::new();

    for (index, raw) in pages.iter().enumerate() {
        let requested_page = index + 1;
        let page = listing_page(raw, company_id)?;
        let prior = listings.len();
        validate_listing_page(
            &page,
            requested_page,
            prior,
            expected_meta.as_ref(),
            company_id,
        )?;
        expected_meta.get_or_insert(page.meta.clone());

        for raw_listing in page.vacancies {
            let row: ListingRow = serde_json::from_value(raw_listing.clone())
                .map_err(|error| schema(company_id, format!("invalid listing vacancy: {error}")))?;
            if !ids.insert(row.id) {
                return Err(schema(
                    company_id,
                    format!("duplicate listing vacancy ID {}", row.id),
                ));
            }
            let slug = required(&row.slug, "slug", row.id, company_id)?;
            let title = required(&row.title, "title", row.id, company_id)?;
            let detail_url = detail_url(&base, row.id, &slug, company_id)?;
            let department = joined_categories(&row.subtitle.option_values);
            listings.push(Listing {
                id: row.id,
                title,
                city: required(&row.city, "city", row.id, company_id)?,
                department,
                detail_url,
                raw: raw_listing,
            });
        }
    }

    let meta = expected_meta.ok_or_else(|| schema(company_id, "returned no listing pages"))?;
    if pages.len() != meta.total_page_count.max(1) || listings.len() != meta.num_total_hits {
        return Err(schema(
            company_id,
            format!(
                "listing returned {} pages and {} vacancies; expected {} pages and {} vacancies",
                pages.len(),
                listings.len(),
                meta.total_page_count,
                meta.num_total_hits
            ),
        ));
    }
    Ok(listings)
}

fn parse_details(
    company_id: &str,
    base_url: &str,
    listings: Vec<Listing>,
    details: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    if listings.len() != details.len() {
        return Err(schema(
            company_id,
            format!(
                "received {} details for {} listings",
                details.len(),
                listings.len()
            ),
        ));
    }
    listings
        .into_iter()
        .zip(details)
        .map(|(listing, detail)| observed_job(company_id, base_url, listing, detail))
        .collect()
}

fn observed_job(
    company_id: &str,
    base_url: &str,
    listing: Listing,
    detail: &str,
) -> Result<ObservedJob, SourceError> {
    let profile = site_profile(company_id)?;
    let base = official_base(base_url, company_id)?;
    let document = Html::parse_document(detail);
    validate_vacancy_id(&document, listing.id, company_id)?;
    let canonical = canonical_url(&document, &base, company_id)?;
    validate_canonical_identity(&canonical, listing.id, company_id)?;

    let posting = parse_job_posting(detail, profile.source_name)?;
    let raw_posting = job_posting_value(detail, profile.source_name)?;
    let application_endpoint =
        application_endpoint(&document, &base, listing.id, &raw_posting, company_id)?;
    let title = posting
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| schema(company_id, format!("detail {} has no title", listing.id)))?;
    if title != listing.title {
        return Err(schema(
            company_id,
            format!("detail {} title does not match its listing", listing.id),
        ));
    }
    let hiring_organization = posting
        .hiring_organization
        .as_ref()
        .and_then(|organization| organization.name.as_deref())
        .map(str::trim);
    if hiring_organization.is_none()
        || profile
            .hiring_organization
            .is_some_and(|expected| hiring_organization != Some(expected))
    {
        return Err(schema(
            company_id,
            format!(
                "detail {} has an unexpected hiring organization",
                listing.id
            ),
        ));
    }
    let date = posting.date_posted.as_deref().ok_or_else(|| {
        schema(
            company_id,
            format!("detail {} has no datePosted", listing.id),
        )
    })?;
    let published_at = DateTime::parse_from_rfc3339(date)
        .map_err(|error| {
            schema(
                company_id,
                format!("detail {} has invalid datePosted: {error}", listing.id),
            )
        })?
        .with_timezone(&Utc);
    let description = html_markdown(&posting.description);
    if description.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} has an empty description", listing.id),
        ));
    }
    if posting.job_location.is_empty() {
        return Err(schema(
            company_id,
            format!("detail {} has no locations", listing.id),
        ));
    }
    let mut locations = Vec::new();
    for place in &posting.job_location {
        let location = place
            .name
            .as_deref()
            .or(place.address.address_locality.as_deref())
            .map(str::trim)
            .filter(|location| !location.is_empty())
            .unwrap_or(&listing.city);
        let country_is_nl = place
            .address
            .address_country
            .as_deref()
            .map(str::trim)
            .is_some_and(|country| matches!(country, "Nederland" | "NLD"));
        if !country_is_nl {
            return Err(schema(
                company_id,
                format!("detail {} has an unresolved country", listing.id),
            ));
        }
        if !locations.iter().any(|existing| existing == location) {
            locations.push(location.to_owned());
        }
    }

    let apply_url = apply_url(&base, &canonical, listing.id, company_id);
    Ok(ObservedJob {
        source_id: listing.id.to_string(),
        title: listing.title,
        department: listing.department,
        team: None,
        employment_type: None,
        locations,
        countries: vec!["NL".to_owned()],
        job_url: canonical.to_string(),
        apply_url: apply_url.to_string(),
        description,
        raw_payload: serde_json::json!({
            "listing": listing.raw,
            "jobPosting": raw_posting,
            "applicationEndpoint": application_endpoint,
        }),
        published_at: Some(published_at),
    })
}

fn listing_page(raw: &str, company_id: &str) -> Result<ListingPage, SourceError> {
    serde_json::from_str(raw)
        .map_err(|error| schema(company_id, format!("invalid listing response: {error}")))
}

fn validate_meta(
    meta: &ListingMeta,
    requested_page: usize,
    expected: Option<&ListingMeta>,
    company_id: &str,
) -> Result<(), SourceError> {
    if meta.max_per_page == 0 {
        return Err(schema(company_id, "listing has zero page metadata"));
    }
    // ponytail: 100 pages bounds corrupt request amplification; raise only if ABN's board grows past it.
    if meta.total_page_count > MAX_PAGE_COUNT {
        return Err(schema(company_id, "listing declares too many pages"));
    }
    if meta.page_number != requested_page {
        return Err(schema(
            company_id,
            format!(
                "listing page gap: requested {requested_page}, got {}",
                meta.page_number
            ),
        ));
    }
    let calculated = meta.num_total_hits.div_ceil(meta.max_per_page);
    if meta.total_page_count != calculated {
        return Err(schema(
            company_id,
            "listing totalPageCount disagrees with num_total_hits and maxPerPage",
        ));
    }
    if let Some(expected) = expected
        && (meta.num_total_hits != expected.num_total_hits
            || meta.max_per_page != expected.max_per_page
            || meta.total_page_count != expected.total_page_count)
    {
        return Err(schema(company_id, "listing metadata changed between pages"));
    }
    if meta.num_total_hits > 0 && requested_page > meta.total_page_count {
        return Err(schema(company_id, "listing returned an undeclared page"));
    }
    Ok(())
}

fn validate_listing_page(
    page: &ListingPage,
    requested_page: usize,
    prior: usize,
    expected: Option<&ListingMeta>,
    company_id: &str,
) -> Result<(), SourceError> {
    validate_meta(&page.meta, requested_page, expected, company_id)?;
    let expected_on_page = page
        .meta
        .num_total_hits
        .saturating_sub(prior)
        .min(page.meta.max_per_page);
    if page.vacancies.len() != expected_on_page {
        return Err(schema(
            company_id,
            format!(
                "listing page {requested_page} returned {} of {expected_on_page} expected vacancies",
                page.vacancies.len()
            ),
        ));
    }
    Ok(())
}

fn validate_vacancy_id(
    document: &Html,
    expected: u64,
    company_id: &str,
) -> Result<(), SourceError> {
    let selector = Selector::parse(
        r#"[data-component="Favorite"][data-vacancy-id]:not(.partial_vacancy_list-item)"#,
    )
    .expect("static primary vacancy ID selector");
    let ids = document
        .select(&selector)
        .map(|element| {
            element
                .value()
                .attr("data-vacancy-id")
                .expect("selected attribute exists")
                .parse::<u64>()
        })
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| schema(company_id, format!("invalid data-vacancy-id: {error}")))?;
    if ids.is_empty() || ids.iter().any(|id| *id != expected) {
        return Err(schema(
            company_id,
            format!("detail data-vacancy-id does not match listing {expected}"),
        ));
    }
    Ok(())
}

fn application_endpoint(
    document: &Html,
    base: &Url,
    id: u64,
    posting: &Value,
    company_id: &str,
) -> Result<String, SourceError> {
    if company_id == "topicus" {
        let selector =
            Selector::parse("#ub-apply-form[data-hireserve-form-container][data-vacancy-id]")
                .expect("static Topicus application selector");
        let ids = document
            .select(&selector)
            .filter_map(|element| element.value().attr("data-vacancy-id"))
            .collect::<Vec<_>>();
        let posting_id = posting.pointer("/identifier/value").and_then(Value::as_str);
        if ids.len() != 1 || posting_id != Some(ids[0]) {
            return Err(schema(
                company_id,
                format!("detail {id} has a mismatched application vacancy ID"),
            ));
        }
        return Ok(format!("hireserve:{}", ids[0]));
    }

    let selector = if company_id == "brand-new-day" {
        Selector::parse(r#"[data-endpoint^="/solliciteren/"]"#)
            .expect("static Brand New Day application selector")
    } else {
        Selector::parse("[data-endpoint]").expect("static application selector")
    };
    let endpoints = document
        .select(&selector)
        .filter_map(|element| element.value().attr("data-endpoint"))
        .collect::<Vec<_>>();
    if endpoints.len() != 1 {
        return Err(schema(
            company_id,
            format!("detail {id} must have one application endpoint"),
        ));
    }
    let url = base.join(endpoints[0]).map_err(|error| {
        schema(
            company_id,
            format!("detail {id} has an invalid application endpoint: {error}"),
        )
    })?;
    require_official_origin(base, &url, "application endpoint", company_id)?;
    let expected_path = if company_id == "brand-new-day" {
        format!("/solliciteren/{id}/inline")
    } else {
        format!("/en/solliciteren/{id}/inline")
    };
    if url.path() != expected_path || url.query().is_some() {
        return Err(schema(
            company_id,
            format!("detail {id} application endpoint has a mismatched ID"),
        ));
    }
    Ok(url.to_string())
}

fn canonical_url(document: &Html, base: &Url, company_id: &str) -> Result<Url, SourceError> {
    let selector =
        Selector::parse(r#"link[rel~="canonical"][href]"#).expect("static canonical selector");
    let mut links = document.select(&selector);
    let href = links
        .next()
        .and_then(|link| link.value().attr("href"))
        .ok_or_else(|| schema(company_id, "detail has no canonical URL"))?;
    if links.next().is_some() {
        return Err(schema(company_id, "detail has multiple canonical URLs"));
    }
    let canonical = base
        .join(href)
        .map_err(|error| schema(company_id, format!("invalid canonical URL: {error}")))?;
    require_official_origin(base, &canonical, "canonical", company_id)?;
    Ok(canonical)
}

fn validate_canonical_identity(
    canonical: &Url,
    expected_id: u64,
    company_id: &str,
) -> Result<(), SourceError> {
    let profile = site_profile(company_id)?;
    let segments = canonical
        .path_segments()
        .map(Iterator::collect::<Vec<_>>)
        .unwrap_or_default();
    let id_index = profile.detail_prefix.len();
    if segments.len() != id_index + 2
        || segments[..id_index] != *profile.detail_prefix
        || segments[id_index].parse::<u64>().ok() != Some(expected_id)
        || segments[id_index + 1].is_empty()
        || canonical.query().is_some()
        || canonical.fragment().is_some()
    {
        return Err(schema(
            company_id,
            format!("detail {expected_id} canonical URL has a mismatched identity"),
        ));
    }
    Ok(())
}

fn detail_url(base: &Url, id: u64, slug: &str, company_id: &str) -> Result<Url, SourceError> {
    let profile = site_profile(company_id)?;
    let mut url = base.clone();
    let mut segments = url
        .path_segments_mut()
        .map_err(|()| schema(company_id, "official base URL cannot be a base"))?;
    segments.clear();
    for segment in profile.detail_prefix {
        segments.push(segment);
    }
    segments.push(&id.to_string()).push(slug);
    drop(segments);
    Ok(url)
}

fn apply_url(base: &Url, canonical: &Url, id: u64, company_id: &str) -> Url {
    if company_id == "topicus" {
        let mut url = canonical.clone();
        url.set_fragment(Some("vacancy-application-form"));
        return url;
    }
    let mut url = base.clone();
    url.set_path(&format!("/vacature-solliciteren/{id}"));
    url
}

fn official_base(value: &str, company_id: &str) -> Result<Url, SourceError> {
    let profile = site_profile(company_id)?;
    let url = Url::parse(value)
        .map_err(|error| schema(company_id, format!("invalid base URL: {error}")))?;
    if url.scheme() != "https"
        || url.host_str() != Some(profile.host)
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || !matches!(url.path(), "" | "/")
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(schema(
            company_id,
            format!("base URL must be exactly https://{}", profile.host),
        ));
    }
    Ok(url)
}

fn require_official_origin(
    base: &Url,
    url: &Url,
    kind: &str,
    company_id: &str,
) -> Result<(), SourceError> {
    let profile = site_profile(company_id)?;
    if url.scheme() != "https"
        || url.host_str() != Some(profile.host)
        || url.origin() != base.origin()
    {
        return Err(schema(
            company_id,
            format!("{kind} URL is not on the exact official HTTPS origin"),
        ));
    }
    Ok(())
}

fn required(value: &str, field: &str, id: u64, company_id: &str) -> Result<String, SourceError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(schema(
            company_id,
            format!("listing vacancy {id} has no {field}"),
        ));
    }
    Ok(value.to_owned())
}

fn joined_categories(values: &[Category]) -> Option<String> {
    let mut categories = Vec::new();
    for value in values {
        let title = value.title.trim();
        if !title.is_empty() && !categories.contains(&title) {
            categories.push(title);
        }
    }
    (!categories.is_empty()).then(|| categories.join(" / "))
}

fn schema(company_id: &str, message: impl std::fmt::Display) -> SourceError {
    SourceError::schema(format!("Getnoticed response for {company_id}: {message}"))
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct ListingMeta {
    num_total_hits: usize,
    #[serde(rename = "pageNumber")]
    page_number: usize,
    #[serde(rename = "maxPerPage")]
    max_per_page: usize,
    #[serde(rename = "totalPageCount")]
    total_page_count: usize,
}

#[derive(Debug, Deserialize)]
struct ListingPage {
    meta: ListingMeta,
    vacancies: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct ListingRow {
    id: u64,
    slug: String,
    title: String,
    city: String,
    #[serde(default)]
    subtitle: Subtitle,
}

#[derive(Debug, Default, Deserialize)]
struct Subtitle {
    #[serde(default)]
    option_values: Vec<Category>,
}

#[derive(Debug, Deserialize)]
struct Category {
    title: String,
}

struct Listing {
    id: u64,
    title: String,
    city: String,
    department: Option<String>,
    detail_url: Url,
    raw: Value,
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, future::ready};

    use super::*;

    #[test]
    fn topicus_profile_builds_and_validates_official_job_links() {
        let base = official_base("https://www.werkenbijtopicus.nl", "topicus").unwrap();
        let detail = detail_url(&base, 302, "app-developer-parro", "topicus").unwrap();
        assert_eq!(
            detail.as_str(),
            "https://www.werkenbijtopicus.nl/vacature/302/app-developer-parro"
        );
        validate_canonical_identity(&detail, 302, "topicus").unwrap();
        assert_eq!(
            apply_url(&base, &detail, 302, "topicus").as_str(),
            "https://www.werkenbijtopicus.nl/vacature/302/app-developer-parro#vacancy-application-form"
        );

        let document = Html::parse_document(
            r#"<div id="ub-apply-form" data-hireserve-form-container data-vacancy-id="1327172"></div>"#,
        );
        let posting = serde_json::json!({"identifier": {"value": "1327172"}});
        assert_eq!(
            application_endpoint(&document, &base, 302, &posting, "topicus").unwrap(),
            "hireserve:1327172"
        );
    }

    #[tokio::test]
    async fn corrupt_first_page_does_not_request_page_two() {
        let source = GetnoticedSource::new(
            "abn-amro",
            "https://www.werkenbijabnamro.nl",
            Some("Nederland".to_owned()),
            Client::new(),
        );
        let requests = Cell::new(0);
        let corrupt = serde_json::json!({
            "meta": {
                "num_total_hits": 800_000,
                "pageNumber": 1,
                "maxPerPage": 8,
                "totalPageCount": 100_000
            },
            "vacancies": [{}, {}, {}, {}, {}, {}, {}, {}]
        })
        .to_string();

        let result = source
            .fetch_listing_pages(|request| {
                requests.set(requests.get() + 1);
                assert_eq!(
                    request.build().unwrap().url().query_pairs().next().unwrap(),
                    ("pageNumber".into(), "1".into())
                );
                ready(Ok(corrupt.clone()))
            })
            .await;

        assert_eq!(result.unwrap_err().kind, SourceErrorKind::Schema);
        assert_eq!(requests.get(), 1);
    }
}
