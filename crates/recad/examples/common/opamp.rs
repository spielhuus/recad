use std::collections::HashMap;

use recad::draw::*;
use recad::{Schema, RecadError, Pt};

pub fn draw() -> Result<Schema, RecadError> {
    // 1. Initialize the Schema Builder
    // This creates an empty schematic canvas.
    let mut builder = SchemaBuilder::new("Inverting_Amplifier");

    // 2. Place the Operational Amplifier (U1)
    // We place it at a specific absolute coordinate (X: 100, Y: 100)
    let opamp = Symbol::new("U1", "TL072", "Amplifier_Operational:TL072")
        .attr(Attribute::At(At::Pt(Pt { x: 50.0*2.54, y: 50.0*2.54 })))
        .attr(Attribute::Unit(1))
        .attr(Attribute::Mirror("x".to_string()))
        .attr(Attribute::Property(HashMap::from([
            ("Sim.Name".to_string(), "TL072c".to_string())
        ])));
    builder.draw(opamp)?;

    // 3. Ground the Non-Inverting Input (Pin 3)
    // Move the drawing cursor exactly to U1's Pin 3, draw a wire down, and place GND
    builder.move_to(At::Pin("U1".to_string(), "3".to_string()));
    
    builder.draw(Symbol::new("#PWRGND", "GND", "power:GND"))?;

    // 4. Create the Input Path (VIN -> R1 -> U1 Pin 2)
    builder.move_to(At::Pin("U1".to_string(), "2".to_string()));
    
    // Draw a junction at Pin 2 because the feedback loop will connect here
    builder.draw(Junction::new())?;

    // Draw wire leftwards from the op-amp
    builder.draw(
        Wire::new()
            .attr(Attribute::Direction(Direction::Left))
            .attr(Attribute::Length(10.0)),
    )?;

    // Place the Input Resistor (R1)
    // We rotate it 90 degrees to make it horizontal, and anchor its Pin 2 to the current wire end
    builder.draw(
        Symbol::new("R1", "100k", "Device:R")
            .attr(Attribute::Rotate(90.0))
            .attr(Attribute::Anchor("2".to_string())),
    )?;

    // Draw a small wire leftwards from R1's Pin 1 to the input label
    builder.draw(
        Wire::new()
            .attr(Attribute::Direction(Direction::Left))
            .attr(Attribute::Length(5.0)),
    )?;
    
    builder.draw(
        GlobalLabel::new("VIN").attr(Attribute::Rotate(180.0)), // Point label outwards
    )?;


    // 5. Create the Output Path (U1 Pin 1 -> VOUT)
    builder.move_to(At::Pin("U1".to_string(), "1".to_string()));
    
    // Draw a junction for the other side of the feedback loop
    builder.draw(Junction::new())?;

    // Draw a wire rightwards
    builder.draw(
        Wire::new()
            .attr(Attribute::Direction(Direction::Right))
            .attr(Attribute::Length(10.0)),
    )?;
    
    builder.draw(GlobalLabel::new("VOUT"))?;


    // 6. Draw the Feedback Loop (R2)
    // The `Feedback` tool automatically draws a rectangular wire loop between two pins
    // and inserts a component along the horizontal segment.
    let r2 = Symbol::new("R2", "100k", "Device:R").attr(Attribute::Rotate(90.0));

    let mut fb = Feedback::new();
    fb.atref = Some(("U1".to_string(), "2".to_string())); // Start at inverting input
    fb.toref = Some(("U1".to_string(), "1".to_string())); // End at output
    fb.height = 15.0; // Draw the wire loop 15mm ABOVE the op-amp (Negative Y is Up)
    fb.with = Some(r2); // Insert R2 in the middle of the loop
    builder.draw(fb)?;

    let opamp = Symbol::new("U1", "TL072", "Amplifier_Operational:TL072")
        .attr(Attribute::At(At::Pt(Pt { x: 50.0*2.54, y: 60.0*2.54 })))
        .attr(Attribute::Unit(2))
        .attr(Attribute::Property(HashMap::from([
            ("Sim.Name".to_string(), "TL072c".to_string())
        ])));
    builder.draw(opamp)?;
    builder.draw(NoConnect::new().attr(Attribute::At(At::Pin("U1".to_string(), "5".to_string()))))?;
    builder.draw(NoConnect::new().attr(Attribute::At(At::Pin("U1".to_string(), "6".to_string()))))?;
    builder.draw(NoConnect::new().attr(Attribute::At(At::Pin("U1".to_string(), "7".to_string()))))?;

    let opamp_power = Symbol::new("U1", "TL072", "Amplifier_Operational:TL072")
        .attr(Attribute::At(At::Pt(Pt { x: 60.0*2.54, y: 60.0*2.54 })))
        .attr(Attribute::Unit(3))
        .attr(Attribute::Property(HashMap::from([
            ("Sim.Name".to_string(), "TL072c".to_string())
        ])));
    builder.draw(opamp_power)?;
    builder.draw(Symbol::new("+15V", "+15V", "power:+15V")
        .attr(Attribute::At(At::Pin("U1".to_string(), "8".to_string()))))?;
    builder.draw(Symbol::new("-15V", "-15V", "power:-15V")
        .attr(Attribute::At(At::Pin("U1".to_string(), "4".to_string())))
        .attr(Attribute::Rotate(180.0)))?;
    builder.finalize().cloned()
}
