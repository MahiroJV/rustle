//! The main list of files and folders, in three layouts:
//! Details (columns), List (compact), Icons (grid).

use std::collections::HashMap;
use std::path::PathBuf;

use iced::widget::text::Wrapping;
use iced::widget::{
    Column, button, column, container, image, responsive, row, scrollable, space, svg, text,
};
use iced::{ContentFit, Element, Fill, Length};
use rustle_core::{
    Entry, FileKind, IconSize, Layout, SortKey, ViewSettings, extension, file_kind, format_size,
};

use crate::dates::format_modified;
use crate::icons;
use crate::message::Message;
use crate::textfit::{fit_chars, shorten_middle};
use crate::theme::{Colors, ROW_HEIGHT, row_button};
use crate::thumbs::Thumbs;

// Column widths in the Details layout (Name takes the rest).
const ITEMS_W: f32 = 90.0;
const SIZE_W: f32 = 90.0;
const MODIFIED_W: f32 = 150.0;
const TYPE_W: f32 = 110.0;

/// The file list's scroll area id, so the app can scroll it (e.g. back to
/// the top when a new folder opens).
pub const LIST_ID: &str = "file-list";

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
    scroll_y: f32,
    c: Colors,
) -> Element<'a, Message> {
    // Only references, so this is cheap even for thousands of entries.
    let visible: Vec<&'a Entry> = entries
        .iter()
        .filter(|e| show_hidden || !e.hidden)
        .collect();
    match view.layout {
        Layout::Details => details(visible, view, counts, scroll_y, c),
        Layout::List => list(visible, view.icon_size, scroll_y, c),
        Layout::Icons => {
            let thumbs = view.previews.then_some(thumbs);
            icons_grid(visible, view.icon_size, thumbs, scroll_y, c)
        }
    }
}

/// Rows built above and below the visible ones, so fast scrolling doesn't
/// show empty space for a frame.
pub const OVERSCAN: usize = 10;

/// Which rows (by index) to build: the ones on screen plus `OVERSCAN` more
/// on each side. `pitch` is the height of one row including the gap.
pub fn visible_range(
    count: usize,
    pitch: f32,
    offset_y: f32,
    viewport_h: f32,
) -> std::ops::Range<usize> {
    if count == 0 {
        return 0..0;
    }
    // Zero, negative or NaN ("not a number"): can't compute, build everything.
    if pitch.is_nan() || pitch <= 0.0 {
        return 0..count;
    }
    let top = offset_y.max(0.0); // negative when "bouncing" past the top
    let first = (top / pitch).floor() as usize;
    let last = ((top + viewport_h.max(0.0)) / pitch).ceil() as usize;
    first.saturating_sub(OVERSCAN).min(count)..(last + OVERSCAN).min(count)
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
    svg(icons::file_icon(&e.name, e.is_dir))
        .width(px)
        .height(px)
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

/// A scrolling list that only builds the rows on screen (a "virtual list").
/// The rows above and below are replaced by empty space of the same height,
/// so the scrollbar looks exactly as if all rows were there.
///
/// - `total`: number of rows, `pitch`: height of one row incl. the gap,
/// - `scroll_y`: how far the list is scrolled (from `Message::ListScrolled`),
/// - `viewport_h`: the visible height (from `responsive`),
/// - `make_row(i)`: builds row number `i`.
fn virtual_rows<'a>(
    total: usize,
    pitch: f32,
    scroll_y: f32,
    viewport_h: f32,
    make_row: impl Fn(usize) -> Element<'a, Message>,
) -> Element<'a, Message> {
    let range = visible_range(total, pitch, scroll_y, viewport_h);
    let above = range.start as f32 * pitch;
    let below = (total - range.end) as f32 * pitch;

    let mut rows = Column::new().push(space().height(above));
    for i in range {
        // Each row gets exactly `pitch` of height, so the math above holds.
        rows = rows.push(container(make_row(i)).height(pitch));
    }
    rows = rows.push(space().height(below));

    scrollable(rows)
        .id(LIST_ID)
        .on_scroll(|viewport| Message::ListScrolled(viewport.absolute_offset().y))
        .height(Fill)
        .into()
}

