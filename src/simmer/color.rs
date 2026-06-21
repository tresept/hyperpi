use crossterm::style::Color;

pub(crate) fn auto_pause_color(base: Color) -> Color {
    lerp_color(base, Color::Rgb { r: 0, g: 0, b: 0 }, 0.55)
}

pub(crate) fn auto_highlight(base: Color) -> Color {
    if let Color::Rgb { .. } = base {
        lerp_color(
            base,
            Color::Rgb {
                r: 255,
                g: 255,
                b: 255,
            },
            0.65,
        )
    } else {
        Color::White
    }
}

pub(crate) fn lerp_color(from: Color, to: Color, t: f32) -> Color {
    if let (
        Color::Rgb {
            r: from_r,
            g: from_g,
            b: from_b,
        },
        Color::Rgb {
            r: to_r,
            g: to_g,
            b: to_b,
        },
    ) = (from, to)
    {
        Color::Rgb {
            r: lerp_u8(from_r, to_r, t),
            g: lerp_u8(from_g, to_g, t),
            b: lerp_u8(from_b, to_b, t),
        }
    } else {
        to
    }
}

fn lerp_u8(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t).round() as u8
}
