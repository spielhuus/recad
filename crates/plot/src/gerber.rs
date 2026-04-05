use std::{
    collections::HashMap,
    io::Write,
};

use models::transform::Transform;
use types::gr::{Effects, FontAnchor, FontBaseline, Pos, Pt, Pts, Rect};

use font::OSIFONT;
use crate::{
    Paint, Plotter,
};

// use crate::plot::{FontAnchor, FontBaseline};

use ttf_parser::{Face, OutlineBuilder};

struct FontOutlineBuilder {
    paths: Vec<Vec<Pt>>,
    current_path: Vec<Pt>,
    current_pt: Pt,
    transform: Transform,
    scale_x: f64,
    scale_y: f64,
    offset_x: f64,
    offset_y: f64,
}

impl FontOutlineBuilder {
    fn tx(&self, x: f32, y: f32) -> Pt {
        let local_pt = Pt {
            x: self.offset_x + x as f64 * self.scale_x,
            y: self.offset_y - y as f64 * self.scale_y, // TrueType Y goes up; Canvas goes down
        };
        self.transform.transform_point(local_pt)
    }
}

impl OutlineBuilder for FontOutlineBuilder {
    fn move_to(&mut self, x: f32, y: f32) {
        if !self.current_path.is_empty() {
            self.paths.push(std::mem::take(&mut self.current_path));
        }
        let pt = self.tx(x, y);
        self.current_pt = pt;
        self.current_path.push(pt);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let pt = self.tx(x, y);
        self.current_pt = pt;
        self.current_path.push(pt);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let steps = 10;
        let p0 = self.current_pt;
        let p1 = self.tx(x1, y1);
        let p2 = self.tx(x, y);

        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let mt = 1.0 - t;
            let px = mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x;
            let py = mt * mt * p0.y + 2.0 * mt * t * p1.y + t * t * p2.y;
            self.current_path.push(Pt { x: px, y: py });
        }
        self.current_pt = p2;
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let steps = 15;
        let p0 = self.current_pt;
        let p1 = self.tx(x1, y1);
        let p2 = self.tx(x2, y2);
        let p3 = self.tx(x, y);

        for i in 1..=steps {
            let t = i as f64 / steps as f64;
            let mt = 1.0 - t;
            let px = mt * mt * mt * p0.x
                + 3.0 * mt * mt * t * p1.x
                + 3.0 * mt * t * t * p2.x
                + t * t * t * p3.x;
            let py = mt * mt * mt * p0.y
                + 3.0 * mt * mt * t * p1.y
                + 3.0 * mt * t * t * p2.y
                + t * t * t * p3.y;
            self.current_path.push(Pt { x: px, y: py });
        }
        self.current_pt = p3;
    }

    fn close(&mut self) {
        if let Some(first) = self.current_path.first().copied() {
            self.current_path.push(first);
        }
        self.paths.push(std::mem::take(&mut self.current_path));
    }
}

enum PathOp {
    Move(Pt),
    Line(Pt),
    Close,
}


fn get_bbox(path: &[Pt]) -> (f64, f64, f64, f64) {
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for pt in path {
        if pt.x < min_x { min_x = pt.x; }
        if pt.y < min_y { min_y = pt.y; }
        if pt.x > max_x { max_x = pt.x; }
        if pt.y > max_y { max_y = pt.y; }
    }
    (min_x, min_y, max_x, max_y)
}

/// Advanced PIP with Bounding Box fast-rejection and vertex-glitch prevention.
fn is_path_inside(path_a: &[Pt], path_b: &[Pt]) -> bool {
    let (min_xa, min_ya, max_xa, max_ya) = get_bbox(path_a);
    let (min_xb, min_yb, max_xb, max_yb) = get_bbox(path_b);
    
    // Quick reject: A cannot be inside B if its bounding box isn't inside B's
    let eps = 1e-4;
    if min_xa < min_xb - eps || min_ya < min_yb - eps ||
       max_xa > max_xb + eps || max_ya > max_yb + eps {
        return false; 
    }

    let mut inside_count = 0;
    let mut outside_count = 0;

    for &pt in path_a.iter() {
        let mut inside = false;
        let n = path_b.len();
        let mut j = n - 1;
        
        // Add a microscopic jitter to the ray to prevent perfect collinear vertex hits!
        let test_y = pt.y + 1e-5; 
        
        for i in 0..n {
            let pi = path_b[i];
            let pj = path_b[j];
            
            if ((pi.y > test_y) != (pj.y > test_y)) &&
               (pt.x < (pj.x - pi.x) * (test_y - pi.y) / (pj.y - pi.y) + pi.x) {
                inside = !inside;
            }
            j = i;
        }
        if inside { inside_count += 1; } 
        else { outside_count += 1; }
    }
    
    // Path is inside if the majority of its points evaluate to inside
    inside_count > outside_count
}

