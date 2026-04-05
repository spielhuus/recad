use std::{collections::HashMap, path::Path};

use types::{
    constants::el, error::RecadError, gr::{
        Effects, PaperSize, Pos, Pt,
        Pts, Stroke, TitleBlock,
    }
};

use sexp::{Sexp, SexpExt, SexpTree, SexpValue, SexpValueExt};

///Pcb file format for all versions of KiCad from 6.0.
#[derive(Default, Debug)]
pub struct Pcb {
    ///The version token attribute defines the pcb version
    ///using the YYYYMMDD date format.
    pub version: String,
    ///The UNIQUE_IDENTIFIER defines the universally unique identifier for
    ///the pcb file.
    pub uuid: String,
    ///The generator token attribute defines the program used to write the file.
    pub generator: String,
    ///The generator_version token attribute defines the program version
    ///used to write the file.
    pub generator_version: Option<String>,
    pub paper: PaperSize,
    //
    //General
    //The TitleBlock of the PCB
    pub title_block: TitleBlock,
    //The PCB setup.
    pub setup: Option<Setup>,
    //Graphical Lines
    pub gr_lines: Vec<GrLine>,
    pub vias: Vec<Via>,
    pub gr_texts: Vec<GrText>,
    //Layers
    pub layers: Vec<Layer>,

    //Setup
    //
    //Properties
    ///The ```net``` token defines a net for the board. This section is
    ///required. <br><br>
    pub nets: Vec<Net>,
    //
    ///The footprints on the pcb.
    pub footprints: Vec<Footprint>,
    //
    //Graphic Items
    //
    //Images
    pub segments: Vec<Segment>,
    //Zones
    pub zones: Vec<Zone>,
    //
    //Groups
}

impl Pcb {
    ///Load a pcb from a path
    pub fn load(path: &Path) -> Result<Self, RecadError> {
        let parser = sexp::parser::SexpParser::load(path)?;
        let tree = sexp::SexpTree::from(parser.iter())?;
        let err: Result<Self, RecadError> = tree.try_into();
        match err {
            Ok(res) => Ok(res),
            Err(err) => Err(parser.get_error(err)),
        }
    }
    // pub fn drc(&self, schema: &Schema) -> Vec<DRCViolation> {
    //     let checker = Drc::new(self, schema);
    //     checker.run()
    // }
}


impl<'a> std::convert::TryFrom<SexpTree<'a>> for Pcb {
    type Error = RecadError;
    fn try_from(sexp: SexpTree) -> Result<Self, Self::Error> {
        let mut pcb = Pcb::default();
        for node in sexp.root().nodes() {
            match node.name.as_ref() {
                el::VERSION => pcb.version = node.require_get(0)?,
                el::GENERATOR => pcb.generator = node.require_get(0)?,
                el::GENERATOR_VERSION => pcb.generator_version = node.require_get(0).ok(),
                el::UUID => pcb.uuid = node.require_get(0)?,
                el::TITLE_BLOCK => pcb.title_block = TitleBlock::try_from(node)?,
                el::LAYERS => {
                    pcb.layers = node
                        .nodes()
                        .map(Layer::try_from)
                        .collect::<Result<Vec<Layer>, Self::Error>>()?
                }
                el::SETUP => pcb.setup = node.try_into().ok(),
                el::GR_LINE => pcb.gr_lines.push(node.try_into()?),
                el::VIA => pcb.vias.push(node.try_into()?),
                el::GR_TEXT => pcb.gr_texts.push(node.try_into()?),
                el::ZONE => pcb.zones.push(node.try_into()?),
                el::SEGMENT => pcb.segments.push(node.try_into()?),
                el::NET => pcb.nets.push(node.try_into()?),
                el::FOOTPRINT => pcb.footprints.push(node.try_into()?),
                el::PAPER => {
                    let paper: String = node.require_get(0)?;
                    pcb.paper = PaperSize::from(paper.as_str());
                }
                _ => {
                    spdlog::warn!("unknown pcb node: {:?}", node)
                }
            }
        }
        Ok(pcb)
    }
}


/// Global PCB setup configuration
#[derive(Debug, Clone, Default)]
pub struct Setup {
    pub pad_to_mask_clearance: Option<f64>,
    pub solder_mask_min_width: Option<f64>,
    pub allow_soldermask_bridges_in_footprints: bool,
    pub pcbplotparams: Option<PcbPlotParams>,
}


impl<'a> std::convert::TryFrom<&Sexp<'a>> for Setup {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            pad_to_mask_clearance: sexp.first("pad_to_mask_clearance")?,
            solder_mask_min_width: sexp.first("solder_mask_min_width")?,
            allow_soldermask_bridges_in_footprints: sexp
                .first("allow_soldermask_bridges_in_footprints")?
                .unwrap_or(false),
            pcbplotparams: sexp
                .query("pcbplotparams")
                .next()
                .map(|n| n.try_into())
                .transpose()?,
        })
    }
}