fn details<'a>(
    entries: Vec<&'a Entry>,
    view: ViewSettings,
    counts: &'a HashMap<PathBuf, Option<u64>>,
    scroll_y: f32,
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
        container(text("Items").size(13).color(c.muted))
            .width(ITEMS_W)
            .padding([2, 4]),
        column_title("Size", SortKey::Size, SIZE_W.into()),
        column_title("Modified", SortKey::Modified, MODIFIED_W.into()),
        column_title("Type", SortKey::Kind, TYPE_W.into()),
    ]
    .padding([0, 8]);

    let px = row_icon_px(view.icon_size);
    let pitch = row_height(px) + ROW_GAP;
    let list = responsive(move |size| {
        // The Name column gets what the other columns leave over.
        let other_columns = ITEMS_W + SIZE_W + MODIFIED_W + TYPE_W;
        let name_w = size.width - other_columns - px - ROW_CHROME_W;
        virtual_rows(entries.len(), pitch, scroll_y, size.height, |i| {
            let e = entries[i];
            let items = if e.is_dir {
                match counts.get(&e.path) {
                    Some(Some(n)) => format!("{n} items"),
                    Some(None) => "—".into(), // couldn't read the folder
                    None => String::new(),    // still counting
                }
            } else {
                String::new()
            };
            let size = if e.is_dir {
                String::new()
            } else {
                format_size(e.size)
            };
            let cell = |s: String, w: f32| text(s).size(13).color(c.muted).width(w);
            let line = row![
                row![icon(e, px), name_cell(&e.name, name_w)]
                    .spacing(8)
                    .align_y(iced::Center)
                    .width(Fill),
                cell(items, ITEMS_W),
                cell(size, SIZE_W),
                cell(format_modified(e.modified), MODIFIED_W),
                cell(type_label(e), TYPE_W),
            ]
            .align_y(iced::Center);
            row_button_for(e, line, px, c)
        })
    });

    column![header, list].spacing(6).into()
}

fn list<'a>(
    entries: Vec<&'a Entry>,
    size: IconSize,
    scroll_y: f32,
    c: Colors,
) -> Element<'a, Message> {
    let px = row_icon_px(size);
    let pitch = row_height(px) + ROW_GAP;
    responsive(move |area| {
        let name_w = area.width - px - ROW_CHROME_W;
        virtual_rows(entries.len(), pitch, scroll_y, area.height, |i| {
            let e = entries[i];
            let line = row![icon(e, px), name_cell(&e.name, name_w)]
                .spacing(8)
                .align_y(iced::Center);
            row_button_for(e, line, px, c)
        })
    })
    .into()
}

/// Space between rows, in pixels.
const ROW_GAP: f32 = 1.0;
/// Size of file names in rows.
const NAME_FONT: f32 = 15.0;
/// Width inside a row that isn't text: padding (12 + 12), the gap after the
/// icon (8) and room for the scrollbar (12).
const ROW_CHROME_W: f32 = 44.0;

/// A file name on one line that never spills over its neighbors:
/// - shortened with "…" in the middle to roughly fit `width_px`,
/// - no wrapping (rows have a fixed height),
/// - clipped, in case the letter-width estimate is a little off.
fn name_cell<'a>(name: &str, width_px: f32) -> Element<'a, Message> {
    let fitted = shorten_middle(name, fit_chars(width_px, NAME_FONT));
    container(text(fitted).size(NAME_FONT).wrapping(Wrapping::None))
        .width(Fill)
        .clip(true)
        .into()
}

/// Row height for a given icon size: at least `ROW_HEIGHT`, growing with
/// bigger icons.
fn row_height(icon_px: f32) -> f32 {
    ROW_HEIGHT.max(icon_px + 10.0)
}

/// One full-width row.
fn row_button_for<'a>(
    e: &Entry,
    line: impl Into<Element<'a, Message>>,
    icon_px: f32,
    c: Colors,
) -> Element<'a, Message> {
    button(line)
        .width(Fill)
        .height(row_height(icon_px))
        .padding([4, 12])
        .style(row_button(c))
        .on_press(open_message(e))
        .into()
}

/// `thumbs` is `Some` when picture previews are on.
fn icons_grid<'a>(
    entries: Vec<&'a Entry>,
    size: IconSize,
    thumbs: Option<&'a Thumbs>,
    scroll_y: f32,
    c: Colors,
) -> Element<'a, Message> {
    const GAP: f32 = 6.0;
    const GRID_NAME_FONT: f32 = 12.0;
    let px = grid_icon_px(size);
    // Room for the icon, two lines of name and padding.
    let (cell_w, cell_h) = (px + 64.0, px + 60.0);

    responsive(move |area| {
        // How many cells fit side by side; the grid is then "virtual" by rows.
        let columns = grid_columns(area.width, cell_w, GAP);
        let total_rows = entries.len().div_ceil(columns);
        // The real cell width once the columns share the row; names may use
        // two lines of it (minus the cell padding).
        let real_cell_w = (area.width - GAP * (columns - 1) as f32) / columns as f32;
        let name_chars = 2 * fit_chars(real_cell_w - 12.0, GRID_NAME_FONT) - 1;
        virtual_rows(total_rows, cell_h + GAP, scroll_y, area.height, |r| {
            let mut line = row![].spacing(GAP);
            for i in r * columns..((r + 1) * columns) {
                let Some(&e) = entries.get(i) else {
                    // Last row not full: empty slots keep the cells equally wide.
                    line = line.push(space().width(Fill));
                    continue;
                };
                // The thumbnail if previews are on and it's ready; otherwise
                // the icon (also while the thumbnail is still being made).
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
                    // At most two lines; clipped so a third can never show.
                    container(
                        text(shorten_middle(&e.name, name_chars))
                            .size(GRID_NAME_FONT)
                            .wrapping(Wrapping::WordOrGlyph)
                            .center()
                            .width(Fill),
                    )
                    .height(GRID_NAME_FONT * 2.6)
                    .clip(true),
                ]
                .spacing(6)
                .align_x(iced::Center);
                line = line.push(
                    button(cell)
                        .width(Fill)
                        .height(cell_h)
                        .padding(6)
                        .style(row_button(c))
                        .on_press(open_message(e)),
                );
            }
            line.into()
        })
    })
    .into()
}

