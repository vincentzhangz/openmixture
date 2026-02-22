use iced::{
    Element, Length, alignment,
    widget::{container, row, text},
};
use iced_font_awesome::fa_icon_solid;

use crate::ui::{
    message::Message,
    style::{panel_style, rgb},
};

pub fn view(loaded_clips: usize, total_clips: usize) -> Element<'static, Message> {
    let right_status = row![
        container(text(format!("{loaded_clips}/{total_clips} Clips")).size(12))
            .padding(5)
            .style(|_| panel_style(rgb(43, 32, 84), rgb(99, 82, 156), rgb(231, 225, 247))),
    ]
    .spacing(6)
    .align_y(alignment::Vertical::Center);

    container(
        row![
            row![
                fa_icon_solid("video").size(16.0),
                text("OpenMixture").size(16)
            ]
            .spacing(8),
            container(right_status)
                .width(Length::Fill)
                .align_right(Length::Fill),
        ]
        .spacing(12)
        .align_y(alignment::Vertical::Center),
    )
    .padding(7)
    .style(|_| panel_style(rgb(10, 15, 22), rgb(44, 57, 71), rgb(225, 232, 240)))
    .into()
}
