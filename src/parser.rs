//! Library to parse and write the Kicad sexp files.
//!
//! The library provides low level acces to the sexp nodes, no model for the kicad data is provided.
//!
//! # Examples
//!
//! ```
//! // load a Kicad schema and access the root node:
//! use sexp::{SexpParser, SexpTree};
//! let doc = SexpParser::load("tests/summe.kicad_sch").unwrap();
//! let tree = SexpTree::from(doc.iter()).unwrap();
//! let root = tree.root().unwrap();
//! assert_eq!("kicad_sch", root.name);
//!
//! // get all symbol elements from the document:
//! use sexp::{el, Sexp};
//! let symbols = root.query(el::SYMBOL).collect::<Vec<&Sexp>>();
//! assert_eq!(151, symbols.len());
//!
//! // find the symbol with the reference "R1":
//! use sexp::SexpProperty;
//! let symbol = tree
//!     .root()
//!     .unwrap()
//!     .query(el::SYMBOL)
//!     .find(|s| {
//!         let name: String = s.property(el::PROPERTY_REFERENCE).unwrap();
//!         name == "R1"
//!     })
//!     .unwrap();
//! assert_eq!(String::from("10"),
//!             <Sexp as SexpProperty<String>>::property(
//!                 symbol,
//!                 el::PROPERTY_VALUE
//!             ).unwrap());
//! ```
//! create a document:
//!
//! ```
//! use sexp::{sexp, Builder};
//! let mut tree = sexp!((kicad_sch (version {sexp::KICAD_SCHEMA_VERSION}) (generator "elektron")
//!     (uuid "e91be4a5-3c12-4daa-bee2-30f8afcd4ab8")
//!     (paper r"A4")
//!     (lib_symbols)
//! ));
//! let root = tree.root().unwrap();
//! assert_eq!("kicad_sch", root.name);
//! ```
//! In the last example a sexp model was created. The macro is straight forward.
//! sexp supports string and quoted string. To define quoted steings directly when you
//! define raw strings. When a quoted string should be created this can either be done
//! with a raw String (`r"some text"`) or when it is created from a variable with a
//! bang (`!{variable}`, `!{func(param)}`).

/// Parse and access sexp files.
use std::{fs, io::Write, str::CharIndices};

use ndarray::{arr1, Array1, Array2};

pub mod math;
pub mod schema;

use pathfinder_geometry::vector::Vector2F;
pub use schema::Schema;

///Kicad schema file version
pub const KICAD_SCHEMA_VERSION: &str = "20211123";
///Kicad schema generator name.
pub const KICAD_SCHEMA_GENERATOR: &str = "elektron";

///create an UUID.
#[macro_export]
macro_rules! uuid {
    () => {
        Uuid::new_v4().to_string().as_str()
    };
}

///Enum of sexp error results.
#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    ///Can not manipulate file.
    #[error("{0}:{1}")]
    SexpError(String, String),
    #[error("Can not load content: '{0}' ({1})")]
    IoError(String, String),
    #[error("Library not found {0}.")]
    LibraryNotFound(String),
}
impl std::convert::From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(String::from("io::Error"), err.to_string())
    }
}

///Graphical styles for a symbol pin.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PinGraphicalStyle {
    Line,
    Inverted,
    Clock,
    InvertedClock,
    InputLow,
    ClockLow,
    OutputLow,
    EdgeClockHigh,
    NonLogic,
}

///Get the pin graphical style from String.
impl std::convert::From<String> for PinGraphicalStyle {
    fn from(pin_type: String) -> Self {
        if pin_type == "line" {
            Self::Line
        } else if pin_type == "inverted" {
            Self::Inverted
        } else if pin_type == "clock" {
            Self::Clock
        } else if pin_type == "inverted_clock" {
            Self::InvertedClock
        } else if pin_type == "input_low" {
            Self::InputLow
        } else if pin_type == "clock_low" {
            Self::ClockLow
        } else if pin_type == "output_low" {
            Self::OutputLow
        } else if pin_type == "edge_clock_high" {
            Self::EdgeClockHigh
        } else if pin_type == "non_logic" {
            Self::NonLogic
        } else {
            panic!(
                "PinGraphicalStyle::from: unknown graphial pin style {}",
                pin_type
            );
        }
    }
}

///Display pin graphical style.
impl std::fmt::Display for PinGraphicalStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            PinGraphicalStyle::Line => write!(f, "line")?,
            PinGraphicalStyle::Inverted => write!(f, "inverted")?,
            PinGraphicalStyle::Clock => write!(f, "clock")?,
            PinGraphicalStyle::InvertedClock => write!(f, "inverted_clock")?,
            PinGraphicalStyle::InputLow => write!(f, "input_low")?,
            PinGraphicalStyle::ClockLow => write!(f, "clock_low")?,
            PinGraphicalStyle::OutputLow => write!(f, "output_low")?,
            PinGraphicalStyle::EdgeClockHigh => write!(f, "edge_clock_high")?,
            PinGraphicalStyle::NonLogic => write!(f, "non_logic")?,
        };
        Ok(())
    }
}

