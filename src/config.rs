use std::{collections::HashMap, fs, path::Path};

use ratatui::style::Color;
use regex::RegexBuilder;
use reqwest::Url;
use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub schema_version: u32,
    pub database_path: String,
    pub companies: Vec<CompanyConfig>,
    pub filters: FiltersConfig,
    pub scan: ScanConfig,
    #[serde(default)]
    pub analytics: AnalyticsConfig,
    pub ui: UiConfig,
    pub keybindings: KeybindingsConfig,
}

impl Config {
    pub fn load(path: impl AsRef<Path>) -> Result<Self, ConfigError> {
        let path = path.as_ref();
        let contents = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.display().to_string(),
            source,
        })?;
        let config: Self = toml::from_str(&contents).map_err(|source| ConfigError::Parse {
            path: path.display().to_string(),
            source,
        })?;

        config.validate()?;
        Ok(config)
    }

    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.schema_version != 1 {
            return Err(ConfigError::invalid("schema_version", "must be exactly 1"));
        }
        if self.scan.concurrency == 0 {
            return Err(ConfigError::invalid(
                "scan.concurrency",
                "must be greater than zero",
            ));
        }
        if self.scan.timeout_seconds == 0 {
            return Err(ConfigError::invalid(
                "scan.timeout_seconds",
                "must be greater than zero",
            ));
        }
        self.filters.validate()?;
        self.analytics.validate()?;

        let mut company_ids = std::collections::HashSet::new();
        for (index, company) in self.companies.iter().enumerate() {
            let id_path = format!("companies[{index}].id");
            if company.id.trim().is_empty() {
                return Err(ConfigError::invalid(id_path, "must not be empty"));
            }
            if !company_ids.insert(company.id.as_str()) {
                return Err(ConfigError::invalid(id_path, "must be unique"));
            }
            for (location, country) in &company.location_country_overrides {
                validate_country(
                    country,
                    format!("companies[{index}].location_country_overrides.{location}"),
                )?;
            }
            validate_source(company, index)?;
        }

        if !matches!(self.ui.theme.as_str(), "clean-dark" | "clean-light") {
            return Err(ConfigError::invalid(
                "ui.theme",
                "must be one of: clean-dark, clean-light",
            ));
        }
        self.ui.theme_overrides.validate()?;
        self.keybindings.validate()
    }
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("could not read configuration {path}: {source}")]
    Read {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not parse configuration {path}: {source}")]
    Parse {
        path: String,
        #[source]
        source: toml::de::Error,
    },
    #[error("invalid configuration at {field}: {message}")]
    Invalid { field: String, message: String },
}

