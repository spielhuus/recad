use crate::{Linecap, Paint, Plot, PlotCommand, Plotter, border::draw_border, theme::{Style, Theme}};
use models::{
    geometry::Bbox, schema::{Schema, SchemaItem}, symbols::Pin, transform::Transform
    };
use types::{constants::el, error::RecadError, gr::{Arc, Circle, Effects, FillType, Font, GraphicItem, Justify, Polyline, Pos, Pt, Pts, Rect, Rectangle}};

const NO_CONNECT_R: [Pt; 2] = [
    Pt {
        x: -el::NO_CONNECT_SIZE,
        y: -el::NO_CONNECT_SIZE,
    },
    Pt {
        x: el::NO_CONNECT_SIZE,
        y: el::NO_CONNECT_SIZE,
    },
];

const NO_CONNECT_L: [Pt; 2] = [
    Pt {
        x: -el::NO_CONNECT_SIZE,
        y: el::NO_CONNECT_SIZE,
    },
    Pt {
        x: el::NO_CONNECT_SIZE,
        y: -el::NO_CONNECT_SIZE,
    },
];

macro_rules! outline {
    ($self:expr, $item:expr, $plotter:expr) => {
        if cfg!(debug_assertions) {
            let outline = $item.outline()?;
            $plotter.move_to(outline.start);
            $plotter.line_to(Pt {
                x: outline.end.x,
                y: outline.start.y,
            });
            $plotter.line_to(outline.end);
            $plotter.line_to(Pt {
                x: outline.start.x,
                y: outline.end.y,
            });
            $plotter.line_to(outline.start);
            $plotter.close();
            $plotter.stroke(Paint::outline());
        }
    };
}

/// Corrects the text rotation by adding the symbol's rotation to the text's local rotation.
pub fn apply_text_rotation(
    symbol_pos: Pos,
    text_pos: &mut Pos,
    effects: &mut Effects,
    symbol_mirror: &Option<String>,
) {
    // 1. Combine angles and normalize to[0, 360)
    let mut total_angle = (symbol_pos.angle + text_pos.angle) % 360.0;
    if total_angle < 0.0 {
        total_angle += 360.0;
    }

    let mut flip_x = false;
    let mut flip_y = false;

    // 2. Handle Symbol Mirroring
    if let Some(mirror) = symbol_mirror {
        // Mirror Y or XY inverts the X coordinates
        if mirror == "y" || mirror == "xy" {
            flip_x = !flip_x;
        }
        // Mirror X or XY inverts the Y coordinates
        if mirror == "x" || mirror == "xy" {
            flip_y = !flip_y;
        }
    }

    // 3. Keep Upright Rule (Readability)
    // If text is reading downwards or upside down (> 90 and <= 270),
    // KiCad rotates it 180 degrees to keep it readable.
    if total_angle > 90.0 && total_angle <= 270.0 {
        total_angle = (total_angle + 180.0) % 360.0;
        flip_x = !flip_x;
        flip_y = !flip_y; // A 180deg rotation flips BOTH axes physically!
    }

    // 4. Apply required justification swaps to maintain physical position
    if flip_x {
        if effects.justify.contains(&Justify::Left) {
            effects.remove(Justify::Left);
            effects.justify.push(Justify::Right);
        } else if effects.justify.contains(&Justify::Right) {
            effects.remove(Justify::Right);
            effects.justify.push(Justify::Left);
        }
    }

    if flip_y {
        if effects.justify.contains(&Justify::Top) {
            effects.remove(Justify::Top);
            effects.justify.push(Justify::Bottom);
        } else if effects.justify.contains(&Justify::Bottom) {
            effects.remove(Justify::Bottom);
            effects.justify.push(Justify::Top);
        }
    }

    text_pos.angle = total_angle;
}

/// Resolves the final drawing position for text by calculating the absolute
fn resolve_text_layout(text: &str, mut pos: Pos, mut effects: Effects) -> (Pos, Effects) {
    let (w, h) = match font::dimension(text, &effects) {
        Ok(dim) => (dim.x, dim.y),
        Err(_) => (0.0, effects.font.size.1 as f64),
    }; 

    // find the local anchor coordinate (ax, ay) relative to top-left (0,0)
    let ax = if effects.justify.contains(&Justify::Right) {
        w
    } else if effects.justify.contains(&Justify::Left) {
        0.0
    } else {
        w / 2.0
    };

    let ay = if effects.justify.contains(&Justify::Bottom) {
        h
    } else if effects.justify.contains(&Justify::Top) {
        0.0
    } else {
        h / 2.0
    };

    let dx = 0.0 - ax;
    let dy = h - ay;

    pos.angle = -pos.angle;

    let angle_rad = pos.angle.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    let rot_dx = dx * cos_a - dy * sin_a;
    let rot_dy = dx * sin_a + dy * cos_a;

    pos.x += rot_dx;
    pos.y += rot_dy;

    effects.justify = vec![Justify::Left, Justify::Bottom];

    (pos, effects)
}

/// Returns drawing priority (higher = drawn on top)
fn draw_priority(item: &SchemaItem) -> u8 {
    match item {
        // Background layer (drawn first)
        SchemaItem::Wire(_) => 0,
        SchemaItem::Bus(_) => 0,
        SchemaItem::BusEntry(_) => 0,
        SchemaItem::Polyline(_) => 0,
        SchemaItem::Rectangle(_) => 0,
        SchemaItem::Circle(_) => 0,
        SchemaItem::Arc(_) => 0,

        // Middle layer
        SchemaItem::Symbol(_) => 1,
        SchemaItem::Text(_) => 1,
        SchemaItem::LocalLabel(_) => 1,
        SchemaItem::GlobalLabel(_) => 1,
        SchemaItem::HierarchicalLabel(_) => 1,
        SchemaItem::HierarchicalSheet(_) => 1,
        SchemaItem::NetclassFlag(_) => 1,
        SchemaItem::TextBox(_) => 1,

        // Foreground layer (drawn last = on top)
        SchemaItem::Junction(_) => 2,
        SchemaItem::NoConnect(_) => 2,
        _ => 0,
    }
}


