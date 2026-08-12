use std::collections::HashSet;

use chrono::{DateTime, Utc};
use reqwest::{Client, Url};
use serde::Deserialize;
use serde_json::Value;

use crate::domain::{ObservedJob, SourceScan};

use super::{JobSource, SourceError, http::send_text};

const PAGE_SIZE: usize = 10;
const DESCRIPTION_FIELDS: [&str; 5] = [
    "howDoYouMakeOurCustomerSmile",
    "theBiggestChallenge",
    "whatYouWillDoAs",
    "whyYouCanMakeADifference",
    "whereYoullBeWorking",
];

pub struct BolSource {
    company_id: String,
    base_url: String,
    client: Client,
}

impl BolSource {
    pub fn new(company_id: impl Into<String>, base_url: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            base_url: base_url.into(),
            client,
        }
    }
}

#[async_trait::async_trait]
impl JobSource for BolSource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let endpoint = endpoint_url(&self.base_url, &self.company_id)?;
        let mut collection = BolCollection::new(&self.company_id, &self.base_url, Some(PAGE_SIZE));
        for page in 1.. {
            let raw = send_text(
                self.client.post(endpoint.clone()).json(&serde_json::json!({
                    "page": page,
                    "language": "nl_NL",
                })),
                "bol",
            )
            .await?;
            if collection.add_page(&raw)? {
                return Ok(SourceScan::Complete {
                    observations: collection.finish()?,
                });
            }
        }
        unreachable!("bol pagination returns after reaching the declared total")
    }
}

pub fn parse_bol_pages(
    company_id: &str,
    base_url: &str,
    pages: &[&str],
) -> Result<Vec<ObservedJob>, SourceError> {
    let mut collection = BolCollection::new(company_id, base_url, None);
    for page in pages {
        collection.add_page(page)?;
    }
    collection.finish()
}

struct BolCollection<'a> {
    company_id: &'a str,
    base_url: &'a str,
    expected_total: Option<BolTotal>,
    page_size: Option<usize>,
    ids: HashSet<String>,
    observations: Vec<ObservedJob>,
    complete: bool,
}

impl<'a> BolCollection<'a> {
    fn new(company_id: &'a str, base_url: &'a str, page_size: Option<usize>) -> Self {
        Self {
            company_id,
            base_url,
            expected_total: None,
            page_size,
            ids: HashSet::new(),
            observations: Vec::new(),
            complete: false,
        }
    }

    fn add_page(&mut self, raw: &str) -> Result<bool, SourceError> {
        if self.complete {
            return Err(SourceError::schema(format!(
                "bol returned a page after reaching the declared total for {}",
                self.company_id
            )));
        }
        let page: BolPage = serde_json::from_str(raw).map_err(|error| {
            SourceError::schema(format!(
                "invalid bol response for {}: {error}",
                self.company_id
            ))
        })?;
        let hits = page.hits.ok_or_else(|| {
            SourceError::schema(format!(
                "bol response for {} is missing hits",
                self.company_id
            ))
        })?;
        if hits.total.relation != "eq" {
            return Err(SourceError::schema(format!(
                "bol total relation for {} is not exact",
                self.company_id
            )));
        }
        if let Some(expected) = &self.expected_total {
            if expected != &hits.total {
                return Err(SourceError::schema(format!(
                    "bol pagination metadata changed for {}",
                    self.company_id
                )));
            }
        } else {
            self.expected_total = Some(hits.total.clone());
        }
        let rows = hits.rows.ok_or_else(|| {
            SourceError::schema(format!(
                "bol response for {} is missing hit rows",
                self.company_id
            ))
        })?;
        let expected_total = hits.total.value;
        let prior_count = self.observations.len();
        if rows.is_empty() && prior_count < expected_total {
            return Err(SourceError::schema(format!(
                "bol returned an empty page before reaching {expected_total} jobs for {}",
                self.company_id
            )));
        }
        let expected_page_size = *self.page_size.get_or_insert(rows.len());
        if rows.len() > expected_page_size
            || (rows.len() < expected_page_size && prior_count + rows.len() < expected_total)
        {
            return Err(SourceError::schema(format!(
                "bol returned a short or oversized page before reaching {expected_total} jobs for {}",
                self.company_id
            )));
        }

        for raw_hit in rows {
            let hit: BolHit = serde_json::from_value(raw_hit).map_err(|error| {
                SourceError::schema(format!("invalid bol hit for {}: {error}", self.company_id))
            })?;
            if !self.ids.insert(hit.id.clone()) {
                return Err(SourceError::schema(format!(
                    "duplicate bol job {} for {}",
                    hit.id, self.company_id
                )));
            }
            self.observations.push(observed_job(
                self.company_id,
                self.base_url,
                hit.id,
                hit.source,
            )?);
            if self.observations.len() > expected_total {
                return Err(SourceError::schema(format!(
                    "bol returned more than {expected_total} jobs for {}",
                    self.company_id
                )));
            }
        }
        self.complete = self.observations.len() == expected_total;
        Ok(self.complete)
    }

