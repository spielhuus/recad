use ndarray::{arr2, Array2, ArrayView, Axis};

pub mod bbox;
mod transform;
pub mod fonts;
pub use transform::Transform;

use crate::gr::Rect;
use crate::symbols::{LibrarySymbol, Pin};
use crate::{
    gr::{Pos, Pt, Pts},
    schema,
};

pub trait ToNdarray<T, O> {
    fn ndarray(&self) -> O;
}

impl ToNdarray<Pos, Array2<f32>> for Pos {
    fn ndarray(&self) -> Array2<f32> {
        arr2(&[[self.x, self.y]])
    }
}

impl ToNdarray<Pt, Array2<f32>> for Pt {
    fn ndarray(&self) -> Array2<f32> {
        arr2(&[[self.x, self.y]])
    }
}

impl ToNdarray<Array2<f32>, Pt> for Array2<f32> {
    fn ndarray(&self) -> Pt {
        Pt {
            x: self[[0, 0]],
            y: self[[0, 1]],
        }
    }
}

impl ToNdarray<Pts, Array2<f32>> for Pts {
    fn ndarray(&self) -> Array2<f32> {
        let mut pts: Array2<f32> = Array2::zeros((0, 2));
        for pt in &self.0 {
            pts.push_row(ArrayView::from(&[pt.x, pt.y])).unwrap()
        }
        pts
    }
}

impl ToNdarray<Array2<f32>, Pts> for Array2<f32> {
    fn ndarray(&self) -> Pts {
        let mut result = Vec::<Pt>::new();
        for p in self.axis_iter(Axis(0)) {
            result.push(Pt { x: p[0], y: p[1] });
        }
        Pts(result)
    }
}

impl ToNdarray<Pts, Array2<f32>> for Rect {
    fn ndarray(&self) -> Array2<f32> {
        arr2(&[[self.start.x, self.start.y], [self.end.x, self.end.y]])
    }
}

impl ToNdarray<Array2<f32>, Rect> for Array2<f32> {
    fn ndarray(&self) -> Rect {
        Rect {
            start: Pt {
                x: self[[0, 0]],
                y: self[[0, 1]],
            },
            end: Pt {
                x: self[[1, 0]],
                y: self[[1, 1]],
            },
        }
    }
}

///Calculate the position of a pin in a symbol.
pub fn pin_position(symbol: &schema::Symbol, pin: &Pin) -> Pt {
    let pos: Array2<f32> = pin.pos.ndarray();
    let transform = Transform::new()
        .mirror(&symbol.mirror)
        .translation(Pt {
            x: symbol.pos.x,
            y: symbol.pos.y,
        })
        .rotation(symbol.pos.angle);
    let pos = transform.transform(&pos);

    Pt {
        x: pos[[0, 0]],
        y: pos[[0, 1]],
    }
}

pub fn place_properties(lib: &LibrarySymbol, symbol: &mut schema::Symbol) {
    //let bbox = symbol.outline();

    let pins = lib.pins(symbol.unit);
    let pin_directions: Vec<f32> = pins
        .iter()
        .map(|p| {
            if p.pos.angle + symbol.pos.angle >= 360.0 {
                p.pos.angle + symbol.pos.angle - 360.0
            } else {
                p.pos.angle + symbol.pos.angle
            }
        })
        .collect();

    let vis_props = symbol.props.iter().filter(|p| p.visible()).count();

    if !pin_directions.contains(&90.0) {
        let mut start = symbol.pos.y - 2.5 - vis_props as f32 * 1.75;
        for prop in &mut symbol.props {
            if !prop.visible() {
                continue;
            }
            prop.pos.x = symbol.pos.x;
            prop.pos.y = start;
            prop.effects.justify.clear();
            start += 1.75;
        }
    }

    //let pos: Array1<f32> = property.pos.ndarray();
    //let transform = Transform::new()
    //    .translation(Pt {
    //        x: symbol.pos.x,
    //        y: symbol.pos.y,
    //    })
    //    .rotation(symbol.pos.angle);
    //let pos = transform.transform1(&pos);
    //Pt::default()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::{gr::Pt, Schema};

    #[test]
    fn pin_position() {
        let schema = Schema::load(Path::new("tests/opamp.kicad_sch")).unwrap();
        let symbol = schema.symbol("U1", 1).unwrap();
        let lib_symbol = schema.library_symbol(&symbol.lib_id).unwrap();
        let positions = lib_symbol
            .pins(1)
            .iter()
            .map(|p| super::pin_position(symbol, p))
            .collect::<Vec<Pt>>();
        let res = [
            Pt {
                x: 101.60,
                y: 80.01,
            },
            Pt {
                x: 86.36,
                y: 82.55,
            },
            Pt {
                x: 86.36,
                y: 77.47,
            },
        ];
        assert_eq!(res[0], *positions.first().unwrap());
        assert_eq!(res[1], *positions.get(1).unwrap());
        assert_eq!(res[2], *positions.get(2).unwrap());
    }
}
