use std::{fs::File, path::Path};

use draw::{Junction, NoConnect, SchemaBuilder, Symbol, Wire};
use recad::{
    draw::{At, Attribute, Direction, Drawable, Drawer},
    Pt,
    plot::{Themes, PlotCommand, Plot, Plotter, SvgPlotter},
};
fn init() {
    crate::common::setup();
    let path = Path::new("target/out");
    if !path.exists() {
        std::fs::create_dir_all(path).unwrap();
    }
}

#[test]
fn draw_opamp() {
    init();

    let mut builder = SchemaBuilder::new("test_opamp");
    builder.move_to(At::Pt(Pt { x: 50.8, y: 50.8 }));
    builder
        .draw(Symbol::new("TP1", "Vin", "Connector:TestPoint").attr(Attribute::Rotate(90.0)))
        .unwrap();
    builder
        .draw(Symbol::new("R1", "100k", "Device:R").attr(Attribute::Rotate(90.0)))
        .unwrap();
    builder.draw(Junction::new()).unwrap();
    builder
        .draw(
            Symbol::new("U1", "TL072", "Amplifier_Operational:TL072")
                .attr(Attribute::Anchor("2".to_string()))
                .attr(Attribute::Mirror("x".to_string())),
        )
        .unwrap();
    builder.draw(Junction::new()).unwrap();
    builder
        .draw(Symbol::new("TP2", "Vout", "Connector:TestPoint").attr(Attribute::Rotate(270.0)))
        .unwrap();
    builder
        .draw(
            Wire::new()
                .attr(Attribute::At(At::Pin("U1".to_string(), "2".to_string())))
                .attr(Attribute::Direction(Direction::Up))
                .attr(Attribute::Length(4.0 * 2.54)),
        )
        .unwrap();
    builder
        .draw(
            Symbol::new("R2", "100k", "Device:R")
                .attr(Attribute::Rotate(90.0))
                .attr(Attribute::Anchor("1".to_string()))
                .attr(Attribute::Tox(At::Pin("U1".to_string(), "1".to_string()))),
        )
        .unwrap();
    builder
        .draw(Wire::new().attr(Attribute::Toy(At::Pin("U1".to_string(), "1".to_string()))))
        .unwrap();
    builder.draw(Junction::new()).unwrap();
    builder
        .draw(
            Symbol::new("GND", "GND", "power:GND")
                .attr(Attribute::At(At::Pin("U1".to_string(), "3".to_string()))),
        )
        .unwrap();

    builder
        .draw(
            Symbol::new("U1", "TL072", "Amplifier_Operational:TL072")
                .attr(Attribute::At(At::Pt(Pt {
                    x: 2.54 * 25.0,
                    y: 2.54 * 30.0,
                })))
                .attr(Attribute::Unit(2)),
        )
        .unwrap();
    builder
        .draw(NoConnect::new().attr(Attribute::At(At::Pin("U1".to_string(), "5".to_string()))))
        .unwrap();
    builder
        .draw(NoConnect::new().attr(Attribute::At(At::Pin("U1".to_string(), "6".to_string()))))
        .unwrap();
    builder
        .draw(NoConnect::new().attr(Attribute::At(At::Pin("U1".to_string(), "7".to_string()))))
        .unwrap();

    builder
        .draw(
            Symbol::new("U1", "TL072", "Amplifier_Operational:TL072")
                .attr(Attribute::At(At::Pt(Pt {
                    x: 2.54 * 35.0,
                    y: 2.54 * 30.0,
                })))
                .attr(Attribute::Unit(3)),
        )
        .unwrap();
    builder
        .draw(
            Symbol::new("+15V", "+15V", "power:+15V")
                .attr(Attribute::At(At::Pin("U1".to_string(), "8".to_string()))),
        )
        .unwrap();
    builder
        .draw(
            Symbol::new("-15V", "-15V", "power:-15V")
                .attr(Attribute::At(At::Pin("U1".to_string(), "4".to_string())))
                .attr(Attribute::Rotate(180.0)),
        )
        .unwrap();

    // add the power flags
    builder
        .draw(
            Symbol::new("+15V", "+15V", "power:+15V").attr(Attribute::At(At::Pt(Pt {
                x: 2.54 * 36.0,
                y: 2.54 * 30.0,
            }))),
        )
        .unwrap();
    builder
        .draw(Symbol::new("PWR_FLAG", "PWR_FLAG", "power:PWR_FLAG"))
        .unwrap();
    builder
        .draw(
            Symbol::new("-15V", "-15V", "power:-15V")
                .attr(Attribute::At(At::Pt(Pt {
                    x: 2.54 * 37.0,
                    y: 2.54 * 30.0,
                })))
                .attr(Attribute::Rotate(180.0)),
        )
        .unwrap();
    builder
        .draw(Symbol::new("PWR_FLAG", "PWR_FLAG", "power:PWR_FLAG"))
        .unwrap();
    builder
        .draw(
            Symbol::new("GND", "GND", "power:GND").attr(Attribute::At(At::Pt(Pt {
                x: 2.54 * 38.0,
                y: 2.54 * 30.0,
            }))),
        )
        .unwrap();
    builder
        .draw(Symbol::new("PWR_FLAG", "PWR_FLAG", "power:PWR_FLAG"))
        .unwrap();

    let mut file = File::create("target/out/test_draw_opamp.kicad_sch").unwrap();
    builder.schema.write(&mut file).unwrap();

    let mut svg = SvgPlotter::new();
    builder.schema
        .plot(
            &mut svg,
            &PlotCommand::new()
                .theme(Some(Themes::Kicad2020))
                .border(Some(false))
                .scale(Some(10.0)),
        )
        .unwrap();
    svg.save(Path::new("target/out/test_draw_opamp.svg"))
        .unwrap();
}
