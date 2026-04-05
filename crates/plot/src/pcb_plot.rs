use models::{pcb::{GraphicItem, Pad, PadShape, Pcb}, transform::Transform};
use types::{error::RecadError, gr::{Color, Justify, Pos, Pt, Pts, Rect}};

use crate::{Linecap, Paint, Plot, PlotCommand, Plotter, border::draw_border, theme::Theme};

/// Helper to determine layer color
pub fn layer_color(layer: &str) -> Color {
    match layer {
        "F.Cu" => Color::Rgba(200, 54, 54, 200),
        "B.Cu" => Color::Rgba(54, 200, 54, 200),
        "In1.Cu" => Color::Rgba(200, 200, 54, 200),
        "In2.Cu" => Color::Rgba(200, 54, 200, 200),
        "In3.Cu" => Color::Rgba(54, 200, 200, 200),
        "In4.Cu" => Color::Rgba(54, 54, 200, 200),
        "F.SilkS" => Color::Rgba(0, 132, 132, 200),
        "B.SilkS" => Color::Rgba(132, 0, 132, 200),
        "F.Mask" => Color::Rgba(132, 0, 0, 200),
        "B.Mask" => Color::Rgba(0, 132, 0, 200),
        "F.CrtYd" => Color::Rgba(192, 192, 192, 128),
        "B.CrtYd" => Color::Rgba(192, 192, 192, 128),
        "F.Fab" => Color::Rgba(192, 192, 0, 200),
        "B.Fab" => Color::Rgba(0, 192, 192, 200),
        "Edge.Cuts" => Color::Rgba(200, 200, 0, 255),
        "Margin" => Color::Rgba(255, 0, 255, 200),
        _ => Color::Rgba(128, 128, 128, 128),
    }
}

fn should_plot(layer: &str, layers: &[String]) -> bool {
    layers.is_empty() || layers.contains(&layer.to_string())
}

/// Helper to check if a pad's layer matches a target requested layer, handling KiCad wildcards.
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

/// Helper to flip layers if the footprint is placed on the bottom side of the PCB
pub fn flip_layer(layer: &str, fp_layer: &str) -> String {
    if fp_layer.starts_with("B.") {
        if layer.starts_with("F.") {
            return layer.replacen("F.", "B.", 1);
        } else if layer.starts_with("B.") {
            return layer.replacen("B.", "F.", 1);
        }
    }
    layer.to_string()
}

/// Applies rotation logic to the text alignments.
pub fn apply_rotation_to_alignments(angle: f64, alignments: &mut Vec<Justify>) {
    let angle = (angle % 360.0).round() as i32;

    let remap = |a: &Justify| -> Justify {
        match angle {
            0 => *a,
            90 | -270 => match a {
                Justify::Left => Justify::Bottom,
                Justify::Right => Justify::Top,
                Justify::Center => Justify::Center,
                Justify::Top => Justify::Left,
                Justify::Bottom => Justify::Right,
                Justify::Mirror => Justify::Mirror,
            },
            180 | -180 => match a {
                Justify::Left => Justify::Right,
                Justify::Right => Justify::Left,
                Justify::Center => Justify::Center,
                Justify::Top => Justify::Bottom,
                Justify::Bottom => Justify::Top,
                Justify::Mirror => Justify::Mirror,
            },
            270 | -90 => match a {
                Justify::Left => Justify::Top,
                Justify::Right => Justify::Bottom,
                Justify::Center => Justify::Center,
                Justify::Top => Justify::Right,
                Justify::Bottom => Justify::Left,
                Justify::Mirror => Justify::Mirror,
            },
            _ => *a,
        }
    };

    *alignments = alignments.iter().map(remap).collect();
}

