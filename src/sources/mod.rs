pub mod afas;
pub mod albert_heijn;
pub mod amazon;
pub mod anwb;
pub mod ashby;
pub mod bol;
pub mod buckaroo;
pub mod chipsoft;
pub mod coolblue;
pub mod deel;
pub mod ebay;
pub mod eneco;
pub mod exact;
pub mod getnoticed;
pub mod google;
pub mod greenhouse;
pub mod http;
pub mod ing;
pub mod jibe;
pub mod json_ld;
pub mod lever;
pub mod microsoft;
pub mod ns;
pub mod pay;
pub mod personio;
pub mod postnl;
pub mod rabobank;
pub mod recruitee;
pub mod successfactors;
pub mod successfactors_api;
pub mod uber;
pub mod workable;
pub mod workday;
pub mod yuki;

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

fn country_code_for_location(location: &str) -> Option<&'static str> {
    let location = location.trim();
    for (name, code) in [
        ("The Netherlands", "NL"),
        ("Netherlands", "NL"),
        ("United Kingdom", "GB"),
        ("United States", "US"),
        ("Czech Republic", "CZ"),
        ("Korea, South", "KR"),
        ("Australia", "AU"),
        ("Brazil", "BR"),
        ("Bulgaria", "BG"),
        ("China", "CN"),
        ("France", "FR"),
        ("Germany", "DE"),
        ("Hong Kong", "HK"),
        ("India", "IN"),
        ("Indonesia", "ID"),
        ("Israel", "IL"),
        ("Italy", "IT"),
        ("Japan", "JP"),
        ("Latvia", "LV"),
        ("Malaysia", "MY"),
        ("Mexico", "MX"),
        ("Portugal", "PT"),
        ("Singapore", "SG"),
        ("Spain", "ES"),
        ("Sweden", "SE"),
        ("Thailand", "TH"),
        ("Vietnam", "VN"),
    ] {
        if location == name || location.ends_with(&format!(", {name}")) {
            return Some(code);
        }
    }

    match location {
        "Amsterdam" => Some("NL"),
        "AUSTRALIA" => Some("AU"),
        "Austin" | "Remote, USA" => Some("US"),
        "Copenhagen" => Some("DK"),
        "Bengaluru" | "Mumbai" => Some("IN"),
        "Berlin" | "Munich" => Some("DE"),
        "Chicago" | "New York" | "San Francisco" | "Washington D.C., District of Columbia" => {
            Some("US")
        }
        "Hong Kong" => Some("HK"),
        "Kuala Lumpur" => Some("MY"),
        "London" => Some("GB"),
        "Madrid" => Some("ES"),
        "Mexico City" => Some("MX"),
        "Milan" => Some("IT"),
        "Paris" => Some("FR"),
        "Prague" => Some("CZ"),
        "Sao Jose dos Campos" | "Sao Paulo" => Some("BR"),
        "Shanghai" => Some("CN"),
        "Singapore" => Some("SG"),
        "Stockholm" => Some("SE"),
        "Sydney" => Some("AU"),
        "Tokyo" => Some("JP"),
        _ => None,
    }
}
