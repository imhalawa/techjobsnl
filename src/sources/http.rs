use std::time::Duration;

use chrono::{DateTime, Utc};
use reqwest::{RequestBuilder, StatusCode, header::HeaderValue};

use crate::domain::SourceErrorKind;

use super::SourceError;

pub async fn send_text(request: RequestBuilder, source: &str) -> Result<String, SourceError> {
    let response = request.send().await.map_err(transport_error)?;
    let status = response.status();
    let retry_after = response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(parse_retry_after);

    if !status.is_success() {
        return Err(SourceError {
            kind: match status {
                StatusCode::TOO_MANY_REQUESTS => SourceErrorKind::RateLimit,
                StatusCode::REQUEST_TIMEOUT => SourceErrorKind::Timeout,
                _ => SourceErrorKind::Transport,
            },
            message: if status == StatusCode::TOO_MANY_REQUESTS {
                format!("{source} rate limit exceeded")
            } else {
                format!("{source} returned HTTP {status}")
            },
            http_status: Some(status.as_u16()),
            retry_after,
            retryable: status == StatusCode::TOO_MANY_REQUESTS
                || status == StatusCode::REQUEST_TIMEOUT
                || status.is_server_error(),
        });
    }

    response.text().await.map_err(transport_error)
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

fn parse_retry_after(value: &HeaderValue) -> Option<Duration> {
    let value = value.to_str().ok()?;
    if let Ok(seconds) = value.parse::<u64>() {
        return Some(Duration::from_secs(seconds));
    }

    let retry_at = DateTime::parse_from_rfc2822(value)
        .ok()?
        .with_timezone(&Utc);
    Some(
        retry_at
            .signed_duration_since(Utc::now())
            .to_std()
            .unwrap_or_default(),
    )
}
