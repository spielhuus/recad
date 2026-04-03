use std::{
    fmt,
    hash::{Hash, Hasher},
};

use crate::{
    constants::el,
    error::RecadError, //plot::{FontAnchor, FontBaseline}, 
};

///`Pos` sets the location (x, y) and orientation of an object.
#[derive(Debug, Copy, Clone, Default, PartialEq)]
pub struct Pos {
    pub x: f64,
    pub y: f64,
    pub angle: f64,
}

// TODO move this to where it is used
// impl Pos {
//     /// Helper to convert Pos directly to a Transformation Matrix
//     pub fn to_mat3(&self) -> DMat3 {
//         DMat3::from_scale_angle_translation(
//             DVec2::ONE,
//             self.angle.to_radians(),
//             DVec2::new(self.x, self.y),
//         )
//     }
// }

///`Pt` defines the positional coordinates of an object.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

impl From<Pos> for Pt {
    fn from(p: Pos) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl fmt::Display for Pt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} x {:.2}", self.x, self.y)
    }
}

// impl From<Pt> for DVec2 {
//     fn from(pt: Pt) -> Self {
//         DVec2::new(pt.x, pt.y)
//     }
// }
//
// impl From<DVec2> for Pt {
//     fn from(v: DVec2) -> Self {
//         Self { x: v.x, y: v.y }
//     }
// }

impl std::ops::Add for Pt {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}

impl std::ops::Sub for Pt {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}

impl std::ops::Mul<f64> for Pt {
    type Output = Self;

    fn mul(self, other: f64) -> Self {
        Self {
            x: self.x * other,
            y: self.y * other,
        }
    }
}

const SCALE: f64 = 10000.0;

///The `GridPt` struct represents a grid-aligned point with integer coordinates.
///
/// It is used for snapping points to a grid, typically for PCB or schematic design.
/// The coordinates are stored as `i32` values scaled by a factor of 10,000 to maintain
/// precision while using integers.
///
/// # Fields
/// * `x` - The X coordinate in grid units (scaled integer).
/// * `y` - The Y coordinate in grid units (scaled integer).
#[derive(Debug, Clone, Copy, Eq, PartialEq)] // Derive Eq/PartialEq automatically
pub struct GridPt {
    pub x: i32,
    pub y: i32,
}

impl Hash for GridPt {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.hash(state);
        self.y.hash(state);
    }
}

impl From<crate::gr::Pt> for GridPt {
    fn from(pt: crate::gr::Pt) -> Self {
        Self {
            x: (pt.x * SCALE).round() as i32,
            y: (pt.y * SCALE).round() as i32,
        }
    }
}

impl From<crate::gr::Pos> for GridPt {
    fn from(pt: crate::gr::Pos) -> Self {
        Self {
            x: (pt.x * SCALE).round() as i32,
            y: (pt.y * SCALE).round() as i32,
        }
    }
}

///The `Pts` token defines a list of X/Y coordinate points.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pts(pub Vec<Pt>);

///The `Rect` token defines the start end endpoint of a Rectangle.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rect {
    pub start: Pt,
    pub end: Pt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Mirror {
    X,
    Y,
    XY,
}

///```Color``` variants for different color types.
///
///The ```Class``` variant stores the kicad fill type,
#[derive(Debug, Copy, Clone, Default, Eq, PartialEq)]
pub enum Color {
    #[default]
    None,
    Rgb(u8, u8, u8),
    Rgba(u8, u8, u8, u8),
    //Class(FillType),
}

impl Color {
    pub fn black() -> Self {
        Self::Rgba(0, 0, 0, 255)
    }
    pub fn red() -> Self {
        Self::Rgba(255, 0, 0, 255)
    }
    pub fn green() -> Self {
        Self::Rgba(0, 255, 0, 255)
    }
    pub fn blue() -> Self {
        Self::Rgba(0, 0, 255, 255)
    }
    pub fn grey() -> Self {
        Self::Rgba(128, 128, 128, 255)
    }
    pub fn magenta() -> Self {
        Self::Rgba(255, 0, 255, 255)
    }

