use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{DateTime, Utc};
use futures_util::{StreamExt, stream};
use tokio::{
    sync::mpsc::UnboundedSender,
    time::{sleep, timeout},
};

use crate::{
    config::{CompanyConfig, ScanConfig},
    domain::{ClassifiedJob, ScanEvent, ScanFailure, SourceErrorKind, SourceScan},
    filter::EligibilityFilter,
    sources::{JobSource, SourceError},
    storage::Store,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RunSummary {
    pub completed: usize,
    pub failed: usize,
    pub incomplete: usize,
}

pub struct ScanService {
    sources: Vec<Arc<dyn JobSource>>,
    filter: EligibilityFilter,
    companies: HashMap<String, CompanyConfig>,
    store: Arc<Mutex<Store>>,
    scan_config: ScanConfig,
}

impl ScanService {
    pub fn new(
        sources: Vec<Arc<dyn JobSource>>,
        filter: EligibilityFilter,
        companies: Vec<CompanyConfig>,
        store: Arc<Mutex<Store>>,
        scan_config: ScanConfig,
    ) -> Self {
        Self {
            sources,
            filter,
            companies: companies
                .into_iter()
                .map(|company| (company.id.clone(), company))
                .collect(),
            store,
            scan_config,
        }
    }

    pub async fn run(
        &self,
        run_id: impl Into<String>,
        event_tx: UnboundedSender<ScanEvent>,
    ) -> RunSummary {
        let run_id = run_id.into();
        let scheduled_sources = self
            .sources
            .iter()
            .filter(|source| {
                self.companies
                    .get(source.company_id())
                    .is_some_and(|company| company.enabled)
            })
            .cloned()
            .collect::<Vec<_>>();
        let _ = event_tx.send(ScanEvent::RunStarted {
            run_id: run_id.clone(),
            company_count: scheduled_sources.len(),
        });

        let filter = Arc::new(self.filter.clone());
        let timeout_seconds = self.scan_config.timeout_seconds;
        let retry_count = self.scan_config.retry_count;
        let mut company_scans = Vec::with_capacity(scheduled_sources.len());
        for source in scheduled_sources {
            let company = self.companies.get(source.company_id()).cloned();
            let filter = Arc::clone(&filter);
            let event_tx = event_tx.clone();
            company_scans.push(scan_one(
                source,
                company,
                filter,
                timeout_seconds,
                retry_count,
                event_tx,
            ));
        }
        let mut company_scans =
            stream::iter(company_scans).buffer_unordered(self.scan_config.concurrency);
        let mut summary = RunSummary::default();

        while let Some(company_scan) = company_scans.next().await {
            let company_id = company_scan.company_id.clone();
            let outcome = self.persist_outcome(&run_id, company_scan);
            match outcome {
                CompanyOutcome::Complete {
                    observed_count,
                    eligible_count,
                    ..
                } => {
                    summary.completed += 1;
                    let _ = event_tx.send(ScanEvent::CompanyCompleted {
                        company_id,
                        observed_count,
                        eligible_count,
                    });
                }
                CompanyOutcome::Incomplete {
                    diagnostic,
                    observed_count,
                    ..
                } => {
                    summary.incomplete += 1;
                    let _ = event_tx.send(ScanEvent::CompanyIncomplete {
                        company_id,
                        diagnostic,
                        observed_count,
                    });
                }
                CompanyOutcome::Failed { failure, .. } => {
                    summary.failed += 1;
                    let _ = event_tx.send(ScanEvent::CompanyFailed {
                        company_id,
                        kind: failure.kind,
                        diagnostic: failure.diagnostic,
                    });
                }
            }
        }

        let _ = event_tx.send(ScanEvent::RunFinished {
            run_id,
            completed: summary.completed,
            failed: summary.failed,
            incomplete: summary.incomplete,
        });
        summary
    }

    fn persist_outcome(&self, run_id: &str, scan: CompanyScan) -> CompanyOutcome {
        let Some(company) = scan.company else {
            return scan.outcome;
        };

        let mut store = match self.store.lock() {
            Ok(store) => store,
            Err(error) => {
                return storage_failure(format!("could not lock job store: {error}"));
            }
        };
        let storage_result = match &scan.outcome {
            CompanyOutcome::Complete { jobs, .. } => store.record_complete_scan(
                run_id,
                &company,
                jobs,
                scan.window.started_at,
                scan.window.completed_at,
            ),
            CompanyOutcome::Incomplete {
                diagnostic,
                observed_count,
            } => store.record_incomplete_scan(
                run_id,
                &company,
                diagnostic,
                *observed_count,
                scan.window.started_at,
                scan.window.completed_at,
            ),
            CompanyOutcome::Failed { failure } => store.record_failed_scan(
                run_id,
                &company,
                failure,
                scan.window.started_at,
                scan.window.completed_at,
            ),
        };
        drop(store);

        match storage_result {
            Ok(()) => scan.outcome,
            Err(error) => storage_failure(format!(
                "could not record scan for {}: {error}",
                scan.company_id
            )),
        }
    }
}

async fn scan_one(
    source: Arc<dyn JobSource>,
    company: Option<CompanyConfig>,
    filter: Arc<EligibilityFilter>,
    timeout_seconds: u64,
    retry_count: u32,
    event_tx: UnboundedSender<ScanEvent>,
) -> CompanyScan {
    let company_id = source.company_id().to_owned();
    let started_at = Utc::now();
    let _ = event_tx.send(ScanEvent::CompanyStarted {
        company_id: company_id.clone(),
    });
    let Some(company_config) = company.as_ref() else {
        return CompanyScan {
            company_id: company_id.clone(),
            company,
            window: ScanWindow {
                started_at,
                completed_at: Utc::now(),
            },
            outcome: CompanyOutcome::Failed {
                failure: ScanFailure {
                    kind: SourceErrorKind::Configuration,
                    diagnostic: format!(
                        "source company `{company_id}` has no matching company configuration"
                    ),
                },
            },
        };
    };

    let source_scan = scan_with_retry(source.as_ref(), timeout_seconds, retry_count).await;
    let completed_at = Utc::now();
    let outcome = match source_scan {
        Err(error) => CompanyOutcome::Failed {
            failure: source_failure(error),
        },
        Ok(source_scan) => classify_scan(source_scan, company_config, &filter),
    };

    CompanyScan {
        company_id,
        company,
        window: ScanWindow {
            started_at,
            completed_at,
        },
        outcome,
    }
}

async fn scan_with_retry(
    source: &dyn JobSource,
    timeout_seconds: u64,
    retry_count: u32,
) -> Result<SourceScan, SourceError> {
    let mut retries_used = 0;
    loop {
        let result = timeout(Duration::from_secs(timeout_seconds), source.scan())
            .await
            .unwrap_or_else(|_| {
                Err(SourceError {
                    kind: SourceErrorKind::Timeout,
                    message: format!("scan attempt timed out after {timeout_seconds} seconds"),
                    http_status: None,
                    retry_after: None,
                    retryable: true,
                })
            });
        match result {
            Err(error) if retries_used < retry_count && should_retry(&error) => {
                let delay = error
                    .retry_after
                    .unwrap_or_else(|| retry_delay(source.company_id(), retries_used));
                sleep(delay).await;
                retries_used += 1;
            }
            result => return result,
        }
    }
}

fn should_retry(error: &SourceError) -> bool {
    if !error.retryable {
        return false;
    }

    matches!(
        (error.kind, error.http_status),
        (_, Some(429))
            | (SourceErrorKind::Timeout, _)
            | (SourceErrorKind::Transport, Some(500..=599))
    )
}

fn retry_delay(company_id: &str, retries_used: u32) -> Duration {
    let base_ms = if retries_used == 0 { 250 } else { 500 };
    let seed = company_id
        .bytes()
        .fold(retries_used as u64 + 1, |seed, byte| {
            seed.wrapping_mul(31).wrapping_add(u64::from(byte))
        });
    Duration::from_millis(base_ms + seed % 26)
}

fn classify_scan(
    source_scan: SourceScan,
    company: &CompanyConfig,
    filter: &EligibilityFilter,
) -> CompanyOutcome {
    let (observations, source_diagnostic) = match source_scan {
        SourceScan::Complete { observations } => (observations, None),
        SourceScan::Incomplete {
            observations,
            diagnostic,
        } => (observations, Some(diagnostic)),
    };
    let observed_count = observations.len();
    let mut jobs = Vec::with_capacity(observed_count);
    let mut diagnostics = Vec::new();

    for observed in observations {
        match filter.classify(&observed, &company.location_country_overrides) {
            Ok(eligibility) => jobs.push(ClassifiedJob {
                observed,
                eligibility,
            }),
            Err(error) => diagnostics.push(error.to_string()),
        }
    }

    if let Some(diagnostic) = source_diagnostic {
        diagnostics.insert(0, diagnostic);
    }
    if !diagnostics.is_empty() {
        return CompanyOutcome::Incomplete {
            diagnostic: diagnostics.join("; "),
            observed_count,
        };
    }

    let eligible_count = jobs.iter().filter(|job| job.eligibility.eligible).count();
    CompanyOutcome::Complete {
        jobs,
        observed_count,
        eligible_count,
    }
}

fn source_failure(error: SourceError) -> ScanFailure {
    ScanFailure {
        kind: error.kind,
        diagnostic: error.message,
    }
}

fn storage_failure(diagnostic: String) -> CompanyOutcome {
    CompanyOutcome::Failed {
        failure: ScanFailure {
            kind: SourceErrorKind::Storage,
            diagnostic,
        },
    }
}

struct CompanyScan {
    company_id: String,
    company: Option<CompanyConfig>,
    window: ScanWindow,
    outcome: CompanyOutcome,
}

#[derive(Clone, Copy)]
struct ScanWindow {
    started_at: DateTime<Utc>,
    completed_at: DateTime<Utc>,
}

enum CompanyOutcome {
    Complete {
        jobs: Vec<ClassifiedJob>,
        observed_count: usize,
        eligible_count: usize,
    },
    Incomplete {
        diagnostic: String,
        observed_count: usize,
    },
    Failed {
        failure: ScanFailure,
    },
}
