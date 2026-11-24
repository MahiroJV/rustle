//! The View menu: a small popover under the View button with layout,
//! icon size and sort order.

use iced::widget::{column, container, radio, row, text};
use iced::{Element, Theme};
use rustle_core::{IconSize, Layout, SortKey, ViewSettings};

use crate::message::Message;
use crate::theme::{Colors, RADIUS};

pub const WIDTH: f32 = 260.0;

pub fn view<'a>(v: ViewSettings, c: Colors) -> Element<'a, Message> {
    let title = |t: &'static str| text(t).size(12).color(c.muted);

    let layout = column![
        title("LAYOUT"),
        radio("Details", Layout::Details, Some(v.layout), Message::SetLayout),
        radio("List", Layout::List, Some(v.layout), Message::SetLayout),
        radio("Icons", Layout::Icons, Some(v.layout), Message::SetLayout),
    ]
    .spacing(6);

    let size = column![
        title("ICON SIZE"),
        radio("Small", IconSize::Small, Some(v.icon_size), Message::SetIconSize),
        radio("Medium", IconSize::Medium, Some(v.icon_size), Message::SetIconSize),
        radio("Large", IconSize::Large, Some(v.icon_size), Message::SetIconSize),
        radio("Extra large", IconSize::ExtraLarge, Some(v.icon_size), Message::SetIconSize),
    ]
    .spacing(6);

    let sort = column![
        title("SORT BY"),
        radio("Name", SortKey::Name, Some(v.sort), Message::SetSort),
        radio("Size", SortKey::Size, Some(v.sort), Message::SetSort),
        radio("Date modified", SortKey::Modified, Some(v.sort), Message::SetSort),
        radio("Type", SortKey::Kind, Some(v.sort), Message::SetSort),
        row![
            radio("Ascending", false, Some(v.descending), Message::SetDescending),
            radio("Descending", true, Some(v.descending), Message::SetDescending),
        ]
        .spacing(12),
    ]
    .spacing(6);

    let hint = text("Ctrl+1/2/3 layout · Ctrl+wheel or Ctrl+= / Ctrl+- size")
        .size(11)
        .color(c.muted);

    container(column![layout, size, sort, hint].spacing(16))
        .padding(14)
        .width(WIDTH)
        .style(move |_theme: &Theme| container::Style {
            // Always solid, even when the window is see-through, so the
            // menu stays readable on top of the file list.
            background: Some(iced::Color { a: 1.0, ..c.raised }.into()),
            border: iced::Border {
                radius: RADIUS.into(),
                width: 1.0,
                color: c.muted.scale_alpha(0.3),
            },
            shadow: iced::Shadow {
                color: iced::Color::BLACK.scale_alpha(0.35),
                offset: iced::Vector::new(0.0, 6.0),
                blur_radius: 18.0,
            },
            ..container::Style::default()
        })
        .into()
}
