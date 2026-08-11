pub mod ashby;

use std::time::Duration;

use crate::domain::{SourceErrorKind, SourceScan};

#[async_trait::async_trait]
pub trait JobSource: Send + Sync {
    fn company_id(&self) -> &str;
    async fn scan(&self) -> Result<SourceScan, SourceError>;
}

#[derive(Debug, thiserror::Error)]
#[error("{kind}: {message}")]
pub struct SourceError {
    pub kind: SourceErrorKind,
    pub message: String,
    pub http_status: Option<u16>,
    pub retry_after: Option<Duration>,
    pub retryable: bool,
}

impl SourceError {
    pub(crate) fn schema(message: impl Into<String>) -> Self {
        Self {
            kind: SourceErrorKind::Schema,
            message: message.into(),
            http_status: None,
            retry_after: None,
            retryable: false,
        }
    }
}
