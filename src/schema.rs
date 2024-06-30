//!Schema definition with all required fields and data types.

use std::{fmt::Display, path::Path};

use crate::{
    draw::At,
    gr::{Color, Effects, PaperSize, Pos, Property, Pt, Pts, Stroke, TitleBlock},
    sexp::constants::el,
    Error, Schema,
};

#[derive(Debug, Clone)]
pub struct Text {
    ///```Pos``` defines the X and Y coordinates of the junction.
    pub pos: Pos,
    ///The text to display.
    pub text: String,
    ///the text effects of the text.
    pub effects: Effects,
    ///is the text a simulation instruction. 
    ///This is not supported in recad and only 
    ///implemented to be compatible with KiCad
    pub exclude_from_sim: bool, 
    ///Universally unique identifier for the junction
    pub uuid: String,
}

///The junction token defines a junction in the schematic.
#[derive(Debug, Clone)]
pub struct Junction {
    ///```Pos``` defines the X and Y coordinates of the junction.
    pub pos: Pos,
    ///Diameter of the junction
    pub diameter: f32,
    pub color: Option<Color>,
    ///Universally unique identifier for the junction
    pub uuid: String,
}

///The wire tokens define wires in the schematic.
#[derive(Debug, Clone)]
pub struct Wire {
    ///```Pts``` defines the list of X and Y coordinates
    ///of start and end points of the wire
    pub pts: Pts,
    ///```Stroke``` defines how the wire or bus is drawn
    pub stroke: Stroke,
    /////Universally unique identifier for the wire
    pub uuid: String,
}

///The LocalLabel define LocalLabel in the schematic.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalLabel {
    ///Label text
    pub text: String,
    ///Position of the label
    pub pos: Pos,
    ///```Effects``` defines the effects of the label
    pub effects: Effects,
    ///Color of the label
    pub color: Option<Color>,
    ///Universally unique identifier for the label
    pub uuid: String,
    ///Are the fields automatically populated with the schematic default values
    pub fields_autoplaced: bool,
}

///The gloabal_label tokens define Global Label in the schematic.
#[derive(Debug, Clone, PartialEq)]
pub struct GlobalLabel {
    ///Label text
    pub text: String,
    ///The shape of the box.
    pub shape: Option<String>,
    ///Position of the label
    pub pos: Pos,
    ///```Effects``` defines the effects of the label
    pub effects: Effects,
    //Universally unique identifier for the label
    pub uuid: String,
    //TODO properties: Properties,
}

///The no_connect token defines a unused pin connection in the schematic.
#[derive(Debug, Clone)]
pub struct NoConnect {
    ///```Pos``` defines the X and Y coordinates of the no connect.
    pub pos: Pos,
    //Universally unique identifier for the no connect.
    pub uuid: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PinProperty {
    pub name: String,
    pub effects: Effects,
}
/// Enum representing the different types of electrical pins.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, PartialOrd)]
pub enum ElectricalTypes {
    /// Input pin is an input.
    Input,
    /// Output pin is an output.
    Output,
    /// Bidirectional pin can be both input and output.
    #[default]
    Bidirectional,
    /// Tri-state pin is a tri-state output.
    TriState,
    /// Passive pin is electrically passive.
    Passive,
    /// Free pin is not internally connected.
    Free,
    /// Unspecified pin does not have a specified electrical type.
    Unspecified,
    /// Power in pin is a power input.
    PowerIn,
    /// Power out pin is a power output.
    PowerOut,
    /// Open collector pin is an open collector output.
    OpenCollector,
    /// Open emitter pin is an open emitter output.
    OpenEmitter,
    /// No connect pin has no electrical connection.
    NoConnect,
}

impl From<&str> for ElectricalTypes {
    fn from(s: &str) -> Self {
        match s {
            "input" => Self::Input,
            "output" => Self::Output,
            "bidirectional" => Self::Bidirectional,
            "tri_state" => Self::TriState,
            "passive" => Self::Passive,
            "free" => Self::Free,
            "unspecified" => Self::Unspecified,
            "power_in" => Self::PowerIn,
            "power_out" => Self::PowerOut,
            "open_collector" => Self::OpenCollector,
            "open_emitter" => Self::OpenEmitter,
            el::NO_CONNECT => Self::NoConnect,
            _ => Self::Unspecified,
        }
    }
}

impl Display for ElectricalTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Input => "input",
            Self::Output => "output",
            Self::Bidirectional => "bidirectional",
            Self::TriState => "tri_state",
            Self::Passive => "passive",
            Self::Free => "free",
            Self::Unspecified => "unspecified",
            Self::PowerIn => "power_in",
            Self::PowerOut => "power_out",
            Self::OpenCollector => "open_collector",
            Self::OpenEmitter => "open_emitter",
            Self::NoConnect => el::NO_CONNECT,
        };
        write!(f, "{}", name)
    }
}

