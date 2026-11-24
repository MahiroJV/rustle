//! The Home page: shown on startup and in new tabs. Cards for the user's
//! folders, and one card per drive with a bar showing how full it is.

use iced::widget::{button, column, grid, progress_bar, row, scrollable, space, svg, text};
use iced::{Background, Border, Element, Fill, Theme};
use rustle_core::format_size;
use rustle_fs::places::{Drive, Place};

use crate::icons::{self, Ui};
use crate::message::Message;
use crate::theme::{Colors, RADIUS, card_button};

/// A drive this full (or more) gets a red bar.
const NEARLY_FULL: f32 = 0.9;

pub fn view<'a>(places: &'a [Place], drives: &'a [Drive], c: Colors) -> Element<'a, Message> {
    let heading = |t: &'static str| text(t).size(12).color(c.muted);

    let folder_cards = places.iter().map(|p| {
        let card = row![
            svg(icons::place_icon(&p.label)).width(28).height(28),
            text(&p.label).size(15),
        ]
        .spacing(12)
        .align_y(iced::Center);
        button(card)
            .width(Fill)
            .height(Fill)
            .padding([10, 14])
            .style(card_button(c))
            .on_press(Message::Navigate(p.path.clone()))
            .into()
    });
    // Each card at most 230 px wide, 4.5× wider than tall.
    let folders = grid(folder_cards)
        .fluid(230)
        .height(grid::Sizing::AspectRatio(4.5))
        .spacing(10);

    let drive_cards = drives.iter().map(|d| {
        let used = used_fraction(d.free, d.total);
        let bar_color = if used >= NEARLY_FULL { c.danger } else { c.accent };
        let card = column![
            row![
                icons::ui(Ui::Drive, c.text, 20.0),
                text(&d.label).size(15),
                space().width(Fill),
                text(d.mount.display().to_string()).size(12).color(c.muted),
            ]
            .spacing(10)
            .align_y(iced::Center),
            progress_bar(0.0..=1.0, used)
                .girth(6)
                .style(move |_theme: &Theme| progress_bar::Style {
                    background: Background::Color(c.raised),
                    bar: Background::Color(bar_color),
                    border: Border::default().rounded(RADIUS),
                }),
            text(format!(
                "{} free of {} · {:.0}% used",
                format_size(d.free),
                format_size(d.total),
                used * 100.0
            ))
            .size(12)
            .color(c.muted),
        ]
        .spacing(8);
        button(card)
            .width(Fill)
            .height(Fill)
            .padding([12, 14])
            .style(card_button(c))
            .on_press(Message::Navigate(d.mount.clone()))
            .into()
    });
    let drives = grid(drive_cards)
        .fluid(340)
        .height(grid::Sizing::AspectRatio(3.6))
        .spacing(10);

    scrollable(
        column![
            text("Home page").size(24),
            heading("FOLDERS"),
            folders,
            heading("DRIVES"),
            drives,
        ]
        .spacing(14)
        .padding([8, 4]),
    )
    .height(Fill)
    .into()
}

/// How full a drive is, from 0.0 (empty) to 1.0 (full).
/// A drive that reports a total of 0 counts as empty (no division by zero).
pub fn used_fraction(free: u64, total: u64) -> f32 {
    if total == 0 {
        return 0.0;
    }
    let used = total.saturating_sub(free);
    (used as f64 / total as f64) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage() {
        assert_eq!(used_fraction(0, 0), 0.0);
        assert_eq!(used_fraction(25, 100), 0.75);
        assert_eq!(used_fraction(100, 100), 0.0);
        // A strange report (more free than total) doesn't go negative.
        assert_eq!(used_fraction(200, 100), 0.0);
    }
}
