use std::fmt::Display;

use sexp::{Sexp, SexpExt, SexpValue, SexpValueExt, SexpWrite, builder::Builder};
use types::{
    constants::el, error::RecadError, gr::{self, Curve, Effects, GraphicItem, Pos}, round, yes_or_no
};

use crate::{
    schema::{Symbol, Property},
};

fn sub_lib_id(input: &str) -> Result<String, RecadError> {
    // Find the position of the colon (':') in the input string
    if let Some(pos) = input.find(':') {
        Ok(input[pos + 1..].to_string())
    } else {
        Err(RecadError::Writer(format!(
            "can not find a colon in \"{}\"",
            input
        )))
    }
}

fn pin_names(node: &Sexp) -> Result<bool, RecadError> {
    // old style
    let hide: Result<Option<String>, RecadError> = node.first(el::PIN_NAMES);
    if let Ok(Some(hidden)) = hide {
        if hidden == el::HIDE {
            return Ok(true);
        }
    }

    // new style
    let Some(pin_name_node) = node.query(el::PIN_NAMES).next() else {
        return Ok(false);
    };

    let is_hidden = pin_name_node.first(el::HIDE)?.unwrap_or(false);
    Ok(is_hidden)
}

fn pin_numbers(node: &Sexp) -> Result<bool, RecadError> {
    // old style
    let hide: Result<Option<String>, RecadError> = node.first(el::PIN_NUMBERS);
    if let Ok(Some(hidden)) = hide {
        if hidden == el::HIDE {
            return Ok(true);
        }
    }

    // check for new style
    let Some(pin_num_node) = node.query(el::PIN_NUMBERS).next() else {
        return Ok(false);
    };
    let is_hidden = pin_num_node.first(el::HIDE)?.unwrap_or(false);
    Ok(is_hidden)
}

pub fn pin_names_offset(sexp: &Sexp) -> Result<Option<f64>, RecadError> {
    if let Some(names) = sexp.query(el::PIN_NAMES).next() {
        return names.first(el::OFFSET);
    }
    Ok(None)
}


///The symbol token defines a symbol or sub-unit of a parent symbol
#[derive(Debug, Clone, Default)]
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
    ///The exclude_from_sim token attribute determines if the symbol is excluded
    ///from simulation.
    pub exclude_from_sim: bool,
    /// TODO comment
    pub in_pos_files: bool,
    /// TODO comment
    pub duplicate_pin_numbers_are_jumpers: bool,
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
    pub graphics: Vec<GraphicItem>,
    ///The PINS section is a list of pins that are used by the symbol.
    ///This section can be empty if the symbol does not have any pins.
    pub pins: Vec<Pin>,
    pub pin_names_offset: Option<f64>,
    ///The optional UNITS can be one or more child symbol tokens embedded in a parent symbol.
    pub units: Vec<LibrarySymbol>,
    ///The optional unit_name token defines the display name of a subunit in the symbol
    ///editor and symbol chooser. It is only permitted for child symbol tokens embedded
    ///in a parent symbol.
    pub unit_name: Option<String>,
    //TODO data structure not known.
    pub embedded_fonts: Option<String>,
}

impl LibrarySymbol {
    ///  The `unit` refers to a numerical identifier denoting the specific unit the symbol
    ///  represents. A `unit` value of zero (0) implies that the
    ///  symbol is universal across all units.
    pub fn unit(&self) -> u8 {
        let splits = self.lib_id.split('_').collect::<Vec<&str>>();
        splits.get(splits.len() - 2).unwrap().parse::<u8>().unwrap()
    }

    ///The `style` indicates which body style the unit represents.
    pub fn style(&self) -> u8 {
        let splits = self.lib_id.split('_').collect::<Vec<&str>>();
        splits.last().unwrap().parse::<u8>().unwrap()
    }

    ///Get a `Pin` by the pin number
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