/// Corrects the text rotation by adding the symbol's rotation to the text's local rotation.
pub fn apply_text_rotation(symbol_pos: Pos, text_pos: &mut Pos) {
    let mut total_angle = symbol_pos.angle + text_pos.angle;
    total_angle %= 360.0;
    text_pos.angle = total_angle;
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

impl Plot for Pcb {
    fn plot(&self, plotter: &mut impl Plotter, command: &PlotCommand) -> Result<(), RecadError> {
        spdlog::debug!("Plot PCB: {:?}", &command);
        let theme = Theme::from(command.theme);

        for segment in &self.segments {
            if should_plot(&segment.layer, &command.layers) {
                plotter.move_to(segment.start);
                plotter.line_to(segment.end);
                plotter.stroke(Paint {
                    color: layer_color(&segment.layer),
                    fill: None,
                    width: segment.width,
                    linecap: Linecap::default(),
                    ..Default::default()
                });
            }
        }

        for line in &self.gr_lines {
            if should_plot(&line.layer, &command.layers) {
                plotter.move_to(line.start);
                plotter.line_to(line.end);
                plotter.stroke(Paint {
                    color: layer_color(&line.layer),
                    fill: None,
                    width: line.stroke.width.max(0.15),
                    linecap: Linecap::default(),
                    ..Default::default()
                });
            }
        }

        for via in &self.vias {
            let via_layers = [via.layers.0.clone(), via.layers.1.clone()];
            let plot_via =
                command.layers.is_empty() || command.layers.iter().any(|l| via_layers.contains(l));

            if plot_via {
                plotter.circle(
                    Pt {
                        x: via.pos.x,
                        y: via.pos.y,
                    },
                    via.size / 2.0,
                    Paint {
                        color: Color::Rgba(180, 180, 180, 200),
                        fill: Some(Color::Rgba(180, 180, 180, 200)),
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
                            ..Default::default()
                        },
                    );
                }
            }
        }

        for text in &self.gr_texts {
            if should_plot(&text.layer, &command.layers) {
                let mut effects = text.effects.clone();
                if effects.font.color.is_none() {
                    effects.font.color = Some(layer_color(&text.layer));
                }

                let mut p = text.pos;
                p.angle = -p.angle;
                plotter.text(&text.text, p, effects);
            }
        }

        for zone in &self.zones {
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
                                ..Default::default()
                            },
                        );
                    }
                }
            }
        }

        for fp in &self.footprints {
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
                        plot_fp_graphic(
                            plotter,
                            &fp_transform,
                            item,
                            fp.pos,
                            item_color,
                            is_flipped,
                        );
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
                plot_pad(plotter, &fp_transform, pad, pad_color);
            }
        }

        if command.border {
            //TODO: path and sheet
            draw_border(plotter, &self.paper, &self.title_block, "", "", &theme);
        }

        plotter.scale(command.scale);

        let paper_size: (f64, f64) = self.paper.clone().into();
        if command.border {
            plotter.set_view_box(Rect {
                start: Pt { x: 0.0, y: 0.0 },
                end: Pt {
                    x: paper_size.0,
                    y: paper_size.1,
                },
            });
        } else {
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

            for s in &self.segments {
                update_bounds(s.start.x, s.start.y);
                update_bounds(s.end.x, s.end.y);
            }
            for line in &self.gr_lines {
                update_bounds(line.start.x, line.start.y);
                update_bounds(line.end.x, line.end.y);
            }
            for via in &self.vias {
                let hs = via.size / 2.0;
                update_bounds(via.pos.x - hs, via.pos.y - hs);
                update_bounds(via.pos.x + hs, via.pos.y + hs);
            }
            for zone in &self.zones {
                for poly in &zone.filled_polygons {
                    for pt in &poly.pts.0 {
                        update_bounds(pt.x, pt.y);
                    }
                }
            }
            for fp in &self.footprints {
                update_bounds(fp.pos.x, fp.pos.y);
            }

            if found {
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
        }

        Ok(())
    }
}


fn plot_fp_graphic(
    plotter: &mut impl Plotter,
    transform: &Transform,
    item: &GraphicItem,
    _fp_pos: Pos, //TODO check if this is used
    item_color: Color,
    is_flipped: bool,
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

fn plot_pad(plotter: &mut impl Plotter, fp_transform: &Transform, pad: &Pad, pad_color: Color) {
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
        .rotation(-pad.pos.angle); // Negate for CW canvas projection

    let paint = Paint {
        color: pad_color,
        fill: Some(pad_color),
        width: 0.05,
        ..Default::default()
    };

    match pad.shape {
        PadShape::Circle => {
            plotter.circle(center_world, pad.size.0 / 2.0, paint);
        }
        PadShape::Oval => {
            let w = pad.size.0 / 2.0;
            let h = pad.size.1 / 2.0;
            let r = w.min(h);
            let dx = (w - r).max(0.0);
            let dy = (h - r).max(0.0);

            let mut pts = Vec::new();
            let segments = 16;
            // Top Right
            for i in 0..=segments {
                let a = (i as f64 / segments as f64) * std::f64::consts::PI / 2.0;
                pts.push(Pt {
                    x: dx + r * a.sin(),
                    y: dy + r * a.cos(),
                });
            }
            // Bottom Right
            for i in 0..=segments {
                let a = std::f64::consts::PI / 2.0
                    + (i as f64 / segments as f64) * std::f64::consts::PI / 2.0;
                pts.push(Pt {
                    x: dx + r * a.sin(),
                    y: -dy + r * a.cos(),
                });
            }
            // Bottom Left
            for i in 0..=segments {
                let a = std::f64::consts::PI
                    + (i as f64 / segments as f64) * std::f64::consts::PI / 2.0;
                pts.push(Pt {
                    x: -dx + r * a.sin(),
                    y: -dy + r * a.cos(),
                });
            }
            // Top Left
            for i in 0..=segments {
                let a = 1.5 * std::f64::consts::PI
                    + (i as f64 / segments as f64) * std::f64::consts::PI / 2.0;
                pts.push(Pt {
                    x: -dx + r * a.sin(),
                    y: dy + r * a.cos(),
                });
            }
            pts.push(pts[0]); // Close

            let t_world = pad_shape_transform.transform_pts(&pts);

            plotter.polyline(Pts(t_world), paint);
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

            plotter.polyline(Pts(t_world), paint);
        }
        _ => {
            plotter.circle(center_world, pad.size.0.max(0.4) / 2.0, paint);
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
                    ..Default::default()
                },
            );
        }
    }
}
