use std::{collections::HashMap, fs, path::Path};

use ratatui::style::Color;
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

        for (index, country) in self.filters.countries.iter().enumerate() {
            validate_country(country, format!("filters.countries[{index}]"))?;
        }

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
    pub enabled: bool,
    #[serde(default)]
    pub location_country_overrides: HashMap<String, String>,
    pub source: SourceConfig,
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
    Bol {
        base_url: String,
    },
    Rabobank {
        base_url: String,
        country: String,
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
        SourceConfig::Jibe { base_url, client } => {
            validate_https_url(base_url, path("base_url"))?;
            validate_non_empty(client, path("client"))
        }
        SourceConfig::Recruitee { base_url } | SourceConfig::Bol { base_url } => {
            validate_https_url(base_url, path("base_url"))?;
            Ok(())
        }
        SourceConfig::Rabobank { base_url, country } => {
            validate_https_url(base_url, path("base_url"))?;
            validate_country(country, path("country"))
        }
        SourceConfig::Getnoticed {
            base_url,
            country_filter,
        } => {
            validate_https_url(base_url, path("base_url"))?;
            if company.id != "abn-amro" {
                return Err(ConfigError::invalid(
                    path("strategy"),
                    "getnoticed is supported only for abn-amro",
                ));
            }
            if base_url != "https://www.werkenbijabnamro.nl" {
                return Err(ConfigError::invalid(
                    path("base_url"),
                    "must be the official ABN AMRO careers URL",
                ));
            }
            if country_filter.as_deref() != Some("Nederland") {
                return Err(ConfigError::invalid(
                    path("country_filter"),
                    "must be Nederland",
                ));
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

#[derive(Debug, Clone, Deserialize)]
pub struct FiltersConfig {
    pub countries: Vec<String>,
    pub include_families: Vec<String>,
    pub include_title_patterns: Vec<String>,
    pub exclude_title_patterns: Vec<String>,
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
