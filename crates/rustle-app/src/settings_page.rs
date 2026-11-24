//! The settings page (opened with ⚙). It replaces the file area while open.

use iced::widget::{button, column, container, radio, row, slider, text, toggler};
use iced::{Element, Fill};
use rustle_core::settings::{MAX_OPACITY, MIN_OPACITY};
use rustle_core::{Settings, ThemeChoice};

use crate::message::Message;
use crate::theme::{Colors, panel, row_button};

/// `started_transparent`: whether the window was created see-through. If it
/// wasn't, lowering opacity only works after a restart, so we say so.
pub fn view(s: &Settings, started_transparent: bool, c: Colors) -> Element<'_, Message> {
    let note = |t: &'static str| text(t).size(13).color(c.muted);

    let theme = column![
        text("Theme"),
        row![
            radio("Dark", ThemeChoice::Dark, Some(s.theme), Message::SetTheme),
            radio(
                "Light",
                ThemeChoice::Light,
                Some(s.theme),
                Message::SetTheme
            ),
        ]
        .spacing(20),
    ]
    .spacing(8);

    let mut opacity = column![
        row![
            text("Window opacity").width(Fill),
            text(format!("{:.0} %", s.opacity * 100.0)).color(c.muted),
        ],
        slider(MIN_OPACITY..=MAX_OPACITY, s.opacity, Message::SetOpacity)
            .step(0.01_f32)
            .on_release(Message::SaveSettings),
    ]
    .spacing(8);
    if !started_transparent && s.opacity < MAX_OPACITY {
        opacity = opacity.push(note("Restart Rustle to apply."));
    }

    let mut options = column![theme, opacity].spacing(24);

    // Blur isn't available on Windows, so the toggle only exists elsewhere.
    #[cfg(not(windows))]
    {
        options = options.push(
            column![
                toggler(s.blur)
                    .label("Blur behind window")
                    .on_toggle(Message::SetBlur),
                note("Restart to apply. Needs a desktop that supports it (e.g. KDE, Hyprland)."),
            ]
            .spacing(6),
        );
    }

    options = options.push(
        toggler(s.show_hidden)
            .label("Show hidden files")
            .on_toggle(Message::SetShowHidden),
    );

    let card = container(options.padding(20).max_width(520)).style(panel(c.surface));

    column![
        row![
            button("←  Back")
                .style(row_button(c))
                .on_press(Message::CloseSettings),
            text("Settings").size(20),
        ]
        .spacing(12)
        .align_y(iced::Center),
        card,
    ]
    .spacing(16)
    .width(Fill)
    .into()
}
