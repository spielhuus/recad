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

pub mod circuit;

use std::collections::HashMap;
use std::path::Path;

use indexmap::{IndexMap, IndexSet};
use models::geometry::pin_position;
use models::symbols::ElectricalTypes;
use petgraph::graph::{Graph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Undirected;

use types::{
    constants::el,
    disjointset::DisjointSet,
    gr::{GridPt, Pt},
};

use models::{
    schema::{GlobalLabel, LocalLabel, Schema, SchemaItem, Symbol},
    symbols::Pin,
};

#[derive(Debug, Clone)]
pub enum CircuitNode {
    // A Component (e.g., R1, U1)
    Component {
        ref_des: String,
        lib_id: String,
        value: String,
        sim_pins: String,
        sim_name: String,
    },
    // An Electrical Connection (e.g., Net +5V)
    Net {
        id: usize,
        name: String,
    },
}

#[derive(Debug, Clone)]
pub struct PinEdge {
    pub pin_number: String,        // e.g., "1", "A1", "GND"
    pub pin_type: ElectricalTypes, // Input, Output, etc.
}

#[derive(Debug, Clone)]
pub struct CircuitGraph {
    // Undirected graph because current flows both ways (logic is directed, physics is not)
    pub graph: Graph<CircuitNode, PinEdge, Undirected>,
    pub net_map: HashMap<usize, NodeIndex>, // Map NetID to Graph Node
    pub comp_map: HashMap<String, NodeIndex>, // Map RefDes to Graph Node
}

impl CircuitGraph {
    pub fn from_netlist(netlist: Netlist, schema: &Schema) -> Self {
        let mut graph = Graph::new_undirected();
        let mut net_map = HashMap::new();
        let mut comp_map = HashMap::new();

        // Create Net Nodes
        for (net_id, data) in netlist.nets {
            let node_idx = graph.add_node(CircuitNode::Net {
                id: net_id,
                name: data.name.clone(),
            });
            net_map.insert(net_id, node_idx);

            // Connect Components to this Net
            for (ref_des, pin_num) in data.pins {
                let sym = schema.symbol_by_ref(&ref_des).expect("Symbol not found");
                if sym.exclude_from_sim || sym.lib_id.ends_with("PWR_FLAG") {
                    continue;
                }
                let comp_node_idx = *comp_map.entry(ref_des.clone()).or_insert_with(|| {
                    graph.add_node(CircuitNode::Component {
                        ref_des: ref_des.clone(),
                        lib_id: sym.lib_id.clone(),
                        value: sym.property("Value").unwrap_or_default(),
                        sim_pins: sym.property("Sim.Pins").unwrap_or_default(),
                        sim_name: sym.property("Sim.Name").unwrap_or_default(),
                    })
                });

                let pin_type = if let Some(sym) = schema.symbol_by_ref(&ref_des) {
                    if let Some(lib_sym) = schema.library_symbol(&sym.lib_id) {
                        lib_sym
                            .pin(&pin_num)
                            .map(|p| p.electrical_type)
                            .unwrap_or(ElectricalTypes::Unspecified)
                    } else {
                        ElectricalTypes::Unspecified
                    }
                } else {
                    ElectricalTypes::Unspecified
                };

                graph.add_edge(
                    comp_node_idx,
                    node_idx,
                    PinEdge {
                        pin_number: pin_num,
                        pin_type,
                    },
                );
            }
        }

        CircuitGraph {
            graph,
            net_map,
            comp_map,
        }
    }

    pub fn to_circuit(&self, project_name: String, spice: Vec<String>) -> circuit::Circuit {
        let mut circuit = circuit::Circuit::new(project_name, spice);

        for node_idx in self.graph.node_indices() {
            if let CircuitNode::Component {
                ref_des,
                value,
                sim_pins,
                sim_name,
                ..
            } = &self.graph[node_idx]
            {
                let mut connections: Vec<(String, String)> = Vec::new();

                for edge in self.graph.edges(node_idx) {
                    let target = if edge.source() == node_idx {
                        edge.target()
                    } else {
                        edge.source()
                    };

                    let pin_num = edge.weight().pin_number.clone();
                    if let CircuitNode::Net { name, .. } = &self.graph[target] {
                        connections.push((pin_num, self.sanitize_net_name(name)));
                    }
                }

                let node_list: Vec<String> = if !sim_pins.is_empty() {
                    // Sim.Pins property exists (e.g. "1=3 2=2 3=1")
                    let mut pin_map: Vec<(u32, String)> = Vec::new();
                    for part in sim_pins.split_whitespace() {
                        if let Some((pin_str, seq_str)) = part.split_once('=') {
                            if let Ok(seq) = seq_str.parse::<u32>() {
                                pin_map.push((seq, pin_str.to_string()));
                            }
                        }
                    }

                    // Sort by the sequence index (1, 2, 3...)
                    pin_map.sort_by_key(|k| k.0);

                    // Map the sorted pins to their connected net names
                    pin_map
                        .into_iter()
                        .filter_map(|(_, pin_num)| {
                            // Find the net connected to this pin_num
                            connections
                                .iter()
                                .find(|(p, _)| p == &pin_num)
                                .map(|(_, net)| net.clone())
                        })
                        .collect()
                } else {
                    // Default Sort (Numerical/Alphabetic)
                    connections.sort_by(|a, b| self.compare_pin_numbers(&a.0, &b.0));
                    connections.into_iter().map(|(_, net)| net).collect()
                };

                let component_value = if !sim_name.is_empty() {
                    sim_name.clone()
                } else {
                    value.clone()
                };
                circuit.generic_component(ref_des.clone(), node_list, component_value);
            }
        }

        circuit
    }

    /// SPICE requires the ground node to be strictly named "0".
    fn sanitize_net_name(&self, name: &str) -> String {
        match name.to_uppercase().as_str() {
            "GND" | "EARTH" | "GROUND" => "0".to_string(),
            _ => name.to_string(),
        }
    }

    /// Helper to sort pin strings numerically if possible, otherwise alphabetically.
    /// Handles "1", "2", "10" correctly vs "1", "10", "2".
    fn compare_pin_numbers(&self, a: &str, b: &str) -> std::cmp::Ordering {
        let a_num = a.parse::<u32>();
        let b_num = b.parse::<u32>();

        match (a_num, b_num) {
            (Ok(an), Ok(bn)) => an.cmp(&bn),             // Both are numbers
            (Ok(_), Err(_)) => std::cmp::Ordering::Less, // Numbers come before text
            (Err(_), Ok(_)) => std::cmp::Ordering::Greater,
            (Err(_), Err(_)) => a.cmp(b), // Lexicographical fallback
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum NodePositions<'a> {
    Pin(Pt, &'a Pin, &'a Symbol),
    PowerSymbol(Pt, String),
    Label(Pt, &'a LocalLabel),
    GlobalLabel(Pt, &'a GlobalLabel),
    NoConnect(Pt),
}

#[derive(Debug)]
pub struct Netlist {
    // Map a generic Net ID -> List of attached items
    pub nets: IndexMap<usize, NetData>,
}

#[derive(Debug)]
pub struct NetData {
    pub name: String,
    pub pins: Vec<(String, String)>, // (RefDes, PinNumber)
    pub nodes: Vec<GridPt>,          // All coordinates in this net
}

impl Netlist {
    pub fn from(schema: &Schema) -> Self {
        let mut final_nets = IndexMap::new();
        let uri = schema
            .path
            .as_ref()
            .map(|file| Path::new(file).parent().unwrap());
        final_nets.append(&mut Netlist::parse_sheet(schema));
        for item in &schema.items {
            if let SchemaItem::HierarchicalSheet(page) = item {
                spdlog::info!("found page: {:?}", page.filename().unwrap());
                let file = if let Some(uri) = uri {
                    Path::new(uri).join(page.filename().unwrap())
                } else {
                    Path::new(&page.filename().unwrap()).to_path_buf()
                };
                let schema = Schema::load(&file, page.sheet()).unwrap();
                final_nets.append(&mut Netlist::parse_sheet(&schema));
            }
        }
        Netlist { nets: final_nets }
    }

    fn parse_sheet(schema: &Schema) -> IndexMap<usize, NetData> {
        let mut dsu = DisjointSet::new();
        let mut node_map: HashMap<GridPt, Vec<NodePositions>> = HashMap::new();
        let mut points: IndexSet<GridPt> = IndexSet::new();

        // loop through the diagram
        for item in &schema.items {
            match item {
                SchemaItem::Arc(_) => {}
                SchemaItem::Bus(_bus) => todo!("Bus not implemented"),
                SchemaItem::BusEntry(_bus_entry) => todo!("BusEntry Not implemented"),
                SchemaItem::Circle(_circle) => {}
                SchemaItem::Curve(_curve) => {}
                SchemaItem::GlobalLabel(global_label) => {
                    let gp: GridPt = global_label.pos.into();
                    points.insert(gp);
                    node_map
                        .entry(gp)
                        .or_default()
                        .push(NodePositions::GlobalLabel(
                            global_label.pos.into(),
                            global_label,
                        ));
                }
                SchemaItem::HierarchicalSheet(_hierarchical_sheet) => {}
                SchemaItem::HierarchicalLabel(_hierarchical_label) => {
                    todo!("HierarchicalLabel not implemented")
                }
                SchemaItem::Junction(junction) => {
                    // Junctions bridge wires. Usually explicit in schema file.
                    let gp: GridPt = junction.pos.into();
                    points.insert(gp);
                    // Usually we don't need to push to node_map unless we want to debug,
                    // but ensuring it exists in DSU is key.
                }
                SchemaItem::Line(_line) => {}
                SchemaItem::LocalLabel(local_label) => {
                    let gp: GridPt = local_label.pos.into();
                    points.insert(gp);

                    // Register functionality at this node
                    node_map
                        .entry(gp)
                        .or_default()
                        .push(NodePositions::Label(local_label.pos.into(), local_label));
                }
                SchemaItem::NetclassFlag(_netclass_flag) => todo!("Netclass Flag not implemented"),
                SchemaItem::NoConnect(no_connect) => {
                    let gp: GridPt = no_connect.pos.into();
                    points.insert(gp);
                    node_map
                        .entry(gp)
                        .or_default()
                        .push(NodePositions::NoConnect(no_connect.pos.into()));
                }
                SchemaItem::Polyline(_polyline) => {}
                SchemaItem::Rectangle(_rectangle) => {}
                SchemaItem::Symbol(symbol) => {
                    if let Some(lib_sym) = schema.library_symbol(&symbol.lib_id) {
                        // FIX: Treat PWR_FLAG as a component, not a power label.
                        // This allows PWR_FLAG to "drive" the net in ERC checks.
                        let is_power = symbol.lib_id.starts_with("power:")
                            && !symbol.lib_id.ends_with("PWR_FLAG");

                        for p in lib_sym.pins(symbol.unit) {
                            let raw_pos = pin_position(symbol, p);
                            let grid_pos: GridPt = raw_pos.into();

                            points.insert(grid_pos);

                            let entry = node_map.entry(grid_pos).or_default();

                            if is_power {
                                // Treat power symbols as labels (Source of Net Name), not pins.
                                entry.push(NodePositions::PowerSymbol(
                                    raw_pos,
                                    symbol.property("Value").unwrap_or_default(),
                                ));
                            } else {
                                entry.push(NodePositions::Pin(raw_pos, p, symbol));
                            }
                        }
                    }
                }
                SchemaItem::Text(_) => {}
                SchemaItem::TextBox(_text_box) => todo!("TextBox not implemented"),
                SchemaItem::Wire(wire) => {
                    let start: GridPt = wire.pts.0[0].into();
                    let end: GridPt = (*wire.pts.0.last().unwrap()).into();

                    // Track points
                    points.insert(start);
                    points.insert(end);

                    // The core logic: Connect them
                    dsu.union(start, end);

                    // Also handle polyline midpoints if any
                    for segment in wire.pts.0.windows(2) {
                        let p1: GridPt = segment[0].into();
                        let p2: GridPt = segment[1].into();

                        dsu.union(p1, p2);

                        // Track for later lookup
                        points.insert(p1);
                        points.insert(p2);
                    }
                }
            }
        }

        // Create initial geometric groups
        let mut groups: HashMap<GridPt, usize> = HashMap::new();
        let mut next_net_id = 0;

        // Temporary storage for geometric nets
        let mut geo_nets: IndexMap<usize, NetData> = IndexMap::new();

        for point in points {
            let root = dsu.find(point);
            let net_id = *groups.entry(root).or_insert_with(|| {
                let id = next_net_id;
                next_net_id += 1;
                id
            });

            if let Some(items) = node_map.get(&point) {
                let net_data = geo_nets.entry(net_id).or_insert(NetData {
                    name: format!("Net-{}", net_id),
                    pins: Vec::new(),
                    nodes: Vec::new(),
                });

                net_data.nodes.push(point);

                for node in items {
                    match node {
                        NodePositions::Pin(_, pin, sym) => {
                            net_data.pins.push((
                                sym.property(el::PROPERTY_REFERENCE).unwrap_or_default(),
                                pin.number.name.clone(),
                            ));
                        }
                        NodePositions::PowerSymbol(_, name) => {
                            net_data.name = name.clone();
                        }
                        NodePositions::GlobalLabel(_, lbl) => {
                            net_data.name = lbl.text.clone();
                        }
                        NodePositions::Label(_, lbl) => {
                            if net_data.name.starts_with("Net-") {
                                net_data.name = lbl.text.clone();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Merge nets with the same name (Handle Global Power / Labels)
        // This is crucial for ERC: Physically disconnected wires sharing a global
        // name (e.g., +5V) must be treated as one logical net.
        let mut final_nets = IndexMap::new();
        let mut name_to_id: HashMap<String, usize> = HashMap::new();
        let mut next_final_id = 0;

        for (_, data) in geo_nets {
            // Determine target ID: either existing one for this name, or a new one
            // If the name is auto-generated (Net-X), we treat it as distinct unless
            // explicitly connected (which DSU handled in Pass 1).
            let target_id = if !data.name.starts_with("Net-") {
                *name_to_id.entry(data.name.clone()).or_insert_with(|| {
                    let id = next_final_id;
                    next_final_id += 1;
                    id
                })
            } else {
                let id = next_final_id;
                next_final_id += 1;
                id
            };

            let entry = final_nets.entry(target_id).or_insert(NetData {
                name: data.name,
                pins: Vec::new(),
                nodes: Vec::new(),
            });

            entry.pins.extend(data.pins);
            entry.nodes.extend(data.nodes);
        }
        final_nets
    }
}

// #[cfg(test)]
// mod tests {
//     pub const SCHEMA_SUMME: &str = "tests/summe/summe.kicad_sch";
//     use crate::gr::Pt;
//
//     #[test]
//     fn test_wires() {
//         let schema =
//             crate::Schema::load(std::path::Path::new("tests/summe/summe.kicad_sch")).unwrap();
//         let wires = super::Netlist::wires(&schema);
//         let wire = wires
//             .get(&Pt {
//                 x: 179.07,
//                 y: 49.53,
//             })
//             .unwrap();
//         assert_eq!(
//             &vec![
//                 Pt {
//                     x: 179.07,
//                     y: 34.29,
//                 },
//                 Pt {
//                     x: 180.34,
//                     y: 49.53,
//                 },
//                 Pt {
//                     x: 167.64,
//                     y: 49.53,
//                 },
//             ],
//             wire
//         );
//     }
//
//     #[test]
//     fn test_get_wires() {
//         let schema = crate::Schema::load(std::path::Path::new(SCHEMA_SUMME)).unwrap();
//         let wires = super::Netlist::wires(&schema);
//         let mut visited = vec![];
//         let wire = super::Netlist::get_wire(
//             Pt {
//                 x: 179.07,
//                 y: 49.53,
//             },
//             &wires,
//             &mut visited,
//         )
//         .unwrap();
//         assert_eq!(
//             vec![
//                 Pt {
//                     x: 179.07,
//                     y: 34.29,
//                 },
//                 Pt {
//                     x: 180.34,
//                     y: 49.53,
//                 },
//                 Pt {
//                     x: 167.64,
//                     y: 49.53,
//                 },
//             ],
//             wire
//         );
//     }
//
//     #[test]
//     fn test_get_visited_wires() {
//         let schema =
//             crate::Schema::load(std::path::Path::new("tests/summe/summe.kicad_sch")).unwrap();
//         let wires = super::Netlist::wires(&schema);
//         let mut visited = vec![Pt {
//             x: 180.34,
//             y: 49.53,
//         }];
//         let wire = super::Netlist::get_wire(
//             Pt {
//                 x: 179.07,
//                 y: 49.53,
//             },
//             &wires,
//             &mut visited,
//         )
//         .unwrap();
//         assert_eq!(
//             vec![
//                 Pt {
//                     x: 179.07,
//                     y: 34.29,
//                 },
//                 Pt {
//                     x: 167.64,
//                     y: 49.53,
//                 },
//             ],
//             wire
//         );
//     }
//
//     #[test]
//     fn test_seek_wires() {
//         let schema = crate::Schema::load(std::path::Path::new(SCHEMA_SUMME)).unwrap();
//         let wires = super::Netlist::wires(&schema);
//         let mut visited = vec![];
//         let wire = super::Netlist::seek_wire(
//             Pt {
//                 x: 179.07,
//                 y: 49.53,
//             },
//             &wires,
//             &mut visited,
//         );
//         assert_eq!(
//             vec![
//                 Pt {
//                     x: 179.07,
//                     y: 34.29,
//                 },
//                 Pt {
//                     x: 185.42,
//                     y: 34.29
//                 },
//                 Pt {
//                     x: 179.07,
//                     y: 22.86
//                 },
//                 Pt {
//                     x: 185.42,
//                     y: 22.86
//                 },
//                 Pt {
//                     x: 180.34,
//                     y: 49.53
//                 },
//                 Pt {
//                     x: 167.64,
//                     y: 49.53
//                 },
//                 Pt {
//                     x: 166.37,
//                     y: 49.53
//                 },
//                 Pt {
//                     x: 167.64,
//                     y: 41.91
//                 },
//                 Pt {
//                     x: 167.64,
//                     y: 34.29
//                 },
//                 Pt {
//                     x: 167.64,
//                     y: 26.67
//                 },
//                 Pt {
//                     x: 166.37,
//                     y: 26.67
//                 },
//                 Pt {
//                     x: 166.37,
//                     y: 34.29
//                 },
//                 Pt {
//                     x: 166.37,
//                     y: 41.91
//                 }
//             ],
//             wire
//         );
//     }
//
//     #[test]
//     fn check_positions() {
//         let schema = crate::Schema::load(std::path::Path::new(SCHEMA_SUMME)).unwrap();
//         let netlist = super::Netlist::from(&schema).unwrap();
//         assert_eq!(
//             String::from("R33_2__U7_6__C9_2__R36_1"),
//             netlist
//                 .netname(crate::gr::Pt {
//                     x: 207.01,
//                     y: 52.07
//                 })
//                 .unwrap()
//         );
//         assert_eq!(
//             String::from("R7_2__R8_1__U4_3__RV3_2"),
//             netlist
//                 .netname(crate::gr::Pt {
//                     x: 81.28,
//                     y: 102.87
//                 })
//                 .unwrap()
//         );
//         assert_eq!(
//             String::from("+15V"),
//             netlist
//                 .netname(crate::gr::Pt {
//                     x: 153.67,
//                     y: 148.59
//                 })
//                 .unwrap()
//         );
//     }
// }
