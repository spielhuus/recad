use recad::{
    Pos, Pt, Schema, reports::ERCLevel, schema::{ElectricalTypes, LibrarySymbol, Pin, PinProperty, SchemaItem, Symbol}
};
use reports::erc::erc;

/// Helper to build a standard Library Symbol with a list of pins.
/// Pins are assigned to Unit 1, Style 1 to match default symbol behavior.
fn build_lib_sym(name: &str, pins: Vec<(String, ElectricalTypes)>) -> LibrarySymbol {
    // lib_id format: "Name_Unit_Style"
    let unit_lib_id = format!("{}_1_1", name);

    let unit_pins: Vec<Pin> = pins
        .into_iter()
        .map(|(pname, ptype)| Pin {
            number: PinProperty {
                name: pname,
                ..Default::default()
            },
            electrical_type: ptype,
            pos: Pos::default(), // All pins at origin to ensure connection when symbols overlap
            ..Default::default()
        })
        .collect();

    let unit_sym = LibrarySymbol {
        lib_id: unit_lib_id,
        pins: unit_pins,
        // Default required fields
        extends: None,
        power: false,
        pin_numbers: true,
        pin_names: true,
        in_bom: true,
        on_board: true,
        exclude_from_sim: false,
        props: vec![],
        graphics: vec![],
        pin_names_offset: None,
        units: vec![],
        unit_name: None,
        embedded_fonts: None,
        ..Default::default()
    };

    LibrarySymbol {
        lib_id: name.to_string(),
        units: vec![unit_sym],
        pins: vec![],
        extends: None,
        power: false,
        pin_numbers: true,
        pin_names: true,
        in_bom: true,
        on_board: true,
        exclude_from_sim: false,
        props: vec![],
        graphics: vec![],
        pin_names_offset: None,
        unit_name: None,
        embedded_fonts: None,
        ..Default::default()
    }
}

/// Helper to build a multi-unit symbol (e.g., a Dual OpAmp).
/// Creates `units_count` units, each with one Passive pin.
fn build_multi_unit_sym(name: &str, units_count: u8) -> LibrarySymbol {
    let mut units = Vec::new();
    for i in 1..=units_count {
        let unit_lib_id = format!("{}_{}_1", name, i);
        let pin = Pin {
            number: PinProperty {
                name: format!("P{}", i),
                ..Default::default()
            },
            electrical_type: ElectricalTypes::Passive,
            pos: Pos::default(),
            ..Default::default()
        };
        units.push(LibrarySymbol {
            lib_id: unit_lib_id,
            pins: vec![pin],
            extends: None,
            power: false,
            pin_numbers: true,
            pin_names: true,
            in_bom: true,
            on_board: true,
            exclude_from_sim: false,
            props: vec![],
            graphics: vec![],
            pin_names_offset: None,
            units: vec![],
            unit_name: None,
            embedded_fonts: None,
        ..Default::default()
        });
    }

    LibrarySymbol {
        lib_id: name.to_string(),
        units,
        pins: vec![],
        extends: None,
        power: false,
        pin_numbers: true,
        pin_names: true,
        in_bom: true,
        on_board: true,
        exclude_from_sim: false,
        props: vec![],
        graphics: vec![],
        pin_names_offset: None,
        unit_name: None,
        embedded_fonts: None,
        ..Default::default()
    }
}

/// Helper to place a symbol instance into the schema.
fn add_symbol(schema: &mut Schema, ref_des: &str, lib_id: &str, unit: u8, pos: Pt) {
    let mut sym = Symbol::new(ref_des, "Val", lib_id);
    sym.unit = unit;
    sym.pos = Pos {
        x: pos.x,
        y: pos.y,
        angle: 0.0,
    };
    schema.items.push(SchemaItem::Symbol(sym));
}

#[test]
fn test_erc_output_conflict() {
    let mut schema = Schema::new("test", None);

    // Create a "Driver" symbol with one Output pin
    let lib_sym = build_lib_sym("Driver", vec![("1".to_string(), ElectricalTypes::Output)]);
    schema.library_symbols.push(lib_sym);

    // Place two drivers at the same location.
    // Overlapping pins are considered connected by the Netlist extractor.
    add_symbol(&mut schema, "U1", "Driver", 1, Pt { x: 100.0, y: 100.0 });
    add_symbol(&mut schema, "U2", "Driver", 1, Pt { x: 100.0, y: 100.0 });

    let violations = erc(&schema);

    // Expect an Error because two Outputs are driving the same net.
    let has_conflict = violations.iter().any(|v| v.level == ERCLevel::Error);
    assert!(
        has_conflict,
        "Expected Output-Output conflict error, found: {:?}",
        violations
    );
}

