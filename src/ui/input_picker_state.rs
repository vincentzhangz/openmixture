use std::path::{Path, PathBuf};

use iced::{Size, Task, window};

use super::{
    constants::{INPUT_PICKER_HEIGHT, INPUT_PICKER_WIDTH},
    message::Message,
    model::{ClipSource, FileMediaKind, FileSource, InputTemplate},
    platform_file_dialog,
};

pub struct InputPickerState {
    window_id: Option<window::Id>,
    selection: InputTemplate,
    media_path: Option<PathBuf>,
}

impl Default for InputPickerState {
    fn default() -> Self {
        Self {
            window_id: None,
            selection: InputTemplate::Video,
            media_path: None,
        }
    }
}

impl InputPickerState {
    pub fn is_window(&self, window_id: window::Id) -> bool {
        self.window_id == Some(window_id)
    }

    pub fn selection(&self) -> InputTemplate {
        self.selection
    }

    pub fn media_path(&self) -> Option<&Path> {
        self.media_path.as_deref()
    }

    pub fn can_confirm(&self) -> bool {
        if self.selection.is_media() {
            self.media_path.is_some()
        } else {
            true
        }
    }

    pub fn select_template(&mut self, template: InputTemplate) {
        self.selection = template;

        if !template.is_media() {
            self.media_path = None;
        }
    }

    pub fn mark_opened(&mut self, window_id: window::Id) {
        self.window_id = Some(window_id);
    }

    pub fn clear_window_if_closed(&mut self, window_id: window::Id) {
        if self.window_id == Some(window_id) {
            self.window_id = None;
        }
    }

    pub fn open(&mut self) -> Task<Message> {
        if self.window_id.is_some() {
            return Task::none();
        }

        self.selection = InputTemplate::Video;
        self.media_path = None;

        let settings = window::Settings {
            size: Size::new(INPUT_PICKER_WIDTH, INPUT_PICKER_HEIGHT),
            min_size: Some(Size::new(INPUT_PICKER_WIDTH, INPUT_PICKER_HEIGHT)),
            max_size: Some(Size::new(INPUT_PICKER_WIDTH, INPUT_PICKER_HEIGHT)),
            position: window::Position::Centered,
            resizable: false,
            minimizable: false,
            ..window::Settings::default()
        };

        let (window_id, task) = window::open(settings);
        self.window_id = Some(window_id);

        task.map(Message::AddInputPickerOpened)
    }

    pub fn close(&mut self) -> Task<Message> {
        let Some(window_id) = self.window_id.take() else {
            return Task::none();
        };

        window::close(window_id)
    }

    pub fn browse_media(&self) -> Task<Message> {
        if !self.selection.is_media() {
            return Task::none();
        }

        let selected = self.selection;

        Task::perform(
            async move { platform_file_dialog::pick_media_with_system_dialog(selected) },
            Message::InputMediaPicked,
        )
    }

    pub fn set_media_from_drop(&mut self, path: PathBuf) {
        let Some(source) = FileSource::from_path(path.clone()) else {
            return;
        };

        self.selection = if source.kind == FileMediaKind::Video {
            InputTemplate::Video
        } else {
            InputTemplate::Photo
        };
        self.media_path = Some(path);
    }

    pub fn on_media_picked(&mut self, path: Option<PathBuf>) {
        self.media_path = path.and_then(|picked| {
            let source = FileSource::from_path(picked.clone())?;

            if self.selection.accepts_kind(source.kind) {
                Some(picked)
            } else {
                None
            }
        });
    }

    pub fn build_source<F>(&self, mut path_exists: F) -> Option<ClipSource>
    where
        F: FnMut(&Path) -> bool,
    {
        match self.selection {
            InputTemplate::Black => Some(ClipSource::black()),
            InputTemplate::ScreenBar => Some(ClipSource::ScreenBar),
            InputTemplate::Video | InputTemplate::Photo => {
                let path = self.media_path.clone()?;

                if path_exists(&path) {
                    return None;
                }

                let source = FileSource::from_path(path)?;

                if !self.selection.accepts_kind(source.kind) {
                    return None;
                }

                Some(ClipSource::File(source))
            }
        }
    }
}
