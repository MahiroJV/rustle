//! rustle — iced UI. Lists a folder; click a folder to open it, a file to open
//! it in its default app, ".." to go up.

use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::{button, column, row, scrollable, text};
use iced::{Element, Fill, Task};
use rustle_core::{Entry, FileSystem, format_size, sort_entries};
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
    /// Bumped on every navigation; results from older requests are dropped,
    /// so a slow folder can't overwrite one the user clicked afterwards.
    request: u64,
    loading: bool,
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(PathBuf),
    OpenFile(PathBuf),
    Up,
    ToggleHidden,
    Loaded {
        request: u64,
        dir: PathBuf,
        result: Result<Vec<Entry>, String>,
    },
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
            request: 0,
            loading: false,
        };
        let task = app.load(home);
        (app, task)
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

    fn update(&mut self, message: Message) -> Task<Message> {
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
            Message::ToggleHidden => self.show_hidden = !self.show_hidden,
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
        }
        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let mut header = row![
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
        if self.loading {
            header = header.push(text("Loading…"));
        }

        let list = self
            .entries
            .iter()
            .filter(|e| self.show_hidden || !e.hidden)
            .fold(column![].spacing(2), |col, e| {
                let (label, size, msg) = if e.is_dir {
                    (
                        format!("📁 {}", e.name),
                        String::new(),
                        Message::Navigate(e.path.clone()),
                    )
                } else {
                    (
                        format!("📄 {}", e.name),
                        format_size(e.size),
                        Message::OpenFile(e.path.clone()),
                    )
                };
                let line = row![text(label).width(Fill), text(size)].spacing(10);
                col.push(
                    button(line)
                        .width(Fill)
                        .style(button::text)
                        .on_press(msg),
                )
            });

        let mut content = column![header].spacing(10).padding(10);
        if let Some(err) = &self.error {
            content = content.push(text(err));
        }
        content.push(scrollable(list).height(Fill)).into()
    }
}
