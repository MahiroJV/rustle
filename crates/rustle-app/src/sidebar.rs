//! The sidebar: a "Home page" entry, then three sections (Favorites, Places,
//! Drives). Every folder can be opened with ▸ like Explorer's left pane;
//! sub-folders are read from disk only the first time a node is opened.

use std::path::{Path, PathBuf};

use iced::widget::{Column, button, column, mouse_area, row, scrollable, space, svg, text};
use iced::{Element, Fill};
use rustle_core::expand_chain;

use crate::icons::{self, Ui};
use crate::message::Message;
use crate::tab::shorten;
use crate::theme::{Colors, nav_row, row_button};

/// How far each tree level is pushed to the right, in pixels.
const INDENT: f32 = 12.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Section {
    Favorites,
    Places,
    Drives,
}

/// Which icon a node shows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeIcon {
    /// Chosen from the folder name (src, Downloads…), like in the file list.
    Folder,
    /// A user folder (Home, Pictures…), by its label.
    Place,
    Drive,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub path: PathBuf,
    pub label: String,
    pub icon: NodeIcon,
    pub hidden: bool,
    pub expanded: bool,
    /// `None` = not read from disk yet.
    pub children: Option<Vec<Node>>,
    pub loading: bool,
}

impl Node {
    pub fn new(path: PathBuf, label: String, icon: NodeIcon) -> Self {
        Self {
            hidden: label.starts_with('.'),
            path,
            label,
            icon,
            expanded: false,
            children: None,
            loading: false,
        }
    }

    fn find_mut(&mut self, path: &Path) -> Option<&mut Node> {
        if self.path == path {
            return Some(self);
        }
        // Only look inside nodes whose path is a parent of `path`.
        if !path.starts_with(&self.path) {
            return None;
        }
        self.children.as_mut()?.iter_mut().find_map(|c| c.find_mut(path))
    }
}

#[derive(Debug, Default)]
pub struct Sidebar {
    pub favorites: Vec<Node>,
    pub places: Vec<Node>,
    pub drives: Vec<Node>,
}

impl Sidebar {
    fn roots_mut(&mut self, section: Section) -> &mut Vec<Node> {
        match section {
            Section::Favorites => &mut self.favorites,
            Section::Places => &mut self.places,
            Section::Drives => &mut self.drives,
        }
    }

    pub fn find_mut(&mut self, section: Section, path: &Path) -> Option<&mut Node> {
        self.roots_mut(section).iter_mut().find_map(|n| n.find_mut(path))
    }

    /// Replace a section's top-level nodes. Nodes whose path is still there
    /// keep their state (opened, loaded children), so refreshing the drive
    /// list doesn't fold the tree back up.
    pub fn set_roots(&mut self, section: Section, roots: Vec<Node>) {
        let old = std::mem::take(self.roots_mut(section));
        let merged = roots
            .into_iter()
            .map(|new| match old.iter().find(|o| o.path == new.path) {
                Some(o) => Node { label: new.label, ..o.clone() },
                None => new,
            })
            .collect();
        *self.roots_mut(section) = merged;
    }

    /// ▸/▾ clicked. Returns the path if its sub-folders must be read now.
    pub fn toggle(&mut self, section: Section, path: &Path) -> Option<PathBuf> {
        let node = self.find_mut(section, path)?;
        node.expanded = !node.expanded;
        start_loading_if_needed(node)
    }

    /// Sub-folders arrived from disk (an error gives an empty list).
    pub fn set_children(&mut self, section: Section, path: &Path, children: Vec<Node>) {
        if let Some(node) = self.find_mut(section, path) {
            node.children = Some(children);
            node.loading = false;
        }
    }

