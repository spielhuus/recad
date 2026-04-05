use recad::{
    plot::{Plot, PlotCommand, Plotter, SvgPlotter},
    Schema,
};
use std::path::Path;

#[test]
fn test_all_elements() {
    crate::common::setup();
    let schema =
        Schema::load(Path::new("tests/files/all_elements/all_elements.kicad_sch"), None).unwrap();

    // save to file
    let mut svg = recad::plot::SvgPlotter::new();
    schema
        .plot(&mut svg, &PlotCommand::new().border(Some(true)))
        .unwrap();
    svg.save(Path::new("tests/integration/plot/all_elements_recad.svg"))
        .unwrap();

    // check with insta
    let mut plotter = SvgPlotter::new();
    schema
        .plot(&mut plotter, &PlotCommand::new().border(Some(true)))
        .unwrap();

    let mut buffer = Vec::new();
    plotter.write(&mut buffer).unwrap();
    let svg_content = String::from_utf8(buffer).unwrap();
    insta::assert_snapshot!("plot_all_elements_svg", svg_content);
}
