use std::collections::HashMap;

mod kicad2020;
mod blackwhite;

use types::{gr::Color, constants::{FONT_SCALE, el}};

#[derive(Debug, Copy, Clone)]
pub enum Themes {
    Kicad2020,
    BlackWhite,
}

impl std::fmt::Display for Themes { 
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { 
        let s = match self { 
            Self::Kicad2020 => "Kicad2020", 
            Self::BlackWhite => "BlackWhite", 
        }; 
        write!(f, "{}", s) 
    } 
} 

impl From<String> for Themes {
    fn from(str: String) -> Self {
        match str.as_str() {
            "Kicad2020" => Self::Kicad2020,
            "BlackWhite" => Self::BlackWhite,
            _ => Self::Kicad2020,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Theme {
    colors: HashMap<Style, Color>,
    fills: HashMap<Style, Color>,
    widths: HashMap<Style, f64>,
    font_sizes: HashMap<Style, (f32, f32)>,
}

struct ThemeDefinition {
    colors: &'static [(Style, Color)],
    fills: &'static [(Style, Color)],
    widths: &'static [(Style, f64)],
    font_sizes: &'static [(Style, (f32, f32))],
}

impl From<Themes> for Theme {
    fn from(theme: Themes) -> Self {
        let def = match theme {
            Themes::Kicad2020 => &kicad2020::KICAD_2020,
            Themes::BlackWhite => &blackwhite::BLACK_WHITE,
        };

        Self {
            colors: def.colors.iter().cloned().collect(),
            fills: def.fills.iter().cloned().collect(),
            widths: def.widths.iter().cloned().collect(),
            font_sizes: def.font_sizes.iter().cloned().collect(),
        }
    }
}

impl Theme {
    ///get the font face
    pub fn face(&self) -> String {
        String::from(el::OSIFONT)
    }

    ///get the font face
    pub fn font_size(&self, size: (f32, f32), style: Style) -> (f32, f32) {
        if size.0 == 0.0 {
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
        if let Some(color) = color {
            color
        } else {
            *self.colors.get(&style).unwrap_or(&Color::magenta())
        }
    }

    ///Get the fill color for the style.
    ///
    ///rule:
    ///- when the color is rgba(0,0,0,0) it is None and the theme color is used
    ///- otherwise take the in color
    pub fn fill(&self, color: Option<Color>, style: Style) -> Color {
        if let Some(color) = color {
            color
        } else if let Some(fill) = self.fills.get(&style) {
            *fill
        } else {
            panic!("unknown fill {:?}", style);
        }
    }

    ///Get the stroke width for the style.
    ///
    ///rule:
    ///- uses the passed in width if it is not zero.
    pub fn width(&self, width: f64, style: Style) -> f64 {
        if width > 0.0 {
            width * FONT_SCALE 
        } else if let Some(width) = self.widths.get(&style) {
            *width
        } else {
            *self.widths.get(&style).unwrap_or(&0.1)
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
    PinName,
    PinNumber,
    Border,
    TextSheet,
    Todo,
}

//implement from String for Style
impl From<String> for Style {
    fn from(str: String) -> Self {
        match str.as_str() {
            el::BACKGROUND => Self::Background,
            el::WIRE => Self::Wire,
            el::JUNCTION => Self::Junction,
            "noconnect" => Self::NoConnect,
            el::OUTLINE => Self::Outline,
            el::PROPERTY => Self::Property,
            "todo" => Self::Todo,
            _ => Self::Wire,
        }
    }
}

impl std::fmt::Display for Style {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Background => el::BACKGROUND,
            Self::Wire => el::WIRE,
            Self::Junction => el::JUNCTION,
            Self::NoConnect => "noconnect",
            Self::Outline => el::OUTLINE,
            Self::Property => el::PROPERTY,
            Self::Label => "label",
            Self::PinName => "pinname",
            Self::PinNumber => "pinnumber",
            Self::Border => "border",
            Self::TextSheet => "textsheet",
            Self::Todo => "todo",
        };
        write!(f, "{}", s)
    }
}