    fn finish(self) -> Result<Vec<ObservedJob>, SourceError> {
        let expected = self.expected_total.ok_or_else(|| {
            SourceError::schema(format!("bol returned no pages for {}", self.company_id))
        })?;
        if self.observations.len() != expected.value {
            return Err(SourceError::schema(format!(
                "bol returned {} of {} jobs for {}",
                self.observations.len(),
                expected.value,
                self.company_id
            )));
        }
        Ok(self.observations)
    }
}

fn observed_job(
    company_id: &str,
    base_url: &str,
    source_id: String,
    raw_payload: Value,
) -> Result<ObservedJob, SourceError> {
    let source = raw_payload.as_object().ok_or_else(|| {
        SourceError::schema(format!(
            "bol job {source_id} for {company_id} has no source object"
        ))
    })?;
    let public_id = scalar_string(source.get("id"), "id", &source_id, company_id)?;
    let title = required_string(source.get("title"), "title", &source_id, company_id)?;
    if required_string(source.get("status"), "status", &source_id, company_id)? != "PUBLISHED"
        || source.get("internal").and_then(Value::as_bool) != Some(false)
    {
        return Err(SourceError::schema(format!(
            "bol job {source_id} for {company_id} is not public"
        )));
    }
    let office = source
        .get("office")
        .and_then(Value::as_object)
        .and_then(|office| office.get("label"));
    let location = required_string(office, "office label", &source_id, company_id)?;
    let country = match location.as_str() {
        "Utrecht" | "Nieuwegein" => "NL",
        _ => {
            return Err(SourceError::schema(format!(
                "bol job {source_id} for {company_id} has unresolved office {location:?}"
            )));
        }
    };
    let publication_ms = source
        .get("publicationDate")
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            SourceError::schema(format!(
                "bol job {source_id} for {company_id} has no publication timestamp"
            ))
        })?;
    let published_at = DateTime::<Utc>::from_timestamp_millis(publication_ms).ok_or_else(|| {
        SourceError::schema(format!(
            "bol job {source_id} for {company_id} has an invalid publication timestamp"
        ))
    })?;
    let description = description(source, &source_id, company_id)?;
    let job_url = detail_url(base_url, &title, &public_id, company_id)?;

    Ok(ObservedJob {
        source_id,
        title,
        department: nested_label(source.get("jobFamily")),
        team: nested_label(source.get("expertise")),
        employment_type: optional_string(source.get("employmentType")),
        locations: vec![location],
        countries: vec![country.to_owned()],
        apply_url: format!("{job_url}#form"),
        job_url,
        description,
        raw_payload,
        published_at: Some(published_at),
    })
}