/// Computes the signed area exactly as the Gerber viewer will interpret it (Y is inverted).
fn gerber_area(path: &[Pt]) -> f64 {
    let mut area = 0.0;
    let n = path.len();
    if n < 3 { return 0.0; }
    for i in 0..n {
        let j = (i + 1) % n;
        // Invert Y to match the Gerber output format
        let yi = -path[i].y;
        let yj = -path[j].y;
        area += path[i].x * yj - path[j].x * yi;
    }
    area / 2.0
}

pub struct GerberPlotter {
    buffer: Vec<u8>,
    path: Vec<PathOp>,
    apertures: HashMap<u32, u32>, // Width in nanometers (for exact hashing) -> D-code
    next_d_code: u32,
    current_aperture: Option<u32>,
    start_pt: Option<Pt>,
}

impl GerberPlotter {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let mut buffer = Vec::new();
        // Standard RS-274X Headers
        writeln!(&mut buffer, "G04 Gerber RS-274X export by Recad*").unwrap();
        writeln!(&mut buffer, "%FSLAX46Y46*%").unwrap(); // Format Statement: Absolute, 4.6 format
        writeln!(&mut buffer, "%MOMM*%").unwrap(); // Millimeters
        writeln!(&mut buffer, "%LPD*%").unwrap(); // Layer Polarity Dark

        Self {
            buffer,
            path: Vec::new(),
            apertures: HashMap::new(),
            next_d_code: 10, // D-codes 0-9 are reserved by the Gerber spec
            current_aperture: None,
            start_pt: None,
        }
    }

    // Format float coordinates to 4.6 integer format (e.g., 1.234567 -> 1234567)
    fn fmt_coord(val: f64) -> String {
        let v = (val * 1_000_000.0).round() as i64;
        let sign = if v < 0 { "-" } else { "" };
        format!("{}{:010}", sign, v.abs())
    }

    // Select or create a circular aperture (used for lines and round flashes)
    fn use_aperture(&mut self, width: f64) -> std::io::Result<()> {
        let width_nm = (width * 1_000_000.0).round() as u32;

        let d_code = if let Some(&code) = self.apertures.get(&width_nm) {
            code
        } else {
            let code = self.next_d_code;
            self.next_d_code += 1;
            self.apertures.insert(width_nm, code);
            // Define Circular Aperture (C)
            writeln!(&mut self.buffer, "%ADD{}C,{:.6}*%", code, width)?;
            code
        };

        if self.current_aperture != Some(d_code) {
            writeln!(&mut self.buffer, "D{}*", d_code)?;
            self.current_aperture = Some(d_code);
        }

        Ok(())
    }

}

impl Plotter for GerberPlotter {
    fn open(&self) {}

    fn set_view_box(&mut self, _rect: Rect) {}

    fn scale(&mut self, _scale: f64) {}

    fn move_to(&mut self, pt: Pt) {
        self.start_pt = Some(pt);
        self.path.push(PathOp::Move(pt));
    }

    fn line_to(&mut self, pt: Pt) {
        self.path.push(PathOp::Line(pt));
    }

    fn close(&mut self) {
        self.path.push(PathOp::Close);
    }

