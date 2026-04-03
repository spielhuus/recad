use std::collections::{HashMap, HashSet};
use netlist::Netlist;
use types::{
  constants::el,
  disjointset::DisjointSet,
  gr::{GridPt, Pt},
};

use models::{
    pcb::{Footprint, GraphicItem, Pad, Pcb},
    schema::{Schema, SchemaItem},
};

pub fn drc(pcb: &Pcb, schema: &Schema) -> Vec<DRCViolation> {
    Drc::new(pcb, schema).run()
}

#[derive(Debug, Clone, PartialEq)]
pub enum DRCLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct DRCViolation {
    pub level: DRCLevel,
    pub title: String,
    pub description: String,
    pub position: Pt,
    pub markers: Vec<Pt>,
}

pub struct Drc<'a> {
    pcb: &'a Pcb,
    schema: &'a Schema,
    netlist: Netlist,
}

impl<'a> Drc<'a> {
    pub fn new(pcb: &'a Pcb, schema: &'a Schema) -> Self {
        let netlist = Netlist::from(schema);
        Self { 
            pcb, 
            schema, 
            netlist 
        }
    }

    /// Run all DRC checks on the PCB
    pub fn run(&self) -> Vec<DRCViolation> {
        let mut violations = Vec::new();

        self.check_track_widths(&mut violations);
        self.check_via_sizes(&mut violations);
        self.check_items_on_board(&mut violations);
        
        // Logical LVS parity checks (Schema vs PCB definitions)
        self.check_schematic_parity(&mut violations);
        
        // Physical copper connectivity checks
        self.check_connectivity(&mut violations);

        violations
    }

    /// Helper to reliably extract the Reference Designator from a Footprint
    fn get_reference(fp: &Footprint) -> String {
        // Method 1: It might be mapped to the primary properties
        if let Some(r) = fp.property.get("Reference") {
            return r.clone();
        }
        
        // Method 2: Fallback to graphic_items where old KiCad text fields reside
        for item in &fp.graphic_items {
            match item {
                GraphicItem::FpText(text) if text.text_type == "reference" => return text.text.clone(),
                GraphicItem::FpProperty(prop) if prop.name == "Reference" => return prop.value.clone(),
                _ => {}
            }
        }
        "".to_string()
    }

    /// Checks if PCB Footprints and Nets strictly match the Schematic logic
    fn check_schematic_parity(&self, violations: &mut Vec<DRCViolation>) {
        let mut sch_refs = HashSet::new();

        // 1. Gather all required symbols from the schema
        for item in &self.schema.items {
            if let SchemaItem::Symbol(sym) = item {
                if sym.on_board {
                    if let Some(ref_des) = sym.property(el::PROPERTY_REFERENCE) {
                        if !ref_des.starts_with('#') && !ref_des.ends_with('?') {
                            sch_refs.insert(ref_des);
                        }
                    }
                }
            }
        }

        let mut pcb_refs = HashSet::new();
        let mut pcb_pin_to_net = HashMap::new();
        let mut pcb_nets_to_pins: HashMap<u32, Vec<(String, String)>> = HashMap::new();

        // 2. Gather all PCB references and their net pad mappings
        for fp in &self.pcb.footprints {
            let ref_des = Self::get_reference(fp);
            if !ref_des.is_empty() {
                pcb_refs.insert(ref_des.clone());
            }

            for pad in &fp.pads {
                let pin_id = (ref_des.clone(), pad.number.clone());
                pcb_pin_to_net.insert(pin_id.clone(), pad.net.ordinal);
                
                if pad.net.ordinal > 0 {
                    pcb_nets_to_pins.entry(pad.net.ordinal).or_default().push(pin_id);
                }
            }
        }

        // 3. Footprint Check: Missing on PCB
        for sch_ref in &sch_refs {
            if !pcb_refs.contains(sch_ref) {
                violations.push(DRCViolation {
                    level: DRCLevel::Error,
                    title: "Missing Footprint".to_string(),
                    description: format!("Schematic component '{}' is missing on the PCB.", sch_ref),
                    position: Pt::default(),
                    markers: vec![],
                });
            }
        }

        // 4. Footprint Check: Extra on PCB
        for fp in &self.pcb.footprints {
            let ref_des = Self::get_reference(fp);
            if !ref_des.is_empty() && !sch_refs.contains(&ref_des) && !fp.board_only {
                violations.push(DRCViolation {
                    level: DRCLevel::Warning,
                    title: "Extra Footprint".to_string(),
                    description: format!("PCB footprint '{}' is not present in the schematic.", ref_des),
                    position: Pt::from(fp.pos),
                    markers: vec![],
                });
            }
        }

        // 5. Net Check: Parity Schematic -> PCB (Are things connected properly?)
        let mut sch_pin_to_net = HashMap::new();
        for (net_id, net_data) in &self.netlist.nets {
            for pin in &net_data.pins {
                sch_pin_to_net.insert(pin.clone(), *net_id);
            }

            // Ignore nets that don't bridge components
            if net_data.pins.len() < 2 { continue; }

            let mut pcb_net_set = HashSet::new();
            let mut pcb_net_names = HashSet::new();
            let mut missing_pads = Vec::new();

            for pin in &net_data.pins {
                if let Some(&pcb_net_ord) = pcb_pin_to_net.get(pin) {
                    pcb_net_set.insert(pcb_net_ord);
                    if pcb_net_ord == 0 {
                        pcb_net_names.insert("Unconnected".to_string());
                    } else if let Some(net) = self.pcb.nets.iter().find(|n| n.ordinal == pcb_net_ord) {
                        pcb_net_names.insert(net.name.clone());
                    }
                } else {
                    missing_pads.push(pin);
                }
            }

            for pin in missing_pads {
                if pcb_refs.contains(&pin.0) {
                    violations.push(DRCViolation {
                        level: DRCLevel::Error,
                        title: "Missing Pad".to_string(),
                        description: format!("Schematic connects {}-{}, but the pad does not exist on the PCB.", pin.0, pin.1),
                        position: Pt::default(),
                        markers: vec![],
                    });
                }
            }

            if pcb_net_set.len() > 1 {
                violations.push(DRCViolation {
                    level: DRCLevel::Error,
                    title: "Schematic Net Mismatch".to_string(),
                    description: format!(
                        "Schematic net '{}' is mapped to multiple PCB nets or left unconnected: {:?}",
                        net_data.name, pcb_net_names
                    ),
                    position: Pt::default(),
                    markers: vec![],
                });
            }
        }

        // 6. Net Check: Short Circuits PCB -> Schematic
        // Ensure that pins grouped in the same PCB net actually belong to the same Schematic net
        for (pcb_net_ord, pcb_pins) in pcb_nets_to_pins {
            if pcb_pins.len() < 2 { continue; }

            let mut sch_net_set = HashSet::new();
            for pin in &pcb_pins {
                if let Some(&sch_net_id) = sch_pin_to_net.get(pin) {
                    sch_net_set.insert(sch_net_id);
                }
            }

            if sch_net_set.len() > 1 {
                let pcb_net_name = self.pcb.nets.iter().find(|n| n.ordinal == pcb_net_ord)
                    .map(|n| n.name.clone())
                    .unwrap_or_default();
                    
                violations.push(DRCViolation {
                    level: DRCLevel::Error,
                    title: "Short Circuit".to_string(),
                    description: format!(
                        "PCB Net '{}' improperly connects pins that belong to different schematic nets.",
                        pcb_net_name
                    ),
                    position: Pt::default(),
                    markers: vec![],
                });
            }
        }
    }

