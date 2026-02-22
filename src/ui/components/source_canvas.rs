use iced::{
    ContentFit, Element, Length, alignment,
    widget::{button, column, container, image, responsive, row, slider, stack, text},
};
use iced_font_awesome::fa_icon_solid;

use crate::ui::{
    constants::{VIDEO_ASPECT_H, VIDEO_ASPECT_W},
    message::Message,
    model::{SourceDescriptor, Transition, VideoTransport, VisualSource},
    style::{action_button_style, darken, panel_style, rgb, with_alpha},
};

pub fn static_view(
    source: SourceDescriptor,
    live: bool,
    transition_ms: f32,
) -> Element<'static, Message> {
    let frame_source = source.clone();
    let transport = source.video_transport.clone();

    let video_frame = responsive(move |size| {
        let frame_body = render_source_layer(frame_source.visual.clone(), 1.0, 1.0);

        frame_container(size.width, size.height, frame_source.accent, frame_body)
    })
    .width(Length::Fill)
    .height(Length::Fill);

    canvas_shell(
        source.kind_label,
        source.accent,
        live,
        transition_ms,
        video_frame.into(),
        transport.map(video_controls),
    )
}

pub fn transition_view(
    from: SourceDescriptor,
    to: SourceDescriptor,
    mode: Transition,
    progress: f32,
    live: bool,
    transition_ms: f32,
) -> Element<'static, Message> {
    let from_visual = from.visual.clone();
    let to_visual = to.visual.clone();
    let accent = to.accent;
    let mix = transition_curve(mode, progress);

    let video_frame = responsive(move |size| {
        let to_scale = if mode == Transition::Zoom {
            1.08 - (mix * 0.08)
        } else {
            1.0
        };

        let layers: Vec<Element<'static, Message>> = vec![
            render_source_layer(from_visual.clone(), 1.0 - mix, 1.0),
            render_source_layer(to_visual.clone(), mix, to_scale),
        ];

        let frame_body = stack(layers)
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        frame_container(size.width, size.height, accent, frame_body)
    })
    .width(Length::Fill)
    .height(Length::Fill);

    canvas_shell(
        to.kind_label,
        to.accent,
        live,
        transition_ms,
        video_frame.into(),
        None,
    )
}

fn canvas_shell(
    kind_label: &'static str,
    accent: iced::Color,
    live: bool,
    transition_ms: f32,
    frame: Element<'static, Message>,
    transport_controls: Option<Element<'static, Message>>,
) -> Element<'static, Message> {
    let bus_flag = if live { "LIVE" } else { "READY" };
    let flag_color = if live {
        rgb(172, 34, 42)
    } else {
        rgb(226, 135, 34)
    };

    let mut body = column![
        row![
            container(text(bus_flag).size(11))
                .padding(4)
                .style(move |_| {
                    panel_style(
                        with_alpha(flag_color, 0.42),
                        darken(flag_color, 0.18),
                        rgb(247, 249, 251),
                    )
                }),
            container(text(kind_label).size(11))
                .padding(4)
                .style(move |_| {
                    panel_style(
                        with_alpha(accent, 0.28),
                        darken(accent, 0.26),
                        rgb(239, 243, 247),
                    )
                }),
            text(format!("{} ms", transition_ms as u16)).size(11),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
        frame,
    ]
    .spacing(10);

    if let Some(transport_controls) = transport_controls {
        body = body.push(transport_controls);
    }

    container(body)
        .padding(8)
        .height(Length::Fill)
        .style(|_| panel_style(rgb(15, 20, 28), rgb(57, 73, 89), rgb(233, 239, 244)))
        .into()
}

fn frame_container(
    max_width: f32,
    max_height: f32,
    accent: iced::Color,
    body: Element<'static, Message>,
) -> Element<'static, Message> {
    let width = max_width.max(1.0);
    let height = max_height.max(1.0);
    let frame_width = width.min(height * VIDEO_ASPECT_W / VIDEO_ASPECT_H);
    let frame_height = frame_width * VIDEO_ASPECT_H / VIDEO_ASPECT_W;

    container(container(body).width(Length::Fill).height(Length::Fill))
        .width(Length::Fixed(frame_width))
        .height(Length::Fixed(frame_height))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(move |_| panel_style(rgb(5, 8, 12), darken(accent, 0.08), rgb(236, 241, 247)))
        .into()
}

