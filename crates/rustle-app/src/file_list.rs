//! The main list of files and folders, in three layouts:
//! Details (columns), List (compact), Icons (grid).

use std::collections::HashMap;
use std::path::PathBuf;

use iced::widget::{Column, button, column, container, grid, image, row, scrollable, svg, text};
use iced::{ContentFit, Element, Fill, Length};
use rustle_core::{Entry, FileKind, IconSize, Layout, SortKey, ViewSettings, extension, file_kind, format_size};

use crate::dates::format_modified;
use crate::icons;
use crate::message::Message;
use crate::tab::shorten;
use crate::theme::{Colors, ROW_HEIGHT, row_button};
use crate::thumbs::Thumbs;

// Column widths in the Details layout (Name takes the rest).
const ITEMS_W: f32 = 90.0;
const SIZE_W: f32 = 90.0;
const MODIFIED_W: f32 = 150.0;
const TYPE_W: f32 = 110.0;

/// Icon size in pixels for rows (Details and List).
pub fn row_icon_px(size: IconSize) -> f32 {
    match size {
        IconSize::Small => 16.0,
        IconSize::Medium => 20.0,
        IconSize::Large => 24.0,
        IconSize::ExtraLarge => 32.0,
    }
}

/// Icon size in pixels for the Icons grid.
pub fn grid_icon_px(size: IconSize) -> f32 {
    match size {
        IconSize::Small => 32.0,
        IconSize::Medium => 48.0,
        IconSize::Large => 64.0,
        IconSize::ExtraLarge => 96.0,
    }
}

pub fn view<'a>(
    entries: &'a [Entry],
    show_hidden: bool,
    view: ViewSettings,
    counts: &'a HashMap<PathBuf, Option<u64>>,
    thumbs: &'a Thumbs,
    c: Colors,
) -> Element<'a, Message> {
    let visible = entries.iter().filter(move |e| show_hidden || !e.hidden);
    match view.layout {
        Layout::Details => details(visible, view, counts, c),
        Layout::List => list(visible, view.icon_size, c),
        Layout::Icons => {
            let thumbs = view.previews.then_some(thumbs);
            icons_grid(visible, view.icon_size, thumbs, c)
        }
    }
}

/// What clicking an entry does: open a folder here, or open a file in its app.
fn open_message(e: &Entry) -> Message {
    if e.is_dir {
        Message::Navigate(e.path.clone())
    } else {
        Message::OpenFile(e.path.clone())
    }
}

fn icon<'a>(e: &Entry, px: f32) -> svg::Svg<'a> {
    svg(icons::file_icon(&e.name, e.is_dir)).width(px).height(px)
}

/// "Image", "Code"… or, for unknown types, "XYZ file" from the extension.
pub fn type_label(e: &Entry) -> String {
    match file_kind(&e.name, e.is_dir) {
        FileKind::Other => match extension(&e.name) {
            Some(ext) => format!("{} file", ext.to_uppercase()),
            None => "File".into(),
        },
        kind => kind.label().into(),
    }
}

fn details<'a>(
    entries: impl Iterator<Item = &'a Entry>,
    view: ViewSettings,
    counts: &'a HashMap<PathBuf, Option<u64>>,
    c: Colors,
) -> Element<'a, Message> {
    // A clickable column title; the sorted column shows ▲ or ▼.
    let column_title = |label: &'static str, key: SortKey, width: Length| {
        let arrow = match (view.sort == key, view.descending) {
            (true, false) => " ▲",
            (true, true) => " ▼",
            (false, _) => "",
        };
        button(text(format!("{label}{arrow}")).size(13).color(c.muted))
            .width(width)
            .padding([2, 4])
            .style(row_button(c))
            .on_press(Message::SortByColumn(key))
    };
    let header = row![
        column_title("Name", SortKey::Name, Fill),
        // Item counts aren't a sort key, so this title isn't a button.
        container(text("Items").size(13).color(c.muted)).width(ITEMS_W).padding([2, 4]),
        column_title("Size", SortKey::Size, SIZE_W.into()),
        column_title("Modified", SortKey::Modified, MODIFIED_W.into()),
        column_title("Type", SortKey::Kind, TYPE_W.into()),
    ]
    .padding([0, 8]);

    let px = row_icon_px(view.icon_size);
    let rows = entries.fold(Column::new().spacing(1), |col, e| {
        let items = if e.is_dir {
            match counts.get(&e.path) {
                Some(Some(n)) => format!("{n} items"),
                Some(None) => "—".into(), // couldn't read the folder
                None => String::new(),    // still counting
            }
        } else {
            String::new()
        };
        let size = if e.is_dir { String::new() } else { format_size(e.size) };
        let cell = |s: String, w: f32| text(s).size(13).color(c.muted).width(w);
        let line = row![
            row![icon(e, px), text(&e.name)].spacing(8).align_y(iced::Center).width(Fill),
            cell(items, ITEMS_W),
            cell(size, SIZE_W),
            cell(format_modified(e.modified), MODIFIED_W),
            cell(type_label(e), TYPE_W),
        ]
        .spacing(0)
        .align_y(iced::Center);
        col.push(row_button_for(e, line, px, c))
    });

    column![header, scrollable(rows).height(Fill)].spacing(6).into()
}

