use base64::{engine::general_purpose, Engine as _};
use font::OSIFONT;
use std::{collections::HashSet, fs::File, io::Write};

use svg::{
    node::element::{path::Data, Circle, Path, Rectangle, Style, Text},
    write as svgwrite, Document, Node,
};

use types::{gr::{Effects, Justify, Pos, Pt, Pts, Rect}, constants::el};

use super::{Paint, Plotter};

///Plot a schema/pcb to a svg file.
pub struct SvgPlotter {
    viewbox: Option<Rect>,
    scale: f64,
    paths: Document,
    data: Data,
    embedded_fonts: HashSet<String>,
}

pub fn anchor(effects: &Effects) -> String {
    if effects.justify.contains(&Justify::Right) {
        String::from("end")
    } else if effects.justify.contains(&Justify::Left) {
        String::from("start")
    } else {
        String::from("center")
    }
}

pub fn baseline(effects: &Effects) -> String {
    if effects.justify.contains(&Justify::Bottom) {
        String::from("auto")
    } else if effects.justify.contains(&Justify::Top) {
        String::from("hanging")
    } else {
        String::from("auto")
    }
}

#[allow(clippy::new_without_default)]
impl SvgPlotter {
    pub fn new() -> Self {
        SvgPlotter {
            viewbox: None,
            scale: 1.0,
            paths: Document::new(),
            data: Data::new(),
            embedded_fonts: HashSet::new(),
        }
    }

    fn ensure_font_embedded(&mut self, face: &str) {
        if self.embedded_fonts.contains(face) {
            return; // Already embedded
        }

        let font_bytes = OSIFONT; //TODO: also load other fonts
        let b64 = general_purpose::STANDARD.encode(font_bytes);
        let css = format!(
            "@font-face {{\n  font-family: '{}';\n  src: url('data:font/ttf;base64,{}') format('truetype');\n}}",
            face, b64
        );

        self.paths = self.paths.clone().add(Style::new(css));
        self.embedded_fonts.insert(face.to_string());
    }
}

impl Plotter for SvgPlotter {
    fn open(&self) {
        panic!("open not supported for SvgPlotter")
    }

    fn save(self, path: &std::path::Path) -> std::io::Result<()> {
        let mut buffer: Vec<u8> = Vec::new();
        self.write(&mut buffer)?;
        let mut file = File::create(path)?;
        file.write_all(buffer.as_slice())
    }

    fn write<W: Write>(mut self, writer: &mut W) -> std::io::Result<(u32, u32)> {
        // Use self.paths directly
        if let Some(viewbox) = &self.viewbox {
            self.paths = self.paths.set(
                "width",
                format!(
                    "{}mm",
                    types::round(viewbox.end.x * self.scale)
                ),
            );
            self.paths = self.paths.set(
                "height",
                format!(
                    "{}mm",
                    types::round(viewbox.end.y * self.scale)
                ),
            );
            self.paths = self.paths.set(
                "viewBox",
                (
                    viewbox.start.x,
                    viewbox.start.y,
                    viewbox.end.x,
                    viewbox.end.y,
                ),
            );
        }
        svgwrite(writer, &self.paths).unwrap();
        Ok((0, 0)) // TODO
    }

    fn set_view_box(&mut self, rect: Rect) {
        self.viewbox = Some(rect)
    }

    fn scale(&mut self, scale: f64) {
        self.scale = scale;
    }

    fn move_to(&mut self, pt: Pt) {
        let data = self.data.clone().move_to((pt.x, pt.y));
        self.data = data;
    }

    fn line_to(&mut self, pt: Pt) {
        let data = self.data.clone().line_to((pt.x, pt.y));
        self.data = data;
    }

    fn close(&mut self) {
        let data = self.data.clone().close();
        self.data = data;
    }

    fn stroke(&mut self, stroke: Paint) {
        let mut path = Path::new()
            .set("stroke", stroke.color.to_hex())
            .set("stroke-opacity", stroke.color.alpha())
            .set("stroke-width", stroke.width)
            .set("stroke-linecap", stroke.linecap.to_string())
            .set("d", self.data.clone());

        if let Some(fill_color) = stroke.fill {
            path = path.set("fill", fill_color.to_hex());
            path = path.set("fill-opacity", fill_color.alpha());
        } else {
            path = path.set("fill", el::NONE);
        }

        self.paths.append(path);
        self.data = Data::new();
    }

    fn rect(&mut self, rect: Rect, stroke: Paint) {
        self.paths.append(
            Rectangle::new()
                .set("x", format!("{:.2}", rect.start.x))
                .set("y", format!("{:.2}", rect.start.y))
                .set("width", format!("{:.2}", rect.end.x))
                .set("height", format!("{:.2}", rect.end.y))
                .set(
                    "fill",
                    if stroke.fill.is_some() {
                        stroke.fill.unwrap().to_hex()
                    } else {
                        el::NONE.to_string()
                    },
                )
                .set("stroke", stroke.color.to_hex())
                .set("stroke-width", format!("{:.2}", stroke.width)),
        );
    }

