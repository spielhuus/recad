//!  Extract Netlist from Schematic File:
//!
//!  **Strategy for Extracting Netlist from Kicad Schematic File:**
//!
//! 1. Collect all wire endpoints (nodes) in the schematic file.
//! 2. Identify and group together connections that share the same coordinates (junctions).
//! 3. Iterate through each of the identified junctions.
//! 4. For each junction, find the associated wire(s) at that point.
//! 5. Traverse all wires connected to the current wire at the junction.
//! 6. For each traversed wire endpoint, identify and group together connections with the same coordinates (junctions).
//! 7. Assign net names to the identified groups of connections based on their connectivity; connections consisting of a single element are named NC (No Connection).

use indexmap::IndexMap;

use crate::{
    gr::Pt, schema::{GlobalLabel, LocalLabel, SchemaItem, Symbol}, sexp::constants::el, symbols::Pin, Circuit, Error, Schema
};

#[derive(Clone, Debug, PartialEq)]
enum NodePositions<'a> {
    Pin(Pt, &'a Pin, &'a Symbol),
    Label(Pt, &'a LocalLabel),
    GlobalLabel(Pt, &'a GlobalLabel),
    NoConnect(Pt),
    Junction(Pt),
}

/// create a netlist from a schematic file.
pub struct Netlist<'a> {
    node_positions: IndexMap<Pt, Vec<NodePositions<'a>>>,
    names: IndexMap<Pt, String>,
}

