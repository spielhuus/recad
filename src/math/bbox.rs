use ndarray::{arr2, Array, Array2};

use crate::{
    gr::{Effects, Justify, Pos, Property, Pt, Rect},
    schema::{GlobalLabel, Junction, LocalLabel, NoConnect, Symbol, Wire},
    sexp::constants::el,
    Error, Schema,
};

use super::{pin_position, ToNdarray, Transform};

///calculates the outline of a list of points.
pub fn calculate(pts: Array2<f32>) -> Rect {
    let axis1 = pts.slice(ndarray::s![.., 0]);
    let axis2 = pts.slice(ndarray::s![.., 1]);
    Rect {
        start: Pt {
            x: *axis1
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
            y: *axis2
                .iter()
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
        },
        end: Pt {
            x: *axis1
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
            y: *axis2
                .iter()
                .max_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap(),
        },
    }
}

fn text(text: &str, pos: &Pos, effects: &Effects) -> Result<Rect, Error> {
    let mut dim = super::fonts::dimension(text, effects)?;
    //TODO this is not nice.
    let start = if pos.angle == 0.0 {
        Pt {
            x: if effects.justify.contains(&Justify::Right) {
                pos.x - dim[[0, 0]]
            } else if effects.justify.contains(&Justify::Left) {
                pos.x
            } else {
                pos.x - dim[[0, 0]] / 2.0
            },
            y: if effects.justify.contains(&Justify::Top) {
                pos.y
            } else if effects.justify.contains(&Justify::Bottom) {
                pos.y - dim[[0, 1]]
            } else {
                pos.y - dim[[0, 1]] / 2.0
            },
        }
    } else if pos.angle == 90.0 {
        //let transform = Transform::new().rotation(pos.angle);
        //dim = transform.transform1(&dim);
        Pt {
            x: if effects.justify.contains(&Justify::Right) {
                pos.x
            } else if effects.justify.contains(&Justify::Left) {
                pos.x - dim[[0, 0]]
            } else {
                pos.x - dim[[0, 0]] / 2.0
            },
            y: if effects.justify.contains(&Justify::Top) {
                pos.y
            } else if effects.justify.contains(&Justify::Bottom) {
                pos.y - dim[[0, 1]]
            } else {
                pos.y - dim[[0, 1]] / 2.0
            },
        }
    } else if pos.angle == 180.0 {
        let transform = Transform::new().rotation(pos.angle);
        dim = transform.transform(&dim);

        Pt {
            x: if effects.justify.contains(&Justify::Right) {
                pos.x
            } else if effects.justify.contains(&Justify::Left) {
                pos.x - dim[[0, 0]]
            } else {
                pos.x - dim[[0, 0]] / 2.0
            },
            y: if effects.justify.contains(&Justify::Top) {
                pos.y + dim[[0, 1]]
            } else if effects.justify.contains(&Justify::Bottom) {
                pos.y
            } else {
                pos.y - dim[[0, 1]] / 2.0
            },
        }
    } else if pos.angle == 270.0 {
        Pt {
            x: if effects.justify.contains(&Justify::Right) {
                pos.x
            } else if effects.justify.contains(&Justify::Left) {
                pos.x - dim[[0, 0]]
            } else {
                pos.x - dim[[0, 0]] / 2.0
            },
            y: if effects.justify.contains(&Justify::Top) {
                pos.y
            } else if effects.justify.contains(&Justify::Bottom) {
                pos.y - dim[[0, 1]]
            } else {
                pos.y - dim[[0, 1]] / 2.0
            },
        }
    } else {
        panic!("unsupported angle {}", pos.angle);
    };

    if dim[[0, 0]] < 0.0 || dim[[0, 1]] < 0.0 {
        Ok(Rect {
            start: Pt {
                x: start.x - dim[[0, 0]].abs(),
                y: start.y - dim[[0, 1]].abs(),
            },
            end: start,
        })
    } else {
        Ok(Rect {
            start,
            end: Pt {
                x: start.x + dim[[0, 0]].abs(),
                y: start.y + dim[[0, 1]].abs(),
            },
        })
    }
}

