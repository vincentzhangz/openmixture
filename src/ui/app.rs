use std::{
    collections::{HashMap, HashSet},
    panic::{AssertUnwindSafe, catch_unwind},
    path::PathBuf,
    time::Instant,
};

use iced::{
    Element, Event, Length, Size, Subscription, Task, Theme, event, exit,
    widget::{column, container, row},
    window,
};

use super::{
    components,
    constants::{
        INITIAL_CLIP_SLOT_COUNT, MAX_TRANSITION_MS, MIN_TRANSITION_MS, WINDOW_HEIGHT, WINDOW_WIDTH,
    },
    input_picker_state::InputPickerState,
    message::Message,
    model::{
        ActiveTransition, ClipSlot, ClipSource, FileMediaKind, Scene, SourceDescriptor, Transition,
        VideoTransport, ViewTarget, VisualSource, default_scenes,
    },
    style::{compact_name, panel_style, rgb},
    video::VideoPlayer,
};

pub struct Studio {
    main_window_id: window::Id,
    input_picker: InputPickerState,
    scenes: Vec<Scene>,
    clip_slots: Vec<ClipSlot>,
    preview_target: ViewTarget,
    program_target: ViewTarget,
    transition: Transition,
    transition_ms: f32,
    active_transition: Option<ActiveTransition>,
    transition_progress: f32,
    drop_hovered: bool,
    drop_slot: usize,
    video_players: HashMap<PathBuf, VideoPlayer>,
    video_failed: HashSet<PathBuf>,
}

impl Default for Studio {
    fn default() -> Self {
        Self::with_main_window(window::Id::unique())
    }
}

impl Studio {
    pub fn boot() -> (Self, Task<Message>) {
        let settings = window::Settings {
            size: Size::new(WINDOW_WIDTH, WINDOW_HEIGHT),
            position: window::Position::Centered,
            ..window::Settings::default()
        };

        let (main_window_id, task) = window::open(settings);
        (
            Self::with_main_window(main_window_id),
            task.map(Message::MainWindowOpened),
        )
    }

    fn with_main_window(main_window_id: window::Id) -> Self {
        Self {
            main_window_id,
            input_picker: InputPickerState::default(),
            scenes: default_scenes(),
            clip_slots: (0..INITIAL_CLIP_SLOT_COUNT)
                .map(|_| ClipSlot::black())
                .collect(),
            preview_target: ViewTarget::Scene(1),
            program_target: ViewTarget::Scene(0),
            transition: Transition::Fade,
            transition_ms: 350.0,
            active_transition: None,
            transition_progress: 0.0,
            drop_hovered: false,
            drop_slot: 0,
            video_players: HashMap::new(),
            video_failed: HashSet::new(),
        }
    }

    pub fn title(&self, window_id: window::Id) -> String {
        if self.input_picker.is_window(window_id) {
            "Add Input".to_string()
        } else {
            super::constants::APP_TITLE.to_string()
        }
    }

    pub fn theme(&self, _window_id: window::Id) -> Theme {
        Theme::Dark
    }