impl<'a> Netlist<'a> {
    fn collect_points(schema: &'a Schema) -> IndexMap<Pt, Vec<NodePositions<'a>>> {
        let mut positions: IndexMap<Pt, Vec<NodePositions<'a>>> = IndexMap::new();

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
                                let pin_pos = crate::math::pin_position(symbol, p);
                                positions
                                    .entry(pin_pos)
                                    .or_default()
                                    .push(NodePositions::Pin(pin_pos, p, symbol));
                            }
                        });
                }
                SchemaItem::NoConnect(nc) => {
                    positions
                        .entry(nc.pos.into())
                        .or_default()
                        .push(NodePositions::NoConnect(nc.pos.into()));
                }
                SchemaItem::Junction(junction) => {
                    positions
                        .entry(junction.pos.into())
                        .or_default()
                        .push(NodePositions::Junction(junction.pos.into()));
                }
                SchemaItem::LocalLabel(label) => {
                    positions
                        .entry(label.pos.into())
                        .or_default()
                        .push(NodePositions::Label(label.pos.into(), label));
                }
                SchemaItem::GlobalLabel(label) => {
                    positions
                        .entry(label.pos.into())
                        .or_default()
                        .push(NodePositions::GlobalLabel(label.pos.into(), label));
                }
                SchemaItem::Bus(_) => {} //TODO
                SchemaItem::BusEntry(_) => {}
                SchemaItem::HierarchicalSheet(_) => {}
                SchemaItem::HierarchicalLabel(_) => {}
                SchemaItem::NetclassFlag(_) => {}
                _ => {}
            }
        }
        positions
    }

    /** This function takes a reference to a [`Schema`] and returns a `IndexMap<Pt, Pt>`.
    It iterates through the items in the schema, filtering only `Wire` items. For
    each `Wire`, it creates an entry in the map with the starting point as key
    and the ending point as value, and also creates a reciprocal entry
    to ensure bidirectionality. */
    fn wires(schema: &Schema) -> IndexMap<Pt, Vec<Pt>> {
        let mut wires: IndexMap<Pt, Vec<Pt>> = IndexMap::new();
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
                wires.entry(pt0).or_default().push(pt1);
                wires.entry(pt1).or_default().push(pt0);
            });
        wires
    }

    fn get_wire(pt: Pt, wires: &IndexMap<Pt, Vec<Pt>>, visited: &mut Vec<Pt>) -> Option<Vec<Pt>> {
        visited.push(pt);
        let wires = wires.get(&pt);
        wires.map(|wires| wires
                    .iter()
                    .filter_map(|point| {
                        if visited.contains(point) {
                            None
                        } else {
                            Some(*point)
                        }
                    })
                    .collect::<Vec<Pt>>())
    }

    fn seek_wire(pt: Pt, wires: &IndexMap<Pt, Vec<Pt>>, visited: &mut Vec<Pt>) -> Vec<Pt> {
        let mut found = vec![];
        if let Some(wire) = Netlist::get_wire(pt, wires, visited) {
            for wire in wire {
                found.push(wire);
                found.append(&mut Netlist::seek_wire(wire, wires, visited));
            }
        }
        found
    }

    fn generate_names(results: &IndexMap<Pt, Vec<NodePositions>>) -> IndexMap<Pt, String> {
        let mut names = IndexMap::new();
        for (key, items) in results.iter() {
            let mut name = String::new();
            let mut label = None;
            let mut positions = vec![key];
            let mut first = true;
            for item in items.iter() {
                match item {
                    NodePositions::Pin(pos, pin, symbol) => {
                        positions.push(pos);
                        if symbol.lib_id.starts_with("power:") {
                            label = Some(symbol.property(el::PROPERTY_VALUE));
                        } else if first {
                            first = false;
                            name.push_str(&symbol.property(el::PROPERTY_REFERENCE));
                            name.push('_');
                            name.push_str(&pin.number.name);
                        } else {
                            name.push_str("__");
                            name.push_str(&symbol.property(el::PROPERTY_REFERENCE));
                            name.push('_');
                            name.push_str(&pin.number.name);
                        }
                    },
                    NodePositions::Label(pos, l) => {
                        positions.push(pos);
                        label = Some(l.text.clone());
                    },
                    NodePositions::GlobalLabel(pos, l) => {
                        positions.push(pos);
                        label = Some(l.text.clone());
                    },
                    NodePositions::NoConnect(pos) => {
                        positions.push(pos);
                    },
                    NodePositions::Junction(pos) => {
                        positions.push(pos);
                    },
                }
            }

            if let Some(label) = label {
                for pos in positions {
                    names.insert(*pos, label.clone());
                }
            } else {
                for pos in positions {
                    names.insert(*pos, name.clone());
                }
            }
        }
        names
    }

    pub fn from(schema: &'a crate::Schema) -> Result<Self, Error> {
        let wires = Netlist::wires(schema);
        let positions = Netlist::collect_points(schema);

        let mut final_positions = IndexMap::new();
        let mut visited_positions = vec![];
        let mut visited_wires = vec![];
        for (pos, nodes) in positions.iter() {
            let mut new_nodes = nodes.clone();
            if !visited_positions.contains(pos) {
                visited_positions.push(*pos);
                for wire in Netlist::seek_wire(*pos, &wires, &mut visited_wires) {
                    if let Some(other) = positions.get(&wire) {
                        visited_positions.push(wire);
                        for node in other {
                            new_nodes.push(node.clone());
                        }
                    }
                }
                final_positions.insert(*pos, new_nodes);
            }
        }

        Ok(Netlist {
            names: Netlist::generate_names(&final_positions),
            node_positions: final_positions,
        })
    }

    pub fn netname(&self, pt: Pt) -> Option<String> {
        self.names.get(&pt).cloned()
    }

    pub fn circuit(&self, circuit: &mut Circuit) -> Result<(), Error> {
        ////Create a spice entry for each referenca
        //for (reference, symbols) in &self.symbols {
        //    let lib_id: String = symbols.first().unwrap().value(el::LIB_ID).unwrap();
        //    //but not for the power symbols
        //    if lib_id.starts_with("power:") {
        //        continue;
        //    }
        //
        //    let first_symbol = &symbols.first().unwrap();
        //
        //    //skip symbol when Netlist_Enabled is 'N'
        //    let netlist_enabled: Option<String> = first_symbol.property("Spice_Netlist_Enabled"); //TODO differenet
        //                                                                                          //name in new
        //                                                                                          //KiCAD verison
        //    if let Some(enabled) = netlist_enabled {
        //        if enabled == "N" {
        //            continue;
        //        }
        //    }
        //
        //    //create the pin order
        //    let lib_symbols = self
        //        .schema
        //        .root()
        //        .unwrap()
        //        .query(el::LIB_SYMBOLS)
        //        .next()
        //        .unwrap();
        //    let lib = lib_symbols
        //        .query(el::SYMBOL)
        //        .find(|s| {
        //            let name: String = s.get(0).unwrap();
        //            name == lib_id
        //        })
        //        .unwrap();
        //    let my_pins = pin_names(lib).unwrap();
        //    let mut pin_sequence: Vec<String> = my_pins.keys().map(|s| s.to_string()).collect();
        //    pin_sequence.sort_by_key(|x| x.parse::<i32>().unwrap()); //TODO could be string
        //
        //    //when Node_Sequence is defined, use it
        //    let netlist_sequence: Option<String> = first_symbol.property("Spice_Node_Sequence"); //TODO
        //    if let Some(sequence) = netlist_sequence {
        //        pin_sequence.clear();
        //        let splits: Vec<&str> = sequence.split(' ').collect();
        //        for s in splits {
        //            pin_sequence.push(s.to_string());
        //        }
        //    }
        //
        //    let mut nodes = Vec::new();
        //    for n in pin_sequence {
        //        let pin = my_pins.get(&n).unwrap();
        //        for symbol in symbols {
        //            let unit: usize = symbol.value(el::SYMBOL_UNIT).unwrap();
        //            if unit == pin.1 {
        //                let at = pin.0.query(el::AT).next().unwrap();
        //                let x: f64 = at.get(0).unwrap();
        //                let y: f64 = at.get(1).unwrap();
        //                let pts = Shape::transform(*symbol, &arr1(&[x, y]));
        //                let p0 = Point::new(pts[0], pts[1]);
        //                if let Some(nn) = self.node_name(&p0) {
        //                    nodes.push(nn);
        //                } else {
        //                    nodes.push(String::from("NF"));
        //                }
        //            }
        //        }
        //    }
        //
        //    //write the spice netlist item
        //    let spice_primitive: Option<String> = first_symbol.property("Spice_Primitive"); //TODO
        //    let spice_model = first_symbol.property("Spice_Model");
        //    let spice_value = first_symbol.property("Value");
        //    if let Some(primitive) = spice_primitive {
        //        if primitive == "X" {
        //            circuit.circuit(reference.to_string(), nodes, spice_model.unwrap())?;
        //        } else if primitive == "Q" {
        //            circuit.bjt(
        //                reference.to_string(),
        //                nodes[0].clone(),
        //                nodes[1].clone(),
        //                nodes[2].clone(),
        //                spice_model.unwrap(),
        //            );
        //        } else if primitive == "J" {
        //            circuit.jfet(
        //                reference.to_string(),
        //                nodes[0].clone(),
        //                nodes[1].clone(),
        //                nodes[2].clone(),
        //                spice_model.unwrap(),
        //            );
        //        } else if primitive == "D" {
        //            circuit.diode(
        //                reference.to_string(),
        //                nodes[0].clone(),
        //                nodes[1].clone(),
        //                spice_model.unwrap(),
        //            );
        //        } else {
        //            println!(
        //                "Other node with 'X' -> {}{} - - {}",
        //                primitive,
        //                reference,
        //                spice_value.unwrap()
        //            );
        //        }
        //    } else if reference.starts_with('R') {
        //        circuit.resistor(
        //            reference.clone(),
        //            nodes[0].clone(),
        //            nodes[1].clone(),
        //            spice_value.unwrap(),
        //        );
        //    } else if reference.starts_with('C') {
        //        circuit.capacitor(
        //            reference.clone(),
        //            nodes[0].clone(),
        //            nodes[1].clone(),
        //            spice_value.unwrap(),
        //        );
        //    // } else if std::env::var("ELEKTRON_DEBUG").is_ok() {
        //    } else {
        //        println!(
        //            "Unkknwon Reference: {} ({:?}) {}",
        //            reference,
        //            nodes,
        //            spice_value.unwrap()
        //        );
        //    }
        //}
        Ok(())
    }
}

