//! The path bar, like GNOME Files: the current path as clickable buttons.
//! Click a piece to jump there. Click the empty space (or press Ctrl+L) to
//! type a path instead; Enter goes there, Escape goes back to the buttons.

use std::path::Path;

use iced::widget::{button, mouse_area, row, scrollable, space, text, text_input};
use iced::{Element, Fill};
use rustle_core::crumbs;

use crate::icons::{self, Ui};
use crate::message::Message;
use crate::tab::Tab;
use crate::theme::{Colors, crumb_button};

/// The text field's widget id, so Ctrl+L can focus it.
pub const LOCATION_ID: &str = "location";

pub fn view<'a>(tab: &'a Tab, home: &Path, editing: bool, c: Colors) -> Element<'a, Message> {
    if editing {
        return text_input("Type a path and press Enter", &tab.location_input)
            .id(LOCATION_ID)
            .on_input(Message::LocationChanged)
            .on_submit(Message::LocationSubmit)
            .padding([5, 8])
            .into();
    }

    let pieces = crumbs(&tab.cwd, home);
    let last = pieces.len().saturating_sub(1);
    let buttons = pieces
        .into_iter()
        .enumerate()
        .fold(row![].spacing(2).align_y(iced::Center), |bar, (i, crumb)| {
            let current = i == last;
            let mut label = row![].spacing(6).align_y(iced::Center);
            if crumb.is_home {
                let color = if current { c.text } else { c.muted };
                label = label.push(icons::ui(Ui::Home, color, 16.0));
            }
            label = label.push(text(crumb.label));

            let bar = if i > 0 { bar.push(text("/").color(c.muted)) } else { bar };
            bar.push(
                button(label)
                    .padding([4, 8])
                    .style(crumb_button(c, current))
                    .on_press(Message::Navigate(crumb.path)),
            )
        });

    // Long paths scroll sideways and stay pinned to the end, so the current
    // folder is always visible. The scrollbar is hidden (zero width).
    let crumbs_row = scrollable(buttons)
        .direction(scrollable::Direction::Horizontal(
            scrollable::Scrollbar::new().width(0).scroller_width(0),
        ))
        .anchor_right();

    row![
        crumbs_row,
        // The empty space after the last piece switches to typing mode.
        mouse_area(space().width(Fill).height(30)).on_press(Message::StartEditLocation),
    ]
    .align_y(iced::Center)
    .into()
}