    fn stroke(&mut self, stroke: Paint) {
        let width = stroke.width.max(0.001); // Prevent 0-width which is invalid in standard Gerber
        self.use_aperture(width).unwrap();

        for op in std::mem::take(&mut self.path) {
            match op {
                PathOp::Move(pt) => {
                    // Y is inverted (-pt.y) to match the standard Gerber Cartesian system
                    writeln!(
                        &mut self.buffer,
                        "X{}Y{}D02*",
                        Self::fmt_coord(pt.x),
                        Self::fmt_coord(-pt.y)
                    )
                    .unwrap();
                }
                PathOp::Line(pt) => {
                    writeln!(
                        &mut self.buffer,
                        "X{}Y{}D01*",
                        Self::fmt_coord(pt.x),
                        Self::fmt_coord(-pt.y)
                    )
                    .unwrap();
                }
                PathOp::Close => {
                    if let Some(start) = self.start_pt {
                        writeln!(
                            &mut self.buffer,
                            "X{}Y{}D01*",
                            Self::fmt_coord(start.x),
                            Self::fmt_coord(-start.y)
                        )
                        .unwrap();
                    }
                }
            }
        }
        self.start_pt = None;
    }

    fn rect(&mut self, r: Rect, stroke: Paint) {
        let pts = Pts(vec![
            r.start,
            Pt {
                x: r.end.x,
                y: r.start.y,
            },
            r.end,
            Pt {
                x: r.start.x,
                y: r.end.y,
            },
            r.start, // Explicitly close the rect
        ]);
        self.polyline(pts, stroke);
    }

    fn arc(&mut self, _start: Pt, _mid: Pt, _end: Pt, _stroke: Paint) {
        // TODO: Map Recad 3-point arcs to Gerber I/J Center-relative coordinates
        spdlog::warn!("Arcs in Gerber natively are partially implemented. Falling back.");
    }

    fn circle(&mut self, center: Pt, radius: f64, stroke: Paint) {
        if stroke.fill.is_some() {
            // It's a filled circle (likely a via/pad). The most efficient way in Gerber is to "flash" it.
            let diameter = radius * 2.0;
            self.use_aperture(diameter).unwrap();
            writeln!(
                &mut self.buffer,
                "X{}Y{}D03*",
                Self::fmt_coord(center.x),
                Self::fmt_coord(-center.y)
            )
            .unwrap();
        } else {
            // Unfilled circle outline using G02 (CW Circular Interpolation)
            self.use_aperture(stroke.width).unwrap();

            let start_x = center.x + radius;
            let start_y = center.y;

            writeln!(
                &mut self.buffer,
                "X{}Y{}D02*",
                Self::fmt_coord(start_x),
                Self::fmt_coord(-start_y)
            )
            .unwrap();

            writeln!(&mut self.buffer, "G75*").unwrap(); // Multi-quadrant circular mode
            writeln!(
                &mut self.buffer,
                "G02X{}Y{}I{}J{}D01*",
                Self::fmt_coord(start_x),
                Self::fmt_coord(-start_y),
                Self::fmt_coord(-radius),
                Self::fmt_coord(0.0)
            )
            .unwrap();
            writeln!(&mut self.buffer, "G01*").unwrap(); // Reset back to linear interpolation mode
        }
    }

