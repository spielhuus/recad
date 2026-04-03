use sexp::{Sexp, SexpValue, SexpValueExt};
use types::{constants::el, error::RecadError, gr::Pos};

use crate::schema::Property;

mod footprint;
pub mod pcb;
pub mod schema;
pub mod geometry;
pub mod symbols;
pub mod transform;
pub mod library; 

// TODO where to place this
///create an UUID.
#[macro_export]
macro_rules! uuid {
    () => {
        uuid::Uuid::new_v4().to_string()
    };
}

fn properties(node: &Sexp) -> Result<Vec<Property>, RecadError> {
    node.query(el::PROPERTY)
        .map(|x| {
            Ok(Property {
                pos: Pos::try_from(x)?,
                key: x.require_get(0)?,
                value: x.require_get(1)?,
                effects: (x).try_into()?,
                hide: x.first(el::HIDE)?,
            })
        })
        .collect()
}
