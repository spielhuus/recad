use crate::gr::{GraphicItem, Polyline, Pos, Property, Pt};


///A `Footprint` in the schema
///
///Represents a footprint in the board file format.
#[derive(Debug, Clone, PartialEq)]
pub struct Footprint {
    /// Link to the footprint library.
    pub library_link: Option<String>,
    /// Indicates if the footprint is locked.
    pub locked: bool,
    /// Indicates if the footprint is placed.
    pub placed: bool,
    /// The canonical layer the footprint is placed on.
    pub layer: Layer,
    /// The last time the footprint was edited.
    pub tedit: String,
    /// The unique identifier for the footprint.
    pub tstamp: Option<String>,
    /// The position of the footprint within the board.
    pub pos: Option<Pos>,
    /// A string of search tags for the footprint.
    pub tags: Option<String>,
    /// A string containing the description of the footprint.
    pub descr: Option<String>,
    /// A list of properties for the footprint.
    pub properties: Vec<Property>,
    /// The hierarchical path of the schematic symbol linked to the footprint.
    pub path: String,
    /// The vertical cost for automatic footprint placement.
    pub autoplace_cost90: Option<u8>,
    /// The horizontal cost for automatic footprint placement.
    pub autoplace_cost180: Option<u8>,
    /// The solder mask distance from all pads in the footprint.
    pub solder_mask_margin: Option<f32>,
    /// The solder paste distance from all pads in the footprint.
    pub solder_paste_margin: Option<f32>,
    /// The percentage of the pad size used for solder paste.
    pub solder_paste_ratio: Option<f32>,
    /// The clearance to all board copper objects for all pads.
    pub clearance: Option<f32>,
    /// How all pads are connected to filled zones.
    pub zone_connect: Option<ConnectionType>,
    /// The thermal relief spoke width for zone connections.
    pub thermal_width: Option<f32>,
    /// The distance from the pad to the zone for thermal relief connections.
    pub thermal_gap: Option<f32>,
    /// The attributes of the footprint.
    pub attributes: Option<Attributes>,
    /// A list of canonical layer names private to the footprint.
    pub private_layers: Option<Vec<Layer>>,
    /// A list of net-tie pad groups.
    pub net_tie_pad_groups: Option<Vec<String>>,
    /// A list of graphical objects in the footprint.
    pub graphic_items: Vec<GraphicItem>,
    /// A list of pads in the footprint.
    pub pads: Vec<Pad>,
    /// A list of keep out zones in the footprint.
    pub zones: Vec<Zone>,
    //TODO /// A list of grouped objects in the footprint.
    //pub groups: Vec<Group>,
    ///// The 3D model associated with the footprint.
    //pub model_3d: Option<Model3D>,
}

///Definition of the layer type
#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub enum LayerType {
  Jumper,
  Mixed,
  Power,
  Signal,
  User,
}

/// The optional zone_connect token defines how all pads are connected to filled zone
#[derive(Clone, Debug, PartialEq)]
enum ConnectionType {
    /// Pads are not connect to zone.
    NoConnect,
    // /Pads are connected to zone using thermal reliefs.
    ThermalRelief,
    /// Pads are connected to zone using solid fill.
    Fill,
}

/// Attributes of the footprint.
///
/// Defines the list of attributes of the footprint.
#[derive(Debug, Clone, PartialEq)]
pub struct Attributes {
    /// The type of footprint.
    pub attr_type: FootprintType,
    /// Indicates if the footprint is only defined in the board.
    pub board_only: bool,
    /// Indicates if the footprint should be excluded from position files.
    pub exclude_from_pos_files: bool,
    /// Indicates if the footprint should be excluded from bill of materials (BOM) files.
    pub exclude_from_bom: bool,
}

/// Defines the type of footprint.
#[derive(Debug, Clone, PartialEq)]
pub enum FootprintType {
    /// Surface Mount Device (SMD) type.
    SMD,
    /// Through-hole type.
    ThroughHole,
}