    /// Check if any track segment violates the minimum track width
    fn check_track_widths(&self, violations: &mut Vec<DRCViolation>) {
        let min_width = self.pcb.setup.as_ref()
            .and_then(|s| s.solder_mask_min_width)
            .unwrap_or(0.2); 

        for track in &self.pcb.segments {
            if track.width < min_width {
                violations.push(DRCViolation {
                    level: DRCLevel::Error,
                    title: "Track width too small".to_string(),
                    description: format!(
                        "Track on layer '{}' has width {:.3}mm, which is less than the minimum {:.3}mm.",
                        track.layer, track.width, min_width
                    ),
                    position: track.start,
                    markers: vec![track.end],
                });
            }
        }
    }

    /// Check if any via violates the minimum via size or drill
    fn check_via_sizes(&self, violations: &mut Vec<DRCViolation>) {
        let min_via_size = 0.4;  
        let min_via_drill = 0.2; 

        for via in &self.pcb.vias {
            if via.size < min_via_size {
                violations.push(DRCViolation {
                    level: DRCLevel::Error,
                    title: "Via size too small".to_string(),
                    description: format!(
                        "Via size {:.3}mm is less than the minimum {:.3}mm.",
                        via.size, min_via_size
                    ),
                    position: Pt::from(via.pos),
                    markers: vec![],
                });
            }

            if via.drill < min_via_drill {
                violations.push(DRCViolation {
                    level: DRCLevel::Error,
                    title: "Via drill too small".to_string(),
                    description: format!(
                        "Via drill {:.3}mm is less than the minimum {:.3}mm.",
                        via.drill, min_via_drill
                    ),
                    position: Pt::from(via.pos),
                    markers: vec![],
                });
            }
        }
    }