    ///Get the unit by the pin number
    pub fn pin_unit(&self, number: &str) -> Option<u8> {
        for u in &self.units {
            for p in &u.pins {
                if p.number.name == number {
                    return Some(u.unit());
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

    /// Returns the number of units this symbol has.
    /// It finds the maximum unit number among all embedded subunits.
    /// If there are no units or only universal units (unit 0), it defaults to 1.
    pub fn unit_count(&self) -> usize {
        let max_unit = self.units.iter().map(|u| u.unit()).max().unwrap_or(0);

        if max_unit == 0 {
            1
        } else {
            max_unit as usize
        }
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for LibrarySymbol {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(LibrarySymbol {
            lib_id: sexp.require_get(0)?,
            extends: sexp.first(el::EXTENDS)?,
            power: sexp.query(el::POWER).next().is_some(),
            exclude_from_sim: sexp.first(el::EXCLUDE_FROM_SIM)?.unwrap_or(false),
            in_bom: sexp.first(el::IN_BOM)?.unwrap_or(true),
            on_board: sexp.first(el::ON_BOARD)?.unwrap_or(true),
            in_pos_files: sexp.first(el::IN_POS_FILES)?.unwrap_or(true),
            duplicate_pin_numbers_are_jumpers: sexp.first(el::DUPLICATE_PIN_NUMBERS_ARE_JUMPERS)?.unwrap_or(true),

            props: crate::properties(sexp)?,
            graphics: sexp
                .nodes()
                .map(|node| -> Result<Option<GraphicItem>, Self::Error> {
                    match node.name.as_ref() {
                        el::ARC => Ok(Some(GraphicItem::Arc(node.try_into()?))),
                        el::CIRCLE => Ok(Some(GraphicItem::Circle(node.try_into()?))),
                        el::CURVE => Ok(Some(GraphicItem::Curve(Curve {
                            pts: node.try_into()?,
                            stroke: node.try_into()?,
                            fill: sexp.try_into()?,
                        }))),
                        el::POLYLINE => Ok(Some(GraphicItem::Polyline(node.try_into()?))),
                        el::LINE => Ok(Some(GraphicItem::Line(node.try_into()?))),
                        el::RECTANGLE => Ok(Some(GraphicItem::Rectangle(node.try_into()?))),
                        el::TEXT => Ok(Some(GraphicItem::Text(gr::Text {
                            text: node.require_get(0)?,
                            pos: Pos::try_from(node)?,
                            effects: node.try_into()?,
                            uuid: None,
                        }))),
                        el::EMBEDDED_FONTS => Ok(Some(GraphicItem::EmbeddedFont(node.require_get(0)?))),
                        _ => {
                            if node.name != el::PIN
                                && node.name != el::SYMBOL
                                && node.name != el::POWER
                                && node.name != el::PIN_NUMBERS
                                && node.name != el::PIN_NAMES
                                && node.name != el::IN_BOM
                                && node.name != el::ON_BOARD
                                && node.name != el::IN_POS_FILES
                                && node.name != el::DUPLICATE_PIN_NUMBERS_ARE_JUMPERS
                                && node.name != el::EXCLUDE_FROM_SIM
                                && node.name != el::PROPERTY
                                && node.name != el::EXTENDS
                            {
                                spdlog::warn!("unknown graphic type: {}", node.name);
                            }
                            Ok(None)
                        }
                    }
                })
                .filter_map(Result::transpose)
                .collect::<Result<Vec<_>, Self::Error>>()?,

            pins: sexp
                .nodes()
                .map(|node| -> Result<Option<Pin>, Self::Error> {
                    match node.name.as_ref() {
                        el::PIN => {
                            let electrical_type: String = node.require_get(0)?;
                            let graphical_style: String = node.require_get(1)?;
                            let length: f64 = node.require_first(el::LENGTH)?;
                            let name_node = node.require_node(el::NAME)?;
                            let number_node = node.require_node(el::NUMBER)?;
                            Ok(Some(Pin {
                                electrical_type: ElectricalTypes::from(electrical_type.as_str()),
                                graphical_style: PinGraphicalStyle::from(graphical_style.as_str()),
                                pos: Pos::try_from(node)?,
                                length,
                                hide: node.first(el::HIDE)?.unwrap_or(false),
                                name: PinProperty {
                                    name: name_node.require_get(0)?,
                                    effects: name_node.try_into()?,
                                },
                                number: PinProperty {
                                    name: number_node.require_get(0)?,
                                    effects: number_node.try_into()?,
                                },
                            }))
                        }
                        _ => Ok(None),
                    }
                })
                .filter_map(Result::transpose)
                .collect::<Result<Vec<_>, Self::Error>>()?,

            pin_numbers: pin_numbers(sexp)?,
            pin_names: pin_names(sexp)?,
            pin_names_offset: pin_names_offset(sexp)?,
            units: sexp
                .query(el::SYMBOL)
                .map(LibrarySymbol::try_from)
                .collect::<Result<Vec<LibrarySymbol>, RecadError>>()?,
            unit_name: sexp.first("unit_name")?, //TODO check name in sexp file.
            embedded_fonts: if let Some(ef) = sexp.query(el::EMBEDDED_FONTS).next() {
                let font: String = ef.require_get(0)?;
                Some(font)
            } else {
                None
            },
        })
    }
}

impl SexpWrite for LibrarySymbol {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::SYMBOL);
        builder.text(&self.lib_id);
        if self.power {
            builder.push(el::POWER);
            builder.end();
        }
        if self.pin_numbers {
            builder.push(el::PIN_NUMBERS);
            builder.push(el::HIDE);
            builder.value(yes_or_no(self.pin_numbers));
            builder.end();
            builder.end();
        }
        if let Some(offset) = self.pin_names_offset {
            builder.push(el::PIN_NAMES);
            builder.push(el::OFFSET);
            builder.value(offset);
            builder.end();
            if self.pin_names {
                builder.push(el::HIDE);
                builder.value(yes_or_no(self.pin_names));
                builder.end();
            }
            builder.end();
        } else if self.pin_names {
            builder.push(el::PIN_NAMES);
            builder.push(el::HIDE);
            builder.value(yes_or_no(self.pin_names));
            builder.end();
            builder.end();
        }
        builder.push(el::EXCLUDE_FROM_SIM);
        builder.value(yes_or_no(self.exclude_from_sim));
        builder.end();
        builder.push(el::IN_BOM);
        builder.value(yes_or_no(self.in_bom));
        builder.end();
        builder.push(el::ON_BOARD);
        builder.value(yes_or_no(self.on_board));
        builder.end();

        for p in &self.props {
            p.write(builder)?;
        }

        for subsymbol in &self.units {
            builder.push(el::SYMBOL);
            builder.text(format!(
                "{}_{}_{}",
                sub_lib_id(self.lib_id.trim_start_matches(':'))?,
                subsymbol.unit(),
                subsymbol.style()
            ));

            for graph in &subsymbol.graphics {
                match graph {
                    GraphicItem::Arc(a) => a.write(builder)?,
                    GraphicItem::Circle(c) => c.write(builder)?,
                    GraphicItem::Curve(_) => {} // TODO
                    GraphicItem::Line(_) => {}
                    GraphicItem::Polyline(p) => p.write(builder)?,
                    GraphicItem::Rectangle(r) => r.write(builder)?,
                    GraphicItem::Text(_) => {}
                    GraphicItem::EmbeddedFont(_) => {}
                }
            }
            for pin in &subsymbol.pins {
                pin.write(builder)?;
            }
            builder.end();
        }

        if let Some(embedded_fonts) = &self.embedded_fonts {
            builder.push(el::EMBEDDED_FONTS);
            builder.value(embedded_fonts);
            builder.end();
        }
        builder.end();
        Ok(())
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
    pub length: f64,
    /// Whether the pin is hidden or not.
    pub hide: bool,
    /// The name and text effects for the pin.
    pub name: PinProperty,
    /// The number and text effects for the pin.
    pub number: PinProperty,
}

impl SexpWrite for Pin {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::PIN);
        builder.value(self.electrical_type);
        builder.value(&self.graphical_style);
        builder.push(el::AT);
        builder.value(round(self.pos.x));
        builder.value(round(self.pos.y));
        builder.value(round(self.pos.angle));
        builder.end();
        builder.push(el::LENGTH);
        builder.value(self.length);
        builder.end();
        if self.hide {
            builder.push(el::HIDE);
            builder.value(el::YES);
            builder.end();
        }
        builder.push(el::NAME);
        builder.text(&self.name.name);
        self.name.effects.write(builder)?;
        builder.end();

        builder.push(el::NUMBER);
        builder.text(&self.number.name);
        self.number.effects.write(builder)?;
        builder.end();

        builder.end();

        Ok(())
    }
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
#[derive(Copy, Clone, Debug, Default, Eq, PartialEq, Hash, PartialOrd)]
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
            el::INPUT => Self::Input,
            el::OUTPUT => Self::Output,
            el::BIDIRECTIONAL => Self::Bidirectional,
            el::TRI_STATE => Self::TriState,
            el::PASSIVE => Self::Passive,
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
            Self::Input => el::INPUT,
            Self::Output => el::OUTPUT,
            Self::Bidirectional => el::BIDIRECTIONAL,
            Self::TriState => el::TRI_STATE,
            Self::Passive => el::PASSIVE,
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