    pub fn to_hex(&self) -> String {
        match self {
            Color::None => el::NONE.to_string(),
            Color::Rgb(r, g, b) => format!("#{:02X}{:02X}{:02X}", r, g, b),
            Color::Rgba(r, g, b, _) => format!("#{:02X}{:02X}{:02X}", r, g, b),
        }
    }

    pub fn alpha(&self) -> f32 {
        match self {
            Color::None => 0.0,
            Color::Rgb(_, _, _) => 1.0,
            Color::Rgba(_, _, _, a) => *a as f32 / 255.0,
        }
    }
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::None => write!(f, "(0, 0, 0, 0)"),
            Color::Rgb(r, g, b) => write!(f, "rgb({}, {}, {})", r, g, b),
            Color::Rgba(r, g, b, a) => write!(f, "rgba({}, {}, {}, {})", r, g, b, a),
        }
    }
}

// implement the from trait and return [f64; 4]
impl From<Color> for [f32; 4] {
    fn from(c: Color) -> Self {
        // Simple sRGB to Linear conversion for WGPU
        fn srgb_to_linear(c: u8) -> f32 {
            let f = c as f32 / 255.0;
            if f <= 0.04045 {
                f / 12.92
            } else {
                ((f + 0.055) / 1.055).powf(2.4)
            }
        }

        match c {
            Color::None => [0.0, 0.0, 0.0, 0.0],
            Color::Rgb(r, g, b) => [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0],
            Color::Rgba(r, g, b, a) => [
                srgb_to_linear(r),
                srgb_to_linear(g),
                srgb_to_linear(b),
                a as f32 / 255.0,
            ],
        }
    }
}

#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub enum FillType {
    #[default]
    None,
    Background,
    Outline,
    Color(Color),
}

impl From<&str> for FillType {
    fn from(s: &str) -> Self {
        match s {
            el::BACKGROUND => FillType::Background,
            el::OUTLINE => FillType::Outline,
            el::NONE => FillType::None,
            _ => panic!("unknown fill type: {}", s),
        }
    }
}

impl fmt::Display for FillType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            match self {
                FillType::Background => el::BACKGROUND,
                FillType::None => el::NONE,
                FillType::Outline => el::OUTLINE,
                FillType::Color(_) => el::COLOR,
            }
        )
    }
}


///Enum to represent abstract graphic items.
#[derive(Clone, Debug, PartialEq)]
pub enum GraphicItem {
    Arc(Arc),
    Circle(Circle),
    Curve(Curve),
    Line(Line),
    Polyline(Polyline),
    Rectangle(Rectangle),
    Text(Text),
    EmbeddedFont(String),
}

///A `Polyline` in the schema or pcb
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Polyline {
    ///The COORDINATE_POINT_LIST defines the list of X/Y coordinates of the
    ///line(s). There must be a minimum of two points.
    pub pts: Pts,
    ///The STROKE_DEFINITION defines how the polygon formed by the lines
    ///outline is drawn.
    pub stroke: Stroke,
    ///The fill token attributes define how the polygon formed by the lines is filled.
    pub fill: Option<FillType>,
    /// Optional Universally unique identifier for the junction.
    /// This is used to identify the polyline on a schema.
    pub uuid: Option<String>,
}

///An `Arc` in the schema or pcb
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Arc {
    ///The start token defines the coordinates of start point of the arc.
    pub start: Pt,
    ///The mid token defines the coordinates of mid point of the arc.
    pub mid: Pt,
    ///The end token defines the coordinates of end point of the arc.
    pub end: Pt,
    ///The STROKE_DEFINITION defines how the arc outline is drawn.
    pub stroke: Stroke,
    ///The fill token attributes define how the arc is filled.
    pub fill: FillType,
    /// Optional Universally unique identifier for the junction.
    /// This is used to identify the arc on a schema.
    pub uuid: Option<String>,
}

