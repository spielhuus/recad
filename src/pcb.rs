use std::collections::HashMap;

use crate::gr::{Pos, Pt, Pts, Stroke};


///Definition of the layer type
pub struct Layer {
  ///The layer ORDINAL is an integer used to associate the layer stack ordering. 
  ///This is mostly to ensure correct mapping when the number of layers is 
  ///increased in the future.
  pub ordinal: u32,
	///The CANONICAL_NAME is the layer name defined for internal board use.
  pub canonical_name: String,
	///The layer TYPE defines the type of layer and can be defined as 
  ///jumper, mixed, power, signal, or user.
  pub layer_type: LayerType,
	///The optional USER_NAME attribute defines the custom user name.
  pub user_name: String,

}

//create a layer type enum
pub enum LayerType {
  Jumper,
  Mixed,
  Power,
  Signal,
  User,
}

pub enum ViaType {
    Blind,
    Micro,
}

/// Defines a track segment in a PCB design.
pub struct Segment {
    /// Coordinates of the beginning of the line.
    pub start: Pt,

    /// Coordinates of the end of the line.
    pub end: Pt,

    /// Line width.
    pub width: f32,

    /// The canonical layer the track segment resides on.
    pub layer: String,

    /// Indicates if the line cannot be edited.
    pub locked: bool,

    /// The net number that the segment belongs to.
    pub net: u32,

    /// A unique identifier for the line object.
    pub tstamp: String,
}

pub struct Via {
    /// Specifies the via type. Valid via types are `blind` and `micro`.
    /// If no type is defined, the via is a through-hole type.
    pub via_type: Option<ViaType>, 
    /// Indicates if the via cannot be edited.
    pub locked: bool,
    /// Coordinates of the center of the via.
    pub pos: Pos,
    /// Diameter of the via's annular ring.
    pub size: f64,
    /// Diameter of the drill hole for the via.
    pub drill: f64,
    /// The layers that the via connects.
    pub layers: (String, String),
    /// Specifies whether to remove unused layers.
    pub remove_unused_layers: bool,
    /// Specifies whether to keep end layers.
    /// This is only relevant when `remove_unused_layers` is true.
    pub keep_end_layers: bool,
    /// Indicates that the via is free to be moved outside its assigned net.
    pub free: bool,
    /// The net number that the via belongs to.
    pub net: u32,
    /// A unique identifier for the via.
    pub tstamp: String,
}

///The ```net``` token defines a net for the board. This section is required.
pub struct Net {
    ///The oridinal attribute is an integer that defines the net order.
    pub ordinal: u32,
    ///The net name is a string that defines the name of the net.
    pub name: String,
}

///defines a footprint type
pub enum FootprintType {
    Smd,
    ThroughHole,
    ExcludeFromPosFiles,
}

impl From<String> for FootprintType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "smd" => FootprintType::Smd,
            "through_hole" => FootprintType::ThroughHole,
            "exclude_from_pos_files" => FootprintType::ExcludeFromPosFiles,
            _ => panic!("Invalid footprint type: {}", s),
        }
    }
}

/// Defines a footprint in a PCB design.
pub struct Footprint {
    /// The link to the footprint library. Only applies to footprints defined in the board file format.
    pub library_link: String,

    /// Indicates that the footprint cannot be edited.
    pub locked: bool,

    /// Indicates that the footprint has not been placed.
    pub placed: bool,

    /// The canonical layer the footprint is placed.
    pub layer: String,

    /// The last time the footprint was edited.
    pub tedit: Option<String>,

    /// The unique identifier for the footprint. Only applies to footprints defined in the board file format.
    pub tstamp: Option<String>,

    /// The X and Y coordinates and rotational angle of the footprint. Only applies to footprints defined in the board file format.
    pub pos: Pos,

    /// A string containing the description of the footprint.
    pub descr: Option<String>,

    /// A string of search tags for the footprint.
    pub tags: Option<String>,

    /// A property for the footprint.
    pub property: HashMap<String, String>,

    /// The hierarchical path of the schematic symbol linked to the footprint. Only applies to footprints defined in the board file format.
    pub path: Option<String>,

    /// The vertical cost when using the automatic footprint placement tool. Valid values are integers 1 through 10. Only applies to footprints defined in the board file format.
    pub autoplace_cost90: Option<u8>,

