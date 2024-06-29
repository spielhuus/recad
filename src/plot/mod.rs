//!Plot the recad drawings.
use std::{fmt, io::Write};

use lazy_static::lazy_static;
use ndarray::{arr2, Array2, Axis};

use crate::{
    gr::{Circle, Color, GraphicItem, Polyline, Pt, Pts, Rect, Rectangle},
    math::{bbox::Bbox, ToNdarray, Transform},
    schema,
    sexp::constants::el,
    Schema,
};

mod svg;
pub mod theme;

pub use svg::SvgPlotter;

use theme::{Style, Theme, Themes};

//crwate a macro with the name outline and 1 parameter
macro_rules! outline {
    ($self:expr, $item:expr) => {
        if cfg!(debug_assertions) {
            let outline = $item.outline(&$self.schema);
            $self.plotter.rect(
                Rect {
                    start: outline.start,
                    end: Pt {
                        x: outline.end.x - outline.start.x,
                        y: outline.end.y - outline.start.y,
                    },
                },
                Paint::red(),
            );
        }
    }
}

///The paint for the plotter.
#[derive(Clone)]
pub struct Paint {
    color: Color,
    fill: Option<Color>,
    width: f32,
}

impl Paint {
    pub fn black() -> Self {
        Self {
            color: Color::black(),
            fill: None,
            width: 0.25,
        }
    }
    pub fn red() -> Self {
        Self {
            color: Color::red(),
            fill: None,
            width: 0.25,
        }
    }
    pub fn green() -> Self {
        Self {
            color: Color::green(),
            fill: None,
            width: 0.25,
        }
    }
    pub fn blue() -> Self {
        Self {
            color: Color::blue(),
            fill: None,
            width: 0.25,
        }
    }
    pub fn grey() -> Self {
        Self {
            color: Color::grey(),
            fill: None,
            width: 0.25,
        }
    }
}

///The fot effects for the drawings.
pub struct FontEffects {
    angle: f32,
    anchor: String,
    baseline: String,
    face: String,
    size: f32,
    color: Color,
}

#[derive(Debug)]
//Line CAP, endings.
pub enum LineCap {
    Butt,
    Round,
    Square,
}

impl fmt::Display for LineCap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LineCap::Butt => write!(f, "butt"),
            LineCap::Round => write!(f, "round"),
            LineCap::Square => write!(f, "square"),
        }
    }
}

pub trait Plotter {
    fn open(&self);

    ///set the view box.
    fn set_view_box(&mut self, rect: Rect);

    ///Move the path cursor to position.
    fn move_to(&mut self, pt: Pt);
    ///Draw a line to position.
    fn line_to(&mut self, pt: Pt);
    ///Close the path.
    fn close(&mut self);
    ///Sroke the path.
    fn stroke(&mut self, stroke: Paint);

    ///Draw a rectancle with stroke.
    fn rect(&mut self, r: Rect, stroke: Paint);
    fn circle(&mut self, center: Pt, radius: f32, stroke: Paint);
    fn text(&mut self, text: &str, pt: Pt, effects: FontEffects);

    ///Draw a polyline with the given Pts.
    fn polyline(&mut self, pts: Pts, stroke: Paint);

    ///Write the result to a Writer.
    fn write<W: Write>(self, writer: &mut W) -> std::io::Result<()>;
}

lazy_static! {
    static ref NO_CONNECT_R: Array2<f32> = arr2(&[
        [-el::NO_CONNECT_SIZE, -el::NO_CONNECT_SIZE],
        [el::NO_CONNECT_SIZE, el::NO_CONNECT_SIZE]
    ]);
    static ref NO_CONNECT_L: Array2<f32> = arr2(&[
        [-el::NO_CONNECT_SIZE, el::NO_CONNECT_SIZE],
        [el::NO_CONNECT_SIZE, -el::NO_CONNECT_SIZE]
    ]);
}

pub struct SchemaPlotter<P: Plotter> {
    plotter: P,
    schema: Schema,
    theme: Theme,
}

impl<P: Plotter> SchemaPlotter<P> {
    pub fn new(schema: Schema, plotter: P, theme: Themes) -> Self {
        Self {
            schema,
            plotter,
            theme: Theme::from(theme),
        }
    }

