use super::ObservedJob;

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
