use std::collections::{HashMap, HashSet};
use netlist::Netlist;
use types::{
    gr::{GridPt, Pt},
    constants::el,
};

use models::{
    geometry::pin_position, schema::{Schema, SchemaItem, Symbol}, symbols::ElectricalTypes
};

pub fn erc(schema: &Schema) -> Vec<ERCViolation> {
    Erc::new(schema).run()
}

#[derive(Debug, Clone, PartialEq)]
pub enum ERCLevel {
    Error,
    Warning,
}

#[derive(Debug, Clone)]
pub struct ERCViolation {
    pub level: ERCLevel,
    pub title: String,
    pub description: String,
    pub position: Pt,
    pub markers: Vec<Pt>,
}

/// The result of comparing two pins
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConnectionResult {
    Ok,
    Warning,
    Error,
}

trait ErcCheck {
    fn check_connection(&self, other: &ElectricalTypes) -> ConnectionResult;
    fn is_driver(&self) -> bool;
}

impl ErcCheck for ElectricalTypes {
    fn check_connection(&self, other: &ElectricalTypes) -> ConnectionResult {
        use ConnectionResult::*;
        use ElectricalTypes::*;

        match (self, other) {
            // Error cases
            (Output, Output) => Error,
            (Output, PowerOut) => Error,
            (PowerOut, PowerOut) => Error,
            (OpenEmitter, Output) => Error,

            // Warning cases
            (Output, Bidirectional) => Warning,
            (PowerOut, Bidirectional) => Warning,
            (TriState, Output) => Warning,

            // Unconnected Logic
            (Unspecified, _) => Warning,
            (_, Unspecified) => Warning,

            // Passive usually connects fine
            (Passive, _) | (_, Passive) => Ok,

            // Default OK
            _ => Ok,
        }
    }

    /// Does this pin drive the net?
    fn is_driver(&self) -> bool {
        matches!(
            self,
            ElectricalTypes::Output
                | ElectricalTypes::PowerOut
                | ElectricalTypes::OpenCollector
                | ElectricalTypes::OpenEmitter
                | ElectricalTypes::Bidirectional
                | ElectricalTypes::TriState
        )
    }
}

pub struct Erc<'a> {
    schema: &'a Schema,
    netlist: Netlist,
}

impl<'a> Erc<'a> {
    pub fn new(schema: &'a Schema) -> Self {
        let netlist = Netlist::from(schema);
        Self { schema, netlist }
    }

    pub fn run(&self) -> Vec<ERCViolation> {
        let mut violations = Vec::new();

        // Build a set of No Connect locations for fast lookup
        let mut no_connects: HashSet<GridPt> = HashSet::new();
        for item in &self.schema.items {
            if let SchemaItem::NoConnect(nc) = item {
                no_connects.insert(nc.pos.into());
            }
        }

        // 1. Check Symbols
        violations.append(&mut self.check_symbols());

        // 2. Iterate over every Net
        for (_net_id, net_data) in &self.netlist.nets {
            let mut pins_on_net = Vec::new();

            for (ref_des, pin_number) in &net_data.pins {
                let pos_opt = self.resolve_pin_pos(ref_des, pin_number);

                // SKIP Check: If this pin is explicitly marked No Connect
                if let Some(pos) = pos_opt {
                    if no_connects.contains(&pos.into()) {
                        continue;
                    }
                }

                let pin_type = self
                    .resolve_pin_type(ref_des, pin_number)
                    .unwrap_or(ElectricalTypes::Unspecified);

                pins_on_net.push((ref_des, pin_number, pin_type, pos_opt));
            }

            // Pin-to-Pin Conflict Check
            for (i, (ref1, pin1, type1, pos1)) in pins_on_net.iter().enumerate() {
                for (ref2, pin2, type2, pos2) in pins_on_net.iter().skip(i + 1) {
                    match type1.check_connection(type2) {
                        ConnectionResult::Error => {
                            violations.push(ERCViolation {
                                level: ERCLevel::Error,
                                title: "Pin Conflict".to_string(),
                                description: format!(
                                    "Pin {}-{} ({}) connected to Pin {}-{} ({})",
                                    ref1, pin1, type1, ref2, pin2, type2
                                ),
                                position: pos1.unwrap_or(Pt { x: 0.0, y: 0.0 }),
                                markers: vec![pos2.unwrap_or(Pt { x: 0.0, y: 0.0 })],
                            });
                        }
                        ConnectionResult::Warning => {
                            violations.push(ERCViolation {
                                level: ERCLevel::Warning,
                                title: "Pin Conflict".to_string(),
                                description: format!(
                                    "Pin {}-{} ({}) connected to Pin {}-{} ({})",
                                    ref1, pin1, type1, ref2, pin2, type2
                                ),
                                position: pos1.unwrap_or(Pt { x: 0.0, y: 0.0 }),
                                markers: vec![pos2.unwrap_or(Pt { x: 0.0, y: 0.0 })],
                            });
                        }
                        ConnectionResult::Ok => {}
                    }
                }
            }

            // 2b. Missing Driver Check (Logic Update)
            let has_input = pins_on_net
                .iter()
                .any(|(_, _, t, _)| *t == ElectricalTypes::Input);
            let has_power_in = pins_on_net
                .iter()
                .any(|(_, _, t, _)| *t == ElectricalTypes::PowerIn);
            let has_driver = pins_on_net.iter().any(|(_, _, t, _)| t.is_driver());

            // Fix: Passives suppress "Net not driven" for Inputs
            let has_passive = pins_on_net
                .iter()
                .any(|(_, _, t, _)| *t == ElectricalTypes::Passive);

            if has_input && !has_driver && !has_passive {
                // ... [Report Warning] ...
                violations.push(ERCViolation {
                    level: ERCLevel::Warning,
                    title: "Net not driven".to_string(),
                    description: format!(
                        "Net '{}' has inputs but no driving output.",
                        net_data.name
                    ),
                    position: pins_on_net[0].3.unwrap_or(Pt { x: 0.0, y: 0.0 }),
                    markers: vec![],
                });
            }

            if has_power_in && !has_driver {
                // For Power Inputs, we usually DON'T suppress via Passive (e.g. connecting a resistor to VCC pin is usually wrong unless it's a specific pull-up, but strict ERC demands a supply).
                // However, we MUST rely on the Netlist merge fix above to see the PWR_FLAG.

                // ... [Report Error] ...
                violations.push(ERCViolation {
                    level: ERCLevel::Error,
                    title: "Power Input not driven".to_string(),
                    description: format!(
                        "Net '{}' connects to Power Input but has no Power Output or PWR_FLAG.",
                        net_data.name
                    ),
                    position: pins_on_net[0].3.unwrap_or(Pt { x: 0.0, y: 0.0 }),
                    markers: vec![],
                });
            }
        }

        violations
    }