pub trait Bbox {
    fn outline(&self, schema: &Schema) -> Result<Rect, Error>;
}
impl Bbox for Junction {
    fn outline(&self, _: &Schema) -> Result<Rect, Error> {
        let d = if self.diameter == 0.0 {
            el::JUNCTION_DIAMETER / 2.0
        } else {
            self.diameter / 2.0
        };
        Ok(Rect {
            start: Pt {
                x: self.pos.x - d,
                y: self.pos.y - d,
            },
            end: Pt {
                x: self.pos.x + d,
                y: self.pos.y + d,
            },
        })
    }
}

impl Bbox for NoConnect {
    fn outline(&self, _: &Schema) -> Result<Rect, Error> {
        Ok(Rect {
            start: Pt {
                x: self.pos.x - el::NO_CONNECT_SIZE,
                y: self.pos.y - el::NO_CONNECT_SIZE,
            },
            end: Pt {
                x: self.pos.x + el::NO_CONNECT_SIZE,
                y: self.pos.y + el::NO_CONNECT_SIZE,
            },
        })
    }
}

impl Bbox for LocalLabel {
    fn outline(&self, _: &Schema) -> Result<Rect, Error> {
        text(&self.text, &self.pos, &self.effects)
    }
}

impl Bbox for GlobalLabel {
    fn outline(&self, _: &Schema) -> Result<Rect, Error> {
        text(&self.text, &self.pos, &self.effects)
    }
}

impl Bbox for Wire {
    fn outline(&self, _: &Schema) -> Result<Rect, Error> {
        Ok(Rect {
            start: self.pts.0[0],
            end: self.pts.0[1],
        })
    }
}

impl Bbox for Symbol {
    fn outline(&self, schema: &Schema) -> Result<Rect, Error> {
        let lib_symbol = schema.library_symbol(&self.lib_id).unwrap();
        let transform = Transform::new()
            .translation(Pt {
                x: self.pos.x,
                y: self.pos.y,
            })
            .rotation(self.pos.angle)
            .mirror(&self.mirror);

        let mut pts = Array::zeros((0, 2));
        for s in &lib_symbol.units {
            for g in &s.graphics {
                match g {
                    crate::gr::GraphicItem::Arc(_) => {} //TODO
                    crate::gr::GraphicItem::Circle(circle) => {
                        pts.push_row(
                            transform
                                .transform(&arr2(&[[
                                    circle.center.x - circle.radius,
                                    circle.center.y - circle.radius,
                                ]]))
                                .row(0),
                        )
                        .expect("insertion failed");
                        pts.push_row(
                            transform
                                .transform(&arr2(&[[
                                    circle.center.x + circle.radius,
                                    circle.center.y + circle.radius,
                                ]]))
                                .row(0),
                        )
                        .expect("insertion failed");
                    }
                    crate::gr::GraphicItem::Curve(_) => {} //TODO
                    crate::gr::GraphicItem::Line(line) => {
                        line.pts.0.iter().for_each(|p| {
                            pts.push_row(transform.transform(&p.ndarray()).row(0))
                                .expect("insertion failed");
                        });
                    }
                    crate::gr::GraphicItem::Polyline(poly) => {
                        poly.pts.0.iter().for_each(|p| {
                            pts.push_row(transform.transform(&p.ndarray()).row(0))
                                .expect("insertion failed");
                        });
                    }
                    crate::gr::GraphicItem::Rectangle(rect) => {
                        pts.push_row(transform.transform(&rect.start.ndarray()).row(0))
                            .expect("insertion failed");
                        pts.push_row(transform.transform(&rect.end.ndarray()).row(0))
                            .expect("insertion failed");
                    }
                    crate::gr::GraphicItem::Text(_) => {} //TODO
                }
            }
        }
        for p in &lib_symbol.pins(self.unit) {
            pts.push_row(pin_position(self, p).ndarray().row(0))
                .expect("insertion failed");
        }
        for prop in &self.props {
            if prop.visible() {}
        }

        //calculate the bounds
        Ok(calculate(pts))
    }
}

impl Bbox for Property {
    fn outline(&self, _: &Schema) -> Result<Rect, Error> {
        text(&self.value, &self.pos, &self.effects)
    }
}
