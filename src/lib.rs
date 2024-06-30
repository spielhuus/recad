use std::path::{Path, PathBuf};

use {
    pcb::{Footprint, Layer, Net, Segment},
    schema::LibrarySymbol,
    sexp::{parser::SexpParser, SexpTree},
};

pub mod draw;
pub mod gr;
mod math;
pub mod netlist;
pub mod pcb;
pub mod plot;
pub mod schema;
mod schema_reader;
mod schema_writer;
mod schema_ploter;
mod sexp;

///create an UUID.
#[macro_export]
macro_rules! uuid {
    () => {
        uuid::Uuid::new_v4().to_string()
    };
}

fn round(n: f32) -> f32 {
    format!("{:.2}", n).parse().unwrap()
}

fn yes_or_no(input: bool) -> String {
    if input {
        String::from(el::YES)
    } else {
        String::from(el::NO)
    }
}

///The Error struct used for all error handling.
#[derive(Debug)]
pub struct Error(pub String, pub String);

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Self("io".to_string(), e.to_string())
    }
}

#[derive(Debug, Default)]
///Define the `Schematic` file format.
///
///Open a schematic file from a path.
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
pub struct Schema {
    ///The ```version``` token attribute defines the schematic version
    ///using the YYYYMMDD date format.<br><br>
    pub version: String,
    ///The ```uuid``` defines the universally unique identifier for
    ///the schematic file.<br><br>
    pub uuid: String,
    ///The ```generator``` token attribute defines the program used
    ///to write the file.<br><br>
    pub generator: String,
    ///The ```generator_version``` token attribute defines the
    ///program version used to write the file.<br><br>
    pub generator_version: Option<String>,
    pub paper: gr::PaperSize,
    pub title_block: gr::TitleBlock,
    pub library_symbols: Vec<schema::LibrarySymbol>,
    pub junctions: Vec<schema::Junction>,
    pub no_connects: Vec<schema::NoConnect>,
    pub wires: Vec<schema::Wire>,
    //pub wires_and_buses: Vec<WireAndBus>,
    //pub images: Vec<Image>,
    //pub graphical_lines: Vec<GraphicalLine>,
    pub graphical_texts: Vec<Text>,
    pub local_labels: Vec<schema::LocalLabel>,
    pub global_labels: Vec<schema::GlobalLabel>,
    pub symbols: Vec<schema::Symbol>,
    pub busses: Vec<Bus>,
    pub bus_entries: Vec<BusEntry>,
    pub polylines: Vec<Polyline>,
    //pub hierarchical_sheets: Vec<HierarchicalSheet>,
    //pub root_sheet_instances: Vec<RootSheetInstance>,
    
    ///attributes for the builder.
    grid: f32,
    last_pos: draw::At,
}

///Pcb file format for all versions of KiCad from 6.0.
#[derive(Default)]
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
    //
    //General
    //
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
    //
    //Groups
}

impl Pcb {
    ///Load a pcb from a path
    pub fn load(path: &Path) -> Self {
        let parser = crate::sexp::parser::SexpParser::load(path).unwrap();
        let tree = crate::sexp::SexpTree::from(parser.iter()).unwrap();
        tree.into()
    }
}

///implement the symbol lirarary.
pub struct SymbolLibrary {
    pathlist: Vec<PathBuf>,
}

use plot::{theme::Theme, Plotter};
use schema::{Bus, BusEntry, Polyline, Text};
use sexp::{constants::el, SexpValue};

impl SymbolLibrary {
    ///Load a symbol from the symbol library, the name is the combination
    ///of the filename of the library and the symbol name. 
    pub fn load(&self, name: &str) -> Result<LibrarySymbol, Error> {
        let t: Vec<&str> = name.split(':').collect();
        for path in &self.pathlist {
            let filename = &format!("{}/{}.kicad_sym", path.to_str().unwrap(), t[0]);
            if let Ok(doc) = SexpParser::load(Path::new(filename)) {
                if let Ok(tree) = SexpTree::from(doc.iter()) {
                    for node in tree.root().unwrap().query(el::SYMBOL) {
                        let sym_name: String = node.get(0).unwrap();
                        if sym_name == t[1] {
                            let mut node: LibrarySymbol = Into::<Result<LibrarySymbol, Error>>::into(node)?;
                            node.lib_id = format!("{}:{}", t[0], t[1]);
                            return Ok(node);
                        }
                    }
                }
            }
        }
        Err(Error(
            String::from("load_library"),
            format!("can not find library: {}", name),
        ))
    }
}

///Creat a schema or pcb file from code.
pub trait Drawer<T, F> {
    fn draw(self, item: T) -> F;
}

//#[derive(Debug, PartialEq, PartialOrd, Clone)]
//pub struct ImageCommand {
//    pub filename: String,
//}
//
//impl ImageCommand {
//    pub fn new<S>(filename: S) -> Self
//    where
//        S: Into<String>,
//    {
//        Self {
//            filename: filename.into(),
//        }
//    }
//
//    pub(crate) fn command(&self) -> Result<Vec<u8>, Error> {
//        let mut image_command = Vec::new();
//        for c in self.filename.chars() {
//            if (c == '"') || (c == '\\') || (c == '\n') {
//                return Err(Error(String::from("image"), String::from("Invalid character found!".to_string())));
//            }
//            image_command.push(c);
//        }
//
//        Ok(image_command)
//    }
//}


pub trait Plot {
    fn plot(self, plotter: &mut impl Plotter, theme: &Theme) -> Result<(), Error>;
}
