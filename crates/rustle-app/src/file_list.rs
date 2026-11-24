//! The main list of files and folders.

use iced::widget::{button, column, row, scrollable, svg, text};
use iced::{Element, Fill};
use rustle_core::{Entry, format_size};

use crate::icons;
use crate::message::Message;
use crate::theme::{Colors, ROW_HEIGHT, row_button};

/// Icon size in the list. Becomes adjustable with the View menu (Task 5b).
const FILE_ICON_SIZE: f32 = 18.0;

pub fn view<'a>(entries: &'a [Entry], show_hidden: bool, c: Colors) -> Element<'a, Message> {
    let header = row![
        text("Name").color(c.muted).width(Fill),
        text("Size").color(c.muted),
    ]
    .padding([0, 12]);

    let rows = entries
        .iter()
        .filter(|e| show_hidden || !e.hidden)
        .fold(column![].spacing(1), |col, e| {
            let (size, msg) = if e.is_dir {
                (String::new(), Message::Navigate(e.path.clone()))
            } else {
                (format_size(e.size), Message::OpenFile(e.path.clone()))
            };
            let icon = svg(icons::file_icon(&e.name, e.is_dir))
                .width(FILE_ICON_SIZE)
                .height(FILE_ICON_SIZE);
            let line = row![
                icon,
                text(&e.name).width(Fill),
                text(size).color(c.muted),
            ]
            .spacing(8)
            .align_y(iced::Center);
            col.push(
                button(line)
                    .width(Fill)
                    .height(ROW_HEIGHT)
                    .padding([4, 12])
                    .style(row_button(c))
                    .on_press(msg),
            )
        });

    column![header, scrollable(rows).height(Fill)]
        .spacing(6)
        .into()
}
