//! The app's state, how it changes (`update`) and how it looks (`view`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::keyboard::Modifiers;
use iced::widget::{button, column, container, image, mouse_area, opaque, row, space, stack, text};
use iced::{Element, Fill, Subscription, Task, Theme};
use rustle_core::{Entry, FileSystem, Layout, Settings, sort_entries, sort_entries_by};
use rustle_fs::LocalFs;
use rustle_fs::places::{self, Drive, Place};

use crate::icons::{self, Ui};
use crate::keybinds;
use crate::message::{Message, Nav};
use crate::pathbar::{self, LOCATION_ID};
use crate::sidebar::{self, Node, NodeIcon, Section, Sidebar};
use crate::tab::{Tab, home_page_marker, shorten};
use crate::theme::{self, Colors, SIDEBAR_WIDTH, panel, row_button, tab_button};
use crate::thumbs::Thumbs;
use crate::{file_list, home_page, settings_page, view_menu};

const TAB_TITLE_MAX: usize = 24;
const TAB_MAX_WIDTH: f32 = 220.0;
/// Counting items in thousands of folders isn't worth it; stop here.
const MAX_FOLDERS_TO_COUNT: usize = 5000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Files,
    Settings,
}

pub struct App {
    // The UI depends on the trait, not on LocalFs (dependency inversion).
    fs: Arc<dyn FileSystem>,
    settings: Settings,
    /// Whether the window was created see-through (fixed until restart).
    started_transparent: bool,
    page: Page,
    /// Never empty: closing the last tab is not allowed.
    tabs: Vec<Tab>,
    active: usize,
    next_tab_id: u64,
    /// Home folder, looked up once (the path bar shows it as "Home").
    home: PathBuf,
    /// The path bar shows a text field (true) instead of buttons (false).
    editing_location: bool,
    view_menu_open: bool,
    /// Which of Ctrl/Shift/Alt are held right now (for Ctrl + mouse wheel).
    modifiers: Modifiers,
    /// Picture thumbnails in memory + the queue of ones still to make.
    thumbs: Thumbs,
    /// The user's standard folders and the drives (Home page + sidebar).
    places: Vec<Place>,
    drives: Vec<Drive>,
    sidebar: Sidebar,
    error: Option<String>,
}

impl App {
    pub fn new(settings: Settings, started_transparent: bool) -> (Self, Task<Message>) {
        let fs: Arc<dyn FileSystem> = Arc::new(LocalFs);
        let home = fs.home();
        let mut app = Self {
            fs,
            settings,
            started_transparent,
            page: Page::Files,
            // Rustle starts on the Home page.
            tabs: vec![Tab::on_home_page(0, home.clone())],
            active: 0,
            next_tab_id: 1,
            home: home.clone(),
            editing_location: false,
            view_menu_open: false,
            modifiers: Modifiers::empty(),
            thumbs: Thumbs::default(),
            places: Vec::new(),
            drives: Vec::new(),
            sidebar: Sidebar::default(),
            error: None,
        };
        let task = Task::batch([app.load(0, home, Nav::Reload), load_places()]);
        (app, task)
    }

    fn tab(&self) -> &Tab {
        &self.tabs[self.active]
    }

    fn tab_by_id(&mut self, id: u64) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    /// The current palette, with backgrounds faded by the opacity setting.
    fn colors(&self) -> Colors {
        theme::palette(self.settings.theme).with_opacity(self.settings.opacity)
    }

    pub fn title(&self) -> String {
        format!("{} — Rustle", self.tab().title())
    }

    pub fn theme(&self) -> Theme {
        theme::iced_theme(self.settings.theme)
    }

    /// The window background. Its alpha is what makes the window see-through.
    pub fn style(&self, _theme: &Theme) -> iced::theme::Style {
        let c = self.colors();
        iced::theme::Style {
            background_color: c.bg,
            text_color: c.text,
        }
    }

    /// Keyboard shortcuts and mouse side buttons, active all the time.
    pub fn subscription(&self) -> Subscription<Message> {
        iced::event::listen_with(keybinds::on_event)
    }

