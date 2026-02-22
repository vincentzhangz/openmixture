use iced::{
    Background, Border, Color, Shadow,
    widget::{button, container},
};

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgb8(r, g, b)
}

pub fn with_alpha(color: Color, alpha: f32) -> Color {
    Color { a: alpha, ..color }
}

pub fn darken(color: Color, amount: f32) -> Color {
    let clamp = |value: f32| value.clamp(0.0, 1.0);

    Color {
        r: clamp(color.r - amount),
        g: clamp(color.g - amount),
        b: clamp(color.b - amount),
        a: color.a,
    }
}

pub fn brighten(color: Color, amount: f32) -> Color {
    let clamp = |value: f32| value.clamp(0.0, 1.0);

    Color {
        r: clamp(color.r + amount),
        g: clamp(color.g + amount),
        b: clamp(color.b + amount),
        a: color.a,
    }
}

pub fn panel_style(background: Color, border: Color, text: Color) -> container::Style {
    container::Style::default()
        .background(background)
        .border(Border {
            width: 1.0,
            radius: 3.0.into(),
            color: border,
        })
        .color(text)
}

pub fn action_button_style(
    status: button::Status,
    background: Color,
    border: Color,
    text: Color,
) -> button::Style {
    let bg = match status {
        button::Status::Active => background,
        button::Status::Hovered => brighten(background, 0.07),
        button::Status::Pressed => darken(background, 0.05),
        button::Status::Disabled => with_alpha(background, 0.35),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: if matches!(status, button::Status::Disabled) {
            with_alpha(text, 0.45)
        } else {
            text
        },
        border: Border {
            width: 1.0,
            radius: 3.0.into(),
            color: border,
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn tile_button_style(status: button::Status, active: bool, accent: Color) -> button::Style {
    let base = if active {
        with_alpha(accent, 0.20)
    } else {
        Color::TRANSPARENT
    };

    let bg = match status {
        button::Status::Active => base,
        button::Status::Hovered => {
            if active {
                with_alpha(accent, 0.32)
            } else {
                with_alpha(rgb(65, 79, 94), 0.25)
            }
        }
        button::Status::Pressed => with_alpha(accent, 0.38),
        button::Status::Disabled => with_alpha(base, 0.35),
    };

    button::Style {
        background: Some(Background::Color(bg)),
        text_color: rgb(230, 235, 242),
        border: Border {
            width: 1.0,
            radius: 3.0.into(),
            color: if active { accent } else { rgb(63, 79, 95) },
        },
        shadow: Shadow::default(),
        snap: true,
    }
}

pub fn compact_name(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }

    let mut compact = String::new();

    for (index, ch) in value.chars().enumerate() {
        if index + 1 >= max {
            break;
        }

        compact.push(ch);
    }

    compact.push('…');
    compact
}
