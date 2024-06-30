mod tests {
    mod parser {
        use std::{fs::File, io::Write, path::Path};

        use recad::{draw::{At, Label, Symbol, Wire}, gr::Pt, plot::{theme::{Theme, Themes}, SvgPlotter, Plotter}, Drawer, Plot, Schema};
        fn init() {
            let _ = env_logger::builder().is_test(true).try_init();
        }
        #[test]

        //with schemdraw.Drawing() as d:
        //    op = elm.Opamp(leads=True)
        //    elm.Line().down(d.unit/4).at(op.in2)
        //    elm.Ground(lead=False)
        //    Rin = elm.Resistor().at(op.in1).left().idot().label('$R_{in}$', loc='bot').label('$v_{in}$', loc='left')
        //    elm.Line().up(d.unit/2).at(op.in1)
        //    elm.Resistor().tox(op.out).label('$R_f$')
        //    elm.Line().toy(op.out).dot()
        //    elm.Line().right(d.unit/4).at(op.out).label('$v_{o}$', loc='right')


        fn draw_schema() {
            init();

            let schema = Schema::load(Path::new("tests/echo/echo.kicad_sch")).unwrap();
            let mut file = std::fs::File::create("/tmp/summe.kicad_sch").unwrap();
            schema.write(&mut file).unwrap();
            let mut builder = Schema::new()
                .move_to(At::Pt(Pt { x: 50.8, y: 50.8 }))
                .draw(Label::new("Vin").rotate(180.0))
                .draw(Wire::new().right().len(4.0))
                .draw(Wire::new().up().len(4.0))
                .draw(Wire::new().right().len(4.0))
                .draw(Symbol::new("R1", "100k", "Device:R")
                    .rotate(90.0)
                    .anchor("1"))
                .draw(Wire::new().right())
                .draw(Symbol::new("U1", "TL072", "Amplifier_Operational:LM2904")
                    .anchor("3"))
                .draw(Wire::new().up().len(4.0));
            
            //builder.write(&mut std::io::stdout()).unwrap();
            let mut file = File::create("/tmp/test_builder.kicad_sch").unwrap();
            builder.write(&mut file).unwrap();

            let mut svg = SvgPlotter::new();
            let res = builder.plot(&mut svg, &Theme::from(Themes::Kicad2020));
            let mut file = File::create("/tmp/test_builder.svg").unwrap();
            svg.write(&mut file).unwrap();
        }
    }
}