//implemnt the dispaly trait for netlist
impl<'a> std::fmt::Display for Netlist<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Netlist:")?;
        for pt in &self.node_positions {
            writeln!(f, "- Pt({}x{})", pt.0.x, pt.0.y)?;
            for node in pt.1 {
                match node {
                    NodePositions::Pin(_, pin, symbol) => {
                        writeln!(
                            f,
                            "    Pin({}:{})",
                            symbol.property(el::PROPERTY_REFERENCE),
                            pin.number.name
                        )?;
                    }
                    NodePositions::Label(_, l) => {
                        writeln!(f, "    LocalLabel({})", l.text)?;
                    }
                    NodePositions::GlobalLabel(_, l) => {
                        writeln!(f, "    GlobalLabel({})", l.text)?;
                    }
                    NodePositions::NoConnect(_) => {
                        writeln!(f, "    NoConnect()")?;
                    }
                    NodePositions::Junction(_) => {
                        writeln!(f, "    Junction()")?;
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::gr::Pt;

    #[test]
    fn test_wires() {
        let schema = crate::Schema::load(std::path::Path::new("tests/summe.kicad_sch")).unwrap();
        let wires = super::Netlist::wires(&schema);
        let wire = wires
            .get(&Pt {
                x: 179.07,
                y: 49.53,
            })
            .unwrap();
        assert_eq!(
            &vec![
                Pt {
                    x: 179.07,
                    y: 34.29,
                },
                Pt {
                    x: 180.34,
                    y: 49.53,
                },
                Pt {
                    x: 167.64,
                    y: 49.53,
                },
            ],
            wire
        );
    }

    #[test]
    fn test_get_wires() {
        let schema = crate::Schema::load(std::path::Path::new("tests/summe.kicad_sch")).unwrap();
        let wires = super::Netlist::wires(&schema);
        let mut visited = vec![];
        let wire = super::Netlist::get_wire(
            Pt {
                x: 179.07,
                y: 49.53,
            },
            &wires,
            &mut visited,
        )
        .unwrap();
        assert_eq!(
            vec![
                Pt {
                    x: 179.07,
                    y: 34.29,
                },
                Pt {
                    x: 180.34,
                    y: 49.53,
                },
                Pt {
                    x: 167.64,
                    y: 49.53,
                },
            ],
            wire
        );
    }

    #[test]
    fn test_get_visited_wires() {
        let schema = crate::Schema::load(std::path::Path::new("tests/summe.kicad_sch")).unwrap();
        let wires = super::Netlist::wires(&schema);
        let mut visited = vec![Pt {
            x: 180.34,
            y: 49.53,
        }];
        let wire = super::Netlist::get_wire(
            Pt {
                x: 179.07,
                y: 49.53,
            },
            &wires,
            &mut visited,
        )
        .unwrap();
        assert_eq!(
            vec![
                Pt {
                    x: 179.07,
                    y: 34.29,
                },
                Pt {
                    x: 167.64,
                    y: 49.53,
                },
            ],
            wire
        );
    }

    #[test]
    fn test_seek_wires() {
        let schema = crate::Schema::load(std::path::Path::new("tests/summe.kicad_sch")).unwrap();
        let wires = super::Netlist::wires(&schema);
        let mut visited = vec![];
        let wire = super::Netlist::seek_wire(
            Pt {
                x: 179.07,
                y: 49.53,
            },
            &wires,
            &mut visited,
        );
        assert_eq!(
            vec![
                Pt {
                    x: 179.07,
                    y: 34.29
                },
                Pt {
                    x: 185.42,
                    y: 34.29
                },
                Pt {
                    x: 179.07,
                    y: 22.86
                },
                Pt {
                    x: 185.42,
                    y: 22.86
                },
                Pt {
                    x: 180.34,
                    y: 49.53
                },
                Pt {
                    x: 167.64,
                    y: 49.53
                },
                Pt {
                    x: 166.37,
                    y: 49.53
                },
                Pt {
                    x: 167.64,
                    y: 41.91
                },
                Pt {
                    x: 167.64,
                    y: 34.29
                },
                Pt {
                    x: 167.64,
                    y: 26.67
                },
                Pt {
                    x: 166.37,
                    y: 26.67
                },
                Pt {
                    x: 166.37,
                    y: 34.29
                },
                Pt {
                    x: 166.37,
                    y: 41.91
                }
            ],
            wire
        );
    }

    #[test]
    fn check_positions() {
        let schema = crate::Schema::load(std::path::Path::new("tests/summe.kicad_sch")).unwrap();
        let netlist = super::Netlist::from(&schema).unwrap();
        assert_eq!(String::from("R33_2__U7_6__C9_2__R36_1"), netlist.netname(crate::gr::Pt { x: 207.01, y: 52.07 }).unwrap());
        assert_eq!(String::from("R7_2__R8_1__U4_3__RV3_2"), netlist.netname(crate::gr::Pt { x: 81.28, y: 102.87 }).unwrap());
        assert_eq!(String::from("+15V"), netlist.netname(crate::gr::Pt { x: 153.67, y: 148.59 }).unwrap());
    }
}