// 1. Top-Level Implementation
impl Plot for Schema {
    fn plot(&self, plotter: &mut impl Plotter, command: &PlotCommand) -> Result<(), RecadError> {
        spdlog::debug!(
            "start plotting schema, path: {:?}, command: {:?}",
            self.path,
            command
        );
        let theme = Theme::from(command.theme);
        let paper_size: (f64, f64) = self.paper.clone().into();
        let mut pages: Vec<(String, String)> = vec![(
            "/".into(),
            self.path.clone().unwrap_or("(none)".into()).to_string(),
        )];

        // Create sorted references to items
        let mut items: Vec<&SchemaItem> = self.items.iter().collect();
        items.sort_by_key(|item| draw_priority(item));

        // outline
        let mut bbox: Vec<Rect> = Vec::new();

        // first pass
        for item in items {
            match item {
                SchemaItem::Symbol(symbol) => {
                    let library = self.library_symbol(&symbol.lib_id).unwrap();
                    bbox.push(symbol.outline(library)?);
                    //TODO XXX outline!(self, symbol, plotter);
                    for prop in &symbol.props {
                        if prop.visible() {
                            let value = if prop.key == el::PROPERTY_REFERENCE
                                && library.unit_count() > 1
                            {
                                prop.value.clone()
                                    + &((b'a' + (symbol.unit as u32 - 1) as u8) as char).to_string()
                            } else {
                                prop.value.clone()
                            };
                            let mut pos = prop.pos;
                            let mut effects = prop.effects.clone();
                            apply_text_rotation(symbol.pos, &mut pos, &mut effects, &symbol.mirror);

                            let effects = Effects {
                                font: Font {
                                    face: Some(theme.face()),
                                    size: theme.font_size(effects.font.size, Style::Property),
                                    thickness: effects.font.thickness,
                                    bold: effects.font.bold,
                                    italic: effects.font.italic,
                                    line_spacing: effects.font.line_spacing,
                                    color: Some(theme.color(effects.font.color, Style::Property)),
                                },
                                justify: effects.justify.clone(),
                                hide: prop.visible(),
                                ..Default::default()
                            };

                            let mut prop = prop.clone();
                            prop.value = value.clone();
                            prop.effects = effects.clone();
                            prop.pos = pos;
                            bbox.push(prop.outline()?);
                            if cfg!(debug_assertions) {
                                outline!(self, prop, plotter);
                            }
                            let (final_pos, final_effects) =
                                resolve_text_layout(&value, pos, effects);
                            plotter.text(&value, final_pos, final_effects);
                        }
                    }

                    let transform = Transform::new()
                        .translation(symbol.pos.into())
                        .mirror(&symbol.mirror)
                        .rotation(symbol.pos.angle);

                    for lib_symbol in &library.units {
                        if lib_symbol.unit() == 0 || lib_symbol.unit() == symbol.unit {
                            for g in &lib_symbol.graphics {
                                match g {
                                    GraphicItem::Arc(a) => {
                                        arc(plotter, &transform, a, &Style::Outline, &theme);
                                    }
                                    GraphicItem::Polyline(p) => {
                                        polyline(plotter, &transform, p, &Style::Outline, &theme);
                                    }
                                    GraphicItem::Rectangle(p) => {
                                        rectangle(plotter, &transform, p, &Style::Outline, &theme);
                                    }
                                    GraphicItem::Circle(c) => {
                                        circle(plotter, &transform, c, &Style::Outline, &theme);
                                    }
                                    GraphicItem::Curve(_) => todo!(),
                                    GraphicItem::Line(_) => todo!(),
                                    GraphicItem::EmbeddedFont(_) => todo!(),
                                    GraphicItem::Text(t) => {
                                        let mut effects =
                                            Effects {
                                                font: Font {
                                                    face: Some(theme.face()),
                                                    size: theme.font_size(
                                                        t.effects.font.size,
                                                        Style::Property,
                                                    ),
                                                    thickness: t.effects.font.thickness,
                                                    bold: t.effects.font.bold,
                                                    italic: t.effects.font.italic,
                                                    line_spacing: t.effects.font.line_spacing,
                                                    color: Some(theme.color(
                                                        t.effects.font.color,
                                                        Style::Property,
                                                    )),
                                                },
                                                justify: t.effects.justify.clone(),
                                                hide: t.effects.hide,
                                                ..Default::default()
                                            };

                                        // Transform the anchor position
                                        let t_pt = transform.transform_point(Pt {
                                            x: t.pos.x,
                                            y: t.pos.y,
                                        });
                                        let mut final_pos = Pos {
                                            x: t_pt.x,
                                            y: t_pt.y,
                                            angle: t.pos.angle, // Start with local angle
                                        };

                                        // Add symbol rotation
                                        apply_text_rotation(
                                            symbol.pos,
                                            &mut final_pos,
                                            &mut effects,
                                            &symbol.mirror,
                                        );

                                        let (final_pos, final_effects) =
                                            resolve_text_layout(&t.text, final_pos, effects);
                                        plotter.text(&t.text, final_pos, final_effects);
                                    }
                                }
                            }
                        }
                    }
                    for p in &library.pins(symbol.unit) {
                        if p.hide {
                            continue;
                        };
                        pin(
                            plotter,
                            &transform,
                            p,
                            library.pin_numbers,
                            library.pin_names,
                            library.pin_names_offset,
                            library.power,
                            &Style::Outline,
                            &theme,
                            symbol.pos,
                        );
                    }
                }
                SchemaItem::Wire(wire) => {
                    bbox.push(wire.outline()?);
                    outline!(self, wire, plotter);
                    let pts1 = wire.pts.0.first().expect("pts[0] should exist");
                    let pts2 = wire.pts.0.get(1).expect("pts[0] should exist");
                    plotter.move_to(*pts1);
                    plotter.line_to(*pts2);
                    plotter.stroke(Paint {
                        color: theme.color(wire.stroke.color, Style::Wire),
                        fill: None,
                        width: theme.width(wire.stroke.width, Style::Wire),
                        linecap: Linecap::Round,
                        ..Default::default()

                    });
                }
                SchemaItem::NoConnect(nc) => {
                    bbox.push(nc.outline()?);
                    outline!(self, nc, plotter);
                    let transform = Transform::new().translation(nc.pos.into());
                    let r = transform.transform_pts(&NO_CONNECT_R);
                    let l = transform.transform_pts(&NO_CONNECT_L);

                    plotter.move_to(r[0]);
                    plotter.line_to(r[1]);
                    plotter.stroke(Paint {
                        color: theme.color(None, Style::NoConnect),
                        fill: None,
                        width: theme.width(0.0, Style::NoConnect),
                        ..Default::default()
                    });
                    plotter.move_to(l[0]);
                    plotter.line_to(l[1]);
                    plotter.stroke(Paint {
                        color: theme.color(None, Style::NoConnect),
                        fill: None,
                        width: theme.width(0.0, Style::NoConnect),
                        linecap: Linecap::Round,
                        ..Default::default()
                    });
                }
                SchemaItem::Junction(junction) => {
                    bbox.push(junction.outline()?);
                    outline!(self, junction, plotter);
                    plotter.circle(
                        junction.pos.into(),
                        if junction.diameter == 0.0 {
                            el::JUNCTION_DIAMETER / 2.0
                        } else {
                            junction.diameter / 2.0
                        },
                        Paint {
                            color: theme.color(None, Style::Junction),
                            fill: Some(theme.color(None, Style::Junction)),
                            width: theme.width(0.0, Style::Junction),
                            ..Default::default()
                        },
                    );
                }
                SchemaItem::LocalLabel(label) => {
                    let mut upright_label = label.clone();

                    // KiCad defaults LocalLabels to Left / Center if missing
                    if !upright_label.effects.justify.contains(&Justify::Left)
                        && !upright_label.effects.justify.contains(&Justify::Right)
                    {
                        upright_label.effects.justify.push(Justify::Left);
                    }
                    if !upright_label.effects.justify.contains(&Justify::Top)
                        && !upright_label.effects.justify.contains(&Justify::Bottom)
                    {
                        upright_label.effects.justify.push(Justify::Center);
                    }

                    let mut angle = upright_label.pos.angle % 360.0;
                    if angle < 0.0 {
                        angle += 360.0;
                    }
                    if angle > 90.0 && angle <= 270.0 {
                        angle = (angle + 180.0) % 360.0;
                    }
                    upright_label.pos.angle = angle;

                    upright_label.effects.font.face = Some(theme.face());
                    upright_label.effects.font.size =
                        theme.font_size(upright_label.effects.font.size, Style::Property);
                    upright_label.effects.font.color =
                        Some(theme.color(upright_label.effects.font.color, Style::Property));

                    bbox.push(upright_label.outline()?);
                    outline!(self, upright_label, plotter);

                    let (final_pos, final_effects) = resolve_text_layout(
                        &upright_label.text,
                        upright_label.pos,
                        upright_label.effects.clone(),
                    );
                    plotter.text(&upright_label.text, final_pos, final_effects);
                }
                SchemaItem::GlobalLabel(label) => {
                    let (text_w, text_h) =
                        match font::dimension(&label.text, &label.effects) {
                            Ok(d) => (d.x, d.y),
                            Err(_) => (label.text.len() as f64 * 1.0, 1.27), // Fallback estimation
                        };

                    let margin = 0.635;
                    let slant = 1.27;
                    let box_h = text_h + margin;
                    let half_h = box_h / 2.0;
                    let shape_type = label.shape.as_deref().unwrap_or(el::INPUT);
                    let body_len = text_w + margin;

                    let (pts, text_center_offset) = match shape_type {
                        el::INPUT => {
                            (
                                vec![
                                    Pt { x: 0.0, y: 0.0 },
                                    Pt {
                                        x: slant,
                                        y: -half_h,
                                    },
                                    Pt {
                                        x: slant + body_len,
                                        y: -half_h,
                                    },
                                    Pt {
                                        x: slant + body_len,
                                        y: half_h,
                                    },
                                    Pt {
                                        x: slant,
                                        y: half_h,
                                    },
                                    Pt { x: 0.0, y: 0.0 },
                                ],
                                // Text center X
                                slant + (body_len / 2.0),
                            )
                        }
                        el::OUTPUT => {
                            // Flat side at (0,0), arrow tip at right
                            let total_len = body_len + slant;
                            (
                                vec![
                                    Pt { x: 0.0, y: -half_h },
                                    Pt {
                                        x: body_len,
                                        y: -half_h,
                                    },
                                    Pt {
                                        x: total_len,
                                        y: 0.0,
                                    },
                                    Pt {
                                        x: body_len,
                                        y: half_h,
                                    },
                                    Pt { x: 0.0, y: half_h },
                                    Pt { x: 0.0, y: -half_h },
                                ],
                                // Text center X
                                body_len / 2.0,
                            )
                        }
                        el::BIDIRECTIONAL => {
                            // Arrow tips at both ends
                            // Tip 1 at (0,0), Tip 2 at right
                            let total_len = slant + body_len + slant;
                            (
                                vec![
                                    Pt { x: 0.0, y: 0.0 },
                                    Pt {
                                        x: slant,
                                        y: -half_h,
                                    },
                                    Pt {
                                        x: slant + body_len,
                                        y: -half_h,
                                    },
                                    Pt {
                                        x: total_len,
                                        y: 0.0,
                                    },
                                    Pt {
                                        x: slant + body_len,
                                        y: half_h,
                                    },
                                    Pt {
                                        x: slant,
                                        y: half_h,
                                    },
                                    Pt { x: 0.0, y: 0.0 },
                                ],
                                // Text center X
                                slant + (body_len / 2.0),
                            )
                        }
                        _ => {
                            // Passive / Rectangular
                            (
                                vec![
                                    Pt { x: 0.0, y: -half_h },
                                    Pt {
                                        x: body_len,
                                        y: -half_h,
                                    },
                                    Pt {
                                        x: body_len,
                                        y: half_h,
                                    },
                                    Pt { x: 0.0, y: half_h },
                                    Pt { x: 0.0, y: -half_h },
                                ],
                                body_len / 2.0,
                            )
                        }
                    };

                    // draw the shape
                    let transform = Transform::new()
                        .translation(label.pos.into())
                        .rotation(label.pos.angle);
                    let transformed_pts = transform.transform_pts(&pts);
                    let stroke_color = theme.color(label.effects.font.color, Style::Label);

                    plotter.polyline(
                        Pts(transformed_pts),
                        Paint {
                            color: stroke_color,
                            fill: Some(theme.fill(None, Style::Background)),
                            width: theme.width(0.0, Style::Label),
                            ..Default::default()
                        },
                    );

                    let mut text_pos = label.pos;
                    let angle_rad = label.pos.angle.to_radians();
                    text_pos.x += text_center_offset * angle_rad.cos();
                    text_pos.y += text_center_offset * angle_rad.sin();
                    let mut text_angle = label.pos.angle % 180.0;
                    if text_angle < 0.0 {
                        text_angle += 180.0;
                    }
                    let pos = Pos {
                        x: text_pos.x,
                        y: text_pos.y,
                        angle: text_angle,
                    };

                    let mut effects = label.effects.clone();
                    effects.justify = vec![Justify::Center, Justify::Center];

                    let final_effects = Effects {
                        font: Font {
                            face: Some(theme.face()),
                            size: theme.font_size(effects.font.size, Style::Property),
                            thickness: effects.font.thickness,
                            bold: effects.font.bold,
                            italic: effects.font.italic,
                            line_spacing: effects.font.line_spacing,
                            color: Some(stroke_color),
                        },
                        justify: effects.justify,
                        hide: effects.hide,
                        ..Default::default()
                    };

                    let (final_pos, final_effects) =
                        resolve_text_layout(&label.text, pos, final_effects);
                    plotter.text(&label.text, final_pos, final_effects);
                }
                SchemaItem::Polyline(poly) => {
                    plotter.polyline(
                        poly.pts.clone(),
                        Paint {
                            color: theme.color(poly.stroke.color, Style::Outline),
                            fill: match poly.fill {
                                Some(FillType::None) => None,
                                Some(FillType::Background) => {
                                    Some(theme.fill(None, Style::Background))
                                }
                                Some(FillType::Outline) => {
                                    Some(theme.fill(None, Style::Outline))
                                }
                                Some(FillType::Color(color)) => Some(color),
                                None => None,
                            },
                            width: theme.width(poly.stroke.width, Style::Outline),
                            ..Default::default()
                        },
                    );
                }
                SchemaItem::Text(text) => {
                    let mut upright_text = text.clone();

                    if !upright_text.effects.justify.contains(&Justify::Left)
                        && !upright_text.effects.justify.contains(&Justify::Right)
                    {
                        upright_text.effects.justify.push(Justify::Left);
                    }
                    if !upright_text.effects.justify.contains(&Justify::Top)
                        && !upright_text.effects.justify.contains(&Justify::Bottom)
                    {
                        upright_text.effects.justify.push(Justify::Center);
                    }

                    let mut angle = upright_text.pos.angle % 360.0;
                    if angle < 0.0 {
                        angle += 360.0;
                    }
                    if angle > 90.0 && angle <= 270.0 {
                        angle = (angle + 180.0) % 360.0;
                    }
                    upright_text.pos.angle = angle;

                    upright_text.effects.font.face = Some(theme.face());
                    upright_text.effects.font.size =
                        theme.font_size(upright_text.effects.font.size, Style::Property);
                    upright_text.effects.font.color =
                        Some(theme.color(upright_text.effects.font.color, Style::Property));

                    bbox.push(upright_text.outline()?);
                    outline!(self, upright_text, plotter);

                    let (final_pos, final_effects) = resolve_text_layout(
                        &upright_text.text,
                        upright_text.pos,
                        upright_text.effects.clone(),
                    );
                    plotter.text(&upright_text.text, final_pos, final_effects);
                }
                SchemaItem::Circle(circle) => {
                    plotter.circle(
                        circle.center,
                        circle.radius,
                        Paint {
                            color: theme.color(circle.stroke.color, Style::Outline),

                            fill: match circle.fill {
                                FillType::None => None,
                                FillType::Background => {
                                    Some(theme.fill(None, Style::Background))
                                }
                                FillType::Outline => {
                                    Some(theme.fill(None, Style::Outline))
                                }
                                FillType::Color(color) => Some(color),
                            },
                            width: theme.width(circle.stroke.width, Style::Wire),
                            ..Default::default()
                        },
                    );
                }
                SchemaItem::Arc(arc) => {
                    plotter.arc(
                        arc.start,
                        arc.mid,
                        arc.end,
                        Paint {
                            color: theme.color(arc.stroke.color, Style::Outline),
                            fill: match arc.fill {
                                FillType::None => None,
                                FillType::Background => {
                                    Some(theme.fill(None, Style::Background))
                                }
                                FillType::Outline => {
                                    Some(theme.fill(None, Style::Outline))
                                }
                                FillType::Color(color) => Some(color),
                            },
                            width: theme.width(arc.stroke.width, Style::Outline),
                            ..Default::default()
                        },
                    );
                }
                SchemaItem::Rectangle(rect) => {
                    let x_min = rect.start.x.min(rect.end.x);
                    let x_max = rect.start.x.max(rect.end.x);
                    let y_min = rect.start.y.min(rect.end.y);
                    let y_max = rect.start.y.max(rect.end.y);

                    plotter.rect(
                        Rect {
                            start: Pt { x: x_min, y: y_min },
                            end: Pt {
                                x: x_max - x_min,
                                y: y_max - y_min,
                            },
                        },
                        Paint {
                            color: theme.color(rect.stroke.color, Style::Outline),
                            fill: match rect.fill {
                                FillType::None => None,
                                FillType::Background => {
                                    Some(theme.fill(None, Style::Background))
                                }
                                FillType::Outline => {
                                    Some(theme.fill(None, Style::Outline))
                                }
                                FillType::Color(color) => Some(color),
                            },
                            width: theme.width(rect.stroke.width, Style::Wire),
                            ..Default::default()
                        },
                    );
                }
                SchemaItem::TextBox(textbox) => {
                    plotter.rect(
                        Rect {
                            start: textbox.pos.into(),
                            end: Pt {
                                x: textbox.width,
                                y: textbox.height,
                            },
                        },
                        Paint {
                            color: theme.color(textbox.stroke.color, Style::Outline),

                            fill: match textbox.fill {
                                FillType::None => None,
                                FillType::Background => {
                                    Some(theme.fill(None, Style::Background))
                                }
                                FillType::Outline => {
                                    Some(theme.fill(None, Style::Outline))
                                }
                                FillType::Color(color) => Some(color),
                            },
                            width: theme.width(textbox.stroke.width, Style::Wire),
                            ..Default::default()
                        },
                    );
                    spdlog::warn!("Plot:TextBox: {:?}", theme.face(),);
                    let effects = Effects {
                        font: Font {
                            face: textbox.effects.font.face.clone(),
                            size: theme.font_size(textbox.effects.font.size, Style::Property),
                            thickness: textbox.effects.font.thickness,
                            bold: textbox.effects.font.bold,
                            italic: textbox.effects.font.italic,
                            line_spacing: textbox.effects.font.line_spacing,
                            color: Some(theme.color(textbox.effects.font.color, Style::Property)),
                        },
                        justify: textbox.effects.justify.clone(),
                        hide: textbox.effects.hide,
                        ..Default::default()
                    };
                    let (final_pos, final_effects) =
                        resolve_text_layout(&textbox.text, textbox.pos, effects);
                    plotter.text(&textbox.text, final_pos, final_effects);
                }
                SchemaItem::HierarchicalSheet(sheet) => {
                    if let (Some(sheet), Some(filename)) = (sheet.sheet(), sheet.filename()) {
                        pages.push((sheet, filename));
                    } else {
                        spdlog::warn!(
                            "HierarchicalSheet sheet of filename empty: {:?}, {:?}",
                            sheet.sheet(),
                            sheet.filename()
                        );
                    }
                    plotter.rect(
                        Rect {
                            start: sheet.pos.into(),
                            end: Pt {
                                x: sheet.width,
                                y: sheet.height,
                            },
                        },
                        Paint {
                            color: theme.color(sheet.stroke.color, Style::Outline),
                            fill: match sheet.fill {
                                FillType::None => None,
                                FillType::Background => {
                                    Some(theme.fill(None, Style::Background))
                                }
                                FillType::Outline => {
                                    Some(theme.fill(None, Style::Outline))
                                }
                                FillType::Color(color) => Some(color),
                            },
                            width: theme.width(sheet.stroke.width, Style::Outline),
                            ..Default::default()
                        },
                    );

                    for prop in &sheet.props {
                        if prop.visible() {
                            let effects = Effects {
                                font: Font {
                                    face: Some(theme.face()),
                                    size: theme.font_size(prop.effects.font.size, Style::Property),
                                    thickness: prop.effects.font.thickness,
                                    bold: prop.effects.font.bold,
                                    italic: prop.effects.font.italic,
                                    line_spacing: prop.effects.font.line_spacing,
                                    color: Some(
                                        theme.color(prop.effects.font.color, Style::Property),
                                    ),
                                },
                                justify: prop.effects.justify.clone(),
                                hide: prop.visible(),
                                ..Default::default()
                            };

                            let (final_pos, final_effects) =
                                resolve_text_layout(&prop.value, prop.pos, effects);
                            plotter.text(&prop.value, final_pos, final_effects);
                        }
                    }

                    for pin in &sheet.pins {
                        if pin.effects.hide {
                            continue;
                        }

                        let h = 0.635;
                        let l = 1.27;
                        let (pts, text_offset) = match format!("{:?}", pin.connection_type).as_str()
                        {
                            "Input" => (
                                // Points Left (Into the sheet relative to Right Edge)
                                vec![
                                    Pt { x: 0.0, y: -h },
                                    Pt { x: -l, y: 0.0 },
                                    Pt { x: 0.0, y: h },
                                    Pt { x: 0.0, y: -h },
                                ],
                                l,
                            ),
                            "Output" => (
                                // Points Right (Tip at edge)
                                vec![
                                    Pt { x: -l, y: -h },
                                    Pt { x: 0.0, y: 0.0 },
                                    Pt { x: -l, y: h },
                                    Pt { x: -l, y: -h },
                                ],
                                l,
                            ),
                            "Bidirectional" => (
                                // Diamond
                                vec![
                                    Pt { x: 0.0, y: 0.0 },
                                    Pt { x: -l / 2.0, y: h },
                                    Pt { x: -l, y: 0.0 },
                                    Pt { x: -l / 2.0, y: -h },
                                    Pt { x: 0.0, y: 0.0 },
                                ],
                                l,
                            ),
                            _ => (vec![Pt { x: 0.0, y: 0.0 }, Pt { x: -l, y: 0.0 }], l),
                        };

                        let transform = Transform::new()
                            .translation(pin.pos.into())
                            .rotation(pin.pos.angle);

                        let transformed_pts = transform.transform_pts(&pts);

                        plotter.polyline(
                            Pts(transformed_pts),
                            Paint {
                                color: theme.color(pin.effects.font.color, Style::PinName),
                                fill: Some(theme.fill(None, Style::Background)),
                                width: theme.width(0.0, Style::PinName),
                                ..Default::default()
                            },
                        );

                        let margin = 0.6;
                        let dist = text_offset + margin;

                        let angle_rad = (pin.pos.angle + 180.0).to_radians();
                        let mut text_pos = pin.pos;
                        text_pos.x += dist * angle_rad.cos();
                        text_pos.y += dist * angle_rad.sin();

                        let mut text_rot = pin.pos.angle % 180.0;
                        if text_rot < 0.0 {
                            text_rot += 180.0;
                        }

                        let pos = Pos {
                            x: text_pos.x,
                            y: text_pos.y,
                            angle: text_rot,
                        };

                        let effects = Effects {
                            font: Font {
                                face: Some(theme.face()),
                                size: theme.font_size(pin.effects.font.size, Style::PinName),
                                thickness: pin.effects.font.thickness,
                                bold: pin.effects.font.bold,
                                italic: pin.effects.font.italic,
                                line_spacing: pin.effects.font.line_spacing,
                                color: Some(theme.color(pin.effects.font.color, Style::PinName)),
                            },
                            justify: pin.effects.justify.clone(),
                            hide: pin.effects.hide,
                            ..Default::default()
                        };

                        let (final_pos, final_effects) =
                            resolve_text_layout(&pin.name, pos, effects);
                        plotter.text(&pin.name, final_pos, final_effects);
                    }
                }
                SchemaItem::HierarchicalLabel(label) => {
                    let h = 0.635;
                    let l = 1.524;
                    let slant = 0.635;
                    let (pts, text_offset) = match label.shape.as_deref().unwrap_or(el::INPUT) {
                        el::INPUT => (
                            vec![
                                Pt { x: 0.0, y: 0.0 },
                                Pt { x: slant, y: -h },
                                Pt { x: l, y: -h },
                                Pt { x: l, y: h },
                                Pt { x: slant, y: h },
                                Pt { x: 0.0, y: 0.0 },
                            ],
                            l,
                        ),
                        el::OUTPUT => (
                            vec![
                                Pt { x: 0.0, y: -h },
                                Pt {
                                    x: l - slant,
                                    y: -h,
                                },
                                Pt { x: l, y: 0.0 },
                                Pt { x: l - slant, y: h },
                                Pt { x: 0.0, y: h },
                                Pt { x: 0.0, y: -h },
                            ],
                            l,
                        ),
                        el::BIDIRECTIONAL => (
                            vec![
                                Pt { x: 0.0, y: 0.0 },
                                Pt { x: slant, y: -h },
                                Pt {
                                    x: l - slant,
                                    y: -h,
                                },
                                Pt { x: l, y: 0.0 },
                                Pt { x: l - slant, y: h },
                                Pt { x: slant, y: h },
                                Pt { x: 0.0, y: 0.0 },
                            ],
                            l,
                        ),
                        _ => (
                            // Passive / Rectangular fallback
                            vec![
                                Pt { x: 0.0, y: -h },
                                Pt { x: l, y: -h },
                                Pt { x: l, y: h },
                                Pt { x: 0.0, y: h },
                                Pt { x: 0.0, y: -h },
                            ],
                            l,
                        ),
                    };

                    let transform = Transform::new()
                        .translation(label.pos.into())
                        .rotation(label.pos.angle);

                    let transformed_pts = transform.transform_pts(&pts);

                    plotter.polyline(
                        Pts(transformed_pts),
                        Paint {
                            color: theme.color(label.effects.font.color, Style::Label),
                            fill: Some(theme.fill(None, Style::Background)),
                            width: theme.width(0.0, Style::Label),
                            ..Default::default()
                        },
                    );

                    let margin = 0.3;
                    let total_offset = text_offset + margin;

                    let mut text_pos = label.pos;
                    let angle_rad = label.pos.angle.to_radians();
                    text_pos.x += total_offset * angle_rad.cos();
                    text_pos.y += total_offset * angle_rad.sin();

                    let effects = Effects {
                        font: Font {
                            face: Some(theme.face()),
                            size: theme.font_size(label.effects.font.size, Style::Property),
                            thickness: label.effects.font.thickness,
                            bold: label.effects.font.bold,
                            italic: label.effects.font.italic,
                            line_spacing: label.effects.font.line_spacing,
                            color: Some(theme.color(label.effects.font.color, Style::Property)),
                        },
                        justify: label.effects.justify.clone(),
                        hide: label.effects.hide,
                        ..Default::default()
                    };

                    let (final_pos, final_effects) =
                        resolve_text_layout(&label.text, text_pos, effects);
                    plotter.text(&label.text, final_pos, final_effects);
                }
                SchemaItem::NetclassFlag(flag) => {
                    if let Some(shape) = &flag.shape {
                        if shape == "round" {
                            plotter.circle(
                                flag.pos.into(),
                                el::JUNCTION_DIAMETER / 2.0,
                                Paint {
                                    color: theme.color(None, Style::Junction),
                                    fill: None,
                                    width: theme.width(0.0, Style::Wire),
                                    ..Default::default()
                                },
                            );
                        }
                    }

                    for prop in &flag.props {
                        if prop.visible() {
                            let effects = Effects {
                                font: Font {
                                    face: Some(theme.face()),
                                    size: theme.font_size(prop.effects.font.size, Style::Property),
                                    thickness: prop.effects.font.thickness,
                                    bold: prop.effects.font.bold,
                                    italic: prop.effects.font.italic,
                                    line_spacing: prop.effects.font.line_spacing,
                                    color: Some(
                                        theme.color(prop.effects.font.color, Style::Property),
                                    ),
                                },
                                justify: prop.effects.justify.clone(),
                                hide: prop.visible(),
                                ..Default::default()
                            };
                            let (final_pos, final_effects) =
                                resolve_text_layout(&prop.value, prop.pos, effects);
                            plotter.text(&prop.value, final_pos, final_effects);
                        }
                    }
                }
                SchemaItem::BusEntry(entry) => {
                    let start = Pt {
                        x: entry.pos.x,
                        y: entry.pos.y,
                    };
                    let end = Pt {
                        x: entry.pos.x + entry.size.0,
                        y: entry.pos.y + entry.size.1,
                    };
                    plotter.move_to(start);
                    plotter.line_to(end);
                    plotter.stroke(Paint {
                        color: theme.color(entry.stroke.color, Style::Wire),
                        fill: None,
                        width: theme.width(entry.stroke.width, Style::Wire),
                        ..Default::default()
                    });
                }
                SchemaItem::Bus(bus) => {
                    if let Some(first) = bus.pts.0.first() {
                        plotter.move_to(*first);
                        for pt in bus.pts.0.iter().skip(1) {
                            plotter.line_to(*pt);
                        }
                        plotter.stroke(Paint {
                            color: theme.color(bus.stroke.color, Style::Wire),
                            fill: None,
                            width: theme.width(bus.stroke.width, Style::Wire),
                            ..Default::default()
                        });
                    }
                }
                _ => spdlog::error!("plotting item not supported: {:?}", item),
            }
        }

        if command.border {
            // Draw the border LAST so it sits on top, or FIRST if you want it background.
            // KiCad usually draws it on a specific drawing sheet layer.
            draw_border(
                plotter,
                &self.paper,
                &self.title_block,
                &self.path.clone().unwrap_or("{none}".to_string()),
                &self.sheet.clone().unwrap_or("/".to_string()),
                &theme,
            );
        }

        let mut min_x = f64::MAX;
        let mut min_y = f64::MAX;
        let mut min_w = f64::MIN;
        let mut min_h = f64::MIN;
        for point in bbox {
            min_x = min_x.min(point.start.x);
            min_y = min_y.min(point.start.y);
            min_x = min_x.min(point.end.x);
            min_y = min_y.min(point.end.y);
            min_w = min_w.max(point.start.x);
            min_h = min_h.max(point.start.y);
            min_w = min_w.max(point.end.x);
            min_h = min_h.max(point.end.y);
        }
        min_x -= 1.0;
        min_y -= 1.0;
        min_w += 1.0;
        min_h += 1.0;

        // apply scaling and view box...
        plotter.scale(command.scale);
        if command.border {
            plotter.set_view_box(Rect {
                start: Pt { x: 0.0, y: 0.0 },
                end: Pt {
                    x: paper_size.0,
                    y: paper_size.1,
                },
            });
        } else {
            plotter.set_view_box(Rect {
                start: Pt { x: min_x, y: min_y },
                end: Pt {
                    x: min_w - min_x,
                    y: min_h - min_y,
                },
            });
        }

        if cfg!(debug_assertions) {
            plotter.rect(
                Rect {
                    start: Pt { x: min_x, y: min_y },
                    end: Pt {
                        x: min_w - min_x,
                        y: min_h - min_y,
                    },
                },
                Paint::red(),
            );
        }
        //add the pages to the plotter
        plotter.set_pages(pages);
        Ok(())
    }
}

