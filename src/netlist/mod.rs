use std::{
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
};

use crate::{self as model, gr::Pt, schema, Error};

#[derive(Clone, Debug, PartialEq)]
pub enum NodePositions<'a> {
    Pin(Pt, &'a schema::Pin, &'a schema::Symbol),
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
//impl Node {
//    pub fn from(identifier: Option<String>, points: Vec<Point>) -> Self {
//        Self { identifier, points }
//    }
//}

/// The Netlist struct
///
/// Create a netlist as a graph.
///
//TODO #[derive(Clone)]
pub struct Netlist<'a> {
    //TODO schema: &'a crate::Schema,
    pub nodes: Vec<Node>, //TODO only public for tests
    node_positions: Vec<(Pt, NodePositions<'a>)>,
}

impl<'a> Netlist<'a> {
    pub fn from(schema: &'a crate::Schema) -> Result<Self, Error> {

        //collect all start and end positions of the wires.
        //Insert it twice, start->end, end->start.
        let mut wires: HashMap<Pt, Pt> = HashMap::new();
        for w in schema.wires.iter() {
            let pt0 = w.pts.0[0];
            let pt1 = w.pts.0[1];
            wires.insert(pt0, pt1);
            wires.insert(pt1, pt0);
        }

        let node_positions = Netlist::positions(schema)?;
        let mut netlist = Self {
            //TODO schema,
            //symbols,
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
                        let mut pins: Vec<&schema::Pin> = vec![p];
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

        //colect symbols and pins
        for s in &schema.symbols {
            if s.lib_id.starts_with("Mechanical:") {
                continue;
            }
            schema.library_symbol(&s.lib_id).into_iter().for_each(|l| {
                for p in l.pins(s.unit) {
                    let pin_pos = model::math::pin_position(s, p);
                    positions.push((pin_pos, NodePositions::Pin(pin_pos, p, s)));
                }
            });
        }

        //collect symbols and pins
        for nc in &schema.no_connects {
            let pt = Pt { x: nc.pos.x, y: nc.pos.y };
            positions.push((pt, NodePositions::NoConnect(pt)));
        }

        //collect junctions
        for j in &schema.junctions {
            let pt = Pt { x: j.pos.x, y: j.pos.y };
            positions.push((pt, NodePositions::NoConnect(pt)));
        }

        //collect labels
        for l in &schema.local_labels {
            let pt = Pt { x: l.pos.x, y: l.pos.y };
            positions.push((pt, NodePositions::Label(pt, l)));
        }
        for l in &schema.global_labels {
            let pt = Pt { x: l.pos.x, y: l.pos.y };
            positions.push((pt, NodePositions::GlobalLabel(pt, l)));
        }

        ////for node in schema.nodes {
        //for symbol in schema.children() {
        //    } else if symbol.name == el::WIRE {
        //        let pts = symbol.query(el::PTS).next().unwrap();
        //        let xy = pts.query(el::XY).collect::<Vec<&Sexp>>();
        //        let xy1: Array1<f64> = xy.first().unwrap().values();
        //        let xy2: Array1<f64> = xy.get(1).unwrap().values();
        //        positions.push((
        //            Point::new(xy1[0], xy1[1]),
        //            NodePositions::Wire(Point::new(xy1[0], xy1[1]), Point::new(xy2[0], xy2[1])),
        //        ));
        //}
        Ok(positions)
    }
//
//    ///Get the node name for the Point.
//    pub fn node_name(&self, point: &Point) -> Option<String> {
//        for n in &self.nodes {
//            if n.points.contains(point) {
//                return n.identifier.clone();
//            }
//        }
//        None
//    }

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

//    pub fn circuit(&self, circuit: &mut Circuit) -> Result<(), Error> {
//        //Create a spice entry for each referenca
//        for (reference, symbols) in &self.symbols {
//            let lib_id: String = symbols.first().unwrap().value(el::LIB_ID).unwrap();
//            //but not for the power symbols
//            if lib_id.starts_with("power:") {
//                continue;
//            }
//
//            let first_symbol = &symbols.first().unwrap();
//
//            //skip symbol when Netlist_Enabled is 'N'
//            let netlist_enabled: Option<String> = first_symbol.property("Spice_Netlist_Enabled"); //TODO differenet
//                                                                                                  //name in new
//                                                                                                  //KiCAD verison
//            if let Some(enabled) = netlist_enabled {
//                if enabled == "N" {
//                    continue;
//                }
//            }
//
//            //create the pin order
//            let lib_symbols = self
//                .schema
//                .root()
//                .unwrap()
//                .query(el::LIB_SYMBOLS)
//                .next()
//                .unwrap();
//            let lib = lib_symbols
//                .query(el::SYMBOL)
//                .find(|s| {
//                    let name: String = s.get(0).unwrap();
//                    name == lib_id
//                })
//                .unwrap();
//            let my_pins = pin_names(lib).unwrap();
//            let mut pin_sequence: Vec<String> = my_pins.keys().map(|s| s.to_string()).collect();
//            pin_sequence.sort_by_key(|x| x.parse::<i32>().unwrap()); //TODO could be string
//
//            //when Node_Sequence is defined, use it
//            let netlist_sequence: Option<String> = first_symbol.property("Spice_Node_Sequence"); //TODO
//            if let Some(sequence) = netlist_sequence {
//                pin_sequence.clear();
//                let splits: Vec<&str> = sequence.split(' ').collect();
//                for s in splits {
//                    pin_sequence.push(s.to_string());
//                }
//            }
//
//            let mut nodes = Vec::new();
//            for n in pin_sequence {
//                let pin = my_pins.get(&n).unwrap();
//                for symbol in symbols {
//                    let unit: usize = symbol.value(el::SYMBOL_UNIT).unwrap();
//                    if unit == pin.1 {
//                        let at = pin.0.query(el::AT).next().unwrap();
//                        let x: f64 = at.get(0).unwrap();
//                        let y: f64 = at.get(1).unwrap();
//                        let pts = Shape::transform(*symbol, &arr1(&[x, y]));
//                        let p0 = Point::new(pts[0], pts[1]);
//                        if let Some(nn) = self.node_name(&p0) {
//                            nodes.push(nn);
//                        } else {
//                            nodes.push(String::from("NF"));
//                        }
//                    }
//                }
//            }
//
//            //write the spice netlist item
//            let spice_primitive: Option<String> = first_symbol.property("Spice_Primitive"); //TODO
//            let spice_model = first_symbol.property("Spice_Model");
//            let spice_value = first_symbol.property("Value");
//            if let Some(primitive) = spice_primitive {
//                if primitive == "X" {
//                    circuit.circuit(reference.to_string(), nodes, spice_model.unwrap())?;
//                } else if primitive == "Q" {
//                    circuit.bjt(
//                        reference.to_string(),
//                        nodes[0].clone(),
//                        nodes[1].clone(),
//                        nodes[2].clone(),
//                        spice_model.unwrap(),
//                    );
//                } else if primitive == "J" {
//                    circuit.jfet(
//                        reference.to_string(),
//                        nodes[0].clone(),
//                        nodes[1].clone(),
//                        nodes[2].clone(),
//                        spice_model.unwrap(),
//                    );
//                } else if primitive == "D" {
//                    circuit.diode(
//                        reference.to_string(),
//                        nodes[0].clone(),
//                        nodes[1].clone(),
//                        spice_model.unwrap(),
//                    );
//                } else {
//                    println!(
//                        "Other node with 'X' -> {}{} - - {}",
//                        primitive,
//                        reference,
//                        spice_value.unwrap()
//                    );
//                }
//            } else if reference.starts_with('R') {
//                circuit.resistor(
//                    reference.clone(),
//                    nodes[0].clone(),
//                    nodes[1].clone(),
//                    spice_value.unwrap(),
//                );
//            } else if reference.starts_with('C') {
//                circuit.capacitor(
//                    reference.clone(),
//                    nodes[0].clone(),
//                    nodes[1].clone(),
//                    spice_value.unwrap(),
//                );
//            // } else if std::env::var("ELEKTRON_DEBUG").is_ok() {
//            } else {
//                println!(
//                    "Unkknwon Reference: {} ({:?}) {}",
//                    reference,
//                    nodes,
//                    spice_value.unwrap()
//                );
//            }
//        }
//
//        Ok(())
//    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn check_positions() {
        let schema = crate::Schema::load(std::path::Path::new("tests/summe.kicad_sch")).unwrap();
        let netlist = super::Netlist::from(&schema).unwrap();
        assert_eq!(String::from("+15V"), netlist.netname(crate::gr::Pt { x: 153.67, y: 148.59 }).unwrap());
    }
}