    /// Open the tree down to `cwd`, in the section whose top-level folder
    /// contains it most closely. Returns the folders whose sub-folders must be
    /// read first; call again once they arrive to continue deeper.
    pub fn reveal(&mut self, cwd: &Path) -> Vec<(Section, PathBuf)> {
        let order = [Section::Favorites, Section::Places, Section::Drives];
        let roots: Vec<(Section, PathBuf)> = order
            .iter()
            .flat_map(|&s| {
                let list = match s {
                    Section::Favorites => &self.favorites,
                    Section::Places => &self.places,
                    Section::Drives => &self.drives,
                };
                list.iter().map(move |n| (s, n.path.clone()))
            })
            .collect();
        let paths: Vec<PathBuf> = roots.iter().map(|(_, p)| p.clone()).collect();
        let Some((index, chain)) = expand_chain(&paths, cwd) else {
            return Vec::new();
        };
        let (section, root) = roots[index].clone();

        // Open the root and every folder on the way, but not `cwd` itself:
        // it only needs to be visible (and highlighted), not opened.
        let mut to_open = vec![root];
        to_open.extend(chain);
        to_open.pop();

        let mut loads = Vec::new();
        for path in to_open {
            let Some(node) = self.find_mut(section, &path) else {
                break; // its parent's sub-folders aren't loaded yet
            };
            node.expanded = true;
            if let Some(load) = start_loading_if_needed(node) {
                loads.push((section, load));
                break; // can't go deeper until this one is read
            }
            if node.loading {
                break;
            }
        }
        loads
    }
}

fn start_loading_if_needed(node: &mut Node) -> Option<PathBuf> {
    if node.expanded && node.children.is_none() && !node.loading {
        node.loading = true;
        Some(node.path.clone())
    } else {
        None
    }
}

// ---------------------------------------------------------------- drawing

/// Everything `view` needs to know about the app.
pub struct ViewState<'a> {
    pub cwd: Option<&'a Path>,
    pub on_home_page: bool,
    pub show_hidden: bool,
    pub c: Colors,
}

pub fn view<'a>(sidebar: &'a Sidebar, s: ViewState<'a>) -> Element<'a, Message> {
    let c = s.c;
    let home_icon = if s.on_home_page { c.text } else { c.muted };
    let mut col = column![
        button(row![icons::ui(Ui::Home, home_icon, 16.0), text("Home page").size(14)].spacing(10))
            .width(Fill)
            .padding([5, 8])
            .style(nav_row(c, s.on_home_page))
            .on_press(Message::ShowHomePage),
    ]
    .spacing(2);

    let sections = [
        ("FAVORITES", Section::Favorites, &sidebar.favorites),
        ("PLACES", Section::Places, &sidebar.places),
        ("DRIVES", Section::Drives, &sidebar.drives),
    ];
    for (title, section, nodes) in sections {
        if nodes.is_empty() {
            continue; // e.g. no favorites yet
        }
        col = col.push(space().height(10));
        col = col.push(text(title).size(11).color(c.muted));
        for node in nodes.iter() {
            col = node_view(col, node, section, 0, &s);
        }
    }
    scrollable(col).height(Fill).into()
}