    /// The horizontal cost when using the automatic footprint placement tool. Valid values are integers 1 through 10. Only applies to footprints defined in the board file format.
    pub autoplace_cost180: Option<u8>,

    /// The solder mask distance from all pads in the footprint. If not set, the board solder_mask_margin setting is used.
    pub solder_mask_margin: Option<f32>,

    /// The solder paste distance from all pads in the footprint. If not set, the board solder_paste_margin setting is used.
    pub solder_paste_margin: Option<f32>,

    /// The percentage of the pad size used to define the solder paste for all pads in the footprint. If not set, the board solder_paste_ratio setting is used.
    pub solder_paste_ratio: Option<f32>,

    /// The clearance to all board copper objects for all pads in the footprint. If not set, the board clearance setting is used.
    pub clearance: Option<f32>,

    /// How all pads are connected to filled zones. Valid values are 0 to 3.
    /// 0: Pads are not connected to the zone.
    /// 1: Pads are connected to the zone using thermal reliefs.
    /// 2: Pads are connected to the zone using solid fill.
    pub zone_connect: Option<u8>,

    /// The thermal relief spoke width used for zone connections for all pads in the 
    /// footprint. Only affects pads connected to zones with thermal reliefs. 
    /// If not set, the zone thermal_width setting is used.
    pub thermal_width: Option<f32>,

    /// The distance from the pad to the zone of thermal relief connections for all 
    /// pads in the footprint. If not set, the zone thermal_gap setting is used.
    pub thermal_gap: Option<f32>,

    /// The footprint type.
    pub footprint_type: FootprintType,

    ///The optional board_only token indicates that the footprint is only defined in 
    ///the board and has no reference to any schematic symbol.
    pub board_only: bool,

    ///The optional exclude_from_pos_files token indicates that the footprint 
    ///position information should not be included when creating position files.
    pub exclude_from_pos_files: bool,

    ///The optional exclude_from_bom token indicates that the footprint should 
    ///be excluded when creating bill of materials (BOM) files.
    pub exclude_from_bom: bool,

    /// A list of canonical layer names which are private to the footprint.
    pub private_layers: Option<Vec<String>>,

    /// A list of net-tie pad groups.
    pub net_tie_pad_groups: Option<Vec<String>>,

    /// A list of one or more graphical objects in the footprint.
    pub graphic_items: Vec<GraphicItem>,

    /// A list of pads in the footprint.
    pub pads: Vec<Pad>,

    /// A list of keep out zones in the footprint.
    pub zones: Vec<String>,

    /// A list of grouped objects in the footprint.
    pub groups: Vec<String>,

    /// The 3D model object associated with the footprint.
    pub model_3d: Option<String>,
}

/// Defines text in a footprint definition.
pub struct FpText {
    /// The type of text. Valid types are reference, value, and user.
    pub text_type: String,
    /// The text string.
    pub text: String,
    /// The position identifier with X, Y coordinates and optional orientation angle.
    pub pos: Pos,
    /// Indicates if the text orientation can be anything other than the upright orientation.
    pub unlocked: bool,
    /// The canonical layer the text resides on.
    pub layer: String,
    /// Indicates if the text is hidden.
    pub hide: bool,
    /// Defines how the text is displayed.
    pub effects: String,
    /// The unique identifier of the text object.
    pub tstamp: String,
}

/// Defines a rectangle containing line-wrapped text in a footprint.
pub struct FpTextBox {
    /// Specifies if the text box can be moved.
    pub locked: bool,
    /// The content of the text box.
    pub text: String,
    /// Defines the top-left of a cardinally oriented text box.
    pub start: Option<(f64, f64)>,
    /// Defines the bottom-right of a cardinally oriented text box.
    pub end: Option<(f64, f64)>,
    /// Defines the four corners of a non-cardinally oriented text box.
    pub pts: Option<Vec<(f64, f64)>>,
    /// Defines the rotation of the text box in degrees.
    pub angle: Option<f64>,
    /// The canonical layer the text box resides on.
    pub layer: String,
    /// The unique identifier of the text box.
    pub tstamp: String,
    /// The style of the text in the text box.
    pub text_effects: String,
    /// The style of an optional border to be drawn around the text box.
    pub stroke_definition: Option<String>,
    /// A render cache for TrueType fonts.
    pub render_cache: Option<String>,
}

