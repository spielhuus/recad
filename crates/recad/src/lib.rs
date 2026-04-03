pub use types::error::RecadError;
pub use types::gr::{Pt, Pts, Rect, Pos, Color};

pub use models::schema::Schema;
pub use models::pcb::Pcb;

pub mod draw {
    pub use draw::{At, Attribute, DotPosition, Drawer, Drawable, LabelPosition, SchemaBuilder, Symbol, Wire, Junction, Direction, LocalLabel, Feedback, GlobalLabel, NoConnect};
}

pub mod plot {
    pub use plot::{Plot, PlotCommand, Plotter, SvgPlotter, WgpuPlotter, GerberPlotter, theme::{Theme, Themes}};
}

pub mod schema {
    pub use models::schema::LocalLabel;
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

// pub mod prelude {
//     pub use types::gr::{Pt, Rect, Pos, Color};
//     pub use types::error::RecadError;
//
//     pub use models::schema::Schema;
//     pub use models::pcb::Pcb;
//
//     pub use plot::{Plot, Plotter};
//     pub use models::geometry::Bbox;
// }
//

