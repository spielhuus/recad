// use crate::{
//     draw::At,
//     gr::{Color, Justify, Pos, Pt, Pts, Rect},
//     math::Transform,
//     pcb::{GraphicItem as GraphicItem, Pad, PadShape},
//     plot::{theme::Theme, Linecap, Paint, PlotCommand, Plotter},
//     Pcb, Plot, RecadError,
// };
// use crate::pcb_plotter::layer_color;

use models::{
    pcb::{GraphicItem, Pad, PadShape, Pcb},
    transform::Transform,
};
use types::{
    error::RecadError,
    gr::{Color, Justify, Pos, Pt, Pts, Rect},
};

use crate::{pcb_plot::layer_color, Linecap, Paint, Plot, PlotCommand, Plotter};

/// Wrapper to plot a PCB in 3D natively
pub struct Pcb3D<'a>(pub &'a Pcb);

fn layer_z_index(layer: &str) -> f32 {
    match layer {
        "F.SilkS" => 0.81,
        "F.Paste" => 0.82,
        "F.Mask" => 0.805,
        "F.Cu" => 0.8,
        "In1.Cu" => 0.4,
        "In2.Cu" => 0.0,
        "In3.Cu" => -0.4,
        "B.Cu" => -0.8,
        "B.Mask" => -0.805,
        "B.Paste" => -0.82,
        "B.SilkS" => -0.81,
        "Edge.Cuts" => 0.0,
        _ => 0.0,
    }
}

fn should_plot(layer: &str, layers: &[String]) -> bool {
    layers.is_empty() || layers.contains(&layer.to_string())
}

fn layer_matches(pad_layer: &str, target_layer: &str) -> bool {
    if pad_layer == target_layer {
        return true;
    }
    if pad_layer == "*.Cu" && target_layer.ends_with(".Cu") {
        return true;
    }
    if pad_layer == "*.Mask" && target_layer.ends_with(".Mask") {
        return true;
    }
    if pad_layer == "F&B.Cu" && (target_layer == "F.Cu" || target_layer == "B.Cu") {
        return true;
    }
    false
}

fn approximate_bezier(pts: &[Pt], segments: usize) -> Vec<Pt> {
    if pts.len() != 4 {
        return pts.to_vec();
    }
    let mut out = Vec::new();
    let (p0, p1, p2, p3) = (pts[0], pts[1], pts[2], pts[3]);
    for i in 0..=segments {
        let t = i as f64 / segments as f64;
        let mt = 1.0 - t;
        let x = mt * mt * mt * p0.x
            + 3.0 * mt * mt * t * p1.x
            + 3.0 * mt * t * t * p2.x
            + t * t * t * p3.x;
        let y = mt * mt * mt * p0.y
            + 3.0 * mt * mt * t * p1.y
            + 3.0 * mt * t * t * p2.y
            + t * t * t * p3.y;
        out.push(Pt { x, y });
    }
    out
}

impl<'a> Plot for Pcb3D<'a> {
    // fn move_to(&mut self, _pt: At) {}
    //
    // fn get_pt(&self, _at: &At) -> Pt {
    //     Pt { x: 0.0, y: 0.0 }
    // }