/// One node and (if open) its children, indented by `depth`.
fn node_view<'a>(
    mut col: Column<'a, Message>,
    node: &'a Node,
    section: Section,
    depth: u16,
    s: &ViewState<'a>,
) -> Column<'a, Message> {
    if node.hidden && !s.show_hidden && depth > 0 {
        return col;
    }
    let c = s.c;
    let is_current = !s.on_home_page && s.cwd == Some(node.path.as_path());

    let chevron_icon = if node.expanded { Ui::ChevronDown } else { Ui::ChevronRight };
    // A loaded folder without sub-folders gets no arrow (nothing to open).
    let has_no_children = node.children.as_ref().is_some_and(|ch| {
        ch.iter().all(|n| n.hidden && !s.show_hidden)
    });
    let chevron: Element<'a, Message> = if has_no_children {
        space().width(20).into()
    } else {
        button(icons::ui(chevron_icon, c.muted, 14.0))
            .padding(3)
            .style(row_button(c))
            .on_press(Message::SidebarToggle(section, node.path.clone()))
            .into()
    };

    let icon = match node.icon {
        NodeIcon::Folder => svg(icons::file_icon(&node.label, true)),
        NodeIcon::Place => svg(icons::place_icon(&node.label)),
        NodeIcon::Drive => icons::ui(Ui::Drive, c.text, 16.0),
    }
    .width(16)
    .height(16);

    let max_chars = 22usize.saturating_sub(depth as usize * 2).max(8);
    let label = button(row![icon, text(shorten(&node.label, max_chars)).size(14)].spacing(8))
        .width(Fill)
        .padding([4, 6])
        .style(nav_row(c, is_current))
        .on_press(Message::Navigate(node.path.clone()));
    // Middle-click opens the folder in a new tab.
    let label = mouse_area(label).on_middle_press(Message::NewTab(Some(node.path.clone())));

    col = col.push(
        row![space().width(f32::from(depth) * INDENT), chevron, label]
            .spacing(2)
            .align_y(iced::Center),
    );

    if node.expanded {
        if let Some(children) = &node.children {
            for child in children {
                col = node_view(col, child, section, depth + 1, s);
            }
        } else if node.loading {
            col = col.push(
                row![
                    space().width(f32::from(depth + 1) * INDENT + 24.0),
                    text("Loading…").size(12).color(c.muted)
                ],
            );
        }
    }
    col
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(path: &str) -> Node {
        let label = Path::new(path).file_name().map_or(path.into(), |n| n.to_string_lossy().into());
        Node::new(path.into(), label, NodeIcon::Folder)
    }

    fn sidebar_with_home() -> Sidebar {
        let mut s = Sidebar::default();
        s.set_roots(Section::Places, vec![folder("/h")]);
        s.set_roots(Section::Drives, vec![folder("/")]);
        s
    }

    #[test]
    fn reveal_opens_step_by_step() {
        let mut s = sidebar_with_home();
        // /h is the deepest root containing /h/a/b → open /h, load its children.
        assert_eq!(s.reveal(Path::new("/h/a/b")), vec![(Section::Places, PathBuf::from("/h"))]);
        assert!(s.places[0].expanded);
        // Children of /h arrive; revealing again continues one level deeper.
        s.set_children(Section::Places, Path::new("/h"), vec![folder("/h/a"), folder("/h/z")]);
        assert_eq!(s.reveal(Path::new("/h/a/b")), vec![(Section::Places, PathBuf::from("/h/a"))]);
        s.set_children(Section::Places, Path::new("/h/a"), vec![folder("/h/a/b")]);
        // Now everything on the way is open, and /h/a/b itself is not opened.
        assert!(s.reveal(Path::new("/h/a/b")).is_empty());
        let b = s.find_mut(Section::Places, Path::new("/h/a/b")).unwrap();
        assert!(!b.expanded);
        // The drive tree was left alone.
        assert!(!s.drives[0].expanded);
    }

    #[test]
    fn toggle_loads_once_then_just_folds() {
        let mut s = sidebar_with_home();
        assert_eq!(s.toggle(Section::Drives, Path::new("/")), Some(PathBuf::from("/")));
        s.set_children(Section::Drives, Path::new("/"), vec![folder("/etc")]);
        assert_eq!(s.toggle(Section::Drives, Path::new("/")), None); // closes
        assert!(!s.drives[0].expanded);
        assert_eq!(s.toggle(Section::Drives, Path::new("/")), None); // reopens, already loaded
        assert!(s.drives[0].expanded);
    }

    #[test]
    fn refreshing_roots_keeps_open_state() {
        let mut s = sidebar_with_home();
        s.toggle(Section::Drives, Path::new("/"));
        s.set_children(Section::Drives, Path::new("/"), vec![folder("/etc")]);
        s.set_roots(Section::Drives, vec![folder("/"), folder("/mnt/usb")]);
        assert!(s.drives[0].expanded);
        assert!(s.drives[0].children.is_some());
        assert_eq!(s.drives.len(), 2);
    }

    #[test]
    fn same_path_in_two_sections_is_separate() {
        let mut s = Sidebar::default();
        s.set_roots(Section::Favorites, vec![folder("/h")]);
        s.set_roots(Section::Places, vec![folder("/h")]);
        s.toggle(Section::Places, Path::new("/h"));
        assert!(s.places[0].expanded);
        assert!(!s.favorites[0].expanded);
    }
}
