use std::{
    fmt,
    path::{Path, PathBuf},
    time::Instant,
};

use iced::{Color, widget::image};

use super::style::rgb;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Cut,
    Fade,
    Wipe,
    Zoom,
}

impl Transition {
    pub const ALL: [Transition; 4] = [
        Transition::Cut,
        Transition::Fade,
        Transition::Wipe,
        Transition::Zoom,
    ];
}

impl fmt::Display for Transition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Transition::Cut => "Cut",
            Transition::Fade => "Fade",
            Transition::Wipe => "Wipe",
            Transition::Zoom => "Zoom",
        };

        write!(f, "{name}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewTarget {
    Scene(usize),
    Clip(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputTemplate {
    Video,
    Photo,
    Black,
    ScreenBar,
}

impl InputTemplate {
    pub const ALL: [InputTemplate; 4] = [
        InputTemplate::Video,
        InputTemplate::Photo,
        InputTemplate::Black,
        InputTemplate::ScreenBar,
    ];

    pub fn icon(self) -> &'static str {
        match self {
            InputTemplate::Video => "film",
            InputTemplate::Photo => "image",
            InputTemplate::Black => "square",
            InputTemplate::ScreenBar => "bars",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            InputTemplate::Video => "Video",
            InputTemplate::Photo => "Photo",
            InputTemplate::Black => "Black Content",
            InputTemplate::ScreenBar => "Screen Bar",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            InputTemplate::Video => "Load a video file",
            InputTemplate::Photo => "Load an image file",
            InputTemplate::Black => "Solid black frame",
            InputTemplate::ScreenBar => "Color bars test pattern",
        }
    }

    pub fn prompt(self) -> &'static str {
        match self {
            InputTemplate::Video => "Select a video file to add as input.",
            InputTemplate::Photo => "Select an image file to add as input.",
            InputTemplate::Black => "Add a black input source.",
            InputTemplate::ScreenBar => "Add a color bars test source.",
        }
    }

    pub fn is_media(self) -> bool {
        matches!(self, InputTemplate::Video | InputTemplate::Photo)
    }

    pub fn accepts_kind(self, kind: FileMediaKind) -> bool {
        match self {
            InputTemplate::Video => kind == FileMediaKind::Video,
            InputTemplate::Photo => kind == FileMediaKind::Image,
            InputTemplate::Black | InputTemplate::ScreenBar => true,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Scene {
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileMediaKind {
    Image,
    Video,
}

impl FileMediaKind {
    pub fn icon(self) -> &'static str {
        match self {
            FileMediaKind::Image => "image",
            FileMediaKind::Video => "film",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            FileMediaKind::Image => "IMAGE",
            FileMediaKind::Video => "VIDEO",
        }
    }

    pub fn accent(self) -> Color {
        match self {
            FileMediaKind::Image => rgb(130, 86, 192),
            FileMediaKind::Video => rgb(45, 141, 203),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileSource {
    pub name: String,
    pub path: PathBuf,
    pub kind: FileMediaKind,
}

impl FileSource {
    pub fn from_path(path: PathBuf) -> Option<Self> {
        let kind = classify_file_kind(&path)?;
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .map(std::string::ToString::to_string)
            .unwrap_or_else(|| path.display().to_string());

        Some(Self { name, path, kind })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClipSource {
    Black,
    ScreenBar,
    File(FileSource),
    #[allow(dead_code)]
    Ndi {
        source_name: String,
    },
}

impl ClipSource {
    pub fn black() -> Self {
        ClipSource::Black
    }

    pub fn from_path(path: PathBuf) -> Option<Self> {
        FileSource::from_path(path).map(ClipSource::File)
    }

    pub fn is_black(&self) -> bool {
        matches!(self, ClipSource::Black)
    }

    pub fn uses_path(&self, path: &Path) -> bool {
        matches!(self, ClipSource::File(file) if file.path == path)
    }

    pub fn icon(&self) -> &'static str {
        match self {
            ClipSource::Black => "square",
            ClipSource::ScreenBar => "bars",
            ClipSource::File(file) => file.kind.icon(),
            ClipSource::Ndi { .. } => "tower-broadcast",
        }
    }

    pub fn kind_label(&self) -> &'static str {
        match self {
            ClipSource::Black => "BLACK",
            ClipSource::ScreenBar => "SCREENBAR",
            ClipSource::File(file) => file.kind.label(),
            ClipSource::Ndi { .. } => "NDI",
        }
    }

    pub fn accent(&self) -> Color {
        match self {
            ClipSource::Black => rgb(62, 74, 89),
            ClipSource::ScreenBar => rgb(204, 122, 43),
            ClipSource::File(file) => file.kind.accent(),
            ClipSource::Ndi { .. } => rgb(30, 146, 160),
        }
    }

    pub fn tile_name(&self, fallback_index: usize) -> String {
        match self {
            ClipSource::Black => format!("Clip {fallback_index}"),
            ClipSource::ScreenBar => format!("ScreenBar {fallback_index}"),
            ClipSource::File(file) => file.name.clone(),
            ClipSource::Ndi { source_name } => source_name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClipSlot {
    pub source: ClipSource,
}

impl ClipSlot {
    pub fn black() -> Self {
        Self {
            source: ClipSource::black(),
        }
    }

    pub fn with_source(source: ClipSource) -> Self {
        Self { source }
    }

    pub fn is_black(&self) -> bool {
        self.source.is_black()
    }
}

#[derive(Debug, Clone)]
pub enum VisualSource {
    Black,
    ScreenBar,
    Image(PathBuf),
    Video(Option<image::Handle>),
    Ndi,
}

#[derive(Debug, Clone)]
pub struct VideoTransport {
    pub path: PathBuf,
    pub is_playing: bool,
    pub progress: f32,
    pub position_secs: f32,
    pub duration_secs: f32,
}

#[derive(Debug, Clone)]
pub struct SourceDescriptor {
    pub badge: String,
    pub accent: Color,
    pub kind_label: &'static str,
    pub visual: VisualSource,
    pub video_transport: Option<VideoTransport>,
}

#[derive(Debug, Clone, Copy)]
pub struct ActiveTransition {
    pub from: ViewTarget,
    pub to: ViewTarget,
    pub mode: Transition,
    pub started_at: Instant,
    pub duration_ms: f32,
}

pub fn default_scenes() -> Vec<Scene> {
    vec![
        Scene {
            name: "Opening Wide".into(),
        },
        Scene {
            name: "Host Close".into(),
        },
        Scene {
            name: "Guest Split".into(),
        },
        Scene {
            name: "Lower Third".into(),
        },
        Scene {
            name: "Demo Feed".into(),
        },
        Scene {
            name: "Fullscreen Slide".into(),
        },
    ]
}

fn classify_file_kind(path: &Path) -> Option<FileMediaKind> {
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .unwrap_or_default();

    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "bmp" | "webp" | "gif" | "svg" => Some(FileMediaKind::Image),
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" => Some(FileMediaKind::Video),
        _ => None,
    }
}