/// Defines a graphic line in a footprint.
pub struct FpLine {
    /// The coordinates of the beginning of the line.
    pub start: Pt,
    /// The coordinates of the end of the line.
    pub end: Pt,
    /// The canonical layer the line resides on.
    pub layer: String,
    //TODO The line width.
    //pub width: f32,
    /// The style of the line.
    pub stroke: Stroke,
    /// Indicates if the line cannot be edited.
    pub locked: bool,
    /// The unique identifier of the line object.
    pub tstamp: String,
}

/// Defines a graphic rectangle in a footprint.
pub struct FpRect {
    /// The coordinates of the upper left corner of the rectangle.
    pub start: Pt,
    /// The coordinates of the lower right corner of the rectangle.
    pub end: Pt,
    /// The canonical layer the rectangle resides on.
    pub layer: String,
    /// The line width of the rectangle.
    pub width: f32,
    /// The style of the rectangle.
    pub stroke_definition: Option<String>,
    /// Defines how the rectangle is filled. Valid types are solid and none.
    pub fill: Option<String>,
    /// Indicates if the rectangle cannot be edited.
    pub locked: bool,
    /// The unique identifier of the rectangle object.
    pub tstamp: String,
}

/// Defines a graphic circle in a footprint.
pub struct FpCircle {
    /// The coordinates of the center of the circle.
    pub center: Pt,
    /// The coordinates of the end of the radius of the circle.
    pub end: Pt,
    /// The canonical layer the circle resides on.
    pub layer: String,
    /// The line width of the circle.
    pub width: f32,
    /// The style of the circle.
    pub stroke_definition: Option<String>,
    /// Defines how the circle is filled. Valid types are solid and none.
    pub fill: Option<String>,
    /// Indicates if the circle cannot be edited.
    pub locked: bool,
    /// The unique identifier of the circle object.
    pub tstamp: String,
}

/// Defines a graphic arc in a footprint.
pub struct FpArc {
    /// The coordinates of the start position of the arc radius.
    pub start: Pt,
    /// The coordinates of the midpoint along the arc.
    pub mid: Pt,
    /// The coordinates of the end position of the arc radius.
    pub end: Pt,
    /// The canonical layer the arc resides on.
    pub layer: String,
    /// The line width of the arc.
    pub width: f32,
    /// The style of the arc.
    pub stroke_definition: Option<String>,
    /// Indicates if the arc cannot be edited.
    pub locked: bool,
    /// The unique identifier of the arc object.
    pub tstamp: String,
}

/// Defines a graphic polygon in a footprint.
pub struct FpPoly {
    /// The list of X/Y coordinates of the polygon outline.
    pub pts: Pts,
    /// The canonical layer the polygon resides on.
    pub layer: String,
    /// The line width of the polygon.
    pub width: f32,
    /// The style of the polygon.
    pub stroke_definition: Option<String>,
    /// Defines how the polygon is filled. Valid types are solid and none.
    pub fill: Option<String>,
    /// Indicates if the polygon cannot be edited.
    pub locked: bool,
    /// The unique identifier of the polygon object.
    pub tstamp: String,
}

/// Defines a graphic Cubic Bezier curve in a footprint.
pub struct FpCurve {
    /// The four X/Y coordinates of each point of the curve.
    pub pts: Pts,
    /// The canonical layer the curve resides on.
    pub layer: String,
    /// The line width of the curve.
    pub width: f32,
    /// The style of the curve.
    pub stroke_definition: Option<String>,
    /// Indicates if the curve cannot be edited.
    pub locked: bool,
    /// The unique identifier of the curve object.
    pub tstamp: String,
}

/// Enum for different graphic items
pub enum GraphicItem {
    FpLine(FpLine),
    FpRect(FpRect),
    FpArc(FpArc),
    FpCircle(FpCircle),
    FpCurve(FpCurve),
    FpPoly(FpPoly),
    AnnotationBoundingBox,
}

/// Struct for custom pad primitives
pub struct CustomPadPrimitives {
    /// List of graphical items defining the custom pad shape
    pub graphic_items: Vec<GraphicItem>,
    /// Line width of the graphical items
    pub width: f32,
    /// Optional: If the geometry defined by graphical items should be filled
    pub fill: Option<bool>,
}

pub enum PadType {
    ThruHole,
    Smd,
    Connect,
    NpThruHole,
}

