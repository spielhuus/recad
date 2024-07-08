use std::io::Write;

use svg::{
    node::element::{path::Data, Circle, Path, Rectangle, Text},
    write as svgwrite, Document, Node,
};

use crate::gr::{Color, Pt, Pts, Rect};

use super::{FontEffects, Paint, Plotter};

///Plot a schema/pcb to a svg file.
pub struct SvgPlotter {
    viewbox: Option<Rect>,
    paths: Document,
    data: Data,
}

#[allow(clippy::new_without_default)]
impl SvgPlotter {
    pub fn new() -> Self {
        SvgPlotter {
            viewbox: None,
            paths: Document::new(),
            data: Data::new(),
        }
    }
}

impl Plotter for SvgPlotter {
    fn open(&self) {
        panic!("open not supported for SvgPlotter")
    }
    fn write<W: Write>(self, writer: &mut W) -> std::io::Result<()> {
        let mut document: Document = Document::new();
        if let Some(viewbox) = self.viewbox {
            document = document.set(
                "viewBox",
                (
                    viewbox.start.x,
                    viewbox.start.y,
                    viewbox.end.x,
                    viewbox.end.y,
                ),
            );
        }

        //for path in self.paths {
        //    document = document.add(path);
        //}

        document.append(self.paths);
        svgwrite(writer, &document).unwrap();
        Ok(())
    }

    fn set_view_box(&mut self, rect: Rect) {
        self.viewbox = Some(rect)
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
        self.paths.append(
            Path::new()
                .set(
                    "fill",
                    if stroke.fill.is_some() {
                        stroke.fill.unwrap().to_string()
                    } else {
                        "none".to_string()
                    },
                )
                .set("stroke", stroke.color.to_string())
                .set("stroke-width", stroke.width)
                .set("d", self.data.clone()),
        );
        self.data = Data::new();
    }

    fn rect(&mut self, rect: Rect, stroke: Paint) {
            self.paths.append(Rectangle::new()
                .set("x", format!("{:.2}", rect.start.x))
                .set("y", format!("{:.2}", rect.start.y))
                .set("width", format!("{:.2}", rect.end.x))
                .set("height", format!("{:.2}", rect.end.y))
                .set(
                    "fill",
                    if stroke.fill.is_some() {
                        stroke.fill.unwrap().to_string()
                    } else {
                        "none".to_string()
                    },
                )
                .set("stroke", stroke.color.to_string())
                .set("stroke-width", format!("{:.2}", stroke.width))
            );
    }

    fn arc(&mut self, center: Pt, radius: f32, stroke: Paint) {
        //TODO self.paths.append(
        //    Arc::new()
        //        .set("cx", center.x)
        //        .set("cy", center.y)
        //        .set("r", radius)
        //        .set("fill", stroke.fill.unwrap_or(Color::None).to_string())
        //        .set("stroke", stroke.color.to_string())
        //        .set("stroke-width", stroke.width),
        //);
    }

    fn circle(&mut self, center: Pt, radius: f32, stroke: Paint) {
        self.paths.append(
            Circle::new()
                .set("cx", center.x)
                .set("cy", center.y)
                .set("r", radius)
                .set("fill", stroke.fill.unwrap_or(Color::None).to_string())
                .set("stroke", stroke.color.to_string())
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


        //for (i, p) in pts.0.iter().enumerate() {
        //    if i == 0 {
        //        self.move_to(*p);
        //    } else {
        //        self.line_to(*p);
        //    }
        //}
        self.stroke(stroke);
    }

    fn text(&mut self, text: &str, pt: Pt, effects: FontEffects) {
        let mut t = Text::new(text)
            .set("text-anchor", effects.anchor)
            .set("dominant-baseline", effects.baseline)
            .set("font-family", effects.face)
            .set("font-size", format!("{}pt", effects.size))
            .set("fill", effects.color.to_string());

        if effects.angle != 0.0 {
            t = t.set("transform", format!("translate({},{}) rotate({})", pt.x, pt.y, effects.angle));
        } else {
            t = t.set("transform", format!("translate({},{})", pt.x, pt.y));
        }
        self.paths.append(t);
    }

}

#[cfg(test)]
mod tests {
    use ndarray::{arr2, Array2};
    use std::fs::File;

