//! The app's state, how it changes (`update`) and how it looks (`view`).

use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::{button, column, container, row, text};
use iced::{Element, Fill, Task, Theme};
use rustle_core::{Entry, FileSystem, Settings, sort_entries};
use rustle_fs::LocalFs;

use crate::message::Message;
use crate::theme::{self, Colors, SIDEBAR_WIDTH, panel, row_button};
use crate::{file_list, settings_page};

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
    cwd: PathBuf,
    entries: Vec<Entry>,
    error: Option<String>,
    /// Bumped on every navigation; results from older requests are dropped,
    /// so a slow folder can't overwrite one the user clicked afterwards.
    request: u64,
    loading: bool,
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
            cwd: home.clone(),
            entries: Vec::new(),
            error: None,
            request: 0,
            loading: false,
        };
        let task = app.load(home);
        (app, task)
    }

    /// The current palette, with backgrounds faded by the opacity setting.
    fn colors(&self) -> Colors {
        theme::palette(self.settings.theme).with_opacity(self.settings.opacity)
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

    /// Read `dir` on a blocking thread so a big or slow folder doesn't freeze
    /// the window.
    fn load(&mut self, dir: PathBuf) -> Task<Message> {
        self.request += 1;
        self.loading = true;
        let request = self.request;
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
                request,
                dir,
                result,
            },
        )
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

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Navigate(path) => return self.load(path),
            Message::Up => {
                if let Some(parent) = self.cwd.parent().map(PathBuf::from) {
                    return self.load(parent);
                }
            }
            Message::OpenFile(path) => {
                self.error = self.fs.open(&path).err().map(|e| e.to_string());
            }
            Message::Loaded {
                request,
                dir,
                result,
            } => {
                if request != self.request {
                    return Task::none(); // stale
                }
                self.loading = false;
                match result {
                    Ok(entries) => {
                        self.entries = entries;
                        self.cwd = dir;
                        self.error = None;
                    }
                    Err(e) => self.error = Some(e),
                }
            }

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

        let main: Element<'_, Message> = match self.page {
            Page::Files => file_list::view(&self.entries, self.settings.show_hidden, c),
            Page::Settings => settings_page::view(&self.settings, self.started_transparent, c),
        };

        let body = row![
            container(self.sidebar(c))
                .width(SIDEBAR_WIDTH)
                .height(Fill)
                .padding(10)
                .style(panel(c.surface)),
            container(main).width(Fill).height(Fill).padding([4, 8]),
        ]
        .spacing(8);

        column![self.toolbar(c), body.height(Fill), self.status_bar(c)]
            .spacing(8)
            .padding(8)
            .into()
    }

    fn toolbar(&self, c: Colors) -> Element<'_, Message> {
        let nav = |label: &'static str, msg: Option<Message>| {
            button(text(label))
                .padding([4, 10])
                .style(row_button(c))
                .on_press_maybe(msg)
        };
        let bar = row![
            // Back/forward arrive with tabs (Task 5); for now they're disabled.
            nav("←", None),
            nav("→", None),
            nav("↑", self.cwd.parent().map(|_| Message::Up)),
            text(self.cwd.display().to_string()).width(Fill),
            nav("⚙", Some(Message::OpenSettings)),
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
        let hidden = self.entries.iter().filter(|e| e.hidden).count();
        let left = match &self.error {
            Some(e) => text(e.clone()).color(c.danger),
            None if self.loading => text("Loading…").color(c.muted),
            None => text(format!("{} items · {hidden} hidden", self.entries.len())).color(c.muted),
        };
        row![left.size(13)].padding([0, 6]).into()
    }
}