fn list<'a>(
    entries: impl Iterator<Item = &'a Entry>,
    size: IconSize,
    c: Colors,
) -> Element<'a, Message> {
    let px = row_icon_px(size);
    let rows = entries.fold(Column::new().spacing(1), |col, e| {
        let line = row![icon(e, px), text(&e.name)].spacing(8).align_y(iced::Center);
        col.push(row_button_for(e, line, px, c))
    });
    scrollable(rows).height(Fill).into()
}

/// One full-width row; it grows with bigger icons.
fn row_button_for<'a>(
    e: &Entry,
    line: impl Into<Element<'a, Message>>,
    icon_px: f32,
    c: Colors,
) -> Element<'a, Message> {
    button(line)
        .width(Fill)
        .height(ROW_HEIGHT.max(icon_px + 10.0))
        .padding([4, 12])
        .style(row_button(c))
        .on_press(open_message(e))
        .into()
}

/// `thumbs` is `Some` when picture previews are on.
fn icons_grid<'a>(
    entries: impl Iterator<Item = &'a Entry>,
    size: IconSize,
    thumbs: Option<&'a Thumbs>,
    c: Colors,
) -> Element<'a, Message> {
    let px = grid_icon_px(size);
    let cells = entries.map(|e| {
        // The thumbnail if previews are on and it's ready; otherwise the icon
        // (also while the thumbnail is still being made).
        let picture: Element<'a, Message> = match thumbs.and_then(|t| t.get(&e.path)) {
            Some(handle) => image(handle.clone())
                .width(px)
                .height(px)
                .content_fit(ContentFit::Contain)
                .into(),
            None => icon(e, px).into(),
        };
        let cell = column![
            picture,
            // Long names are shortened so one cell can't grow huge.
            text(shorten(&e.name, 40)).size(12).center().width(Fill),
        ]
        .spacing(6)
        .align_x(iced::Center);
        button(cell)
            .width(Fill)
            .height(Fill)
            .padding(6)
            .style(row_button(c))
            .on_press(open_message(e))
            .into()
    });

    // `fluid`: as many columns as fit, each at most this wide. Cells are a
    // bit taller than wide (ratio < 1) to leave room for two lines of name.
    let cells = grid(cells).fluid(px + 64.0).height(grid::Sizing::AspectRatio(0.8)).spacing(6);
    scrollable(cells).height(Fill).into()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, is_dir: bool) -> Entry {
        Entry {
            name: name.into(),
            path: name.into(),
            is_dir,
            size: 0,
            hidden: false,
            modified: None,
        }
    }

    #[test]
    fn type_labels() {
        assert_eq!(type_label(&entry("src", true)), "Folder");
        assert_eq!(type_label(&entry("main.rs", false)), "Code");
        assert_eq!(type_label(&entry("data.xyz", false)), "XYZ file");
        assert_eq!(type_label(&entry("Makefile", false)), "File");
    }

    #[test]
    fn icons_grow_with_size() {
        let mut last = 0.0;
        for s in IconSize::ALL {
            assert!(row_icon_px(s) > last);
            last = row_icon_px(s);
        }
        // The smallest grid icon is at least as big as the biggest row icon.
        assert!(grid_icon_px(IconSize::Small) >= row_icon_px(IconSize::ExtraLarge));
    }
}