///A `Circle` in the schema or pcb
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Circle {
    //The center token defines the coordinates of center point of the circle.
    pub center: Pt,
    //The radius token defines the length of the radius of the circle.
    pub radius: f64,
    //The STROKE_DEFINITION defines how the circle outline is drawn.
    pub stroke: Stroke,
    //The FILL_DEFINITION defines how the circle is filled.
    pub fill: FillType,
    /// Optional Universally unique identifier for the junction.
    /// This is used to identify the circle on a schema.
    pub uuid: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Curve {
    //The COORDINATE_POINT_LIST defines the four X/Y coordinates of each point of the curve.
    pub pts: Pts,
    //The STROKE_DEFINITION defines how the curve outline is drawn.
    pub stroke: Stroke,
    //The FILL_DEFINITION defines how the curve is filled.
    pub fill: FillType,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Line {
    //The COORDINATE_POINT_LIST defines the list of X/Y coordinates of the line(s). There must be a minimum of two points.
    pub pts: Pts,
    //The STROKE_DEFINITION defines how the polygon formed by the lines outline is drawn.
    pub stroke: Stroke,
    //The fill token attributes define how the polygon formed by the lines is filled.
    pub fill: FillType,
    /// Optional Universally unique identifier for the junction.
    /// This is used to identify the circle on a schema.
    pub uuid: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Rectangle {
    //The start token attributes define the coordinates of the start point of the rectangle.
    pub start: Pt,
    //The end token attributes define the coordinates of the end point of the rectangle.
    pub end: Pt,
    //The STROKE_DEFINITION defines how the rectangle outline is drawn.
    pub stroke: Stroke,
    //The FILL_DEFINITION defines how the rectangle is filled.
    pub fill: FillType,
    /// Optional Universally unique identifier for the junction.
    /// This is used to identify the circle on a schema.
    pub uuid: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Text {
    //The "TEXT" attribute is a quoted string that defines the text.
    pub text: String,
    //The POSITION_IDENTIFIER defines the X and Y coordinates and rotation angle of the text.
    pub pos: Pos,
    //The TEXT_EFFECTS defines how the text is displayed.
    pub effects: Effects,
    /// Optional Universally unique identifier for the junction.
    /// This is used to identify the circle on a schema.
    pub uuid: Option<String>,
}


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FontAnchor {
    Start,
    End,
    Middle,
}

impl fmt::Display for FontAnchor {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FontAnchor::Start => write!(f, "start"),
            FontAnchor::End => write!(f, "end"),
            FontAnchor::Middle => write!(f, "middle"),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FontBaseline {
    Auto,
    Hanging,
    Middle,
}

impl fmt::Display for FontBaseline {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FontBaseline::Auto => write!(f, "auto"),
            FontBaseline::Hanging => write!(f, "hanging"),
            FontBaseline::Middle => write!(f, "middle"),
        }
    }
}

///All text objects can have an optional effects section
///that defines how the text is displayed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Effects {
    /// font attributes
    pub font: Font,
    /// text justification
    pub justify: Vec<Justify>,
    /// whether the text is hidden
    pub hide: bool,
    pub z_index: f32,
}

impl Effects {
    pub fn anchor(&self) -> FontAnchor {
        if self.justify.contains(&Justify::Right) {
            FontAnchor::End
        } else if self.justify.contains(&Justify::Left) {
            FontAnchor::Start
        } else {
            FontAnchor::Middle
        }
    }
    pub fn baseline(&self) -> FontBaseline {
        if self.justify.contains(&Justify::Bottom) {
            FontBaseline::Auto
        } else if self.justify.contains(&Justify::Top) {
            FontBaseline::Hanging
        } else {
            FontBaseline::Middle
        }
    }

    //remove an item from the justification
    pub fn remove(&mut self, item: Justify) {
        self.justify.retain(|&x| x != item);
    }
}

///All text effects have an font section
#[derive(Debug, Clone, PartialEq)]
pub struct Font {
    /// TrueType font family name or "KiCad Font".
    pub face: Option<String>,
    /// The font's height and width.
    pub size: (f32, f32),
    /// The line thickness of the font.
    pub thickness: Option<f64>,
    /// Whether the font is bold.
    pub bold: bool,
    /// Whether the font is italicized.
    pub italic: bool,
    /// Spacing between lines (not yet supported).
    pub line_spacing: Option<f64>,
    /// Color of the text (not yet supported).
    pub color: Option<Color>,
}

impl Default for Font {
    fn default() -> Self {
        Self {
            face: None,
            size: (1.27, 1.27),
            thickness: None,
            bold: false,
            italic: false,
            line_spacing: None,
            color: None,
        }
    }
}

///The Stroke struct represents a graphical object's outline drawing settings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stroke {
    // Width of the graphic object's outline (in pixels).
    pub width: f64,
    // Type of line style to use when drawing the graphic object's outline.
    pub stroke_type: Option<StrokeType>, // An enum for different line styles.
    // Color settings for the graphic object's outline
    pub color: Option<Color>,
}

///The stroke token defines how the outlines of graphical objects are drawn.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum StrokeType {
    Dash,
    DashDot,
    DashDotDot,
    Dot,
    #[default]
    Default,
    Solid,
}