    pub fn subscription(&self) -> Subscription<Message> {
        Subscription::batch([
            event::listen_with(map_runtime_event),
            window::frames().map(Message::TransitionTick),
        ])
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SelectPreviewClip(index) => {
                if index < self.clip_slots.len() {
                    self.preview_target = ViewTarget::Clip(index);
                    self.drop_slot = index;
                }
            }
            Message::OpenAddInputPicker => {
                return self.input_picker.open();
            }
            Message::CloseAddInputPicker => {
                return self.input_picker.close();
            }
            Message::SelectInputTemplate(template) => {
                self.input_picker.select_template(template);
            }
            Message::BrowseInputMedia => {
                return self.input_picker.browse_media();
            }
            Message::InputMediaPicked(path) => {
                self.input_picker.on_media_picked(path);
            }
            Message::ConfirmAddInput => {
                if self.confirm_add_input() {
                    return self.input_picker.close();
                }
            }
            Message::AddInputPickerOpened(window_id) => {
                self.input_picker.mark_opened(window_id);
            }
            Message::MainWindowOpened(_window_id) => {}
            Message::WindowClosed(window_id) => {
                self.input_picker.clear_window_if_closed(window_id);

                if window_id == self.main_window_id {
                    return exit();
                }
            }
            Message::Take => {
                self.queue_take();
            }
            Message::QuickCut => {
                self.transition = Transition::Cut;
                self.transition_ms = MIN_TRANSITION_MS;
                self.active_transition = None;
                self.transition_progress = 0.0;

                if self.program_target != self.preview_target {
                    let from = self.program_target;
                    let to = self.preview_target;
                    self.complete_take(from, to);
                }
            }
            Message::RestartVideoPlayback(path) => {
                self.ensure_video_player(&path);

                if let Some(player) = self.video_players.get_mut(&path) {
                    if let Err(error) = player.restart_from_beginning() {
                        eprintln!("video restart failed for {}: {error}", path.display());
                    }
                }
            }
            Message::ToggleVideoPlayback(path) => {
                self.ensure_video_player(&path);

                if let Some(player) = self.video_players.get_mut(&path) {
                    player.toggle_playback();
                }
            }
            Message::ScrubVideo(path, progress) => {
                self.ensure_video_player(&path);

                if let Some(player) = self.video_players.get_mut(&path) {
                    if let Err(error) = player.seek_to_progress(progress) {
                        eprintln!("video seek failed for {}: {error}", path.display());
                    }
                }
            }
            Message::SetTransition(transition) => {
                self.transition = transition;
            }
            Message::SetTransitionDuration(duration) => {
                self.transition_ms = duration.clamp(MIN_TRANSITION_MS, MAX_TRANSITION_MS);
            }
            Message::TransitionTick(now) => {
                self.update_transition_tick(now);
                self.update_video_tick(now);
            }
            Message::FileHovered(window_id) => {
                if window_id == self.main_window_id {
                    self.drop_hovered = true;
                }
            }
            Message::FileDropped(window_id, path) => {
                if window_id == self.main_window_id {
                    self.drop_hovered = false;
                    self.handle_file_drop(path);
                } else if self.input_picker.is_window(window_id) {
                    self.input_picker.set_media_from_drop(path);
                }
            }
            Message::FilesHoveredLeft(window_id) => {
                if window_id == self.main_window_id {
                    self.drop_hovered = false;
                }
            }
        }