    fn plot(&self, plotter: &mut impl Plotter, command: &PlotCommand) -> Result<(), RecadError> {
        // Find Extents for Board Substrate
        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut max_x = f64::MIN;
        let mut max_y = f64::MIN;
        let mut found = false;

        let mut update_bounds = |x: f64, y: f64| {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
            found = true;
        };

        for s in &self.0.segments {
            update_bounds(s.start.x, s.start.y);
            update_bounds(s.end.x, s.end.y);
        }
        for line in &self.0.gr_lines {
            update_bounds(line.start.x, line.start.y);
            update_bounds(line.end.x, line.end.y);
        }
        for via in &self.0.vias {
            let hs = via.size / 2.0;
            update_bounds(via.pos.x - hs, via.pos.y - hs);
            update_bounds(via.pos.x + hs, via.pos.y + hs);
        }
        for zone in &self.0.zones {
            for poly in &zone.filled_polygons {
                for pt in &poly.pts.0 {
                    update_bounds(pt.x, pt.y);
                }
            }
        }
        for fp in &self.0.footprints {
            update_bounds(fp.pos.x, fp.pos.y);
        }

        if found {
            let padding = 5.0;
            // Draw a dark green fiberglass core in the center of the Z axis
            plotter.rect(
                Rect {
                    start: Pt {
                        x: min_x - padding,
                        y: min_y - padding,
                    },
                    end: Pt {
                        x: (max_x - min_x) + padding * 2.0,
                        y: (max_y - min_y) + padding * 2.0,
                    },
                },
                Paint {
                    color: Color::Rgba(15, 30, 15, 255),
                    fill: Some(Color::Rgba(15, 30, 15, 255)),
                    width: 0.0,
                    linecap: Linecap::default(),
                    z_index: 0.0, // Dead center
                },
            );
        }

        for segment in &self.0.segments {
            if should_plot(&segment.layer, &command.layers) {
                plotter.move_to(segment.start);
                plotter.line_to(segment.end);
                plotter.stroke(Paint {
                    color: layer_color(&segment.layer),
                    fill: None,
                    width: segment.width,
                    linecap: Linecap::default(),
                    z_index: layer_z_index(&segment.layer),
                });
            }
        }

        for line in &self.0.gr_lines {
            if should_plot(&line.layer, &command.layers) {
                plotter.move_to(line.start);
                plotter.line_to(line.end);
                plotter.stroke(Paint {
                    color: layer_color(&line.layer),
                    fill: None,
                    width: line.stroke.width.max(0.15),
                    linecap: Linecap::default(),
                    z_index: layer_z_index(&line.layer),
                });
            }
        }

        for via in &self.0.vias {
            let via_layers = [via.layers.0.clone(), via.layers.1.clone()];
            if command.layers.is_empty() || command.layers.iter().any(|l| via_layers.contains(l)) {
                // To simulate a 3D via hole, draw it on the top layer and bottom layer
                for z in [0.8, -0.8] {
                    plotter.circle(
                        Pt {
                            x: via.pos.x,
                            y: via.pos.y,
                        },
                        via.size / 2.0,
                        Paint {
                            color: Color::Rgba(180, 180, 180, 200),
                            fill: Some(Color::Rgba(180, 180, 180, 200)),
                            z_index: z,
                            ..Default::default()
                        },
                    );
                    if via.drill > 0.0 {
                        plotter.circle(
                            Pt {
                                x: via.pos.x,
                                y: via.pos.y,
                            },
                            via.drill / 2.0,
                            Paint {
                                color: Color::Rgba(0, 0, 0, 255),
                                fill: Some(Color::Rgba(0, 0, 0, 255)),
                                z_index: z + if z > 0.0 { 0.01 } else { -0.01 },
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }

        for text in &self.0.gr_texts {
            if should_plot(&text.layer, &command.layers) {
                let mut effects = text.effects.clone();
                if effects.font.color.is_none() {
                    effects.font.color = Some(layer_color(&text.layer));
                }
                effects.z_index = layer_z_index(&text.layer);

                let mut p = text.pos;
                p.angle = -p.angle;
                plotter.text(&text.text, p, effects);
            }
        }

        for zone in &self.0.zones {
            if should_plot(&zone.layer, &command.layers) {
                let c = layer_color(&zone.layer);
                let fill_c = match c {
                    Color::Rgba(r, g, b, _) => Color::Rgba(r, g, b, 100),
                    _ => c,
                };
                for poly in &zone.filled_polygons {
                    if !poly.pts.0.is_empty() {
                        let mut closed_pts = poly.pts.0.clone();
                        if closed_pts.first() != closed_pts.last() {
                            closed_pts.push(*closed_pts.first().unwrap());
                        }
                        plotter.polyline(
                            Pts(closed_pts),
                            Paint {
                                color: fill_c,
                                fill: Some(fill_c),
                                z_index: layer_z_index(&zone.layer),
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }

        for fp in &self.0.footprints {
            // Determine if the footprint is on the bottom layer
            let is_flipped = fp.layer.starts_with("B.");

            // The .kicad_pcb file already stores bottom layer footprint coordinates pre-mirrored.
            // We strictly just need to rotate and translate.
            let fp_transform = Transform::new()
                .translation(Pt {
                    x: fp.pos.x,
                    y: fp.pos.y,
                })
                .rotation(-fp.pos.angle);

            for item in &fp.graphic_items {
                if let Some(actual_layer) = item.layer() {
                    if should_plot(actual_layer, &command.layers) {
                        let item_color = layer_color(actual_layer);
                        let z = layer_z_index(actual_layer);
                        plot_fp_graphic_3d(plotter, &fp_transform, item, item_color, is_flipped, z);
                    }
                }
            }

            for pad in &fp.pads {
                let actual_pad_layers = &pad.layers;
                let target_layer =
                    if command.layers.is_empty() {
                        actual_pad_layers
                            .iter()
                            .find(|l| !l.contains('*') && l.ends_with(".Cu"))
                            .unwrap_or(&fp.layer)
                            .clone()
                    } else {
                        match command.layers.iter().find(|&cmd_l| {
                            actual_pad_layers.iter().any(|pl| layer_matches(pl, cmd_l))
                        }) {
                            Some(l) => l.clone(),
                            None => continue,
                        }
                    };

                let pad_color = layer_color(&target_layer);
                let z = layer_z_index(&target_layer);
                plot_pad_3d(plotter, &fp_transform, pad, pad_color, z);
            }

            // if let Some(model_3d) = &fp.model_3d {
            //     //TODO get path from env
            //     let model_path = model_3d.replace("${KICAD8_3DMODEL_DIR}", "/usr/share/kicad/3dmodels/");
            //     if Path::new(&model_path).exists() {
            //         spdlog::debug!("3d library: {:?}", model_path);
            //
            //     } else if model_path.ends_with(".wrl") {
            //         let model_path = model_path.replace(".wrl", ".step");
            //         if Path::new(&model_path).exists() {
            //             spdlog::debug!("3d library: {:?}", model_path);
            //
            //             // 1. Initialize the OpenCascade STEP reader
            //             unsafe {
            //                 let mut reader = STEPControl_Reader::new();
            //                 let c_path = CString::new(model_path.clone()).unwrap();
            //
            //                 // IFSelect_RetDone is typically 1
            //                 if reader.ReadFile(c_path.as_ptr()) == 1 {
            //                     reader.TransferRoots();
            //                     let shape = reader.OneShape();
            //
            //                     // 2. Tessellate the mathematical B-Rep shape into a polygon mesh
            //                     // The parameter `0.01` is the linear deflection (meshing tolerance/quality)
            //                     let mut mesher = BRepMesh_IncrementalMesh::new(&shape, 0.01);
            //                     mesher.Perform();
            //
            //                     // 3. Extract the triangles face by face
            //                     let mut explorer = TopExp_Explorer::new(&shape, TopAbs_ShapeEnum::TopAbs_FACE);
            //
            //                     let mut total_vertices = 0;
            //                     let mut total_triangles = 0;
            //
            //                     while explorer.More() {
            //                         let face = explorer.Current();
            //                         let mut location = TopLoc_Location::new();
            //
            //                         // Grab the polygon mesh generated by the incremental mesher
            //                         let triangulation_handle = BRep_Tool::Triangulation(&face, &mut location);
            //
            //                         if !triangulation_handle.IsNull() {
            //                             let triangulation = triangulation_handle.get();
            //
            //                             total_vertices += triangulation.NbNodes();
            //                             total_triangles += triangulation.NbTriangles();
            //
            //                             // For wgpu, you would extract the specific vertex coordinates here:
            //                             // for i in 1..=triangulation.NbNodes() {
            //                             //     let node = triangulation.Node(i);
            //                             //     // node.X(), node.Y(), node.Z()
            //                             // }
            //
            //                             // And the triangle indices:
            //                             // for i in 1..=triangulation.NbTriangles() {
            //                             //     let tri = triangulation.Triangle(i);
            //                             //     // tri.Get(ref n1, ref n2, ref n3)
            //                             // }
            //                         }
            //
            //                         explorer.Next();
            //                     }
            //
            //                     spdlog::debug!(
            //                         "Meshed STEP model '{}': {} vertices, {} triangles",
            //                         model_path, total_vertices, total_triangles
            //                     );
            //
            //                 } else {
            //                     spdlog::warn!("OpenCascade failed to read STEP file: {}", model_path);
            //                 }
            //             }
            //
            //         } else {
            //             spdlog::warn!("3d library not found: {:?}", model_path);
            //         }
            //     }
            // }
        }

        plotter.scale(command.scale);

        if found && !command.border {
            let padding = 5.0;
            plotter.set_view_box(Rect {
                start: Pt {
                    x: min_x - padding,
                    y: min_y - padding,
                },
                end: Pt {
                    x: (max_x - min_x) + padding * 2.0,
                    y: (max_y - min_y) + padding * 2.0,
                },
            });
        }

        Ok(())
    }
}

fn plot_fp_graphic_3d(
    plotter: &mut impl Plotter,
    transform: &Transform,
    item: &GraphicItem,
    item_color: Color,
    is_flipped: bool,
    z_index: f32,
) {
    match item {
        GraphicItem::FpLine(line) => {
            let t_start = transform.transform_point(line.start);
            let t_end = transform.transform_point(line.end);

            plotter.move_to(t_start);
            plotter.line_to(t_end);
            plotter.stroke(Paint {
                color: item_color,
                fill: None,
                width: line.stroke.width.max(0.15),
                z_index,
                ..Default::default()
            });
        }
        GraphicItem::FpRect(rect) => {
            let pts = vec![
                Pt {
                    x: rect.start.x,
                    y: rect.start.y,
                },
                Pt {
                    x: rect.end.x,
                    y: rect.start.y,
                },
                Pt {
                    x: rect.end.x,
                    y: rect.end.y,
                },
                Pt {
                    x: rect.start.x,
                    y: rect.end.y,
                },
                Pt {
                    x: rect.start.x,
                    y: rect.start.y,
                },
            ];
            let t_pts = transform.transform_pts(&pts);

            plotter.polyline(
                Pts(t_pts),
                Paint {
                    color: item_color,
                    fill: if rect.fill.as_deref() == Some("solid") {
                        Some(item_color)
                    } else {
                        None
                    },
                    width: rect.width,
                    z_index,
                    ..Default::default()
                },
            );
        }
        GraphicItem::FpCircle(circle) => {
            let t_center = transform.transform_point(circle.center);
            let dx = circle.end.x - circle.center.x;
            let dy = circle.end.y - circle.center.y;
            let radius = (dx * dx + dy * dy).sqrt();

            plotter.circle(
                t_center,
                radius,
                Paint {
                    color: item_color,
                    fill: if circle.fill.as_deref() == Some("solid") {
                        Some(item_color)
                    } else {
                        None
                    },
                    width: circle.width.max(0.0),
                    z_index,
                    ..Default::default()
                },
            );
        }
        GraphicItem::FpPoly(poly) => {
            let mut t_pts = transform.transform_pts(&poly.pts.0);
            if let (Some(first), Some(last)) = (t_pts.first(), t_pts.last()) {
                if first != last {
                    t_pts.push(*first);
                }
            }

            plotter.polyline(
                Pts(t_pts),
                Paint {
                    color: item_color,
                    fill: if poly.fill.as_deref() == Some("solid") {
                        Some(item_color)
                    } else {
                        None
                    },
                    width: poly.width,
                    z_index,
                    ..Default::default()
                },
            );
        }
        GraphicItem::FpArc(arc) => {
            let t_start = transform.transform_point(arc.start);
            let t_mid = transform.transform_point(arc.mid);
            let t_end = transform.transform_point(arc.end);

            plotter.arc(
                t_start,
                t_mid,
                t_end,
                Paint {
                    color: item_color,
                    fill: None,
                    width: arc.width,
                    z_index,
                    ..Default::default()
                },
            );
        }
        GraphicItem::FpCurve(curve) => {
            let t_pts = transform.transform_pts(&curve.pts.0);
            let approx = approximate_bezier(&t_pts, 32);
            plotter.polyline(
                Pts(approx),
                Paint {
                    color: item_color,
                    fill: None,
                    width: curve.width,
                    z_index,
                    ..Default::default()
                },
            );
        }
        GraphicItem::FpText(text) => {
            if text.hide {
                return;
            }

            let t_pos = transform.transform_point(Pt {
                x: text.pos.x,
                y: text.pos.y,
            });

            // The text angle in KiCad is absolute to the board. Negate to match CW Y-down canvas.
            let mut text_angle_cw = -text.pos.angle % 360.0;
            if text_angle_cw < 0.0 {
                text_angle_cw += 360.0;
            }

            let mut effects = text.effects.clone();
            if effects.font.color.is_none() {
                effects.font.color = Some(item_color);
            }
            effects.z_index = z_index;

            if is_flipped {
                effects.justify = effects
                    .justify
                    .into_iter()
                    .map(|j| match j {
                        Justify::Left => Justify::Right,
                        Justify::Right => Justify::Left,
                        other => other,
                    })
                    .collect();
            }

            plotter.text(
                &text.text,
                Pos {
                    x: t_pos.x,
                    y: t_pos.y,
                    angle: text_angle_cw,
                },
                effects,
            );
        }
        GraphicItem::FpProperty(prop) => {
            if prop.hide {
                return;
            }

            if let Some(pos) = prop.pos {
                let t_pos = transform.transform_point(Pt { x: pos.x, y: pos.y });

                let mut text_angle_cw = -pos.angle % 360.0;
                if text_angle_cw < 0.0 {
                    text_angle_cw += 360.0;
                }

                let mut effects = prop.effects.clone().unwrap_or_default();
                if effects.font.color.is_none() {
                    effects.font.color = Some(item_color);
                }
                effects.z_index = z_index;

                if is_flipped {
                    effects.justify = effects
                        .justify
                        .into_iter()
                        .map(|j| match j {
                            Justify::Left => Justify::Right,
                            Justify::Right => Justify::Left,
                            other => other,
                        })
                        .collect();
                }

                plotter.text(
                    &prop.value,
                    Pos {
                        x: t_pos.x,
                        y: t_pos.y,
                        angle: text_angle_cw,
                    },
                    effects,
                );
            }
        }
        _ => {}
    }
}

fn plot_pad_3d(
    plotter: &mut impl Plotter,
    fp_transform: &Transform,
    pad: &Pad,
    pad_color: Color,
    z_index: f32,
) {
    // 1. Find absolute center
    let center_local = Pt {
        x: pad.pos.x,
        y: pad.pos.y,
    };
    let center_world = fp_transform.transform_point(center_local);

    // 2. Transform for the pad shape
    // The pad.pos.angle is the ABSOLUTE angle of the pad on the board.
    let pad_shape_transform = Transform::new()
        .translation(center_world)
        .rotation(-pad.pos.angle);

    let paint = Paint {
        color: pad_color,
        fill: Some(pad_color),
        width: 0.05,
        z_index,
        ..Default::default()
    };

    match pad.shape {
        PadShape::Circle => {
            plotter.circle(center_world, pad.size.0 / 2.0, paint.clone());
        }
        PadShape::Oval => {
            let w = pad.size.0 / 2.0;
            let h = pad.size.1 / 2.0;
            let r = w.min(h);
            let dx = (w - r).max(0.0);
            let dy = (h - r).max(0.0);

            let mut pts = Vec::new();
            let segments = 16;
            for i in 0..=segments {
                let a = (i as f64 / segments as f64) * std::f64::consts::PI / 2.0;
                pts.push(Pt {
                    x: dx + r * a.sin(),
                    y: dy + r * a.cos(),
                });
            }
            for i in 0..=segments {
                let a = std::f64::consts::PI / 2.0
                    + (i as f64 / segments as f64) * std::f64::consts::PI / 2.0;
                pts.push(Pt {
                    x: dx + r * a.sin(),
                    y: -dy + r * a.cos(),
                });
            }
            for i in 0..=segments {
                let a = std::f64::consts::PI
                    + (i as f64 / segments as f64) * std::f64::consts::PI / 2.0;
                pts.push(Pt {
                    x: -dx + r * a.sin(),
                    y: -dy + r * a.cos(),
                });
            }
            for i in 0..=segments {
                let a = 1.5 * std::f64::consts::PI
                    + (i as f64 / segments as f64) * std::f64::consts::PI / 2.0;
                pts.push(Pt {
                    x: -dx + r * a.sin(),
                    y: dy + r * a.cos(),
                });
            }
            pts.push(pts[0]);

            let t_world = pad_shape_transform.transform_pts(&pts);
            plotter.polyline(Pts(t_world), paint.clone());
        }
        PadShape::Rect | PadShape::RoundRect | PadShape::Trapezoid => {
            let w = pad.size.0 / 2.0;
            let h = pad.size.1 / 2.0;
            let pts = vec![
                Pt { x: -w, y: -h },
                Pt { x: w, y: -h },
                Pt { x: w, y: h },
                Pt { x: -w, y: h },
                Pt { x: -w, y: -h },
            ];
            let t_world = pad_shape_transform.transform_pts(&pts);
            plotter.polyline(Pts(t_world), paint.clone());
        }
        _ => {
            plotter.circle(center_world, pad.size.0.max(0.4) / 2.0, paint.clone());
        }
    }

    if let Some(drill) = pad.drill {
        if drill > 0.0 {
            plotter.circle(
                center_world,
                drill / 2.0,
                Paint {
                    color: Color::Rgba(0, 0, 0, 255),
                    fill: Some(Color::Rgba(0, 0, 0, 255)),
                    width: 0.0,
                    z_index: paint.z_index + if paint.z_index > 0.0 { 0.01 } else { -0.01 },
                    ..Default::default()
                },
            );
        }
    }
}
