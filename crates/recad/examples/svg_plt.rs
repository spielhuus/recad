use std::{
    env,
    path::{Path, PathBuf},
    sync::Arc,
};

use models::{pcb::Pcb, schema::Schema};
use plot::{Plot, Plotter, PlotCommand};
use spdlog::sink::FileSink;
use types::error::RecadError;

fn main() -> Result<(), u16> {
    let file_sink = FileSink::builder()
        .path(PathBuf::from("logs/application.log"))
        .truncate(false)
        .build()
        .unwrap();

    let logger = spdlog::default_logger()
        .fork_with(|logger| {
            // Access the sinks of the new logger and push our file sink
            logger.sinks_mut().push(Arc::new(file_sink));
            Ok(())
        })
        .unwrap();

    logger.set_level_filter(spdlog::LevelFilter::All);
    logger.set_flush_level_filter(spdlog::LevelFilter::All);
    spdlog::set_default_logger(logger);

    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} input_file output_file", args[0]);
        return Err(1);
    }

    let input_path = Path::new(&args[1]);
    let output_path = Path::new(&args[2]);
    let extension = input_path.extension().and_then(|s| s.to_str());

    match extension {
        Some("kicad_sch") => {
            let schema = Schema::load(input_path, None).unwrap();
            let mut svg = plot::SvgPlotter::new();
            schema
                .plot(&mut svg, &PlotCommand::new().border(Some(true)))
                .unwrap();
            svg.save(output_path).unwrap();
        }
        Some("kicad_pcb") => {
            println!("plot pcb: {}", input_path.to_str().expect("filename as string"));
            let pcb = match Pcb::load(input_path) {
                Ok(pcb) => pcb,
                Err(err) => {
                    if let RecadError::Sexp{line: _, col: _, msg} = err {
                        eprint!("{}", msg);
                        return Err(1);
                    } else {
                        eprint!("{}", err);
                        return Err(1);
                    }
                }
            };
            for layer in &pcb.layers {
                let mut svg = plot::SvgPlotter::new();
                let file = format!("{}_{}.svg", &args[2], layer.canonical_name);
                let output_path = Path::new(&file);
                match pcb.plot(
                    &mut svg,
                    &PlotCommand::new()
                        .border(Some(true))
                        .layers(vec![layer.canonical_name.to_string()]),
                ) {
                    Ok(_) => {},
                    Err(err) => {
                    eprint!("{}", err);
                    return Err(1);
                    }
                }
                match svg.save(output_path) {
                    Ok(_) => {},
                    Err(err) => {
                    eprint!("{}", err);
                    return Err(1);
                    }
                }
            }
        }
        Some(extension) => {
            eprintln!("file extension not supported: {}", extension);
        }
        _ => eprintln!("can not guess file type: {}", args[1]),
    }
    Ok(())
}