    pub fn plot(&mut self) {
        let paper_size: (f32, f32) = self.schema.paper.clone().into();
        self.plotter.set_view_box(Rect {
            start: Pt { x: 0.0, y: 0.0 },
            end: Pt {
                x: paper_size.0,
                y: paper_size.1,
            },
        });

        for symbol in &self.schema.symbols {
            outline!(self, symbol);
            for prop in &symbol.props {
                if prop.visible() {
                    outline!(self, prop);
                    self.plotter.text(
                        &prop.value,
                        prop.pos.into(),
                        FontEffects {
                            angle: if symbol.pos.angle + prop.pos.angle >= 360.0 {
                                symbol.pos.angle + prop.pos.angle - 360.0
                            } else if symbol.pos.angle + prop.pos.angle >= 180.0 {
                                symbol.pos.angle + prop.pos.angle - 180.0
                            } else {
                                symbol.pos.angle + prop.pos.angle
                            },
                            anchor: prop.effects.anchor(),
                            baseline: prop.effects.baseline(),
                            face: self.theme.face(), //TODO prop.effects.font.face.clone().unwrap(),
                            size: self
                                .theme
                                .font_size(prop.effects.font.size, Style::Property)
                                .0,
                            color: self.theme.color(prop.effects.font.color, Style::Property),
                        },
                    );
                }
            }

            let library = self.schema.library_symbol(&symbol.lib_id).unwrap();
            let transform = Transform::new()
                .translation(symbol.pos.into())
                .rotation(symbol.pos.angle)
                .mirror(&Some(String::from("x"))); //&symbol.mirror);

            for lib_symbol in &library.units {
                if lib_symbol.unit() == 0 || lib_symbol.unit() == symbol.unit {
                    for g in &lib_symbol.graphics {
                        match g {
                            GraphicItem::Polyline(p) => {
                                polyline(
                                    &mut self.plotter,
                                    &transform,
                                    p,
                                    &Style::Outline,
                                    &self.theme,
                                );
                            }
                            GraphicItem::Rectangle(p) => {
                                rectangle(
                                    &mut self.plotter,
                                    &transform,
                                    p,
                                    &Style::Outline,
                                    &self.theme,
                                );
                            }
                            GraphicItem::Circle(c) => {
                                circle(
                                    &mut self.plotter,
                                    &transform,
                                    c,
                                    &Style::Outline,
                                    &self.theme,
                                );
                            }
                            _ => {
                                log::warn!("unknown graphic item: {:?}", g);
                            }
                        }
                    }
                }
            }
            for p in &library.pins(symbol.unit) {
                pin(
                    &mut self.plotter,
                    &transform,
                    p,
                    library.pin_numbers,
                    library.pin_names,
                    library.pin_names_offset,
                    library.power,
                    &Style::Outline,
                    &self.theme,
                );
            }
        }
        for wire in &self.schema.wires {
            outline!(self, wire);
            let pts1 = wire.pts.0.first().expect("pts[0] should exist");
            let pts2 = wire.pts.0.get(1).expect("pts[0] should exist");
            self.plotter.move_to(*pts1);
            self.plotter.line_to(*pts2);
            self.plotter.stroke(Paint {
                color: self.theme.color(wire.stroke.color, Style::Wire),
                fill: None,
                width: self.theme.width(wire.stroke.width, Style::Wire),
            });
        }
        for nc in &self.schema.no_connects {
            outline!(self, nc);
            let transform = Transform::new().translation(nc.pos.into());
            let r = transform.transform(&NO_CONNECT_R);
            let l = transform.transform(&NO_CONNECT_L);

            self.plotter.move_to(Pt {
                x: r[[0, 0]],
                y: r[[0, 1]],
            });
            self.plotter.line_to(Pt {
                x: r[[1, 0]],
                y: r[[1, 1]],
            });
            self.plotter.stroke(Paint {
                color: self.theme.color(None, Style::NoConnect),
                fill: None,
                width: self.theme.width(0.0, Style::NoConnect),
            });

            self.plotter.move_to(Pt {
                x: l[[0, 0]],
                y: l[[0, 1]],
            });
            self.plotter.line_to(Pt {
                x: l[[1, 0]],
                y: l[[1, 1]],
            });
            self.plotter.stroke(Paint {
                color: self.theme.color(None, Style::NoConnect),
                fill: None,
                width: self.theme.width(0.0, Style::NoConnect),
            });
        }
        for junction in &self.schema.junctions {
            outline!(self, junction);
            self.plotter.circle(
                junction.pos.into(),
                if junction.diameter == 0.0 {
                    el::JUNCTION_DIAMETER / 2.0
                } else {
                    junction.diameter / 2.0
                },
                Paint {
                    color: self.theme.color(None, Style::Junction),
                    fill: None,
                    width: self.theme.width(0.0, Style::Junction),
                },
            );
        }
        for label in &self.schema.local_labels {
            outline!(self, label);
            let text_pos: Array2<f32> = if label.pos.angle == 0.0 {
                arr2(&[[label.pos.x + 1.0, label.pos.y]])
            } else if label.pos.angle == 90.0 {
                arr2(&[[label.pos.x, label.pos.y - 1.0]])
            } else if label.pos.angle == 180.0 {
                arr2(&[[label.pos.x - 1.0, label.pos.y]])
            } else {
                arr2(&[[label.pos.x, label.pos.y + 1.0]])
            };
            let text_angle = if label.pos.angle >= 180.0 {
                label.pos.angle - 180.0
            } else {
                label.pos.angle
            };
            self.plotter.text(
                &label.text,
                text_pos.ndarray(),
                FontEffects {
                    angle: text_angle,
                    anchor: label.effects.anchor(),
                    baseline: label.effects.baseline(),
                    face: self.theme.face(), //TODO label.effects.font.face.clone().unwrap(),
                    size: self
                        .theme
                        .font_size(label.effects.font.size, Style::Label)
                        .0,
                    color: self.theme.color(label.effects.font.color, Style::Property),
                },
            );
        }
        
        for label in &self.schema.global_labels {
            outline!(self, label);
            //let angle: f64 = utils::angle(item.item).unwrap();
            //let pos: Array1<f64> = utils::at(.item).unwrap();
            let text_pos: Array2<f32> = if label.pos.angle == 0.0 {
                arr2(&[[label.pos.x + 1.0, label.pos.y]])
            } else if label.pos.angle == 90.0 {
                arr2(&[[label.pos.x, label.pos.y - 1.0]])
            } else if label.pos.angle == 180.0 {
                arr2(&[[label.pos.x - 1.0, label.pos.y]])
            } else {
                arr2(&[[label.pos.x, label.pos.y + 1.0]])
            };
            let text_angle = if label.pos.angle >= 180.0 {
                label.pos.angle - 180.0
            } else {
                label.pos.angle
            };
            self.plotter.text(
                &label.text,
                text_pos.ndarray(),
                FontEffects {
                    angle: text_angle,
                    anchor: label.effects.anchor(),
                    baseline: label.effects.baseline(),
                    face: self.theme.face(), //TODO label.effects.font.face.clone().unwrap(),
                    size: self
                        .theme
                        .font_size(label.effects.font.size, Style::Label)
                        .0,
                    color: self.theme.color(label.effects.font.color, Style::Property),
                },
            );

            //if item.global {
            //    let mut outline = LabelElement::make_label(size);
            //    if angle != 0.0 {
            //        let theta = angle.to_radians();
            //        let rot = arr2(&[[theta.cos(), -theta.sin()], [theta.sin(), theta.cos()]]);
            //        outline = outline.dot(&rot);
            //    }
            //    outline = outline + pos.clone();
            //    plot_items.push(PlotItem::Polyline(
            //        10,
            //        Polyline::new(
            //            outline,
            //            self.theme.get_stroke(
            //                Stroke::new(),
            //                &[Style::GlobalLabel, Style::Fill(FillType::Background)],
            //            ),
            //            Some(LineCap::Round),
            //            None,
            //        ),
            //    ));
            //}
        }
        let outline = self.schema.outline();
        self.plotter.rect(
            Rect {
                start: outline.start,
                end: Pt {
                    x: outline.end.x - outline.start.x,
                    y: outline.end.y - outline.start.y,
                },
            },
            Paint::red(),
        );
    }