    /// Read `dir` for one tab on a blocking thread, so a big or slow folder
    /// doesn't freeze the window.
    fn load(&mut self, tab_id: u64, dir: PathBuf, nav: Nav) -> Task<Message> {
        // Copied out first: `tab_by_id` below borrows `self` mutably.
        let (key, descending) = (self.settings.view.sort, self.settings.view.descending);
        let Some(tab) = self.tab_by_id(tab_id) else {
            return Task::none();
        };
        tab.request += 1;
        tab.loading = true;
        let request = tab.request;
        let fs = Arc::clone(&self.fs);

        Task::perform(
            async move {
                let read_dir = dir.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let mut entries = fs.list(&read_dir).map_err(|e| e.to_string())?;
                    sort_entries_by(&mut entries, key, descending);
                    Ok(entries)
                })
                .await
                .unwrap_or_else(|e| Err(format!("listing failed: {e}")));
                (dir, result)
            },
            move |(dir, result)| Message::Loaded {
                tab_id,
                request,
                dir,
                nav,
                result,
            },
        )
    }

    /// Load `dir` in the active tab.
    fn go(&mut self, dir: PathBuf, nav: Nav) -> Task<Message> {
        let id = self.tab().id;
        self.load(id, dir, nav)
    }

    /// Count the items inside every sub-folder of a tab's listing, in the
    /// background. `request` ties the result to this listing, so counts for
    /// a folder the user already left are thrown away.
    fn count_items(&self, tab_id: u64, request: u64, folders: Vec<PathBuf>) -> Task<Message> {
        if folders.is_empty() {
            return Task::none();
        }
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    folders
                        .into_iter()
                        .map(|dir| {
                            let n = rustle_fs::count_items(&dir).ok();
                            (dir, n)
                        })
                        .collect()
                })
                .await
                .unwrap_or_default()
            },
            move |counts| Message::ItemCounts {
                tab_id,
                request,
                counts,
            },
        )
    }

    /// Ask for thumbnails of the pictures in the active tab, if previews are
    /// on and the Icons layout is shown. Otherwise the waiting queue is
    /// emptied, so no work is done for pictures nobody can see.
    fn request_thumbs(&mut self) -> Task<Message> {
        let v = self.settings.view;
        if !(v.previews && v.layout == Layout::Icons) {
            self.thumbs.want(Vec::new());
            return Task::none();
        }
        let show_hidden = self.settings.show_hidden;
        let wanted: Vec<PathBuf> = self
            .tab()
            .entries
            .iter()
            .filter(|e| !e.is_dir && (show_hidden || !e.hidden))
            .filter(|e| rustle_fs::thumbs::is_previewable(&e.name, e.size))
            .map(|e| e.path.clone())
            .collect();
        self.thumbs.want(wanted);
        self.start_thumb_jobs()
    }

    /// Start background jobs for queued thumbnails, while slots are free.
    fn start_thumb_jobs(&mut self) -> Task<Message> {
        let jobs = self.thumbs.next_jobs().into_iter().map(|path| {
            Task::perform(
                async move {
                    let p = path.clone();
                    let handle = tokio::task::spawn_blocking(move || {
                        rustle_fs::thumbs::thumbnail(&p)
                            .ok()
                            .map(|t| image::Handle::from_rgba(t.width, t.height, t.rgba))
                    })
                    .await
                    .ok()
                    .flatten();
                    (path, handle)
                },
                |(path, handle)| Message::ThumbLoaded { path, handle },
            )
        });
        Task::batch(jobs)
    }

    /// Read one sidebar folder's sub-folders in the background.
    fn load_children(&self, section: Section, path: PathBuf) -> Task<Message> {
        let fs = Arc::clone(&self.fs);
        Task::perform(
            async move {
                let dir = path.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let mut folders: Vec<Entry> = fs
                        .list(&dir)
                        .map_err(|e| e.to_string())?
                        .into_iter()
                        .filter(|e| e.is_dir)
                        .collect();
                    sort_entries(&mut folders);
                    Ok(folders)
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()));
                (path, result)
            },
            move |(path, result)| Message::SidebarChildren {
                section,
                path,
                result,
            },
        )
    }

    /// Open the sidebar tree down to the active tab's folder (not on the
    /// Home page). Starts reading any folders it needs on the way.
    fn reveal_current(&mut self) -> Task<Message> {
        if self.tab().home_page {
            return Task::none();
        }
        let cwd = self.tab().cwd.clone();
        let loads = self.sidebar.reveal(&cwd);
        Task::batch(loads.into_iter().map(|(section, path)| self.load_children(section, path)))
    }

    /// Put the current favorites, folders and drives into the sidebar.
    fn refresh_sidebar_roots(&mut self) {
        let favorites = self
            .settings
            .favorites
            .iter()
            .map(|p| {
                let label = p.file_name().map_or_else(|| p.display().to_string(), |n| n.to_string_lossy().into_owned());
                Node::new(p.clone(), label, NodeIcon::Folder)
            })
            .collect();
        let places = self
            .places
            .iter()
            .map(|p| Node::new(p.path.clone(), p.label.clone(), NodeIcon::Place))
            .collect();
        let drives = self
            .drives
            .iter()
            .map(|d| Node::new(d.mount.clone(), d.label.clone(), NodeIcon::Drive))
            .collect();
        self.sidebar.set_roots(Section::Favorites, favorites);
        self.sidebar.set_roots(Section::Places, places);
        self.sidebar.set_roots(Section::Drives, drives);
    }

    /// Re-sort every tab after the sort order changed (no disk access needed).
    fn resort(&mut self) {
        let v = self.settings.view;
        for tab in &mut self.tabs {
            sort_entries_by(&mut tab.entries, v.sort, v.descending);
        }
    }

    /// Write the settings file on a blocking thread.
    fn save_settings(&self) -> Task<Message> {
        let settings = self.settings.clone();
        Task::perform(
            async move {
                tokio::task::spawn_blocking(move || {
                    rustle_fs::save_settings(&rustle_fs::settings_path(), &settings)
                        .map_err(|e| e.to_string())
                })
                .await
                .unwrap_or_else(|e| Err(e.to_string()))
            },
            Message::SettingsSaved,
        )
    }

    fn close_tab(&mut self, id: u64) {
        if self.tabs.len() == 1 {
            return; // keep at least one tab
        }
        let Some(index) = self.tabs.iter().position(|t| t.id == id) else {
            return;
        };
        self.tabs.remove(index);
        // Keep the same tab active if it's still there; otherwise the neighbor.
        if index < self.active || self.active >= self.tabs.len() {
            self.active -= 1;
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            // ---- browsing ----
            Message::Navigate(path) => {
                self.editing_location = false;
                return self.go(path, Nav::Fresh);
            }
            Message::Up => {
                // "Up" has no meaning on the Home page.
                if self.tab().home_page {
                    return Task::none();
                }
                if let Some(parent) = self.tab().cwd.parent().map(PathBuf::from) {
                    return self.go(parent, Nav::Fresh);
                }
            }
            Message::Back => {
                if let Some(target) = self.tab().history.peek_back().map(PathBuf::from) {
                    if target == home_page_marker() {
                        // Back to the Home page: nothing to load, switch now.
                        let tab = &mut self.tabs[self.active];
                        let here = tab.location();
                        tab.history.back(here);
                        tab.home_page = true;
                        return Task::none();
                    }
                    return self.go(target, Nav::Back);
                }
            }
            Message::Forward => {
                if let Some(target) = self.tab().history.peek_forward().map(PathBuf::from) {
                    if target == home_page_marker() {
                        let tab = &mut self.tabs[self.active];
                        let here = tab.location();
                        tab.history.forward(here);
                        tab.home_page = true;
                        return Task::none();
                    }
                    return self.go(target, Nav::Forward);
                }
            }
            Message::Refresh => {
                // On the Home page, F5 re-reads folders and drives (free space).
                if self.tab().home_page {
                    return load_places();
                }
                let cwd = self.tab().cwd.clone();
                return self.go(cwd, Nav::Reload);
            }
            Message::ShowHomePage => {
                self.page = Page::Files;
                self.editing_location = false;
                let tab = &mut self.tabs[self.active];
                if !tab.home_page {
                    let here = tab.location();
                    tab.history.visit(here);
                    tab.home_page = true;
                }
                // Free space may have changed since last time.
                return load_places();
            }
            Message::PlacesLoaded { places, drives } => {
                self.places = places;
                self.drives = drives;
                self.refresh_sidebar_roots();
                return self.reveal_current();
            }
            Message::SidebarToggle(section, path) => {
                if let Some(load) = self.sidebar.toggle(section, &path) {
                    return self.load_children(section, load);
                }
            }
            Message::SidebarChildren {
                section,
                path,
                result,
            } => {
                let children = match result {
                    Ok(folders) => folders
                        .into_iter()
                        .map(|e| Node::new(e.path, e.name, NodeIcon::Folder))
                        .collect(),
                    Err(e) => {
                        // e.g. no permission: the node just shows nothing inside.
                        self.error = Some(e);
                        Vec::new()
                    }
                };
                self.sidebar.set_children(section, &path, children);
                // If we were opening the tree towards the current folder, go on.
                return self.reveal_current();
            }
            Message::OpenFile(path) => {
                self.error = self.fs.open(&path).err().map(|e| e.to_string());
            }
            Message::Loaded {
                tab_id,
                request,
                dir,
                nav,
                result,
            } => {
                let Some(tab) = self.tab_by_id(tab_id) else {
                    return Task::none(); // the tab was closed meanwhile
                };
                if request != tab.request {
                    return Task::none(); // an older request; a newer one is coming
                }
                tab.loading = false;
                match result {
                    Ok(entries) => {
                        // Where the tab was (a folder, or the Home page).
                        let old = tab.location();
                        tab.cwd = dir;
                        // History changes only now, after a successful load.
                        match nav {
                            Nav::Fresh if old != tab.cwd => tab.history.visit(old),
                            Nav::Back => {
                                tab.history.back(old);
                            }
                            Nav::Forward => {
                                tab.history.forward(old);
                            }
                            Nav::Fresh | Nav::Reload => {}
                        }
                        // Going anywhere leaves the Home page; a reload
                        // (startup, F5) keeps it.
                        if nav != Nav::Reload {
                            tab.home_page = false;
                        }
                        let folders: Vec<PathBuf> = entries
                            .iter()
                            .filter(|e| e.is_dir)
                            .take(MAX_FOLDERS_TO_COUNT)
                            .map(|e| e.path.clone())
                            .collect();
                        tab.entries = entries;
                        tab.item_counts.clear();
                        tab.location_input = tab.cwd.display().to_string();
                        self.error = None;
                        let counting = self.count_items(tab_id, request, folders);
                        return Task::batch([
                            counting,
                            self.request_thumbs(),
                            self.reveal_current(),
                        ]);
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            Message::ItemCounts {
                tab_id,
                request,
                counts,
            } => {
                if let Some(tab) = self.tab_by_id(tab_id)
                    && tab.request == request
                {
                    tab.item_counts.extend(counts);
                }
            }

            // ---- view ----
            Message::ToggleViewMenu => self.view_menu_open = !self.view_menu_open,
            Message::CloseViewMenu => self.view_menu_open = false,
            Message::SetLayout(layout) => {
                self.settings.view.layout = layout;
                self.page = Page::Files;
                return Task::batch([self.save_settings(), self.request_thumbs()]);
            }
            Message::SetPreviews(on) => {
                self.settings.view.previews = on;
                return Task::batch([self.save_settings(), self.request_thumbs()]);
            }
            Message::ThumbLoaded { path, handle } => {
                self.thumbs.finished(path, handle);
                // A job slot is free now: start the next waiting thumbnail.
                return self.start_thumb_jobs();
            }
            Message::SetIconSize(size) => {
                self.settings.view.icon_size = size;
                return self.save_settings();
            }
            Message::ZoomIn => {
                self.settings.view.icon_size = self.settings.view.icon_size.bigger();
                return self.save_settings();
            }
            Message::ZoomOut => {
                self.settings.view.icon_size = self.settings.view.icon_size.smaller();
                return self.save_settings();
            }
            Message::SetSort(key) => {
                self.settings.view.sort = key;
                self.resort();
                return self.save_settings();
            }
            Message::SetDescending(descending) => {
                self.settings.view.descending = descending;
                self.resort();
                return self.save_settings();
            }
            Message::SortByColumn(key) => {
                self.settings.view.click_column(key);
                self.resort();
                return self.save_settings();
            }
            Message::ModifiersChanged(mods) => self.modifiers = mods,
            Message::Wheel(y) => {
                // Only Ctrl + wheel zooms; a plain wheel just scrolls the list.
                if self.modifiers.command() {
                    let size = self.settings.view.icon_size;
                    self.settings.view.icon_size = if y > 0.0 { size.bigger() } else { size.smaller() };
                    if self.settings.view.icon_size != size {
                        return self.save_settings();
                    }
                }
            }

            // ---- path bar ----
            Message::StartEditLocation => {
                self.editing_location = true;
                let tab = &mut self.tabs[self.active];
                tab.location_input = tab.cwd.display().to_string();
                // The text field appears in the next `view`; then these
                // put the cursor in it and select the text, ready to type over.
                return Task::batch([
                    iced::widget::operation::focus(LOCATION_ID),
                    iced::widget::operation::select_all(LOCATION_ID),
                ]);
            }
            Message::LocationChanged(input) => {
                self.tabs[self.active].location_input = input;
            }
            Message::LocationSubmit => {
                self.editing_location = false;
                let path = expand_home(self.tab().location_input.trim(), &self.home);
                return self.go(path, Nav::Fresh);
            }
            Message::Escape => {
                self.editing_location = false;
                self.view_menu_open = false;
            }

            // ---- tabs ----
            Message::NewTab(path) => {
                let id = self.next_tab_id;
                self.next_tab_id += 1;
                // A plain new tab (+, Ctrl+T) opens the Home page; a folder
                // opened in a new tab (middle-click) opens that folder.
                let (tab, dir) = match path {
                    Some(dir) => (Tab::new(id, dir.clone()), dir),
                    None => (Tab::on_home_page(id, self.home.clone()), self.home.clone()),
                };
                self.tabs.push(tab);
                self.active = self.tabs.len() - 1;
                self.page = Page::Files;
                self.editing_location = false;
                return self.load(id, dir, Nav::Reload);
            }
            Message::CloseTab(id) => self.close_tab(id),
            Message::CloseActiveTab => self.close_tab(self.tab().id),
            Message::SelectTab(index) => {
                if index < self.tabs.len() {
                    self.active = index;
                    self.error = None;
                    self.editing_location = false;
                    return Task::batch([self.request_thumbs(), self.reveal_current()]);
                }
            }
            Message::NextTab => {
                self.active = (self.active + 1) % self.tabs.len();
                return self.reveal_current();
            }
            Message::PrevTab => {
                self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
                return self.reveal_current();
            }

            // ---- settings ----
            Message::OpenSettings => self.page = Page::Settings,
            Message::CloseSettings => self.page = Page::Files,
            Message::SetTheme(choice) => {
                self.settings.theme = choice;
                return self.save_settings();
            }
            Message::SetOpacity(value) => {
                // Only the look changes here; SaveSettings comes on release.
                self.settings.opacity = rustle_core::clamp_opacity(value);
            }
            Message::SetBlur(on) => {
                self.settings.blur = on;
                return self.save_settings();
            }
            Message::SetShowHidden(on) => {
                self.settings.show_hidden = on;
                return Task::batch([self.save_settings(), self.request_thumbs()]);
            }
            Message::ToggleHidden => {
                self.settings.show_hidden = !self.settings.show_hidden;
                return Task::batch([self.save_settings(), self.request_thumbs()]);
            }
            Message::SaveSettings => return self.save_settings(),
            Message::SettingsSaved(result) => {
                if let Err(e) = result {
                    self.error = Some(format!("Couldn't save settings: {e}"));
                }
            }
        }
        Task::none()
    }

    pub fn view(&self) -> Element<'_, Message> {
        let c = self.colors();
        let tab = self.tab();

        let main: Element<'_, Message> = match self.page {
            Page::Files if tab.home_page => home_page::view(&self.places, &self.drives, c),
            Page::Files => file_list::view(
                &tab.entries,
                self.settings.show_hidden,
                self.settings.view,
                &tab.item_counts,
                &self.thumbs,
                c,
            ),
            Page::Settings => settings_page::view(&self.settings, self.started_transparent, c),
        };

        // GNOME Files layout: the sidebar runs the full height on the left;
        // everything else (header bar, tabs, files, status) is on the right.
        // The tab bar is always visible, on top, like in a web browser.
        let right = column![
            self.tab_bar(c),
            self.header_bar(c),
            container(main).width(Fill).height(Fill).padding([4, 8]),
            self.status_bar(c),
        ]
        .spacing(8);

        let base = row![
            container(self.sidebar(c))
                .width(SIDEBAR_WIDTH)
                .height(Fill)
                .padding(10)
                .style(panel(c.surface)),
            right.width(Fill).height(Fill),
        ]
        .spacing(8)
        .padding(8);

        if !self.view_menu_open {
            return base.into();
        }

        // The View menu floats on top (a `stack` draws layers over each other):
        // 1. the normal window,
        // 2. an invisible full-window area: clicking anywhere outside the menu closes it,
        // 3. the menu itself, near the View button (top right).
        let backdrop = mouse_area(space().width(Fill).height(Fill)).on_press(Message::CloseViewMenu);
        let menu = container(opaque(view_menu::view(self.settings.view, c)))
            .width(Fill)
            .align_right(Fill)
            .padding(iced::Padding { top: 96.0, right: 56.0, bottom: 0.0, left: 0.0 });
        stack![base, backdrop, menu].into()
    }

    /// Browser-style tabs: each up to `TAB_MAX_WIDTH` wide (narrower when
    /// there are many), with the + button right after the last tab.
    fn tab_bar(&self, c: Colors) -> Element<'_, Message> {
        let can_close = self.tabs.len() > 1;
        let tabs = self.tabs.iter().enumerate().fold(row![].spacing(4), |bar, (i, t)| {
            let mut label = row![text(shorten(&t.title(), TAB_TITLE_MAX)).size(14).width(Fill)]
                .spacing(8)
                .align_y(iced::Center);
            // The last tab can't be closed, so it gets no × button.
            if can_close {
                label = label.push(
                    button(icons::ui(Ui::Close, c.muted, 14.0))
                        .padding(2)
                        .style(row_button(c))
                        .on_press(Message::CloseTab(t.id)),
                );
            }
            let tab = button(label)
                .width(Fill)
                .padding([5, 10])
                .style(tab_button(c, i == self.active))
                .on_press(Message::SelectTab(i));
            // Middle-click closes a tab, like in browsers.
            let tab = mouse_area(tab).on_middle_press(Message::CloseTab(t.id));
            bar.push(container(tab).width(Fill).max_width(TAB_MAX_WIDTH))
        });

        row![
            tabs,
            button(icons::ui(Ui::Plus, c.text, 16.0))
                .padding([5, 8])
                .style(row_button(c))
                .on_press(Message::NewTab(None)),
        ]
        .spacing(4)
        .align_y(iced::Center)
        .into()
    }

    /// Back / forward, the path bar, new tab, settings.
    fn header_bar(&self, c: Colors) -> Element<'_, Message> {
        let tab = self.tab();
        // A disabled button (msg = None) gets a grey icon.
        let icon_button = |icon: Ui, msg: Option<Message>| {
            let color = if msg.is_some() { c.text } else { c.muted };
            button(icons::ui(icon, color, 18.0))
                .padding([5, 8])
                .style(row_button(c))
                .on_press_maybe(msg)
        };
        let bar = row![
            icon_button(Ui::Back, tab.history.can_back().then_some(Message::Back)),
            icon_button(Ui::Forward, tab.history.can_forward().then_some(Message::Forward)),
            container(pathbar::view(tab, &self.home, self.editing_location, c)).width(Fill),
            icon_button(Ui::View, Some(Message::ToggleViewMenu)),
            icon_button(Ui::Settings, Some(Message::OpenSettings)),
        ]
        .spacing(4)
        .align_y(iced::Center);

        container(bar).padding(6).width(Fill).style(panel(c.surface)).into()
    }

    fn sidebar(&self, c: Colors) -> Element<'_, Message> {
        let tab = self.tab();
        sidebar::view(
            &self.sidebar,
            sidebar::ViewState {
                cwd: Some(&tab.cwd),
                on_home_page: tab.home_page,
                show_hidden: self.settings.show_hidden,
                c,
            },
        )
    }

    fn status_bar(&self, c: Colors) -> Element<'_, Message> {
        let tab = self.tab();
        let hidden = tab.entries.iter().filter(|e| e.hidden).count();
        let left = match &self.error {
            Some(e) => text(e.clone()).color(c.danger),
            None if tab.home_page => {
                text(format!("{} folders · {} drives", self.places.len(), self.drives.len()))
                    .color(c.muted)
            }
            None if tab.loading => text("Loading…").color(c.muted),
            None => text(format!("{} items · {hidden} hidden", tab.entries.len())).color(c.muted),
        };
        row![left.size(13)].padding([0, 6]).into()
    }
}

/// `~` and `~/x` mean the home folder, like in a terminal.
fn expand_home(input: &str, home: &Path) -> PathBuf {
    match input.strip_prefix('~') {
        Some("") => home.to_path_buf(),
        Some(rest) if rest.starts_with('/') => home.join(&rest[1..]),
        _ => PathBuf::from(input),
    }
}

/// Read the user's folders and the drive list in the background.
fn load_places() -> Task<Message> {
    Task::perform(
        async {
            tokio::task::spawn_blocking(|| (places::user_folders(), places::drives()))
                .await
                .unwrap_or_default()
        },
        |(places, drives)| Message::PlacesLoaded { places, drives },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tilde_means_home() {
        let home = Path::new("/home/me");
        assert_eq!(expand_home("~", home), PathBuf::from("/home/me"));
        assert_eq!(expand_home("~/Music", home), PathBuf::from("/home/me/Music"));
        assert_eq!(expand_home("/etc", home), PathBuf::from("/etc"));
        // "~bob" is another user's home in a shell; we don't guess, keep it as typed.
        assert_eq!(expand_home("~bob", home), PathBuf::from("~bob"));
    }
}