    fn resolve_pin_type(&self, ref_des: &str, pin_number: &str) -> Option<ElectricalTypes> {
        // We just need the Library Symbol to find the type, any instance of the symbol will do to find the LibID
        let symbol = self.schema.symbol_by_ref(ref_des)?;
        let lib_symbol = self.schema.library_symbol(&symbol.lib_id)?;
        let pin = lib_symbol.pin(pin_number)?;
        Some(pin.electrical_type)
    }

    fn resolve_pin_pos(&self, ref_des: &str, pin_number: &str) -> Option<Pt> {
        // 1. Get the LibID from any instance
        let any_symbol = self.schema.symbol_by_ref(ref_des)?;

        // 2. Get the Library Symbol
        let lib_symbol = self.schema.library_symbol(&any_symbol.lib_id)?;

        // 3. Find out which Unit this specific pin belongs to
        let unit_idx = lib_symbol.pin_unit(pin_number)?;

        // 4. Find the specific placed Symbol instance for that Unit
        let placed_symbol = self.schema.symbol(ref_des, unit_idx)?;

        // 5. Get the pin definition
        let pin = lib_symbol.pin(pin_number)?;

        // 6. Calculate position based on the CORRECT unit's position
        Some(pin_position(placed_symbol, pin))
    }

    fn check_symbols(&self) -> Vec<ERCViolation> {
        let mut violations = Vec::new();
        let mut symbol_groups: HashMap<String, Vec<&Symbol>> = HashMap::new();

        for item in &self.schema.items {
            if let SchemaItem::Symbol(sym) = item {
                if let Some(ref_des) = sym.property(el::PROPERTY_REFERENCE) {
                    if ref_des.ends_with('?') || ref_des.starts_with('#') {
                        continue;
                    }
                    symbol_groups.entry(ref_des).or_default().push(sym);
                } else {
                    spdlog::warn!("erc::check_symbols: reference for symbol not found", );
                }
            }
        }

        for (ref_des, symbols) in symbol_groups {
            violations.append(&mut self.check_symbol_group(&ref_des, &symbols));
        }

        violations
    }

    fn check_symbol_group(
        &self,
        ref_des: &str,
        placed_symbols: &[&Symbol],
    ) -> Vec<ERCViolation> {
        let mut violations = Vec::new();
        let mut seen_units: HashMap<u8, &Symbol> = HashMap::new();

        for sym in placed_symbols {
            if let Some(existing) = seen_units.get(&sym.unit) {
                violations.push(ERCViolation {
                    level: ERCLevel::Error,
                    title: "Duplicate Symbol Unit".to_string(),
                    description: format!(
                        "Reference '{}' Unit {} appears multiple times.",
                        ref_des, sym.unit
                    ),
                    position: Pt {
                        x: sym.pos.x,
                        y: sym.pos.y,
                    },
                    markers: vec![Pt {
                        x: existing.pos.x,
                        y: existing.pos.y,
                    }],
                });
            } else {
                seen_units.insert(sym.unit, sym);
            }
        }

        if let Some(first_sym) = placed_symbols.first() {
            if let Some(lib_sym) = self.schema.library_symbol(&first_sym.lib_id) {
                let mut defined_units: HashSet<u8> = HashSet::new();
                for unit_def in &lib_sym.units {
                    let u = unit_def.unit();
                    if u > 0 {
                        defined_units.insert(u);
                    }
                }

                let placed_unit_indices: HashSet<u8> =
                    placed_symbols.iter().map(|s| s.unit).collect();

                for unit in defined_units {
                    if !placed_unit_indices.contains(&unit) {
                        violations.push(ERCViolation {
                            level: ERCLevel::Warning,
                            title: "Missing Symbol Unit".to_string(),
                            description: format!(
                                "Reference '{}' has defined units, but Unit {} is not placed.",
                                ref_des, unit
                            ),
                            position: Pt {
                                x: first_sym.pos.x,
                                y: first_sym.pos.y,
                            },
                            markers: vec![],
                        });
                    }
                }
            }
        }

        violations
    }
}