///Enum representing the different pin graphical styles in KiCad.
#[derive(Clone, Debug, Default, Eq, PartialEq, Hash, PartialOrd)]
pub enum PinGraphicalStyle {
    #[default]
    ///see: <img src="https://dev-docs.kicad.org/en/file-formats/sexpr-intro/images/pinshape_normal_16.png"/>
    Line,
    ///see: <img src="https://dev-docs.kicad.org/en/file-formats/sexpr-intro/images/pinshape_invert_16.png"/>
    Inverted,
    ///see: <img src="https://dev-docs.kicad.org/en/file-formats/sexpr-intro/images/pinshape_clock_normal_16.png"/>
    Clock,
    ///see: <img src="https://dev-docs.kicad.org/en/file-formats/sexpr-intro/images/pinshape_clock_invert_16.png"/>
    InvertedClock,
    ///see: <img src="https://dev-docs.kicad.org/en/file-formats/sexpr-intro/images/pinshape_active_low_input_16.png"/>
    InputLow,
    ///see: <img src="https://dev-docs.kicad.org/en/file-formats/sexpr-intro/images/pinshape_clock_active_low_16.png"/>
    ClockLow,
    ///see: <img src="https://dev-docs.kicad.org/en/file-formats/sexpr-intro/images/pinshape_active_low_output_16.png"/>
    OutputLow,
    /// see: <img src="https://dev-docs.kicad.org/en/file-formats/sexpr-intro/images/pinshape_clock_fall_16.png"/>
    EdgeClockHigh,
    ///see: <img src="https://dev-docs.kicad.org/en/file-formats/sexpr-intro/images/pinshape_nonlogic_16.png"/>
    NonLogic,
}

impl From<&str> for PinGraphicalStyle {
    fn from(s: &str) -> Self {
        match s {
            "line" => Self::Line,
            "inverted" => Self::Inverted,
            "clock" => Self::Clock,
            "inverted_clock" => Self::InvertedClock,
            "input_low" => Self::InputLow,
            "clock_low" => Self::ClockLow,
            "output_low" => Self::OutputLow,
            "edge_clock_high" => Self::EdgeClockHigh,
            "nonlogic" => Self::NonLogic,
            _ => Self::Line,
        }
    }
}

impl Display for PinGraphicalStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = match self {
            Self::Line => "line",
            Self::Inverted => "inverted",
            Self::Clock => "clock",
            Self::InvertedClock => "inverted_clock",
            Self::InputLow => "input_low",
            Self::ClockLow => "clock_low",
            Self::OutputLow => "output_low",
            Self::EdgeClockHigh => "edge_clock_high",
            Self::NonLogic => "nonlogic",
        };
        write!(f, "{}", name)
    }
}

///The pin token defines a pin in a symbol definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pin {
    ///The PIN_ELECTRICAL_TYPE defines the pin electrical connection.
    pub electrical_type: ElectricalTypes,
    ///The PIN_GRAPHICAL_STYLE defines the graphical style used to draw the pin.
    pub graphical_style: PinGraphicalStyle,
    ///The POSITION_IDENTIFIER defines the X and Y coordinates and rotation angle
    ///of the connection point of the pin relative to the symbol
    ///origin position. The only supported rotation angles for
    ///pins are 0, 90, 180, and 270 degrees.
    pub pos: Pos,
    ///The length token attribute defines the LENGTH of the pin.
    pub length: f32,
    ///The hide token attribute defines if the pin is hidden.
    pub hide: bool,
    ///The name token defines a quoted string containing the NAME of the pin
    ///and the TEXT_EFFECTS defines how the text is displayed.
    pub name: PinProperty,
    ///The number token defines a quoted string containing the NUMBER
    ///of the pin and the TEXT_EFFECTS defines how the text is displayed.
    pub number: PinProperty,
}

