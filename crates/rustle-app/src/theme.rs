//! Rustle's look: the two color palettes and the style functions widgets use.

use iced::theme::Palette;
use iced::widget::{button, container};
use iced::{Background, Border, Color, Theme, color};
use rustle_core::ThemeChoice;

/// Corner radius used everywhere.
pub const RADIUS: f32 = 6.0;
pub const ROW_HEIGHT: f32 = 28.0;
pub const SIDEBAR_WIDTH: f32 = 220.0;

/// All the colors of one theme. `Copy` so style closures can keep their own.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    pub bg: Color,
    pub surface: Color,
    pub raised: Color,
    pub text: Color,
    pub muted: Color,
    pub accent: Color,
    pub danger: Color,
}

pub fn palette(choice: ThemeChoice) -> Colors {
    match choice {
        ThemeChoice::Dark => Colors {
            bg: color!(0x16181D),
            surface: color!(0x1E2128),
            raised: color!(0x262A33),
            text: color!(0xE6E8EE),
            muted: color!(0x8B91A0),
            accent: color!(0xE07A3F),
            danger: color!(0xE5534B),
        },
        ThemeChoice::Light => Colors {
            bg: color!(0xF6F7F9),
            surface: color!(0xFFFFFF),
            raised: color!(0xECEEF2),
            text: color!(0x1C1F26),
            muted: color!(0x6B7280),
            accent: color!(0xE07A3F),
            danger: color!(0xE5534B),
        },
    }
}

impl Colors {
    /// The same palette with see-through backgrounds, for window opacity.
    /// Text and accent stay solid so they remain readable.
    pub fn with_opacity(self, opacity: f32) -> Self {
        let fade = |c: Color| Color { a: c.a * opacity, ..c };
        Self {
            bg: fade(self.bg),
            surface: fade(self.surface),
            raised: fade(self.raised),
            ..self
        }
    }
}

/// The iced theme. Built-in widgets (slider, toggler, radio…) take their
/// colors from this, so they match without extra styling.
pub fn iced_theme(choice: ThemeChoice) -> Theme {
    let c = palette(choice);
    let name = match choice {
        ThemeChoice::Dark => "Rustle Dark",
        ThemeChoice::Light => "Rustle Light",
    };
    Theme::custom(
        name,
        Palette {
            background: c.bg,
            text: c.text,
            primary: c.accent,
            success: c.accent,
            warning: c.accent,
            danger: c.danger,
        },
    )
}

/// A flat button for list rows and the toolbar: no background until hovered.
pub fn row_button(c: Colors) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let background = match status {
            button::Status::Hovered | button::Status::Pressed => Some(Background::Color(c.raised)),
            button::Status::Active | button::Status::Disabled => None,
        };
        let text_color = match status {
            button::Status::Disabled => c.muted,
            _ => c.text,
        };
        button::Style {
            background,
            text_color,
            border: Border::default().rounded(RADIUS),
            ..button::Style::default()
        }
    }
}

/// A tab in the tab bar. The active tab is filled; the others are muted
/// until hovered.
pub fn tab_button(c: Colors, active: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let background = if active {
            Some(Background::Color(c.surface))
        } else if hovered {
            Some(Background::Color(c.raised))
        } else {
            None
        };
        button::Style {
            background,
            text_color: if active || hovered { c.text } else { c.muted },
            border: Border::default().rounded(RADIUS),
            ..button::Style::default()
        }
    }
}

/// A rounded box filled with `color` (sidebar, toolbar, settings card…).
pub fn panel(color: Color) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(Background::Color(color)),
        border: Border::default().rounded(RADIUS),
        ..container::Style::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_accent_is_copper() {
        assert_eq!(
            palette(ThemeChoice::Dark).accent,
            Color::from_rgb8(0xE0, 0x7A, 0x3F)
        );
    }

    #[test]
    fn light_background() {
        assert_eq!(
            palette(ThemeChoice::Light).bg,
            Color::from_rgb8(0xF6, 0xF7, 0xF9)
        );
    }

    #[test]
    fn opacity_fades_backgrounds_not_text() {
        let c = palette(ThemeChoice::Dark).with_opacity(0.5);
        assert_eq!(c.bg.a, 0.5);
        assert_eq!(c.text.a, 1.0);
    }
}
