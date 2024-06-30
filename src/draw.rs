//!Drawers for building schemas.
use std::path::PathBuf;

use crate::{
    gr::{Effects, Pos, Pt, Pts, Stroke}, math, schema, sexp::constants::el, Drawer, Schema
};

///Attributes for the elements.
#[derive(Debug, Clone)]
pub enum Attribute {
    Anchor(String),
    Direction(Direction),
    Id(String),
    Mirror(String),
    Length(f64),
    Rotate(f64),
    Tox(At),
    Toy(At),
    Property(String),
    Dot(Vec<DotPosition>),
}

///Dot position
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DotPosition {
    Start,
    End,
}

///Direction enum
#[derive(Debug, Clone)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

///Draw a Wire from the actual posistion to position.
#[derive(Debug, Clone)]
pub struct To {
    ///The Attributes.
    pub attributes: Vec<Attribute>,
}

impl To {
    ///Create a new empty To.
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
        }
    }

    pub fn push(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }

    ///Get the Wire length.
    pub fn length(&self) -> Option<f64> {
        for i in &self.attributes {
            if let Attribute::Length(length) = i {
                return Some(*length);
            }
        }
        None
    }
    ///Get the direction.
    pub fn direction(&self) -> &Direction {
        for i in &self.attributes {
            if let Attribute::Direction(direction) = i {
                return direction;
            }
        }
        &Direction::Left
    }
    ///Get the tox position.
    pub fn tox(&self) -> Option<&At> {
        for i in &self.attributes {
            if let Attribute::Tox(at) = i {
                return Some(at);
            }
        }
        None
    }
    ///Get the toy position.
    pub fn toy(&self) -> Option<&At> {
        for i in &self.attributes {
            if let Attribute::Toy(at) = i {
                return Some(at);
            }
        }
        None
    }
    //Get the dot positions.
    pub fn dot(&self) -> Option<&Vec<DotPosition>> {
        for i in &self.attributes {
            if let Attribute::Dot(dot) = i {
                return Some(dot);
            }
        }
        None
    }
}

impl Default for To {
    fn default() -> Self {
        Self::new()
    }
}

///Represents different position identifiers
///
///Points can be different things.
///- the coordinates of a point.
///- the coordinates of a pin.
///- The coordinates of a previous element.
#[derive(Debug, Clone, PartialEq)]
pub enum At {
    ///A simple point with x and y in mm.
    Pt(Pt),
    ///The posiition of a ```Pin``` by refernce and pin number.
    Pin(String, String),
    ///TODO
    Dot(String),
}

impl Default for At {
    fn default() -> Self {
        At::Pt(Pt { x: 0.0, y: 0.0 })
    }
}

///implment the drawer functions for the schema.
impl Schema {
    ///Move the cursor position to the pt.
    pub fn move_to(mut self, pt: At) -> Self {
        self.last_pos = pt;
        self
    }

    ///Resolve the At position to a Pt
    fn get_pt(&self, at: &At) -> Pt {
        match at {
            At::Pt(pt) => *pt,
            At::Pin(_, _) => todo!(),
            At::Dot(_) => todo!(),
        }
    }
}

pub struct Label {
    pub text: String,
    pub angle: f32,
}

impl Label {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            angle: 0.0,
        }
    }
    pub fn rotate(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }
}

impl Drawer<Label, Schema> for Schema {
    fn draw(mut self, label: Label) -> Schema {
        let pt = self.get_pt(&self.last_pos);
        let label = schema::LocalLabel {
            text: label.text.to_string(),
            pos: Pos {
                x: pt.x,
                y: pt.y,
                angle: label.angle,
            },
            effects: Effects::default(),
            color: None,
            uuid: crate::uuid!(),
            fields_autoplaced: true,
        };
        self.local_labels.push(label);
        self
    }
}

pub struct Dot {}

impl Dot {
    pub fn new() -> Self {
        Self {}
    }
}

impl Drawer<Dot, Schema> for Schema {
    fn draw(mut self, dot: Dot) -> Schema {
        let pt = self.get_pt(&self.last_pos);
        let dot = schema::Junction {
            pos: Pos {
                x: pt.x,
                y: pt.y,
                angle: 0.0,
            },
            diameter: 0.0,
            color: None,
            uuid: crate::uuid!(),
        };
        self.junctions.push(dot);
        self
    }
}

pub struct Wire {
    len: f32,
    attrs: To,
}

impl Wire {
    pub fn new() -> Self {
        Self {
            len: 2.54,
            attrs: To::new(),
        }
    }
}

