use std::fmt::Display;

use crate::{gr::{Effects, Pos, Property}, schema::Symbol, sexp::constants::el};


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

/// A struct representing a pin in a symbol definition.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pin {
    /// The electrical type of the pin.
    pub electrical_type: ElectricalTypes,
    /// The graphical style for the pin.
    pub graphical_style: PinGraphicalStyle,
    /// The position of the connection point relative to the symbol origin.
    pub pos: Pos,
    /// The length of the pin.
    pub length: f32,
    /// Whether the pin is hidden or not.
    pub hide: bool,
    /// The name and text effects for the pin.
    pub name: PinProperty,
    /// The number and text effects for the pin.
    pub number: PinProperty,
}

/// Properties of a schematic pin.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PinProperty {
    /// The name of the property associated with the pin.
    pub name: String,
    /// Defines the visual effects applied to the label (e.g., font style, shadow).
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
