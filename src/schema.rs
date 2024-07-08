use std::{fmt, io::Write, path::Path};

use ndarray::{Array, ArrayView};

use crate::{
    draw::At,
    gr::{
        Arc, Circle, Color, Curve, Effects, FillType, Line, PaperSize, Polyline, Pos, Property, Pt,
        Pts, Rect, Rectangle, Stroke, TitleBlock,
    },
    math::{bbox::Bbox, ToNdarray},
    sexp::{builder::Builder, constants::el},
    symbols::LibrarySymbol,
    Error, Schema, SexpWrite,
};

// TODO A schema text has the `exclude_from_sim` field, which is not included in `gr:Text`

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

///A `TextBox`in the schema
#[derive(Debug, Clone)]
pub struct TextBox {
    /// X and Y coordinates of the text.
    pub pos: Pos,
    /// The text to display.
    pub text: String,
    /// The width of the text box.
    pub width: f32,
    /// The height of the text box.
    pub height: f32,
    /// Defines how the box is drawn.
    pub stroke: Stroke,
    /// Defines the fill style of the box.
    pub fill: FillType,
    /// Text effects such as font, color, etc.
    pub effects: Effects,
    /// Whether the text is a simulation instruction (not supported in recad).
    pub exclude_from_sim: bool,
    /// Universally unique identifier for the text.
    pub uuid: String,
}

///A junction represents a connection point where multiple wires
///or components intersect, allowing electrical current to
///flow between them.
#[derive(Debug, Clone, Default)]
pub struct Junction {
    /// `Pos` defines the X and Y coordinates of the junction.
    pub pos: Pos,
    /// Diameter of the junction.
    pub diameter: f32,
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

/// `BusEentry` is a component representing an individual pin within
/// a multi-pin connection in a [`Bus`]
#[derive(Debug, Clone)]
pub struct BusEntry {
    /// The X and Y coordinates of the junction.
    pub pos: Pos,
    /// The size of the bus entry.
    pub size: (f32, f32),
    /// How the bus is drawn.
    pub stroke: Stroke,
    /// A universally unique identifier for this entry.
    pub uuid: String,
}

/// Wires represent electrical connections between components or points,
/// showing the circuit's interconnections and paths for electric current flow.
#[derive(Debug, Clone)]
pub struct Wire {
    /// The list of X and Y coordinates of start and end points of the wire.
    pub pts: Pts,
    /// Defines how the wire or bus is drawn.
    pub stroke: Stroke,
    /// Universally unique identifier for the wire.
    pub uuid: String,
}

/// A `LocalLabel` refers to an identifier assigned to individual
/// components or objects within a specific grouping on
/// the same `[SchemaPage]`.
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
}

impl LocalLabel {
    pub fn new(text: String) -> Self {
        Self {
            text,
            pos: Pos::default(),
            effects: Effects::default(),
            color: None,
            uuid: crate::uuid!(),
            fields_autoplaced: false,
        }
    }
}

///A `GlobalLabel` is a custom identifier that can be assigned to
///multiple objects or components across the entire design.
#[derive(Debug, Clone, PartialEq)]
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
}

/// `NoConnect` represents no electrical connection between two points.
/// It's used for clarity in cases where there should be no path but
/// one isn't explicitly shown. Proper usage ensures correct net
/// connections, avoiding errors, and passes ERC checks.
#[derive(Debug, Clone)]
pub struct NoConnect {
    /// The X and Y coordinates of the no-connect within the schematic.
    pub pos: Pos,
    /// Universally unique identifier for the no-connect.
    pub uuid: String,
}

///A `HierarchicalSheet`  represents a nested or hierarchical
///grouping of components or objects within a larger schematic.
#[derive(Debug, Clone, PartialEq)]
pub struct HierarchicalSheet {
    /// The position of the sheet within the schematic.
    pub pos: Pos,
    /// The width of the sheet.
    pub width: f32,
    /// The height of the sheet.
    pub height: f32,
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

/// Assigns a user-defined label or category to a net (connection) within an electronic schematic
#[derive(Debug, Clone, PartialEq)]
pub struct NetclassFlag {
    /// The length og the netclass flag.
    pub length: f32,
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

impl From<String> for ConnectionType {
    fn from(s: String) -> Self {
        match s.to_lowercase().as_str() {
            "input" => ConnectionType::Input,
            "output" => ConnectionType::Output,
            "bidirectional" => ConnectionType::Bidirectional,
            "tri_state" => ConnectionType::TriState,
            "passive" => ConnectionType::Passive,
            _ => panic!("Invalid connection type"),
        }
    }
}

impl fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            ConnectionType::Input => "input",
            ConnectionType::Output => "output",
            ConnectionType::Bidirectional => "bidirectional",
            ConnectionType::TriState => "tri_state",
            ConnectionType::Passive => "passive",
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
    pub instances: Vec<Instance>,
}

impl Symbol {
    ///get a property value by key
    pub fn property(&self, key: &str) -> String {
        self.props
            .iter()
            .filter_map(|p| {
                if p.key == key {
                    Some(p.value.to_string())
                } else {
                    None
                }
            })
            .collect::<String>()
    }
    pub fn set_property(&mut self, key: &str, value: &str) {
        self.props.iter_mut().for_each(|p| {
            if p.key == key {
                p.value = value.to_string();
            }
        });
    }
}

///General functions for the schema.
impl Schema {
    ///Create an empty schema.
    pub fn new() -> Self {
        Self {
            version: String::from("0.0"),
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
            items: Vec::new(),
            sheet_instances: Vec::new(),
            grid: 1.27,
            last_pos: At::Pt(Pt { x: 0.0, y: 0.0 }),
        }
    }