/// Parameters related to PCB plotting and Gerber generation
#[derive(Debug, Clone, Default)]
pub struct PcbPlotParams {
    pub layerselection: Option<String>,
    pub plot_on_all_layers_selection: Option<String>,
    pub disableapertmacros: bool,
    pub usegerberextensions: bool,
    pub usegerberattributes: bool,
    pub usegerberadvancedattributes: bool,
    pub creategerberjobfile: bool,
    pub dashed_line_dash_ratio: Option<f64>,
    pub dashed_line_gap_ratio: Option<f64>,
    pub svgprecision: Option<u32>,
    pub plotframeref: bool,
    pub viasonmask: bool,
    pub mode: Option<u32>,
    pub useauxorigin: bool,
    pub hpglpennumber: Option<u32>,
    pub hpglpenspeed: Option<u32>,
    pub hpglpendiameter: Option<f64>,
    pub pdf_front_fp_property_popups: bool,
    pub pdf_back_fp_property_popups: bool,
    pub dxfpolygonmode: bool,
    pub dxfimperialunits: bool,
    pub dxfusepcbnewfont: bool,
    pub psnegative: bool,
    pub psa4output: bool,
    pub plotreference: bool,
    pub plotvalue: bool,
    pub plotfptext: bool,
    pub plotinvisibletext: bool,
    pub sketchpadsonfab: bool,
    pub subtractmaskfromsilk: bool,
    pub outputformat: Option<u32>,
    pub mirror: bool,
    pub drillshape: Option<u32>,
    pub scaleselection: Option<u32>,
    pub outputdirectory: Option<String>,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for PcbPlotParams {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            layerselection: sexp.first("layerselection")?,
            plot_on_all_layers_selection: sexp.first("plot_on_all_layers_selection")?,
            disableapertmacros: sexp.first("disableapertmacros")?.unwrap_or_default(),
            usegerberextensions: sexp.first("usegerberextensions")?.unwrap_or_default(),
            usegerberattributes: sexp.first("usegerberattributes")?.unwrap_or_default(),
            usegerberadvancedattributes: sexp
                .first("usegerberadvancedattributes")?
                .unwrap_or_default(),
            creategerberjobfile: sexp.first("creategerberjobfile")?.unwrap_or_default(),
            dashed_line_dash_ratio: sexp.first("dashed_line_dash_ratio")?,
            dashed_line_gap_ratio: sexp.first("dashed_line_gap_ratio")?,
            svgprecision: sexp.first("svgprecision")?,
            plotframeref: sexp.first("plotframeref")?.unwrap_or_default(),
            viasonmask: sexp.first("viasonmask")?.unwrap_or_default(),
            mode: sexp.first("mode")?,
            useauxorigin: sexp.first("useauxorigin")?.unwrap_or_default(),
            hpglpennumber: sexp.first("hpglpennumber")?,
            hpglpenspeed: sexp.first("hpglpenspeed")?,
            hpglpendiameter: sexp.first("hpglpendiameter")?,
            pdf_front_fp_property_popups: sexp
                .first("pdf_front_fp_property_popups")?
                .unwrap_or_default(),
            pdf_back_fp_property_popups: sexp
                .first("pdf_back_fp_property_popups")?
                .unwrap_or_default(),
            dxfpolygonmode: sexp.first("dxfpolygonmode")?.unwrap_or_default(),
            dxfimperialunits: sexp.first("dxfimperialunits")?.unwrap_or_default(),
            dxfusepcbnewfont: sexp.first("dxfusepcbnewfont")?.unwrap_or_default(),
            psnegative: sexp.first("psnegative")?.unwrap_or_default(),
            psa4output: sexp.first("psa4output")?.unwrap_or_default(),
            plotreference: sexp.first("plotreference")?.unwrap_or_default(),
            plotvalue: sexp.first("plotvalue")?.unwrap_or_default(),
            plotfptext: sexp.first("plotfptext")?.unwrap_or_default(),
            plotinvisibletext: sexp.first("plotinvisibletext")?.unwrap_or_default(),
            sketchpadsonfab: sexp.first("sketchpadsonfab")?.unwrap_or_default(),
            subtractmaskfromsilk: sexp.first("subtractmaskfromsilk")?.unwrap_or_default(),
            outputformat: sexp.first("outputformat")?,
            mirror: sexp.first("mirror")?.unwrap_or_default(),
            drillshape: sexp.first("drillshape")?,
            scaleselection: sexp.first("scaleselection")?,
            outputdirectory: sexp.first("outputdirectory")?,
        })
    }
}

///Definition of the layer type
#[derive(Debug)]
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
    pub user_name: Option<String>,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Layer {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            ordinal: sexp.name.parse::<u32>().map_err(|_| RecadError::Sexp {
                line: sexp.line,
                col: sexp.column,
                msg: format!("Invalid layer ordinal: '{}'", sexp.name),
            })?,
            canonical_name: sexp.require_get(0)?,
            layer_type: {
                let name: String = sexp.require_get(1)?;
                name.try_into()?
            },
            user_name: sexp.get(2)?,
        })
    }
}

impl std::convert::TryFrom<String> for LayerType {
    type Error = RecadError;
    fn try_from(name: String) -> Result<Self, Self::Error> {
        match name.as_str() {
            "jumper" => Ok(LayerType::Jumper),
            "mixed" => Ok(LayerType::Mixed),
            "power" => Ok(LayerType::Power),
            "signal" => Ok(LayerType::Signal),
            "user" => Ok(LayerType::User),
            _ => Err(RecadError::Pcb(format!("Invalid layer type: {}", name))),
        }
    }
}

//create a layer type enum
#[derive(Debug)]
pub enum LayerType {
    Jumper,
    Mixed,
    Power,
    Signal,
    User,
}

