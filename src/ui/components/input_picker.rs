use std::path::Path;

use iced::{
    Element, Length, alignment,
    widget::{Space, button, column, container, row, text},
};
use iced_font_awesome::fa_icon_solid;

use crate::ui::{
    message::Message,
    model::InputTemplate,
    style::{action_button_style, panel_style, rgb, with_alpha},
};

pub struct InputPickerProps<'a> {
    pub selected: InputTemplate,
    pub selected_media_path: Option<&'a Path>,
    pub can_confirm: bool,
}

pub fn view(props: InputPickerProps<'_>) -> Element<'_, Message> {
    let left_list = InputTemplate::ALL
        .iter()
        .copied()
        .fold(column!().spacing(4), |column, template| {
            column.push(template_item(template, props.selected == template))
        });

    let prompt = container(text(props.selected.prompt()).size(12))
        .padding(9)
        .width(Length::Fill)
        .style(|_| panel_style(rgb(8, 13, 20), rgb(42, 58, 76), rgb(229, 236, 243)));

    let detail_panel = if props.selected.is_media() {
        media_detail_panel(props.selected_media_path)
    } else {
        static_detail_panel(props.selected)
    };

    let ok_button = button(
        row![fa_icon_solid("check").size(12.0), text("OK").size(12),]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
    )
    .on_press_maybe(props.can_confirm.then_some(Message::ConfirmAddInput))
    .padding([7, 16])
    .style(|_, status| {
        action_button_style(
            status,
            rgb(30, 116, 70),
            rgb(70, 170, 116),
            rgb(241, 247, 243),
        )
    });

    let cancel_button = button(
        row![fa_icon_solid("xmark").size(12.0), text("Cancel").size(12),]
            .spacing(6)
            .align_y(alignment::Vertical::Center),
    )
    .on_press(Message::CloseAddInputPicker)
    .padding([7, 16])
    .style(|_, status| {
        action_button_style(
            status,
            rgb(44, 58, 74),
            rgb(82, 100, 119),
            rgb(232, 237, 243),
        )
    });

    let right_content = column![
        prompt,
        detail_panel,
        row![Space::new().width(Length::Fill), ok_button, cancel_button]
            .spacing(8)
            .width(Length::Fill)
            .align_y(alignment::Vertical::Center),
    ]
    .spacing(8)
    .height(Length::Fill);

    container(
        row![
            container(left_list)
                .width(Length::Fixed(210.0))
                .height(Length::Fill)
                .padding(8)
                .style(|_| panel_style(rgb(7, 12, 19), rgb(49, 66, 84), rgb(220, 228, 236))),
            container(right_content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(8)
                .style(|_| panel_style(rgb(14, 19, 28), rgb(62, 80, 98), rgb(223, 230, 238))),
        ]
        .spacing(8)
        .height(Length::Fill),
    )
    .padding(8)
    .width(Length::Fill)
    .height(Length::Fill)
    .style(|_| panel_style(rgb(11, 16, 24), rgb(56, 74, 92), rgb(224, 231, 239)))
    .into()
}

fn template_item(template: InputTemplate, active: bool) -> Element<'static, Message> {
    let accent = if active {
        rgb(38, 121, 72)
    } else {
        rgb(52, 67, 84)
    };

    let background = if active {
        rgb(15, 84, 28)
    } else {
        rgb(11, 20, 30)
    };

    button(
        container(
            row![
                fa_icon_solid(template.icon()).size(13.0),
                column![
                    text(template.title()).size(13),
                    text(template.description()).size(10),
                ]
                .spacing(1),
            ]
            .spacing(8)
            .align_y(alignment::Vertical::Center),
        )
        .padding(8)
        .width(Length::Fill)
        .style(move |_| panel_style(background, accent, rgb(240, 245, 250))),
    )
    .on_press(Message::SelectInputTemplate(template))
    .padding(0)
    .width(Length::Fill)
    .style(move |_, status| {
        let base = if active {
            with_alpha(rgb(46, 159, 96), 0.32)
        } else {
            with_alpha(rgb(57, 76, 95), 0.16)
        };

        action_button_style(status, base, accent, rgb(234, 240, 246))
    })
    .into()
}

fn media_detail_panel(selected_media_path: Option<&Path>) -> Element<'static, Message> {
    let file_text = selected_media_path
        .and_then(Path::to_str)
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| "No media selected".to_string());

    let browse = button(
        row![
            fa_icon_solid("folder-open").size(12.0),
            text("Browse").size(12),
        ]
        .spacing(6)
        .align_y(alignment::Vertical::Center),
    )
    .on_press(Message::BrowseInputMedia)
    .padding([7, 12])
    .style(|_, status| {
        action_button_style(
            status,
            rgb(37, 72, 117),
            rgb(75, 126, 193),
            rgb(236, 242, 248),
        )
    });

    let chosen_name = selected_media_path
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("Drop or browse a file")
        .to_string();

    column![
        row![
            container(text(file_text).size(11))
                .padding(7)
                .width(Length::Fill)
                .style(|_| panel_style(rgb(10, 17, 26), rgb(50, 68, 87), rgb(223, 232, 241))),
            browse,
        ]
        .spacing(8)
        .align_y(alignment::Vertical::Center),
        container(
            column![
                row![fa_icon_solid("film").size(42.0), text(chosen_name).size(18),]
                    .spacing(12)
                    .align_y(alignment::Vertical::Center),
                text("Supported: video and image files").size(11),
            ]
            .spacing(8)
            .align_x(alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(|_| panel_style(rgb(18, 24, 33), rgb(66, 84, 102), rgb(222, 229, 236))),
    ]
    .spacing(8)
    .height(Length::Fill)
    .into()
}

fn static_detail_panel(template: InputTemplate) -> Element<'static, Message> {
    container(
        column![
            row![
                fa_icon_solid(template.icon()).size(48.0),
                text(template.title()).size(24),
            ]
            .spacing(14)
            .align_y(alignment::Vertical::Center),
            text(template.description()).size(13),
        ]
        .spacing(10)
        .align_x(alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(|_| panel_style(rgb(18, 24, 33), rgb(66, 84, 102), rgb(222, 229, 236)))
    .into()
}