fn render_source_layer(
    visual: VisualSource,
    opacity: f32,
    scale: f32,
) -> Element<'static, Message> {
    let layer_opacity = opacity.clamp(0.0, 1.0);

    match visual {
        VisualSource::Image(path) => image::Image::new(path)
            .width(Length::Fill)
            .height(Length::Fill)
            .content_fit(ContentFit::Contain)
            .opacity(layer_opacity)
            .scale(scale)
            .into(),
        VisualSource::ScreenBar => screen_bar_layer(layer_opacity, scale),
        VisualSource::Video(Some(frame)) => container(
            image::Image::new(frame)
                .width(Length::Fill)
                .height(Length::Fill)
                .content_fit(ContentFit::Contain)
                .opacity(layer_opacity)
                .scale(scale),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_| iced::widget::container::Style::default().background(rgb(0, 0, 0)))
        .into(),
        VisualSource::Black | VisualSource::Video(None) | VisualSource::Ndi => container(column![])
            .width(Length::Fill)
            .height(Length::Fill)
            .style(move |_| {
                iced::widget::container::Style::default()
                    .background(with_alpha(rgb(0, 0, 0), layer_opacity))
            })
            .into(),
    }
}

fn transition_curve(mode: Transition, progress: f32) -> f32 {
    let p = progress.clamp(0.0, 1.0);

    match mode {
        Transition::Cut => 1.0,
        Transition::Fade => p,
        Transition::Wipe => (p * 1.18).clamp(0.0, 1.0),
        Transition::Zoom => p.powf(0.74),
    }
}

fn video_controls(transport: VideoTransport) -> Element<'static, Message> {
    let restart_path = transport.path.clone();
    let play_icon = if transport.is_playing {
        "pause"
    } else {
        "play"
    };
    let play_path = transport.path.clone();
    let scrub_path = transport.path.clone();
    let progress = transport.progress.clamp(0.0, 1.0);

    let toggle = button(fa_icon_solid(play_icon).size(11.0))
        .on_press(Message::ToggleVideoPlayback(play_path))
        .padding([4, 10])
        .style(|_, status| {
            action_button_style(
                status,
                rgb(33, 111, 63),
                rgb(73, 168, 111),
                rgb(240, 247, 243),
            )
        });

    let scrub = slider(0.0..=1.0, progress, move |value| {
        Message::ScrubVideo(scrub_path.clone(), value)
    })
    .step(0.001)
    .width(Length::Fill);

    let restart = button(fa_icon_solid("rotate-left").size(11.0))
        .on_press(Message::RestartVideoPlayback(restart_path))
        .padding([4, 10])
        .style(|_, status| {
            action_button_style(
                status,
                rgb(37, 72, 117),
                rgb(75, 126, 193),
                rgb(236, 242, 248),
            )
        });

    let time_label = if transport.duration_secs > 0.0 {
        format!(
            "{} / {}",
            format_seconds(transport.position_secs),
            format_seconds(transport.duration_secs),
        )
    } else {
        format_seconds(transport.position_secs)
    };

    container(
        row![
            restart,
            toggle,
            scrub,
            row![fa_icon_solid("clock").size(11.0), text(time_label).size(11)]
                .spacing(5)
                .align_y(alignment::Vertical::Center),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .padding(6)
    .style(|_| panel_style(rgb(12, 18, 27), rgb(53, 71, 88), rgb(218, 226, 236)))
    .into()
}

fn format_seconds(seconds: f32) -> String {
    let total = seconds.max(0.0).round() as u64;
    let minutes = total / 60;
    let secs = total % 60;
    format!("{minutes:02}:{secs:02}")
}

fn screen_bar_layer(opacity: f32, scale: f32) -> Element<'static, Message> {
    let _ = scale;

    let bars = row![
        bar(rgb(237, 237, 237), opacity),
        bar(rgb(235, 235, 0), opacity),
        bar(rgb(0, 235, 235), opacity),
        bar(rgb(0, 214, 0), opacity),
        bar(rgb(235, 0, 235), opacity),
        bar(rgb(235, 0, 0), opacity),
        bar(rgb(0, 0, 235), opacity),
    ]
    .spacing(0)
    .width(Length::Fill)
    .height(Length::Fill);

    container(bars)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn bar(color: iced::Color, opacity: f32) -> Element<'static, Message> {
    container(column![])
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .style(move |_| {
            iced::widget::container::Style::default().background(with_alpha(color, opacity))
        })
        .into()
}
