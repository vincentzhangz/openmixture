use iced::{
    Element, Length, alignment,
    widget::{button, column, container, pick_list, row, slider, text},
};
use iced_font_awesome::fa_icon_solid;

use crate::ui::{
    constants::{MAX_TRANSITION_MS, MIN_TRANSITION_MS},
    message::Message,
    model::Transition,
    style::{action_button_style, panel_style, rgb},
};

pub fn view(transition: Transition, transition_ms: f32) -> Element<'static, Message> {
    let transition_picker = pick_list(
        &Transition::ALL[..],
        Some(transition),
        Message::SetTransition,
    )
    .width(Length::Fill);

    let controls = column![
        row![
            fa_icon_solid("shuffle").size(14.0),
            text("Transition").size(12)
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
        transition_picker,
        text(format!("{} ms", transition_ms as u16)).size(12),
        slider(
            MIN_TRANSITION_MS..=MAX_TRANSITION_MS,
            transition_ms,
            Message::SetTransitionDuration,
        )
        .width(Length::Fill),
        button(row![fa_icon_solid("scissors").size(14.0), text("CUT").size(12)].spacing(5))
            .on_press(Message::QuickCut)
            .style(|_, status| {
                action_button_style(
                    status,
                    rgb(204, 125, 28),
                    rgb(238, 176, 89),
                    rgb(250, 244, 236),
                )
            })
            .padding(7)
            .width(Length::Fill),
        button(row![fa_icon_solid("play").size(14.0), text("TAKE").size(12)].spacing(5))
            .on_press(Message::Take)
            .style(|_, status| {
                action_button_style(
                    status,
                    rgb(172, 34, 42),
                    rgb(226, 87, 97),
                    rgb(250, 238, 241),
                )
            })
            .padding(7)
            .width(Length::Fill),
    ]
    .spacing(6)
    .align_x(alignment::Horizontal::Center);

    container(controls)
        .padding(6)
        .width(Length::Fixed(168.0))
        .height(Length::Fill)
        .style(|_| panel_style(rgb(14, 21, 30), rgb(66, 83, 99), rgb(220, 228, 236)))
        .into()
}