impl ConfigError {
    fn invalid(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Invalid {
            field: field.into(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompanyConfig {
    pub id: String,
    pub name: String,
    #[serde(default = "unknown_company_metadata")]
    pub industry: String,
    #[serde(default = "unknown_company_metadata")]
    pub scale: String,
    pub enabled: bool,
    #[serde(default)]
    pub location_country_overrides: HashMap<String, String>,
    pub source: SourceConfig,
}

fn unknown_company_metadata() -> String {
    "Unknown".into()
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "strategy", rename_all = "kebab-case")]
pub enum SourceConfig {
    Ashby {
        board: String,
    },
    Greenhouse {
        board: String,
        country_filter: Option<String>,
    },
    Jibe {
        base_url: String,
        client: String,
    },
    Ebay {
        listing_url: String,
    },
    Recruitee {
        base_url: String,
    },
    Personio {
        base_url: String,
    },
    Lever {
        api_url: String,
        country_filter: Option<String>,
    },
    Workable {
        account: String,
        country_filter: Option<String>,
    },
    Workday {
        base_url: String,
        tenant: String,
        site: String,
        country: String,
        country_code: String,
    },
    Yuki {
        feed_url: String,
    },
    Teamtailor {
        feed_url: String,
        employer: String,
    },
    Bol {
        base_url: String,
    },
    Coolblue {
        listing_url: String,
    },
    Rabobank {
        base_url: String,
        country: String,
    },
    Eneco {
        listing_url: String,
    },
    Exact {
        listing_url: String,
    },
    Afas {
        listing_url: String,
    },
    AlbertHeijn {
        base_url: String,
    },
    Ing {
        listing_url: String,
    },
    Getnoticed {
        base_url: String,
        country_filter: Option<String>,
    },
    PagedHtml {
        listing_url: String,
        offset_parameter: String,
        page_size: usize,
    },
    Unsupported {
        reason: String,
    },
}

impl SourceConfig {
    pub fn strategy_name(&self) -> &'static str {
        match self {
            Self::Ashby { .. } => "Ashby",
            Self::Greenhouse { .. } => "Greenhouse",
            Self::Jibe { .. } => "Jibe",
            Self::Ebay { .. } => "eBay",
            Self::Recruitee { .. } => "Recruitee",
            Self::Personio { .. } => "Personio XML Feed",
            Self::Lever { .. } => "Lever",
            Self::Workable { .. } => "Workable",
            Self::Workday { .. } => "Workday",
            Self::Yuki { .. } => "Teamtailor JSON Feed",
            Self::Teamtailor { .. } => "Teamtailor JSON Feed",
            Self::Bol { .. } => "bol.com",
            Self::Coolblue { .. } => "Coolblue HTML",
            Self::Rabobank { .. } => "Rabobank API",
            Self::Eneco { .. } => "Eneco HTML",
            Self::Exact { .. } => "Exact HTML + JSON-LD",
            Self::Afas { .. } => "AFAS HTML + JSON-LD",
            Self::AlbertHeijn { .. } => "Albert Heijn API",
            Self::Ing { .. } => "ING HTML",
            Self::Getnoticed { .. } => "Getnoticed",
            Self::PagedHtml { .. } => "Paged HTML",
            Self::Unsupported { .. } => "Unsupported",
        }
    }

    pub fn reference(&self) -> &str {
        match self {
            Self::Lever { api_url, .. } => api_url,
            Self::Workable { account, .. } => account,
            Self::Ashby { board } | Self::Greenhouse { board, .. } => board,
            Self::Jibe { base_url, .. }
            | Self::Recruitee { base_url }
            | Self::Personio { base_url }
            | Self::Bol { base_url }
            | Self::Rabobank { base_url, .. }
            | Self::AlbertHeijn { base_url }
            | Self::Getnoticed { base_url, .. } => base_url,
            Self::Workday { base_url, .. } => base_url,
            Self::Ebay { listing_url }
            | Self::Yuki {
                feed_url: listing_url,
            }
            | Self::Teamtailor {
                feed_url: listing_url,
                ..
            }
            | Self::Coolblue { listing_url }
            | Self::Eneco { listing_url }
            | Self::Exact { listing_url }
            | Self::Afas { listing_url }
            | Self::Ing { listing_url }
            | Self::PagedHtml { listing_url, .. } => listing_url,
            Self::Unsupported { reason } => reason,
        }
    }
}

fn validate_source(company: &CompanyConfig, index: usize) -> Result<(), ConfigError> {
    let path = |field: &str| format!("companies[{index}].source.{field}");
    match &company.source {
        SourceConfig::Ashby { board } => validate_non_empty(board, path("board")),
        SourceConfig::Greenhouse {
            board,
            country_filter,
        } => {
            validate_non_empty(board, path("board"))?;
            if let Some(country) = country_filter {
                validate_country(country, path("country_filter"))?;
            }
            Ok(())
        }
        SourceConfig::Lever {
            api_url,
            country_filter,
        } => {
            validate_https_url(api_url, path("api_url"))?;
            if let Some(country) = country_filter {
                validate_country(country, path("country_filter"))?;
            }
            Ok(())
        }
        SourceConfig::Workable {
            account,
            country_filter,
        } => {
            validate_non_empty(account, path("account"))?;
            if let Some(country) = country_filter {
                validate_country(country, path("country_filter"))?;
            }
            Ok(())
        }
        SourceConfig::Workday {
            base_url,
            tenant,
            site,
            country,
            country_code,
        } => {
            validate_https_url(base_url, path("base_url"))?;
            validate_non_empty(tenant, path("tenant"))?;
            validate_non_empty(site, path("site"))?;
            validate_non_empty(country, path("country"))?;
            validate_country(country_code, path("country_code"))
        }
        SourceConfig::Jibe { base_url, client } => {
            validate_https_url(base_url, path("base_url"))?;
            validate_non_empty(client, path("client"))
        }
        SourceConfig::Recruitee { base_url }
        | SourceConfig::Personio { base_url }
        | SourceConfig::Bol { base_url } => {
            validate_https_url(base_url, path("base_url"))?;
            Ok(())
        }
        SourceConfig::Yuki { feed_url } => {
            validate_https_url(feed_url, path("feed_url"))?;
            if company.id != "yuki" || feed_url != "https://jobs.yukisoftware.com/jobs.json" {
                return Err(ConfigError::invalid(
                    path("feed_url"),
                    "must be Yuki's exact official JSON feed",
                ));
            }
            Ok(())
        }
        SourceConfig::Teamtailor { feed_url, employer } => {
            validate_https_url(feed_url, path("feed_url"))?;
            validate_non_empty(employer, path("employer"))
        }
        SourceConfig::Coolblue { listing_url } => {
            validate_https_url(listing_url, path("listing_url"))?;
            if listing_url != "https://www.coolblue.nl/en/vacancies/search" {
                return Err(ConfigError::invalid(
                    path("listing_url"),
                    "must be the official Coolblue Netherlands vacancy URL",
                ));
            }
            Ok(())
        }
        SourceConfig::Rabobank { base_url, country } => {
            validate_https_url(base_url, path("base_url"))?;
            validate_country(country, path("country"))
        }
        SourceConfig::Eneco { listing_url } => validate_https_url(listing_url, path("listing_url")),
        SourceConfig::Exact { listing_url } => {
            validate_https_url(listing_url, path("listing_url"))?;
            if listing_url != "https://www.exact.com/careers/vacancies" {
                return Err(ConfigError::invalid(
                    path("listing_url"),
                    "must be the official Exact vacancy URL",
                ));
            }
            Ok(())
        }
        SourceConfig::Afas { listing_url } => {
            validate_https_url(listing_url, path("listing_url"))?;
            if listing_url != "https://www.werkenbijafas.nl/alle-vacatures" {
                return Err(ConfigError::invalid(
                    path("listing_url"),
                    "must be the official AFAS vacancy URL",
                ));
            }
            Ok(())
        }
        SourceConfig::AlbertHeijn { base_url } => validate_https_url(base_url, path("base_url")),
        SourceConfig::Getnoticed {
            base_url,
            country_filter,
        } => {
            validate_https_url(base_url, path("base_url"))?;
            match company.id.as_str() {
                "abn-amro"
                    if base_url == "https://www.werkenbijabnamro.nl"
                        && country_filter.as_deref() == Some("Nederland") => {}
                "topicus"
                    if base_url == "https://www.werkenbijtopicus.nl"
                        && country_filter.is_none() => {}
                "abn-amro" | "topicus" => {
                    return Err(ConfigError::invalid(
                        path("base_url"),
                        "must match the company's official Getnoticed careers URL and filter",
                    ));
                }
                _ => {
                    return Err(ConfigError::invalid(
                        path("strategy"),
                        "getnoticed is supported only for abn-amro and topicus",
                    ));
                }
            }
            Ok(())
        }
        SourceConfig::Ing { listing_url } | SourceConfig::Ebay { listing_url } => {
            validate_https_url(listing_url, path("listing_url"))
        }
        SourceConfig::PagedHtml {
            listing_url,
            offset_parameter,
            page_size,
        } => {
            validate_https_url(listing_url, path("listing_url"))?;
            validate_non_empty(offset_parameter, path("offset_parameter"))?;
            if *page_size == 0 {
                return Err(ConfigError::invalid(
                    path("page_size"),
                    "must be greater than zero",
                ));
            }
            Ok(())
        }
        SourceConfig::Unsupported { reason } => {
            validate_non_empty(reason, path("reason"))?;
            if company.enabled {
                return Err(ConfigError::invalid(
                    path("strategy"),
                    "unsupported sources must be disabled",
                ));
            }
            Ok(())
        }
    }
}

fn validate_non_empty(value: &str, path: String) -> Result<(), ConfigError> {
    if value.trim().is_empty() {
        return Err(ConfigError::invalid(path, "must not be empty"));
    }
    Ok(())
}

fn validate_https_url(value: &str, path: String) -> Result<(), ConfigError> {
    validate_non_empty(value, path.clone())?;
    let valid =
        Url::parse(value).is_ok_and(|url| url.scheme() == "https" && url.host_str().is_some());
    if !valid {
        return Err(ConfigError::invalid(path, "must be an HTTPS URL"));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct FiltersConfig {
    pub countries: Vec<String>,
    #[serde(default = "default_new_job_max_age_days")]
    pub new_job_max_age_days: u32,
    pub include_title_patterns: Vec<String>,
    pub exclude_title_patterns: Vec<String>,
}

impl FiltersConfig {
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.new_job_max_age_days == 0 {
            return Err(ConfigError::invalid(
                "filters.new_job_max_age_days",
                "must be greater than zero",
            ));
        }
        if self.countries.is_empty() {
            return Err(ConfigError::invalid(
                "filters.countries",
                "must contain at least one country",
            ));
        }
        for (index, country) in self.countries.iter().enumerate() {
            validate_country(country, format!("filters.countries[{index}]"))?;
        }
        for (field, patterns) in [
            ("include_title_patterns", &self.include_title_patterns),
            ("exclude_title_patterns", &self.exclude_title_patterns),
        ] {
            for (index, pattern) in patterns.iter().enumerate() {
                RegexBuilder::new(pattern)
                    .case_insensitive(true)
                    .build()
                    .map_err(|error| {
                        ConfigError::invalid(
                            format!("filters.{field}[{index}]"),
                            format!("must be a valid regular expression: {error}"),
                        )
                    })?;
            }
        }
        Ok(())
    }
}

fn default_new_job_max_age_days() -> u32 {
    7
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct AnalyticsConfig {
    #[serde(default)]
    pub provider: AnalyticsProvider,
    #[serde(default = "default_minimum_skill_occurrence")]
    pub minimum_skill_occurrence: usize,
    #[serde(default = "default_maximum_skills")]
    pub maximum_skills: usize,
    #[serde(default = "default_ai_timeout_seconds")]
    pub ai_timeout_seconds: u64,
    #[serde(default = "default_minimum_cooccurrence")]
    pub minimum_cooccurrence: usize,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AnalyticsProvider {
    #[default]
    Local,
    Claude,
    Codex,
}

impl Default for AnalyticsConfig {
    fn default() -> Self {
        Self {
            provider: AnalyticsProvider::Local,
            minimum_skill_occurrence: default_minimum_skill_occurrence(),
            maximum_skills: default_maximum_skills(),
            ai_timeout_seconds: default_ai_timeout_seconds(),
            minimum_cooccurrence: default_minimum_cooccurrence(),
        }
    }
}

impl AnalyticsConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        if self.minimum_cooccurrence == 0 {
            return Err(ConfigError::invalid(
                "analytics.minimum_cooccurrence",
                "must be greater than zero",
            ));
        }
        for (field, value) in [
            ("minimum_skill_occurrence", self.minimum_skill_occurrence),
            ("maximum_skills", self.maximum_skills),
        ] {
            if value == 0 {
                return Err(ConfigError::invalid(
                    format!("analytics.{field}"),
                    "must be greater than zero",
                ));
            }
        }
        if self.ai_timeout_seconds == 0 {
            return Err(ConfigError::invalid(
                "analytics.ai_timeout_seconds",
                "must be greater than zero",
            ));
        }
        Ok(())
    }
}

impl AnalyticsProvider {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }
}

fn default_minimum_cooccurrence() -> usize {
    3
}

fn default_minimum_skill_occurrence() -> usize {
    2
}

fn default_maximum_skills() -> usize {
    50
}

fn default_ai_timeout_seconds() -> u64 {
    60
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScanConfig {
    pub concurrency: usize,
    pub timeout_seconds: u64,
    pub retry_count: u32,
    pub user_agent: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UiConfig {
    pub theme: String,
    pub unicode_icons: bool,
    #[serde(default)]
    pub theme_overrides: ThemeOverrides,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThemeOverrides {
    pub background: Option<String>,
    pub focused_border: Option<String>,
    pub unfocused_border: Option<String>,
    pub selected_row: Option<String>,
    pub primary_text: Option<String>,
    pub muted_text: Option<String>,
    pub open: Option<String>,
    pub new: Option<String>,
    pub applied: Option<String>,
    pub warning: Option<String>,
    pub error: Option<String>,
}

impl ThemeOverrides {
    fn validate(&self) -> Result<(), ConfigError> {
        for (token, value) in [
            ("background", &self.background),
            ("focused_border", &self.focused_border),
            ("unfocused_border", &self.unfocused_border),
            ("selected_row", &self.selected_row),
            ("primary_text", &self.primary_text),
            ("muted_text", &self.muted_text),
            ("open", &self.open),
            ("new", &self.new),
            ("applied", &self.applied),
            ("warning", &self.warning),
            ("error", &self.error),
        ] {
            if let Some(value) = value
                && !is_supported_colour(value)
            {
                return Err(ConfigError::invalid(
                    format!("ui.theme_overrides.{token}"),
                    "must be a named ANSI colour or #RRGGBB",
                ));
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct KeybindingsConfig {
    pub scan: String,
    pub search: String,
    pub filter: String,
    pub toggle_applied: String,
    pub history: String,
    pub open: String,
    #[serde(default = "default_copy_key")]
    pub copy: String,
    pub help: String,
    pub quit: String,
}

impl KeybindingsConfig {
    fn validate(&self) -> Result<(), ConfigError> {
        let bindings = [
            ("scan", &self.scan),
            ("search", &self.search),
            ("filter", &self.filter),
            ("toggle_applied", &self.toggle_applied),
            ("history", &self.history),
            ("open", &self.open),
            ("copy", &self.copy),
            ("help", &self.help),
            ("quit", &self.quit),
        ];
        let mut values = std::collections::HashSet::new();
        for (name, binding) in bindings {
            let mut characters = binding.chars();
            let Some(character) = characters.next() else {
                return Err(ConfigError::invalid(
                    format!("keybindings.{name}"),
                    "must be exactly one character",
                ));
            };
            if characters.next().is_some() {
                return Err(ConfigError::invalid(
                    format!("keybindings.{name}"),
                    "must be exactly one character",
                ));
            }
            if character.is_control() {
                return Err(ConfigError::invalid(
                    format!("keybindings.{name}"),
                    "must be a non-control character",
                ));
            }
            if matches!(binding.as_str(), "j" | "k" | "J" | "K") {
                return Err(ConfigError::invalid(
                    format!("keybindings.{name}"),
                    "must not collide with a fixed navigation key",
                ));
            }
            if !values.insert(binding) {
                return Err(ConfigError::invalid(
                    format!("keybindings.{name}"),
                    "must not duplicate another keybinding",
                ));
            }
        }
        Ok(())
    }
}

fn default_copy_key() -> String {
    "c".into()
}

fn validate_country(country: &str, field: String) -> Result<(), ConfigError> {
    let is_country_code =
        country.len() == 2 && country.bytes().all(|byte| byte.is_ascii_uppercase());
    if is_country_code {
        Ok(())
    } else {
        Err(ConfigError::invalid(
            field,
            "must be a two-letter uppercase ASCII country code",
        ))
    }
}

fn is_supported_colour(value: &str) -> bool {
    matches!(
        value.parse(),
        Ok(Color::Reset
            | Color::Black
            | Color::Red
            | Color::Green
            | Color::Yellow
            | Color::Blue
            | Color::Magenta
            | Color::Cyan
            | Color::Gray
            | Color::DarkGray
            | Color::LightRed
            | Color::LightGreen
            | Color::LightYellow
            | Color::LightBlue
            | Color::LightMagenta
            | Color::LightCyan
            | Color::White
            | Color::Rgb(_, _, _))
    )
}
