use std::{env, path::Path};
use std::path::PathBuf;
use std::sync::Arc;

use models::pcb::Pcb;
use plot::{
    PlotCommand, 
    Plot,
    pcb_plot::layer_color,
    pcb_plot_3d::Pcb3D,
    wgpu_3d::Wgpu3dPlotter,
};

use types::gr::Color;
use winit::event_loop::EventLoop;

use spdlog::sink::FileSink;

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
        Some("kicad_pcb") => {
            let pcb = Pcb::load(input_path).unwrap();
            let event_loop = EventLoop::with_user_event().build().unwrap();
            
            // Use the new 3D plotter backend
            let mut app = Wgpu3dPlotter::new(&event_loop);

            let common_layers = vec![
                "F.Cu",
                "B.Cu",
                "In1.Cu",
                "In2.Cu",
                "In3.Cu",
                "In4.Cu",
                "F.SilkS",
                "B.SilkS",
                "F.Mask",
                "B.Mask",
                "F.CrtYd",
                "B.CrtYd",
                "F.Fab",
                "B.Fab",
                "Edge.Cuts",
                "Margin",
            ];

            let selected_layer = "All".to_string();
            let selected_layer_rc =
                std::rc::Rc::new(std::cell::RefCell::new(selected_layer.clone()));
            let pcb_rc = std::rc::Rc::new(pcb);

            let layer_rc_ui = selected_layer_rc.clone();
            app.set_ui_callback(move |ui| {
                ui.heading("Document");
                ui.label("Type: PCB 3D");
                ui.separator();
                ui.heading("Layers");
                let mut changed = false;
                let mut current_layer = layer_rc_ui.borrow_mut();

                egui::ScrollArea::vertical()
                    .max_height(300.0)
                    .show(ui, |ui| {
                        if ui
                            .radio_value(&mut *current_layer, "All".to_string(), "All Layers")
                            .changed()
                        {
                            changed = true;
                        }
                        ui.separator();

                        for layer in &common_layers {
                            ui.horizontal(|ui| {
                                let c = layer_color(layer);
                                let color = if let Color::Rgba(r, g, b, a) = c {
                                    egui::Color32::from_rgba_unmultiplied(r, g, b, a)
                                } else {
                                    egui::Color32::GRAY
                                };
                                let (rect, _response) = ui.allocate_exact_size(
                                    egui::vec2(16.0, 16.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 0.0, color);

                                if ui
                                    .radio_value(
                                        &mut *current_layer,
                                        layer.to_string(),
                                        layer.to_string(),
                                    )
                                    .changed()
                                {
                                    changed = true;
                                }
                            });
                        }
                    });

                changed
            });

            let layer_rc_replot = selected_layer_rc.clone();
            let pcb_replot = pcb_rc.clone();
            app.set_replot_callback(move |app_ref| {
                let layer = layer_rc_replot.borrow().clone();
                let layers = if layer == "All" { vec![] } else { vec![layer] };
                
                // Wrap the PCB in Pcb3D and disable border to show the 3D substrate
                Pcb3D(&pcb_replot)
                    .plot(
                        app_ref,
                        &PlotCommand::new()
                            .border(Some(false))
                            .scale(Some(20.0))
                            .layers(layers),
                    )
                    .unwrap();
            });

            let initial_layers = if selected_layer == "All" {
                vec![]
            } else {
                vec![selected_layer]
            };
            
            // Initial plot wrapper
            Pcb3D(&pcb_rc)
                .plot(
                    &mut app,
                &PlotCommand::new()
                        .border(Some(false))
                        .scale(Some(20.0))
                        .layers(initial_layers),
                )
                .unwrap();
                
            event_loop.run_app(&mut app).unwrap();
        }
        Some(extension) => {
            eprintln!("file extension not supported: {}", extension);
        }
        _ => eprintln!("can not guess file type: {}", args[1]),
    }
    
    spdlog::default_logger().flush();
}