fn polyline(
    plotter: &mut impl Plotter,
    transform: &Transform,
    poly: &Polyline,
    style: &Style,
    theme: &Theme,
) {
    let pts = transform.transform_pts(&poly.pts.0);
    for (i, p) in pts.iter().enumerate() {
        if i == 0 {
            plotter.move_to(*p);
        } else {
            plotter.line_to(*p);
        }
    }
    plotter.stroke(Paint {
        color: theme.color(poly.stroke.color, style.clone()),
        fill: match poly.fill {
            Some(FillType::None) => None,
            Some(FillType::Background) => Some(theme.fill(None, Style::Background)),
            Some(FillType::Outline) => Some(theme.fill(None, Style::Outline)),
            Some(FillType::Color(color)) => Some(color),
            None => None,
        },
        width: theme.width(poly.stroke.width, style.clone()),
        linecap: Linecap::Round,
        ..Default::default()
    });
}
fn arc(plotter: &mut impl Plotter, transform: &Transform, arc: &Arc, style: &Style, theme: &Theme) {
    plotter.arc(
        transform.transform_point(arc.start),
        transform.transform_point(arc.mid),
        transform.transform_point(arc.end),
        Paint {
            color: theme.color(None, style.clone()),
            fill: None,
            width: theme.width(0.0, style.clone()),
            ..Default::default()
        },
    );
}
fn rectangle<P: Plotter>(
    plotter: &mut P,
    transform: &Transform,
    rect: &Rectangle,
    style: &Style,
    theme: &Theme,
) {
    let p1 = transform.transform_point(rect.start);
    let p2 = transform.transform_point(rect.end);
    let x_min = p1.x.min(p2.x);
    let x_max = p1.x.max(p2.x);
    let y_min = p1.y.min(p2.y);
    let y_max = p1.y.max(p2.y);

    plotter.rect(
        Rect {
            start: Pt { x: x_min, y: y_min },
            end: Pt {
                x: x_max - x_min,
                y: y_max - y_min,
            },
        },
        Paint {
            color: theme.color(None, style.clone()),
            fill: None,
            width: theme.width(0.0, style.clone()),
            ..Default::default()
        },
    );
}
fn circle<P: Plotter>(
    plotter: &mut P,
    transform: &Transform,
    circle: &Circle,
    style: &Style,
    theme: &Theme,
) {
    let t_center = transform.transform_point(circle.center);
    plotter.circle(
        t_center,
        circle.radius,
        Paint {
            color: theme.color(None, style.clone()),
            fill: None,
            width: theme.width(0.0, style.clone()),
            ..Default::default()
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn pin<P: Plotter>(
    plotter: &mut P,
    transform: &Transform,
    pin: &Pin,
    pin_numbers: bool,
    pin_names: bool,
    pin_names_offset: Option<f64>,
    power: bool,
    style: &Style,
    theme: &Theme,
    _symbol_pos: Pos,
) {
    // Pin definition:
    // In KiCad library coordinates, a pin points "Right" (0 deg) from (0,0) to (Length, 0).
    // The direction is handled by the `transform` which includes the pin's local rotation.
    // We should NOT flip the length based on angle here manually; let `transform` do the rotation.
    let pin_line = [
        Pt { x: 0.0, y: 0.0 },
        Pt {
            x: pin.length,
            y: 0.0,
        },
    ];

    let transform_pin = Transform::new()
        .translation(Pt {
            x: pin.pos.x,
            y: pin.pos.y,
        })
        .rotation(pin.pos.angle);

    let pin_pts_local = transform_pin.transform_pts(&pin_line);
    let pts = transform.transform_pts(&pin_pts_local);

    plotter.move_to(pts[0]);
    plotter.line_to(pts[1]);
    plotter.stroke(Paint {
        color: theme.color(None, style.clone()),
        fill: None,
        width: theme.width(0.0, style.clone()),
        ..Default::default()
    });

    let p0 = pts[0];
    let p1 = pts[1];
    let d = p1 - p0;
    let angle_rad = d.y.atan2(d.x);
    let angle_deg = (angle_rad.to_degrees() + 360.0) % 360.0; // Normalize 0..360

    if !pin_numbers && !power {
        let mid = Pt {
            x: (p0.x + p1.x) / 2.0,
            y: (p0.y + p1.y) / 2.0,
        };

        let dist = 1.27; // 50 mils

        let offset = if (45.0..135.0).contains(&angle_deg) || (225.0..315.0).contains(&angle_deg) {
            Pt { x: -dist, y: 0.0 }
        } else {
            Pt { x: 0.0, y: -dist }
        };

        let mut effects = pin.number.effects.clone();
        effects.font.face = Some(theme.face());
        effects.font.size = theme.font_size(pin.number.effects.font.size, Style::PinNumber);
        effects.font.color = Some(theme.color(None, Style::PinNumber));
        effects.justify = vec![Justify::Center, Justify::Center];
        effects.hide = false;

        let pos = Pos {
            x: mid.x + offset.x,
            y: mid.y + offset.y,
            angle: 0.0,
        };

        let (final_pos, final_effects) = resolve_text_layout(&pin.number.name, pos, effects);
        plotter.text(&pin.number.name, final_pos, final_effects);
    }

    
    if !pin_names && pin.name.name != "~" && !power {
        let Some(mut offset) = pin_names_offset else {
            return;
        };

        let len = (d.x * d.x + d.y * d.y).sqrt();
        let dir = if len > 0.001 {
            Pt {
                x: d.x / len,
                y: d.y / len,
            }
        } else {
            Pt { x: 1.0, y: 0.0 }
        };

        let (justify, text_rot) = if !(45.0..315.0).contains(&angle_deg) {
            (vec![Justify::Left], 0.0)
        } else if (45.0..135.0).contains(&angle_deg) {
            (vec![Justify::Right], 90.0)
        } else if (135.0..225.0).contains(&angle_deg) {
            (vec![Justify::Right], 0.0)
        } else {
            (vec![Justify::Left], 90.0)
        };

        //padding from pin
        offset += pin.name.effects.font.size.0 as f64 * 0.15;

        let anchor = Pt {
            x: p1.x + dir.x * offset,
            y: p1.y + dir.y * offset,
        };

        let mut effects = Effects::default(); 
        effects.font.face = Some(theme.face());
        effects.font.size = theme.font_size(pin.name.effects.font.size, Style::PinName);
        effects.font.color = Some(theme.color(None, Style::PinName));
        effects.justify = justify;
        effects.hide = false;

        let pos = Pos {
            x: anchor.x,
            y: anchor.y,
            angle: text_rot,
        };

        let (final_pos, final_effects) = resolve_text_layout(&pin.name.name, pos, effects);
        plotter.text(&pin.name.name, final_pos, final_effects);
    }
}

