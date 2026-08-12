use crossterm::event::KeyEvent;

use crate::{config::Config, domain::JobRecord};

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    None,
}

pub struct App {
    config: Config,
    jobs: Vec<JobRecord>,
    theme: Theme,
    icons: IconSet,
    view: View,
    input_mode: InputMode,
    selected_index: usize,
}

impl App {
    pub fn new(config: Config, jobs: Vec<JobRecord>) -> Self {
        let theme = Theme::from_config(&config.ui.theme, &config.ui.theme_overrides);
        let icons = if config.ui.unicode_icons {
            IconSet::unicode()
        } else {
            IconSet::ascii()
        };
        Self {
            config,
            jobs,
            theme,
            icons,
            view: View::Active,
            input_mode: InputMode::Normal,
            selected_index: 0,
        }
    }

    pub fn replace_jobs(&mut self, jobs: Vec<JobRecord>) {
        self.jobs = jobs;
        self.selected_index = self.selected_index.min(self.jobs.len().saturating_sub(1));
    }

    pub fn handle_key(&mut self, _key: KeyEvent) -> AppCommand {
        AppCommand::None
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn jobs(&self) -> &[JobRecord] {
        &self.jobs
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
        self.jobs.get(self.selected_index)
    }
}
