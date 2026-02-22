use std::{path::PathBuf, time::Instant};

use iced::window;

use super::model::{InputTemplate, Transition};

#[derive(Debug, Clone)]
pub enum Message {
    SelectPreviewClip(usize),
    OpenAddInputPicker,
    CloseAddInputPicker,
    SelectInputTemplate(InputTemplate),
    BrowseInputMedia,
    InputMediaPicked(Option<PathBuf>),
    ConfirmAddInput,
    AddInputPickerOpened(window::Id),
    MainWindowOpened(window::Id),
    WindowClosed(window::Id),
    Take,
    QuickCut,
    RestartVideoPlayback(PathBuf),
    ToggleVideoPlayback(PathBuf),
    ScrubVideo(PathBuf, f32),
    SetTransition(Transition),
    SetTransitionDuration(f32),
    TransitionTick(Instant),
    FileHovered(window::Id),
    FileDropped(window::Id, PathBuf),
    FilesHoveredLeft(window::Id),
}
