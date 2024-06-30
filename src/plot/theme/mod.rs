use std::collections::HashMap;

use crate::{gr::Color, sexp::constants::el};

pub enum Themes {
    Kicad2020,
}

#[derive(Debug, Clone, Default)]
pub struct Theme {
    colors: HashMap<Style, Color>,
    widths: HashMap<Style, f32>,
    font_sizes: HashMap<Style, (f32, f32)>,
}

impl From<Themes> for Theme {
    fn from(theme: Themes) -> Self {
        
        let mut colors = HashMap::new();
        for c in COLORS {
            colors.insert(c.0, c.1);
        }

        let mut widths = HashMap::new();
        for c in WIDTHS {
            widths.insert(c.0, c.1);
        }
        
        let mut font_sizes = HashMap::new();
        for c in FONT_SIZES {
            font_sizes.insert(c.0, c.1);
        }

        Self {
            colors,
            widths,
            font_sizes,
        }
    }
}

impl Theme {

    ///get the font face
    pub fn face(&self) -> String {
        String::from("osifont")
    }

    ///get the font face
    pub fn font_size(&self, size: (f32, f32), style: Style) -> (f32, f32) {
        if  size.0 == 0.0 {
            *self.font_sizes.get(&style).unwrap()
        } else {
            size
        }
    }

    ///Get the color for the style.
    ///
    ///rule:
    ///- when the color is rgba(0,0,0,0) it is None and the theme color is used
    ///- otherwise take the in color
    pub fn color(&self, color: Option<Color>, style: Style) -> Color {
        log::trace!("get color {:?} -> {:?}", color, style);
        if let Some(color) = color {
            color
        } else {
            *self.colors.get(&style).unwrap()
        }
    }

    ///Get the stroke width for the style.
    ///
    ///rule:
    ///- overwrite the input width when it is defined in the theme.
    ///  this means that all width's that are 0.0 must be deffined
    ///  in the theme.
    pub fn width(&self, width: f32, style: Style) -> f32 {
        log::trace!("get width {:?} -> {:?}", width, style);
        if let Some(width) = self.widths.get(&style) {
            *width
        } else {
            *self.widths.get(&style).unwrap()
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Style {
    Background,
    Wire,
    Junction,
    NoConnect,
    Outline,
    Property,
    Label,
    Todo,
}

//implement from String for Style
impl From<String> for Style {
    fn from(str: String) -> Self {
        match str.as_str() {
            "background" => Self::Background,
            el::WIRE => Self::Wire,
            el::JUNCTION => Self::Junction,
            "noconnect" => Self::NoConnect,
            "outline" => Self::Outline,
            el::PROPERTY => Self::Property,
            "todo" => Self::Todo,
            _ => Self::Wire,
        }
    }
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Background => "background",
            Self::Wire => el::WIRE,
            Self::Junction => el::JUNCTION,
            Self::NoConnect => "noconnect",
            Self::Outline => "outline",
            Self::Property => el::PROPERTY,
            Self::Label => "label",
            Self::Todo => "todo",
        };
        write!(f, "{}", s)
    }
}

const COLORS: [(Style, Color); 5] = [
    (Style::Wire, Color::Rgba(0, 255, 0, 1)),
    (Style::NoConnect, Color::Rgba(255, 255, 0, 1)),
    (Style::Junction, Color::Rgba(155, 255, 12, 1)),
    (Style::Outline, Color::Rgba(0, 0, 0, 1)),
    (Style::Property, Color::Rgba(5, 105, 12, 1)),
];

const WIDTHS: [(Style, f32); 4] = [
    (Style::Wire, 0.35),
    (Style::NoConnect, 0.25),
    (Style::Junction, 0.25),
    (Style::Outline, 0.35),
];

const FONT_SIZES: [(Style, (f32, f32)); 2] = [
    (Style::Property, (1.75, 1.75)),
    (Style::Label, (1.75, 1.75)),
];
