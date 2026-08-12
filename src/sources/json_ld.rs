use scraper::{Html, Selector};
use serde::Deserialize;

use super::SourceError;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobPosting {
    pub identifier: Option<JobIdentifier>,
    pub url: Option<String>,
    pub title: Option<String>,
    pub description: String,
    pub employment_type: Option<String>,
    pub date_posted: Option<String>,
    #[serde(default, deserialize_with = "one_or_many")]
    pub job_location: Vec<JobLocation>,
    pub hiring_organization: Option<HiringOrganization>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct JobIdentifier {
    pub name: Option<String>,
    pub value: String,
}

impl<'de> Deserialize<'de> for JobIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum RawIdentifier {
            Text(String),
            Property { name: Option<String>, value: String },
        }

        Ok(match RawIdentifier::deserialize(deserializer)? {
            RawIdentifier::Text(value) => Self { name: None, value },
            RawIdentifier::Property { name, value } => Self { name, value },
        })
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobLocation {
    pub name: Option<String>,
    pub address: PostalAddress,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostalAddress {
    pub street_address: Option<String>,
    pub address_locality: Option<String>,
    pub address_region: Option<String>,
    pub postal_code: Option<String>,
    pub address_country: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HiringOrganization {
    pub name: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

pub fn parse_job_posting(html: &str, source: &str) -> Result<JobPosting, SourceError> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(r#"script[type="application/ld+json"]"#)
        .expect("static JSON-LD selector must be valid");

    for script in document.select(&selector) {
        let raw = script.text().collect::<String>();
        let value: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
            SourceError::schema(format!("invalid JSON-LD from {source}: {error}"))
        })?;
        if let Some(posting) = find_job_posting(&value) {
            let posting: JobPosting = serde_json::from_value(posting.clone()).map_err(|error| {
                SourceError::schema(format!("invalid JobPosting from {source}: {error}"))
            })?;
            if html_text(&posting.description).is_empty() {
                return Err(SourceError::schema(format!(
                    "JobPosting from {source} has an empty description"
                )));
            }
            return Ok(posting);
        }
    }

    Err(SourceError::schema(format!(
        "no JobPosting JSON-LD found in {source}"
    )))
}

pub fn html_text(html: &str) -> String {
    Html::parse_fragment(html)
        .root_element()
        .text()
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

fn find_job_posting(value: &serde_json::Value) -> Option<&serde_json::Value> {
    match value {
        serde_json::Value::Object(object) => {
            if object.get("@type").is_some_and(is_job_posting_type) {
                return Some(value);
            }
            object.get("@graph").and_then(find_job_posting)
        }
        serde_json::Value::Array(values) => values.iter().find_map(find_job_posting),
        _ => None,
    }
}

fn is_job_posting_type(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::String(value) => value == "JobPosting",
        serde_json::Value::Array(values) => values.iter().any(is_job_posting_type),
        _ => false,
    }
}

fn one_or_many<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany<T> {
        One(T),
        Many(Vec<T>),
    }

    Ok(match OneOrMany::deserialize(deserializer)? {
        OneOrMany::One(value) => vec![value],
        OneOrMany::Many(values) => values,
    })
}