///The symbol token defines a symbol or sub-unit of a parent symbol
#[derive(Debug, Clone)]
pub struct LibrarySymbol {
    ///Each symbol must have a unique "LIBRARY_ID" for each top level symbol in the library
    ///or a unique "UNIT_ID" for each unit embedded in a parent symbol. Library identifiers
    ///are only valid it top level symbols and unit identifiers are on valid as unit symbols
    ///inside a parent symbol.
    pub lib_id: String,
    ///The optional extends token attribute defines the "LIBRARY_ID" of another symbol inside
    ///the current library from which to derive a new symbol. Extended symbols currently can
    ///only have different SYMBOL_PROPERTIES than their parent symbol.
    pub extends: Option<String>,
    ///The optional power token attribute defines if the symbol is a power source.
    pub power: bool,
    ///The optional pin_numbers token defines the visibility setting of the symbol pin numbers
    ///for the entire symbol. If not defined, the all of the pin numbers in the symbol are visible.
    pub pin_numbers: bool,
    ///The optional pin_names token defines the attributes for all of the pin names of the symbol.
    ///The optional offset token defines the pin name offset for all pin names of the symbol.
    ///If not defined, the pin name offset is 0.508mm (0.020"). If the pin_name token is not
    ///defined, the all symbol pins are shown with the default offset.
    pub pin_names: bool,
    ///The in_bom token, defines if a symbol is to be include in the bill of material output.
    ///The only valid attributes are yes and no.
    pub in_bom: bool,
    ///The on_board token, defines if a symbol is to be exported from the schematic to the
    ///printed circuit board. The only valid attributes are yes and no.
    pub on_board: bool,
    ///The exclude_from_sim token attribute determines if the symbol is exluded
    ///from simulation.
    pub exclude_from_sim: bool,
    ///The SYMBOL_PROPERTIES is a list of properties that define the symbol. The following
    ///properties are mandatory when defining a parent symbol:
    ///  "Reference",
    ///  "Value",
    ///  "Footprint",
    ///  and "Datasheet".
    ///All other properties are optional. Unit symbols cannot have any properties.
    pub props: Vec<Property>,
    ///The GRAPHIC ITEMS section is list of graphical
    ///  arcs, circles, curves, lines, polygons, rectangles
    ///and text that define the symbol drawing. This section can be empty if the
    ///symbol has no graphical items.
    pub graphics: Vec<crate::gr::GraphicItem>,
    ///The PINS section is a list of pins that are used by the symbol.
    ///This section can be empty if the symbol does not have any pins.
    pub pins: Vec<Pin>,
    pub pin_names_offset: Option<f32>,
    ///The optional UNITS can be one or more child symbol tokens embedded in a parent symbol.
    pub units: Vec<LibrarySymbol>,
    ///The optional unit_name token defines the display name of a subunit in the symbol
    ///editor and symbol chooser. It is only permitted for child symbol tokens embedded
    ///in a parent symbol.
    pub unit_name: Option<String>,
}

impl LibrarySymbol {
    ///"UNIT" is an integer that identifies which unit the symbol represents. A "UNIT"
    ///value of zero (0) indicates that the symbol is common to all units.
    pub fn unit(&self) -> u8 {
        let splits = self.lib_id.split('_').collect::<Vec<&str>>();
        splits.get(splits.len() - 2).unwrap().parse::<u8>().unwrap()
    }

    ///The "STYLE" indicates which body style the unit represents.
    pub fn style(&self) -> u8 {
        let splits = self.lib_id.split('_').collect::<Vec<&str>>();
        splits.last().unwrap().parse::<u8>().unwrap()
    }

    ///Get a Pin by the pin number
    pub fn pin(&self, number: &str) -> Option<&Pin> {
        for u in &self.units {
            for p in &u.pins {
                if p.number.name == number {
                    return Some(p);
                }
            }
        }
        None
    }
    ///Get all pins for a symbol unit
    pub fn pins(&self, unit: u8) -> Vec<&Pin> {
        let mut pins = Vec::new();
        for u in &self.units {
            if u.unit() == 0 || u.unit() == unit {
                for p in &u.pins {
                    pins.push(p);
                }
            }
        }
        pins
    }

    pub fn symbol(&self, unit: u8) -> Symbol {
        let mut symbol = Symbol {
            lib_id: self.lib_id.clone(),
            unit,
            in_bom: true,
            on_board: true,
            uuid: crate::uuid!(),
            ..Default::default()
        };

        //set properties
        for ls in &self.props {
            if !ls.key.starts_with("ki_") {
                symbol.props.push(ls.clone());
            }
        }
        symbol
    }
}

///The instances token defines a symbol instance. 
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Instance {
    pub project: String,
    pub path: String,
    pub reference: String,
    pub unit: u8,
}