/// Defines a graphic line on the board (outside of footprints).
#[derive(Debug, Clone)]
pub struct GrLine {
    /// Coordinates of the beginning of the line.
    pub start: Pt,
    /// Coordinates of the end of the line.
    pub end: Pt,
    /// The canonical layer the line resides on.
    pub layer: String,
    /// The style and width of the line.
    pub stroke: Stroke,
    /// The universally unique identifier of the line.
    pub uuid: Option<String>,
    /// The legacy unique identifier of the line (often found in older files).
    pub tstamp: Option<String>,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for GrLine {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            start: sexp.require_node(el::START)?.try_into()?,
            end: sexp.require_node(el::END)?.try_into()?,
            layer: sexp.require_first(el::LAYER)?,
            stroke: sexp.try_into()?,
            uuid: sexp.first(el::UUID)?,
            tstamp: sexp.first(el::TSTAMP)?,
        })
    }
}

#[derive(Debug)]
pub enum ViaType {
    Blind,
    Micro,
}

/// Defines a track segment in a PCB design.
#[derive(Debug)]
pub struct Segment {
    /// Coordinates of the beginning of the line.
    pub start: Pt,

    /// Coordinates of the end of the line.
    pub end: Pt,

    /// Line width.
    pub width: f64,

    /// The canonical layer the track segment resides on.
    pub layer: String,

    /// Indicates if the line cannot be edited.
    pub locked: bool,

    /// The net number that the segment belongs to.
    pub net: u32,

    /// A unique identifier for the line object.
    pub tstamp: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Segment {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            start: sexp.require_node(el::START)?.try_into()?,
            end: sexp.require_node(el::END)?.try_into()?,
            width: sexp.require_first(el::WIDTH)?,
            layer: sexp.require_first(el::LAYER)?,
            locked: sexp.first("locked")?.unwrap_or(false),
            net: sexp.require_first("net")?,
            tstamp: match sexp.first(el::TSTAMP)? {
                Some(ts) => ts,
                None => sexp.first(el::UUID)?.unwrap_or_default(),
            },
        })
    }
}

#[derive(Debug)]
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

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Via {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let via_type = match sexp.value_iter().next() {
            Some("blind") => Some(ViaType::Blind),
            Some("micro") => Some(ViaType::Micro),
            _ => None,
        };

        let layers_node = sexp.require_node("layers")?;

        Ok(Self {
            via_type,
            locked: sexp.first("locked")?.unwrap_or(false),
            pos: Pos::try_from(sexp)?,
            size: sexp.first("size")?.unwrap_or(0.0),
            drill: sexp.first("drill")?.unwrap_or(0.0),
            layers: (layers_node.require_get(0)?, layers_node.require_get(1)?),
            remove_unused_layers: sexp.first("remove_unused_layers")?.unwrap_or(false),
            keep_end_layers: sexp.first("keep_end_layers")?.unwrap_or(false),
            free: sexp.value_iter().any(|v| v == "free"),
            net: sexp.first("net")?.unwrap_or(0),
            // Map legacy 'tstamp' or modern 'uuid' to the same field
            tstamp: match sexp.first(el::TSTAMP)? {
                Some(ts) => ts,
                None => sexp.first(el::UUID)?.unwrap_or_default(),
            },
        })
    }
}

///The ```net``` token defines a net for the board. This section is required.
#[derive(Debug)]
pub struct Net {
    ///The ordinal attribute is an integer that defines the net order.
    pub ordinal: u32,
    ///The net name is a string that defines the name of the net.
    pub name: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Net {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            ordinal: sexp.require_get(0)?,
            name: sexp.require_get(1)?,
        })
    }
}

///defines a footprint type
#[derive(Debug)]
pub enum FootprintType {
    Smd,
    ThroughHole,
    ExcludeFromPosFiles,
}

impl TryFrom<String> for FootprintType {
    type Error = RecadError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.as_str() {
            "smd" => Ok(FootprintType::Smd),
            "through_hole" => Ok(FootprintType::ThroughHole),
            "exclude_from_pos_files" => Ok(FootprintType::ExcludeFromPosFiles),
            _ => Err(RecadError::Pcb(format!("Invalid footprint type: {}", s))),
        }
    }
}