    /// Checks if all footprints fall within the physical PCB Outline
    fn check_items_on_board(&self, violations: &mut Vec<DRCViolation>) {
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut has_outline = false;

        // Iterate through graphical lines specifically on Edge.Cuts to find board boundary bounds
        for line in &self.pcb.gr_lines { 
            if line.layer == "Edge.Cuts" {
                has_outline = true;
                let pts =[line.start, line.end];
                for p in pts {
                    min_x = min_x.min(p.x);
                    min_y = min_y.min(p.y);
                    max_x = max_x.max(p.x);
                    max_y = max_y.max(p.y);
                }
            }
        }

        if !has_outline {
            return; 
        }

        // Iterate through footprints checking if their pos falls outside the rect
        for fp in &self.pcb.footprints {
            if fp.pos.x < min_x || fp.pos.x > max_x || fp.pos.y < min_y || fp.pos.y > max_y {
                violations.push(DRCViolation {
                    level: DRCLevel::Warning,
                    title: "Footprint outside PCB outline".to_string(),
                    description: format!(
                        "Footprint '{}' is located at ({:.2}, {:.2}) which is outside the board bounds.",
                        Self::get_reference(fp), fp.pos.x, fp.pos.y
                    ),
                    position: Pt::from(fp.pos),
                    markers: vec![],
                });
            }
        }
    }

    /// Verifies that all pads of the same net are connected physically via copper tracks or vias
    fn check_connectivity(&self, violations: &mut Vec<DRCViolation>) {
        // 1. Group items by net ordinal (0 means unconnected/no-net)
        let mut net_pads: HashMap<u32, Vec<&Pad>> = HashMap::new();
        for fp in &self.pcb.footprints {
            for pad in &fp.pads {
                if pad.net.ordinal > 0 {
                    net_pads.entry(pad.net.ordinal).or_default().push(pad);
                }
            }
        }

        let mut net_segments = HashMap::new();
        for seg in &self.pcb.segments {
            if seg.net > 0 {
                net_segments.entry(seg.net).or_insert_with(Vec::new).push(seg);
            }
        }

        let mut net_vias = HashMap::new();
        for via in &self.pcb.vias {
            if via.net > 0 {
                net_vias.entry(via.net).or_insert_with(Vec::new).push(via);
            }
        }

        // 2. Process each net
        for (net_id, pads) in net_pads {
            if pads.is_empty() {
                continue;
            }

            let mut dsu = DisjointSet::new();
            let mut routing_points = Vec::new();

            // Connect track segments to each other
            if let Some(segments) = net_segments.get(&net_id) {
                for seg in segments {
                    dsu.union(GridPt::from(seg.start), GridPt::from(seg.end));
                    routing_points.push(seg.start);
                    routing_points.push(seg.end);
                }
            }

            // Connect vias to routing points
            if let Some(vias) = net_vias.get(&net_id) {
                for via in vias {
                    routing_points.push(Pt::from(via.pos));
                }
            }

            // Map each pad to routing points that intersect it
            for pad in &pads {
                let pad_grid_pt = GridPt::from(pad.pos);
                let mut is_connected = false;

                for &pt in &routing_points {
                    if self.point_in_pad(pt, pad) {
                        dsu.union(pad_grid_pt, GridPt::from(pt));
                        is_connected = true;
                    }
                }

                if !is_connected && pads.len() > 1 {
                    let fp_ref = self.pcb.footprints.iter().find(|f| f.pads.iter().any(|p| p.uuid == pad.uuid)).map(|f| Self::get_reference(f)).unwrap_or_default();
                    
                    // Completely physically unconnected pin
                    violations.push(DRCViolation {
                        level: DRCLevel::Error,
                        title: "Unconnected Pin".to_string(),
                        description: format!(
                            "Pad {}-{} (Net {}) is missing physical track connections.",
                            fp_ref, pad.number, pad.net.name
                        ),
                        position: Pt::from(pad.pos),
                        markers: vec![],
                    });
                }
            }

            // 3. Find if the net is split into multiple "islands"
            if pads.len() > 1 {
                let mut distinct_roots = HashSet::new();
                for pad in &pads {
                    let root = dsu.find(GridPt::from(pad.pos));
                    distinct_roots.insert(root);
                }

                // If there's more than 1 root group, the net is unrouted/broken
                if distinct_roots.len() > 1 {
                    violations.push(DRCViolation {
                        level: DRCLevel::Error,
                        title: "Unrouted Net / Broken Topology".to_string(),
                        description: format!(
                            "Net '{}' is physically broken into {} unconnected segments/islands.",
                            pads[0].net.name, distinct_roots.len()
                        ),
                        position: Pt::from(pads[0].pos),
                        markers: pads.iter().map(|p| Pt::from(p.pos)).collect(),
                    });
                }
            }
        }
    }

    /// Helper: Checks if a generic Pt is geometrically inside a Pad's area
    fn point_in_pad(&self, pt: Pt, pad: &Pad) -> bool {
        let dx = pt.x - pad.pos.x;
        let dy = pt.y - pad.pos.y;

        // Rotate point by -pad.angle to align with pad's local axes
        let angle_rad = -pad.pos.angle.to_radians();
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin();

        let local_x = dx * cos_a - dy * sin_a;
        let local_y = dx * sin_a + dy * cos_a;

        // Approximate pad as a rectangle for collision detection. 
        let half_w = pad.size.0 / 2.0;
        let half_h = pad.size.1 / 2.0;

        local_x.abs() <= half_w && local_y.abs() <= half_h
    }
}