/// Main struct for a footprint pad
#[derive(Clone, Debug, PartialEq)]
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
    pub net: u32,
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

/// Represents a track segment used to fill the zone.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    /// The start point of the segment.
    pub start: Pt,
    /// The end point of the segment.
    pub end: Pt,
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
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

/// Attributes of a `Zone` in the schema
///
/// Represents a zone on the board or footprint, serving as filled copper zones or keep out areas.
#[derive(Debug, Clone, PartialEq)]
pub struct Zone {
    /// The net ordinal number the zone is part of.
    pub net: u32,
    /// The name of the net if the zone is not a keep out area.
    pub net_name: String,
    /// The canonical layer the zone resides on.
    pub layer: Layer,
    /// The unique identifier of the zone object.
    pub tstamp: String,
    /// The name of the zone if one has been assigned.
    pub name: Option<String>,
    /// The zone outline display hatch style and pitch.
    pub hatch: Hatch,
    /// The zone priority if it is not zero.
    pub priority: Option<u8>,
    /// The pad connection type and clearance.
    pub connect_pads: ConnectPads,
    /// The minimum fill width allowed in the zone.
    pub min_thickness: f32,
    /// Specifies if the zone line width is not used when determining the zone fill area.
    pub filled_areas_thickness: Option<bool>,
    /// The keep out items if the zone is defined as a keep out area.
    pub keepout_settings: Option<ZoneKeepoutSettings>,
    /// The zone fill settings.
    pub fill_settings: ZoneFillSettings,
    /// The coordinate point list of the polygon outline.
    pub polygon: Vec<Polyline>,
    /// All of the polygons used to fill the zone.
    pub fill_polygons: Option<Polygon>,
    /// A list of track segments used to fill the zone.
    pub fill_segments: Option<Vec<Segment>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Polygon {
    /// The list of coordinate points defining the polygon.
    pub points: Vec<Polyline>,
}

/// Represents the hatch style and pitch of the zone outline.
#[derive(Debug, Clone, PartialEq)]
pub struct Hatch {
    /// The hatch style.
    pub style: HatchStyle,
    /// The hatch pitch.
    pub pitch: f32,
}

/// Defines the hatch style of the zone outline.
#[derive(Debug, Clone, PartialEq)]
pub enum HatchStyle {
    /// No hatch.
    None,
    /// Edge hatch.
    Edge,
    /// Full hatch.
    Full,
}

/// Defines the pad connection type and clearance.
#[derive(Debug, Clone, PartialEq)]
pub struct ConnectPads {
    /// The pad connection type.
    pub connection_type: PadConnectionType,
    /// The pad clearance.
    pub clearance: f32,
}

/// Defines the pad connection type.
#[derive(Debug, Clone, PartialEq)]
pub enum PadConnectionType {
    /// Pads are not connected to the zone.
    No,
    /// Pads are connected to the zone using thermal relief.
    ThermalRelief,
    /// Pads are connected to the zone using solid fill.
    SolidFill,
    /// Only through hole pads are connected using thermal relief, surface mount pads are connected using solid fill.
    ThruHoleOnly,
}

/// Represents the keep out settings of the zone.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneKeepoutSettings {
    /// The type of keep out.
    pub keepout_type: KeepoutType,
    /// Additional properties of the keep out.
    pub properties: Vec<Property>,
}

/// Defines the keep out type.
#[derive(Debug, Clone, PartialEq)]
pub enum KeepoutType {
    /// Keep out for all copper.
    AllCopper,
    /// Keep out for vias only.
    ViasOnly,
    /// Keep out for tracks only.
    TracksOnly,
}

/// Represents the zone fill settings.
#[derive(Debug, Clone, PartialEq)]
pub struct ZoneFillSettings {
    /// The fill type.
    pub fill_type: FillType,
    /// Additional properties of the fill.
    pub properties: Vec<Property>,
}

/// Defines the fill type of the zone.
#[derive(Debug, Clone, PartialEq)]
pub enum FillType {
    /// Solid fill.
    Solid,
    /// Hatch fill.
    Hatch,
}