///the types of a sexp atom.

///Utility methods to access some common nodes.
pub mod utils {
    use super::{el, Sexp, SexpAtom, SexpParser, SexpTree, SexpValueQuery};
    use crate::Error;
    use lazy_static::lazy_static;
    use ndarray::{s, Array1};
    use regex::Regex;

    ///get the position from the at node.
    pub fn at(element: &Sexp) -> Option<Array1<f64>> {
        Some(
            <Sexp as SexpValueQuery<Array1<f64>>>::value(element, el::AT)
                .unwrap()
                .slice_move(s![0..2]),
        )
    }

    ///get the angle from the at node.
    pub fn angle(element: &Sexp) -> Option<f64> {
        element.query(el::AT).next().unwrap().get(2)
    }

    lazy_static! {
        static ref RE: regex::Regex = Regex::new(r"^.*_(\d*)_(\d*)$").unwrap();
    }

    /// extract the unit number from the subsymbol name
    pub fn unit_number(name: String) -> usize {
        if let Some(line) = RE.captures_iter(&name).next() {
            line[1].parse().unwrap()
        } else {
            0
        }
    }

    ///get a pin of a library symbol.
    pub fn pin<'a>(root: &'a Sexp, number: &str) -> Option<&'a Sexp> {
        for _unit in root.query(el::SYMBOL) {
            for pin in _unit.query(el::PIN) {
                let n: String = pin.query(el::PIN_NUMBER).next().unwrap().get(0).unwrap();
                if number == n {
                    return Some(pin);
                }
            }
        }
        None
    }

    ///get all the pins of a library symbol.
    pub fn pins(root: &Sexp, unit: usize) -> Result<Vec<&Sexp>, Error> {
        let mut items: Vec<&Sexp> = Vec::new();
        for _unit in root.query(el::SYMBOL) {
            let number = unit_number(_unit.get(0).unwrap());
            if unit == 0 || number == 0 || number == unit {
                for pin in _unit.query(el::PIN) {
                    items.push(pin);
                }
            }
        }
        if items.is_empty() {
            let name: String = root.get(0).unwrap();
            Err(Error::SexpError(name.clone(), unit.to_string()))
        } else {
            Ok(items)
        }
    }

    ///get the library from the schema document.
    pub fn get_library<'a>(root: &'a Sexp, lib_id: &str) -> Option<&'a Sexp> {
        let libraries: &Sexp = root.query(el::LIB_SYMBOLS).next().unwrap();
        let lib: Vec<&Sexp> = libraries
            .nodes()
            .filter(|l| {
                let identifier: String = l.get(0).unwrap();
                identifier == lib_id
            })
            .collect();
        if lib.len() == 1 {
            Some(lib.first().unwrap())
        } else {
            None
        }
    }

    /// load a library
    ///
    /// # Arguments
    ///
    /// * `name`     - The symbol name.
    /// * `pathlist` - List of library paths.
    /// * `return`   - Library symbol as Sexp struct.
    pub fn library(name: &str, pathlist: Vec<String>) -> Result<Sexp, Error> {
        let t: Vec<&str> = name.split(':').collect();
        for path in &pathlist {
            let filename = &format!("{}/{}.kicad_sym", path, t[0]);
            if let Ok(doc) = SexpParser::load(filename) {
                if let Ok(tree) = SexpTree::from(doc.iter()) {
                    for node in tree.root()?.query(el::SYMBOL) {
                        let sym_name: String = node.get(0).unwrap();
                        if sym_name == t[1] {
                            let mut node = node.clone();
                            node.set(0, SexpAtom::Text(name.to_string()))?;
                            return Ok(node.clone());
                        }
                    }
                }
            }
        }
        Err(Error::LibraryNotFound(name.to_string()))
    }
}

///call the sexp document builder macro.
#[macro_export]
macro_rules! sexp {
   ($($inner:tt)*) => {
       {
        use sexp_macro::parse_sexp;
        let mut document = Builder::new();
        parse_sexp!(document, $($inner)*);
        document.sexp().unwrap()
       }
    };
}
//pub use sexp;

//const NO_NEW_LINE: [&str; 13] = [
//    "at",
//    "pin_names",
//    "offset",
//    "in_bom",
//    "on_board",
//    "font",
//    "size",
//    el::JUSTIFY,
//    "lib_id",
//    "effects",
//    "width",
//    "type",
//    "length",
//];

///Write the document to a Write trait.
pub trait SexpWriter {
    fn write(&self, out: &mut dyn Write, indent: usize) -> Result<bool, Error>;
}