/// Defines a footprint in a PCB design.
#[derive(Debug)]
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
    pub solder_mask_margin: Option<f64>,

    /// The solder paste distance from all pads in the footprint. If not set, the board solder_paste_margin setting is used.
    pub solder_paste_margin: Option<f64>,

    /// The percentage of the pad size used to define the solder paste for all pads in the footprint. If not set, the board solder_paste_ratio setting is used.
    pub solder_paste_ratio: Option<f64>,

    /// The clearance to all board copper objects for all pads in the footprint. If not set, the board clearance setting is used.
    pub clearance: Option<f64>,

    /// How all pads are connected to filled zones. Valid values are 0 to 3.
    /// 0: Pads are not connected to the zone.
    /// 1: Pads are connected to the zone using thermal reliefs.
    /// 2: Pads are connected to the zone using solid fill.
    pub zone_connect: Option<u8>,

    /// The thermal relief spoke width used for zone connections for all pads in the
    /// footprint. Only affects pads connected to zones with thermal reliefs.
    /// If not set, the zone thermal_width setting is used.
    pub thermal_width: Option<f64>,

    /// The distance from the pad to the zone of thermal relief connections for all
    /// pads in the footprint. If not set, the zone thermal_gap setting is used.
    pub thermal_gap: Option<f64>,

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

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Footprint {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            library_link: sexp.get(0)?.unwrap_or_default(),
            locked: sexp.first("locked")?.unwrap_or(false),
            placed: sexp.first("placed")?.unwrap_or(false),
            layer: sexp.first(el::LAYER)?.unwrap_or("F.Cu".to_string()),
            tedit: sexp.first("tedit")?,
            tstamp: sexp.first(el::TSTAMP)?,
            pos: Pos::try_from(sexp)?,
            descr: sexp.first(el::DESC)?,
            tags: sexp.first(el::TAGS)?,
            property: sexp.query(el::PROPERTY).try_fold(
                HashMap::new(),
                |mut m, s| -> Result<_, RecadError> {
                    if let (Some(k), Some(v)) = (s.get(0)?, s.get(1)?) {
                        m.insert(k, v);
                    }
                    Ok(m)
                },
            )?,
            path: sexp.first("path")?,
            autoplace_cost90: sexp.first("autoplace_cost90")?,
            autoplace_cost180: sexp.first("autoplace_cost180")?,
            solder_mask_margin: sexp.first("solder_mask_margin")?,
            solder_paste_margin: sexp.first("solder_paste_margin")?,
            solder_paste_ratio: sexp.first("solder_paste_ratio")?,
            clearance: sexp.first("clearance")?,
            zone_connect: sexp.first("zone_connect")?,
            thermal_width: sexp.first("thermal_width")?,
            thermal_gap: sexp.first("thermal_gap")?,
            footprint_type: FootprintType::try_from(sexp.require_first::<String>("attr")?)?,
            board_only: sexp
                .query("attr")
                .next()
                .map(|attr| attr.value_iter().any(|v| v == "board_only"))
                .unwrap_or(false),
            exclude_from_pos_files: sexp
                .query("attr")
                .next()
                .map(|attr| attr.value_iter().any(|v| v == "exclude_from_pos_files"))
                .unwrap_or(false),
            exclude_from_bom: sexp
                .query("attr")
                .next()
                .map(|attr| attr.value_iter().any(|v| v == "exclude_from_bom"))
                .unwrap_or(false),
            private_layers: None,
            net_tie_pad_groups: None,
            graphic_items: GraphicItem::parse_many(sexp)?,
            pads: sexp
                .query(el::PAD)
                .map(Pad::try_from)
                .collect::<Result<Vec<Pad>, Self::Error>>()?,
            zones: sexp
                .query("zone")
                .map(|z| -> Result<String, RecadError> { z.require_get(0) })
                .collect::<Result<Vec<String>, Self::Error>>()?,
            groups: sexp
                .query("group")
                .map(|g| -> Result<String, RecadError> { g.require_get(0) })
                .collect::<Result<Vec<String>, Self::Error>>()?,
            model_3d: sexp.first("model")?,
        })
    }
}

/// Defines text in a footprint definition.
#[derive(Debug)]
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
    pub effects: Effects,
    /// The unique identifier of the text object.
    pub tstamp: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for FpText {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            text_type: sexp.require_get(0)?,
            text: sexp.require_get(1)?,
            pos: Pos::try_from(sexp)?,
            unlocked: sexp.first(el::UNLOCKED)?.unwrap_or(false),
            layer: sexp.first(el::LAYER)?.unwrap_or_default(),
            hide: sexp.first(el::HIDE)?.unwrap_or(false),
            effects: sexp.try_into()?,
            tstamp: match sexp.first(el::TSTAMP)? {
                Some(ts) => ts,
                None => sexp.first(el::UUID)?.unwrap_or_default(),
            },
        })
    }
}

/// Defines a rectangle containing line-wrapped text in a footprint.
#[derive(Debug)]
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
#[derive(Debug)]
pub struct FpLine {
    /// The coordinates of the beginning of the line.
    pub start: Pt,
    /// The coordinates of the end of the line.
    pub end: Pt,
    /// The canonical layer the line resides on.
    pub layer: String,
    //TODO The line width.
    //pub width: f64,
    /// The style of the line.
    pub stroke: Stroke,
    /// Indicates if the line cannot be edited.
    pub locked: bool,
    /// The unique identifier of the line object.
    pub tstamp: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for FpLine {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            start: sexp.require_node(el::START)?.try_into()?,
            end: sexp.require_node(el::END)?.try_into()?,
            layer: sexp.require_first(el::LAYER)?,

            stroke: sexp.try_into()?,
            locked: sexp.first("locked")?.unwrap_or(false),
            tstamp: sexp.first(el::TSTAMP)?.unwrap_or_else(|| "now".to_string()),
        })
    }
}

/// Defines a graphic rectangle in a footprint.
#[derive(Debug)]
pub struct FpRect {
    /// The coordinates of the upper left corner of the rectangle.
    pub start: Pt,
    /// The coordinates of the lower right corner of the rectangle.
    pub end: Pt,
    /// The canonical layer the rectangle resides on.
    pub layer: String,
    /// The line width of the rectangle.
    pub width: f64,
    /// The style of the rectangle.
    pub stroke_definition: Option<String>,
    /// Defines how the rectangle is filled. Valid types are solid and none.
    pub fill: Option<String>,
    /// Indicates if the rectangle cannot be edited.
    pub locked: bool,
    /// The unique identifier of the rectangle object.
    pub tstamp: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for FpRect {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        // Handle width: legacy (width X) or new (stroke (width X))
        let width = if let Some(w) = sexp.first(el::WIDTH)? {
            w
        } else {
            let s: Stroke = sexp.try_into()?;
            s.width
        };

        // Handle stroke definition as string (type)
        let stroke_definition = if let Some(stroke) = sexp.query(el::STROKE).next() {
            stroke.first(el::TYPE)?
        } else {
            None
        };

        Ok(Self {
            start: sexp.require_node(el::START)?.try_into()?,
            end: sexp.require_node(el::END)?.try_into()?,
            layer: sexp.require_first(el::LAYER)?,
            width,
            stroke_definition,
            fill: sexp
                .query(el::FILL)
                .next()
                .map(|f| f.get(0))
                .transpose()?
                .flatten(),
            locked: sexp.first("locked")?.unwrap_or(false),
            tstamp: sexp.first(el::TSTAMP)?.unwrap_or("".to_string()),
        })
    }
}

