//! The main list of files and folders.

use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Fill};
use rustle_core::{Entry, format_size};

use crate::message::Message;
use crate::theme::{Colors, ROW_HEIGHT, row_button};

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
            let (icon, size, msg) = if e.is_dir {
                ("📁", String::new(), Message::Navigate(e.path.clone()))
            } else {
                ("📄", format_size(e.size), Message::OpenFile(e.path.clone()))
            };
            let line = row![
                text(format!("{icon}  {}", e.name)).width(Fill),
                text(size).color(c.muted),
            ];
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
