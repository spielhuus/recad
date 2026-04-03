use types::gr::Color;
use crate::theme::{Style, ThemeDefinition};

pub const KICAD_2020: ThemeDefinition = ThemeDefinition {
    colors: &[
        (Style::Wire, Color::Rgba(0, 150, 0, 255)),
        (Style::NoConnect, Color::Rgba(0, 0, 132, 255)),
        (Style::Junction, Color::Rgba(0, 150, 0, 255)),
        (Style::Outline, Color::Rgba(132, 0, 0, 255)),
        (Style::PinName, Color::Rgba(132, 0, 0, 255)),
        (Style::PinNumber, Color::Rgba(132, 0, 0, 255)),
        (Style::Property, Color::Rgba(5, 105, 12, 255)),
        (Style::TextSheet, Color::Rgba(110, 110, 110, 255)),
        (Style::Border, Color::Rgba(90, 90, 90, 255)),
    ],
    fills: &[
        (Style::Background, Color::Rgba(255, 255, 194, 255)),
        (Style::Outline, Color::Rgba(132, 0, 0, 255)),
    ],
    widths: &[
        (Style::Wire, 0.35),
        (Style::NoConnect, 0.25),
        (Style::Junction, 0.1),
        (Style::Outline, 0.35),
    ],
    font_sizes: &[
        (Style::Property, (1.75, 1.75)),
        (Style::PinNumber, (0.25, 0.25)),
        (Style::PinName, (0.35, 0.35)),
        (Style::Label, (1.75, 1.75)),
        (Style::TextSheet, (2.5, 2.5)),
    ],
};
