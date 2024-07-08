//!Common data types.

use std::fmt;

use crate::sexp::constants::el;

///`Pos` sets the location (x, y) and orientation of an object.
#[derive(Debug, Copy, Clone, Default)]
pub struct Pos {
    pub x: f32,
    pub y: f32,
    pub angle: f32,
}

impl PartialEq for Pos {
    fn eq(&self, other: &Self) -> bool {
        let s_self = format!("{:.2}x{:.2}x{:.2}", self.x, self.y, self.angle);
        let s_other = format!("{:.2}x{:.2}x{:.2}", other.x, other.y, other.angle);
        s_self == s_other
    }
}
impl Eq for Pos {}

impl std::hash::Hash for Pos {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{:.2}x{:.2}x{:.2}", self.x, self.y, self.angle).hash(state);
    }
}

///`Pt` defines the positional coordinates of an object.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pt {
    pub x: f32,
    pub y: f32,
}

impl PartialEq for Pt {
    fn eq(&self, other: &Self) -> bool {
        format!("{:.2}x{:.2}", self.x, self.y) == format!("{:.2}x{:.2}", other.x, other.y)
    }
}
impl Eq for Pt {}

impl std::hash::Hash for Pt {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        format!("{:.2}x{:.2}", self.x, self.y).hash(state);
    }
}

impl From<Pos> for Pt {
    fn from(p: Pos) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl fmt::Display for Pt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2}x{:.2}", self.x, self.y)
    }
}

///The `Pts` token defines a list of X/Y coordinate points.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Pts(pub Vec<Pt>);

///The `Rect` token defines the start end enpoint of a Rectangle.
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
}

impl fmt::Display for Color {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Color::None => write!(f, "none"),
            Color::Rgb(r, g, b) => write!(f, "rgb({}, {}, {})", r, g, b),
            Color::Rgba(r, g, b, a) => write!(f, "rgba({}, {}, {}, {})", r, g, b, a),
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
            "background" => FillType::Background,
            "outline" => FillType::Outline,
            "none" => FillType::None,
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
                FillType::Background => "background",
                FillType::None => "none",
                FillType::Outline => "outline",
                FillType::Color(_) => "color",
            }
        )
    }
}

///The property token defines a symbol property when used inside a symbol definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Property {
    ///The ```key``` string defines the name of the property and must be unique.
    pub key: String,
    //The ```value``` string defines the value of the property.
    pub value: String,
    //The POSITION_IDENTIFIER defines the X and Y coordinates
    //and rotation angle of the property.
    pub pos: Pos,
    //The TEXT_EFFECTS section defines how the text is displayed.
    pub effects: Effects,
}

impl Property {
    ///Check if the property is visible
    pub fn visible(&self) -> bool {
        !self.effects.hide //TODO check for all hidden props [~, ki_], and where there are checked elsewhere
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
    pub fill: FillType,
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
    pub radius: f32,
    //The STROKE_DEFINITION defines how the circle outline is drawn.
    pub stroke: Stroke,
    //The FILL_DEFINTION defines how the circle is filled.
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
    //The FILL_DEFINTION defines how the curve is filled.
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
    //The FILL_DEFINTION defines how the rectangle is filled.
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

///All text objects can have an optional effects section
///that defines how the text is displayed.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Effects {
    pub font: Font,            // font attributes
    pub justify: Vec<Justify>, // text justification
    pub hide: bool,            // whether the text is hidden
}

///All text effects have an font section
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Font {
    pub face: Option<String>,      // TrueType font family name or "KiCad Font"
    pub size: (f32, f32),          // font height and width
    pub thickness: Option<f32>,    // line thickness of the font
    pub bold: bool,                // whether the font is bold
    pub italic: bool,              // whether the font is italicized
    pub line_spacing: Option<f32>, // spacing between lines (not yet supported)
    pub color: Option<Color>,      // spacing between lines (not yet supported)
}

impl Effects {
    pub fn anchor(&self) -> String {
        if self.justify.contains(&Justify::Right) {
            String::from("end")
        } else if self.justify.contains(&Justify::Left) {
            String::from("start")
        } else {
            String::from("middle")
        }
    }
    pub fn baseline(&self) -> String {
        if self.justify.contains(&Justify::Bottom) {
            String::from("auto")
        } else if self.justify.contains(&Justify::Top) {
            String::from("hanging")
        } else {
            String::from("middle")
        }
    }
}

///The Stroke struct represents a graphical object's outline drawing settings.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stroke {
    // Width of the graphic object's outline (in pixels).
    pub width: f32,
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
            "dashDot" => Self::DashDot,
            "dashDotDot" => Self::DashDotDot,
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
            StrokeType::DashDot => "dashDot",
            StrokeType::DashDotDot => "dashDotDot",
            StrokeType::Dot => "dot",
            StrokeType::Solid => "solid",
            StrokeType::Default => "default",
        };
        write!(f, "{}", s)
    }
}

#[derive(Default, Debug, Clone, Eq, PartialEq)]
pub enum Justify {
    Bottom,
    #[default]
    Center,
    Left,
    Mirror,
    Right,
    Top,
}

impl From<String> for Justify {
    fn from(s: String) -> Self {
        match s.as_str() {
            "bottom" => Justify::Bottom,
            "left" => Justify::Left,
            el::MIRROR => Justify::Mirror,
            "right" => Justify::Right,
            "top" => Justify::Top,
            _ => panic!("unknown justify: {}", s),
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

/// The paper siues. DIN pagper sizes are used.
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
impl std::convert::From<&String> for PaperSize {
    fn from(size: &String) -> Self {
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
impl std::convert::From<PaperSize> for (f32, f32) {
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
