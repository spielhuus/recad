use std::fmt;

#[derive(Debug, Eq, PartialEq)]
pub enum RecadError {
    Sexp {
        line: usize,
        col: usize,
        msg: String,
    },
    Spice(String),
    Pcb(String),
    Io(String),
    Writer(String),
    Plotter(String),
    Schema(String),
    NgSpice(String),
    Font(String),
}

impl fmt::Display for RecadError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RecadError::Sexp { line, col, msg } => write!(f, "SexpError:{}:{} {}", line, col, msg),
            RecadError::Pcb(msg) => write!(f, "PCBError: {}", msg),
            RecadError::Spice(msg) => write!(f, "SpiceError: {}", msg),
            RecadError::Io(msg) => write!(f, "IoError: {}", msg),
            RecadError::Writer(msg) => write!(f, "WriterError: {}", msg),
            RecadError::Plotter(msg) => write!(f, "PlotterError: {}", msg),
            RecadError::Schema(msg) => write!(f, "SchemaError: {}", msg),
            RecadError::NgSpice(msg) => write!(f, "NgSpiceError: {}", msg),
            RecadError::Font(msg) => write!(f, "FontError: {}", msg),
        }
    }
}

impl std::error::Error for RecadError {}

impl From<std::io::Error> for RecadError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e.to_string())
    }
}
