use ratatui::style::Color;

use crate::config::ThemeOverrides;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Theme {
    pub background: Color,
    pub focused_border: Color,
    pub unfocused_border: Color,
    pub hovered_row: Color,
    pub selected_row: Color,
    pub primary_text: Color,
    pub muted_text: Color,
    pub open: Color,
    pub new: Color,
    pub applied: Color,
    pub warning: Color,
    pub error: Color,
}

impl Theme {
    pub const fn clean_dark() -> Self {
        Self {
            background: Color::Rgb(7, 17, 31),
            focused_border: Color::Rgb(66, 200, 210),
            unfocused_border: Color::Rgb(35, 62, 82),
            hovered_row: Color::Rgb(15, 35, 55),
            selected_row: Color::Rgb(28, 49, 70),
            primary_text: Color::Rgb(224, 232, 240),
            muted_text: Color::Rgb(126, 148, 166),
            open: Color::Rgb(86, 211, 130),
            new: Color::Rgb(66, 200, 210),
            applied: Color::Rgb(188, 132, 246),
            warning: Color::Rgb(245, 183, 65),
            error: Color::Rgb(255, 105, 97),
        }
    }

    pub const fn clean_light() -> Self {
        Self {
            background: Color::White,
            focused_border: Color::Blue,
            unfocused_border: Color::Gray,
            hovered_row: Color::Rgb(224, 240, 255),
            selected_row: Color::LightBlue,
            primary_text: Color::Black,
            muted_text: Color::DarkGray,
            open: Color::Green,
            new: Color::Blue,
            applied: Color::Magenta,
            warning: Color::Yellow,
            error: Color::Red,
        }
    }

    pub fn from_config(base_name: &str, overrides: &ThemeOverrides) -> Self {
        let mut theme = match base_name {
            "clean-light" => Self::clean_light(),
            _ => Self::clean_dark(),
        };
        apply(&mut theme.background, &overrides.background);
        apply(&mut theme.focused_border, &overrides.focused_border);
        apply(&mut theme.unfocused_border, &overrides.unfocused_border);
        apply(&mut theme.selected_row, &overrides.selected_row);
        apply(&mut theme.primary_text, &overrides.primary_text);
        apply(&mut theme.muted_text, &overrides.muted_text);
        apply(&mut theme.open, &overrides.open);
        apply(&mut theme.new, &overrides.new);
        apply(&mut theme.applied, &overrides.applied);
        apply(&mut theme.warning, &overrides.warning);
        apply(&mut theme.error, &overrides.error);
        theme
    }
}

fn apply(target: &mut Color, value: &Option<String>) {
    if let Some(value) = value.as_ref().and_then(|value| value.parse().ok()) {
        *target = value;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IconSet {
    pub open: &'static str,
    pub new: &'static str,
    pub applied: &'static str,
    pub saved: &'static str,
    pub history: &'static str,
    pub scanning: &'static str,
    pub source_failure: &'static str,
}

impl IconSet {
    pub const fn unicode() -> Self {
        Self {
            open: "●",
            new: "◆",
            applied: "✔",
            saved: "★",
            history: "◉",
            scanning: "⟳",
            source_failure: "▲",
        }
    }

    pub const fn ascii() -> Self {
        Self {
            open: "O",
            new: "*",
            applied: "A",
            saved: "S",
            history: "H",
            scanning: "R",
            source_failure: "!",
        }
    }
}
