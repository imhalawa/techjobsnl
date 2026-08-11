use std::{collections::HashSet, time::Duration};

use chrono::{DateTime, Utc};
use reqwest::{Client, StatusCode, Url, header::HeaderValue, redirect::Policy};
use serde::{Deserialize, Serialize};

use crate::domain::{ObservedJob, SourceErrorKind, SourceScan};

use super::{JobSource, SourceError};

const ASHBY_BOARD_ENDPOINT: &str = "https://api.ashbyhq.com/posting-api/job-board";
const REDIRECT_LIMIT: usize = 5;

pub struct AshbySource {
    company_id: String,
    board: String,
    client: Client,
}

impl AshbySource {
    pub fn new(company_id: impl Into<String>, board: impl Into<String>, client: Client) -> Self {
        Self {
            company_id: company_id.into(),
            board: board.into(),
            client,
        }
    }
}

pub fn build_client(user_agent: &str, timeout: Duration) -> Result<Client, SourceError> {
    Client::builder()
        .user_agent(user_agent)
        .timeout(timeout)
        .cookie_store(true)
        .redirect(Policy::limited(REDIRECT_LIMIT))
        .build()
        .map_err(|error| SourceError {
            kind: SourceErrorKind::Configuration,
            message: format!("could not configure HTTP client: {error}"),
            http_status: None,
            retry_after: None,
            retryable: false,
        })
}

#[async_trait::async_trait]
impl JobSource for AshbySource {
    fn company_id(&self) -> &str {
        &self.company_id
    }

    async fn scan(&self) -> Result<SourceScan, SourceError> {
        let response = self
            .client
            .get(board_url(&self.board))
            .send()
            .await
            .map_err(transport_error)?;
        let status = response.status();
        let retry_after = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(parse_retry_after);

        if status == StatusCode::TOO_MANY_REQUESTS {
            return Err(SourceError {
                kind: SourceErrorKind::RateLimit,
                message: "Ashby rate limit exceeded".to_owned(),
                http_status: Some(status.as_u16()),
                retry_after,
                retryable: true,
            });
        }
        if !status.is_success() {
            return Err(SourceError {
                kind: if status == StatusCode::REQUEST_TIMEOUT {
                    SourceErrorKind::Timeout
                } else {
                    SourceErrorKind::Transport
                },
                message: format!("Ashby returned HTTP {status}"),
                http_status: Some(status.as_u16()),
                retry_after,
                retryable: retryable_status(status),
            });
        }

        let raw_json = response.text().await.map_err(transport_error)?;
        let observations = parse_ashby_response(&self.company_id, &raw_json)?;
        Ok(SourceScan::Complete { observations })
    }
}

fn transport_error(error: reqwest::Error) -> SourceError {
    let retryable = error.is_connect()
        || error.is_timeout()
        || error.is_body()
        || (error.is_request() && !error.is_builder() && !error.is_redirect());
    SourceError {
        kind: if error.is_timeout() {
            SourceErrorKind::Timeout
        } else {
            SourceErrorKind::Transport
        },
        message: error.to_string(),
        http_status: error.status().map(|status| status.as_u16()),
        retry_after: None,
        retryable,
    }
}

fn board_url(board: &str) -> Url {
    let mut url = Url::parse(ASHBY_BOARD_ENDPOINT).expect("Ashby endpoint constant must be valid");
    url.path_segments_mut()
        .expect("Ashby endpoint must be a base URL")
        .push(board);
    url
}

fn parse_retry_after(value: &HeaderValue) -> Option<Duration> {
    parse_retry_after_at(value, Utc::now())
}

fn parse_retry_after_at(value: &HeaderValue, now: DateTime<Utc>) -> Option<Duration> {
    let value = value.to_str().ok()?;
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    let retry_at = DateTime::parse_from_rfc2822(value)
        .ok()?
        .with_timezone(&Utc);
    Some(
        retry_at
            .signed_duration_since(now)
            .to_std()
            .unwrap_or_default(),
    )
}

