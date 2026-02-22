use iced::{
    Element, Length, alignment,
    widget::{Row, Space, button, column, container, row, text},
};
use iced_font_awesome::fa_icon_solid;

use crate::ui::{
    message::Message,
    model::{ClipSlot, ViewTarget},
    style::{action_button_style, panel_style, rgb},
};

use super::{bus_header, clip_tile};

pub struct InputBankProps<'a> {
    pub clip_slots: &'a [ClipSlot],
    pub preview_target: ViewTarget,
    pub program_target: ViewTarget,
    pub armed_slot: usize,
    pub drop_hovered: bool,
}

pub fn view(props: InputBankProps<'_>) -> Element<'_, Message> {
    let loaded = props
        .clip_slots
        .iter()
        .filter(|slot| !slot.is_black())
        .count();

    let title = bus_header::view(
        "Inputs",
        rgb(38, 122, 78),
        format!("{loaded}/{} clips", props.clip_slots.len()),
    );

    let clip_strip =
        props
            .clip_slots
            .iter()
            .enumerate()
            .fold(Row::new().spacing(6), |row, (index, slot)| {
                row.push(clip_tile::view(clip_tile::ClipTileProps {
                    index,
                    slot,
                    preview_target: props.preview_target,
                    program_target: props.program_target,
                    armed_slot: props.armed_slot,
                }))
            });

    let drop_message = if props.drop_hovered {
        "Release to add new input"
    } else {
        "Drag & drop image or video files to add new clips"
    };

    let add_input = button(
        row![fa_icon_solid("plus").size(12.0), text("Add Input").size(12),]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
    )
    .on_press(Message::OpenAddInputPicker)
    .padding(7)
    .style(|_, status| {
        action_button_style(
            status,
            rgb(33, 120, 76),
            rgb(71, 172, 118),
            rgb(240, 247, 243),
        )
    });

    container(
        column![
            title,
            text(drop_message).size(11),
            text("Media Bin").size(11),
            clip_strip,
            Space::new().height(Length::Fill),
            add_input,
        ]
        .spacing(6),
    )
    .padding(6)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| panel_style(rgb(12, 18, 27), rgb(53, 71, 88), rgb(218, 226, 236)))
    .into()
}