    use crate::{
        gr::{Color, Pt, Rect},
        math::{ToNdarray, Transform},
        plot::{Paint, Plotter},
    };

    #[test]
    fn draw() {
        let mut svg = super::SvgPlotter::new();
        svg.set_view_box(Rect{start: Pt { x: 0.0, y: 0.0 }, end: Pt { x: 100.0, y: -100.0 }});
        //svg.move_to(Pt { x: 100.0, y: 100.0 });
        //svg.line_to(Pt { x: 200.0, y: 100.0 });
        //svg.line_to(Pt { x: 200.0, y: 200.0 });
        //svg.line_to(Pt { x: 100.0, y: 200.0 });
        //svg.line_to(Pt { x: 100.0, y: 100.0 });
        //svg.stroke(Paint::black());
        //svg.circle(Pt { x: 110.0, y: 110.0 }, 7.5, Paint::green());
        //
        //svg.rect(
        //    Rect {
        //        start: Pt { x: 120.0, y: 120.0 },
        //        end: Pt { x: 60.0, y: 60.0 },
        //    },
        //    Paint::blue(),
        //);
        //
        //let tri = arr2(&[
        //    [30.0, 30.0],
        //    [15.0, 0.0],
        //    [15.0, 0.0],
        //    [0.0, 30.0],
        //    [0.0, 30.0],
        //    [30.0, 30.0],
        //]);
        //let transform = Transform::new().translation(Pt { x: 200.0, y: 200.0 });
        //let pts: Array2<f32> = transform.transform2(&tri);
        //svg.polyline(pts.ndarray(), Paint::red());
        //
        ////translate
        //let transform = Transform::new().translation(Pt { x: 210.0, y: 210.0 });
        //let pts: Array2<f32> = transform.transform2(&tri);
        //svg.polyline(pts.ndarray(), Paint::green());
        //
        ////rotate
        //let mut x = 220.0;
        //let mut y = 220.0;
        //let mut angle = 15.0;
        //while angle < 360.0 {
        //    let transform = Transform::new().translation(Pt { x, y }).rotation(angle);
        //    let pts: Array2<f32> = transform.transform2(&tri);
        //    svg.polyline(pts.ndarray(), Paint::grey());
        //
        //    x += 10.0;
        //    y += 10.0;
        //    angle += 15.0;
        //}
        //
        ////rotate and mirror
        //let mut x = 300.0;
        //let mut y = 220.0;
        //let mut angle = 15.0;
        //while angle < 360.0 {
        //    let transform = Transform::new()
        //        .translation(Pt { x, y })
        //        .rotation(angle)
        //        .mirror(&Some(String::from("x")));
        //    let pts: Array2<f32> = transform.transform2(&tri);
        //    svg.polyline(pts.ndarray(), Paint::green());
        //
        //    x += 10.0;
        //    y += 10.0;
        //    angle += 15.0;
        //}

        //draw the house
        let tri = arr2(&[
            [00.0, 00.0],    // Bottom-left corner of the house
            [00.0, 20.0],    // Top-left corner of the house
            [20.0, 20.0],    // Top-right corner of the house
            [20.0, 00.0],    // Bottom-right corner of the house
            [00.0, 00.0],    // Closing the outline back to the bottom-left corner
            [00.0, 20.0],    // Move to top-left corner for the roof
            [10.0, 30.0],    // Top-middle point (peak of the roof)
            [20.0, 20.0],    // Top-right corner of the house
            [00.0, 00.0],    // Bottom-left to top-right (cross)
            [20.0, 20.0],    // Bottom-left to top-right (cross)
            [20.0, 00.0],    // Bottom-right to top-left (cross)
            [00.0, 20.0],     // Bottom-right to top-left (cross)            
        ]);
        let transform = Transform::new().translation(Pt { x: 10.0, y: 10.0 });
        let pts: Array2<f32> = transform.transform(&tri);
        svg.polyline(pts.ndarray(), Paint {
            color: Color::blue(),
            fill: None,
            width: 0.5,
        });

        let mut file = File::create("/tmp/paint.svg").unwrap();
        svg.write(&mut file).unwrap();
    }
}
