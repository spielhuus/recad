use std::{
    collections::HashMap, env, path::{Path, PathBuf}, sync::Arc
};

mod common;

use models::schema::Schema;
use netlist::{CircuitGraph, Netlist};
use ngspice::NgSpiceError;
use spdlog::sink::FileSink;

use std::fs::File;
use std::io::{Write, BufWriter};

fn main() {
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

    let schema = common::opamp::draw().unwrap();
    let netlist = Netlist::from(&schema);
    let graph = CircuitGraph::from_netlist(netlist, &schema);
    let mut circuit = graph.to_circuit("opamp".to_string(), vec![String::from("./tests/spice")]);

    circuit.voltage("1", "+15V", "GND", "DC 15V");
    circuit.voltage("2", "-15V", "GND", "DC -15V");
    circuit.voltage("3", "Vin", "GND", "DC 5V AC 5V SIN(0, 5V, 1k)");

    println!("{}", circuit.to_str(true).unwrap().join("\n"));

    // Save it to a file
    let mut buffer = Vec::new();
    schema.write(&mut buffer).unwrap();
    std::fs::write("Inverting_Amplifier.kicad_sch", buffer).unwrap();

    println!("Saved to Inverting_Amplifier.kicad_sch!");

    let mut simulation = simulation::Simulation::new(circuit);
    let result = simulation.tran("10u", "2m", "0").unwrap();


    //create the gnuplot file
    let output_gp_path = "sim.gp";
    let xlabel = "V";
    let ylabel = "T";
    let title = "Inverting Opamp";

// --- Write self-contained .gp file ---
    let file = File::create(output_gp_path)
        .map_err(|e| NgSpiceError::Spice(-1, e.to_string())).unwrap();
    let mut w = BufWriter::new(file);
    
    // Gnuplot header
    writeln!(w, "#!/usr/bin/gnuplot -persist").unwrap();
    writeln!(w, "# Self-contained plot: {} vs {}", xlabel, ylabel).unwrap();
    writeln!(w, "set title \"{}\"", title).unwrap();
    writeln!(w, "set xlabel \"{}\"", xlabel).unwrap();
    writeln!(w, "set ylabel \"{}\"", ylabel).unwrap();
    writeln!(w, "set grid").unwrap();
    writeln!(w, "set key top right").unwrap();
    
    // Output to PNG (comment out for interactive-only)
    writeln!(w, "set terminal pngcairo size 1200,700 enhanced font \"Sans,12\"").unwrap();
    writeln!(w, "set output \"{}.png\"", 
             Path::new(output_gp_path).file_stem().unwrap().to_string_lossy()).unwrap();
    
    // Line styles
    writeln!(w, "set style line 1 lc rgb '#E41A1C' lt 1 lw 2 pt 7 ps 0.5").unwrap();
    writeln!(w, "set style line 2 lc rgb '#377EB8' lt 1 lw 2 pt 7 ps 0.5").unwrap();
    
    // Embedded data block (gnuplot 5+ heredoc syntax)
    writeln!(w, "$Data << EOD").unwrap();
    writeln!(w, "# time vin vout").unwrap();
    for (t, (v_in, v_out)) in result["time"].iter().zip(result["vin"].iter().zip(result["vout"].iter())) {
        writeln!(w, "{:.8e} {:.8e} {:.8e}", t, v_in, v_out).unwrap();
    }
    writeln!(w, "EOD").unwrap();
    
    // Plot command referencing embedded data
    writeln!(w).unwrap();
    writeln!(w, "plot $Data using 1:2 with lines ls 1 title \"vin\", \\").unwrap();
    writeln!(w, "     $Data using 1:3 with lines ls 2 title \"vout\"").unwrap();
    
    w.flush().map_err(|e| NgSpiceError::Spice(-1, e.to_string())).unwrap();

}