#[test]
fn test_erc_input_floating() {
    let mut schema = Schema::new("test", None);

    // Create an "Inputter" symbol with one Input pin
    let lib_sym = build_lib_sym("Inputter", vec![("1".to_string(), ElectricalTypes::Input)]);
    schema.library_symbols.push(lib_sym);

    // Place one input symbol unconnected
    add_symbol(&mut schema, "U1", "Inputter", 1, Pt { x: 100.0, y: 100.0 });

    let violations = erc(&schema);

    // Expect a Warning (Net not driven)
    let has_warning = violations.iter().any(|v| v.title == "Net not driven");
    assert!(
        has_warning,
        "Expected 'Net not driven' warning for floating input, found: {:?}",
        violations
    );
}

#[test]
fn test_erc_power_input_floating() {
    let mut schema = Schema::new("test", None);

    // Create a "MCU" symbol with one Power Input pin
    let lib_sym = build_lib_sym("MCU", vec![("1".to_string(), ElectricalTypes::PowerIn)]);
    schema.library_symbols.push(lib_sym);

    // Place one MCU symbol unconnected
    add_symbol(&mut schema, "U1", "MCU", 1, Pt { x: 100.0, y: 100.0 });

    let violations = erc(&schema);

    // Expect an Error (Power Input requires a driver)
    let has_error = violations
        .iter()
        .any(|v| v.title == "Power Input not driven");
    assert!(
        has_error,
        "Expected 'Power Input not driven' error, found: {:?}",
        violations
    );
}

#[test]
fn test_erc_duplicate_unit() {
    let mut schema = Schema::new("test", None);

    // Create a dual-unit symbol
    let lib_sym = build_multi_unit_sym("Dual", 2);
    schema.library_symbols.push(lib_sym);

    // Place Unit 1 of U1 twice
    add_symbol(&mut schema, "U1", "Dual", 1, Pt { x: 10.0, y: 10.0 });
    add_symbol(&mut schema, "U1", "Dual", 1, Pt { x: 20.0, y: 20.0 });

    let violations = erc(&schema);

    // Expect Error for duplicate unit
    let has_error = violations
        .iter()
        .any(|v| v.title == "Duplicate Symbol Unit");
    assert!(
        has_error,
        "Expected 'Duplicate Symbol Unit' error, found: {:?}",
        violations
    );
}

#[test]
fn test_erc_missing_unit() {
    let mut schema = Schema::new("test", None);

    // Create a dual-unit symbol
    let lib_sym = build_multi_unit_sym("Dual", 2);
    schema.library_symbols.push(lib_sym);

    // Place ONLY Unit 1 of U1 (Unit 2 is missing)
    add_symbol(&mut schema, "U1", "Dual", 1, Pt { x: 10.0, y: 10.0 });

    let violations = erc(&schema);

    // Expect Warning for missing unit
    let has_warning = violations.iter().any(|v| v.title == "Missing Symbol Unit");
    assert!(
        has_warning,
        "Expected 'Missing Symbol Unit' warning, found: {:?}",
        violations
    );
}

#[test]
fn test_erc_clean_connection() {
    let mut schema = Schema::new("test", None);

    // Create Driver (Output) and Receiver (Input)
    let drv = build_lib_sym("Driver", vec![("1".to_string(), ElectricalTypes::Output)]);
    let rx = build_lib_sym("Receiver", vec![("1".to_string(), ElectricalTypes::Input)]);
    schema.library_symbols.push(drv);
    schema.library_symbols.push(rx);

    // Connect them by placing them at the same coordinate
    add_symbol(&mut schema, "U1", "Driver", 1, Pt { x: 50.0, y: 50.0 });
    add_symbol(&mut schema, "U2", "Receiver", 1, Pt { x: 50.0, y: 50.0 });

    let violations = erc(&schema);

    assert!(
        violations.is_empty(),
        "Expected no violations for valid Output->Input connection, found: {:?}",
        violations
    );
}

#[test]
fn test_erc_passive_suppression() {
    let mut schema = Schema::new("test", None);

    // Create Resistor (Passive) and Receiver (Input)
    let res = build_lib_sym(
        "Resistor",
        vec![("1".to_string(), ElectricalTypes::Passive)],
    );
    let rx = build_lib_sym("Receiver", vec![("1".to_string(), ElectricalTypes::Input)]);
    schema.library_symbols.push(res);
    schema.library_symbols.push(rx);

    // Connect Passive to Input
    add_symbol(&mut schema, "R1", "Resistor", 1, Pt { x: 50.0, y: 50.0 });
    add_symbol(&mut schema, "U1", "Receiver", 1, Pt { x: 50.0, y: 50.0 });

    let violations = erc(&schema);

    // Standard behavior: Passive components suppress "Net not driven" warnings for inputs
    let has_warning = violations.iter().any(|v| v.title == "Net not driven");
    assert!(
        !has_warning,
        "Passive component should suppress 'Net not driven' warning"
    );
}