impl SexpWriter for Sexp {
    fn write(&self, out: &mut dyn Write, indent: usize) -> Result<bool, Error> {
        let mut has_new_line = false;
        let mut has_children = false;
        if indent > 0 {
            if NO_NEW_LINE.contains(&self.name.as_str()) {
                out.write_all(b" ")?;
            } else {
                has_new_line = true;
                out.write_all(b"\n")?;
                out.write_all("  ".repeat(indent).as_bytes())?;
            }
        }
        out.write_all(b"(")?;
        out.write_all(self.name.as_bytes())?;
        for node in &self.nodes {
            match node {
                SexpAtom::Node(node) => {
                    has_children |= node.write(out, indent + 1)?;
                }
                SexpAtom::Value(value) => {
                    out.write_all(b" ")?;
                    out.write_all(value.as_bytes())?;
                }
                SexpAtom::Text(value) => {
                    out.write_all(b" \"")?;
                    out.write_all(value.as_bytes())?;
                    out.write_all(b"\"")?;
                }
            }
        }
        if has_children && has_new_line {
            out.write_all(b"\n")?;
            out.write_all("  ".repeat(indent).as_bytes())?;
            out.write_all(b")")?;
        } else if indent == 0 {
            out.write_all(b"\n)\n")?;
        } else {
            out.write_all(b")")?;
        }
        Ok(has_new_line)
    }
}

#[cfg(test)]
mod tests {
    use crate::{sexp, SexpParser, SexpWriter, State};

    use super::Builder;

    #[test]
    fn check_index() {
        let doc = SexpParser::from(String::from(
            r#"(node value1 value2 "value 3" "value 4" "" "value \"four\"" endval)"#,
        ));
        let mut iter = doc.iter();
        let state = iter.next();
        assert_eq!(state, Some(State::StartSymbol("node")));

        let state = iter.next();
        assert_eq!(state, Some(State::Values("value1")));

        let state = iter.next();
        assert_eq!(state, Some(State::Values("value2")));

        let state = iter.next();
        assert_eq!(state, Some(State::Text("value 3")));

        let state = iter.next();
        assert_eq!(state, Some(State::Text("value 4")));

        let state = iter.next();
        assert_eq!(state, Some(State::Text("")));

        let state = iter.next();
        assert_eq!(state, Some(State::Text(r#"value \"four\""#)));

        let state = iter.next();
        assert_eq!(state, Some(State::Values("endval")));

        let state = iter.next();
        assert_eq!(state, Some(State::EndSymbol));
    }

    #[test]
    fn build_document() {
        /* (kicad_sch (version 20211123) (generator elektron)

            (uuid e91be4a5-3c12-4daa-bee2-30f8afcd4ab8)

            (paper "A4")
            (lib_symbols
            )
        )*/
        let mut document = Builder::new();
        document.push("kicad_sch");
        document.push("version");
        document.value("20211123");
        document.end();
        document.push("generator");
        document.value("elektron");
        document.end();
        document.push(el::UUID);
        document.value("e91be4a5-3c12-4daa-bee2-30f8afcd4ab8");
        document.end();
        document.push(el::PAPER);
        document.text("A4");
        document.end();
        document.push(el::LIB_SYMBOLS);
        document.end();

        document.end();
        let tree = document.sexp().unwrap();
        tree.root()
            .unwrap()
            .write(&mut std::io::stdout(), 0)
            .unwrap();
    }
    #[test]
    fn macro_document() {
        let tree = sexp!(("kicad_sch" ("version" {super::KICAD_SCHEMA_VERSION}) ("generator" "elektron")
            (el::UUID "e91be4a5-3c12-4daa-bee2-30f8afcd4ab8")
            (el::PAPER r"A4")
            (el::LIB_SYMBOLS)
        ));
        let mut writer: Vec<u8> = Vec::new();
        tree.root().unwrap().write(&mut writer, 0).unwrap();
        let result = std::str::from_utf8(&writer).unwrap();
        assert_eq!(String::from("(kicad_sch\n  (version 20211123)\n  (generator elektron)\n  (uuid e91be4a5-3c12-4daa-bee2-30f8afcd4ab8)\n  (paper \"A4\")\n  (lib_symbols)\n)\n"), result);
    }
    #[test]
    fn remove_element() {
        let mut tree = sexp!(("kicad_sch" ("version" {super::KICAD_SCHEMA_VERSION}) ("generator" "elektron")
            (el::UUID "e91be4a5-3c12-4daa-bee2-30f8afcd4ab8")
            (el::PAPER r"A4")
            (el::LIB_SYMBOLS)
        ));
        let mut writer: Vec<u8> = Vec::new();
        tree.root().unwrap().write(&mut writer, 0).unwrap();
        let result = std::str::from_utf8(&writer).unwrap();
        assert_eq!(String::from("(kicad_sch\n  (version 20211123)\n  (generator elektron)\n  (uuid e91be4a5-3c12-4daa-bee2-30f8afcd4ab8)\n  (paper \"A4\")\n  (lib_symbols)\n)\n"), result);

        tree.root_mut().unwrap().remove(el::PAPER).unwrap();
        let mut writer: Vec<u8> = Vec::new();
        tree.root().unwrap().write(&mut writer, 0).unwrap();
        let result = std::str::from_utf8(&writer).unwrap();
        assert_eq!(String::from("(kicad_sch\n  (version 20211123)\n  (generator elektron)\n  (uuid e91be4a5-3c12-4daa-bee2-30f8afcd4ab8)\n  (lib_symbols)\n)\n"), result);
    }
}