/// Defines a graphic circle in a footprint.
#[derive(Debug)]
pub struct FpCircle {
    /// The coordinates of the center of the circle.
    pub center: Pt,
    /// The coordinates of the end of the radius of the circle.
    pub end: Pt,
    /// The canonical layer the circle resides on.
    pub layer: String,
    /// The line width of the circle.
    pub width: f64,
    /// The style of the circle.
    pub stroke_definition: Option<String>,
    /// Defines how the circle is filled. Valid types are solid and none.
    pub fill: Option<String>,
    /// Indicates if the circle cannot be edited.
    pub locked: bool,
    /// The unique identifier of the circle object.
    pub tstamp: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for FpCircle {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let width = if let Ok(Some(w)) = sexp.first(el::WIDTH) {
            w
        } else {
            let s: Stroke = sexp.try_into()?;
            s.width
        };
        let stroke_definition = if let Some(stroke) = sexp.query(el::STROKE).next() {
            stroke.first(el::TYPE)?
        } else {
            None
        };

        Ok(Self {
            center: sexp.require_node(el::CENTER)?.try_into()?,
            end: sexp.require_node(el::END)?.try_into()?,
            layer: sexp.require_first(el::LAYER)?,
            width,
            stroke_definition,
            fill: sexp
                .query(el::FILL)
                .next()
                .map(|f| f.get(0))
                .transpose()?
                .flatten(),
            locked: sexp.first("locked")?.unwrap_or(false),
            tstamp: sexp.first(el::TSTAMP)?.unwrap_or("".to_string()),
        })
    }
}

/// Defines a graphic arc in a footprint.
#[derive(Debug)]
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
    pub width: f64,
    /// The style of the arc.
    pub stroke_definition: Option<String>,
    /// Indicates if the arc cannot be edited.
    pub locked: bool,
    /// The unique identifier of the arc object.
    pub tstamp: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for FpArc {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let width = if let Some(w) = sexp.first(el::WIDTH)? {
            w
        } else {
            let s: Stroke = sexp.try_into()?;
            s.width
        };
        let stroke_definition = if let Some(stroke) = sexp.query(el::STROKE).next() {
            stroke.first(el::TYPE)?
        } else {
            None
        };

        Ok(Self {
            start: sexp.require_node(el::START)?.try_into()?,
            mid: sexp.require_node(el::MID)?.try_into()?,
            end: sexp.require_node(el::END)?.try_into()?,
            layer: sexp.require_first(el::LAYER)?,
            width,
            stroke_definition,
            locked: sexp.first("locked")?.unwrap_or(false),
            tstamp: sexp.first(el::TSTAMP)?.unwrap_or("".to_string()),
        })
    }
}

/// Defines a graphic polygon in a footprint.
#[derive(Debug)]
pub struct FpPoly {
    /// The list of X/Y coordinates of the polygon outline.
    pub pts: Pts,
    /// The canonical layer the polygon resides on.
    pub layer: String,
    /// The line width of the polygon.
    pub width: f64,
    /// The style of the polygon.
    pub stroke_definition: Option<String>,
    /// Defines how the polygon is filled. Valid types are solid and none.
    pub fill: Option<String>,
    /// Indicates if the polygon cannot be edited.
    pub locked: bool,
    /// The unique identifier of the polygon object.
    pub tstamp: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for FpPoly {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let width = if let Some(w) = sexp.first(el::WIDTH)? {
            w
        } else {
            let s: Stroke = sexp.try_into()?;
            s.width
        };
        let stroke_definition = if let Some(stroke) = sexp.query(el::STROKE).next() {
            stroke.first(el::TYPE)?
        } else {
            None
        };

        Ok(Self {
            pts: sexp.try_into()?,
            layer: sexp.require_first(el::LAYER)?,
            width,
            stroke_definition,
            fill: sexp
                .query(el::FILL)
                .next()
                .map(|f| f.get(0))
                .transpose()?
                .flatten(),
            locked: sexp.first("locked")?.unwrap_or(false),
            tstamp: sexp.first(el::TSTAMP)?.unwrap_or("".to_string()),
        })
    }
}

