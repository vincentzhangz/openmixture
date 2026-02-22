pub mod components;

mod app;
mod constants;
mod input_picker_state;
mod message;
mod model;
mod platform_file_dialog;
mod style;
mod video;

pub use app::Studio;

pub fn run() -> iced::Result {
    iced::daemon(Studio::boot, Studio::update, Studio::view)
        .title(Studio::title)
        .theme(Studio::theme)
        .subscription(Studio::subscription)
        .run()
}
