use std::path::Path;

use recad::{
    plot::{Plot, Themes, PlotCommand, Plotter},
    schema::SchemaItem,
};

const TESTS_DIR: &str = "target/out/tests";
const ALL_IN: &str = "tests/files/all_elements/all_elements.kicad_sch";
const ECHO_IN: &str = "tests/files/echo/echo.kicad_sch";
const ECHO_OUT: &str = "target/out/echo.kicad_sch";
const ECHO_PLOT: &str = "target/out/echo.svg";
const SUMME_IN: &str = "tests/files/summe/summe.kicad_sch";
const SUMME_OUT: &str = "target/out/summe.kicad_sch";
const SUMME_PLOT: &str = "target/out/summe.svg";
//const CP3_IN: &str = "tests/cp3/cp3.kicad_sch";
//const CP3_OUT: &str = "target/out/cp3.kicad_sch";
//const CP3_PLOT: &str = "target/out/cp3.svg";

fn init() {
    crate::common::setup();
    let path = std::path::Path::new(TESTS_DIR);
    if !path.exists() {
        std::fs::create_dir_all(path).unwrap();
    }
}

#[test]
fn load_echo() {
    init();
    let schema = recad::Schema::load(std::path::Path::new(ECHO_IN), None).unwrap();
    let mut file = std::fs::File::create(ECHO_OUT).unwrap();
    schema.write(&mut file).unwrap();

    let mut svg = recad::plot::SvgPlotter::new();
    schema
        .plot(
            &mut svg,
            &PlotCommand::new()
                .theme(Some(Themes::Kicad2020))
                .border(Some(true)),
        )
        .unwrap();
    svg.save(Path::new(ECHO_PLOT)).unwrap();
}

#[test]
fn load_summe() {
    init();
    let schema = recad::Schema::load(std::path::Path::new(SUMME_IN), None).unwrap();
    let mut file = std::fs::File::create(SUMME_OUT).unwrap();
    schema.write(&mut file).unwrap();

    let mut svg = recad::plot::SvgPlotter::new();
    schema
        .plot(
            &mut svg,
            &PlotCommand::new()
                .theme(Some(Themes::Kicad2020))
                .border(Some(true)),
        )
        .unwrap();
    svg.save(Path::new(SUMME_PLOT)).unwrap();
}

#[test]
fn get_hierarchical_sheet_filename() {
    init();
    let schema = recad::Schema::load(std::path::Path::new(ALL_IN), None).unwrap();
    let mut filename: Option<String> = None;
    for item in &schema.items {
        if let SchemaItem::HierarchicalSheet(sheet) = item {
            filename = sheet.filename();
        }
    }
    assert_eq!(filename, Some("sheet.kicad_sch".to_string()));
}