/// Defines a property in a footprint definition.
/// Properties can be graphical (like Reference/Value text) or non-graphical metadata.
#[derive(Debug)]
pub struct FpProperty {
    /// The property key (e.g., "Reference", "Value", "ki_fp_filters")
    pub name: String,
    /// The property value (e.g., "R1", "10k", "LED*")
    pub value: String,
    /// The position identifier with X, Y coordinates and optional orientation angle.
    pub pos: Option<Pos>,
    /// Indicates if the text orientation can be anything other than the upright orientation.
    pub unlocked: bool,
    /// The canonical layer the text resides on.
    pub layer: Option<String>,
    /// Indicates if the text is hidden.
    pub hide: bool,
    /// Defines how the text is displayed.
    pub effects: Option<Effects>,
    /// The unique identifier of the property.
    pub tstamp: Option<String>,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for FpProperty {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let name = sexp.require_get(0)?;
        let value = sexp.require_get(1)?;
        
        // Determine if it has graphical attributes by looking for the 'at' node
        let is_graphical = sexp.query(el::AT).next().is_some();
        
        Ok(Self {
            name,
            value,
            pos: if is_graphical { Some(Pos::try_from(sexp)?) } else { None },
            unlocked: sexp.first(el::UNLOCKED)?.unwrap_or(false) 
                || sexp.value_iter().any(|v| v == el::UNLOCKED),
            layer: sexp.first(el::LAYER)?,
            hide: sexp.first(el::HIDE)?.unwrap_or(false) 
                || sexp.value_iter().any(|v| v == el::HIDE),
            effects: if is_graphical { sexp.try_into().ok() } else { None },
            tstamp: match sexp.first(el::TSTAMP)? {
                Some(ts) => Some(ts),
                None => sexp.first(el::UUID)?,
            },
        })
    }
}

/// Defines a graphic Cubic Bezier curve in a footprint.
#[derive(Debug)]
pub struct FpCurve {
    /// The four X/Y coordinates of each point of the curve.
    pub pts: Pts,
    /// The canonical layer the curve resides on.
    pub layer: String,
    /// The line width of the curve.
    pub width: f64,
    /// The style of the curve.
    pub stroke_definition: Option<String>,
    /// Indicates if the curve cannot be edited.
    pub locked: bool,
    /// The unique identifier of the curve object.
    pub tstamp: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for FpCurve {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let width = if let Some(w) = sexp.first(el::WIDTH)? {
            w
        } else {
            let s: Stroke = sexp.try_into()?;
            s.width
        };
        let stroke_definition = if let Some(stroke) = sexp.query(el::STROKE).next() {
            stroke.first(el::TYPE)?
        } else {
            None
        };

        Ok(Self {
            pts: sexp.try_into()?,
            layer: sexp.require_first(el::LAYER)?,
            width,
            stroke_definition,
            locked: sexp.first("locked")?.unwrap_or(false),
            tstamp: sexp.first(el::TSTAMP)?.unwrap_or("".to_string()),
        })
    }
}

/// Enum for different graphic items
#[derive(Debug)]
pub enum GraphicItem {
    FpLine(FpLine),
    FpRect(FpRect),
    FpArc(FpArc),
    FpCircle(FpCircle),
    FpCurve(FpCurve),
    FpPoly(FpPoly),
    FpText(FpText),
    FpProperty(FpProperty),
    AnnotationBoundingBox,
}

impl GraphicItem {
    pub fn layer(&self) -> Option<&str> {
        match self {
            GraphicItem::FpLine(item) => Some(&item.layer),
            GraphicItem::FpRect(item) => Some(&item.layer),
            GraphicItem::FpArc(item) => Some(&item.layer),
            GraphicItem::FpCircle(item) => Some(&item.layer),
            GraphicItem::FpCurve(item) => Some(&item.layer),
            GraphicItem::FpPoly(item) => Some(&item.layer),
            GraphicItem::FpText(item) => Some(&item.layer),
            GraphicItem::FpProperty(item) => item.layer.as_deref(),
            GraphicItem::AnnotationBoundingBox => None,
        }
    }

    pub fn has_layer(&self, layer: &[String]) -> bool {
        if layer.is_empty() {
            true
        } else {
            match self {
                GraphicItem::FpLine(item) => layer.contains(&item.layer),
                GraphicItem::FpRect(item) => layer.contains(&item.layer),
                GraphicItem::FpArc(item) => layer.contains(&item.layer),
                GraphicItem::FpCircle(item) => layer.contains(&item.layer),
                GraphicItem::FpCurve(item) => layer.contains(&item.layer),
                GraphicItem::FpPoly(item) => layer.contains(&item.layer),
                GraphicItem::FpText(item) => layer.contains(&item.layer),
                GraphicItem::FpProperty(item) => {
                    item.layer.as_ref().is_some_and(|l| layer.contains(l))
                }
                GraphicItem::AnnotationBoundingBox => false,
            }
        }
    }

    /// Parses a list of graphic items from a parent Sexp node
    /// TODO: cant the Items be collected like the pads?
    pub fn parse_many(sexp: &Sexp) -> Result<Vec<GraphicItem>, RecadError> {
        let mut res = Vec::new();
        for n in sexp.nodes() {
            match n.name.as_ref() {
                el::FP_LINE => res.push(GraphicItem::FpLine(n.try_into()?)),
                "fp_rect" => res.push(GraphicItem::FpRect(n.try_into()?)),
                "fp_circle" => res.push(GraphicItem::FpCircle(n.try_into()?)),
                "fp_arc" => res.push(GraphicItem::FpArc(n.try_into()?)),
                "fp_poly" => res.push(GraphicItem::FpPoly(n.try_into()?)),
                "fp_curve" => res.push(GraphicItem::FpCurve(n.try_into()?)),
                "fp_text" => res.push(GraphicItem::FpText(n.try_into()?)),
                "property" => res.push(GraphicItem::FpProperty(n.try_into()?)),
                _ => {}
            }
        }
        Ok(res)
    }
}

