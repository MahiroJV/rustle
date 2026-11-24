//! The app's state, how it changes (`update`) and how it looks (`view`).

use std::path::{Path, PathBuf};
use std::sync::Arc;

use iced::widget::{button, column, container, mouse_area, row, text};
use iced::{Element, Fill, Subscription, Task, Theme};
use rustle_core::{FileSystem, Settings, sort_entries};
use rustle_fs::LocalFs;

use crate::icons::{self, Ui};
use crate::keybinds;
use crate::message::{Message, Nav};
use crate::pathbar::{self, LOCATION_ID};
use crate::tab::{Tab, shorten};
use crate::theme::{self, Colors, SIDEBAR_WIDTH, panel, row_button, tab_button};
use crate::{file_list, settings_page};

const TAB_TITLE_MAX: usize = 24;
const TAB_MAX_WIDTH: f32 = 220.0;

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
            tabs: vec![Tab::new(0, home.clone())],
            active: 0,
            next_tab_id: 1,
            home: home.clone(),
            editing_location: false,
            error: None,
        };
        let task = app.load(0, home, Nav::Reload);
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
                    sort_entries(&mut entries);
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
                if let Some(parent) = self.tab().cwd.parent().map(PathBuf::from) {
                    return self.go(parent, Nav::Fresh);
                }
            }
            Message::Back => {
                if let Some(target) = self.tab().history.peek_back().map(PathBuf::from) {
                    return self.go(target, Nav::Back);
                }
            }
            Message::Forward => {
                if let Some(target) = self.tab().history.peek_forward().map(PathBuf::from) {
                    return self.go(target, Nav::Forward);
                }
            }
            Message::Refresh => {
                let cwd = self.tab().cwd.clone();
                return self.go(cwd, Nav::Reload);
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
                        let old = std::mem::replace(&mut tab.cwd, dir);
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
                        tab.entries = entries;
                        tab.location_input = tab.cwd.display().to_string();
                        self.error = None;
                    }
                    Err(e) => self.error = Some(e),
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
            Message::Escape => self.editing_location = false,

            // ---- tabs ----
            Message::NewTab(path) => {
                let dir = path.unwrap_or_else(|| self.home.clone());
                let id = self.next_tab_id;
                self.next_tab_id += 1;
                self.tabs.push(Tab::new(id, dir.clone()));
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
                }
            }
            Message::NextTab => self.active = (self.active + 1) % self.tabs.len(),
            Message::PrevTab => {
                self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
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
                return self.save_settings();
            }
            Message::ToggleHidden => {
                self.settings.show_hidden = !self.settings.show_hidden;
                return self.save_settings();
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
            Page::Files => file_list::view(&tab.entries, self.settings.show_hidden, c),
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

        row![
            container(self.sidebar(c))
                .width(SIDEBAR_WIDTH)
                .height(Fill)
                .padding(10)
                .style(panel(c.surface)),
            right.width(Fill).height(Fill),
        ]
        .spacing(8)
        .padding(8)
        .into()
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
            icon_button(Ui::Settings, Some(Message::OpenSettings)),
        ]
        .spacing(4)
        .align_y(iced::Center);

        container(bar).padding(6).width(Fill).style(panel(c.surface)).into()
    }

    /// Placeholder until the real sidebar tree (Task 8).
    fn sidebar(&self, c: Colors) -> Element<'_, Message> {
        column![
            text("Places").color(c.muted).size(13),
            text("The folder tree comes here.").color(c.muted).size(13),
        ]
        .spacing(6)
        .into()
    }

    fn status_bar(&self, c: Colors) -> Element<'_, Message> {
        let tab = self.tab();
        let hidden = tab.entries.iter().filter(|e| e.hidden).count();
        let left = match &self.error {
            Some(e) => text(e.clone()).color(c.danger),
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