/// How many grid cells of `cell_w` fit in `width` (at least 1).
pub fn grid_columns(width: f32, cell_w: f32, gap: f32) -> usize {
    (((width + gap) / (cell_w + gap)).floor() as usize).max(1)
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
    fn at_the_top_only_a_screenful_is_built() {
        // 5,574 rows of 29 px, a 600 px window: rows 0..21 visible + 10 extra.
        assert_eq!(visible_range(5574, 29.0, 0.0, 600.0), 0..31);
    }

    #[test]
    fn in_the_middle() {
        // Scrolled to row 100: rows 100..121 visible, 10 extra on each side.
        assert_eq!(visible_range(5574, 29.0, 2900.0, 600.0), 90..131);
    }

    #[test]
    fn at_the_end_it_stops_at_the_last_row() {
        assert_eq!(visible_range(5574, 29.0, 5574.0 * 29.0, 600.0), 5564..5574);
    }

    #[test]
    fn short_lists_are_built_completely() {
        assert_eq!(visible_range(7, 29.0, 0.0, 600.0), 0..7);
    }

    #[test]
    fn odd_inputs_dont_panic() {
        assert_eq!(visible_range(0, 29.0, 0.0, 600.0), 0..0);
        assert_eq!(visible_range(50, 29.0, -40.0, 600.0), 0..31); // overscroll bounce
        assert_eq!(visible_range(50, 0.0, 0.0, 600.0), 0..50); // nonsense pitch → everything
    }

    #[test]
    fn grid_columns_fit_the_width() {
        assert_eq!(grid_columns(1000.0, 112.0, 6.0), 8); // 8×112 + 7×6 = 938 ≤ 1000
        assert_eq!(grid_columns(50.0, 112.0, 6.0), 1); // never 0
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

#[cfg(test)]
mod perf_probe {
    use super::*;
    use rustle_core::FileSystem;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn time_building_bin() {
        let mut entries = rustle_fs::LocalFs
            .list(std::path::Path::new("/usr/bin"))
            .unwrap();
        rustle_core::sort_entries(&mut entries);
        let counts: HashMap<PathBuf, Option<u64>> = HashMap::new();
        let thumbs = Thumbs::default();
        let c = crate::theme::palette(rustle_core::ThemeChoice::Dark);
        let _ = (&counts, &thumbs);
        // What one screen update builds now: the visible rows of a ~900 px
        // window, using the same row builder as the List layout.
        let pitch = row_height(row_icon_px(IconSize::Medium)) + ROW_GAP;
        for (label, total) in [
            ("all rows (old way)", entries.len()),
            ("virtual list (new)", entries.len()),
        ] {
            let t = Instant::now();
            for _ in 0..5 {
                let range = if label.starts_with("all") {
                    0..total
                } else {
                    visible_range(total, pitch, 0.0, 900.0)
                };
                let n = range.len();
                let mut col = Column::new();
                for i in range {
                    let e = &entries[i];
                    col = col.push(row_button_for(
                        e,
                        row![icon(e, 20.0), text(&e.name)],
                        20.0,
                        c,
                    ));
                }
                let _ = (col, n);
            }
            println!(
                "{label}: {} rows built, {:.2} ms per update",
                if label.starts_with("all") {
                    total
                } else {
                    visible_range(total, pitch, 0.0, 900.0).len()
                },
                t.elapsed().as_secs_f64() * 1000.0 / 5.0
            );
        }
        let t = Instant::now();
        for e in &entries {
            let _ = crate::dates::format_modified(e.modified);
        }
        println!(
            "format_modified x{}: {:.1} ms",
            entries.len(),
            t.elapsed().as_secs_f64() * 1000.0
        );
        let t = Instant::now();
        for e in &entries {
            let _ = icons::file_icon(&e.name, e.is_dir);
        }
        println!(
            "file_icon x{}: {:.1} ms",
            entries.len(),
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
}
