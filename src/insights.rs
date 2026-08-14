use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{JobFacts, Seniority, SkillKind, WorkMode},
    domain::{JobKey, JobRecord},
    storage::{ScanOutcome, ScanReadModel},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyticsFilters {
    pub window_days: u16,
    pub company: Option<String>,
    pub role: Option<String>,
    pub seniority: Option<Seniority>,
    pub work_mode: Option<WorkMode>,
}

impl Default for AnalyticsFilters {
    fn default() -> Self {
        Self {
            window_days: 30,
            company: None,
            role: None,
            seniority: None,
            work_mode: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SkillStatus {
    Known,
    Learning,
    Interested,
}

impl SkillStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Known => "known",
            Self::Learning => "learning",
            Self::Interested => "interested",
        }
    }

    pub const fn next(self) -> Option<Self> {
        match self {
            Self::Known => Some(Self::Learning),
            Self::Learning => Some(Self::Interested),
            Self::Interested => None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LibraryState {
    pub jobs: BTreeSet<JobKey>,
    pub skills: BTreeMap<String, Option<SkillStatus>>,
    pub stacks: BTreeSet<Vec<String>>,
    pub roles: BTreeMap<String, bool>,
    pub companies: BTreeSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    High,
    Low,
}

impl Confidence {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::High => "high",
            Self::Low => "low",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Momentum {
    New,
    Rising,
    Stable,
    Falling,
    LowConfidence,
}

impl Momentum {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::New => "new",
            Self::Rising => "rising",
            Self::Stable => "stable",
            Self::Falling => "falling",
            Self::LowConfidence => "low confidence",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MetricRow {
    pub name: String,
    pub current_count: usize,
    pub current_share_per_mille: usize,
    pub period_count: usize,
    pub previous_count: usize,
    pub delta_count: isize,
    pub delta_share_per_mille: isize,
    pub confidence: Confidence,
    pub momentum: Momentum,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillTrend {
    pub metric: MetricRow,
    pub kind: SkillKind,
    pub status: Option<SkillStatus>,
    pub saved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct StackKey(pub Vec<String>);

impl StackKey {
    pub fn label(&self) -> String {
        self.0.join(" + ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackTrend {
    pub key: StackKey,
    pub metric: MetricRow,
    pub saved: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recommendation {
    pub skill: String,
    pub kind: SkillKind,
    pub saved: bool,
    pub demand_count: usize,
    pub target_role_count: usize,
    pub adjacent_known_count: usize,
    pub momentum: Momentum,
    pub confidence: Confidence,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct AnalyticsReport {
    pub generated_at: DateTime<Utc>,
    pub earliest_observation: Option<DateTime<Utc>>,
    pub active_job_count: usize,
    pub period_job_count: usize,
    pub previous_job_count: usize,
    pub comparable_company_count: usize,
    pub hard_skills: Vec<SkillTrend>,
    pub soft_skills: Vec<SkillTrend>,
    pub stacks: Vec<StackTrend>,
    pub roles: Vec<MetricRow>,
    pub seniority: Vec<MetricRow>,
    pub experience: Vec<MetricRow>,
    pub work: Vec<MetricRow>,
    pub employment: Vec<MetricRow>,
    pub education: Vec<MetricRow>,
    pub companies: Vec<MetricRow>,
    pub recommendations: Vec<Recommendation>,
}

#[derive(Debug, Clone)]
pub struct AnalyticsWork {
    revision: u64,
    jobs: Vec<JobRecord>,
    facts: HashMap<JobKey, JobFacts>,
    scans: Vec<ScanReadModel>,
    filters: AnalyticsFilters,
    library: LibraryState,
    minimum_stack_support: usize,
    minimum_skill_occurrence: usize,
    maximum_skills: usize,
}

#[derive(Debug)]
pub struct AnalyticsResult {
    pub(crate) revision: u64,
    pub(crate) report: AnalyticsReport,
}

impl AnalyticsWork {
    pub(crate) fn new(
        revision: u64,
        jobs: Vec<JobRecord>,
        facts: HashMap<JobKey, JobFacts>,
        scans: Vec<ScanReadModel>,
        filters: AnalyticsFilters,
        library: LibraryState,
        limits: (usize, usize, usize),
    ) -> Self {
        let (minimum_stack_support, minimum_skill_occurrence, maximum_skills) = limits;
        Self {
            revision,
            jobs,
            facts,
            scans,
            filters,
            library,
            minimum_stack_support,
            minimum_skill_occurrence,
            maximum_skills,
        }
    }

    pub fn compute(self) -> AnalyticsResult {
        let mut report = AnalyticsReport::build(
            &self.jobs,
            &self.facts,
            &self.scans,
            &self.filters,
            &self.library,
            Utc::now(),
            self.minimum_stack_support,
        );
        report
            .hard_skills
            .retain(|item| item.metric.current_count >= self.minimum_skill_occurrence);
        report
            .soft_skills
            .retain(|item| item.metric.current_count >= self.minimum_skill_occurrence);
        report
            .recommendations
            .retain(|item| item.demand_count >= self.minimum_skill_occurrence);
        report.hard_skills.truncate(self.maximum_skills);
        report.soft_skills.truncate(self.maximum_skills);
        report.recommendations.truncate(self.maximum_skills);
        AnalyticsResult {
            revision: self.revision,
            report,
        }
    }
}

impl AnalyticsReport {
    pub fn build(
        jobs: &[JobRecord],
        facts: &HashMap<JobKey, JobFacts>,
        scans: &[ScanReadModel],
        filters: &AnalyticsFilters,
        library: &LibraryState,
        now: DateTime<Utc>,
        minimum_stack_support: usize,
    ) -> Self {
        let current_start = now - Duration::days(i64::from(filters.window_days));
        let previous_start = current_start - Duration::days(i64::from(filters.window_days));
        let current_companies = complete_scan_companies(scans, current_start, now);
        let previous_companies = complete_scan_companies(scans, previous_start, current_start);
        let comparable_companies = current_companies
            .intersection(&previous_companies)
            .cloned()
            .collect::<HashSet<_>>();

        let matches = |job: &&JobRecord| {
            facts
                .get(&job.key)
                .is_some_and(|job_facts| matches_filters(job, job_facts, filters))
        };
        let active = jobs
            .iter()
            .filter(|job| job.source_open)
            .filter(matches)
            .collect::<Vec<_>>();
        let period = jobs
            .iter()
            .filter(matches)
            .filter(|job| {
                comparable_companies.contains(&job.key.company_id)
                    && job.first_seen_at >= current_start
                    && job.first_seen_at < now
            })
            .collect::<Vec<_>>();
        let previous = jobs
            .iter()
            .filter(matches)
            .filter(|job| {
                comparable_companies.contains(&job.key.company_id)
                    && job.first_seen_at >= previous_start
                    && job.first_seen_at < current_start
            })
            .collect::<Vec<_>>();

        let skill_names = jobs
            .iter()
            .filter_map(|job| facts.get(&job.key))
            .flat_map(|job_facts| job_facts.skills.keys().cloned())
            .collect::<BTreeSet<_>>();
        let mut skills = skill_names
            .into_iter()
            .filter_map(|name| {
                let kind = jobs
                    .iter()
                    .filter_map(|job| facts.get(&job.key))
                    .find_map(|job_facts| job_facts.skills.get(&name).map(|item| item.kind))?;
                let metric = metric(
                    &name,
                    count_skill(&active, facts, &name),
                    active.len(),
                    count_skill(&period, facts, &name),
                    period.len(),
                    count_skill(&previous, facts, &name),
                    previous.len(),
                );
                Some(SkillTrend {
                    status: library.skills.get(&name).copied().flatten(),
                    saved: library.skills.contains_key(&name),
                    metric,
                    kind,
                })
            })
            .collect::<Vec<_>>();
        skills.sort_by(skill_order);
        let hard_skills = skills
            .iter()
            .filter(|skill| skill.kind == SkillKind::Hard)
            .cloned()
            .collect::<Vec<_>>();
        let soft_skills = skills
            .iter()
            .filter(|skill| skill.kind == SkillKind::Soft)
            .cloned()
            .collect::<Vec<_>>();

        let stacks = stack_trends(
            &active,
            &period,
            &previous,
            facts,
            library,
            minimum_stack_support,
        );
        let roles = category_metrics(&active, &period, &previous, |job| {
            facts.get(&job.key).map(|value| value.role_family.clone())
        });
        let seniority = category_metrics(&active, &period, &previous, |job| {
            facts
                .get(&job.key)
                .map(|value| seniority_name(value.seniority).to_owned())
        });
        let experience = category_metrics(&active, &period, &previous, |job| {
            facts.get(&job.key).map(experience_bucket)
        });
        let work = category_metrics(&active, &period, &previous, |job| {
            facts
                .get(&job.key)
                .map(|value| work_mode_name(value.work_mode).to_owned())
        });
        let employment = category_metrics(&active, &period, &previous, |job| {
            Some(
                job.classified
                    .observed
                    .employment_type
                    .clone()
                    .unwrap_or_else(|| "Not stated".to_owned()),
            )
        });
        let education = category_metrics(&active, &period, &previous, |job| {
            facts.get(&job.key).map(|value| {
                value.education.as_ref().map_or_else(
                    || "Not stated".to_owned(),
                    |education| {
                        if education.allows_equivalent_experience {
                            "Degree or equivalent experience".to_owned()
                        } else {
                            "Degree stated".to_owned()
                        }
                    },
                )
            })
        });
        let companies = category_metrics(&active, &period, &previous, |job| {
            Some(job.key.company_id.clone())
        });
        let recommendations = recommendations(&skills, &active, facts, library);

        Self {
            generated_at: now,
            earliest_observation: jobs
                .iter()
                .filter(|job| {
                    facts
                        .get(&job.key)
                        .is_some_and(|value| matches_filters(job, value, filters))
                })
                .map(|job| job.first_seen_at)
                .min(),
            active_job_count: active.len(),
            period_job_count: period.len(),
            previous_job_count: previous.len(),
            comparable_company_count: comparable_companies.len(),
            hard_skills,
            soft_skills,
            stacks,
            roles,
            seniority,
            experience,
            work,
            employment,
            education,
            companies,
            recommendations,
        }
    }
}

pub fn matches_filters(job: &JobRecord, facts: &JobFacts, filters: &AnalyticsFilters) -> bool {
    filters
        .company
        .as_deref()
        .is_none_or(|company| job.key.company_id == company)
        && filters
            .role
            .as_deref()
            .is_none_or(|role| facts.role_family == role)
        && filters
            .seniority
            .is_none_or(|seniority| facts.seniority == seniority)
        && filters
            .work_mode
            .is_none_or(|work_mode| facts.work_mode == work_mode)
}

fn complete_scan_companies(
    scans: &[ScanReadModel],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> HashSet<String> {
    scans
        .iter()
        .filter(|scan| {
            scan.outcome == ScanOutcome::Complete
                && scan.completed_at >= start
                && scan.completed_at < end
        })
        .map(|scan| scan.company_id.clone())
        .collect()
}

fn count_skill(jobs: &[&JobRecord], facts: &HashMap<JobKey, JobFacts>, name: &str) -> usize {
    jobs.iter()
        .filter(|job| {
            facts
                .get(&job.key)
                .is_some_and(|value| value.skills.contains_key(name))
        })
        .count()
}

fn metric(
    name: &str,
    current_count: usize,
    current_total: usize,
    period_count: usize,
    period_total: usize,
    previous_count: usize,
    previous_total: usize,
) -> MetricRow {
    let current_share = share(current_count, current_total);
    let period_share = share(period_count, period_total);
    let previous_share = share(previous_count, previous_total);
    let confidence =
        if period_total >= 10 && previous_total >= 10 && period_count.max(previous_count) >= 3 {
            Confidence::High
        } else {
            Confidence::Low
        };
    let delta_share = period_share as isize - previous_share as isize;
    let momentum = if confidence == Confidence::Low {
        Momentum::LowConfidence
    } else if previous_count == 0 && period_count > 0 {
        Momentum::New
    } else if delta_share >= 20 {
        Momentum::Rising
    } else if delta_share <= -20 {
        Momentum::Falling
    } else {
        Momentum::Stable
    };
    MetricRow {
        name: name.to_owned(),
        current_count,
        current_share_per_mille: current_share,
        period_count,
        previous_count,
        delta_count: period_count as isize - previous_count as isize,
        delta_share_per_mille: delta_share,
        confidence,
        momentum,
    }
}

fn share(count: usize, total: usize) -> usize {
    count.saturating_mul(1_000) / total.max(1)
}

fn skill_order(left: &SkillTrend, right: &SkillTrend) -> std::cmp::Ordering {
    right
        .metric
        .current_count
        .cmp(&left.metric.current_count)
        .then_with(|| {
            right
                .metric
                .delta_share_per_mille
                .cmp(&left.metric.delta_share_per_mille)
        })
        .then_with(|| left.metric.name.cmp(&right.metric.name))
}

fn category_metrics(
    current: &[&JobRecord],
    period: &[&JobRecord],
    previous: &[&JobRecord],
    name: impl Fn(&JobRecord) -> Option<String>,
) -> Vec<MetricRow> {
    let current_counts = category_counts(current, &name);
    let period_counts = category_counts(period, &name);
    let previous_counts = category_counts(previous, &name);
    let names = current_counts
        .keys()
        .chain(period_counts.keys())
        .chain(previous_counts.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut rows = names
        .into_iter()
        .map(|item| {
            metric(
                &item,
                current_counts.get(&item).copied().unwrap_or(0),
                current.len(),
                period_counts.get(&item).copied().unwrap_or(0),
                period.len(),
                previous_counts.get(&item).copied().unwrap_or(0),
                previous.len(),
            )
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .current_count
            .cmp(&left.current_count)
            .then_with(|| left.name.cmp(&right.name))
    });
    rows
}

fn category_counts(
    jobs: &[&JobRecord],
    name: &impl Fn(&JobRecord) -> Option<String>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for job in jobs {
        if let Some(name) = name(job) {
            *counts.entry(name).or_default() += 1;
        }
    }
    counts
}

fn stack_trends(
    current: &[&JobRecord],
    period: &[&JobRecord],
    previous: &[&JobRecord],
    facts: &HashMap<JobKey, JobFacts>,
    library: &LibraryState,
    minimum_support: usize,
) -> Vec<StackTrend> {
    let current_counts = stack_counts(current, facts);
    let period_counts = stack_counts(period, facts);
    let previous_counts = stack_counts(previous, facts);
    let mut supported = current_counts
        .iter()
        .filter(|(_, count)| **count >= minimum_support)
        .map(|(stack, count)| (stack.clone(), *count))
        .collect::<Vec<_>>();
    let all_supported = supported.clone();
    supported.retain(|(stack, count)| {
        !all_supported.iter().any(|(larger, larger_count)| {
            larger.0.len() > stack.0.len()
                && larger_count == count
                && stack.0.iter().all(|skill| larger.0.contains(skill))
        })
    });
    let mut rows = supported
        .into_iter()
        .map(|(key, current_count)| StackTrend {
            metric: metric(
                &key.label(),
                current_count,
                current.len(),
                period_counts.get(&key).copied().unwrap_or(0),
                period.len(),
                previous_counts.get(&key).copied().unwrap_or(0),
                previous.len(),
            ),
            saved: library.stacks.contains(&key.0),
            key,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .metric
            .current_count
            .cmp(&left.metric.current_count)
            .then_with(|| left.key.cmp(&right.key))
    });
    rows
}

fn stack_counts(
    jobs: &[&JobRecord],
    facts: &HashMap<JobKey, JobFacts>,
) -> BTreeMap<StackKey, usize> {
    let individual = jobs
        .iter()
        .flat_map(|job| {
            facts
                .get(&job.key)
                .into_iter()
                .flat_map(|value| value.skills.iter())
        })
        .filter(|(_, evidence)| evidence.kind == SkillKind::Hard)
        .fold(HashMap::<String, usize>::new(), |mut counts, (name, _)| {
            *counts.entry(name.clone()).or_default() += 1;
            counts
        });
    let mut counts = BTreeMap::new();
    for job in jobs {
        let Some(job_facts) = facts.get(&job.key) else {
            continue;
        };
        let mut skills = job_facts
            .skills
            .iter()
            .filter(|(_, evidence)| evidence.kind == SkillKind::Hard)
            .map(|(name, _)| name.clone())
            .collect::<Vec<_>>();
        skills.sort_by(|left, right| {
            individual
                .get(right)
                .cmp(&individual.get(left))
                .then_with(|| left.cmp(right))
        });
        // ponytail: cap dense postings at 12 skills; raise only if real data loses useful stacks.
        skills.truncate(12);
        skills.sort();
        for size in 2..=5.min(skills.len()) {
            combinations(&skills, size, 0, &mut Vec::new(), &mut |items| {
                *counts.entry(StackKey(items.to_vec())).or_default() += 1;
            });
        }
    }
    counts
}

fn combinations(
    items: &[String],
    size: usize,
    start: usize,
    chosen: &mut Vec<String>,
    visit: &mut impl FnMut(&[String]),
) {
    if chosen.len() == size {
        visit(chosen);
        return;
    }
    for index in start..=items.len() - (size - chosen.len()) {
        chosen.push(items[index].clone());
        combinations(items, size, index + 1, chosen, visit);
        chosen.pop();
    }
}

fn recommendations(
    skills: &[SkillTrend],
    active: &[&JobRecord],
    facts: &HashMap<JobKey, JobFacts>,
    library: &LibraryState,
) -> Vec<Recommendation> {
    let known = library
        .skills
        .iter()
        .filter(|(_, status)| **status == Some(SkillStatus::Known))
        .map(|(name, _)| name.as_str())
        .collect::<HashSet<_>>();
    let targets = library
        .roles
        .iter()
        .filter(|(_, target)| **target)
        .map(|(name, _)| name.as_str())
        .collect::<HashSet<_>>();
    let mut rows = skills
        .iter()
        .filter(|skill| !known.contains(skill.metric.name.as_str()))
        .map(|skill| {
            let target_role_count = active
                .iter()
                .filter(|job| {
                    let Some(job_facts) = facts.get(&job.key) else {
                        return false;
                    };
                    (targets.is_empty() || targets.contains(job_facts.role_family.as_str()))
                        && job_facts.skills.contains_key(&skill.metric.name)
                })
                .count();
            let adjacent_known_count = active
                .iter()
                .filter(|job| {
                    facts.get(&job.key).is_some_and(|job_facts| {
                        job_facts.skills.contains_key(&skill.metric.name)
                            && job_facts
                                .skills
                                .keys()
                                .any(|name| known.contains(name.as_str()))
                    })
                })
                .count();
            let reason = format!(
                "{} active jobs · {} target-role jobs · {} jobs beside known skills · {}",
                skill.metric.current_count,
                target_role_count,
                adjacent_known_count,
                skill.metric.momentum.as_str()
            );
            Recommendation {
                skill: skill.metric.name.clone(),
                kind: skill.kind,
                saved: library.skills.contains_key(&skill.metric.name),
                demand_count: skill.metric.current_count,
                target_role_count,
                adjacent_known_count,
                momentum: skill.metric.momentum,
                confidence: skill.metric.confidence,
                reason,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .target_role_count
            .cmp(&left.target_role_count)
            .then_with(|| right.adjacent_known_count.cmp(&left.adjacent_known_count))
            .then_with(|| right.demand_count.cmp(&left.demand_count))
            .then_with(|| left.skill.cmp(&right.skill))
    });
    rows
}

pub fn seniority_name(value: Seniority) -> &'static str {
    match value {
        Seniority::Intern => "Intern",
        Seniority::Junior => "Junior",
        Seniority::Mid => "Mid",
        Seniority::Senior => "Senior",
        Seniority::Lead => "Lead",
        Seniority::Manager => "Manager",
        Seniority::Unknown => "Unknown",
    }
}

pub fn work_mode_name(value: WorkMode) -> &'static str {
    match value {
        WorkMode::Remote => "Remote",
        WorkMode::Hybrid => "Hybrid",
        WorkMode::OnSite => "On-site",
        WorkMode::Unknown => "Unknown",
    }
}

pub fn experience_bucket(facts: &JobFacts) -> String {
    let months = facts
        .experience
        .iter()
        .map(|fact| fact.minimum_months)
        .min();
    match months {
        None => "Not stated",
        Some(0..=24) => "0–2 years",
        Some(25..=48) => "3–4 years",
        Some(49..=72) => "5–6 years",
        Some(_) => "7+ years",
    }
    .to_owned()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;

    use super::*;
    use crate::{
        analytics,
        domain::{ClassifiedJob, Eligibility, ObservedJob},
        storage::ScanOutcome,
    };

    fn job(index: usize, seen: DateTime<Utc>, description: &str) -> JobRecord {
        JobRecord {
            key: JobKey::new("acme", index.to_string()),
            classified: ClassifiedJob {
                observed: ObservedJob {
                    source_id: index.to_string(),
                    title: "Backend Engineer".into(),
                    department: None,
                    team: None,
                    employment_type: Some("Full time".into()),
                    locations: vec!["Amsterdam".into()],
                    countries: vec!["NL".into()],
                    job_url: format!("https://example.test/{index}"),
                    apply_url: format!("https://example.test/{index}/apply"),
                    description: description.into(),
                    raw_payload: serde_json::json!({}),
                    published_at: Some(seen),
                },
                eligibility: Eligibility {
                    eligible: true,
                    reason: "eligible".into(),
                },
            },
            source_open: true,
            is_new: false,
            first_seen_at: seen,
            last_seen_at: seen,
            closed_at: None,
            reopened_at: None,
            applied_at: None,
        }
    }

    fn scan(at: DateTime<Utc>) -> ScanReadModel {
        ScanReadModel {
            run_id: at.to_rfc3339(),
            company_id: "acme".into(),
            company_name: "Acme".into(),
            completed_at: at,
            outcome: ScanOutcome::Complete,
            observed_count: 12,
            error_kind: None,
            diagnostic: None,
        }
    }

    #[test]
    fn report_builds_comparable_trends_stacks_and_personal_recommendations() {
        let now = Utc.with_ymd_and_hms(2026, 8, 14, 12, 0, 0).unwrap();
        let mut jobs = (0..12)
            .map(|index| {
                job(
                    index,
                    now - Duration::days(10),
                    if index < 8 {
                        "Python AWS Docker communication skills"
                    } else {
                        "Java AWS"
                    },
                )
            })
            .collect::<Vec<_>>();
        jobs.extend((12..24).map(|index| {
            job(
                index,
                now - Duration::days(40),
                if index < 15 { "Python AWS" } else { "Java AWS" },
            )
        }));
        let facts = jobs
            .iter()
            .map(|job| (job.key.clone(), analytics::extract(job)))
            .collect::<HashMap<_, _>>();
        let scans = vec![
            scan(now - Duration::days(5)),
            scan(now - Duration::days(35)),
        ];
        let mut library = LibraryState::default();
        library
            .skills
            .insert("Python".into(), Some(SkillStatus::Known));
        library.roles.insert("Backend".into(), true);

        let report = AnalyticsReport::build(
            &jobs,
            &facts,
            &scans,
            &AnalyticsFilters::default(),
            &library,
            now,
            3,
        );

        assert_eq!(report.active_job_count, 24);
        assert_eq!(report.period_job_count, 12);
        assert_eq!(report.previous_job_count, 12);
        assert_eq!(report.comparable_company_count, 1);
        let python = report
            .hard_skills
            .iter()
            .find(|skill| skill.metric.name == "Python")
            .unwrap();
        assert_eq!(python.metric.period_count, 8);
        assert_eq!(python.metric.previous_count, 3);
        assert_eq!(python.metric.momentum, Momentum::Rising);
        assert!(report.stacks.iter().any(|stack| {
            stack.key.0 == ["AWS", "Docker", "Python"] && stack.metric.current_count == 8
        }));
        assert!(
            report
                .recommendations
                .iter()
                .all(|recommendation| recommendation.skill != "Python")
        );
        assert_eq!(report.roles[0].name, "Backend");
    }
}
