use std::{
    cell::Cell,
    collections::HashMap,
    time::{Duration as StdDuration, Instant},
};

use chrono::{Duration, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::{Constraint, Layout, Rect};

use crate::{
    analytics::{
        self, JobFacts, Seniority, SkillEvidence, SkillKind, SkillSuggestion, SuggestionStatus,
        WorkMode,
    },
    config::{Config, FiltersConfig},
    domain::{JobKey, JobRecord, ScanEvent, SourceScan},
    insights::{
        self, AnalyticsFilters, AnalyticsReport, LibraryState, MetricRow, SkillStatus, StackKey,
    },
    storage::{ScanReadModel, SourceHealth, SourceReadModel},
};

use super::{IconSet, Theme};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Active,
    New,
    Applied,
    History,
    Scans,
    Sources,
    Analytics,
    Library,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticsTab {
    Overview,
    Skills,
    Stacks,
    Market,
}

impl AnalyticsTab {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Skills => "Skills",
            Self::Stacks => "Stacks",
            Self::Market => "Market",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketSection {
    Roles,
    Seniority,
    Experience,
    Work,
    Companies,
}

impl MarketSection {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Roles => "Roles",
            Self::Seniority => "Seniority",
            Self::Experience => "Experience",
            Self::Work => "Work",
            Self::Companies => "Companies",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LibraryTab {
    Jobs,
    Skills,
    Stacks,
    Roles,
    Companies,
}

impl LibraryTab {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Jobs => "Jobs",
            Self::Skills => "Skills",
            Self::Stacks => "Stacks",
            Self::Roles => "Roles",
            Self::Companies => "Companies",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkillStat {
    pub name: String,
    pub kind: SkillKind,
    pub job_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelatedSkillStat {
    pub name: String,
    pub job_count: usize,
    pub union_count: usize,
    pub jaccard_per_mille: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryStat {
    pub name: String,
    pub job_count: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AnalyticsCoverage {
    pub total: usize,
    pub descriptions: usize,
    pub published_dates: usize,
    pub work_mode: usize,
    pub seniority: usize,
    pub experience: usize,
    pub education: usize,
    pub employment_type: usize,
    pub healthy_sources: usize,
    pub enabled_sources: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    Setting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Navigation,
    Content,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    NewJobAge,
    Countries,
    IncludedTitles,
    ExcludedTitles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseTarget {
    Navigation(usize),
    AnalyticsTab(usize),
    MarketSection(usize),
    LibraryTab(usize),
    Item(usize),
    HardSkill(usize),
    SoftSkill(usize),
    Evidence(usize),
    Details,
    Divider,
    Setting(usize),
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    None,
    StartScan,
    ToggleApplied(JobKey),
    OpenUrl(String),
    CopyUrl(String),
    ReloadJobs,
    SaveFilters(FiltersConfig),
    SaveAnalyticsState(AnalyticsFilters, LibraryState),
    ReviewSkillSuggestion(String, SuggestionStatus),
    Quit,
}

#[derive(Debug, Clone, Copy, Default)]
struct ScanProgress {
    active: bool,
    company_count: usize,
    finished: usize,
    failed: usize,
    incomplete: usize,
}

pub struct App {
    config: Config,
    jobs: Vec<JobRecord>,
    scans: Vec<ScanReadModel>,
    sources: Vec<SourceReadModel>,
    job_facts: HashMap<JobKey, JobFacts>,
    theme: Theme,
    icons: IconSet,
    view: View,
    input_mode: InputMode,
    focus: Focus,
    navigation_index: usize,
    selected_index: usize,
    analytics_kind: SkillKind,
    hard_skill_index: usize,
    soft_skill_index: usize,
    analytics_tab: AnalyticsTab,
    market_section: MarketSection,
    library_tab: LibraryTab,
    analytics_filters: AnalyticsFilters,
    library: LibraryState,
    analytics_scans: Vec<ScanReadModel>,
    skill_suggestions: Vec<SkillSuggestion>,
    evidence_index: usize,
    active_job_count: usize,
    company_filter: Option<String>,
    preserve_selection_on_replace: bool,
    search_query: String,
    detail_scroll: u16,
    detail_scroll_max: Cell<u16>,
    narrow_details_visible: bool,
    help_visible: bool,
    setting_input: String,
    setting_error: Option<String>,
    job_list_width: Option<u16>,
    divider_dragging: bool,
    scan_progress: ScanProgress,
    hovered: Option<MouseTarget>,
    pressed: Option<MouseTarget>,
    last_list_scroll: Option<(isize, Instant)>,
}

impl App {
    pub fn new(config: Config, jobs: Vec<JobRecord>) -> Self {
        let job_facts = extract_job_facts(&jobs);
        Self::new_with_facts(config, jobs, job_facts)
    }

    pub fn new_with_facts(
        config: Config,
        jobs: Vec<JobRecord>,
        job_facts: HashMap<JobKey, JobFacts>,
    ) -> Self {
        let theme = Theme::from_config(&config.ui.theme, &config.ui.theme_overrides);
        let icons = if config.ui.unicode_icons {
            IconSet::unicode()
        } else {
            IconSet::ascii()
        };
        let active_job_count = jobs.iter().filter(|job| job.source_open).count();
        Self {
            config,
            jobs,
            scans: Vec::new(),
            sources: Vec::new(),
            job_facts,
            theme,
            icons,
            view: View::Active,
            input_mode: InputMode::Normal,
            focus: Focus::Content,
            navigation_index: 0,
            selected_index: 0,
            analytics_kind: SkillKind::Hard,
            hard_skill_index: 0,
            soft_skill_index: 0,
            analytics_tab: AnalyticsTab::Overview,
            market_section: MarketSection::Roles,
            library_tab: LibraryTab::Jobs,
            analytics_filters: AnalyticsFilters::default(),
            library: LibraryState::default(),
            analytics_scans: Vec::new(),
            skill_suggestions: Vec::new(),
            evidence_index: 0,
            active_job_count,
            company_filter: None,
            preserve_selection_on_replace: true,
            search_query: String::new(),
            detail_scroll: 0,
            detail_scroll_max: Cell::new(u16::MAX),
            narrow_details_visible: false,
            help_visible: false,
            setting_input: String::new(),
            setting_error: None,
            job_list_width: None,
            divider_dragging: false,
            scan_progress: ScanProgress::default(),
            hovered: None,
            pressed: None,
            last_list_scroll: None,
        }
    }

    pub fn replace_jobs(&mut self, jobs: Vec<JobRecord>, active_job_count: usize) {
        let job_facts = extract_job_facts(&jobs);
        self.replace_jobs_with_facts(jobs, active_job_count, job_facts);
    }

    pub fn replace_jobs_with_facts(
        &mut self,
        jobs: Vec<JobRecord>,
        active_job_count: usize,
        job_facts: HashMap<JobKey, JobFacts>,
    ) {
        self.job_facts = job_facts;
        if matches!(
            self.view,
            View::Scans | View::Sources | View::Analytics | View::Library | View::Settings
        ) {
            self.jobs = jobs;
            self.active_job_count = active_job_count;
            if self.view == View::Analytics {
                self.hard_skill_index = self.hard_skill_index.min(
                    self.skill_stats_for(SkillKind::Hard)
                        .len()
                        .saturating_sub(1),
                );
                self.soft_skill_index = self.soft_skill_index.min(
                    self.skill_stats_for(SkillKind::Soft)
                        .len()
                        .saturating_sub(1),
                );
                self.selected_index = self.analytics_skill_index(self.analytics_kind);
                self.evidence_index = self
                    .evidence_index
                    .min(self.selected_skill_evidence().len().saturating_sub(1));
            }
            return;
        }
        let selected_key = self
            .preserve_selection_on_replace
            .then(|| self.selected_job().map(|job| job.key.clone()))
            .flatten();
        let fallback_index = self.selected_index;
        self.jobs = jobs;
        self.active_job_count = active_job_count;
        self.selected_index = selected_key
            .as_ref()
            .and_then(|key| self.visible_jobs().position(|job| &job.key == key))
            .unwrap_or_else(|| fallback_index.min(self.visible_jobs().count().saturating_sub(1)));
        self.preserve_selection_on_replace = true;
    }

    pub fn replace_read_models(
        &mut self,
        scans: Vec<ScanReadModel>,
        sources: Vec<SourceReadModel>,
    ) {
        let selected_scan = (self.view == View::Scans)
            .then(|| self.scans.get(self.selected_index))
            .flatten()
            .map(|scan| (scan.run_id.clone(), scan.company_id.clone()));
        self.scans = scans;
        self.sources = sources;
        self.selected_index = selected_scan
            .as_ref()
            .and_then(|key| {
                self.scans
                    .iter()
                    .position(|scan| (&scan.run_id, &scan.company_id) == (&key.0, &key.1))
            })
            .unwrap_or_else(|| self.selected_index.min(self.item_count().saturating_sub(1)));
    }

    pub fn replace_analytics_state(
        &mut self,
        filters: AnalyticsFilters,
        library: LibraryState,
        scans: Vec<ScanReadModel>,
    ) {
        self.analytics_filters = filters;
        self.library = library;
        self.analytics_scans = scans;
        self.selected_index = self.selected_index.min(self.item_count().saturating_sub(1));
    }

    pub fn replace_skill_suggestions(&mut self, suggestions: Vec<SkillSuggestion>) {
        self.skill_suggestions = suggestions;
        self.selected_index = self.selected_index.min(self.item_count().saturating_sub(1));
    }

    pub fn analytics_filters(&self) -> &AnalyticsFilters {
        &self.analytics_filters
    }

    pub fn library(&self) -> &LibraryState {
        &self.library
    }

    pub fn analytics_tab(&self) -> AnalyticsTab {
        self.analytics_tab
    }

    pub fn market_section(&self) -> MarketSection {
        self.market_section
    }

    pub fn library_tab(&self) -> LibraryTab {
        self.library_tab
    }

    pub fn analytics_report(&self) -> AnalyticsReport {
        let mut report = AnalyticsReport::build(
            &self.jobs,
            &self.job_facts,
            &self.analytics_scans,
            &self.analytics_filters,
            &self.library,
            Utc::now(),
            self.config.analytics.minimum_cooccurrence,
        );
        let minimum = self.config.analytics.minimum_skill_occurrence;
        let maximum = self.config.analytics.maximum_skills;
        report
            .hard_skills
            .retain(|item| item.metric.current_count >= minimum);
        report
            .soft_skills
            .retain(|item| item.metric.current_count >= minimum);
        report
            .recommendations
            .retain(|item| item.demand_count >= minimum);
        report.hard_skills.truncate(maximum);
        report.soft_skills.truncate(maximum);
        report.recommendations.truncate(maximum);
        report
    }

    pub fn market_rows(&self) -> Vec<MetricRow> {
        let report = self.analytics_report();
        match self.market_section {
            MarketSection::Roles => report.roles,
            MarketSection::Seniority => report.seniority,
            MarketSection::Experience => report.experience,
            MarketSection::Work => report
                .work
                .into_iter()
                .chain(report.employment)
                .chain(report.education)
                .collect(),
            MarketSection::Companies => report.companies,
        }
    }

    pub fn analytics_evidence_jobs(&self) -> Vec<&JobRecord> {
        let report = self.analytics_report();
        let selected_skill = match self.analytics_tab {
            AnalyticsTab::Overview => report
                .recommendations
                .get(self.selected_index)
                .map(|item| item.skill.as_str()),
            AnalyticsTab::Skills => {
                let rows = match self.analytics_kind {
                    SkillKind::Hard => &report.hard_skills,
                    SkillKind::Soft => &report.soft_skills,
                };
                rows.get(self.selected_index)
                    .map(|item| item.metric.name.as_str())
            }
            AnalyticsTab::Stacks | AnalyticsTab::Market => None,
        };
        let selected_stack = (self.analytics_tab == AnalyticsTab::Stacks)
            .then(|| report.stacks.get(self.selected_index))
            .flatten();
        let selected_market = (self.analytics_tab == AnalyticsTab::Market)
            .then(|| self.market_rows().get(self.selected_index).cloned())
            .flatten();
        self.jobs
            .iter()
            .filter(|job| job.source_open)
            .filter(|job| {
                self.job_facts.get(&job.key).is_some_and(|facts| {
                    insights::matches_filters(job, facts, &self.analytics_filters)
                })
            })
            .filter(|job| {
                let Some(facts) = self.job_facts.get(&job.key) else {
                    return false;
                };
                if let Some(skill) = selected_skill {
                    return facts.skills.contains_key(skill);
                }
                if let Some(stack) = selected_stack {
                    return stack
                        .key
                        .0
                        .iter()
                        .all(|skill| facts.skills.contains_key(skill));
                }
                let Some(metric) = &selected_market else {
                    return false;
                };
                match self.market_section {
                    MarketSection::Roles => facts.role_family == metric.name,
                    MarketSection::Seniority => {
                        insights::seniority_name(facts.seniority) == metric.name
                    }
                    MarketSection::Experience => insights::experience_bucket(facts) == metric.name,
                    MarketSection::Work => {
                        insights::work_mode_name(facts.work_mode) == metric.name
                            || job.classified.observed.employment_type.as_deref()
                                == Some(metric.name.as_str())
                            || facts.education.as_ref().map_or(
                                metric.name == "Not stated",
                                |education| {
                                    if education.allows_equivalent_experience {
                                        metric.name == "Degree or equivalent experience"
                                    } else {
                                        metric.name == "Degree stated"
                                    }
                                },
                            )
                    }
                    MarketSection::Companies => job.key.company_id == metric.name,
                }
            })
            .collect()
    }

    pub fn analytics_evidence_text(&self, job: &JobRecord) -> String {
        let Some(facts) = self.job_facts.get(&job.key) else {
            return "No extracted evidence".to_owned();
        };
        let report = self.analytics_report();
        let skill = match self.analytics_tab {
            AnalyticsTab::Overview => report
                .recommendations
                .get(self.selected_index)
                .map(|item| item.skill.as_str()),
            AnalyticsTab::Skills => match self.analytics_kind {
                SkillKind::Hard => report.hard_skills.get(self.selected_index),
                SkillKind::Soft => report.soft_skills.get(self.selected_index),
            }
            .map(|item| item.metric.name.as_str()),
            AnalyticsTab::Stacks | AnalyticsTab::Market => None,
        };
        if let Some(evidence) = skill.and_then(|name| facts.skills.get(name)) {
            return evidence.context.clone();
        }
        if self.analytics_tab == AnalyticsTab::Stacks
            && let Some(stack) = report.stacks.get(self.selected_index)
        {
            return stack
                .key
                .0
                .iter()
                .filter_map(|name| facts.skills.get(name))
                .map(|evidence| evidence.context.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
        }
        match self.market_section {
            MarketSection::Roles | MarketSection::Seniority => {
                format!("Job title: {}", job.classified.observed.title)
            }
            MarketSection::Experience => facts
                .experience
                .first()
                .map(|fact| fact.evidence.clone())
                .unwrap_or_else(|| "Experience not stated".to_owned()),
            MarketSection::Work => facts
                .education
                .as_ref()
                .map(|fact| fact.evidence.clone())
                .or_else(|| job.classified.observed.employment_type.clone())
                .unwrap_or_else(|| insights::work_mode_name(facts.work_mode).to_owned()),
            MarketSection::Companies => {
                format!("Company: {}", self.company_name(&job.key.company_id))
            }
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AppCommand {
        self.handle_key_with_width(key, u16::MAX)
    }

    pub fn handle_key_with_width(&mut self, key: KeyEvent, width: u16) -> AppCommand {
        if key.kind == KeyEventKind::Release {
            return AppCommand::None;
        }
        if self.input_mode == InputMode::Search {
            return self.handle_search_key(key.code);
        }
        if self.input_mode == InputMode::Setting {
            return self.handle_setting_key(key.code);
        }
        if self.help_visible && key.code == KeyCode::Esc {
            self.help_visible = false;
            return AppCommand::None;
        }
        if self.narrow_details_visible && key.code == KeyCode::Esc {
            self.narrow_details_visible = false;
            return AppCommand::None;
        }
        if self.focus == Focus::Navigation {
            return self.handle_navigation_key(key.code);
        }
        if self.view == View::Analytics
            && let Some(command) = self.handle_analytics_key(key.code)
        {
            return command;
        }
        if self.view == View::Library
            && let Some(command) = self.handle_library_key(key.code)
        {
            return command;
        }

        match key.code {
            KeyCode::Left
                if self.view == View::Analytics && self.analytics_tab == AnalyticsTab::Skills =>
            {
                self.select_analytics_kind(SkillKind::Hard)
            }
            KeyCode::Right
                if self.view == View::Analytics && self.analytics_tab == AnalyticsTab::Skills =>
            {
                self.select_analytics_kind(SkillKind::Soft)
            }
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Char('J') if self.view == View::Analytics => self.move_evidence_selection(1),
            KeyCode::Char('K') if self.view == View::Analytics => self.move_evidence_selection(-1),
            KeyCode::Char('J') => self.scroll_details(1),
            KeyCode::Char('K') => self.scroll_details(-1),
            KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc => {
                self.focus = Focus::Navigation;
                self.navigation_index = view_index(self.view);
            }
            KeyCode::Enter if self.view == View::Settings => {
                self.start_setting_edit();
            }
            KeyCode::Enter if width < 80 => {
                self.narrow_details_visible = !self.narrow_details_visible;
            }
            KeyCode::Enter => return self.open_selected(),
            KeyCode::Char('*') => {
                if let Some(key) = self.selected_job().map(|job| job.key.clone()) {
                    if !self.library.jobs.remove(&key) {
                        self.library.jobs.insert(key);
                    }
                    return self.save_analytics_state_command();
                }
            }
            KeyCode::Char(character) => return self.handle_action_key(character),
            _ => {}
        }
        AppCommand::None
    }

    fn handle_analytics_key(&mut self, code: KeyCode) -> Option<AppCommand> {
        match code {
            KeyCode::Char('[') => {
                self.analytics_tab =
                    ANALYTICS_TABS[analytics_tab_index(self.analytics_tab).saturating_sub(1)];
                self.reset_analytics_selection();
                Some(AppCommand::None)
            }
            KeyCode::Char(']') => {
                self.analytics_tab = ANALYTICS_TABS
                    [(analytics_tab_index(self.analytics_tab) + 1).min(ANALYTICS_TABS.len() - 1)];
                self.reset_analytics_selection();
                Some(AppCommand::None)
            }
            KeyCode::Char(value @ '1'..='4') => {
                self.analytics_tab =
                    ANALYTICS_TABS[(value as usize - '1' as usize).min(ANALYTICS_TABS.len() - 1)];
                self.reset_analytics_selection();
                Some(AppCommand::None)
            }
            KeyCode::Char('t') => {
                self.analytics_filters.window_days = match self.analytics_filters.window_days {
                    7 => 30,
                    30 => 90,
                    _ => 7,
                };
                self.selected_index = 0;
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('+') | KeyCode::Char('=') => {
                self.analytics_filters.window_days =
                    self.analytics_filters.window_days.saturating_add(1);
                self.reset_analytics_selection();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('-') => {
                self.analytics_filters.window_days =
                    self.analytics_filters.window_days.saturating_sub(1).max(1);
                self.reset_analytics_selection();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('C') => {
                self.cycle_analytics_company();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('R') => {
                self.cycle_analytics_role();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('S') => {
                self.analytics_filters.seniority = cycle_option(
                    self.analytics_filters.seniority,
                    &[
                        Seniority::Intern,
                        Seniority::Junior,
                        Seniority::Mid,
                        Seniority::Senior,
                        Seniority::Lead,
                        Seniority::Manager,
                        Seniority::Unknown,
                    ],
                );
                self.reset_analytics_selection();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('W') => {
                self.analytics_filters.work_mode = cycle_option(
                    self.analytics_filters.work_mode,
                    &[
                        WorkMode::Remote,
                        WorkMode::Hybrid,
                        WorkMode::OnSite,
                        WorkMode::Unknown,
                    ],
                );
                self.reset_analytics_selection();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('x') => {
                let window_days = self.analytics_filters.window_days;
                self.analytics_filters = AnalyticsFilters {
                    window_days,
                    ..AnalyticsFilters::default()
                };
                self.reset_analytics_selection();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('*') => {
                self.toggle_selected_analytics_item();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('m') if self.analytics_tab == AnalyticsTab::Skills => {
                self.cycle_selected_skill_status();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Left if self.analytics_tab == AnalyticsTab::Skills => {
                self.select_analytics_kind(SkillKind::Hard);
                Some(AppCommand::None)
            }
            KeyCode::Right if self.analytics_tab == AnalyticsTab::Skills => {
                self.select_analytics_kind(SkillKind::Soft);
                Some(AppCommand::None)
            }
            KeyCode::Left if self.analytics_tab == AnalyticsTab::Market => {
                self.market_section =
                    MARKET_SECTIONS[market_section_index(self.market_section).saturating_sub(1)];
                self.selected_index = 0;
                Some(AppCommand::None)
            }
            KeyCode::Right if self.analytics_tab == AnalyticsTab::Market => {
                self.market_section = MARKET_SECTIONS[(market_section_index(self.market_section)
                    + 1)
                .min(MARKET_SECTIONS.len() - 1)];
                self.selected_index = 0;
                Some(AppCommand::None)
            }
            _ => None,
        }
    }

    fn handle_library_key(&mut self, code: KeyCode) -> Option<AppCommand> {
        match code {
            KeyCode::Char('[') => {
                self.library_tab =
                    LIBRARY_TABS[library_tab_index(self.library_tab).saturating_sub(1)];
                self.selected_index = 0;
                Some(AppCommand::None)
            }
            KeyCode::Char(']') => {
                self.library_tab = LIBRARY_TABS
                    [(library_tab_index(self.library_tab) + 1).min(LIBRARY_TABS.len() - 1)];
                self.selected_index = 0;
                Some(AppCommand::None)
            }
            KeyCode::Char(value @ '1'..='5') => {
                self.library_tab =
                    LIBRARY_TABS[(value as usize - '1' as usize).min(LIBRARY_TABS.len() - 1)];
                self.selected_index = 0;
                Some(AppCommand::None)
            }
            KeyCode::Char('*') => {
                self.remove_selected_library_item();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('m') if self.library_tab == LibraryTab::Skills => {
                self.cycle_selected_library_skill_status();
                Some(self.save_analytics_state_command())
            }
            KeyCode::Char('a') if self.library_tab == LibraryTab::Skills => Some(
                self.selected_pending_suggestion()
                    .map_or(AppCommand::None, |item| {
                        AppCommand::ReviewSkillSuggestion(
                            item.name.clone(),
                            SuggestionStatus::Approved,
                        )
                    }),
            ),
            KeyCode::Char('d') if self.library_tab == LibraryTab::Skills => Some(
                self.selected_pending_suggestion()
                    .map_or(AppCommand::None, |item| {
                        AppCommand::ReviewSkillSuggestion(
                            item.name.clone(),
                            SuggestionStatus::Rejected,
                        )
                    }),
            ),
            KeyCode::Char('m') if self.library_tab == LibraryTab::Roles => {
                if let Some((role, target)) = self
                    .library
                    .roles
                    .iter()
                    .nth(self.selected_index)
                    .map(|(role, target)| (role.clone(), *target))
                {
                    self.library.roles.insert(role, !target);
                }
                Some(self.save_analytics_state_command())
            }
            _ => None,
        }
    }

    pub fn handle_mouse(&mut self, mouse: MouseEvent, width: u16, height: u16) -> AppCommand {
        if self.divider_dragging {
            match mouse.kind {
                MouseEventKind::Drag(MouseButton::Left) => {
                    self.resize_job_panes(mouse.column, width);
                    self.hovered = Some(MouseTarget::Divider);
                    return AppCommand::None;
                }
                MouseEventKind::Up(MouseButton::Left) => {
                    self.resize_job_panes(mouse.column, width);
                    self.divider_dragging = false;
                    self.pressed = None;
                    self.hovered = self.mouse_target(mouse.column, mouse.row, width, height);
                    return AppCommand::None;
                }
                _ => {}
            }
        }
        let target = self.mouse_target(mouse.column, mouse.row, width, height);
        self.hovered = target;
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                self.pressed = target;
                self.divider_dragging = target == Some(MouseTarget::Divider);
                self.focus_mouse_target(target);
            }
            MouseEventKind::Up(MouseButton::Left) => {
                let pressed = self.pressed.take();
                if pressed == target {
                    return self.activate_mouse_target(target);
                }
            }
            MouseEventKind::ScrollUp => self.scroll_mouse_target(target, -1),
            MouseEventKind::ScrollDown => self.scroll_mouse_target(target, 1),
            MouseEventKind::Drag(MouseButton::Left) if target != self.pressed => {
                self.pressed = None;
            }
            _ => {}
        }
        AppCommand::None
    }

    pub fn clear_mouse_state(&mut self) {
        self.hovered = None;
        self.pressed = None;
        self.divider_dragging = false;
    }

    pub fn handle_scan_event(&mut self, event: ScanEvent) {
        match event {
            ScanEvent::RunStarted { company_count, .. } => {
                self.scan_progress = ScanProgress {
                    active: true,
                    company_count,
                    ..ScanProgress::default()
                };
            }
            ScanEvent::CompanyStarted { .. } | ScanEvent::Started { .. } => {}
            ScanEvent::CompanyCompleted { .. } => {
                self.scan_progress.finished += 1;
            }
            ScanEvent::CompanyFailed { .. } | ScanEvent::Failed { .. } => {
                self.scan_progress.finished += 1;
                self.scan_progress.failed += 1;
            }
            ScanEvent::CompanyIncomplete { .. } => {
                self.scan_progress.finished += 1;
                self.scan_progress.incomplete += 1;
            }
            ScanEvent::Completed { source_scan, .. } => {
                self.scan_progress.finished += 1;
                match source_scan {
                    SourceScan::Complete { .. } => {}
                    SourceScan::Incomplete { .. } => {
                        self.scan_progress.incomplete += 1;
                    }
                }
            }
            ScanEvent::RunFinished {
                failed, incomplete, ..
            } => {
                self.scan_progress.active = false;
                self.scan_progress.failed = failed;
                self.scan_progress.incomplete = incomplete;
            }
        }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn jobs(&self) -> &[JobRecord] {
        &self.jobs
    }

    pub fn scans(&self) -> &[ScanReadModel] {
        &self.scans
    }

    pub fn sources(&self) -> &[SourceReadModel] {
        &self.sources
    }

    pub fn visible_jobs(&self) -> impl Iterator<Item = &JobRecord> {
        self.jobs.iter().filter(move |job| {
            let in_view = match self.view {
                View::Active => job.source_open,
                View::New => job.source_open && self.is_job_new(job),
                View::Applied => job.applied_at.is_some(),
                View::History => !job.source_open || job.reopened_at.is_some(),
                View::Scans | View::Sources | View::Analytics | View::Library | View::Settings => {
                    false
                }
            };
            let in_company = self
                .company_filter
                .as_deref()
                .is_none_or(|company_id| job.key.company_id == company_id);
            in_view && in_company && self.matches_search(job)
        })
    }

    pub fn theme(&self) -> Theme {
        self.theme
    }

    pub fn icons(&self) -> IconSet {
        self.icons
    }

    pub fn view(&self) -> View {
        self.view
    }

    pub fn input_mode(&self) -> InputMode {
        self.input_mode
    }

    pub fn focus(&self) -> Focus {
        self.focus
    }

    pub fn navigation_index(&self) -> usize {
        if self.focus == Focus::Navigation {
            self.navigation_index
        } else {
            view_index(self.view)
        }
    }

    pub fn setting_input(&self) -> &str {
        &self.setting_input
    }

    pub fn setting(&self) -> Setting {
        SETTINGS[self.selected_index.min(SETTINGS.len() - 1)]
    }

    pub fn setting_error(&self) -> Option<&str> {
        self.setting_error.as_deref()
    }

    pub fn is_job_new(&self, job: &JobRecord) -> bool {
        let now = Utc::now();
        job.classified
            .observed
            .published_at
            .is_some_and(|published| {
                published <= now
                    && published
                        >= now - Duration::days(i64::from(self.config.filters.new_job_max_age_days))
            })
    }

    pub fn apply_filters(&mut self, filters: FiltersConfig) {
        self.config.filters = filters;
    }

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn evidence_index(&self) -> usize {
        self.evidence_index
    }

    pub fn selected_job(&self) -> Option<&JobRecord> {
        if self.view == View::Library && self.library_tab == LibraryTab::Jobs {
            return self.library_jobs().get(self.selected_index).copied();
        }
        self.visible_jobs().nth(self.selected_index)
    }

    pub fn library_jobs(&self) -> Vec<&JobRecord> {
        self.library
            .jobs
            .iter()
            .filter_map(|key| self.jobs.iter().find(|job| &job.key == key))
            .collect()
    }

    pub fn library_skills(&self) -> Vec<(&str, Option<SkillStatus>)> {
        self.library
            .skills
            .iter()
            .map(|(name, status)| (name.as_str(), *status))
            .collect()
    }

    pub fn pending_skill_suggestions(&self) -> Vec<&SkillSuggestion> {
        self.skill_suggestions
            .iter()
            .filter(|item| item.status == SuggestionStatus::Pending)
            .collect()
    }

    pub fn library_stacks(&self) -> Vec<StackKey> {
        self.library.stacks.iter().cloned().map(StackKey).collect()
    }

    pub fn library_roles(&self) -> Vec<(&str, bool)> {
        self.library
            .roles
            .iter()
            .map(|(name, target)| (name.as_str(), *target))
            .collect()
    }

    pub fn library_companies(&self) -> Vec<&str> {
        self.library.companies.iter().map(String::as_str).collect()
    }

    pub fn analytics_job_count(&self) -> usize {
        self.analytics_jobs().count()
    }

    pub fn skill_stats(&self) -> Vec<SkillStat> {
        let mut stats = self.all_skill_stats();
        stats.truncate(self.config.analytics.maximum_skills);
        stats
    }

    pub fn skill_stats_for(&self, kind: SkillKind) -> Vec<SkillStat> {
        self.all_skill_stats()
            .into_iter()
            .filter(|skill| skill.kind == kind)
            .take(self.config.analytics.maximum_skills)
            .collect()
    }

    fn all_skill_stats(&self) -> Vec<SkillStat> {
        let jobs = self.analytics_jobs().collect::<Vec<_>>();
        let names = jobs
            .iter()
            .filter_map(|job| self.job_facts.get(&job.key))
            .flat_map(|facts| facts.skills.keys().cloned())
            .collect::<std::collections::BTreeSet<_>>();
        let mut stats = names
            .into_iter()
            .filter_map(|name| {
                let job_count = jobs
                    .iter()
                    .filter(|job| self.job_has_skill(job, &name))
                    .count();
                let kind = jobs.iter().find_map(|job| {
                    self.job_facts
                        .get(&job.key)?
                        .skills
                        .get(&name)
                        .map(|evidence| evidence.kind)
                })?;
                (job_count >= self.config.analytics.minimum_skill_occurrence).then_some(SkillStat {
                    kind,
                    name,
                    job_count,
                })
            })
            .collect::<Vec<_>>();
        stats.sort_by(|left, right| {
            right
                .job_count
                .cmp(&left.job_count)
                .then_with(|| left.name.cmp(&right.name))
        });
        stats
    }

    pub fn analytics_skill_kind(&self) -> SkillKind {
        self.analytics_kind
    }

    pub fn analytics_skill_index(&self, kind: SkillKind) -> usize {
        match kind {
            SkillKind::Hard => self.hard_skill_index,
            SkillKind::Soft => self.soft_skill_index,
        }
    }

    pub fn analytics_skill_job_count(&self, kind: SkillKind) -> usize {
        self.analytics_jobs()
            .filter(|job| {
                self.job_facts.get(&job.key).is_some_and(|facts| {
                    facts.skills.values().any(|evidence| evidence.kind == kind)
                })
            })
            .count()
    }

    fn selected_skill(&self) -> Option<SkillStat> {
        self.skill_stats_for(self.analytics_kind)
            .get(self.selected_index)
            .cloned()
    }

    pub fn selected_skill_jobs(&self) -> Vec<&JobRecord> {
        let Some(skill) = self.selected_skill() else {
            return Vec::new();
        };
        self.analytics_jobs()
            .filter(|job| self.job_has_skill(job, &skill.name))
            .collect()
    }

    pub fn selected_skill_evidence(&self) -> Vec<(&JobRecord, &SkillEvidence)> {
        let Some(skill) = self.selected_skill() else {
            return Vec::new();
        };
        self.analytics_jobs()
            .filter_map(|job| {
                self.job_facts
                    .get(&job.key)?
                    .skills
                    .get(&skill.name)
                    .map(|evidence| (job, evidence))
            })
            .collect()
    }

    pub fn related_skill_stats(&self) -> Vec<RelatedSkillStat> {
        let Some(selected) = self.selected_skill() else {
            return Vec::new();
        };
        let jobs = self.analytics_jobs().collect::<Vec<_>>();
        let selected_count = jobs
            .iter()
            .filter(|job| self.job_has_skill(job, &selected.name))
            .count();
        let minimum = self.config.analytics.minimum_cooccurrence;
        let mut related = self
            .all_skill_stats()
            .into_iter()
            .map(|skill| skill.name)
            .filter(|name| name != &selected.name)
            .filter_map(|name| {
                let other_count = jobs
                    .iter()
                    .filter(|job| self.job_has_skill(job, &name))
                    .count();
                let job_count = jobs
                    .iter()
                    .filter(|job| {
                        self.job_has_skill(job, &selected.name) && self.job_has_skill(job, &name)
                    })
                    .count();
                let union_count = selected_count + other_count - job_count;
                (job_count >= minimum && union_count > 0).then(|| RelatedSkillStat {
                    name: name.clone(),
                    job_count,
                    union_count,
                    jaccard_per_mille: job_count * 1_000 / union_count,
                })
            })
            .collect::<Vec<_>>();
        related.sort_by(|left, right| {
            right
                .jaccard_per_mille
                .cmp(&left.jaccard_per_mille)
                .then_with(|| right.job_count.cmp(&left.job_count))
                .then_with(|| left.name.cmp(&right.name))
        });
        related
    }

    pub fn work_mode_stats(&self) -> Vec<CategoryStat> {
        let mut counts = [0; 4];
        for job in self.analytics_jobs() {
            let index = match self.job_facts.get(&job.key).map(|facts| facts.work_mode) {
                Some(WorkMode::Remote) => 0,
                Some(WorkMode::Hybrid) => 1,
                Some(WorkMode::OnSite) => 2,
                Some(WorkMode::Unknown) | None => 3,
            };
            counts[index] += 1;
        }
        ["Remote", "Hybrid", "On-site", "Unknown"]
            .into_iter()
            .zip(counts)
            .map(|(name, job_count)| CategoryStat {
                name: name.into(),
                job_count,
            })
            .collect()
    }

    pub fn seniority_stats(&self) -> Vec<CategoryStat> {
        let mut counts = [0; 7];
        for job in self.analytics_jobs() {
            let index = match self.job_facts.get(&job.key).map(|facts| facts.seniority) {
                Some(Seniority::Intern) => 0,
                Some(Seniority::Junior) => 1,
                Some(Seniority::Mid) => 2,
                Some(Seniority::Senior) => 3,
                Some(Seniority::Lead) => 4,
                Some(Seniority::Manager) => 5,
                Some(Seniority::Unknown) | None => 6,
            };
            counts[index] += 1;
        }
        [
            "Intern", "Junior", "Mid", "Senior", "Lead", "Manager", "Unknown",
        ]
        .into_iter()
        .zip(counts)
        .map(|(name, job_count)| CategoryStat {
            name: name.into(),
            job_count,
        })
        .collect()
    }

    pub fn analytics_coverage(&self) -> AnalyticsCoverage {
        let jobs = self
            .analytics_jobs()
            .filter(|job| {
                self.view != View::Analytics
                    || self.job_facts.get(&job.key).is_some_and(|facts| {
                        insights::matches_filters(job, facts, &self.analytics_filters)
                    })
            })
            .collect::<Vec<_>>();
        let relevant_source = |source: &&SourceReadModel| {
            source.enabled
                && self
                    .analytics_filters
                    .company
                    .as_deref()
                    .is_none_or(|company| source.company_id == company)
        };
        let mut coverage = AnalyticsCoverage {
            total: jobs.len(),
            enabled_sources: self.sources.iter().filter(relevant_source).count(),
            healthy_sources: self
                .sources
                .iter()
                .filter(relevant_source)
                .filter(|source| source.health == SourceHealth::Healthy)
                .count(),
            ..AnalyticsCoverage::default()
        };
        for job in jobs {
            let observed = &job.classified.observed;
            coverage.descriptions += usize::from(!observed.description.trim().is_empty());
            coverage.published_dates += usize::from(observed.published_at.is_some());
            if let Some(facts) = self.job_facts.get(&job.key) {
                coverage.work_mode += usize::from(facts.work_mode != WorkMode::Unknown);
                coverage.seniority += usize::from(facts.seniority != Seniority::Unknown);
                coverage.experience += usize::from(!facts.experience.is_empty());
                coverage.education += usize::from(facts.education.is_some());
                coverage.employment_type += usize::from(facts.employment_type_known);
            }
        }
        coverage
    }

    pub fn active_job_count(&self) -> usize {
        self.active_job_count
    }

    pub fn company_filter(&self) -> Option<&str> {
        self.company_filter.as_deref()
    }

    pub fn company_filter_label(&self) -> &str {
        self.company_filter
            .as_deref()
            .and_then(|id| {
                self.config
                    .companies
                    .iter()
                    .find(|company| company.id == id)
                    .map(|company| company.name.as_str())
            })
            .unwrap_or("All")
    }

    pub fn company_name<'a>(&'a self, company_id: &'a str) -> &'a str {
        self.config
            .companies
            .iter()
            .find(|company| company.id == company_id)
            .map_or(company_id, |company| company.name.as_str())
    }

    pub fn filter_label(&self) -> &str {
        if self.company_filter.is_some() {
            self.company_filter_label()
        } else {
            match self.view {
                View::New => "New",
                View::Applied => "Applied",
                _ => "All",
            }
        }
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn detail_scroll(&self) -> u16 {
        self.detail_scroll.min(self.detail_scroll_max.get())
    }

    pub(crate) fn set_detail_scroll_max(&self, maximum: u16) {
        self.detail_scroll_max.set(maximum);
    }

    pub fn narrow_details_visible(&self) -> bool {
        self.narrow_details_visible
    }

    pub fn help_visible(&self) -> bool {
        self.help_visible
    }

    pub fn hovered(&self, target: MouseTarget) -> bool {
        self.hovered == Some(target)
    }

    pub fn pressed(&self, target: MouseTarget) -> bool {
        self.pressed == Some(target)
    }

    pub(crate) fn job_panes(&self, area: Rect) -> Option<(Rect, Rect)> {
        let (origin, available, default_width) = job_pane_geometry(area)?;
        let list_width = self
            .job_list_width
            .unwrap_or(default_width)
            .clamp(MIN_JOB_LIST_WIDTH, available - MIN_JOB_DETAILS_WIDTH);
        Some((
            Rect::new(origin, area.y, list_width, area.height),
            Rect::new(
                origin + list_width,
                area.y,
                available - list_width,
                area.height,
            ),
        ))
    }

    pub fn footer_status(&self) -> String {
        if self.scan_progress.active {
            format!(
                "SCANNING {}/{}",
                self.scan_progress.finished, self.scan_progress.company_count
            )
        } else {
            let failed = self
                .sources
                .iter()
                .filter(|source| source.enabled && source.health == SourceHealth::Failed)
                .count()
                .max(self.scan_progress.failed);
            let incomplete = self
                .sources
                .iter()
                .filter(|source| source.enabled && source.health == SourceHealth::Incomplete)
                .count()
                .max(self.scan_progress.incomplete);
            if failed > 0 {
                format!("FAILED {failed}")
            } else if incomplete > 0 {
                format!("INCOMPLETE {incomplete}")
            } else {
                "OK".to_owned()
            }
        }
    }

    fn handle_search_key(&mut self, code: KeyCode) -> AppCommand {
        match code {
            KeyCode::Esc => {
                self.search_query.clear();
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Enter => self.input_mode = InputMode::Normal,
            KeyCode::Backspace => {
                self.search_query.pop();
                self.clamp_selection();
            }
            KeyCode::Char(character) => {
                self.search_query.push(character);
                self.clamp_selection();
            }
            _ => {}
        }
        AppCommand::None
    }

    fn mouse_target(&self, column: u16, row: u16, width: u16, height: u16) -> Option<MouseTarget> {
        let content = Rect::new(0, 0, width, height.saturating_sub(1));
        if self.help_visible && content.contains((column, row).into()) {
            return Some(MouseTarget::Help);
        }
        if row == height.saturating_sub(1) {
            let help_width =
                ratatui::text::Line::from(format!("{} help", self.config.keybindings.help)).width()
                    as u16;
            return (column >= width.saturating_sub(help_width)).then_some(MouseTarget::Help);
        }

        let navigation = if self.focus == Focus::Navigation && width < 120 {
            Some(content)
        } else if width >= 120 {
            Some(Rect::new(0, 0, 22.min(width), content.height))
        } else {
            None
        };
        if let Some(area) = navigation
            && let Some(index) = item_at(column, row, area, 1, VIEWS.len(), self.navigation_index)
        {
            return Some(MouseTarget::Navigation(index));
        }
        if self.focus == Focus::Navigation && width < 120 {
            return None;
        }

        match self.view {
            View::Settings => {
                let area = if width >= 120 {
                    Rect::new(22, 0, width.saturating_sub(22), content.height)
                } else {
                    content
                };
                if self.input_mode == InputMode::Setting {
                    area.contains((column, row).into())
                        .then_some(MouseTarget::Setting(self.selected_index))
                } else {
                    item_at(column, row, area, 2, SETTINGS.len(), self.selected_index)
                        .map(MouseTarget::Setting)
                }
            }
            View::Scans | View::Sources => {
                if width < 80 && self.narrow_details_visible {
                    return content
                        .contains((column, row).into())
                        .then_some(MouseTarget::Details);
                }
                let area = if width >= 120 {
                    Rect::new(22, 0, width.saturating_sub(22), content.height)
                } else {
                    content
                };
                if self.view == View::Sources {
                    table_item_at(column, row, area, self.item_count(), self.selected_index)
                        .map(MouseTarget::Item)
                } else {
                    item_at(column, row, area, 2, self.item_count(), self.selected_index)
                        .map(MouseTarget::Item)
                }
            }
            View::Analytics => {
                let surface = if width >= 120 {
                    Rect::new(22, 0, width.saturating_sub(22), content.height)
                } else {
                    content
                };
                self.analytics_surface_target(column, row, surface)
            }
            View::Library => {
                let surface = if width >= 120 {
                    Rect::new(22, 0, width.saturating_sub(22), content.height)
                } else {
                    content
                };
                self.library_surface_target(column, row, surface)
            }
            _ if width >= 120 => {
                let (list, details) = self.job_panes(content).expect("wide job panes");
                if column == list.right().saturating_sub(1) && list.contains((column, row).into()) {
                    return Some(MouseTarget::Divider);
                }
                item_at(column, row, list, 2, self.item_count(), self.selected_index)
                    .map(MouseTarget::Item)
                    .or_else(|| {
                        details
                            .contains((column, row).into())
                            .then_some(MouseTarget::Details)
                    })
            }
            _ if width >= 80 => {
                let (list, details) = self.job_panes(content).expect("medium job panes");
                if column == list.right().saturating_sub(1) && list.contains((column, row).into()) {
                    return Some(MouseTarget::Divider);
                }
                item_at(column, row, list, 2, self.item_count(), self.selected_index)
                    .map(MouseTarget::Item)
                    .or_else(|| {
                        details
                            .contains((column, row).into())
                            .then_some(MouseTarget::Details)
                    })
            }
            _ if self.narrow_details_visible => content
                .contains((column, row).into())
                .then_some(MouseTarget::Details),
            _ => item_at(
                column,
                row,
                content,
                2,
                self.item_count(),
                self.selected_index,
            )
            .map(MouseTarget::Item),
        }
    }

    fn focus_mouse_target(&mut self, target: Option<MouseTarget>) {
        match target {
            Some(MouseTarget::Navigation(index)) => {
                self.focus = Focus::Navigation;
                self.navigation_index = index;
            }
            Some(MouseTarget::AnalyticsTab(index)) => {
                self.focus = Focus::Content;
                self.analytics_tab = ANALYTICS_TABS[index.min(ANALYTICS_TABS.len() - 1)];
                self.reset_analytics_selection();
            }
            Some(MouseTarget::MarketSection(index)) => {
                self.focus = Focus::Content;
                self.market_section = MARKET_SECTIONS[index.min(MARKET_SECTIONS.len() - 1)];
                self.reset_analytics_selection();
            }
            Some(MouseTarget::LibraryTab(index)) => {
                self.focus = Focus::Content;
                self.library_tab = LIBRARY_TABS[index.min(LIBRARY_TABS.len() - 1)];
                self.selected_index = 0;
            }
            Some(MouseTarget::Item(index)) => {
                self.focus = Focus::Content;
                if self.selected_index != index {
                    self.selected_index = index;
                    self.reset_detail_scroll();
                }
            }
            Some(MouseTarget::HardSkill(index)) => {
                self.select_analytics_kind(SkillKind::Hard);
                self.selected_index = index;
                self.hard_skill_index = index;
                self.reset_detail_scroll();
            }
            Some(MouseTarget::SoftSkill(index)) => {
                self.select_analytics_kind(SkillKind::Soft);
                self.selected_index = index;
                self.soft_skill_index = index;
                self.reset_detail_scroll();
            }
            Some(MouseTarget::Evidence(index)) => {
                self.focus = Focus::Content;
                self.evidence_index = index;
            }
            Some(MouseTarget::Setting(index)) => {
                self.focus = Focus::Content;
                self.selected_index = index;
            }
            Some(MouseTarget::Details) => self.focus = Focus::Content,
            Some(MouseTarget::Divider | MouseTarget::Help) | None => {}
        }
    }

    fn activate_mouse_target(&mut self, target: Option<MouseTarget>) -> AppCommand {
        match target {
            Some(MouseTarget::Navigation(_)) => self.activate_navigation(),
            Some(
                MouseTarget::AnalyticsTab(_)
                | MouseTarget::MarketSection(_)
                | MouseTarget::LibraryTab(_),
            ) => AppCommand::None,
            Some(MouseTarget::Setting(_)) if self.input_mode == InputMode::Normal => {
                self.start_setting_edit();
                AppCommand::None
            }
            Some(MouseTarget::Help) => {
                self.help_visible = !self.help_visible;
                self.clear_mouse_state();
                AppCommand::None
            }
            Some(MouseTarget::Divider) => AppCommand::None,
            Some(MouseTarget::Evidence(_)) => self.open_selected_evidence(),
            _ => AppCommand::None,
        }
    }

    fn scroll_mouse_target(&mut self, target: Option<MouseTarget>, direction: isize) {
        match target {
            Some(MouseTarget::Navigation(_)) => {
                self.focus = Focus::Navigation;
                self.navigation_index = if direction < 0 {
                    self.navigation_index.saturating_sub(1)
                } else {
                    (self.navigation_index + 1).min(VIEWS.len() - 1)
                };
            }
            Some(
                MouseTarget::AnalyticsTab(_)
                | MouseTarget::MarketSection(_)
                | MouseTarget::LibraryTab(_),
            ) => {}
            Some(MouseTarget::Item(_)) => {
                if !self.accept_list_scroll(direction) {
                    return;
                }
                self.focus = Focus::Content;
                self.move_selection(direction);
            }
            Some(MouseTarget::HardSkill(_)) => {
                if !self.accept_list_scroll(direction) {
                    return;
                }
                self.select_analytics_kind(SkillKind::Hard);
                self.move_selection(direction);
            }
            Some(MouseTarget::SoftSkill(_)) => {
                if !self.accept_list_scroll(direction) {
                    return;
                }
                self.select_analytics_kind(SkillKind::Soft);
                self.move_selection(direction);
            }
            Some(MouseTarget::Evidence(_)) => {
                if !self.accept_list_scroll(direction) {
                    return;
                }
                self.focus = Focus::Content;
                self.move_evidence_selection(direction);
            }
            Some(MouseTarget::Details) => {
                self.focus = Focus::Content;
                self.scroll_details(direction * 3);
            }
            Some(MouseTarget::Divider) => {}
            Some(MouseTarget::Setting(_)) if self.accept_list_scroll(direction) => {
                self.move_selection(direction)
            }
            Some(MouseTarget::Setting(_)) => {}
            Some(MouseTarget::Help) | None => {}
        }
    }

    fn handle_action_key(&mut self, character: char) -> AppCommand {
        let key = character.to_string();
        let keys = &self.config.keybindings;
        if key == keys.scan {
            AppCommand::StartScan
        } else if key == keys.search {
            self.input_mode = InputMode::Search;
            AppCommand::None
        } else if key == keys.filter {
            self.cycle_filter();
            self.reset_view_state();
            self.preserve_selection_on_replace = false;
            AppCommand::ReloadJobs
        } else if key == keys.toggle_applied {
            self.selected_job().map_or(AppCommand::None, |job| {
                AppCommand::ToggleApplied(job.key.clone())
            })
        } else if key == keys.history {
            self.view = if self.view == View::History {
                View::Active
            } else {
                View::History
            };
            self.navigation_index = view_index(self.view);
            self.reset_view_state();
            self.preserve_selection_on_replace = false;
            AppCommand::ReloadJobs
        } else if key == keys.open {
            self.open_selected()
        } else if key == keys.copy {
            self.copy_selected()
        } else if key == keys.help {
            self.help_visible = !self.help_visible;
            AppCommand::None
        } else if key == keys.quit {
            AppCommand::Quit
        } else {
            AppCommand::None
        }
    }

    fn move_selection(&mut self, direction: isize) {
        let last = self.item_count().saturating_sub(1);
        self.selected_index = if direction < 0 {
            self.selected_index.saturating_sub(1)
        } else {
            self.selected_index.saturating_add(1).min(last)
        };
        if self.view == View::Analytics {
            match self.analytics_kind {
                SkillKind::Hard => self.hard_skill_index = self.selected_index,
                SkillKind::Soft => self.soft_skill_index = self.selected_index,
            }
        }
        self.evidence_index = 0;
        self.reset_detail_scroll();
    }

    fn select_analytics_kind(&mut self, kind: SkillKind) {
        if self.analytics_kind == kind {
            return;
        }
        match self.analytics_kind {
            SkillKind::Hard => self.hard_skill_index = self.selected_index,
            SkillKind::Soft => self.soft_skill_index = self.selected_index,
        }
        self.analytics_kind = kind;
        self.selected_index = self.analytics_skill_index(kind);
        self.evidence_index = 0;
        self.reset_detail_scroll();
    }

    fn reset_analytics_selection(&mut self) {
        self.selected_index = 0;
        self.evidence_index = 0;
        self.reset_detail_scroll();
    }

    fn save_analytics_state_command(&self) -> AppCommand {
        AppCommand::SaveAnalyticsState(self.analytics_filters.clone(), self.library.clone())
    }

    fn selected_analytics_skill_name(&self) -> Option<String> {
        let report = self.analytics_report();
        let rows = match self.analytics_kind {
            SkillKind::Hard => &report.hard_skills,
            SkillKind::Soft => &report.soft_skills,
        };
        rows.get(self.selected_index)
            .map(|skill| skill.metric.name.clone())
    }

    fn toggle_selected_analytics_item(&mut self) {
        let report = self.analytics_report();
        match self.analytics_tab {
            AnalyticsTab::Overview => {
                if let Some(skill) = report
                    .recommendations
                    .get(self.selected_index)
                    .map(|item| item.skill.clone())
                {
                    toggle_map_key(&mut self.library.skills, skill, None);
                }
            }
            AnalyticsTab::Skills => {
                if let Some(skill) = self.selected_analytics_skill_name() {
                    toggle_map_key(&mut self.library.skills, skill, None);
                }
            }
            AnalyticsTab::Stacks => {
                if let Some(stack) = report
                    .stacks
                    .get(self.selected_index)
                    .map(|item| item.key.0.clone())
                    && !self.library.stacks.remove(&stack)
                {
                    self.library.stacks.insert(stack);
                }
            }
            AnalyticsTab::Market => match self.market_section {
                MarketSection::Roles => {
                    if let Some(role) = report
                        .roles
                        .get(self.selected_index)
                        .map(|item| item.name.clone())
                    {
                        toggle_map_key(&mut self.library.roles, role, false);
                    }
                }
                MarketSection::Companies => {
                    if let Some(company) = report
                        .companies
                        .get(self.selected_index)
                        .map(|item| item.name.clone())
                        && !self.library.companies.remove(&company)
                    {
                        self.library.companies.insert(company);
                    }
                }
                MarketSection::Seniority | MarketSection::Experience | MarketSection::Work => {}
            },
        }
    }

    fn cycle_selected_skill_status(&mut self) {
        if let Some(skill) = self.selected_analytics_skill_name() {
            cycle_skill_status(&mut self.library.skills, skill);
        }
    }

    fn cycle_selected_library_skill_status(&mut self) {
        let index = self
            .selected_index
            .saturating_sub(self.pending_skill_suggestions().len());
        if let Some(skill) = self.library.skills.keys().nth(index).cloned() {
            cycle_skill_status(&mut self.library.skills, skill);
        }
    }

    fn selected_pending_suggestion(&self) -> Option<&SkillSuggestion> {
        self.pending_skill_suggestions()
            .get(self.selected_index)
            .copied()
    }

    fn remove_selected_library_item(&mut self) {
        match self.library_tab {
            LibraryTab::Jobs => {
                if let Some(key) = self.library.jobs.iter().nth(self.selected_index).cloned() {
                    self.library.jobs.remove(&key);
                }
            }
            LibraryTab::Skills => {
                let index = self
                    .selected_index
                    .saturating_sub(self.pending_skill_suggestions().len());
                if self.selected_index >= self.pending_skill_suggestions().len()
                    && let Some(name) = self.library.skills.keys().nth(index).cloned()
                {
                    self.library.skills.remove(&name);
                }
            }
            LibraryTab::Stacks => {
                if let Some(stack) = self.library.stacks.iter().nth(self.selected_index).cloned() {
                    self.library.stacks.remove(&stack);
                }
            }
            LibraryTab::Roles => {
                if let Some(role) = self.library.roles.keys().nth(self.selected_index).cloned() {
                    self.library.roles.remove(&role);
                }
            }
            LibraryTab::Companies => {
                if let Some(company) = self
                    .library
                    .companies
                    .iter()
                    .nth(self.selected_index)
                    .cloned()
                {
                    self.library.companies.remove(&company);
                }
            }
        }
        self.selected_index = self.selected_index.min(self.item_count().saturating_sub(1));
    }

    fn accept_list_scroll(&mut self, direction: isize) -> bool {
        let now = Instant::now();
        if self.last_list_scroll.is_some_and(|(previous, at)| {
            previous == direction && now.duration_since(at) < StdDuration::from_millis(50)
        }) {
            return false;
        }
        self.last_list_scroll = Some((direction, now));
        true
    }

    fn move_evidence_selection(&mut self, direction: isize) {
        let last = self.analytics_evidence_jobs().len().saturating_sub(1);
        self.evidence_index = if direction < 0 {
            self.evidence_index.saturating_sub(1)
        } else {
            self.evidence_index.saturating_add(1).min(last)
        };
    }

    fn scroll_details(&mut self, direction: isize) {
        let current = self.detail_scroll();
        self.detail_scroll = if direction < 0 {
            current.saturating_sub(direction.unsigned_abs().min(u16::MAX as usize) as u16)
        } else {
            current
                .saturating_add(direction.min(u16::MAX as isize) as u16)
                .min(self.detail_scroll_max.get())
        };
    }

    fn reset_detail_scroll(&mut self) {
        self.detail_scroll = 0;
        self.detail_scroll_max.set(u16::MAX);
    }

    fn item_count(&self) -> usize {
        match self.view {
            View::Scans => self.scans.len(),
            View::Sources => self.sources.len(),
            View::Analytics => {
                let report = self.analytics_report();
                match self.analytics_tab {
                    AnalyticsTab::Overview => report.recommendations.len(),
                    AnalyticsTab::Skills => match self.analytics_kind {
                        SkillKind::Hard => report.hard_skills.len(),
                        SkillKind::Soft => report.soft_skills.len(),
                    },
                    AnalyticsTab::Stacks => report.stacks.len(),
                    AnalyticsTab::Market => match self.market_section {
                        MarketSection::Roles => report.roles.len(),
                        MarketSection::Seniority => report.seniority.len(),
                        MarketSection::Experience => report.experience.len(),
                        MarketSection::Work => report.work.len() + report.education.len(),
                        MarketSection::Companies => report.companies.len(),
                    },
                }
            }
            View::Library => match self.library_tab {
                LibraryTab::Jobs => self.library.jobs.len(),
                LibraryTab::Skills => {
                    self.pending_skill_suggestions().len() + self.library.skills.len()
                }
                LibraryTab::Stacks => self.library.stacks.len(),
                LibraryTab::Roles => self.library.roles.len(),
                LibraryTab::Companies => self.library.companies.len(),
            },
            View::Settings => SETTINGS.len(),
            _ => self.visible_jobs().count(),
        }
    }

    fn activate_navigation(&mut self) -> AppCommand {
        self.view = VIEWS[self.navigation_index];
        self.focus = Focus::Content;
        self.reset_view_state();
        self.preserve_selection_on_replace = false;
        AppCommand::ReloadJobs
    }

    fn handle_navigation_key(&mut self, code: KeyCode) -> AppCommand {
        match code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigation_index = self.navigation_index.saturating_sub(1);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.navigation_index = (self.navigation_index + 1).min(VIEWS.len() - 1);
            }
            KeyCode::Enter => return self.activate_navigation(),
            KeyCode::Esc | KeyCode::Tab | KeyCode::BackTab => self.focus = Focus::Content,
            KeyCode::Char(character) => return self.handle_action_key(character),
            _ => {}
        }
        AppCommand::None
    }

    fn handle_setting_key(&mut self, code: KeyCode) -> AppCommand {
        match code {
            KeyCode::Esc => {
                self.setting_input.clear();
                self.setting_error = None;
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                self.setting_input.pop();
                self.setting_error = None;
            }
            KeyCode::Char(character) => {
                self.setting_input.push(character);
                self.setting_error = None;
            }
            KeyCode::Enter => {
                let mut filters = self.config.filters.clone();
                match self.setting() {
                    Setting::NewJobAge => match self.setting_input.trim().parse() {
                        Ok(days) => filters.new_job_max_age_days = days,
                        Err(_) => {
                            self.setting_error = Some("Enter a positive whole number.".into());
                            return AppCommand::None;
                        }
                    },
                    Setting::Countries => {
                        filters.countries = split_setting(&self.setting_input, ',')
                            .into_iter()
                            .map(|country| country.to_uppercase())
                            .collect();
                    }
                    Setting::IncludedTitles => {
                        filters.include_title_patterns = split_setting(&self.setting_input, ';');
                    }
                    Setting::ExcludedTitles => {
                        filters.exclude_title_patterns = split_setting(&self.setting_input, ';');
                    }
                }
                if let Err(error) = filters.validate() {
                    self.setting_error = Some(error.to_string());
                    return AppCommand::None;
                }
                self.input_mode = InputMode::Normal;
                self.setting_error = None;
                return AppCommand::SaveFilters(filters);
            }
            _ => {}
        }
        AppCommand::None
    }

    fn start_setting_edit(&mut self) {
        self.setting_input = match self.setting() {
            Setting::NewJobAge => self.config.filters.new_job_max_age_days.to_string(),
            Setting::Countries => self.config.filters.countries.join(", "),
            Setting::IncludedTitles => self.config.filters.include_title_patterns.join("; "),
            Setting::ExcludedTitles => self.config.filters.exclude_title_patterns.join("; "),
        };
        self.setting_error = None;
        self.input_mode = InputMode::Setting;
    }

    fn resize_job_panes(&mut self, column: u16, width: u16) {
        let area = Rect::new(0, 0, width, 1);
        let Some((origin, available, _)) = job_pane_geometry(area) else {
            return;
        };
        self.job_list_width = Some(
            column
                .saturating_sub(origin)
                .saturating_add(1)
                .clamp(MIN_JOB_LIST_WIDTH, available - MIN_JOB_DETAILS_WIDTH),
        );
    }

    fn open_selected(&self) -> AppCommand {
        if self.view == View::Analytics {
            return self.open_selected_evidence();
        }
        self.selected_job().map_or(AppCommand::None, |job| {
            AppCommand::OpenUrl(job.classified.observed.job_url.clone())
        })
    }

    fn open_selected_evidence(&self) -> AppCommand {
        self.analytics_evidence_jobs()
            .get(self.evidence_index)
            .map_or(AppCommand::None, |job| {
                AppCommand::OpenUrl(job.classified.observed.job_url.clone())
            })
    }

    fn copy_selected(&self) -> AppCommand {
        if self.view == View::Analytics {
            return self
                .analytics_evidence_jobs()
                .get(self.evidence_index)
                .map_or(AppCommand::None, |job| {
                    AppCommand::CopyUrl(job.classified.observed.job_url.clone())
                });
        }
        self.selected_job().map_or(AppCommand::None, |job| {
            AppCommand::CopyUrl(job.classified.observed.job_url.clone())
        })
    }

    fn reset_view_state(&mut self) {
        self.selected_index = 0;
        self.analytics_kind = SkillKind::Hard;
        self.hard_skill_index = 0;
        self.soft_skill_index = 0;
        self.evidence_index = 0;
        self.reset_detail_scroll();
        self.narrow_details_visible = false;
    }

    fn clamp_selection(&mut self) {
        self.selected_index = self
            .selected_index
            .min(self.visible_jobs().count().saturating_sub(1));
    }

    fn cycle_filter(&mut self) {
        let enabled = self
            .config
            .companies
            .iter()
            .filter(|company| company.enabled)
            .map(|company| company.id.as_str())
            .collect::<Vec<_>>();
        match self.company_filter.as_deref() {
            None if self.view == View::New => self.view = View::Applied,
            None if self.view == View::Applied => self.view = View::Active,
            None => self.company_filter = enabled.first().map(|id| (*id).to_owned()),
            Some(current) => match enabled
                .iter()
                .position(|id| *id == current)
                .and_then(|index| enabled.get(index + 1))
            {
                Some(id) => self.company_filter = Some((*id).to_owned()),
                None => {
                    self.company_filter = None;
                    self.view = View::New;
                }
            },
        }
    }

    fn matches_search(&self, job: &JobRecord) -> bool {
        if self.search_query.is_empty() {
            return true;
        }
        let query = self.search_query.to_lowercase();
        job.classified
            .observed
            .title
            .to_lowercase()
            .contains(&query)
            || job.key.company_id.to_lowercase().contains(&query)
            || self.config.companies.iter().any(|company| {
                company.id == job.key.company_id && company.name.to_lowercase().contains(&query)
            })
    }

    fn analytics_jobs(&self) -> impl Iterator<Item = &JobRecord> {
        self.jobs.iter().filter(|job| {
            job.source_open
                && self
                    .company_filter
                    .as_deref()
                    .is_none_or(|company_id| job.key.company_id == company_id)
                && self.matches_search(job)
        })
    }

    fn analytics_surface_target(&self, column: u16, row: u16, area: Rect) -> Option<MouseTarget> {
        if row == area.y {
            return tab_at_widths(column, area.x, &[13, 11, 11, 11]).map(MouseTarget::AnalyticsTab);
        }
        let main = Rect::new(
            area.x,
            area.y.saturating_add(2),
            area.width,
            area.height.saturating_sub(2),
        );
        if area.width < 90 && self.narrow_details_visible {
            return item_at(
                column,
                row,
                main,
                2,
                self.analytics_evidence_jobs().len(),
                self.evidence_index,
            )
            .map(MouseTarget::Evidence);
        }
        let (surface, evidence) = if area.width >= 90 {
            let panes =
                Layout::horizontal([Constraint::Percentage(64), Constraint::Fill(1)]).split(main);
            (panes[0], Some(panes[1]))
        } else {
            (main, None)
        };
        if let Some(evidence) = evidence
            && evidence.contains((column, row).into())
        {
            return item_at(
                column,
                row,
                evidence,
                2,
                self.analytics_evidence_jobs().len(),
                self.evidence_index,
            )
            .map(MouseTarget::Evidence)
            .or(Some(MouseTarget::Details));
        }
        match self.analytics_tab {
            AnalyticsTab::Overview => {
                let sections = Layout::vertical([Constraint::Percentage(48), Constraint::Fill(1)])
                    .split(surface);
                table_item_at(
                    column,
                    row,
                    sections[1],
                    self.item_count(),
                    self.selected_index,
                )
                .map(MouseTarget::Item)
            }
            AnalyticsTab::Skills => {
                let sections = Layout::vertical([Constraint::Percentage(68), Constraint::Fill(1)])
                    .split(surface);
                let panes = Layout::horizontal([Constraint::Percentage(50), Constraint::Fill(1)])
                    .split(sections[0]);
                let report = self.analytics_report();
                table_item_at(
                    column,
                    row,
                    panes[0],
                    report.hard_skills.len(),
                    self.hard_skill_index,
                )
                .map(MouseTarget::HardSkill)
                .or_else(|| {
                    table_item_at(
                        column,
                        row,
                        panes[1],
                        report.soft_skills.len(),
                        self.soft_skill_index,
                    )
                    .map(MouseTarget::SoftSkill)
                })
            }
            AnalyticsTab::Stacks => {
                let sections = Layout::vertical([Constraint::Percentage(68), Constraint::Fill(1)])
                    .split(surface);
                table_item_at(
                    column,
                    row,
                    sections[0],
                    self.item_count(),
                    self.selected_index,
                )
                .map(MouseTarget::Item)
            }
            AnalyticsTab::Market => {
                let sections =
                    Layout::vertical([Constraint::Length(1), Constraint::Fill(1)]).split(surface);
                if sections[0].contains((column, row).into()) {
                    return tab_at_widths(column, sections[0].x, &[7, 11, 12, 6, 11])
                        .map(MouseTarget::MarketSection);
                }
                let panes = Layout::horizontal([Constraint::Percentage(62), Constraint::Fill(1)])
                    .split(sections[1]);
                table_item_at(
                    column,
                    row,
                    panes[0],
                    self.item_count(),
                    self.selected_index,
                )
                .map(MouseTarget::Item)
            }
        }
    }

    fn library_surface_target(&self, column: u16, row: u16, area: Rect) -> Option<MouseTarget> {
        if row == area.y {
            return tab_at_widths(column, area.x, &[9, 11, 11, 10, 14])
                .map(MouseTarget::LibraryTab);
        }
        let list = Rect::new(
            area.x,
            area.y.saturating_add(1),
            area.width,
            area.height.saturating_sub(1),
        );
        if self.library_tab == LibraryTab::Jobs {
            item_at(column, row, list, 2, self.item_count(), self.selected_index)
        } else {
            table_item_at(column, row, list, self.item_count(), self.selected_index)
        }
        .map(MouseTarget::Item)
    }

    fn cycle_analytics_company(&mut self) {
        let values = self
            .jobs
            .iter()
            .map(|job| job.key.company_id.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        self.analytics_filters.company =
            cycle_owned(self.analytics_filters.company.take(), &values);
        self.reset_analytics_selection();
    }

    fn cycle_analytics_role(&mut self) {
        let values = self
            .job_facts
            .values()
            .map(|facts| facts.role_family.clone())
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        self.analytics_filters.role = cycle_owned(self.analytics_filters.role.take(), &values);
        self.reset_analytics_selection();
    }

    fn job_has_skill(&self, job: &JobRecord, skill: &str) -> bool {
        self.job_facts
            .get(&job.key)
            .is_some_and(|facts| facts.skills.contains_key(skill))
    }
}

fn cycle_option<T: Copy + PartialEq>(current: Option<T>, values: &[T]) -> Option<T> {
    current
        .and_then(|value| values.iter().position(|item| *item == value))
        .and_then(|index| values.get(index + 1).copied())
        .or_else(|| current.is_none().then(|| values.first().copied()).flatten())
}

fn cycle_owned(current: Option<String>, values: &[String]) -> Option<String> {
    current
        .as_ref()
        .and_then(|value| values.iter().position(|item| item == value))
        .and_then(|index| values.get(index + 1).cloned())
        .or_else(|| current.is_none().then(|| values.first().cloned()).flatten())
}

const VIEWS: [View; 9] = [
    View::Active,
    View::New,
    View::Applied,
    View::History,
    View::Scans,
    View::Sources,
    View::Analytics,
    View::Library,
    View::Settings,
];

const ANALYTICS_TABS: [AnalyticsTab; 4] = [
    AnalyticsTab::Overview,
    AnalyticsTab::Skills,
    AnalyticsTab::Stacks,
    AnalyticsTab::Market,
];

const MARKET_SECTIONS: [MarketSection; 5] = [
    MarketSection::Roles,
    MarketSection::Seniority,
    MarketSection::Experience,
    MarketSection::Work,
    MarketSection::Companies,
];

const LIBRARY_TABS: [LibraryTab; 5] = [
    LibraryTab::Jobs,
    LibraryTab::Skills,
    LibraryTab::Stacks,
    LibraryTab::Roles,
    LibraryTab::Companies,
];

const SETTINGS: [Setting; 4] = [
    Setting::NewJobAge,
    Setting::Countries,
    Setting::IncludedTitles,
    Setting::ExcludedTitles,
];

const MIN_JOB_LIST_WIDTH: u16 = 30;
const MIN_JOB_DETAILS_WIDTH: u16 = 36;

fn job_pane_geometry(area: Rect) -> Option<(u16, u16, u16)> {
    let panes = if area.width >= 120 {
        Layout::horizontal([
            Constraint::Length(22),
            Constraint::Percentage(40),
            Constraint::Fill(1),
        ])
        .split(area)
    } else if area.width >= 80 {
        Layout::horizontal([Constraint::Percentage(45), Constraint::Fill(1)]).split(area)
    } else {
        return None;
    };
    let list = if area.width >= 120 {
        panes[1]
    } else {
        panes[0]
    };
    Some((list.x, area.right().saturating_sub(list.x), list.width))
}

fn view_index(view: View) -> usize {
    VIEWS
        .iter()
        .position(|candidate| *candidate == view)
        .unwrap_or(0)
}

fn analytics_tab_index(tab: AnalyticsTab) -> usize {
    ANALYTICS_TABS
        .iter()
        .position(|candidate| *candidate == tab)
        .unwrap_or(0)
}

fn market_section_index(section: MarketSection) -> usize {
    MARKET_SECTIONS
        .iter()
        .position(|candidate| *candidate == section)
        .unwrap_or(0)
}

fn library_tab_index(tab: LibraryTab) -> usize {
    LIBRARY_TABS
        .iter()
        .position(|candidate| *candidate == tab)
        .unwrap_or(0)
}

fn toggle_map_key<T>(map: &mut std::collections::BTreeMap<String, T>, key: String, value: T) {
    if map.remove(&key).is_none() {
        map.insert(key, value);
    }
}

fn cycle_skill_status(
    skills: &mut std::collections::BTreeMap<String, Option<SkillStatus>>,
    skill: String,
) {
    let next = match skills.get(&skill).copied().flatten() {
        None => Some(SkillStatus::Known),
        Some(status) => status.next(),
    };
    skills.insert(skill, next);
}

fn split_setting(value: &str, separator: char) -> Vec<String> {
    value
        .split(separator)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect()
}

fn extract_job_facts(jobs: &[JobRecord]) -> HashMap<JobKey, JobFacts> {
    jobs.iter()
        .map(|job| (job.key.clone(), analytics::extract(job)))
        .collect()
}

#[cfg(any())]
pub(super) fn analytics_skill_panes(area: Rect) -> [Rect; 2] {
    let left_width = area.width / 2;
    [
        Rect::new(area.x, area.y, left_width, area.height),
        Rect::new(
            area.x.saturating_add(left_width),
            area.y,
            area.width.saturating_sub(left_width),
            area.height,
        ),
    ]
}

fn item_at(
    column: u16,
    row: u16,
    area: Rect,
    item_height: u16,
    item_count: usize,
    selected_index: usize,
) -> Option<usize> {
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    if !inner.contains((column, row).into()) || item_count == 0 {
        return None;
    }
    let visible_count = usize::from(inner.height / item_height).max(1);
    let first_visible = selected_index.saturating_sub(visible_count.saturating_sub(1));
    let index = first_visible + usize::from((row - inner.y) / item_height);
    (index < item_count).then_some(index)
}

fn tab_at_widths(column: u16, origin: u16, widths: &[u16]) -> Option<usize> {
    if column < origin {
        return None;
    }
    let offset = column - origin;
    let mut end = 0;
    widths.iter().position(|width| {
        end += width;
        offset < end
    })
}

fn table_item_at(
    column: u16,
    row: u16,
    area: Rect,
    item_count: usize,
    selected_index: usize,
) -> Option<usize> {
    let inner = area.inner(ratatui::layout::Margin::new(1, 1));
    let first_row = inner.y.saturating_add(2);
    if column < inner.x
        || column >= inner.right()
        || row < first_row
        || row >= inner.bottom()
        || item_count == 0
    {
        return None;
    }
    let visible_count = usize::from(inner.height.saturating_sub(2)).max(1);
    let first_visible = selected_index.saturating_sub(visible_count.saturating_sub(1));
    let index = first_visible + usize::from(row - first_row);
    (index < item_count).then_some(index)
}