fn retryable_status(status: StatusCode) -> bool {
    status == StatusCode::REQUEST_TIMEOUT || status.is_server_error()
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AshbyResponse {
    jobs: Option<Vec<AshbyJob>>,
    api_version: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AshbyJob {
    id: String,
    title: String,
    department: Option<String>,
    team: Option<String>,
    employment_type: Option<String>,
    location: String,
    secondary_locations: Vec<AshbyLocation>,
    published_at: Option<String>,
    is_listed: bool,
    address: AshbyAddress,
    job_url: String,
    apply_url: String,
    description_plain: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AshbyLocation {
    location: String,
    address: AshbyAddress,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AshbyAddress {
    #[serde(rename = "postalAddress")]
    postal_address: AshbyPostalAddress,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AshbyPostalAddress {
    address_country: String,
    #[serde(flatten)]
    extra: serde_json::Map<String, serde_json::Value>,
}

pub fn parse_ashby_response(
    company_id: &str,
    raw_json: &str,
) -> Result<Vec<ObservedJob>, SourceError> {
    let response: AshbyResponse = serde_json::from_str(raw_json).map_err(|error| {
        SourceError::schema(format!("invalid Ashby response for {company_id}: {error}"))
    })?;
    if response.api_version != "1" {
        return Err(SourceError::schema(format!(
            "unsupported Ashby API version {:?} for {company_id}",
            response.api_version
        )));
    }
    let jobs = response.jobs.ok_or_else(|| {
        SourceError::schema(format!("Ashby response for {company_id} is missing jobs"))
    })?;

    jobs.into_iter()
        .filter(|job| job.is_listed)
        .map(|job| observed_job(company_id, job))
        .collect()
}

fn observed_job(company_id: &str, job: AshbyJob) -> Result<ObservedJob, SourceError> {
    if job.id.trim().is_empty() {
        return Err(SourceError::schema(format!(
            "Ashby job for {company_id} has an empty id"
        )));
    }
    if job.job_url.trim().is_empty() {
        return Err(SourceError::schema(format!(
            "Ashby job {} for {company_id} has an empty official URL",
            job.id
        )));
    }
    if job.apply_url.trim().is_empty() {
        return Err(SourceError::schema(format!(
            "Ashby job {} for {company_id} has an empty apply URL",
            job.id
        )));
    }
    let published_at = job
        .published_at
        .as_deref()
        .map(DateTime::parse_from_rfc3339)
        .transpose()
        .map_err(|error| {
            SourceError::schema(format!(
                "Ashby job {} for {company_id} has an invalid publication time: {error}",
                job.id
            ))
        })?
        .map(|published_at| published_at.with_timezone(&Utc));

    let mut locations = Vec::with_capacity(1 + job.secondary_locations.len());
    let mut countries = Vec::with_capacity(1 + job.secondary_locations.len());
    let mut seen_locations = HashSet::new();
    let mut seen_countries = HashSet::new();
    push_unique(&mut locations, &mut seen_locations, job.location.clone());
    push_unique(
        &mut countries,
        &mut seen_countries,
        normalise_country(&job.address.postal_address.address_country),
    );
    for secondary in &job.secondary_locations {
        push_unique(
            &mut locations,
            &mut seen_locations,
            secondary.location.clone(),
        );
        push_unique(
            &mut countries,
            &mut seen_countries,
            normalise_country(&secondary.address.postal_address.address_country),
        );
    }

    let raw_payload = serde_json::to_value(&job).map_err(|error| {
        SourceError::schema(format!(
            "could not preserve Ashby job {} for {company_id}: {error}",
            job.id
        ))
    })?;

    Ok(ObservedJob {
        source_id: job.id,
        title: job.title,
        department: job.department,
        team: job.team,
        employment_type: job.employment_type,
        locations,
        countries,
        job_url: job.job_url,
        apply_url: job.apply_url,
        description: job.description_plain,
        raw_payload,
        published_at,
    })
}

fn normalise_country(country: &str) -> String {
    match country {
        "Netherlands" => "NL".to_owned(),
        country => country.to_owned(),
    }
}

fn push_unique(values: &mut Vec<String>, seen: &mut HashSet<String>, value: String) {
    if seen.insert(value.clone()) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use chrono::{TimeZone, Utc};
    use reqwest::{StatusCode, header::HeaderValue};

    use super::{board_url, parse_retry_after_at, retryable_status};

    #[test]
    fn encodes_board_as_one_url_path_segment() {
        let url = board_url("mollie/preview?region=nl#jobs");

        assert_eq!(
            url.as_str(),
            "https://api.ashbyhq.com/posting-api/job-board/mollie%2Fpreview%3Fregion=nl%23jobs"
        );
    }

    #[test]
    fn parses_retry_after_seconds_and_http_dates() {
        let now = Utc.with_ymd_and_hms(2015, 10, 21, 7, 27, 0).unwrap();

        assert_eq!(
            parse_retry_after_at(&HeaderValue::from_static("45"), now),
            Some(Duration::from_secs(45))
        );
        assert_eq!(
            parse_retry_after_at(
                &HeaderValue::from_static("Wed, 21 Oct 2015 07:28:00 GMT"),
                now,
            ),
            Some(Duration::from_secs(60))
        );
    }

    #[test]
    fn retries_request_timeout_and_server_statuses_only() {
        assert!(retryable_status(StatusCode::REQUEST_TIMEOUT));
        assert!(retryable_status(StatusCode::INTERNAL_SERVER_ERROR));
        assert!(!retryable_status(StatusCode::UNAUTHORIZED));
        assert!(!retryable_status(StatusCode::NOT_FOUND));
    }
}
