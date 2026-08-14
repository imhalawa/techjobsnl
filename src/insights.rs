use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use crate::{
    analytics::{JobFacts, Seniority, SkillKind, StackRole, WorkMode},
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
    pub path: Vec<String>,
    pub profile: StackProfile,
    pub metric: MetricRow,
    pub company_count: usize,
    pub association_bps: usize,
    pub saved: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackProfile {
    Application,
    Frontend,
    Data,
    Ai,
    Platform,
    Mobile,
    Testing,
    Security,
}

impl StackProfile {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Application => "Application",
            Self::Frontend => "Frontend",
            Self::Data => "Data",
            Self::Ai => "AI",
            Self::Platform => "Platform",
            Self::Mobile => "Mobile",
            Self::Testing => "Testing",
            Self::Security => "Security",
        }
    }
}

impl StackTrend {
    pub fn path_label(&self) -> String {
        self.path.join(" — ")
    }
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
        report.stacks.truncate(self.maximum_skills);
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
    let graph = StackGraph::build(current, facts);
    let mut supported = graph
        .candidates
        .iter()
        .filter(|(_, candidate)| candidate.support >= minimum_support)
        .filter_map(|(key, candidate)| {
            graph
                .semantic_path(key)
                .map(|(path, association_bps, profile)| {
                    (key.clone(), path, association_bps, profile, candidate)
                })
        })
        .filter(|(_, _, _, _, candidate)| {
            graph.companies.len() < 2 || candidate.companies.len() >= 2
        })
        .collect::<Vec<_>>();
    let supported_keys = supported
        .iter()
        .map(|(key, _, _, _, candidate)| (key.clone(), candidate.support))
        .collect::<Vec<_>>();
    supported.retain(|(key, _, _, _, candidate)| {
        !supported_keys.iter().any(|(larger, larger_support)| {
            larger.0.len() > key.0.len()
                && *larger_support == candidate.support
                && key.0.iter().all(|skill| larger.0.contains(skill))
        })
    });
    let mut rows = supported
        .into_iter()
        .map(|(key, path, association_bps, profile, candidate)| {
            let path_label = path.join(" — ");
            StackTrend {
                metric: metric(
                    &path_label,
                    candidate.support,
                    current.len(),
                    count_stack(period, facts, &key),
                    period.len(),
                    count_stack(previous, facts, &key),
                    previous.len(),
                ),
                company_count: candidate.companies.len(),
                association_bps,
                profile,
                saved: library.stacks.contains(&key.0),
                key,
                path,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .metric
            .current_count
            .cmp(&left.metric.current_count)
            .then_with(|| right.company_count.cmp(&left.company_count))
            .then_with(|| right.association_bps.cmp(&left.association_bps))
            .then_with(|| left.key.cmp(&right.key))
    });
    let mut distinct = Vec::<StackTrend>::new();
    for row in rows {
        if distinct
            .iter()
            .all(|existing| !substantially_overlaps(&existing.key, &row.key))
        {
            distinct.push(row);
        }
    }
    distinct
}

fn substantially_overlaps(left: &StackKey, right: &StackKey) -> bool {
    let shared = left
        .0
        .iter()
        .filter(|skill| right.0.contains(skill))
        .count();
    let union = left.0.len() + right.0.len() - shared;
    shared * 4 >= union * 3
}

#[derive(Default)]
struct StackCandidate {
    support: usize,
    companies: HashSet<String>,
}

#[derive(Default)]
struct StackGraph {
    job_count: usize,
    companies: HashSet<String>,
    nodes: HashMap<String, usize>,
    roles: HashMap<String, StackRole>,
    edges: BTreeMap<(String, String), usize>,
    candidates: BTreeMap<StackKey, StackCandidate>,
}

impl StackGraph {
    fn build(jobs: &[&JobRecord], facts: &HashMap<JobKey, JobFacts>) -> Self {
        let mut graph = Self {
            job_count: jobs.len(),
            ..Self::default()
        };
        for job in jobs {
            graph.companies.insert(job.key.company_id.clone());
            let Some(job_facts) = facts.get(&job.key) else {
                continue;
            };
            for (name, evidence) in &job_facts.skills {
                if let Some(role) = evidence.stack_role {
                    *graph.nodes.entry(name.clone()).or_default() += 1;
                    graph.roles.insert(name.clone(), role);
                }
            }
        }
        for job in jobs {
            let Some(job_facts) = facts.get(&job.key) else {
                continue;
            };
            let stackable = job_facts
                .skills
                .iter()
                .filter(|(_, evidence)| evidence.stack_role.is_some());
            let mut families = HashMap::<String, (&String, u8)>::new();
            for (name, evidence) in stackable {
                let family = evidence.stack_family.as_ref().unwrap_or(name).clone();
                let replace = families.get(&family).is_none_or(|(current, priority)| {
                    evidence.stack_priority > *priority
                        || evidence.stack_priority == *priority && name < *current
                });
                if replace {
                    families.insert(family, (name, evidence.stack_priority));
                }
            }
            let mut skills = families
                .into_values()
                .map(|(name, _)| name.clone())
                .collect::<Vec<_>>();
            skills.sort_by(|left, right| {
                graph
                    .nodes
                    .get(right)
                    .cmp(&graph.nodes.get(left))
                    .then_with(|| left.cmp(right))
            });
            // ponytail: cap dense postings at 12 skills; raise only if real data loses useful paths.
            skills.truncate(12);
            skills.sort();
            combinations(&skills, 2, 0, &mut Vec::new(), &mut |items| {
                if supports_stack(job_facts, items) {
                    *graph
                        .edges
                        .entry(edge_key(&items[0], &items[1]))
                        .or_default() += 1;
                }
            });
            for size in 3..=5.min(skills.len()) {
                combinations(&skills, size, 0, &mut Vec::new(), &mut |items| {
                    if supports_stack(job_facts, items) {
                        let candidate = graph
                            .candidates
                            .entry(StackKey(items.to_vec()))
                            .or_default();
                        candidate.support += 1;
                        candidate.companies.insert(job.key.company_id.clone());
                    }
                });
            }
        }
        graph
    }

