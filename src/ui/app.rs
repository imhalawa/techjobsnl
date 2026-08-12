use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::{
    config::Config,
    domain::{JobKey, JobRecord, ScanEvent, SourceScan},
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppCommand {
    None,
    StartScan,
    ToggleApplied(JobKey),
    OpenUrl(String),
    CopyUrl(String),
    ReloadJobs,
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
    theme: Theme,
    icons: IconSet,
    view: View,
    input_mode: InputMode,
    selected_index: usize,
    active_job_count: usize,
    company_filter: Option<String>,
    preserve_selection_on_replace: bool,
    search_query: String,
    detail_scroll: u16,
    narrow_details_visible: bool,
    help_visible: bool,
    scan_progress: ScanProgress,
}

impl App {
    pub fn new(config: Config, jobs: Vec<JobRecord>) -> Self {
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
            theme,
            icons,
            view: View::Active,
            input_mode: InputMode::Normal,
            selected_index: 0,
            active_job_count,
            company_filter: None,
            preserve_selection_on_replace: true,
            search_query: String::new(),
            detail_scroll: 0,
            narrow_details_visible: false,
            help_visible: false,
            scan_progress: ScanProgress::default(),
        }
    }

    pub fn replace_jobs(&mut self, jobs: Vec<JobRecord>, active_job_count: usize) {
        if matches!(self.view, View::Scans | View::Sources) {
            self.jobs = jobs;
            self.active_job_count = active_job_count;
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

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => self.move_selection(-1),
            KeyCode::Down | KeyCode::Char('j') => self.move_selection(1),
            KeyCode::Char('J') => self.detail_scroll = self.detail_scroll.saturating_add(1),
            KeyCode::Char('K') => self.detail_scroll = self.detail_scroll.saturating_sub(1),
            KeyCode::Left => return self.switch_view(-1),
            KeyCode::Right => return self.switch_view(1),
            KeyCode::Enter if width < 80 => {
                self.narrow_details_visible = !self.narrow_details_visible;
            }
            KeyCode::Enter => return self.open_selected(),
            KeyCode::Esc => {
                self.help_visible = false;
                self.narrow_details_visible = false;
            }
            KeyCode::Char(character) => return self.handle_action_key(character),
            _ => {}
        }
        AppCommand::None
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
                View::New => job.source_open && job.is_new,
                View::Applied => job.applied_at.is_some(),
                View::History => !job.source_open || job.reopened_at.is_some(),
                View::Scans | View::Sources => false,
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

    pub fn selected_index(&self) -> usize {
        self.selected_index
    }

    pub fn selected_job(&self) -> Option<&JobRecord> {
        self.visible_jobs().nth(self.selected_index)
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
        self.detail_scroll
    }

    pub fn narrow_details_visible(&self) -> bool {
        self.narrow_details_visible
    }

    pub fn help_visible(&self) -> bool {
        self.help_visible
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
        self.detail_scroll = 0;
    }

    fn item_count(&self) -> usize {
        match self.view {
            View::Scans => self.scans.len(),
            View::Sources => self.sources.len(),
            _ => self.visible_jobs().count(),
        }
    }

    fn switch_view(&mut self, direction: isize) -> AppCommand {
        const VIEWS: [View; 6] = [
            View::Active,
            View::New,
            View::Applied,
            View::History,
            View::Scans,
            View::Sources,
        ];
        let index = VIEWS
            .iter()
            .position(|view| *view == self.view)
            .unwrap_or(0);
        let next = if direction < 0 {
            index.checked_sub(1).unwrap_or(VIEWS.len() - 1)
        } else {
            (index + 1) % VIEWS.len()
        };
        self.view = VIEWS[next];
        self.reset_view_state();
        self.preserve_selection_on_replace = false;
        AppCommand::ReloadJobs
    }

    fn open_selected(&self) -> AppCommand {
        self.selected_job().map_or(AppCommand::None, |job| {
            AppCommand::OpenUrl(job.classified.observed.job_url.clone())
        })
    }

    fn copy_selected(&self) -> AppCommand {
        self.selected_job().map_or(AppCommand::None, |job| {
            AppCommand::CopyUrl(job.classified.observed.job_url.clone())
        })
    }

    fn reset_view_state(&mut self) {
        self.selected_index = 0;
        self.detail_scroll = 0;
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
}
