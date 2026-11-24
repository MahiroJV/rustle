//! rustle — a lightweight file manager built with iced.

mod app;
mod dates;
mod file_list;
mod home_page;
mod icons;
mod keybinds;
mod message;
mod pathbar;
mod settings_page;
mod sidebar;
mod tab;
mod textfit;
mod theme;
mod thumbs;
mod view_menu;

use app::App;
use iced::window;
use rustle_core::settings::MAX_OPACITY;

fn main() -> iced::Result {
    // Settings are read before the window exists, because transparency and
    // blur can only be chosen when the window is created.
    let settings = rustle_fs::load_settings(&rustle_fs::settings_path());
    let transparent = settings.opacity < MAX_OPACITY || settings.blur;

    let window = window::Settings {
        transparent,
        blur: settings.blur,
        ..window::Settings::default()
    };

    iced::application(
        move || App::new(settings.clone(), transparent),
        App::update,
        App::view,
    )
    .title(App::title)
    .subscription(App::subscription)
    .theme(App::theme)
    .style(App::style)
    .window(window)
    .run()
}