    fn semantic_path(&self, key: &StackKey) -> Option<(Vec<String>, usize, StackProfile)> {
        if key
            .0
            .iter()
            .filter(|skill| {
                matches!(
                    self.roles.get(*skill),
                    Some(StackRole::Language | StackRole::WebLanguage)
                )
            })
            .count()
            > 1
        {
            return None;
        }
        let distinct_roles = key
            .0
            .iter()
            .filter_map(|skill| self.roles.get(skill))
            .copied()
            .collect::<HashSet<_>>();
        let profile = stack_profile(&distinct_roles)?;
        let mut path = key.0.clone();
        path.sort_by(|left, right| {
            self.roles
                .get(left)
                .cmp(&self.roles.get(right))
                .then_with(|| left.cmp(right))
        });
        let association = path
            .windows(2)
            .filter_map(|pair| self.edge_association_bps(&pair[0], &pair[1]))
            .min()
            .unwrap_or_default();
        Some((path, association, profile))
    }

    fn edge_association_bps(&self, left: &str, right: &str) -> Option<usize> {
        let support = self.edges.get(&edge_key(left, right)).copied()?;
        let denominator = self.nodes.get(left)? * self.nodes.get(right)?;
        Some(support * self.job_count * 100 / denominator)
    }
}

fn stack_profile(roles: &HashSet<StackRole>) -> Option<StackProfile> {
    use StackRole as R;

    if roles.len() < 3 {
        return None;
    }
    let has = |candidates: &[R]| candidates.iter().any(|role| roles.contains(role));
    let only = |candidates: &[R]| roles.iter().all(|role| candidates.contains(role));
    let language = has(&[R::Language, R::Runtime]);
    let application_language = has(&[R::Language, R::WebLanguage, R::Runtime]);
    let application = application_language
        && has(&[R::ApplicationFramework])
        && has(&[
            R::ApiProtocol,
            R::Database,
            R::Messaging,
            R::Cloud,
            R::Container,
        ])
        && only(&[
            R::Language,
            R::WebLanguage,
            R::Runtime,
            R::ApplicationFramework,
            R::ApiProtocol,
            R::Database,
            R::Messaging,
            R::Cloud,
            R::Container,
            R::BuildTool,
            R::TestingTool,
        ]);
    let frontend = has(&[R::Language, R::WebLanguage, R::Markup])
        && has(&[R::UiFramework, R::ApplicationFramework])
        && has(&[
            R::Markup,
            R::Styling,
            R::StateManagement,
            R::BuildTool,
            R::TestingTool,
        ])
        && only(&[
            R::Language,
            R::WebLanguage,
            R::Runtime,
            R::Markup,
            R::Styling,
            R::UiFramework,
            R::StateManagement,
            R::ApplicationFramework,
            R::ApiProtocol,
            R::BuildTool,
            R::TestingTool,
        ]);
    let data = language
        && has(&[R::DataProcessing, R::DataPlatform])
        && has(&[R::Database, R::Orchestration, R::Messaging, R::Cloud])
        && only(&[
            R::Language,
            R::Runtime,
            R::Database,
            R::DataLibrary,
            R::DataProcessing,
            R::DataPlatform,
            R::Orchestration,
            R::Messaging,
            R::Cloud,
            R::Container,
            R::Provisioning,
        ]);
    let ai = language
        && has(&[R::AiFramework, R::AiTooling])
        && has(&[R::DataLibrary, R::DataPlatform, R::Cloud, R::Container])
        && only(&[
            R::Language,
            R::Runtime,
            R::Database,
            R::DataLibrary,
            R::DataProcessing,
            R::DataPlatform,
            R::Orchestration,
            R::Messaging,
            R::AiFramework,
            R::AiTooling,
            R::Cloud,
            R::Container,
        ]);
    let platform = has(&[R::Provisioning, R::Delivery])
        && !has(&[R::Language, R::WebLanguage, R::Runtime])
        && has(&[R::Cloud, R::OperatingSystem, R::Container])
        && has(&[
            R::Container,
            R::ContainerTooling,
            R::Delivery,
            R::Networking,
            R::ObservabilityTool,
        ])
        && only(&[
            R::Cloud,
            R::OperatingSystem,
            R::Container,
            R::ContainerTooling,
            R::Provisioning,
            R::SourceControl,
            R::BuildTool,
            R::Delivery,
            R::Networking,
            R::ObservabilityTool,
            R::SecurityTooling,
        ]);
    let mobile = application_language
        && has(&[R::MobilePlatform, R::MobileFramework])
        && has(&[
            R::ApplicationFramework,
            R::BuildTool,
            R::Cloud,
            R::TestingTool,
        ])
        && only(&[
            R::Language,
            R::WebLanguage,
            R::Runtime,
            R::MobilePlatform,
            R::MobileFramework,
            R::ApplicationFramework,
            R::ApiProtocol,
            R::Database,
            R::Cloud,
            R::BuildTool,
            R::TestingTool,
        ]);
    let testing = application_language
        && has(&[R::ApplicationFramework, R::UiFramework])
        && has(&[R::TestingTool])
        && only(&[
            R::Language,
            R::WebLanguage,
            R::Runtime,
            R::UiFramework,
            R::ApplicationFramework,
            R::ApiProtocol,
            R::Database,
            R::BuildTool,
            R::Delivery,
            R::TestingTool,
        ]);
    let security = has(&[R::SecurityProtocol, R::SecurityTooling])
        && has(&[
            R::Language,
            R::WebLanguage,
            R::ApplicationFramework,
            R::ApiProtocol,
        ])
        && has(&[R::Cloud, R::Database, R::Delivery, R::TestingTool])
        && only(&[
            R::Language,
            R::WebLanguage,
            R::Runtime,
            R::ApplicationFramework,
            R::ApiProtocol,
            R::Database,
            R::Cloud,
            R::OperatingSystem,
            R::Container,
            R::Delivery,
            R::Networking,
            R::TestingTool,
            R::SecurityProtocol,
            R::SecurityTooling,
        ]);

    if security {
        Some(StackProfile::Security)
    } else if ai {
        Some(StackProfile::Ai)
    } else if frontend {
        Some(StackProfile::Frontend)
    } else if mobile {
        Some(StackProfile::Mobile)
    } else if data {
        Some(StackProfile::Data)
    } else if platform {
        Some(StackProfile::Platform)
    } else if testing {
        Some(StackProfile::Testing)
    } else if application {
        Some(StackProfile::Application)
    } else {
        None
    }
}

fn edge_key(left: &str, right: &str) -> (String, String) {
    if left <= right {
        (left.to_owned(), right.to_owned())
    } else {
        (right.to_owned(), left.to_owned())
    }
}

pub(crate) fn supports_stack(facts: &JobFacts, skills: &[String]) -> bool {
    skills.iter().all(|skill| facts.skills.contains_key(skill))
        && !skills.iter().enumerate().any(|(index, left)| {
            skills[index + 1..]
                .iter()
                .any(|right| skills_are_alternatives(facts, left, right))
        })
}

fn skills_are_alternatives(facts: &JobFacts, left: &str, right: &str) -> bool {
    let Some(left) = facts.skills.get(left) else {
        return false;
    };
    let Some(right) = facts.skills.get(right) else {
        return false;
    };
    left.context == right.context
        && [" or ", "one of", "at least one", "either"]
            .iter()
            .any(|marker| left.context.to_lowercase().contains(marker))
}

fn count_stack(jobs: &[&JobRecord], facts: &HashMap<JobKey, JobFacts>, key: &StackKey) -> usize {
    jobs.iter()
        .filter(|job| {
            facts
                .get(&job.key)
                .is_some_and(|facts| supports_stack(facts, &key.0))
        })
        .count()
}

fn combinations(
    items: &[String],
    size: usize,
    start: usize,
    chosen: &mut Vec<String>,
    visit: &mut impl FnMut(&[String]),
) {
    if items.len() < size || chosen.len() > size {
        return;
    }
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
                        "Python Django PostgreSQL communication skills"
                    } else {
                        "Java GCP"
                    },
                )
            })
            .collect::<Vec<_>>();
        jobs.extend((12..24).map(|index| {
            job(
                index,
                now - Duration::days(40),
                if index < 15 {
                    "Python Django"
                } else {
                    "Java GCP"
                },
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
            stack.key.0 == ["Django", "PostgreSQL", "Python"]
                && stack.metric.current_count == 8
                && stack.path.len() == 3
                && stack.company_count == 1
        }));
        assert!(report.stacks.iter().all(|stack| stack.key.0.len() >= 3));
        assert!(
            report
                .recommendations
                .iter()
                .all(|recommendation| recommendation.skill != "Python")
        );
        assert_eq!(report.roles[0].name, "Backend");
    }

    #[test]
    fn alternative_skill_lists_do_not_form_graph_paths() {
        let now = Utc.with_ymd_and_hms(2026, 8, 14, 12, 0, 0).unwrap();
        let jobs = (0..3)
            .map(|index| {
                job(
                    index,
                    now - Duration::days(1),
                    "Experience with Java, Python, or Go.",
                )
            })
            .collect::<Vec<_>>();
        let facts = jobs
            .iter()
            .map(|job| (job.key.clone(), analytics::extract(job)))
            .collect::<HashMap<_, _>>();

        let report = AnalyticsReport::build(
            &jobs,
            &facts,
            &[],
            &AnalyticsFilters::default(),
            &LibraryState::default(),
            now,
            3,
        );

        assert!(report.stacks.is_empty());
    }

    #[test]
    fn mixed_architectures_do_not_form_stack_paths() {
        let now = Utc.with_ymd_and_hms(2026, 8, 14, 12, 0, 0).unwrap();
        let jobs = (0..3)
            .map(|index| {
                job(
                    index,
                    now - Duration::days(1),
                    "Python React Databricks AWS",
                )
            })
            .collect::<Vec<_>>();
        let facts = jobs
            .iter()
            .map(|job| (job.key.clone(), analytics::extract(job)))
            .collect::<HashMap<_, _>>();

        let report = AnalyticsReport::build(
            &jobs,
            &facts,
            &[],
            &AnalyticsFilters::default(),
            &LibraryState::default(),
            now,
            3,
        );

        assert!(
            report
                .stacks
                .iter()
                .all(|stack| !stack.path.iter().any(|skill| skill == "React"))
        );
    }
}