    fn arc(&mut self, start: Pt, mid: Pt, end: Pt, stroke: Paint) {
        let (center, radius) = calculate_circle(start, mid, end).unwrap();
        let start_angle = angle(&center, &start);
        let end_angle = angle(&center, &end);
        let sweep_flag = sweep_flag(&start, &mid, &end);

        let large_arc_flag = if end_angle - start_angle > 180.0 {
            "1"
        } else {
            "0"
        };

        let c = Path::new()
            .set(
                "d",
                format!(
                    "M{:.2} {:.2} A{:.2} {:.2} 0.0 {} {} {:.2} {:.2}",
                    start.x, start.y, radius, radius, large_arc_flag, sweep_flag, end.x, end.y
                ),
            )
            .set("fill", el::NONE)
            .set("stroke", stroke.color.to_hex())
            .set("stroke-width", stroke.width);

        self.paths.append(c);
    }

    fn circle(&mut self, center: Pt, radius: f64, stroke: Paint) {
        self.paths.append(
            Circle::new()
                .set("cx", center.x)
                .set("cy", center.y)
                .set("r", radius)
                .set(
                    "fill",
                    match stroke.fill {
                        Some(color) => color.to_hex(),
                        None => el::NONE.to_string(),
                    },
                )
                .set("stroke", stroke.color.to_hex())
                .set("stroke-width", stroke.width),
        );
    }

    fn polyline(&mut self, pts: Pts, stroke: Paint) {
        let mut first: bool = true;
        for pos in pts.0 {
            if first {
                let data = self.data.clone().move_to((pos.x, pos.y));
                self.data = data;
                first = false;
            } else {
                let data = self.data.clone().line_to((pos.x, pos.y));
                self.data = data;
            }
        }

        self.stroke(stroke);
    }

    fn text(&mut self, text: &str, pos: Pos, effects: Effects) {
        let face = effects
            .font
            .face
            .clone()
            .unwrap_or_else(|| el::OSIFONT.to_string()); //TODO use font from effects
        self.ensure_font_embedded(&face);

        let mut t = Text::new(text)
            .set("text-anchor", anchor(&effects))
            .set("dominant-baseline", baseline(&effects))
            .set(
                "font-family",
                effects.font.face.unwrap_or(el::OSIFONT.to_string()),
            )
            .set("font-size", format!("{}pt", effects.font.size.0))
            .set("fill", effects.font.color.unwrap().to_hex());

        if pos.angle != 0.0 {
            t = t.set(
                "transform",
                format!("translate({},{}) rotate({})", pos.x, pos.y, pos.angle),
            );
        } else {
            t = t.set("transform", format!("translate({},{})", pos.x, pos.y));
        }
        self.paths.append(t);
    }
}

// Function to calculate angle between center and point
fn angle(center: &Pt, point: &Pt) -> f64 {
    (point.y - center.y).atan2(point.x - center.x)
}

// calculate the svg sweep flac from star, middle and end points.
pub fn sweep_flag(start: &Pt, mid: &Pt, end: &Pt) -> String {
    if (start.x - mid.x) * (end.y - mid.y) > (start.y - mid.y) * (end.x - mid.x) {
        0
    } else {
        1
    }
    .to_string()
}

fn calculate_circle(p1: Pt, p2: Pt, p3: Pt) -> Option<(Pt, f64)> {
    // Calculate the midpoints of p1-p2 and p2-p3
    let mid1 = Pt {
        x: (p1.x + p2.x) / 2.0,
        y: (p1.y + p2.y) / 2.0,
    };
    let mid2 = Pt {
        x: (p2.x + p3.x) / 2.0,
        y: (p2.y + p3.y) / 2.0,
    };

    // Slopes of the perpendicular bisectors
    let slope1 = -(p2.x - p1.x) / (p2.y - p1.y);
    let slope2 = -(p3.x - p2.x) / (p3.y - p2.y);

    // Check if the points are collinear
    if (p2.y - p1.y) * (p3.x - p2.x) == (p3.y - p2.y) * (p2.x - p1.x) {
        return None;
    }

    // Line equations in the form of y = mx + b
    let b1 = mid1.y - slope1 * mid1.x;
    let b2 = mid2.y - slope2 * mid2.x;

    // Solving for the intersection of the two lines
    let h = (b2 - b1) / (slope1 - slope2);
    let k = slope1 * h + b1;

    // Calculate the radius
    let radius = ((h - p1.x).powi(2) + (k - p1.y).powi(2)).sqrt();

    Some((Pt { x: h, y: k }, radius))
}