impl std::convert::From<&str> for StrokeType {
    fn from(s: &str) -> Self {
        match s {
            "dash" => Self::Dash,
            "dash_dot" => Self::DashDot,
            "dash_dot_dot" => Self::DashDotDot,
            "dot" => Self::Dot,
            "solid" => Self::Solid,
            _ => Self::Default,
        }
    }
}

impl fmt::Display for StrokeType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            StrokeType::Dash => "dash",
            StrokeType::DashDot => "dash_dot",
            StrokeType::DashDotDot => "dash_dot_dot",
            StrokeType::Dot => "dot",
            StrokeType::Solid => "solid",
            StrokeType::Default => "default",
        };
        write!(f, "{}", s)
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub enum Justify {
    Bottom,
    #[default]
    Center,
    Left,
    Mirror,
    Right,
    Top,
}

impl TryFrom<String> for Justify {
    type Error = RecadError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "bottom" => Ok(Justify::Bottom),
            "left" => Ok(Justify::Left),
            el::MIRROR => Ok(Justify::Mirror),
            "right" => Ok(Justify::Right),
            "top" => Ok(Justify::Top),
            _ => Err(RecadError::Pcb(format!("unknown justify: {}", s))),
        }
    }
}

impl fmt::Display for Justify {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            Justify::Bottom => "bottom",
            Justify::Left => "left",
            Justify::Mirror => el::MIRROR,
            Justify::Right => "right",
            Justify::Top => "top",
            Justify::Center => "center",
        };
        write!(f, "{}", s)
    }
}

/// The paper sizes. DIN paper sizes are used.
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub enum PaperSize {
    A5,
    #[default]
    A4,
    A3,
    A2,
    A1,
    A0,
}

///Display the paper size.
impl std::fmt::Display for PaperSize {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

///Parse the paper size from String.
impl std::convert::From<&str> for PaperSize {
    fn from(size: &str) -> Self {
        if size == "A5" {
            Self::A5
        } else if size == "A4" {
            Self::A4
        } else if size == "A3" {
            Self::A3
        } else if size == "A2" {
            Self::A2
        } else if size == "A1" {
            Self::A1
        } else {
            Self::A0
        }
    }
}

///Get the real paper size im mm.
impl std::convert::From<PaperSize> for (f64, f64) {
    fn from(size: PaperSize) -> Self {
        if size == PaperSize::A5 {
            (148.0, 210.0)
        } else if size == PaperSize::A4 {
            (297.0, 210.0)
        } else if size == PaperSize::A3 {
            (420.0, 297.0)
        } else if size == PaperSize::A2 {
            (420.0, 594.0)
        } else if size == PaperSize::A1 {
            (594.0, 841.0)
        } else {
            (841.0, 1189.0)
        }
    }
}

///The title_block token defines the contents of the title block.
#[derive(Debug, Clone, Default)]
pub struct TitleBlock {
    pub title: Option<String>,
    pub date: Option<String>,
    pub revision: Option<String>,
    pub company_name: Option<String>,
    pub comment: Vec<(u8, String)>,
}

impl std::fmt::Display for TitleBlock {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "TitleBlock {{ title: {:?}, date: {:?}, revision: {:?}, company_name: {:?}, comment: {:?} }}", 
            self.title, self.date, self.revision, self.company_name, self.comment)
    }
}
