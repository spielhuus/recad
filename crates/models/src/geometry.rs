use types::{
    constants::el, error::RecadError, gr::{Effects, Justify, Pos, Pt, Rect}
};
use crate::{schema::{GlobalLabel, Junction, LocalLabel, NoConnect, Property, Symbol, Text, Wire}, symbols::Pin, transform::Transform};

///Calculate the position of a pin in a symbol.
pub fn pin_position(symbol: &Symbol, pin: &Pin) -> Pt {
    // Original pin pos
    let p = pin.pos;
    let pt = Pt { x: p.x, y: p.y };

    // Transform order must match schema_plotter: Translation -> Rotation -> Mirror (Scale)
    let transform = Transform::new()
        .translation(Pt {
            x: symbol.pos.x,
            y: symbol.pos.y,
        })
        .mirror(&symbol.mirror)
        .rotation(symbol.pos.angle);

    transform.transform_point(pt)
}

/// Calculates the outline of a list of points.
pub fn calculate(pts: &[Pt]) -> Rect {
    if pts.is_empty() {
        return Rect::default();
    }

    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;

    for p in pts {
        min_x = min_x.min(p.x);
        min_y = min_y.min(p.y);
        max_x = max_x.max(p.x);
        max_y = max_y.max(p.y);
    }

    Rect {
        start: Pt { x: min_x, y: min_y },
        end: Pt { x: max_x, y: max_y },
    }
}

fn text(text: &str, pos: &Pos, effects: &Effects) -> Result<Rect, RecadError> {
    // Calculate text dimensions
    let w = match font::dimension(text, effects) {
        Ok(dim) => dim.x,
        Err(_) => 0.0,
    };
    let h = effects.font.size.1 as f64;

    // Find the local anchor coordinate (ax, ay) relative to top-left (0,0)
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

    let pts_local =[
        Pt { x: -ax, y: -ay },         // Top-Left
        Pt { x: w - ax, y: -ay },      // Top-Right
        Pt { x: w - ax, y: h - ay },   // Bottom-Right
        Pt { x: -ax, y: h - ay },      // Bottom-Left
    ];

    let angle_rad = (-pos.angle).to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();

    let mut pts_global = Vec::with_capacity(4);
    for p in &pts_local {
        let rx = p.x * cos_a - p.y * sin_a;
        let ry = p.x * sin_a + p.y * cos_a;
        
        pts_global.push(Pt {
            x: pos.x + rx,
            y: pos.y + ry,
        });
    }

    Ok(calculate(&pts_global))
}

pub trait Bbox {
    fn outline(&self) -> Result<Rect, RecadError>;
}

impl Bbox for Junction {
    fn outline(&self) -> Result<Rect, RecadError> {
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
    fn outline(&self) -> Result<Rect, RecadError> {
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
    fn outline(&self) -> Result<Rect, RecadError> {
        text(&self.text, &self.pos, &self.effects)
    }
}

impl Bbox for GlobalLabel {
    fn outline(&self) -> Result<Rect, RecadError> {
        text(&self.text, &self.pos, &self.effects)
    }
}

impl Bbox for Text {
    fn outline(&self) -> Result<Rect, RecadError> {
        text(&self.text, &self.pos, &self.effects)
    }
}

impl Bbox for Wire {
    fn outline(&self) -> Result<Rect, RecadError> {
        Ok(calculate(&self.pts.0))
    }
}

impl Bbox for Property {
    fn outline(&self) -> Result<Rect, RecadError> {
        text(&self.value, &self.pos, &self.effects)
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{schema::SchemaItem, schema::Schema, library::SymbolLibrary};

    #[test]
    fn test_bbox_symbol_1() {
        let lib = SymbolLibrary {
            pathlist: vec![PathBuf::from("/usr/share/kicad/symbols")],
        };
        let mut schema = Schema::new("test_bbox", None);
        let lib_sym = lib.load("Amplifier_Operational:LM2904").unwrap();
        let sym = lib_sym.symbol(1);
        schema.library_symbols.push(lib_sym.clone());
        schema.items.push(SchemaItem::Symbol(sym.clone()));
        assert_eq!("Amplifier_Operational:LM2904", sym.lib_id);

        let bbox = sym.outline(&lib_sym).unwrap();
        // Values rounded for robust float comparison if necessary, but here we use exact match from previous test expectations
        assert_eq!(-7.62, bbox.start.x);
        assert_eq!(-5.08, bbox.start.y);
        assert_eq!(7.62, bbox.end.x);
        assert_eq!(5.08, bbox.end.y);
    }
    #[test]
    fn test_bbox_symbol_3() {
        let lib = SymbolLibrary {
            pathlist: vec![PathBuf::from("/usr/share/kicad/symbols")],
        };
        let mut schema = Schema::new("test_bbox", None);
        let lib_sym = lib.load("Amplifier_Operational:LM2904").unwrap();
        let sym = lib_sym.symbol(3);
        schema.library_symbols.push(lib_sym.clone());
        schema.items.push(SchemaItem::Symbol(sym.clone()));
        assert_eq!("Amplifier_Operational:LM2904", sym.lib_id);

        let bbox = sym.outline(&lib_sym).unwrap();
        assert_eq!(-2.54, (bbox.start.x * 100.0).round() / 100.0);
        assert_eq!(-7.62, (bbox.start.y * 100.0).round() / 100.0);
        assert_eq!(-2.54, (bbox.end.x * 100.0).round() / 100.0);
        assert_eq!(7.62, (bbox.end.y * 100.0).round() / 100.0);
    }
}