impl From<String> for PadType {
    fn from(s: String) -> Self {
        match s.as_str() {
            "thru_hole" => PadType::ThruHole,
            "smd" => PadType::Smd,
            "connect" => PadType::Connect,
            "np_thru_hole" => PadType::NpThruHole,
            _ => panic!("Invalid pad type: {}", s),
        }
    }
}

pub enum PadShape {
    Circle,
    Rect,
    Oval,
    Trapezoid,
    RoundRect,
    Custom,
}

//impl the from trait for PadShape using String
impl From<String> for PadShape {
    fn from(s: String) -> Self {
        match s.as_str() {
            "circle" => PadShape::Circle,
            "rect" => PadShape::Rect,
            "oval" => PadShape::Oval,
            "trapezoid" => PadShape::Trapezoid,
            "roundrect" => PadShape::RoundRect,
            "custom" => PadShape::Custom,
            _ => panic!("Invalid pad shape: {}", s),
        }
    }
}

/// Struct for custom pad options
pub struct CustomPadOptions {
    /// Type of clearance for custom pad (outline, convexhull)
    pub clearance_type: Option<String>,
    /// Anchor pad shape of custom pad (rect, circle)
    pub anchor_pad_shape: Option<String>,
}

/// Struct for a pad drill definition
#[derive(Debug)]
pub struct DrillDefinition {
    /// Optional: If the drill is oval
    pub oval: Option<bool>,
    /// Drill diameter
    pub diameter: f32,
    /// Optional: Width of the slot for oval drills
    pub width: Option<f32>,
    /// Optional: X coordinate of drill offset
    pub offset_x: Option<f32>,
    /// Optional: Y coordinate of drill offset
    pub offset_y: Option<f32>,
}

/// Main struct for a footprint pad
pub struct Pad {
    /// Pad number
    pub number: String,
    /// Pad type (thru_hole, smd, connect, np_thru_hole)
    pub pad_type: PadType,
    /// Pad shape (circle, rect, oval, trapezoid, roundrect, custom)
    pub shape: PadShape,
    /// Position identifier (X, Y, orientation)
    pub pos: Pos,
    /// Optional: If the pad is locked
    //TODO pub locked: Option<bool>,
    /// size of the pad
    pub size: (f32, f32),
    /// Optional: Drill definition for the pad
    pub drill: Option<f32>,
    /// Layers the pad resides on
    //pub canonical_layer_list: String,
    /// Optional: Special properties for the pad
    //pub properties: Option<Vec<String>>,
    /// Optional: Remove copper from layers pad is not connected to
    //pub remove_unused_layer: Option<bool>,
    /// Optional: Retain top and bottom layers when removing copper
    //pub keep_end_layers: Option<bool>,
    /// Optional: Scaling factor of pad to corner radius for roundrect/chamfered pads (0 to 1)
    //pub roundrect_rratio: Option<f32>,
    /// Optional: Scaling factor of pad to chamfer size (0 to 1)
    //pub chamfer_ratio: Option<f32>,
    /// Optional: List of pad corners that get chamfered (top_left, top_right, bottom_left, bottom_right)
    //pub chamfer: Option<Vec<String>>,
    /// Integer number and name string of the net connection for the pad
    pub net: Net,
    /// Unique identifier of the pad object
    pub tstamp: String,
    // Optional: Schematic symbol pin name
    //pub pinfunction: Option<String>,
    // Optional: Schematic pin electrical type
    //pub pintype: Option<String>,
    // Optional: Die length between the component pad and physical chip inside the package
    //pub die_length: Option<f32>,
    // Optional: Distance between the pad and the solder mask
    //pub solder_mask_margin: Option<f32>,
    // Optional: Distance the solder paste should be changed for the pad
    //pub solder_paste_margin: Option<f32>,
    // Optional: Percentage to reduce pad outline by to generate solder paste size
    //pub solder_paste_margin_ratio: Option<f32>,
    // Optional: Clearance from all copper to the pad
    //pub clearance: Option<f32>,
    // Optional: Type of zone connect for the pad (0 to 3)
    //pub zone_connect: Option<i32>,
    // Optional: Thermal relief spoke width for zone connection
    //pub thermal_width: Option<f32>,
    // Optional: Distance from the pad to the zone of the thermal relief connection
    //pub thermal_gap: Option<f32>,
    // Optional: Options for a custom pad
    //pub custom_pad_options: Option<CustomPadOptions>,
    // Optional: Drawing objects and options for defining a custom pad
    //pub custom_pad_primitives: Option<CustomPadPrimitives>,
}