    pub fn write<W: Write>(self, writer: &mut W) -> std::io::Result<()> {
        self.plotter.write(writer)
    }
}

fn polyline<P: Plotter>(
    plotter: &mut P,
    transform: &Transform,
    poly: &Polyline,
    style: &Style,
    theme: &Theme,
) {
    let pts = transform.transform(&poly.pts.ndarray());
    for (i, p) in pts.axis_iter(Axis(0)).enumerate() {
        if i == 0 {
            plotter.move_to(Pt { x: p[0], y: p[1] });
        } else {
            plotter.line_to(Pt { x: p[0], y: p[1] });
        }
    }
    plotter.stroke(Paint {
        color: theme.color(None, style.clone()),
        fill: None,
        width: theme.width(0.0, style.clone()),
    });
}

fn rectangle<P: Plotter>(
    plotter: &mut P,
    transform: &Transform,
    rect: &Rectangle,
    style: &Style,
    theme: &Theme,
) {
    let rect = arr2(&[[rect.start.x, rect.start.y], [rect.end.x, rect.end.y]]);
    let t = transform.transform(&rect);

    let x = if t[[0, 0]] > t[[1, 0]] {
        t[[1, 0]]
    } else {
        t[[0, 0]]
    };
    let y = if t[[0, 1]] > t[[1, 1]] {
        t[[1, 1]]
    } else {
        t[[0, 1]]
    };
    let width = (t[[1, 0]] - t[[0, 0]]).abs();
    let height = (t[[1, 1]] - t[[0, 1]]).abs();
    plotter.rect(
        Rect {
            start: Pt { x, y },
            end: Pt {
                x: width,
                y: height,
            },
        },
        Paint {
            color: theme.color(None, style.clone()),
            fill: None,
            width: theme.width(0.0, style.clone()),
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
    let center = arr2(&[[circle.center.x, circle.center.y]]);
    let t = transform.transform(&center);
    plotter.circle(
        Pt { x: t[[0, 0]], y: t[[0, 1]] },
        circle.radius,
        Paint {
            color: theme.color(None, style.clone()),
            fill: None,
            width: theme.width(0.0, style.clone()),
        },
    );
}

#[allow(clippy::too_many_arguments)]
fn pin<P: Plotter>(
    plotter: &mut P,
    transform: &Transform,
    pin: &schema::Pin,
    pin_numbers: bool,
    pin_names: bool,
    pin_names_offset: Option<f32>,
    power: bool,
    style: &Style,
    theme: &Theme,
) {
    let pin_line: Pts = Pts(vec![
        Pt { x: 0.0, y: 0.0 },
        Pt {
            x: pin.length,
            y: 0.0,
        },
    ]);
    let transform_pin = Transform::new()
        .translation(Pt {
            x: pin.pos.x,
            y: pin.pos.y,
        })
        .rotation(pin.pos.angle);
    let pin_pts = transform_pin.transform(&pin_line.ndarray());
    let pts: Pts = transform.transform(&pin_pts).ndarray();
    //TODO draw differnt pin graphic types.
    //https://github.com/KiCad/kicad-source-mirror/blob/c36efec4b20a59e306735e5ecbccc4b59c01460e/eeschema/sch_pin.cpp#L245

    plotter.move_to(pts.0[0]);
    plotter.line_to(pts.0[1]);
    plotter.stroke(Paint {
        color: theme.color(None, style.clone()),
        fill: None,
        width: theme.width(0.0, style.clone()),
    });

    if pin_numbers && !power {
        let to = match pin.pos.angle {
            0.0 => Pt {
                x: pin.length / 2.0,
                y: -0.75,
            },
            90.0 => Pt {
                x: 0.0,
                y: pin.length / 2.0,
            },
            180.0 => Pt {
                x: -pin.length / 2.0,
                y: -0.75,
            },
            270.0 => Pt {
                x: 0.0,
                y: pin.length / 2.0,
            },
            _ => {
                panic!("pin angle: {}, not supported", pin.pos.angle);
            }
        };

        let translate = Transform::new().translation(Pt { x: to.x, y: to.y });
        let line: Pts = translate.transform(&pts.ndarray()).ndarray();
        let pos = line.0[0];
        plotter.text(
            &pin.number.name,
            pos,
            FontEffects {
                angle: 0.0,
                anchor: String::from("middle"),
                baseline: String::from("middle"),
                face: String::from("osifont"),
                size: 1.25,
                color: Color::black(),
            },
        );
    }

    if pin_names && pin.name.name != "~" && !power {
        let Some(offset) = pin_names_offset else {
            return;
        };
        let (to, align) = match pin.pos.angle {
            0.0 => (Pt { x: offset, y: 0.0 }, String::from("left")),
            90.0 => (Pt { x: 0.0, y: offset }, String::from("left")),
            180.0 => (Pt { x: -offset, y: 0.0 }, String::from("right")),
            270.0 => (Pt { x: 0.0, y: offset }, String::from("right")),
            _ => {
                panic!("pin angle: {}, not supported", pin.pos.angle);
            }
        };
        let translate = Transform::new().translation(Pt { x: to.x, y: to.y });
        let line: Pts = translate.transform(&pts.ndarray()).ndarray();
        plotter.text(
            &pin.name.name,
            line.0[1],
            FontEffects {
                angle: 0.0,
                anchor: align,
                baseline: String::from("middle"),
                face: String::from("osifont"),
                size: 1.75,
                color: Color::red(),
            },
        );
    }
}
