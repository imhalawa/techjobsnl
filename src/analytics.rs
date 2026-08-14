use std::collections::BTreeMap;
use std::fmt::Write as _;

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::{config::AnalyticsConfig, domain::JobRecord};

const EXTRACTOR_VERSION: &str = "rules-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkMode {
    Remote,
    Hybrid,
    OnSite,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Seniority {
    Intern,
    Junior,
    Mid,
    Senior,
    Lead,
    Manager,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequirementKind {
    Required,
    Preferred,
    Mentioned,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillEvidence {
    pub matched_alias: String,
    pub context: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceFact {
    pub minimum_months: u16,
    pub maximum_months: Option<u16>,
    pub requirement: RequirementKind,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EducationFact {
    pub requirement: RequirementKind,
    pub allows_equivalent_experience: bool,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobFacts {
    pub skills: BTreeMap<String, SkillEvidence>,
    pub work_mode: WorkMode,
    pub seniority: Seniority,
    pub experience: Vec<ExperienceFact>,
    pub education: Option<EducationFact>,
    pub employment_type_known: bool,
}

pub fn cache_version(config: &AnalyticsConfig) -> String {
    let mut input = String::from(EXTRACTOR_VERSION);
    for (name, aliases) in &config.skills {
        input.push('\0');
        input.push_str(name);
        for alias in aliases {
            input.push('\0');
            input.push_str(alias);
        }
    }
    Sha256::digest(input.as_bytes())
        .iter()
        .fold(String::with_capacity(64), |mut encoded, byte| {
            write!(encoded, "{byte:02x}").expect("writing to a String cannot fail");
            encoded
        })
}

pub fn extract(job: &JobRecord, skills: &BTreeMap<String, Vec<String>>) -> JobFacts {
    let description = &job.classified.observed.description;
    JobFacts {
        skills: skills
            .iter()
            .filter_map(|(name, aliases)| {
                skill_evidence(description, aliases).map(|evidence| (name.clone(), evidence))
            })
            .collect(),
        work_mode: work_mode(job),
        seniority: seniority(&job.classified.observed.title),
        experience: experience_facts(description),
        education: education_fact(description),
        employment_type_known: job
            .classified
            .observed
            .employment_type
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty()),
    }
}

fn skill_evidence(description: &str, aliases: &[String]) -> Option<SkillEvidence> {
    let mut aliases = aliases.iter().collect::<Vec<_>>();
    aliases.sort_by_key(|alias| std::cmp::Reverse(alias.len()));
    aliases.into_iter().find_map(|alias| {
        description.lines().find_map(|line| {
            contains_term(&line.to_lowercase(), &alias.to_lowercase()).then(|| SkillEvidence {
                matched_alias: alias.clone(),
                context: compact(line, 180),
            })
        })
    })
}

fn work_mode(job: &JobRecord) -> WorkMode {
    let mut structured = Vec::new();
    collect_values(
        &job.classified.observed.raw_payload,
        &[
            "jobLocationType",
            "workplaceType",
            "workplace_type",
            "remote",
        ],
        &mut structured,
    );
    structured.extend(job.classified.observed.locations.iter().cloned());
    if let Some(mode) = classify_work_mode(&structured.join(" "), true) {
        return mode;
    }
    classify_work_mode(&job.classified.observed.description, false).unwrap_or(WorkMode::Unknown)
}

fn classify_work_mode(value: &str, structured: bool) -> Option<WorkMode> {
    let value = value.to_lowercase();
    if contains_phrase(&value, &["hybrid", "partly remote", "partially remote"]) {
        Some(WorkMode::Hybrid)
    } else if (structured && (contains_term(&value, "remote") || contains_term(&value, "true")))
        || contains_phrase(
            &value,
            &[
                "telecommute",
                "fully remote",
                "remote role",
                "remote position",
                "work from home",
            ],
        )
    {
        Some(WorkMode::Remote)
    } else if contains_phrase(&value, &["on-site", "onsite", "on site", "office-based"]) {
        Some(WorkMode::OnSite)
    } else {
        None
    }
}

fn seniority(title: &str) -> Seniority {
    let title = title.to_lowercase();
    if contains_phrase(&title, &["manager", "head of", "director"]) {
        Seniority::Manager
    } else if contains_phrase(&title, &["staff", "principal", "lead"]) {
        Seniority::Lead
    } else if contains_term(&title, "senior") || contains_term(&title, "sr") {
        Seniority::Senior
    } else if contains_phrase(
        &title,
        &["junior", "graduate", "entry level", "entry-level"],
    ) {
        Seniority::Junior
    } else if contains_phrase(&title, &["intern", "internship", "trainee"]) {
        Seniority::Intern
    } else if contains_term(&title, "mid") || contains_term(&title, "medior") {
        Seniority::Mid
    } else {
        Seniority::Unknown
    }
}

fn experience_facts(description: &str) -> Vec<ExperienceFact> {
    let pattern =
        Regex::new(r"(?i)\b(\d{1,2})\s*(?:\+|(?:-|–|to)\s*(\d{1,2}))?\s*(?:years?|yrs?)\b")
            .expect("static experience pattern must be valid");
    description
        .lines()
        .flat_map(|line| {
            pattern.captures_iter(line).filter_map(move |captures| {
                let minimum_years = captures.get(1)?.as_str().parse::<u16>().ok()?;
                let maximum_years = captures
                    .get(2)
                    .and_then(|value| value.as_str().parse::<u16>().ok());
                Some(ExperienceFact {
                    minimum_months: minimum_years.saturating_mul(12),
                    maximum_months: maximum_years.map(|years| years.saturating_mul(12)),
                    requirement: requirement_kind(line),
                    evidence: compact(line, 180),
                })
            })
        })
        .collect()
}

fn education_fact(description: &str) -> Option<EducationFact> {
    description.lines().find_map(|line| {
        let lower = line.to_lowercase();
        contains_phrase(
            &lower,
            &[
                "bachelor",
                "master's",
                "masters",
                "master degree",
                "degree",
                "phd",
            ],
        )
        .then(|| EducationFact {
            requirement: requirement_kind(line),
            allows_equivalent_experience: contains_phrase(
                &lower,
                &["equivalent experience", "or equivalent"],
            ),
            evidence: compact(line, 180),
        })
    })
}

fn requirement_kind(line: &str) -> RequirementKind {
    let line = line.to_lowercase();
    if contains_phrase(
        &line,
        &["preferred", "nice to have", "is a plus", "advantage"],
    ) {
        RequirementKind::Preferred
    } else if contains_phrase(&line, &["required", "must have", "minimum", "at least"]) {
        RequirementKind::Required
    } else {
        RequirementKind::Mentioned
    }
}

fn collect_values(value: &Value, keys: &[&str], found: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                if keys
                    .iter()
                    .any(|candidate| key.eq_ignore_ascii_case(candidate))
                {
                    match value {
                        Value::String(value) => found.push(value.clone()),
                        Value::Bool(value) => found.push(value.to_string()),
                        _ => {}
                    }
                }
                collect_values(value, keys, found);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_values(value, keys, found);
            }
        }
        _ => {}
    }
}

fn compact(value: &str, maximum_chars: usize) -> String {
    let compact = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= maximum_chars {
        compact
    } else {
        format!(
            "{}…",
            compact.chars().take(maximum_chars - 1).collect::<String>()
        )
    }
}

fn contains_phrase(text: &str, phrases: &[&str]) -> bool {
    phrases.iter().any(|phrase| contains_term(text, phrase))
}

pub fn contains_term(text: &str, term: &str) -> bool {
    !term.is_empty()
        && text.match_indices(term).any(|(start, _)| {
            let end = start + term.len();
            let before = text[..start].chars().next_back();
            let after = text[end..].chars().next();
            before.is_none_or(|character| !character.is_alphanumeric())
                && after.is_none_or(|character| !character.is_alphanumeric())
        })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::Utc;

    use super::{RequirementKind, Seniority, WorkMode, extract};
    use crate::domain::{ClassifiedJob, Eligibility, JobKey, JobRecord, ObservedJob};

    fn job(title: &str, description: &str) -> JobRecord {
        JobRecord {
            key: JobKey::new("example", "1"),
            classified: ClassifiedJob {
                observed: ObservedJob {
                    source_id: "1".into(),
                    title: title.into(),
                    department: None,
                    team: None,
                    employment_type: Some("Full time".into()),
                    locations: vec!["Amsterdam".into()],
                    countries: vec!["NL".into()],
                    job_url: "https://example.test/job".into(),
                    apply_url: "https://example.test/apply".into(),
                    description: description.into(),
                    raw_payload: serde_json::json!({"jobLocationType": "TELECOMMUTE"}),
                    published_at: None,
                },
                eligibility: Eligibility {
                    eligible: true,
                    reason: "eligible".into(),
                },
            },
            source_open: true,
            is_new: true,
            first_seen_at: Utc::now(),
            last_seen_at: Utc::now(),
            closed_at: None,
            reopened_at: None,
            applied_at: None,
        }
    }

    #[test]
    fn extracts_canonical_skills_and_explainable_work_facts() {
        let skills = BTreeMap::from([
            ("Go".into(), vec!["go".into(), "golang".into()]),
            ("Kubernetes".into(), vec!["k8s".into()]),
        ]);
        let facts = extract(
            &job(
                "Senior Platform Engineer",
                "Five years ongoing work is irrelevant.\nRequired: 3-5 years with Golang and k8s.\nBachelor degree or equivalent experience.",
            ),
            &skills,
        );

        assert_eq!(facts.work_mode, WorkMode::Remote);
        assert_eq!(facts.seniority, Seniority::Senior);
        assert_eq!(
            facts.skills.keys().cloned().collect::<Vec<_>>(),
            ["Go", "Kubernetes"]
        );
        assert_eq!(facts.experience[0].minimum_months, 36);
        assert_eq!(facts.experience[0].maximum_months, Some(60));
        assert_eq!(facts.experience[0].requirement, RequirementKind::Required);
        assert!(facts.education.unwrap().allows_equivalent_experience);
    }
}
