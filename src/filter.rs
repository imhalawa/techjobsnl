use std::collections::{HashMap, HashSet};

use regex::{Regex, RegexBuilder};
use thiserror::Error;

use crate::{
    config::FiltersConfig,
    domain::{Eligibility, ObservedJob},
};

#[derive(Debug, Clone)]
pub struct EligibilityFilter {
    countries: HashSet<String>,
    include_patterns: Vec<Regex>,
    exclude_patterns: Vec<Regex>,
}

impl EligibilityFilter {
    pub fn new(filters: &FiltersConfig) -> Result<Self, FilterError> {
        let include_patterns = filters
            .include_title_patterns
            .iter()
            .map(|pattern| compile_pattern("include title", pattern))
            .collect::<Result<_, _>>()?;

        let exclude_patterns = filters
            .exclude_title_patterns
            .iter()
            .map(|pattern| compile_pattern("exclude title", pattern))
            .collect::<Result<_, _>>()?;

        Ok(Self {
            countries: filters.countries.iter().cloned().collect(),
            include_patterns,
            exclude_patterns,
        })
    }

    pub fn classify(
        &self,
        job: &ObservedJob,
        location_country_overrides: &HashMap<String, String>,
    ) -> Result<Eligibility, FilterError> {
        let countries = resolved_countries(job, location_country_overrides)?;

        if !countries
            .iter()
            .any(|country| self.countries.contains(*country))
        {
            return Ok(Eligibility {
                eligible: false,
                reason: "outside-configured-countries".into(),
            });
        }

        if self
            .exclude_patterns
            .iter()
            .any(|pattern| pattern.is_match(&job.title))
        {
            return Ok(Eligibility {
                eligible: false,
                reason: "excluded-title".into(),
            });
        }

        if !self.include_patterns.is_empty()
            && !self
                .include_patterns
                .iter()
                .any(|pattern| pattern.is_match(&job.title))
        {
            return Ok(Eligibility {
                eligible: false,
                reason: "not-included-title".into(),
            });
        }

        Ok(Eligibility {
            eligible: true,
            reason: "eligible".into(),
        })
    }
}

fn compile_pattern(kind: &'static str, pattern: &str) -> Result<Regex, FilterError> {
    RegexBuilder::new(pattern)
        .case_insensitive(true)
        .build()
        .map_err(|error| FilterError::InvalidPattern {
            kind,
            pattern: pattern.into(),
            message: error.to_string(),
        })
}

fn resolved_countries<'a>(
    job: &'a ObservedJob,
    location_country_overrides: &'a HashMap<String, String>,
) -> Result<Vec<&'a str>, FilterError> {
    if !job.countries.is_empty() {
        return Ok(job.countries.iter().map(String::as_str).collect());
    }

    let unresolved_locations = job
        .locations
        .iter()
        .filter(|location| !location_country_overrides.contains_key(*location))
        .cloned()
        .collect::<Vec<_>>();
    if !unresolved_locations.is_empty() {
        return Err(FilterError::UnresolvedLocation(unresolved_locations));
    }

    Ok(job
        .locations
        .iter()
        .filter_map(|location| location_country_overrides.get(location).map(String::as_str))
        .collect())
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum FilterError {
    #[error("unresolved location labels: {0:?}")]
    UnresolvedLocation(Vec<String>),
    #[error("invalid {kind} pattern `{pattern}`: {message}")]
    InvalidPattern {
        kind: &'static str,
        pattern: String,
        message: String,
    },
}
