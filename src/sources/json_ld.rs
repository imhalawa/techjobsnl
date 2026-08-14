use regex::Regex;
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
    #[serde(default, deserialize_with = "optional_one_or_many_string")]
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
    let posting = job_posting_value(html, source)?;
    let posting: JobPosting = serde_json::from_value(posting).map_err(|error| {
        SourceError::schema(format!("invalid JobPosting from {source}: {error}"))
    })?;
    if html_text(&posting.description).is_empty() {
        return Err(SourceError::schema(format!(
            "JobPosting from {source} has an empty description"
        )));
    }
    Ok(posting)
}

pub fn job_posting_value(html: &str, source: &str) -> Result<serde_json::Value, SourceError> {
    let document = Html::parse_document(html);
    let selector = Selector::parse(r#"script[type="application/ld+json"]"#)
        .expect("static JSON-LD selector must be valid");

    for script in document.select(&selector) {
        let raw = script.text().collect::<String>();
        let value: serde_json::Value = serde_json::from_str(&raw).map_err(|error| {
            SourceError::schema(format!("invalid JSON-LD from {source}: {error}"))
        })?;
        if let Some(posting) = find_job_posting(&value) {
            return Ok(posting.clone());
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

pub fn html_markdown(html: &str) -> String {
    let mut markdown = html.to_owned();
    markdown = Regex::new(r#"(?is)<\s*a[^>]*href\s*=\s*["']([^"']+)["'][^>]*>(.*?)</\s*a\s*>"#)
        .expect("static HTML link pattern must be valid")
        .replace_all(&markdown, "$2 ($1)")
        .into_owned();
    for (pattern, replacement) in [
        (r"(?is)<\s*br\s*/?\s*>", "\n"),
        (r"(?is)<\s*h[1-6](?:\s[^>]*)?>", "\n## "),
        (r"(?is)</\s*h[1-6]\s*>", "\n\n"),
        (r"(?is)<\s*li(?:\s[^>]*)?>", "\n- "),
        (r"(?is)</\s*li\s*>", ""),
        (r"(?is)</\s*(?:ul|ol)\s*>", "\n\n"),
        (r"(?is)<\s*p(?:\s[^>]*)?>", "\n"),
        (r"(?is)</\s*p\s*>", "\n\n"),
        (r"(?is)<\s*(?:strong|b)(?:\s[^>]*)?>", "**"),
        (r"(?is)</\s*(?:strong|b)\s*>", "**"),
        (r"(?is)<\s*(?:em|i)(?:\s[^>]*)?>", "*"),
        (r"(?is)</\s*(?:em|i)\s*>", "*"),
        (r"(?is)<\s*code(?:\s[^>]*)?>", "`"),
        (r"(?is)</\s*code\s*>", "`"),
    ] {
        markdown = Regex::new(pattern)
            .expect("static HTML pattern must be valid")
            .replace_all(&markdown, replacement)
            .into_owned();
    }

    let text = Html::parse_fragment(&markdown)
        .root_element()
        .text()
        .collect::<String>();
    let mut lines = Vec::new();
    for raw_line in text.lines() {
        let line = raw_line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            if lines.last().is_some_and(|line: &String| !line.is_empty()) {
                lines.push(String::new());
            }
        } else {
            lines.push(line);
        }
    }
    lines.join("\n").trim().to_owned()
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

fn optional_one_or_many_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany {
        One(String),
        Many(Vec<String>),
    }

    let values = match Option::<OneOrMany>::deserialize(deserializer)? {
        Some(OneOrMany::One(value)) => vec![value],
        Some(OneOrMany::Many(values)) => values,
        None => return Ok(None),
    };
    let values = values
        .into_iter()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    Ok((!values.is_empty()).then(|| values.join(" / ")))
}

#[cfg(test)]
mod tests {
    use super::html_markdown;

    #[test]
    fn html_markdown_preserves_basic_document_structure() {
        assert_eq!(
            html_markdown(
                "<h2>Role</h2><p>Build <strong>reliable</strong> systems.</p><ul><li>Ship <code>code</code></li></ul><p><a href=\"https://example.test\">Details</a></p>"
            ),
            "## Role\n\nBuild **reliable** systems.\n\n- Ship `code`\n\nDetails (https://example.test)"
        );
    }
}