    fn text(&mut self, text: &str, pos: Pos, effects: Effects) {
        if text.is_empty() || effects.hide {
            return;
        }

        let face = match Face::parse(OSIFONT, 0) {
            Ok(f) => f,
            Err(_) => {
                spdlog::warn!("Failed to parse font bytes");
                return;
            }
        };

        // 1. Calculate scales
        let units_per_em = face.units_per_em() as f64;
        let scale_x = effects.font.size.0 as f64 / units_per_em;
        let scale_y = effects.font.size.1 as f64 / units_per_em;

        // 2. Measure text width
        let mut total_width = 0.0;
        for c in text.chars() {
            if let Some(glyph_id) = face.glyph_index(c) {
                let adv = face.glyph_hor_advance(glyph_id).unwrap_or(0);
                total_width += adv as f64 * scale_x;
            }
        }

        // 3. Justification Setup
        let ascender = face.ascender() as f64;
        let descender = face.descender() as f64; // Typically negative

        let mut offset_x = match effects.anchor() {
            FontAnchor::Start => 0.0,
            FontAnchor::Middle => -total_width / 2.0,
            FontAnchor::End => -total_width,
        };

        let offset_y = match effects.baseline() {
            FontBaseline::Auto => descender * scale_y, // Bottom justified
            FontBaseline::Middle => (ascender + descender) / 2.0 * scale_y, // Center
            FontBaseline::Hanging => ascender * scale_y, // Top justified
        };

        // 4. Transform coordinates
        let transform = Transform::new()
            .translation(Pt { x: pos.x, y: pos.y })
            .rotation(-pos.angle);

        let mut builder = FontOutlineBuilder {
            paths: Vec::new(),
            current_path: Vec::new(),
            current_pt: Pt { x: 0.0, y: 0.0 },
            transform,
            scale_x,
            scale_y,
            offset_x: 0.0, // Updated per-character
            offset_y,
        };

        // 5. Trace out the curves for each character
        for c in text.chars() {
            builder.offset_x = offset_x;
            
            builder.paths.clear();
            builder.current_path.clear();

            if let Some(glyph_id) = face.glyph_index(c) {
                face.outline_glyph(glyph_id, &mut builder);
                
                // Cleanup open path
                if !builder.current_path.is_empty() {
                    if let Some(first) = builder.current_path.first().copied() {
                        builder.current_path.push(first); // Explicitly close path
                    }
                    builder.paths.push(std::mem::take(&mut builder.current_path));
                }

                if !builder.paths.is_empty() {
                    let mut depths = vec![0; builder.paths.len()];

                    // Calculate Geometric Inclusion (Depth) using robust PIP
                    #[allow(clippy::needless_range_loop)]
                    for i in 0..builder.paths.len() {
                        for j in 0..builder.paths.len() {
                            if i == j { continue; }
                            if is_path_inside(&builder.paths[i], &builder.paths[j]) {
                                depths[i] += 1;
                            }
                        }
                    }

                    let mut paths_with_depths: Vec<_> = builder.paths.drain(..).zip(depths).collect();
                    paths_with_depths.sort_by_key(|(_, depth)| *depth);

                    // Start a single Gerber Region for the character
                    writeln!(&mut self.buffer, "G36*").unwrap();

                    for (mut path, depth) in paths_with_depths {
                        if path.len() < 3 { continue; }

                        // Calculate area exactly as it will appear in the Gerber file
                        let area = gerber_area(&path);
                        let is_solid = depth % 2 == 0;

                        // Force standard winding for KiCad/GerbView:
                        // CW (Negative Area) = Add/Solid
                        // CCW (Positive Area) = Subtract/Hole
                        let reverse_needed = if is_solid {
                            area > 0.0 
                        } else {
                            area < 0.0 
                        };

                        if reverse_needed {
                            path.reverse();
                        }

                        for (idx, pt) in path.iter().enumerate() {
                            let d = if idx == 0 { "D02" } else { "D01" };
                            writeln!(
                                &mut self.buffer,
                                "X{}Y{}{}*",
                                Self::fmt_coord(pt.x),
                                Self::fmt_coord(-pt.y),
                                d
                            ).unwrap();
                        }
                    }

                    writeln!(&mut self.buffer, "G37*").unwrap(); // End Region
                }
                
                let adv = face.glyph_hor_advance(glyph_id).unwrap_or(0);
                offset_x += adv as f64 * scale_x;
            }
        }
    }

    fn polyline(&mut self, pts: Pts, stroke: Paint) {
        if stroke.fill.is_some() {
            // Fill polygon using Gerber Regions (G36/G37)
            writeln!(&mut self.buffer, "G36*").unwrap(); // Start Region
            for (i, pt) in pts.0.iter().enumerate() {
                let d = if i == 0 { "D02" } else { "D01" };
                writeln!(
                    &mut self.buffer,
                    "X{}Y{}{}*",
                    Self::fmt_coord(pt.x),
                    Self::fmt_coord(-pt.y),
                    d
                )
                .unwrap();
            }
            writeln!(&mut self.buffer, "G37*").unwrap(); // End Region
        } else {
            // Unfilled outline polyline
            if pts.0.is_empty() {
                return;
            }
            self.move_to(pts.0[0]);
            for pt in &pts.0[1..] {
                self.line_to(*pt);
            }
            self.stroke(stroke);
        }
    }

    fn write<W: Write>(mut self, writer: &mut W) -> std::io::Result<(u32, u32)> {
        writeln!(&mut self.buffer, "M02*")?; // Finalize with End Of File command
        writer.write_all(&self.buffer)?;
        Ok((0, 0)) // Gerber doesn't have an explicit size return like SVG
    }

    fn save(self, path: &std::path::Path) -> std::io::Result<()> {
        let mut file = std::fs::File::create(path)?;
        self.write(&mut file)?;
        Ok(())
    }
}