fn description(
    source: &serde_json::Map<String, Value>,
    source_id: &str,
    company_id: &str,
) -> Result<String, SourceError> {
    let mut blocks = Vec::new();
    for field in DESCRIPTION_FIELDS {
        let Some(section) = source.get(field) else {
            continue;
        };
        if section.is_null() {
            continue;
        }
        let content = section
            .as_object()
            .and_then(|section| section.get("content"))
            .and_then(Value::as_array)
            .ok_or_else(|| malformed_text(source_id, company_id, field))?;
        for block in content {
            let block = block
                .as_object()
                .filter(|block| block.get("_type").and_then(Value::as_str) == Some("block"))
                .ok_or_else(|| malformed_text(source_id, company_id, field))?;
            let children = block
                .get("children")
                .and_then(Value::as_array)
                .ok_or_else(|| malformed_text(source_id, company_id, field))?;
            let mut text = String::new();
            for span in children {
                let span = span
                    .as_object()
                    .filter(|span| span.get("_type").and_then(Value::as_str) == Some("span"))
                    .ok_or_else(|| malformed_text(source_id, company_id, field))?;
                text.push_str(
                    span.get("text")
                        .and_then(Value::as_str)
                        .ok_or_else(|| malformed_text(source_id, company_id, field))?,
                );
            }
            let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
            if !normalized.is_empty() {
                blocks.push(normalized);
            }
        }
    }
    if blocks.is_empty() {
        return Err(SourceError::schema(format!(
            "bol job {source_id} for {company_id} has an empty description"
        )));
    }
    Ok(blocks.join("\n\n"))
}

fn malformed_text(source_id: &str, company_id: &str, field: &str) -> SourceError {
    SourceError::schema(format!(
        "bol job {source_id} for {company_id} has malformed rich text in {field}"
    ))
}

fn endpoint_url(base_url: &str, company_id: &str) -> Result<Url, SourceError> {
    let mut url = Url::parse(base_url).map_err(|error| {
        SourceError::schema(format!("invalid bol base URL for {company_id}: {error}"))
    })?;
    url.set_path("/api/v1/jobs/");
    url.set_query(None);
    Ok(url)
}

fn detail_url(
    base_url: &str,
    title: &str,
    public_id: &str,
    company_id: &str,
) -> Result<String, SourceError> {
    let mut url = Url::parse(base_url).map_err(|error| {
        SourceError::schema(format!("invalid bol base URL for {company_id}: {error}"))
    })?;
    url.set_path("");
    url.set_query(None);
    url.path_segments_mut()
        .map_err(|()| {
            SourceError::schema(format!("bol base URL for {company_id} cannot be a base"))
        })?
        .extend(["nl", "vacatures", &slug(title), public_id, ""]);
    Ok(url.to_string())
}

fn slug(title: &str) -> String {
    let mut slug = String::new();
    let mut separator = false;
    for character in title.chars().flat_map(char::to_lowercase) {
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

fn required_string(
    value: Option<&Value>,
    field: &str,
    source_id: &str,
    company_id: &str,
) -> Result<String, SourceError> {
    let value = value
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or_default();
    if value.is_empty() {
        return Err(SourceError::schema(format!(
            "bol job {source_id} for {company_id} has an empty {field}"
        )));
    }
    Ok(value.to_owned())
}

fn scalar_string(
    value: Option<&Value>,
    field: &str,
    source_id: &str,
    company_id: &str,
) -> Result<String, SourceError> {
    match value {
        Some(Value::String(value)) if !value.trim().is_empty() => Ok(value.trim().to_owned()),
        Some(Value::Number(value)) => Ok(value.to_string()),
        _ => Err(SourceError::schema(format!(
            "bol job {source_id} for {company_id} has an invalid {field}"
        ))),
    }
}

fn nested_label(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_object)
        .and_then(|value| value.get("label"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn optional_string(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq)]
struct BolTotal {
    value: usize,
    relation: String,
}

#[derive(Deserialize)]
struct BolPage {
    hits: Option<BolHits>,
}

#[derive(Deserialize)]
struct BolHits {
    total: BolTotal,
    #[serde(rename = "hits")]
    rows: Option<Vec<Value>>,
}

#[derive(Deserialize)]
struct BolHit {
    #[serde(rename = "_id")]
    id: String,
    #[serde(rename = "_source")]
    source: Value,
}
