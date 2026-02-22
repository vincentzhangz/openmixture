use iced::{
    ContentFit, Element, Length, alignment,
    widget::{button, column, container, image, row, text},
};
use iced_font_awesome::fa_icon_solid;

use crate::ui::{
    message::Message,
    model::{ClipSlot, ClipSource, FileMediaKind, ViewTarget},
    style::{compact_name, darken, panel_style, rgb, tile_button_style, with_alpha},
};

pub struct ClipTileProps<'a> {
    pub index: usize,
    pub slot: &'a ClipSlot,
    pub preview_target: ViewTarget,
    pub program_target: ViewTarget,
    pub armed_slot: usize,
}

pub fn view(props: ClipTileProps<'_>) -> Element<'_, Message> {
    let source = props.slot.source.clone();
    let is_preview = matches!(props.preview_target, ViewTarget::Clip(i) if i == props.index);
    let is_program = matches!(props.program_target, ViewTarget::Clip(i) if i == props.index);
    let is_armed = props.armed_slot == props.index;
    let active = is_preview || is_program || is_armed;

    let accent = source.accent();
    let icon = source.icon();
    let name = compact_name(&source.tile_name(props.index + 1), 16);
    let kind_label = source.kind_label();

    let tag = if is_program {
        "PROGRAM"
    } else if is_preview {
        "PREVIEW"
    } else if is_armed {
        "ARMED"
    } else {
        kind_label
    };

    let media_preview = media_preview(source.clone(), accent);

    button(
        container(
            column![
                media_preview,
                row![fa_icon_solid(icon).size(13.0), text(name).size(12),]
                    .spacing(6)
                    .align_y(alignment::Vertical::Center),
                text(kind_label).size(10),
                container(text(tag).size(10)).padding(3).style(move |_| {
                    panel_style(with_alpha(accent, 0.28), accent, rgb(242, 246, 249))
                }),
            ]
            .spacing(5),
        )
        .padding(7)
        .style(move |_| {
            panel_style(
                darken(accent, 0.46),
                darken(accent, 0.21),
                rgb(226, 233, 240),
            )
        }),
    )
    .on_press(Message::SelectPreviewClip(props.index))
    .padding(0)
    .width(Length::Fixed(172.0))
    .style(move |_, status| tile_button_style(status, active, accent))
    .into()
}

fn media_preview(source: ClipSource, accent: iced::Color) -> Element<'static, Message> {
    match source {
        ClipSource::File(file) if file.kind == FileMediaKind::Image => container(
            image::Image::new(file.path)
                .width(Length::Fill)
                .height(Length::Fixed(74.0))
                .content_fit(ContentFit::Cover),
        )
        .style(move |_| {
            panel_style(
                darken(accent, 0.40),
                darken(accent, 0.22),
                rgb(231, 236, 241),
            )
        })
        .into(),
        ClipSource::Black => container(column![])
            .height(Length::Fixed(74.0))
            .style(|_| panel_style(rgb(0, 0, 0), rgb(52, 65, 79), rgb(220, 226, 234)))
            .into(),
        ClipSource::ScreenBar => screen_bar_preview(),
        other => container(
            row![
                fa_icon_solid(other.icon()).size(13.0),
                text(other.kind_label()).size(11),
            ]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
        )
        .padding(6)
        .height(Length::Fixed(74.0))
        .style(move |_| {
            panel_style(
                darken(accent, 0.40),
                darken(accent, 0.22),
                rgb(231, 236, 241),
            )
        })
        .into(),
    }
}

fn screen_bar_preview() -> Element<'static, Message> {
    let bands = row![
        color_band(rgb(237, 237, 237)),
        color_band(rgb(235, 235, 0)),
        color_band(rgb(0, 235, 235)),
        color_band(rgb(0, 214, 0)),
        color_band(rgb(235, 0, 235)),
        color_band(rgb(235, 0, 0)),
        color_band(rgb(0, 0, 235)),
    ]
    .spacing(0)
    .height(Length::Fill)
    .width(Length::Fill);

    container(bands)
        .height(Length::Fixed(74.0))
        .style(|_| panel_style(rgb(6, 8, 12), rgb(65, 78, 92), rgb(220, 226, 234)))
        .into()
}

fn color_band(color: iced::Color) -> Element<'static, Message> {
    container(column![])
        .width(Length::FillPortion(1))
        .height(Length::Fill)
        .style(move |_| iced::widget::container::Style::default().background(color))
        .into()
}
