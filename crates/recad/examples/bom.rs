use std::{env, path::Path};

use models::schema::Schema;
use spdlog::sink::FileSink;

use std::path::PathBuf;
use std::sync::Arc;

fn main() {
    let file_sink = FileSink::builder()
        .path(PathBuf::from("logs/application.log"))
        .truncate(false)
        .build()
        .unwrap();

    let logger = spdlog::default_logger()
        .fork_with(|logger| {
            logger.sinks_mut().push(Arc::new(file_sink));
            Ok(())
        })
        .unwrap();

    logger.set_level_filter(spdlog::LevelFilter::All);
    logger.set_flush_level_filter(spdlog::LevelFilter::All);
    spdlog::set_default_logger(logger);

    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} input_file", args[0]);
        return;
    }

    let input_path = Path::new(&args[1]);
    let extension = input_path.extension().and_then(|s| s.to_str());

    match extension {
        Some("kicad_sch") => {
            let schema = Schema::load(input_path, None).unwrap();
            let bom = recad::reports::bom(&schema, true, None);
            println!("{:?}", bom);
        }
        Some(extension) => {
            eprintln!("file extension not supported: {}", extension);
        }
        _ => eprintln!("can not guess file type: {}", args[1]),
    }
    spdlog::default_logger().flush();
}