impl Wire {
    pub fn len(mut self, len: f32) -> Self {
        self.len = len;
        self
    }
    pub fn up(mut self) -> Self {
        self.attrs.push(Attribute::Direction(Direction::Up));
        self
    }
    pub fn down(mut self) -> Self {
        self.attrs.push(Attribute::Direction(Direction::Down));
        self
    }
    pub fn left(mut self) -> Self {
        self.attrs.push(Attribute::Direction(Direction::Left));
        self
    }
    pub fn right(mut self) -> Self {
        self.attrs.push(Attribute::Direction(Direction::Right));
        self
    }
}

impl Drawer<Wire, Schema> for Schema {
    fn draw(mut self, wire: Wire) -> Schema {
        let pt = self.get_pt(&self.last_pos);
        let to_pos = match wire.attrs.direction() {
            Direction::Left => Pt {
                x: pt.x - wire.len * self.grid,
                y: pt.y,
            },
            Direction::Right => Pt {
                x: pt.x + wire.len * self.grid,
                y: pt.y,
            },
            Direction::Up => Pt {
                x: pt.x,
                y: pt.y - wire.len * self.grid,
            },
            Direction::Down => Pt {
                x: pt.x,
                y: pt.y + wire.len * self.grid,
            },
        };

        let wire = schema::Wire {
            pts: Pts(vec![pt, to_pos]),
            stroke: Stroke::default(),
            uuid: crate::uuid!(),
        };

        self.wires.push(wire);
        self.last_pos = At::Pt(to_pos);
        self
    }
}

pub struct Symbol {
    pub reference: String,
    pub value: String,
    pub lib_id: String,
    pub unit: u8,
    pub angle: f32,
    pub mirror: Option<String>,
    pub anchor: String,
    pub attrs: To,
}

impl Symbol {
    pub fn new(reference: &str, value: &str, lib_id: &str) -> Self {
        Self {
            reference: reference.to_string(),
            value: value.to_string(),
            lib_id: lib_id.to_string(),
            unit: 1,
            angle: 0.0,
            mirror: None,
            anchor: String::from("1"),
            attrs: To::new(),
        }
    }
    pub fn rotate(mut self, angle: f32) -> Self {
        self.angle = angle;
        self
    }
    pub fn mirror(mut self, mirror: &str) -> Self {
        self.mirror = Some(mirror.to_string());
        self
    }
    pub fn anchor(mut self, pin: &str) -> Self {
        self.anchor = pin.to_string();
        self
    }
    pub fn unit(mut self, unit: u8) -> Self {
        self.unit = unit;
        self
    }
}

impl Symbol {
    //pub fn len(mut self, len: f32) -> Self {
    //    self.len = len;
    //    self
    //}
    //pub fn up(mut self) -> Self {
    //    self.attrs.push(Attribute::Direction(Direction::Up));
    //    self
    //}
    //pub fn down(mut self) -> Self {
    //    self.attrs.push(Attribute::Direction(Direction::Down));
    //    self
    //}
    //pub fn left(mut self) -> Self {
    //    self.attrs.push(Attribute::Direction(Direction::Left));
    //    self
    //}
    //pub fn right(mut self) -> Self {
    //    self.attrs.push(Attribute::Direction(Direction::Right));
    //    self
    //}
}

impl Drawer<Symbol, Schema> for Schema {
    fn draw(mut self, symbol: Symbol) -> Schema {
        //load the library symbol
        let lib = if let Some(lib) = self.library_symbol(&symbol.lib_id) {
            lib.clone()
        } else {
            let lib = crate::SymbolLibrary {
                //TODO not finished
                pathlist: vec![PathBuf::from("/usr/share/kicad/symbols")],
            }
            .load(&symbol.lib_id)
            .unwrap();
            self.library_symbols.push(lib.clone());
            lib
        };

        //create the new symbol
        let mut new_symbol = lib.symbol(symbol.unit);
        new_symbol.pos.angle = symbol.angle;

        //create the transformer
        let pin_pos = crate::math::pin_position(&new_symbol, lib.pin(&symbol.anchor).unwrap());

        //calculate position
        let pt = self.get_pt(&self.last_pos);
        let start_pt = Pt { x: pt.x - pin_pos.x, y: pt.y - pin_pos.y };

        new_symbol.pos.x = start_pt.x;
        new_symbol.pos.y = start_pt.y;

        //set the properties
        new_symbol.set_property(el::PROPERTY_REFERENCE, &symbol.reference);
        new_symbol.set_property(el::PROPERTY_VALUE, &symbol.value);

        //create the pins
        for pin in &lib.pins(symbol.unit) {
            new_symbol
                .pins
                .push((pin.number.name.clone(), crate::uuid!()));
        }

        math::place_properties(&lib, &mut new_symbol);

        //TODO the next pin should be pin 2
        self.last_pos = At::Pt(crate::math::pin_position(
            &new_symbol,
            lib.pin("2").unwrap(),
        ));
        self.symbols.push(new_symbol);
        self
    }
}
