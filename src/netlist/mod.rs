use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    self as model,
    gr::Pt,
    schema::{self, SchemaItem},
    symbols::Pin,
    Error, Schema,
};

#[derive(Clone, Debug, PartialEq)]
pub enum NodePositions<'a> {
    Pin(Pt, &'a Pin, &'a schema::Symbol),
    Wire(Pt, Pt),
    Label(Pt, &'a schema::LocalLabel),
    GlobalLabel(Pt, &'a schema::GlobalLabel),
    NoConnect(Pt),
    Junction(Pt),
}

#[derive(Clone, Debug)]
pub struct Node {
    identifier: Option<String>,
    points: Vec<Pt>,
    // pins: Vec<Pin>,
}

// create a new node with values.
pub struct Netlist<'a> {
    //TODO schema: &'a crate::Schema,
    nodes: Vec<Node>,
    node_positions: Vec<(Pt, NodePositions<'a>)>,
}

impl<'a> Netlist<'a> {

    /** This function takes a reference to a [`Schema`] and returns a `HashMap<Pt, Pt>`.
    It iterates through the items in the schema, filtering only `Wire` items. For
    each [`schema::Wire`], it creates an entry in the map with the starting point as key
    and the ending point as value, and also creates a reciprocal entry
    to ensure bidirectionality. */
    fn wires(schema: &Schema) -> HashMap<Pt, Pt> {
        let mut wires: HashMap<Pt, Pt> = HashMap::new();
        schema
            .items
            .iter()
            .filter_map(|w| match w {
                SchemaItem::Wire(w) => Some(w),
                _ => None,
            })
            .for_each(|w| {
                let pt0 = w.pts.0[0];
                let pt1 = w.pts.0[1];
                wires.insert(pt0, pt1);
                wires.insert(pt1, pt0);
            });
        wires
    }

    pub fn from(schema: &'a crate::Schema) -> Result<Self, Error> {
        let wires = Netlist::wires(schema);

        let node_positions = Netlist::positions(schema)?;
        let mut netlist = Self {
            nodes: Vec::new(),
            node_positions,
        };

        //collect all the pins (Nodes)
        //let mut pins: HashMap<&schema::Pin, (Pt, &schema::Symbol)> = HashMap::new();
        //for symbol in &schema.symbols {
        //    let lib_symbol = schema.library_symbol(&symbol.lib_id).unwrap();
        //    for pin in &lib_symbol.pins {
        //        let pin_pos = model::math::pin_position(symbol, pin).ndarray();
        //        pins.insert(pin, (pin_pos, symbol));
        //    }
        //}

        let used_vec = &mut Vec::new();
        let used = &Rc::new(RefCell::new(used_vec));
        let mut used_pins: Vec<&NodePositions> = Vec::new();
        for pos in &netlist.node_positions {
            if let NodePositions::Pin(point, p, s) = &pos.1 {
                if !used_pins.contains(&&pos.1) {
                    used_pins.push(&pos.1);
                    used.borrow_mut().clear();
                    used.borrow_mut().push(&pos.1);

                    if let Some(nodes) = Netlist::next_node(&pos.0, &netlist.node_positions, used) {
                        let mut identifier: Option<String> = None;
                        let mut points: Vec<Pt> = vec![*point];
                        let mut pins: Vec<&Pin> = vec![p];
                        //if nodes.1.starts_with("power:") {
                        //    identifier = s.property(el::PROPERTY_VALUE);
                        //}
                        for node in &nodes {
                            match node {
                                NodePositions::Pin(point, p, s) => {
                                    if s.lib_id.starts_with("power:") {
                                        identifier = Some(s.lib_id.clone()[6..].to_string())
                                    }
                                    pins.push(p);
                                    points.push(*point);
                                    used_pins.push(node);
                                }
                                NodePositions::Junction(point) => {
                                    points.push(*point);
                                    used_pins.push(&pos.1);
                                }
                                NodePositions::Wire(_, p2) => {
                                    points.push(*point);
                                    points.push(*p2);
                                    used_pins.push(node);
                                }
                                NodePositions::NoConnect(point) => {
                                    points.push(*point);
                                    used_pins.push(node);
                                    identifier = Some(String::from("NC"));
                                }
                                NodePositions::Label(point, l) => {
                                    identifier = Some(l.text.clone());
                                    points.push(*point);
                                    used_pins.push(node);
                                }
                                NodePositions::GlobalLabel(point, l) => {
                                    identifier = Some(l.text.clone());
                                    points.push(*point);
                                    used_pins.push(node);
                                }
                            }
                        }
                        netlist.nodes.push(Node { identifier, points });
                    }
                }
            }
        }

        let mut name = 1;
        for n in &mut netlist.nodes {
            if n.identifier.is_none() {
                n.identifier = Some(name.to_string());
                name += 1;
            }
        }

        Ok(netlist)
    }

    ///get all the positions of the elements.
    fn positions(schema: &'a crate::Schema) -> Result<Vec<(Pt, NodePositions)>, Error> {
        let mut positions: Vec<(Pt, NodePositions)> = Vec::new();

        //colect elements and pins
        for item in &schema.items {
            match item {
                SchemaItem::Symbol(symbol) => {
                    if symbol.lib_id.starts_with("Mechanical:") {
                        continue;
                    }
                    schema
                        .library_symbol(&symbol.lib_id)
                        .into_iter()
                        .for_each(|l| {
                            for p in l.pins(symbol.unit) {
                                let pin_pos = model::math::pin_position(symbol, p);
                                positions.push((pin_pos, NodePositions::Pin(pin_pos, p, symbol)));
                            }
                        });
                }
                SchemaItem::NoConnect(nc) => {
                    let pt = Pt {
                        x: nc.pos.x,
                        y: nc.pos.y,
                    };
                    positions.push((pt, NodePositions::NoConnect(pt)));
                }
                SchemaItem::Junction(junction) => {
                    let pt = Pt {
                        x: junction.pos.x,
                        y: junction.pos.y,
                    };
                    positions.push((pt, NodePositions::NoConnect(pt)));
                }
                SchemaItem::LocalLabel(l) => {
                    let pt = Pt {
                        x: l.pos.x,
                        y: l.pos.y,
                    };
                    positions.push((pt, NodePositions::Label(pt, l)));
                }
                SchemaItem::GlobalLabel(l) => {
                    let pt = Pt {
                        x: l.pos.x,
                        y: l.pos.y,
                    };
                    positions.push((pt, NodePositions::GlobalLabel(pt, l)));
                }
                _ => {}
            }
        }
        Ok(positions)
    }

    ///Get the connected endpoints to this elements.
    fn next_node(
        pos: &'a Pt,
        elements: &'a Vec<(Pt, NodePositions)>,
        used: &Rc<RefCell<&'a mut Vec<&'a NodePositions<'a>>>>,
    ) -> Option<Vec<&'a NodePositions<'a>>> {
        for (p, e) in elements {
            if !used.borrow().contains(&e) {
                match e {
                    NodePositions::Label(_, _) => {
                        if p == pos {
                            used.borrow_mut().push(e);
                            let mut found_nodes: Vec<&'a NodePositions> = vec![e];
                            loop {
                                if let Some(nodes) = &Self::next_node(p, elements, used) {
                                    found_nodes.extend(nodes);
                                    used.borrow_mut().extend(nodes);
                                } else {
                                    return Some(found_nodes);
                                }
                            }
                        }
                    }
                    NodePositions::GlobalLabel(..) => {
                        if p == pos {
                            return Some(vec![e]);
                        }
                    }
                    NodePositions::Junction(..) => {
                        if p == pos {
                            used.borrow_mut().push(e);
                            let mut found_nodes: Vec<&'a NodePositions> = Vec::new();
                            loop {
                                if let Some(nodes) = &Self::next_node(p, elements, used) {
                                    found_nodes.extend(nodes);
                                    used.borrow_mut().extend(nodes);
                                } else {
                                    return Some(found_nodes);
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        for (p, e) in elements {
            if !used.borrow().contains(&e) {
                match e {
                    NodePositions::Pin(_point, _pin, _symbol) => {
                        if p == pos {
                            return Some(vec![e]);
                        }
                    }
                    NodePositions::Wire(_, wire) => {
                        let next = if p == pos {
                            used.borrow_mut().push(e);
                            Self::next_node(wire, elements, used)
                        } else if wire == pos {
                            used.borrow_mut().push(e);
                            Self::next_node(p, elements, used)
                        } else {
                            None
                        };
                        if next.is_some() {
                            return next;
                        }
                    }
                    NodePositions::NoConnect(..) => {
                        if p == pos {
                            return Some(vec![e]);
                        }
                    }
                    _ => {}
                }
            }
        }
        None
    }

    pub fn netname(&self, pt: Pt) -> Option<String> {
        for n in &self.nodes {
            if n.points.contains(&pt) {
                return n.identifier.clone();
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_positions() {
        let schema = crate::Schema::load(std::path::Path::new("tests/summe.kicad_sch")).unwrap();
        let netlist = super::Netlist::from(&schema).unwrap();
        // println!("{:#?}", netlist.nodes);
        //TODO assert_eq!(String::from("+15V"), netlist.netname(crate::gr::Pt { x: 153.67, y: 148.59 }).unwrap());
    }
}
