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
    Timeout,
    RateLimit,
    Schema,
    Browser,
    Storage,
}

impl SourceErrorKind {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Configuration => "configuration",
            Self::Transport => "transport",
            Self::Timeout => "timeout",
            Self::RateLimit => "rate-limit",
            Self::Schema => "schema",
            Self::Browser => "browser",
            Self::Storage => "storage",
        }
    }
}

impl fmt::Display for SourceErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct ScanFailure {
    pub kind: SourceErrorKind,
    pub diagnostic: String,
}

#[derive(Debug, Clone)]
pub enum ScanEvent {
    RunStarted {
        run_id: String,
        company_count: usize,
    },
    CompanyStarted {
        company_id: String,
    },
    CompanyCompleted {
        company_id: String,
        observed_count: usize,
        eligible_count: usize,
    },
    CompanyFailed {
        company_id: String,
        kind: SourceErrorKind,
        diagnostic: String,
    },
    CompanyIncomplete {
        company_id: String,
        diagnostic: String,
        observed_count: usize,
    },
    RunFinished {
        run_id: String,
        completed: usize,
        failed: usize,
        incomplete: usize,
    },
    // Retained for compatibility with the source-level event contract.
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
