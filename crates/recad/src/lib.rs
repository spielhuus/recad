pub use types::error::RecadError;
pub use types::gr::{Pt, Pts, Rect, Pos, Color};

pub use models::schema::Schema;
pub use models::pcb::Pcb;

pub mod draw {
    pub use draw::{At, Attribute, DotPosition, Drawer, Drawable, LabelPosition, SchemaBuilder, Symbol, Wire, Junction, Direction, LocalLabel, Feedback, GlobalLabel, NoConnect};
}

pub mod plot {
    pub use plot::{Plot, PlotCommand, Plotter, GerberPlotter, theme::{Theme, Themes}};
    #[cfg(feature = "svg")]
    pub use plot::SvgPlotter;
    #[cfg(feature = "wgpu")]
    pub use plot::WgpuPlotter;
}

pub mod schema {
    pub use models::schema::{LocalLabel, Junction, SchemaItem, Symbol};
    pub use models::symbols::{LibrarySymbol, ElectricalTypes, Pin, PinProperty};
}

pub mod pcb {
    pub use models::pcb::{FootprintType, LayerType};
}

pub mod simulation {
    pub use netlist::circuit::Circuit;
    pub use simulation::Simulation;
}

pub mod reports {
    pub use reports::{bom::bom, BomItem};
    pub use reports::erc::{erc, Erc, ERCViolation, ERCLevel};
    pub use reports::drc::{drc, Drc, DRCViolation, DRCLevel};
}

pub mod netlist {
    pub use netlist::{Netlist, CircuitGraph};
}

