use std::{env, path::Path};

use plot::{PlotCommand, Plotter, Plot};
use models::{pcb::Pcb, schema::Schema};
use spdlog::sink::FileSink;
use winit::event_loop::EventLoop;

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
            let event_loop = EventLoop::with_user_event().build().unwrap();
            let mut app = plot::WgpuPlotter::new(&event_loop);

            app.set_ui_callback(|ui| {
                ui.heading("Document");
                ui.label("Type: Schematic");
                false // Does not trigger replot from the custom UI area
            });

            // 1. Initial plot to populate internal data, including the `pages` list
            schema
                .plot(&mut app, &PlotCommand::new().border(Some(true)))
                .unwrap();

            // 2. Store the base directory to resolve relative paths for hierarchical sheets
            let base_dir =
                std::rc::Rc::new(input_path.parent().unwrap_or(Path::new("")).to_path_buf());

            // 3. Extract the original pages list generated from the root schema.
            // We need this to keep the UI's combobox populated with all sheets.
            let root_pages = std::rc::Rc::new(app.pages().to_vec());

            // 4. Implement the replot callback which reacts to the egui page combobox
            app.set_replot_callback(move |app_ref| {
                let active_page = app_ref.active_page();

                if let Some((sheet_name, filename)) = root_pages.get(active_page) {
                    let path = if active_page == 0 {
                        // The first page is the root page itself
                        PathBuf::from(filename)
                    } else {
                        // Hierarchical sheet paths are typically relative to the project directory
                        base_dir.join(filename)
                    };

                    // Load the newly selected schema sheet file
                    match Schema::load(&path, Some(sheet_name.clone())) {
                        Ok(child_schema) => {
                            child_schema
                                .plot(app_ref, &PlotCommand::new().border(Some(true)))
                                .unwrap();

                            // A new plot overrides the plotter's pages state to only
                            // include child sheets of the currently loaded schema.
                            // We restore the original hierarchy list to maintain UI navigation.
                            app_ref.set_pages(root_pages.to_vec());
                            app_ref.set_active_page(active_page);
                        }
                        Err(e) => {
                            spdlog::error!("Failed to load schema sheet {:?}: {}", path, e);
                        }
                    }
                }
            });

            event_loop.run_app(&mut app).unwrap();
        }
        Some("kicad_pcb") => {
            let pcb = Pcb::load(input_path).unwrap();
            let event_loop = EventLoop::with_user_event().build().unwrap();
            let mut app = plot::WgpuPlotter::new(&event_loop);

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
                ui.label("Type: PCB");
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
                                let c = plot::pcb_plot::layer_color(layer);
                                let color = if let types::gr::Color::Rgba(r, g, b, a) = c {
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
                pcb_replot
                    .plot(
                        app_ref,
                        &PlotCommand::new()
                            .border(Some(true))
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
            pcb_rc
                .plot(
                    &mut app,
                    &PlotCommand::new()
                        .border(Some(true))
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