///The symbol section of the schematic designates an instance of a symbol from 
///the library symbol section of the schematic.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Symbol {
    ///The LIBRARY_IDENTIFIER defines which symbol in the library symbol
    ///section of the schematic that this schematic symbol references.
    pub lib_id: String,
    ///The ```pos``` defines the X and Y coordinates and angle
    ///of rotation of the symbol.<br><br>
    pub pos: Pos,
    ///The MIRROR defines the if the symbol is mirrored. The only valid
    ///values are x, y, and xy.
    pub mirror: Option<String>,
    ///The unit token attribute defines which unit in the symbol library
    ///definition that the schematic symbol represents.
    pub unit: u8,
    ///The in_bom token attribute determines whether the schematic
    ///symbol appears in any bill of materials output.
    pub in_bom: bool,
    ///The on_board token attribute determines if the footprint associated
    ///with the symbol is exported to the board via the netlist.
    pub on_board: bool,
    ///The exclude_from_sim token attribute determines if the symbol is exluded
    ///from simulation.
    pub exclude_from_sim: bool,
    ///The DNP token attribute determines if the symbol is to be populated.
    pub dnp: bool,
    ///The UNIQUE_IDENTIFIER defines the universally unique identifier for the
    ///symbol. This is used to map the symbol the symbol instance information.
    pub uuid: String,
    ///The PROPERTIES section defines a list of symbol properties
    ///of the schematic symbol.
    pub props: Vec<Property>,
    ///The PINS section is a list of pins utilized by the symbol.
    ///This section may be empty if the symbol lacks any pins.
    pub pins: Vec<(String, String)>,
    ///The instances token defines a list of symbol instances grouped by project. 
    ///Every symbol has at least one instance.
    pub instances: Vec<Instance>,
    
    //The project token attribute defines the name of the project to which the instance data belongs. There can be instance data from other project when schematics are shared across multiple projects. The projects will be sorted by the PROJECT_NAME in alphabetical order.
    //The path token attribute is the path to the sheet instance for the instance data.
    //The reference token attribute is a string that defines the reference designator for the symbol instance.
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
            junctions: Vec::new(),
            no_connects: Vec::new(),
            graphical_texts: Vec::new(),
            wires: Vec::new(),
            local_labels: Vec::new(),
            global_labels: Vec::new(),
            symbols: Vec::new(),
            grid: 1.27,
            last_pos: At::Pt(Pt { x: 0.0, y: 0.0 }),
        }
    }

    ///Load a schema from a path
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
    pub fn symbol(&self, reference: &str, unit: u8) -> Option<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| s.property(el::PROPERTY_REFERENCE) == reference && s.unit == unit)
            .collect::<Vec<&Symbol>>()
            .first()
            .copied()
    }

    //Get a library symbol by lib_id
    pub fn library_symbol(&self, lib_id: &str) -> Option<&LibrarySymbol> {
        self.library_symbols
            .iter()
            .filter(|s| s.lib_id == lib_id)
            .collect::<Vec<&LibrarySymbol>>()
            .first()
            .copied()
    }
}

#[derive(Debug)]
pub struct SchemaIterator<'a> {
    items: Vec<SchemaItem<'a>>,
}

#[derive(Debug)]
pub enum SchemaItem<'a> {
    Junction(&'a Junction),
    NoConnect(&'a NoConnect),
    Wire(&'a Wire),
    LocalLabel(&'a LocalLabel),
    GlobalLabel(&'a GlobalLabel),
    Symbol(&'a Symbol),
}

impl<'a> Iterator for SchemaIterator<'a> {
    type Item = SchemaItem<'a>;
    ///Get the next node.
    fn next(&mut self) -> Option<Self::Item> {
        self.items.pop()
    }
}

impl Schema {
    pub fn iter(&self) -> SchemaIterator {
        let mut items = Vec::new();

        //Junction Section
        for junction in &self.junctions {
            items.push(SchemaItem::Junction(junction));
        }

        //No Connect Section
        for nc in &self.no_connects {
            items.push(SchemaItem::NoConnect(nc));
        }

        //Wire and Bus Section
        for wire in &self.wires {
            items.push(SchemaItem::Wire(wire));
        }

        //Image Section

        //Graphical Line Section

        //Graphical Text Section

        //Local Label Section
        for label in &self.local_labels {
            items.push(SchemaItem::LocalLabel(label));
        }

        //Global Label Section
        for label in &self.global_labels {
            items.push(SchemaItem::GlobalLabel(label));
        }

        //Symbol Section
        for symbol in &self.symbols {
            items.push(SchemaItem::Symbol(symbol));
        }

        SchemaIterator { items }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::Schema;

    #[test]
    fn symbol_property() {
        let schema = Schema::load(Path::new("tests/summe.kicad_sch")).unwrap();
        let symbol = schema.symbols.first().unwrap();
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