    ///Load a schema from a path
    ///
    ///```
    ///use recad::Schema;
    ///use std::path::Path;
    ///
    ///let path = Path::new("tests/summe.kicad_sch");
    ///
    ///let schema = Schema::load(path);
    ///assert!(schema.is_ok());
    ///
    pub fn load(path: &Path) -> Result<Self, Error> {
        let parser = crate::sexp::parser::SexpParser::load(path).unwrap();
        let tree = crate::sexp::SexpTree::from(parser.iter()).unwrap();
        tree.into()
    }
    ///Save a schema to a path
    pub fn save(&self) {
        //TODO
    }

    ///Get a Symbol by reference and unit number.
    ///
    ///```
    /// use recad::Schema;
    /// use std::path::Path;
    ///
    /// let path = Path::new("tests/summe.kicad_sch");
    ///
    /// let schema = Schema::load(path).unwrap();
    /// let symbol = schema.symbol("U1", 1);
    /// assert!(symbol.is_some());
    ///
    pub fn symbol(&self, reference: &str, unit: u8) -> Option<&Symbol> {
        self.items
            .iter()
            .filter_map(|s| match s {
                SchemaItem::Symbol(s) => {
                    if unit == s.unit && reference == s.property("Reference") {
                        Some(s)
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .collect::<Vec<&Symbol>>()
            .first()
            .copied()
    }

    ///// Obtain symbol unit from pin number.
    /////
    /////```
    ///// use recad::Schema;
    ///// use std::path::Path;
    /////
    ///// let path = Path::new("tests/summe.kicad_sch");
    /////
    ///// let schema = Schema::load(path).unwrap();
    ///// assert_eq!(Some(1), schema.pin_unit("U2", "1"));
    ///// assert_eq!(Some(2), schema.pin_unit("U2", "7"));
    /////
    //pub fn pin_unit(&self, reference: &str, pin: &str) -> Option<u8> {
    //    self.items
    //        .iter()
    //        .filter_map(|s| match s {
    //            SchemaItem::Symbol(s) => {
    //                if reference == s.property(el::PROPERTY_REFERENCE) {
    //                    if let Some(lib) = self.library_symbol(&s.lib_id) {
    //                        if lib.pin(pin).is_some() {
    //                            Some(s.unit)
    //                        } else {
    //                            None
    //                        }
    //                    } else {
    //                        None
    //                    }
    //                } else {
    //                    None
    //                }
    //            }
    //            _ => None,
    //        })
    //        .collect::<Vec<u8>>()
    //        .first()
    //        .copied()
    //}

    /// Get a library symbol by lib_id
    ///
    ///```
    /// use recad::Schema;
    /// use std::path::Path;
    ///
    /// let path = Path::new("tests/summe.kicad_sch");
    ///
    /// let schema = Schema::load(path).unwrap();
    /// let symbol = schema.library_symbol("Device:R");
    /// assert!(symbol.is_some());
    ///
    pub fn library_symbol(&self, lib_id: &str) -> Option<&LibrarySymbol> {
        self.library_symbols
            .iter()
            .filter(|s| s.lib_id == lib_id)
            .collect::<Vec<&LibrarySymbol>>()
            .first()
            .copied()
    }

    /// Returns the outline of this [`Schema`].
    pub fn outline(&self) -> Result<Rect, Error> {
        let mut pts = Array::zeros((0, 2));
        for item in &self.items {
            match item {
                crate::schema::SchemaItem::Junction(junction) => {
                    let bound = junction.outline(self)?.ndarray();
                    pts.push_row(ArrayView::from(&[bound[[0, 0]], bound[[0, 1]]]))
                        .expect("insertion failed");
                    pts.push_row(ArrayView::from(&[bound[[1, 0]], bound[[1, 1]]]))
                        .expect("insertion failed");
                }
                crate::schema::SchemaItem::NoConnect(nc) => {
                    let bound = nc.outline(self)?.ndarray();
                    pts.push_row(ArrayView::from(&[bound[[0, 0]], bound[[0, 1]]]))
                        .expect("insertion failed");
                    pts.push_row(ArrayView::from(&[bound[[1, 0]], bound[[1, 1]]]))
                        .expect("insertion failed");
                }
                crate::schema::SchemaItem::Wire(wire) => {
                    let bound = wire.outline(self)?.ndarray();
                    pts.push_row(ArrayView::from(&[bound[[0, 0]], bound[[0, 1]]]))
                        .expect("insertion failed");
                    pts.push_row(ArrayView::from(&[bound[[1, 0]], bound[[1, 1]]]))
                        .expect("insertion failed");
                }
                crate::schema::SchemaItem::LocalLabel(label) => {
                    let bound = label.outline(self)?.ndarray();
                    pts.push_row(ArrayView::from(&[bound[[0, 0]], bound[[0, 1]]]))
                        .expect("insertion failed");
                    pts.push_row(ArrayView::from(&[bound[[1, 0]], bound[[1, 1]]]))
                        .expect("insertion failed");
                }
                crate::schema::SchemaItem::GlobalLabel(label) => {
                    let bound = label.outline(self)?.ndarray();
                    pts.push_row(ArrayView::from(&[bound[[0, 0]], bound[[0, 1]]]))
                        .expect("insertion failed");
                    pts.push_row(ArrayView::from(&[bound[[1, 0]], bound[[1, 1]]]))
                        .expect("insertion failed");
                }
                crate::schema::SchemaItem::Symbol(symbol) => {
                    let bound = symbol.outline(self)?.ndarray();
                    pts.push_row(ArrayView::from(&[bound[[0, 0]], bound[[0, 1]]]))
                        .expect("insertion failed");
                    pts.push_row(ArrayView::from(&[bound[[1, 0]], bound[[1, 1]]]))
                        .expect("insertion failed");

                    for prop in &symbol.props {
                        if prop.visible() {
                            let bound = prop.outline(self)?.ndarray();
                            pts.push_row(ArrayView::from(&[bound[[0, 0]], bound[[0, 1]]]))
                                .expect("insertion failed");
                            pts.push_row(ArrayView::from(&[bound[[1, 0]], bound[[1, 1]]]))
                                .expect("insertion failed");
                        }
                    }
                }
                _ => {}
            }
        }
        Ok(crate::math::bbox::calculate(pts))
    }

    /// write the schema to a `Write`.
    pub fn write(&self, writer: &mut dyn Write) -> Result<(), Error> {
        let mut builder = Builder::new();
        builder.push("kicad_sch");

        builder.push("version");
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
        builder.text(&self.paper.to_string());
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
        for c in &self.title_block.comment {
            builder.push(el::TITLE_BLOCK_COMMENT);
            builder.value(&c.0.to_string());
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
                crate::schema::SchemaItem::Curve(item) => {
                    todo!();
                } //item.write(&mut builder)?,
                crate::schema::SchemaItem::GlobalLabel(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Junction(item) => item.write(&mut builder)?,
                crate::schema::SchemaItem::Line(item) => {
                    todo!();
                } //item.write(&mut builder)?,
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

        builder.end();

        let sexp = builder.sexp().unwrap();
        sexp.write(writer)?;
        writer.write_all("\n".as_bytes())?;

        Ok(())
    }
}

/// Abstraction of the schema items for iteration
#[derive(Debug)]
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{
        schema::{SchemaItem, Symbol},
        Schema,
    };

    #[test]
    fn symbol_property() {
        let schema = Schema::load(Path::new("tests/summe.kicad_sch")).unwrap();
        let symbol = schema
            .items
            .iter()
            .filter_map(|s| match s {
                SchemaItem::Symbol(s) => Some(s),
                _ => None,
            })
            .collect::<Vec<&Symbol>>()[0];
        assert_eq!("J2".to_string(), symbol.property("Reference"));
    }

    #[test]
    fn get_symbol() {
        let schema = Schema::load(Path::new("tests/summe.kicad_sch")).unwrap();
        let symbol = schema.symbol("U1", 1).unwrap();
        assert_eq!("U1", symbol.property("Reference"));
    }

    #[test]
    fn get_lib_symbol() {
        let schema = Schema::load(Path::new("tests/summe.kicad_sch")).unwrap();
        let symbol = schema.symbol("U1", 1).unwrap();
        let lib_symbol = schema.library_symbol(&symbol.lib_id).unwrap();
        assert_eq!(
            "Reference_Voltage:LM4040DBZ-5".to_string(),
            lib_symbol.lib_id
        );
    }

    #[test]
    fn get_lib_symbol_unit() {
        let schema = Schema::load(Path::new("tests/summe.kicad_sch")).unwrap();
        let symbol = schema.symbol("U1", 1).unwrap();
        let lib_symbol = schema.library_symbol(&symbol.lib_id).unwrap();

        let mut iter = lib_symbol.units.iter();
        let first = iter.next().unwrap();
        assert_eq!(0, first.unit());
        assert_eq!(1, first.style());

        let second = iter.next().unwrap();
        assert_eq!(1, second.unit());
        assert_eq!(1, second.style());
    }
}
