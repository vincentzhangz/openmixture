use iced::{
    Color, Element, alignment,
    widget::{container, row, text},
};
use iced_font_awesome::fa_icon_solid;

use crate::ui::{
    message::Message,
    style::{darken, panel_style, rgb, with_alpha},
};

pub fn view(title: &str, accent: Color, meta: String) -> Element<'static, Message> {
    let title = title.to_string();

    container(
        row![
            row![fa_icon_solid("display").size(12.0), text(title).size(13)]
                .spacing(6)
                .align_y(alignment::Vertical::Center),
            container(text(meta).size(11))
                .padding(3)
                .style(|_| { panel_style(rgb(17, 25, 34), rgb(69, 86, 103), rgb(219, 227, 236)) }),
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
    )
    .padding(6)
    .style(move |_| {
        panel_style(
            with_alpha(accent, 0.30),
            darken(accent, 0.30),
            rgb(240, 243, 246),
        )
    })
    .into()
}