// impl<'a> std::convert::TryFrom<&Sexp<'a>> for Vec<GraphicItem> {
//     type Error = RecadError;
//     fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
//         let mut res = Vec::new();
//         for n in sexp.nodes() {
//             match n.name.as_ref() {
//                 el::FP_LINE => res.push(GraphicItem::FpLine(n.try_into()?)),
//                 "fp_rect" => res.push(GraphicItem::FpRect(n.try_into()?)),
//                 "fp_circle" => res.push(GraphicItem::FpCircle(n.try_into()?)),
//                 "fp_arc" => res.push(GraphicItem::FpArc(n.try_into()?)),
//                 "fp_poly" => res.push(GraphicItem::FpPoly(n.try_into()?)),
//                 "fp_curve" => res.push(GraphicItem::FpCurve(n.try_into()?)),
//                 "fp_text" => res.push(GraphicItem::FpText(n.try_into()?)),
//                 "property" => res.push(GraphicItem::FpProperty(n.try_into()?)),
//                 _ => {}
//             }
//         }
//         Ok(res)
//     }
// }

/// Struct for custom pad primitives
pub struct CustomPadPrimitives {
    /// List of graphical items defining the custom pad shape
    pub graphic_items: Vec<GraphicItem>,
    /// Line width of the graphical items
    pub width: f64,
    /// Optional: If the geometry defined by graphical items should be filled
    pub fill: Option<bool>,
}

#[derive(Debug)]
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

#[derive(Debug)]
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
    pub diameter: f64,
    /// Optional: Width of the slot for oval drills
    pub width: Option<f64>,
    /// Optional: X coordinate of drill offset
    pub offset_x: Option<f64>,
    /// Optional: Y coordinate of drill offset
    pub offset_y: Option<f64>,
}

/// Defines a graphic text on the board.
#[derive(Debug, Clone)]
pub struct GrText {
    /// The text string.
    pub text: String,
    /// Indicates if the text is locked.
    pub locked: bool,
    /// The position identifier with X, Y coordinates and optional orientation angle.
    pub pos: Pos,
    /// The canonical layer the text resides on.
    pub layer: String,
    /// Defines how the text is displayed.
    pub effects: Effects,
    /// The unique identifier of the text object.
    pub tstamp: Option<String>,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for GrText {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            text: sexp.require_get(0)?,
            locked: sexp.first("locked")?.unwrap_or(false),
            pos: Pos::try_from(sexp)?,
            layer: sexp.require_first(el::LAYER)?,
            effects: sexp.try_into()?,
            tstamp: match sexp.first(el::TSTAMP)? {
                Some(ts) => Some(ts),
                None => sexp.first(el::UUID)?,
            },
        })
    }
}

/// Main struct for a footprint pad
#[derive(Debug)]
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
    pub size: (f64, f64),
    /// Optional: Drill definition for the pad
    pub drill: Option<f64>,
    /// Layers the pad resides on
    pub layers: Vec<String>,
    //pub canonical_layer_list: String,
    /// Optional: Special properties for the pad
    //pub properties: Option<Vec<String>>,
    /// Optional: Remove copper from layers pad is not connected to
    //pub remove_unused_layer: Option<bool>,
    /// Optional: Retain top and bottom layers when removing copper
    //pub keep_end_layers: Option<bool>,
    /// Optional: Scaling factor of pad to corner radius for roundrect/chamfered pads (0 to 1)
    //pub roundrect_rratio: Option<f64>,
    /// Optional: Scaling factor of pad to chamfer size (0 to 1)
    //pub chamfer_ratio: Option<f64>,
    /// Optional: List of pad corners that get chamfered (top_left, top_right, bottom_left, bottom_right)
    //pub chamfer: Option<Vec<String>>,
    /// Integer number and name string of the net connection for the pad
    pub net: Net,
    /// Unique identifier of the pad object
    pub uuid: Option<String>,
    /// The legacy unique identifier of the line (often found in older files).
    pub tstamp: Option<String>,
    // Optional: Schematic symbol pin name
    //pub pinfunction: Option<String>,
    // Optional: Schematic pin electrical type
    //pub pintype: Option<String>,
    // Optional: Die length between the component pad and physical chip inside the package
    //pub die_length: Option<f64>,
    // Optional: Distance between the pad and the solder mask
    //pub solder_mask_margin: Option<f64>,
    // Optional: Distance the solder paste should be changed for the pad
    //pub solder_paste_margin: Option<f64>,
    // Optional: Percentage to reduce pad outline by to generate solder paste size
    //pub solder_paste_margin_ratio: Option<f64>,
    // Optional: Clearance from all copper to the pad
    //pub clearance: Option<f64>,
    // Optional: Type of zone connect for the pad (0 to 3)
    //pub zone_connect: Option<i32>,
    // Optional: Thermal relief spoke width for zone connection
    //pub thermal_width: Option<f64>,
    // Optional: Distance from the pad to the zone of the thermal relief connection
    //pub thermal_gap: Option<f64>,
    // Optional: Options for a custom pad
    //pub custom_pad_options: Option<CustomPadOptions>,
    // Optional: Drawing objects and options for defining a custom pad
    //pub custom_pad_primitives: Option<CustomPadPrimitives>,
}


