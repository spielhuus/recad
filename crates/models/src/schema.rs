use crate::{geometry::{calculate, pin_position}, library::SymbolLibrary, symbols::LibrarySymbol, transform::Transform};
use sexp::{builder::Builder, Sexp, SexpExt, SexpTree, SexpValue, SexpValueExt, SexpWrite};
use std::{
    fmt,
    io::Write,
    path::{Path, PathBuf},
};
use types::{
    constants::el,
    error::RecadError,
    gr::{
        Arc, Circle, Color, Curve, Effects, FillType, GraphicItem, Line, PaperSize, Polyline, Pos, Pt, Pts, Rect, Rectangle, Stroke, TitleBlock
    },
    round, yes_or_no,
};

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
    pub hide: Option<bool>,
}

impl Property {
    ///Check if the property is visible
    pub fn visible(&self) -> bool {
        if let Some(hide) = self.hide {
            !hide
        } else {
            !self.effects.hide
        }
    }
}

impl SexpWrite for Property {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::PROPERTY);
        builder.text(&self.key);
        builder.text(&self.value);

        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(round(self.pos.angle));
        builder.end();

        self.effects.write(builder)?;

        builder.end();

        Ok(())
    }
}

///A `Text`in the schema
#[derive(Debug, Clone)]
pub struct Text {
    /// X and Y coordinates of the text.
    pub pos: Pos,
    /// The text to display.
    pub text: String,
    /// Text effects such as font, color, etc.
    pub effects: Effects,
    /// Whether the text is a simulation instruction (not supported in recad).
    pub exclude_from_sim: bool,
    /// Universally unique identifier for the text.
    pub uuid: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Text {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Text {
            text: sexp.require_get(0)?,
            pos: Pos::try_from(sexp)?,
            effects: sexp.try_into()?,
            exclude_from_sim: sexp.first(el::EXCLUDE_FROM_SIM)?.unwrap_or(false),
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for Text {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::TEXT);
        builder.text(&self.text);
        builder.push(el::EXCLUDE_FROM_SIM);
        builder.value(yes_or_no(self.exclude_from_sim));
        builder.end();
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(round(self.pos.angle));
        builder.end();
        self.effects.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

///A `TextBox`in the schema
#[derive(Debug, Clone)]
pub struct TextBox {
    /// X and Y coordinates of the text.
    pub pos: Pos,
    /// The text to display.
    pub text: String,
    /// The width of the text box.
    pub width: f64,
    /// The height of the text box.
    pub height: f64,
    /// Defines how the box is drawn.
    pub stroke: Stroke,
    /// Defines the fill style of the box.
    pub fill: FillType,
    /// Text effects such as font, color, etc.
    pub effects: Effects,
    /// Whether the text is a simulation instruction (not supported in recad).
    pub exclude_from_sim: bool,
    /// Universally unique identifier for the text.
    pub margins: Vec<f64>,
    pub uuid: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for TextBox {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let size = sexp.require_node(el::SIZE)?;
        Ok(TextBox {
            pos: Pos::try_from(sexp)?,
            text: sexp.require_get(0)?,
            width: size.require_get(0)?,
            height: size.require_get(1)?,
            stroke: sexp.try_into()?,
            fill: sexp.try_into()?,
            effects: sexp.try_into()?,
            exclude_from_sim: sexp.first(el::EXCLUDE_FROM_SIM)?.unwrap_or(false),
            margins: if let Some(margins_node) = sexp.query(el::MARGINS).next() {
                margins_node
                    .value_iter()
                    .filter_map(|v| v.parse::<f64>().ok())
                    .collect()
            } else {
                Vec::new()
            },
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for TextBox {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::TEXT_BOX);
        builder.text(&self.text);
        builder.push(el::EXCLUDE_FROM_SIM);
        builder.value(yes_or_no(self.exclude_from_sim));
        builder.end();
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(round(self.pos.angle));
        builder.end();
        builder.push(el::SIZE);
        builder.value(round(self.width));
        builder.value(round(self.height));
        builder.end();
        builder.push(el::MARGINS);
        for margin in &self.margins {
            builder.value(round(*margin));
        }
        builder.end();
        self.stroke.write(builder)?;
        self.fill.write(builder)?;
        self.effects.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

/// A junction represents a connection point where multiple wires
/// or components intersect, allowing electrical current to
/// flow between them.
#[derive(Debug, Clone, Default)]
pub struct Junction {
    /// `Pos` defines the X and Y coordinates of the junction.
    pub pos: Pos,
    /// Diameter of the junction.
    pub diameter: f64,
    /// Optional color of the junction.
    pub color: Option<Color>,
    /// Universally unique identifier for the junction.
    pub uuid: String,
}

impl Junction {
    pub fn new() -> Self {
        Self {
            pos: Pos::default(),
            diameter: 0.0,
            color: None,
            uuid: crate::uuid!(),
        }
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Junction {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Junction {
            pos: Pos::try_from(sexp)?,
            diameter: sexp.first(el::DIAMETER)?.unwrap_or(0.0),
            color: sexp.try_into().ok(),
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for Junction {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::JUNCTION);
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.end();
        builder.push(el::DIAMETER);
        builder.value(self.diameter);
        builder.end();
        if let Some(color) = self.color {
            color.write(builder)?;
        } else {
            Color::None.write(builder)?;
        }
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

///A `Bus` is a group of interconnected wires or connections that distribute
///signals among multiple devices or components, allowing them to share the
///same signal source.
#[derive(Debug, Clone)]
pub struct Bus {
    /// The list of X and Y coordinates of start and end points of the bus.
    pub pts: Pts,
    /// Defines how the bus is drawn.
    pub stroke: Stroke,
    /// Universally unique identifier for the bus.
    pub uuid: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Bus {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Bus {
            pts: sexp.try_into()?,
            stroke: sexp.try_into()?,
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for Bus {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::BUS);
        builder.push(el::PTS);
        for pt in &self.pts.0 {
            builder.push(el::XY);
            builder.value(round(pt.x));
            builder.value(round(pt.y));
            builder.end();
        }
        builder.end();
        self.stroke.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

/// `BusEentry` is a component representing an individual pin within
/// a multi-pin connection in a [`Bus`]
#[derive(Debug, Clone)]
pub struct BusEntry {
    /// The X and Y coordinates of the junction.
    pub pos: Pos,
    /// The size of the bus entry.
    pub size: (f64, f64),
    /// How the bus is drawn.
    pub stroke: Stroke,
    /// A universally unique identifier for this entry.
    pub uuid: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for BusEntry {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<BusEntry, Self::Error> {
        let size = sexp.require_node(el::SIZE)?;
        Ok(BusEntry {
            pos: Pos::try_from(sexp)?,
            size: (size.require_get(0)?, size.require_get(1)?),
            stroke: sexp.try_into()?,
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for BusEntry {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::BUS_ENTRY);
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.end();
        builder.push(el::SIZE);
        builder.value(self.size.0);
        builder.value(self.size.1);
        builder.end();
        self.stroke.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

/// Wires represent electrical connections between components or points,
/// showing the circuit's interconnections and paths for electric current flow.
#[derive(Debug, Clone, Default)]
pub struct Wire {
    /// The list of X and Y coordinates of start and end points of the wire.
    pub pts: Pts,
    /// Defines how the wire or bus is drawn.
    pub stroke: Stroke,
    /// Universally unique identifier for the wire.
    pub uuid: String,
    // The drawer attributes of the wire.
    // pub attrs: To,
}

impl Wire {
    pub fn new() -> Self {
        Self {
            pts: Pts::default(),
            stroke: Stroke::default(),
            uuid: crate::uuid!(),
            // attrs: To::default(),
        }
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Wire {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Wire {
            pts: sexp.try_into()?,
            stroke: sexp.try_into()?,
            // attrs: To::new(),
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for Wire {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::WIRE);
        builder.push(el::PTS);
        for pt in &self.pts.0 {
            builder.push(el::XY);
            builder.value(round(pt.x));
            builder.value(round(pt.y));
            builder.end();
        }
        builder.end();
        self.stroke.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

/// A `LocalLabel` refers to an identifier assigned to individual
/// Components or objects within a specific grouping on
/// the same schema page.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalLabel {
    /// The text displayed on the label.
    pub text: String,
    /// The position of the label within the schematic.
    pub pos: Pos,
    /// Defines the visual effects applied to the label (e.g., font style, shadow).
    pub effects: Effects,
    /// Optional color for the label. If not provided, a default color will be used.
    pub color: Option<Color>,
    /// Universally unique identifier for the label.
    pub uuid: String,
    /// Specifies whether the fields and positions are automatically populated.
    pub fields_autoplaced: bool,
    // The drawer attributes of the label.
    // pub attrs: To,
}

impl LocalLabel {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            pos: Pos::default(),
            effects: Effects::default(),
            color: None,
            uuid: crate::uuid!(),
            fields_autoplaced: false,
            // attrs: To::new(),
        }
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for LocalLabel {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(LocalLabel {
            text: sexp.require_get(0)?,
            pos: Pos::try_from(sexp)?,
            effects: sexp.try_into()?,
            color: sexp.try_into().ok(),
            fields_autoplaced: sexp.first(el::FIELDS_AUTOPLACED)?.unwrap_or(true),
            // attrs: To::new(),
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for LocalLabel {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::LABEL);
        builder.text(&self.text);
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(round(self.pos.angle));
        builder.end();
        // TODO is this a field in label
        // if self.fields_autoplaced {
        //     builder.push(el::FIELDS_AUTOPLACED);
        //     builder.value(el::YES);
        //     builder.end();
        // }
        self.effects.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

///A `GlobalLabel` is a custom identifier that can be assigned to
///multiple objects or components across the entire design.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GlobalLabel {
    /// The text displayed on the label.
    pub text: String,
    /// Optional shape of the label's container box. If not provided, the default shape is used.
    pub shape: Option<String>,
    /// The position of the label within the schematic.
    pub pos: Pos,
    /// Specifies whether the fields and positions are automatically populated.
    pub fields_autoplaced: bool,
    /// Defines the visual effects applied to the label (e.g., font style, shadow).
    pub effects: Effects,
    /// The list of symbol properties of the schematic global label..
    pub props: Vec<Property>,
    /// Universally unique identifier for the label.
    pub uuid: String,
    // TODO: Implement Properties struct and use it in this definition.
    // The drawer attributes of the label.
    // pub attrs: To,
}

impl GlobalLabel {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            ..Default::default()
        }
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for GlobalLabel {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(GlobalLabel {
            text: sexp.require_get(0)?,
            shape: sexp.first(el::SHAPE)?,
            pos: Pos::try_from(sexp)?,
            fields_autoplaced: sexp.first(el::FIELDS_AUTOPLACED)?.unwrap_or(true),
            effects: sexp.try_into()?,
            props: crate::properties(sexp)?,
            uuid: sexp.require_first(el::UUID)?,
            // attrs: To::new(),
        })
    }
}

impl SexpWrite for GlobalLabel {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::GLOBAL_LABEL);
        builder.text(&self.text);
        if let Some(shape) = &self.shape {
            builder.push(el::SHAPE);
            builder.value(shape);
            builder.end();
        }
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(round(self.pos.angle));
        builder.end();
        if self.fields_autoplaced {
            builder.push(el::FIELDS_AUTOPLACED);
            builder.value(el::YES);
            builder.end();
        }
        self.effects.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        for prop in &self.props {
            prop.write(builder)?;
        }
        builder.end();
        Ok(())
    }
}

/// `NoConnect` represents no electrical connection between two points.
/// It's used for clarity in cases where there should be no path but
/// one isn't explicitly shown. Proper usage ensures correct net
/// connections, avoiding errors, and passes ERC checks.
#[derive(Debug, Clone, Default)]
pub struct NoConnect {
    /// The X and Y coordinates of the no-connect within the schematic.
    pub pos: Pos,
    /// Universally unique identifier for the no-connect.
    pub uuid: String,
    // The drawer attributes of the no connect.
    // pub attrs: To,
}

impl NoConnect {
    pub fn new() -> Self {
        Self {
            pos: Pos::default(),
            uuid: crate::uuid!(),
            // attrs: To::new(),
        }
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for NoConnect {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(NoConnect {
            pos: Pos::try_from(sexp)?,
            // attrs: To::new(),
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for NoConnect {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::NO_CONNECT);
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.end();
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

///A `HierarchicalSheet`  represents a nested or hierarchical
///grouping of components or objects within a larger schematic.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchicalSheet {
    /// The position of the sheet within the schematic.
    pub pos: Pos,
    /// The width of the sheet.
    pub width: f64,
    /// The height of the sheet.
    pub height: f64,
    /// Specifies whether the fields and positions are automatically populated.
    pub fields_autoplaced: bool,
    /// Defines the stroke style of the sheet outline.
    pub stroke: Stroke,
    /// Defines the fill style of the sheet.
    pub fill: FillType,
    /// Universally unique identifier for the sheet.
    pub uuid: String,
    /// The list of symbol properties of the schematic symbol.
    pub props: Vec<Property>,
    /// The list of hierarchical pins associated with the sheet.
    pub pins: Vec<HierarchicalPin>,
    /// The list of instances grouped by project.
    pub instances: Vec<ProjectInstance>,
    pub in_bom: bool,
    pub on_board: bool,
    pub dnp: bool,
    pub exclude_from_sim: bool,
}

impl HierarchicalSheet {
    pub fn filename(&self) -> Option<String> {
        self.props
            .iter()
            .find(|p| p.key == "Sheetfile")
            .map(|p| p.value.clone())
    }
    pub fn sheet(&self) -> Option<String> {
        self.props
            .iter()
            .find(|p| p.key == "Sheetname")
            .map(|p| p.value.clone())
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for HierarchicalSheet {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let size = sexp.require_node(el::SIZE)?;
        let instance = sexp.require_node(el::INSTANCES)?;
        let project = instance.require_node(el::PROJECT)?;
        let path = project.require_node(el::PATH)?;
        Ok(HierarchicalSheet {
            pos: Pos::try_from(sexp)?,
            width: size.require_get(0)?,
            height: size.require_get(1)?,
            fields_autoplaced: sexp.first(el::FIELDS_AUTOPLACED)?.unwrap_or(true),
            stroke: sexp.try_into()?,
            fill: sexp.try_into()?,
            props: crate::properties(sexp)?,
            pins: sexp
                .query(el::PIN)
                .map(HierarchicalPin::try_from)
                .collect::<Result<Vec<_>, _>>()?,
            instances: vec![ProjectInstance {
                project_name: project.require_get(0)?,
                path: path.require_get(0)?,
                page_number: path.require_first(el::PAGE)?,
            }],
            in_bom: sexp.first(el::IN_BOM)?.unwrap_or(true),
            on_board: sexp.first(el::ON_BOARD)?.unwrap_or(true),
            dnp: sexp.first(el::DNP)?.unwrap_or(false),
            exclude_from_sim: sexp.first(el::EXCLUDE_FROM_SIM)?.unwrap_or(false),
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for HierarchicalSheet {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::SHEET);
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.end();
        builder.push(el::SIZE);
        builder.value(round(self.width));
        builder.value(round(self.height));
        builder.end();
        builder.push(el::EXCLUDE_FROM_SIM);
        builder.value(yes_or_no(self.exclude_from_sim));
        builder.end();
        if self.in_bom {
            builder.push(el::IN_BOM);
            builder.value(el::YES);
            builder.end();
        }
        if self.on_board {
            builder.push(el::ON_BOARD);
            builder.value(el::YES);
            builder.end();
        }
        builder.push(el::DNP);
        builder.value(yes_or_no(self.dnp));
        builder.end();
        if self.fields_autoplaced {
            builder.push(el::FIELDS_AUTOPLACED);
            builder.value(el::YES);
            builder.end();
        }
        self.stroke.write(builder)?;
        builder.push(el::FILL);
        builder.push(el::COLOR);
        builder.value(&self.fill);
        builder.end();
        builder.end();
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        for prop in &self.props {
            prop.write(builder)?;
        }
        for pin in &self.pins {
            pin.write(builder)?;
        }
        //instances
        for instance in &self.instances {
            builder.push(el::INSTANCES);
            builder.push(el::PROJECT);
            builder.text(&instance.project_name);
            builder.push(el::PATH);
            builder.text(&instance.path);
            builder.push(el::PAGE);
            builder.text(&instance.page_number);
            builder.end();
            builder.end();
            builder.end();
            builder.end();
        }
        builder.end();
        Ok(())
    }
}

/// Represents an instance of a hierarchical sheet within a specific project.
#[derive(Debug, Clone, PartialEq)]
pub struct ProjectInstance {
    /// The name of the project.
    pub project_name: String,
    /// The path to the sheet instance.
    pub path: String,
    /// The page number of the sheet instance.
    pub page_number: String,
}

///Represents an electrical connection between the sheet in a schematic
///and the hierarchical label defined in the associated schematic file.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchicalPin {
    /// The name of the sheet pin.
    pub name: String,
    /// The type of electrical connection made by the sheet pin.
    pub connection_type: ConnectionType,
    /// The position of the pin within the sheet.
    pub pos: Pos,
    /// Defines the visual effects applied to the pin name text.
    pub effects: Effects,
    /// Universally unique identifier for the pin.
    pub uuid: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for HierarchicalPin {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let conn_str: String = sexp.require_get(1)?;
        Ok(HierarchicalPin {
            name: sexp.get(0)?.unwrap_or_default(),
            connection_type: ConnectionType::try_from(conn_str)?,
            pos: Pos::try_from(sexp)?,
            effects: sexp.try_into()?,
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for HierarchicalPin {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::PIN);
        builder.text(&self.name);
        builder.value(&self.connection_type);
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(round(self.pos.angle));
        builder.end();
        self.effects.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

/// A Hierarchical Label is a placeholder for an instance within a sub-schema (child schematic)
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchicalLabel {
    /// The text of the hierarchical label.
    pub text: String,
    /// The shape token attribute defines the way the hierarchical label is drawn.
    /// TODO: Should be an enum
    pub shape: Option<String>,
    /// The position of the pin within the sheet.
    pub pos: Pos,
    /// Specifies whether the fields and positions are automatically populated.
    pub fields_autoplaced: bool,
    /// Defines the visual effects applied to the pin name text.
    pub effects: Effects,
    /// The list of properties of the hierarchical label.
    pub props: Vec<Property>,
    /// Universally unique identifier for the pin.
    pub uuid: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for HierarchicalLabel {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(HierarchicalLabel {
            text: sexp.get(0)?.unwrap_or(String::new()),
            shape: sexp.first(el::SHAPE)?,
            pos: Pos::try_from(sexp)?,
            fields_autoplaced: sexp.first(el::FIELDS_AUTOPLACED)?.unwrap_or(true),
            props: crate::properties(sexp)?,
            effects: sexp.try_into()?,
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for HierarchicalLabel {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::HIERARCHICAL_LABEL);
        builder.text(&self.text);
        if let Some(shape) = &self.shape {
            builder.push(el::SHAPE);
            builder.value(shape);
            builder.end();
        }
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(round(self.pos.angle));
        builder.end();
        //TODO: do we also have to remove this from the datastructure?
        // if self.fields_autoplaced {
        //     builder.push(el::FIELDS_AUTOPLACED);
        //     builder.value(el::YES);
        //     builder.end();
        // }
        self.effects.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

/// Assigns a user-defined label or category to a net (connection) within an electronic schematic
#[derive(Debug, Clone, PartialEq)]
pub struct NetclassFlag {
    /// The length og the netclass flag.
    pub length: f64,
    /// The name of the netclass.
    pub name: String,
    /// The shape token attribute defines the way the netclass flag is drawn.
    /// TODO: Should be an enum
    pub shape: Option<String>,
    /// The position of the pin within the sheet.
    pub pos: Pos,
    /// Specifies whether the fields and positions are automatically populated.
    pub fields_autoplaced: bool,
    /// Defines the visual effects applied to the pin name text.
    pub effects: Effects,
    /// The list of properties of the hierarchical label.
    pub props: Vec<Property>,
    /// Universally unique identifier for the pin.
    pub uuid: String,
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for NetclassFlag {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(NetclassFlag {
            name: sexp.get(0)?.unwrap_or(String::new()),
            length: sexp.require_first(el::LENGTH)?,
            shape: sexp.first(el::SHAPE)?,
            pos: Pos::try_from(sexp)?,
            fields_autoplaced: sexp.first(el::FIELDS_AUTOPLACED)?.unwrap_or(true),
            effects: sexp.try_into()?,
            props: crate::properties(sexp)?,
            uuid: sexp.require_first(el::UUID)?,
        })
    }
}

impl SexpWrite for NetclassFlag {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::NETCLASS_FLAG);
        builder.text(&self.name);
        builder.push(el::LENGTH);
        builder.value(self.length);
        builder.end();
        if let Some(shape) = &self.shape {
            builder.push(el::SHAPE);
            builder.value(shape);
            builder.end();
        }
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(self.pos.angle / 255.0);
        builder.end();
        if self.fields_autoplaced {
            builder.push(el::FIELDS_AUTOPLACED);
            builder.value(el::YES);
            builder.end();
        }
        self.effects.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        for prop in &self.props {
            prop.write(builder)?;
        }
        builder.end();
        Ok(())
    }
}

/// Defines the type of electrical connection made by the sheet pin.
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionType {
    /// Input connection type.
    Input,
    /// Output connection type.
    Output,
    /// Bidirectional connection type.
    Bidirectional,
    /// Tri-state connection type.
    TriState,
    /// Passive connection type.
    Passive,
}

impl TryFrom<String> for ConnectionType {
    type Error = RecadError;
    fn try_from(s: String) -> Result<Self, Self::Error> {
        match s.to_lowercase().as_str() {
            el::INPUT => Ok(ConnectionType::Input),
            el::OUTPUT => Ok(ConnectionType::Output),
            el::BIDIRECTIONAL => Ok(ConnectionType::Bidirectional),
            el::TRI_STATE => Ok(ConnectionType::TriState),
            el::PASSIVE => Ok(ConnectionType::Passive),
            _ => Err(RecadError::Pcb(format!("Invalid connection type: {}", s))),
        }
    }
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            ConnectionType::Input => el::INPUT,
            ConnectionType::Output => el::OUTPUT,
            ConnectionType::Bidirectional => el::BIDIRECTIONAL,
            ConnectionType::TriState => el::TRI_STATE,
            ConnectionType::Passive => el::PASSIVE,
        };
        write!(f, "{}", s)
    }
}

/// The instances token defines a symbol instance.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Instance {
    pub project: String,
    pub path: String,
    pub reference: String,
    pub unit: u8,
}

#[allow(unused_imports)]
use crate::symbols;
/// A schematic `Symbol` representing an instance from the [`symbols`] library.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Symbol {
    /// Library identifier: refers to a symbol in the library's symbol section.
    pub lib_id: String,
    /// The `pos` defines the X and Y coordinates and angle of rotation of the symbol.
    pub pos: Pos,
    /// The `mirror` defines if the symbol is mirrored. The only valid values are x, y, and xy.
    pub mirror: Option<String>,
    /// The unit token attribute defines which unit in the symbol library definition that the schematic symbol represents.
    pub unit: u8,
    /// The `in_bom` token attribute determines whether the schematic symbol appears in any bill of materials output.
    pub in_bom: bool,
    /// The `on_board` token attribute determines if the footprint associated with the symbol is exported to the board via the netlist.
    pub on_board: bool,
    /// The `exclude_from_sim` token attribute determines if the symbol is excluded from simulation.
    pub exclude_from_sim: bool,
    /// The `dnp` token attribute determines if the symbol is to be populated.
    pub dnp: bool,
    /// The universally unique identifier for the symbol. This is used to map the symbol to the symbol instance information.
    pub uuid: String,
    /// The list of symbol properties of the schematic symbol.
    pub props: Vec<Property>,
    /// The list of pins utilized by the symbol. This section may be empty if the symbol lacks any pins.
    pub pins: Vec<(String, String)>,
    /// The list of symbol instances grouped by project. Every symbol has at least one instance.
    /// The usage of this section is not clear to me. It lists all pins from the symbol and
    /// not just the one from the unit instance.
    pub instances: Vec<Instance>,
    // The drawer attributes of the wire.
    // pub attrs: To,
}

impl Symbol {
    pub fn new(reference: &str, value: &str, lib_id: &str) -> Self {
        Self {
            lib_id: lib_id.to_string(),
            pos: Pos::default(),
            mirror: None,
            unit: 1,
            in_bom: true,
            on_board: true,
            exclude_from_sim: false,
            dnp: false,
            uuid: crate::uuid!(),
            props: vec![
                Property {
                    key: el::PROPERTY_VALUE.to_string(),
                    value: value.to_string(),
                    pos: Pos::default(),
                    effects: Effects::default(),
                    hide: Some(false),
                },
                Property {
                    key: el::PROPERTY_REFERENCE.to_string(),
                    value: reference.to_string(),
                    pos: Pos::default(),
                    effects: Effects::default(),
                    hide: Some(false),
                },
            ],
            pins: Vec::new(),
            instances: Vec::new(),
            // attrs: To::new(),
        }
    }

    pub fn new_resistor(reference: &str, resistance: &str) -> Self {
        Self::new(reference, resistance, "Device:R")
    }

    pub fn new_capacitor(reference: &str, capacitance: &str) -> Self {
        Self::new(reference, capacitance, "Device:C")
    }

    pub fn new_power(voltage: &str) -> Self {
        Self::new(
            &format!("#PWR{}", voltage),
            voltage,
            &format!("power:{}", voltage),
        )
    }

    pub fn new_gnd() -> Self {
        Self::new("#PWRGND", "GND", "power:GND")
    }

    /// Get a property value by key
    pub fn property(&self, key: &str) -> Option<String> {
        self.props
            .iter()
            .find(|p| p.key == key)
            .map(|p| p.value.clone())
    }

    /// Set a property value by key
    pub fn set_property(&mut self, key: &str, value: &str) {
        self.props.iter_mut().for_each(|p| {
            if p.key == key {
                p.value = value.to_string();
            }
        });
    }


    pub fn outline(&self, lib_symbol: &LibrarySymbol) -> Result<Rect, RecadError> {
        // let lib_symbol = schema.library_symbol(&self.lib_id).ok_or_else(||
        //     RecadError::Schema(format!("Library symbol not found: {}", self.lib_id)))?;

        // Symbol transformation: Translation -> Rotation -> Mirror (Scale)
        let transform = Transform::new()
            .translation(Pt {
                x: self.pos.x,
                y: self.pos.y,
            })
            .mirror(&self.mirror)
            .rotation(self.pos.angle);

        let mut pts = Vec::new();
        for s in &lib_symbol.units {
            if s.unit() == 0 || s.unit() == self.unit {
                for g in &s.graphics {
                    match g {
                        GraphicItem::Arc(arc) => {
                            // Approximate arc with start, mid, end for bbox
                            pts.push(transform.transform_point(arc.start));
                            pts.push(transform.transform_point(arc.mid));
                            pts.push(transform.transform_point(arc.end));
                        }
                        GraphicItem::Circle(circle) => {
                            pts.push(transform.transform_point(Pt {
                                x: circle.center.x - circle.radius,
                                y: circle.center.y - circle.radius,
                            }));
                            pts.push(transform.transform_point(Pt {
                                x: circle.center.x + circle.radius,
                                y: circle.center.y + circle.radius,
                            }));
                        }
                        GraphicItem::Curve(_) => { todo!{"bbox for curve not implemented!"}}
                        GraphicItem::Line(line) => {
                            for p in &line.pts.0 {
                                pts.push(transform.transform_point(*p));
                            }
                        }
                        GraphicItem::Polyline(poly) => {
                            for p in &poly.pts.0 {
                                pts.push(transform.transform_point(*p));
                            }
                        }
                        GraphicItem::Rectangle(rect) => {
                            pts.push(transform.transform_point(rect.start));
                            pts.push(transform.transform_point(rect.end));
                        }
                        GraphicItem::Text(_) => { /* TODO: "bbox for text not implemented" */ }
                        GraphicItem::EmbeddedFont(_) => {}
                    }
                }
            }
        }
        for p in &lib_symbol.pins(self.unit) {
            pts.push(pin_position(self, p));

            let tail_local = Pt {
                x: p.length,
                y: 0.0,
            };
            let transform_pin = Transform::new()
                .translation(Pt {
                    x: p.pos.x,
                    y: p.pos.y,
                })
                .rotation(p.pos.angle);

            let tail_symbol = transform_pin.transform_point(tail_local);
            let tail_world = transform.transform_point(tail_symbol);
            pts.push(tail_world);
        }
        Ok(calculate(&pts))
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Symbol {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Symbol {
            lib_id: sexp.require_first(el::LIB_ID)?,
            pos: Pos::try_from(sexp)?,
            unit: sexp.first(el::SYMBOL_UNIT)?.unwrap_or(1),
            mirror: sexp.first(el::MIRROR)?,
            in_bom: sexp.first(el::IN_BOM)?.unwrap_or(true),
            on_board: sexp.first(el::ON_BOARD)?.unwrap_or(true),
            exclude_from_sim: sexp.first(el::EXCLUDE_FROM_SIM)?.unwrap_or(false),
            dnp: sexp.first(el::DNP)?.unwrap_or(false),
            uuid: sexp.require_first(el::UUID)?,
            props: crate::properties(sexp)?,
            pins: sexp
                .query(el::PIN)
                .map(|p| -> Result<(String, String), RecadError> {
                    let a = p.require_get(0)?;
                    let b = p.require_first(el::UUID)?;
                    Ok((a, b))
                })
                .collect::<Result<Vec<_>, RecadError>>()?,
            instances: {
                if let Some(instances) = sexp.query(el::INSTANCES).next() {
                    let project = instances.require_node(el::PROJECT)?;
                    let path = project.require_node(el::PATH)?;
                    vec![Instance {
                        project: project.require_get(0)?,
                        path: path.require_get(0)?,
                        reference: path.require_first(el::REFERENCE)?,
                        unit: path.require_first(el::SYMBOL_UNIT)?,
                    }]
                } else {
                    vec![]
                }
            },
            // attrs: To::new(),
        })
    }
}

impl SexpWrite for Symbol {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::SYMBOL);
        builder.push(el::LIB_ID);
        builder.text(&self.lib_id);
        builder.end();
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(round(self.pos.angle));
        builder.end();
        if let Some(mirror) = &self.mirror {
            builder.push(el::MIRROR);
            builder.value(mirror);
            builder.end();
        }
        builder.push(el::SYMBOL_UNIT);
        builder.value(self.unit);
        builder.end();
        builder.push(el::EXCLUDE_FROM_SIM);
        builder.value(types::yes_or_no(self.exclude_from_sim));
        builder.end();
        builder.push(el::IN_BOM);
        builder.value(types::yes_or_no(self.in_bom));
        builder.end();
        builder.push(el::ON_BOARD);
        builder.value(types::yes_or_no(self.on_board));
        builder.end();
        builder.push(el::DNP);
        builder.value(types::yes_or_no(self.dnp));
        builder.end();
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();

        for prop in &self.props {
            prop.write(builder)?;
        }

        for pin in &self.pins {
            builder.push(el::PIN);
            builder.text(&pin.0);
            builder.push(el::UUID);
            builder.text(&pin.1);
            builder.end();
            builder.end();
        }

        for instance in &self.instances {
            builder.push(el::INSTANCES);
            builder.push(el::PROJECT);
            builder.text(&instance.project);
            builder.push(el::PATH);
            builder.text(&instance.path);
            builder.push(el::REFERENCE);
            builder.text(&instance.reference);
            builder.end();
            builder.push(el::SYMBOL_UNIT);
            builder.value(instance.unit);
            builder.end();
            builder.end();
            builder.end();
            builder.end();
        }
        builder.end();

        Ok(())
    }
}

#[derive(Clone, Debug, Default)]
///Define the `Schematic` file format.
pub struct Schema {
    /// The file path of this schema
    pub path: Option<String>,
    /// The page name
    pub sheet: Option<String>,
    /// The Project Title
    /// This title serves as a reference for symbol instances throughout the project.
    /// Typically, the title matches the file name.
    pub project: String,
    /// The `version` defines the schematic version
    /// using the YYYYMMDD date format.
    pub version: String,
    /// The `uuid` defines the universally unique identifier for
    /// the schematic file.
    pub uuid: String,
    /// `generator` defines the program used
    /// to write the file.
    pub generator: String,
    /// `generator_version` specifies the program version for file writing
    pub generator_version: Option<String>,
    pub paper: PaperSize,
    pub title_block: TitleBlock,
    pub library_symbols: Vec<LibrarySymbol>,
    pub library_paths: Vec<PathBuf>,
    pub items: Vec<SchemaItem>,
    pub sheet_instances: Vec<Instance>,
    pub embedded_fonts: Option<String>,
}

/// General functions for the schema.
impl Schema {
    ///Create an empty schema.
    pub fn new(project: &str, library_paths: Option<Vec<String>>) -> Self {
        Self {
            path: None,
            sheet: None,
            project: project.to_string(),
            version: String::from("20231120"), // TODO use new version
            uuid: crate::uuid!(),
            generator: String::from("recad"),
            generator_version: None,
            paper: PaperSize::A4,
            title_block: TitleBlock {
                title: None,
                date: None,
                revision: None,
                company_name: None,
                comment: Vec::new(),
            },
            library_symbols: Vec::new(),
            library_paths: if let Some(paths) = library_paths {
                paths.into_iter().map(PathBuf::from).collect()
            } else {
                //TODO use dynamic path
                vec![PathBuf::from("/usr/share/kicad/symbols/".to_string())]
            },
            items: Vec::new(),
            sheet_instances: vec![Instance {
                project: String::new(),
                path: String::from("/"),
                reference: String::from("1"),
                unit: 0,
            }],
            embedded_fonts: None,
        }
    }

    ///Load a schema from a path
    ///
    ///``` TODO
    ///use recad_core::Schema;
    ///use std::path::Path;
    ///
    ///let path = Path::new("tests/summe/summe.kicad_sch");
    ///
    ///let schema = Schema::load(path);
    ///assert!(schema.is_ok());
    ///
    pub fn load(path: &Path, sheet: Option<String>) -> Result<Self, RecadError> {
        spdlog::debug!("load schema: {:?}", path);
        let parser = sexp::parser::SexpParser::load(path)?;
        let tree = sexp::SexpTree::from(parser.iter())?;
        let schema: Result<Self, RecadError> = Schema::try_from(tree);
        if let Ok(mut schema) = schema {
            schema.path = Some(path.to_str().unwrap().to_string());
            schema.sheet = sheet;
            Ok(schema)
        } else {
            schema
        }
    }

    ///Save a schema to a path
    pub fn save(&self) {
        //TODO
    }

    ///Get a Symbol by reference and unit number.
    ///
    ///TODO
    /// use recad_core::Schema;
    /// use std::path::Path;
    ///
    /// let path = Path::new("tests/summe/summe.kicad_sch");
    ///
    /// let schema = Schema::load(path, None).unwrap();
    /// let symbol = schema.symbol("U1", 1);
    /// assert!(symbol.is_some());
    ///
    pub fn symbol(&self, reference: &str, unit: u8) -> Option<&Symbol> {
        self.items.iter().find_map(|s| match s {
            SchemaItem::Symbol(s) => {
                if unit == s.unit
                    && reference == s.property(el::PROPERTY_REFERENCE).unwrap_or_default()
                {
                    Some(s)
                } else {
                    None
                }
            }
            _ => None,
        })
    }

    /// Obtain symbol unit from pin number.
    ///
    ///TODO
    /// use recad_core::Schema;
    /// use std::path::Path;
    ///
    /// let path = Path::new("tests/summe/summe.kicad_sch");
    ///
    /// let schema = Schema::load(path).unwrap();
    /// assert_eq!(Some(1), schema.pin_unit("U2", "1"));
    /// assert_eq!(Some(2), schema.pin_unit("U2", "7"));
    ///
    pub fn pin_unit(&self, reference: &str, pin: &str) -> Option<u8> {
        self.items
            .iter()
            .filter_map(|s| match s {
                SchemaItem::Symbol(s) => {
                    if reference == s.property(el::PROPERTY_REFERENCE).unwrap_or_default() {
                        if let Some(lib) = self.library_symbol(&s.lib_id) {
                            if let Some(unit_def) = lib.pin_unit(pin) {
                                if unit_def == 0 {
                                    return Some(s.unit);
                                } else {
                                    return Some(unit_def);
                                }
                            }
                        }
                    }
                    None
                }
                _ => None,
            })
            .next()
    }

    /// Generate the next power reference.
    pub fn next_power(&self) -> String {
        let max = self
            .items
            .iter()
            .filter_map(|s| {
                if let SchemaItem::Symbol(symbol) = s {
                    let ref_val = symbol.property(el::PROPERTY_REFERENCE).unwrap_or_default();
                    if let Some(str) = ref_val.strip_prefix("#PWR") {
                        str.parse::<u32>().ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .max();

        if let Some(max) = max {
            format!("#PWR{:02}", max + 1)
        } else {
            "#PWR01".to_string()
        }
    }

    /// Generate the next power flag reference..
    pub fn next_flag(&self) -> String {
        let max = self
            .items
            .iter()
            .filter_map(|s| {
                if let SchemaItem::Symbol(symbol) = s {
                    let ref_val = symbol.property(el::PROPERTY_REFERENCE).unwrap_or_default();
                    if let Some(str) = ref_val.strip_prefix("#FLG") {
                        str.parse::<u32>().ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .max();

        if let Some(max) = max {
            format!("#FLG{:02}", max + 1)
        } else {
            "#FLG01".to_string()
        }
    }

    /// Get a library symbol by lib_id
    ///
    ///TODO
    /// use recad_core::Schema;
    /// use std::path::Path;
    ///
    /// let path = Path::new("tests/summe/summe.kicad_sch");
    ///
    /// let schema = Schema::load(path, None).unwrap();
    /// let symbol = schema.library_symbol("Device:R");
    /// assert!(symbol.is_some());
    ///
    pub fn library_symbol(&self, lib_id: &str) -> Option<&LibrarySymbol> {
        self.library_symbols.iter().find(|s| s.lib_id == lib_id)
    }

    /// Get a library symbol by lib_id
    /// Also load the symbol
    ///TODO
    /// use recad_core::Schema;
    /// use std::path::Path;
    ///
    /// let path = Path::new("tests/summe/summe.kicad_sch");
    ///
    /// let schema = Schema::load(path, None).unwrap();
    /// let symbol = schema.library_symbol("Device:R");
    /// assert!(symbol.is_some());
    ///
    pub fn library_symbol_mut(&mut self, lib_id: &str) -> Result<&LibrarySymbol, RecadError> {
        // Load the library symbol
        let symbol = self.library_symbols.iter().find(|s| s.lib_id == lib_id);
        if symbol.is_none() {
            let lib = SymbolLibrary {
                pathlist: self.library_paths.clone(),
            }
            .load(lib_id);
            match lib {
                Ok(lib) => {
                    self.library_symbols.push(lib.clone());

                }
                Err(err) => return Err(err),
            }
        }

        self.library_symbols.iter().find(|s| s.lib_id == lib_id).ok_or_else(||RecadError::Schema(format!("LibrarySymbol {} not found.", lib_id)))
    }

    pub fn symbol_by_ref(&self, ref_des: &str) -> Option<&Symbol> {
        self.items.iter().find_map(|item| {
            if let SchemaItem::Symbol(s) = item {
                if s.property(el::PROPERTY_REFERENCE).unwrap_or_default() == ref_des {
                    return Some(s);
                }
            }
            None
        })
    }

    // write the schema to a `Write`.
    pub fn write(&self, writer: &mut dyn Write) -> Result<(), RecadError> {
        let mut builder = Builder::new();
        builder.push("kicad_sch");

        builder.push(el::VERSION);
        builder.value(&self.version);
        builder.end();

        builder.push("generator");
        builder.text(&self.generator);
        builder.end();

        if let Some(version) = &self.generator_version {
            builder.push("generator_version");
            builder.text(version);
            builder.end();
        }

        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();

        builder.push(el::PAPER);
        builder.text(&self.paper);
        builder.end();

        builder.push(el::TITLE_BLOCK);

        if let Some(title) = &self.title_block.title {
            builder.push(el::TITLE_BLOCK_TITLE);
            builder.text(title);
            builder.end();
        }
        if let Some(date) = &self.title_block.date {
            builder.push(el::TITLE_BLOCK_DATE);
            builder.text(date);
            builder.end();
        }
        if let Some(rev) = &self.title_block.revision {
            builder.push(el::TITLE_BLOCK_REV);
            builder.text(rev);
            builder.end();
        }
        if let Some(company) = &self.title_block.company_name {
            builder.push(el::TITLE_BLOCK_COMPANY);
            builder.text(company);
            builder.end();
        }
        for c in &self.title_block.comment {
            builder.push(el::TITLE_BLOCK_COMMENT);
            builder.value(c.0);
            builder.text(&c.1);
            builder.end();
        }
        builder.end();

        builder.push(el::LIB_SYMBOLS);
        for symbol in &self.library_symbols {
            symbol.write(&mut builder)?;
        }
        builder.end();

        for item in &self.items {
            match item {
                crate::schema::SchemaItem::Arc(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Bus(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::BusEntry(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Circle(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Curve(_item) => {
                    todo!();
                }
                crate::schema::SchemaItem::GlobalLabel(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Junction(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Line(_item) => {
                    todo!();
                }
                crate::schema::SchemaItem::LocalLabel(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::NoConnect(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Polyline(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Rectangle(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Symbol(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Text(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Wire(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::HierarchicalSheet(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::TextBox(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::HierarchicalLabel(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::NetclassFlag(item) => item.write(&mut builder)?,
            }
        }

        for instance in &self.sheet_instances {
            builder.push(el::SHEET_INSTANCES);
            builder.push(el::PATH);
            builder.text(&instance.path);
            builder.push(el::PAGE);
            builder.text(&instance.reference);
            builder.end();
            builder.end();
            builder.end();
        }

        if let Some(embedded_fonts) = &self.embedded_fonts {
            builder.push(el::EMBEDDED_FONTS);
            builder.value(embedded_fonts);
            builder.end();
        }

        builder.end();

        let sexp = builder.sexp().unwrap();
        sexp.write(writer)?;
        writer.write_all("\n".as_bytes())?;

        Ok(())
    }

    // pub fn partlist(
    //     &self,
    //     group: bool,
    //     partlist: Option<PathBuf>,
    // ) -> Result<(Vec<BomItem>, Option<Vec<BomItem>>), RecadError> {
    //     crate::reports::bom::bom(self, group, partlist)
    // }

    // pub fn erc(&self) -> Vec<ERCViolation> {
    //     let checker = Erc::new(self);
    //     checker.run()
    // }
}

// impl fmt::Display for Schema {
//     fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
//         let mut writer = Vec::new();
//         self.write(&mut writer).unwrap();
//         String::from_utf8(writer).unwrap().fmt(f)
//     }
// }

// #[derive(Debug, Clone, Default)]
// pub struct Feedback {
//     pub atref: Option<(String, String)>,
//     pub toref: Option<(String, String)>,
//     pub with: Option<Symbol>,
//     pub height: f64,
//     pub dot: Option<Vec<crate::draw::DotPosition>>,
// }

// impl Feedback {
//     pub fn new() -> Self {
//         Self {
//             height: 5.0 * 2.54,
//             ..Default::default()
//         }
//     }
// }

impl<'a> std::convert::TryFrom<SexpTree<'a>> for Schema {
    type Error = RecadError;
    fn try_from(sexp: SexpTree) -> Result<Self, Self::Error> {
        let mut schema = Schema::default();
        for node in sexp.root().nodes() {
            match node.name.as_ref() {
                el::UUID => schema.uuid = node.require_get(0)?,
                el::GENERATOR => schema.generator = node.get(0)?.unwrap_or_default(),
                el::GENERATOR_VERSION => schema.generator_version = node.get(0)?,
                el::VERSION => schema.version = node.get(0)?.unwrap_or("0".to_string()),
                el::JUNCTION => schema
                    .items
                    .push(SchemaItem::Junction(Junction::try_from(node)?)),
                el::PAPER => {
                    schema.paper = PaperSize::from(node.value_iter().next().unwrap_or("A4"));
                }
                el::WIRE => {
                    schema.items.push(SchemaItem::Wire(Wire::try_from(node)?));
                }
                el::BUS => schema.items.push(SchemaItem::Bus(Bus::try_from(node)?)),
                el::BUS_ENTRY => schema
                    .items
                    .push(SchemaItem::BusEntry(BusEntry::try_from(node)?)),
                el::LABEL => schema
                    .items
                    .push(SchemaItem::LocalLabel(LocalLabel::try_from(node)?)),
                el::GLOBAL_LABEL => schema
                    .items
                    .push(SchemaItem::GlobalLabel(GlobalLabel::try_from(node)?)),
                el::NO_CONNECT => schema
                    .items
                    .push(SchemaItem::NoConnect(NoConnect::try_from(node)?)),
                el::TITLE_BLOCK => schema.title_block = TitleBlock::try_from(node)?,
                el::LIB_SYMBOLS => {
                    schema.library_symbols = node
                        .query(el::SYMBOL)
                        .map(LibrarySymbol::try_from)
                        .collect::<Result<Vec<LibrarySymbol>, RecadError>>()?;
                }
                el::SYMBOL => schema
                    .items
                    .push(SchemaItem::Symbol(Symbol::try_from(node)?)),
                el::CIRCLE => schema.items.push(SchemaItem::Circle(node.try_into()?)),
                el::POLYLINE => schema.items.push(SchemaItem::Polyline(node.try_into()?)),
                el::RECTANGLE => schema.items.push(SchemaItem::Rectangle(node.try_into()?)),
                el::ARC => schema.items.push(SchemaItem::Arc(node.try_into()?)),
                el::TEXT => schema.items.push(SchemaItem::Text(node.try_into()?)),
                el::TEXT_BOX => schema.items.push(SchemaItem::TextBox(node.try_into()?)),
                el::SHEET => {
                    schema
                        .items
                        .push(SchemaItem::HierarchicalSheet(HierarchicalSheet::try_from(
                            node,
                        )?))
                }
                el::HIERARCHICAL_LABEL => {
                    schema
                        .items
                        .push(SchemaItem::HierarchicalLabel(HierarchicalLabel::try_from(
                            node,
                        )?))
                }
                el::NETCLASS_FLAG => schema
                    .items
                    .push(SchemaItem::NetclassFlag(NetclassFlag::try_from(node)?)),
                el::SHEET_INSTANCES => {
                    schema.sheet_instances = node
                        .query(el::PATH)
                        .map(|path| -> Result<Instance, RecadError> {
                            Ok(Instance {
                                project: String::new(),
                                path: path.require_get(0)?,
                                reference: path.require_first(el::PAGE)?,
                                unit: 0,
                            })
                        })
                        .collect::<Result<Vec<_>, Self::Error>>()?;
                }
                el::EMBEDDED_FONTS => schema.embedded_fonts = Some(node.require_get(0)?),
                _ => spdlog::error!("unknown root node: {:?}", node.name),
            }
        }
        Ok(schema)
    }
}

/// Abstraction of the schema items for iteration
#[derive(Clone, Debug)]
pub enum SchemaItem {
    Arc(Arc),
    Bus(Bus),
    BusEntry(BusEntry),
    Circle(Circle),
    Curve(Curve),
    GlobalLabel(GlobalLabel),
    HierarchicalSheet(HierarchicalSheet),
    HierarchicalLabel(HierarchicalLabel),
    Junction(Junction),
    Line(Line),
    LocalLabel(LocalLabel),
    NetclassFlag(NetclassFlag),
    NoConnect(NoConnect),
    Polyline(Polyline),
    Rectangle(Rectangle),
    Symbol(Symbol),
    Text(Text),
    TextBox(TextBox),
    Wire(Wire),
}

// #[cfg(test)]
// mod tests {
//
//     pub const SCHEMA_SUMME: &str = "tests/summe/summe.kicad_sch";
//     use std::path::Path;
//
//     use crate::{
//         schema::{Schema, SchemaItem, Symbol},
//     };
//
//     #[test]
//     fn symbol_property() {
//         let schema = Schema::load(Path::new(SCHEMA_SUMME), None).unwrap();
//         let symbol = schema
//             .items
//             .iter()
//             .filter_map(|s| match s {
//                 SchemaItem::Symbol(s) => Some(s),
//                 _ => None,
//             })
//             .collect::<Vec<&Symbol>>()[0];
//         assert_eq!("J2".to_string(), symbol.property("Reference").unwrap());
//     }
//
//     #[test]
//     fn get_symbol() {
//         let schema = Schema::load(Path::new(SCHEMA_SUMME), None).unwrap();
//         let symbol = schema.symbol("U1", 1).unwrap();
//         assert_eq!("U1", symbol.property("Reference").unwrap());
//     }
//
//     #[test]
//     fn get_lib_symbol() {
//         let schema = Schema::load(Path::new(SCHEMA_SUMME), None).unwrap();
//         let symbol = schema.symbol("U1", 1).unwrap();
//         let lib_symbol = schema.library_symbol(&symbol.lib_id).unwrap();
//         assert_eq!(
//             "Reference_Voltage:LM4040DBZ-5".to_string(),
//             lib_symbol.lib_id
//         );
//     }
//
//     #[test]
//     fn get_lib_symbol_unit() {
//         let schema = Schema::load(Path::new(SCHEMA_SUMME), None).unwrap();
//         let symbol = schema.symbol("U1", 1).unwrap();
//         let lib_symbol = schema.library_symbol(&symbol.lib_id).unwrap();
//
//         let mut iter = lib_symbol.units.iter();
//         let first = iter.next().unwrap();
//         assert_eq!(0, first.unit());
//         assert_eq!(1, first.style());
//
//         let second = iter.next().unwrap();
//         assert_eq!(1, second.unit());
//         assert_eq!(1, second.style());
//     }
// }
