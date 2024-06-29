mod builder;

mod tests {
    mod parser {
        use std::path::Path;
        fn init() {
            let _ = env_logger::builder().is_test(true).try_init();
        }
        #[test]
        fn load_schema() {
            init();
            let schema = recad::Schema::load(Path::new("tests/summe.kicad_sch"));
            let schema = recad::Schema::load(Path::new("tests/echo/echo.kicad_sch"));
                  
            //let schema = crate::Schema::load(Path::new("/usr/share/kicad/demos/kit-dev-coldfire-xilinx_5213/kit-dev-coldfire-xilinx_5213.kicad_sch"));
            //let schema = crate::schema::Schema::load(Path::new("/home/etienne/github/elektrophon/src/threeler/main/main.kicad_sch"));
            
            let mut file = std::fs::File::create("/tmp/summe.kicad_sch").unwrap();
            schema.write2(&mut file).unwrap();

            let svg = recad::plot::SvgPlotter::new();
            let mut plotter = recad::plot::SchemaPlotter::new(schema, svg, recad::plot::theme::Themes::Kicad2020);
            let mut file = std::fs::File::create("/tmp/summe.svg").unwrap();
            plotter.plot();
            plotter.write(&mut file).unwrap();

        }
        #[test]
        fn load_pcb() {
            init();
            let pcb = recad::Pcb::load(Path::new("tests/echo/echo.kicad_pcb"));
 
            assert_eq!(254, pcb.segments.len());
            assert_eq!(51, pcb.nets.len());
            assert_eq!(70, pcb.footprints.len());
            //let schema = crate::Schema::load(Path::new("/usr/share/kicad/demos/kit-dev-coldfire-xilinx_5213/kit-dev-coldfire-xilinx_5213.kicad_sch"));
            //let schema = crate::schema::Schema::load(Path::new("/home/etienne/github/elektrophon/src/threeler/main/main.kicad_sch"));
            
            let svg = recad::plot::SvgPlotter::new();
            //let mut plotter = recad::plot::SchemaPlotter::new(schema, svg, recad::plot::theme::Themes::Kicad2020);
            //let mut file = std::fs::File::create("/tmp/summe.svg").unwrap();
            //plotter.plot();
            //plotter.write(&mut file).unwrap();

        }
    }
}