impl<'a> std::convert::TryFrom<&Sexp<'a>> for Pad {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        // parse drill: (drill oval 1.0 2.0) vs (drill 0.8)
        let drill = if let Some(d) = sexp.query("drill").next() {
            if d.value_iter().next() == Some("oval") {
                // (drill oval W H) -> W is at index 1.
                d.get(1).unwrap_or(None)
            } else {
                // (drill D) -> D is at index 0.
                d.get(0).unwrap_or(None)
            }
        } else {
            None
        };

        // Safely parse net. Library footprints often have no net.
        let net = if let Some(net_node) = sexp.query(el::NET).next() {
            net_node.try_into()?
        } else {
            Net {
                ordinal: 0,
                name: "".to_string(),
            }
        };

        Ok(Self {
            number: sexp.get(0)?.unwrap_or_default(),
            pad_type: PadType::from(sexp.require_get::<String>(1)?),
            shape: PadShape::from(sexp.require_get::<String>(2)?),
            pos: Pos::try_from(sexp)?,
            //locked: SexpStringList::values(sexp).contains(&"locked".to_string()), // Field commented in struct
            size: (
                sexp.query(el::SIZE).next().unwrap().get(0)?.unwrap_or(0.0),
                sexp.query(el::SIZE).next().unwrap().get(1)?.unwrap_or(0.0),
            ),
            drill,
            layers: sexp
                .query("layers")
                .next()
                .map(|n| n.value_iter().map(|s| s.to_string()).collect())
                .unwrap_or_default(),
            //canonical_layer_list: // Field commented in struct
            //properties: // Field commented in struct
            //remove_unused_layer: // Field commented in struct
            //keep_end_layers: // Field commented in struct
            //roundrect_rratio: // Field commented in struct
            //chamfer_ratio: // Field commented in struct
            //chamfer: // Field commented in struct
            net,
            tstamp: sexp.first(el::TSTAMP)?,
            uuid: sexp.first(el::UUID)?,
            //pinfunction: // Field commented in struct
            //pintype: // Field commented in struct
            //die_length: // Field commented in struct
            //solder_mask_margin: // Field commented in struct
            //solder_paste_margin: // Field commented in struct
            //solder_paste_margin_ratio: // Field commented in struct
            //clearance: // Field commented in struct
            //zone_connect: // Field commented in struct
            //thermal_width: // Field commented in struct
            //thermal_gap: // Field commented in struct
            //custom_pad_options: // Field commented in struct
            //custom_pad_primitives: // Field commented in struct
        })
    }
}

/// Hatch settings for a zone
#[derive(Debug, Clone)]
pub struct ZoneHatch {
    pub style: String,
    pub pitch: f64,
}

/// Connect pads settings for a zone
#[derive(Debug, Clone)]
pub struct ZoneConnectPads {
    pub clearance: f64,
}

/// Fill settings for a zone
#[derive(Debug, Clone)]
pub struct ZoneFill {
    pub fill: bool,
    pub thermal_gap: Option<f64>,
    pub thermal_bridge_width: Option<f64>,
}

/// A filled polygon generated by a zone
#[derive(Debug, Clone)]
pub struct FilledPolygon {
    pub layer: Option<String>,
    pub pts: Pts,
}

/// Defines a copper or non-copper zone (pour) on the board.
#[derive(Debug, Clone)]
pub struct Zone {
    pub net: Option<u32>,
    pub net_name: Option<String>,
    pub layer: String,
    pub uuid: Option<String>,
    pub hatch: Option<ZoneHatch>,
    pub connect_pads: Option<ZoneConnectPads>,
    pub min_thickness: Option<f64>,
    pub filled_areas_thickness: bool,
    pub fill: Option<ZoneFill>,
    pub polygon: Pts,
    pub filled_polygons: Vec<FilledPolygon>,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Zone {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Self {
            net: sexp.first("net")?,
            net_name: sexp.first("net_name")?,
            layer: sexp.first(el::LAYER)?.unwrap_or_default(),
            uuid: sexp.first(el::UUID)?,
            hatch: sexp
                .query("hatch")
                .next()
                .map(|h| -> Result<ZoneHatch, RecadError> {
                    Ok(ZoneHatch {
                        style: h.get(0)?.unwrap_or_default(),
                        pitch: h.get(1)?.unwrap_or_default(),
                    })
                })
                .transpose()?,
            connect_pads: sexp
                .query("connect_pads")
                .next()
                .map(|cp| -> Result<ZoneConnectPads, RecadError> {
                    Ok(ZoneConnectPads {
                        clearance: cp.first("clearance")?.unwrap_or_default(),
                    })
                })
                .transpose()?,
            min_thickness: sexp.first("min_thickness")?,
            filled_areas_thickness: sexp.first("filled_areas_thickness")?.unwrap_or(false),
            fill: sexp
                .query(el::FILL)
                .next()
                .map(|f| -> Result<ZoneFill, RecadError> {
                    Ok(ZoneFill {
                        fill: f.value_iter().next() == Some(el::YES),
                        thermal_gap: f.first("thermal_gap")?,
                        thermal_bridge_width: f.first("thermal_bridge_width")?,
                    })
                })
                .transpose()?,
            polygon: sexp.require_node("polygon")?.try_into()?,
            filled_polygons: sexp
                .query("filled_polygon")
                .map(|fp| -> Result<FilledPolygon, RecadError> {
                    Ok(FilledPolygon {
                        layer: fp.first(el::LAYER)?,
                        pts: fp.try_into()?,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}
