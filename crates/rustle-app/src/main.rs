//! rustle — iced UI. Lists a folder; click a folder to open it, ".." to go up.

use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Fill, Task};
use rustle_core::{Entry, FileSystem, sort_entries};
use rustle_fs::LocalFs;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .title("Rustle")
        .run()
}

struct App {
    // The UI depends on the trait, not on LocalFs (dependency inversion).
    fs: Arc<dyn FileSystem>,
    cwd: PathBuf,
    entries: Vec<Entry>,
    error: Option<String>,
    show_hidden: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Open(PathBuf),
    Up,
    ToggleHidden,
}

impl App {
    fn new() -> (Self, Task<Message>) {
        let fs: Arc<dyn FileSystem> = Arc::new(LocalFs);
        let home = fs.home();
        let mut app = Self {
            fs,
            cwd: home.clone(),
            entries: Vec::new(),
            error: None,
            show_hidden: false,
        };
        app.load(home);
        (app, Task::none())
    }

    fn load(&mut self, dir: PathBuf) {
        match self.fs.list(&dir) {
            Ok(mut entries) => {
                sort_entries(&mut entries);
                self.entries = entries;
                self.cwd = dir;
                self.error = None;
            }
            Err(e) => self.error = Some(e.to_string()),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Open(path) => self.load(path),
            Message::Up => {
                if let Some(parent) = self.cwd.parent().map(PathBuf::from) {
                    self.load(parent);
                }
            }
            Message::ToggleHidden => self.show_hidden = !self.show_hidden,
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let header = row![
            button("..").on_press(Message::Up),
            button(if self.show_hidden {
                "Hide hidden"
            } else {
                "Show hidden"
            })
            .on_press(Message::ToggleHidden),
            text(self.cwd.display().to_string()),
        ]
        .spacing(10);

        let list = self
            .entries
            .iter()
            .filter(|e| self.show_hidden || !e.hidden)
            .fold(column![].spacing(2), |col, e| {
                let label = if e.is_dir {
                    format!("📁 {}", e.name)
                } else {
                    format!("📄 {}", e.name)
                };
                let item = button(text(label)).width(Fill).style(button::text);
                col.push(if e.is_dir {
                    item.on_press(Message::Open(e.path.clone()))
                } else {
                    item
                })
            });

        let mut content = column![header].spacing(10).padding(10);
        if let Some(err) = &self.error {
            content = content.push(text(err));
        }
        content.push(scrollable(list).height(Fill)).into()
    }
}
