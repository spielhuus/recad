use std::path::{Path, PathBuf};

use {
    pcb::{Footprint, Layer, Net, Segment},
    symbols::LibrarySymbol,
    sexp::{parser::SexpParser, SexpTree},
};

pub mod draw;
pub mod gr;
mod math;
mod netlist;
pub mod pcb;
pub mod plot;
pub mod schema;
pub mod footprint;
pub mod symbols;
mod symbols_reader;
mod symbols_writer;
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
    pub library_symbols: Vec<LibrarySymbol>,

    pub items: Vec<SchemaItem>,
    pub sheet_instances: Vec<Instance>,
    
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
use schema::{Instance, SchemaItem};
use sexp::{builder::Builder, constants::el, SexpValue};

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

pub trait Plot {
    fn plot(self, plotter: &mut impl Plotter, theme: &Theme) -> Result<(), Error>;
}

trait SexpWrite {
    fn write(&self, builder: &mut Builder) -> Result<(), Error>;
}