        Task::none()
    }

    pub fn view(&self, window_id: window::Id) -> Element<'_, Message> {
        if self.input_picker.is_window(window_id) {
            return components::input_picker::view(components::input_picker::InputPickerProps {
                selected: self.input_picker.selection(),
                selected_media_path: self.input_picker.media_path(),
                can_confirm: self.input_picker.can_confirm(),
            });
        }

        self.main_view()
    }

    fn main_view(&self) -> Element<'_, Message> {
        let root = column![
            components::chrome_bar::view(self.loaded_clip_count(), self.clip_slots.len()),
            self.broadcast_surface(),
        ]
        .spacing(8)
        .padding(8)
        .height(Length::Fill);

        container(root)
            .width(Length::Fill)
            .height(Length::Fill)
            .style(|_| panel_style(rgb(5, 8, 13), rgb(6, 10, 15), rgb(225, 230, 236)))
            .into()
    }

    fn queue_take(&mut self) {
        if self.active_transition.is_some() || self.preview_target == self.program_target {
            return;
        }

        if self.transition == Transition::Cut {
            let from = self.program_target;
            let to = self.preview_target;
            self.complete_take(from, to);
            return;
        }

        self.active_transition = Some(ActiveTransition {
            from: self.program_target,
            to: self.preview_target,
            mode: self.transition,
            started_at: Instant::now(),
            duration_ms: self.transition_ms.max(MIN_TRANSITION_MS),
        });
        self.transition_progress = 0.0;
    }

    fn update_transition_tick(&mut self, now: Instant) {
        let Some(active) = self.active_transition else {
            self.transition_progress = 0.0;
            return;
        };

        let elapsed_ms = now
            .saturating_duration_since(active.started_at)
            .as_secs_f32()
            * 1000.0;
        let progress = (elapsed_ms / active.duration_ms).clamp(0.0, 1.0);
        self.transition_progress = progress;

        if progress >= 1.0 {
            self.complete_take(active.from, active.to);
            self.active_transition = None;
            self.transition_progress = 0.0;
        }
    }

    fn complete_take(&mut self, from: ViewTarget, to: ViewTarget) {
        self.program_target = to;
        self.preview_target = from;
        self.sync_drop_slot_with_preview();
    }

    fn sync_drop_slot_with_preview(&mut self) {
        if let ViewTarget::Clip(index) = self.preview_target {
            if index < self.clip_slots.len() {
                self.drop_slot = index;
            }
        }
    }

    fn handle_file_drop(&mut self, path: PathBuf) {
        if self
            .clip_slots
            .iter()
            .any(|slot| slot.source.uses_path(&path))
        {
            return;
        }

        let Some(source) = ClipSource::from_path(path) else {
            return;
        };

        self.insert_source(source);
    }

    fn insert_source(&mut self, source: ClipSource) {
        self.prepare_source(&source);

        if let Some(index) = self.clip_slots.iter().position(ClipSlot::is_black) {
            self.clip_slots[index] = ClipSlot::with_source(source);
            self.set_preview_slot(index);
            return;
        }

        self.clip_slots.push(ClipSlot::with_source(source));
        let new_index = self.clip_slots.len().saturating_sub(1);
        self.set_preview_slot(new_index);
    }

    fn append_input_source(&mut self, source: ClipSource) {
        self.prepare_source(&source);

        self.clip_slots.push(ClipSlot::with_source(source));
        let index = self.clip_slots.len().saturating_sub(1);
        self.set_preview_slot(index);
    }

    fn prepare_source(&mut self, source: &ClipSource) {
        if let ClipSource::File(file) = source {
            if file.kind == FileMediaKind::Video {
                self.ensure_video_player(&file.path);
            }
        }
    }

    fn set_preview_slot(&mut self, index: usize) {
        self.drop_slot = index;
        self.preview_target = ViewTarget::Clip(index);
    }

    fn confirm_add_input(&mut self) -> bool {
        let Some(source) = self.input_picker.build_source(|path| {
            self.clip_slots
                .iter()
                .any(|slot| slot.source.uses_path(path))
        }) else {
            return false;
        };

        self.append_input_source(source);
        true
    }

    fn loaded_clip_count(&self) -> usize {
        self.clip_slots
            .iter()
            .filter(|slot| !slot.is_black())
            .count()
    }

    fn describe_target(&self, target: ViewTarget) -> SourceDescriptor {
        match target {
            ViewTarget::Scene(index) => {
                let badge = self
                    .scenes
                    .get(index)
                    .map(|scene| scene.name.clone())
                    .unwrap_or_else(|| format!("Scene {}", index + 1));

                SourceDescriptor {
                    badge,
                    accent: rgb(236, 137, 35),
                    kind_label: "SCENE",
                    visual: VisualSource::Black,
                    video_transport: None,
                }
            }
            ViewTarget::Clip(index) => {
                if let Some(slot) = self.clip_slots.get(index) {
                    SourceDescriptor {
                        badge: format!("Clip {}", index + 1),
                        accent: slot.source.accent(),
                        kind_label: slot.source.kind_label(),
                        visual: self.describe_visual(&slot.source),
                        video_transport: self.describe_video_transport(&slot.source),
                    }
                } else {
                    SourceDescriptor {
                        badge: "Clip".into(),
                        accent: rgb(118, 126, 139),
                        kind_label: "FILE",
                        visual: VisualSource::Black,
                        video_transport: None,
                    }
                }
            }
        }
    }

    fn describe_visual(&self, source: &ClipSource) -> VisualSource {
        match source {
            ClipSource::Black => VisualSource::Black,
            ClipSource::ScreenBar => VisualSource::ScreenBar,
            ClipSource::File(file) => match file.kind {
                FileMediaKind::Image => VisualSource::Image(file.path.clone()),
                FileMediaKind::Video => VisualSource::Video(
                    self.video_players
                        .get(&file.path)
                        .and_then(VideoPlayer::frame_handle),
                ),
            },
            ClipSource::Ndi { .. } => VisualSource::Ndi,
        }
    }

    fn describe_video_transport(&self, source: &ClipSource) -> Option<VideoTransport> {
        let ClipSource::File(file) = source else {
            return None;
        };

        if file.kind != FileMediaKind::Video {
            return None;
        }

        self.video_players
            .get(&file.path)
            .map(|player| VideoTransport {
                path: file.path.clone(),
                is_playing: player.is_playing(),
                progress: player.progress(),
                position_secs: player.position_secs(),
                duration_secs: player.duration_secs(),
            })
    }

    fn update_video_tick(&mut self, now: Instant) {
        let mut targets = Vec::new();

        if let Some(path) = self.video_path_for_target(self.preview_target) {
            targets.push(path);
        }
        if let Some(path) = self.video_path_for_target(self.program_target) {
            targets.push(path);
        }
        if let Some(active) = self.active_transition {
            if let Some(path) = self.video_path_for_target(active.from) {
                targets.push(path);
            }
            if let Some(path) = self.video_path_for_target(active.to) {
                targets.push(path);
            }
        }

        targets.sort();
        targets.dedup();

        for path in &targets {
            self.ensure_video_player(path);
        }

        let mut panicked_players = Vec::new();

        for path in targets {
            let panicked = if let Some(player) = self.video_players.get_mut(&path) {
                catch_unwind(AssertUnwindSafe(|| player.tick(now))).is_err()
            } else {
                false
            };

            if panicked {
                panicked_players.push(path);
            }
        }

        for path in panicked_players {
            eprintln!(
                "video playback panicked for {} and has been disabled",
                path.display()
            );
            self.video_players.remove(&path);
            self.video_failed.insert(path);
        }
    }

    fn video_path_for_target(&self, target: ViewTarget) -> Option<PathBuf> {
        match target {
            ViewTarget::Scene(_) => None,
            ViewTarget::Clip(index) => {
                let slot = self.clip_slots.get(index)?;
                match &slot.source {
                    ClipSource::File(file) if file.kind == FileMediaKind::Video => {
                        Some(file.path.clone())
                    }
                    _ => None,
                }
            }
        }
    }

    fn ensure_video_player(&mut self, path: &std::path::Path) {
        if self.video_players.contains_key(path) || self.video_failed.contains(path) {
            return;
        }

        match VideoPlayer::open(path) {
            Ok(player) => {
                self.video_players.insert(path.to_path_buf(), player);
            }
            Err(error) => {
                eprintln!("video playback unavailable for {}: {error}", path.display());
                self.video_failed.insert(path.to_path_buf());
            }
        }
    }

    fn broadcast_surface(&self) -> Element<'_, Message> {
        let top_row = row![
            self.preview_bus(),
            components::transition_controls::view(self.transition, self.transition_ms),
            self.program_bus(),
        ]
        .spacing(8)
        .height(Length::FillPortion(3));

        let lower_row = row![components::input_bank::view(
            components::input_bank::InputBankProps {
                clip_slots: &self.clip_slots,
                preview_target: self.preview_target,
                program_target: self.program_target,
                armed_slot: self.drop_slot,
                drop_hovered: self.drop_hovered,
            }
        )]
        .spacing(8)
        .height(Length::FillPortion(2));

        container(column![top_row, lower_row].spacing(8))
            .padding(7)
            .height(Length::Fill)
            .style(|_| panel_style(rgb(8, 12, 18), rgb(32, 43, 56), rgb(221, 228, 236)))
            .into()
    }

    fn preview_bus(&self) -> Element<'_, Message> {
        let source = self.describe_target(self.preview_target);
        let title =
            components::bus_header::view("Preview", rgb(229, 132, 26), source.badge.clone());

        container(
            column![
                title,
                components::source_canvas::static_view(source, false, self.transition_ms),
            ]
            .spacing(6),
        )
        .padding(6)
        .width(Length::FillPortion(7))
        .height(Length::Fill)
        .style(|_| panel_style(rgb(13, 18, 27), rgb(59, 72, 89), rgb(218, 226, 236)))
        .into()
    }

    fn program_bus(&self) -> Element<'_, Message> {
        if let Some(active) = self.active_transition {
            let from = self.describe_target(active.from);
            let to = self.describe_target(active.to);
            let title = components::bus_header::view(
                "Program",
                rgb(174, 36, 43),
                format!(
                    "{} -> {}",
                    compact_name(&from.badge, 12),
                    compact_name(&to.badge, 12)
                ),
            );

            container(
                column![
                    title,
                    components::source_canvas::transition_view(
                        from,
                        to,
                        active.mode,
                        self.transition_progress,
                        true,
                        self.transition_ms,
                    ),
                ]
                .spacing(6),
            )
            .padding(6)
            .width(Length::FillPortion(7))
            .height(Length::Fill)
            .style(|_| panel_style(rgb(15, 18, 26), rgb(69, 74, 88), rgb(219, 226, 236)))
            .into()
        } else {
            let source = self.describe_target(self.program_target);
            let title =
                components::bus_header::view("Program", rgb(174, 36, 43), source.badge.clone());

            container(
                column![
                    title,
                    components::source_canvas::static_view(source, true, self.transition_ms),
                ]
                .spacing(6),
            )
            .padding(6)
            .width(Length::FillPortion(7))
            .height(Length::Fill)
            .style(|_| panel_style(rgb(15, 18, 26), rgb(69, 74, 88), rgb(219, 226, 236)))
            .into()
        }
    }
}

fn map_runtime_event(
    event: Event,
    _status: event::Status,
    window_id: window::Id,
) -> Option<Message> {
    match event {
        Event::Window(window::Event::Closed) => Some(Message::WindowClosed(window_id)),
        Event::Window(window::Event::FileHovered(_)) => Some(Message::FileHovered(window_id)),
        Event::Window(window::Event::FileDropped(path)) => {
            Some(Message::FileDropped(window_id, path))
        }
        Event::Window(window::Event::FilesHoveredLeft) => {
            Some(Message::FilesHoveredLeft(window_id))
        }
        _ => None,
    }
}
