mod job;
mod scan;

pub use job::{ClassifiedJob, Eligibility, JobKey, JobRecord, ObservedJob};
pub use scan::{ScanEvent, ScanFailure, SourceErrorKind, SourceScan};
