use super::ObservedJob;
use std::fmt;

#[derive(Debug, Clone)]
pub enum SourceScan {
    Complete {
        observations: Vec<ObservedJob>,
    },
    Incomplete {
        observations: Vec<ObservedJob>,
        diagnostic: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceErrorKind {
    Configuration,
    Transport,
    RateLimit,
    Schema,
    Browser,
}

impl fmt::Display for SourceErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Configuration => "configuration",
            Self::Transport => "transport",
            Self::RateLimit => "rate-limit",
            Self::Schema => "schema",
            Self::Browser => "browser",
        };
        formatter.write_str(name)
    }
}

#[derive(Debug, Clone)]
pub struct ScanFailure {
    pub kind: SourceErrorKind,
    pub diagnostic: String,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    Started {
        company_id: String,
    },
    Completed {
        company_id: String,
        source_scan: SourceScan,
    },
    Failed {
        company_id: String,
        kind: SourceErrorKind,
        diagnostic: String,
    },
}
