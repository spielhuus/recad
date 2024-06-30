use std::io::Write;

use crate::{
    gr::{Arc, Circle, Color, Effects, FillType, Polyline, Rectangle, Stroke},
    Error,
};

use super::{builder::Builder, constants::el, Sexp, SexpTree};



impl Stroke {
    pub fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push("stroke");
        builder.push("width");
        builder.value(&self.width.to_string());
        builder.end();
        if let Some(stroketype) = &self.stroke_type {
            builder.push("type");
            builder.value(&stroketype.to_string());
            builder.end();
        }
        builder.end();
        Ok(())
    }
}

impl FillType {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push("fill");
        builder.push("type");
        builder.value(&self.to_string());
        builder.end();
        builder.end();
        Ok(())
    }
}

impl Color {
    pub fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        match self {
            Color::None => {
                builder.push(el::COLOR);
                builder.value(&0.to_string());
                builder.value(&0.to_string());
                builder.value(&0.to_string());
                builder.value(&0.to_string());
                builder.end();
            }
            Color::Rgb(r, g, b) => {
                builder.push(el::COLOR);
                builder.value(&r.to_string());
                builder.value(&g.to_string());
                builder.value(&b.to_string());
                builder.end();
            }
            Color::Rgba(r, g, b, a) => {
                builder.push(el::COLOR);
                builder.value(&r.to_string());
                builder.value(&g.to_string());
                builder.value(&b.to_string());
                builder.value(&a.to_string());
                builder.end();
            }
        }
        Ok(())
    }
}

impl Effects {
    pub fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::EFFECTS);
        builder.push(el::FONT);
        builder.push(el::SIZE);
        builder.value(&self.font.size.0.to_string());
        builder.value(&self.font.size.1.to_string());
        builder.end();
        if self.font.italic {
            builder.push(el::ITALIC);
            builder.value(el::YES);
            builder.end();
        } 
        if self.font.bold {
            builder.push(el::BOLD);
            builder.value(el::YES);
            builder.end();
        } 
        builder.end();

        if !self.justify.is_empty() {
            builder.push(el::JUSTIFY);
            for j in &self.justify {
                builder.value(&j.to_string());
            }
            builder.end();
        }

        if self.hide {
            builder.push(el::HIDE);
            builder.value(&crate::yes_or_no(self.hide));
            builder.end();
        }

        builder.end();
        Ok(())
    }
}

impl Circle {
    pub fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::CIRCLE);
        builder.push(el::CENTER);
        builder.value(&self.center.x.to_string());
        builder.value(&self.center.y.to_string());
        builder.end();
        builder.push(el::RADIUS);
        builder.value(&self.radius.to_string());
        builder.end();
        self.stroke.write(builder)?;
        self.fill.write(builder)?;
        builder.end();
        Ok(())
    }
}

impl Arc {
    pub fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::ARC);
        builder.push("start");
        builder.value(&self.start.x.to_string());
        builder.value(&self.start.y.to_string());
        builder.end();
        builder.push("mid");
        builder.value(&self.mid.x.to_string());
        builder.value(&self.mid.y.to_string());
        builder.end();
        builder.push("end");
        builder.value(&self.end.x.to_string());
        builder.value(&self.end.y.to_string());
        builder.end();
        self.stroke.write(builder)?;
        self.fill.write(builder)?;
        builder.end();
        Ok(())
    }
}

impl Rectangle {
    pub fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push("rectangle");
        builder.push("start");
        builder.value(&self.start.x.to_string());
        builder.value(&self.start.y.to_string());
        builder.end();
        builder.push("end");
        builder.value(&self.end.x.to_string());
        builder.value(&self.end.y.to_string());
        builder.end();
        self.stroke.write(builder)?;
        self.fill.write(builder)?;
        builder.end();
        Ok(())
    }
}

impl Polyline {
    pub fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::POLYLINE);
        builder.push(el::PTS);
        for pt in &self.pts.0 {
            builder.push(el::XY);
            builder.value(&pt.x.to_string());
            builder.value(&pt.y.to_string());
            builder.end();
        }
        builder.end();
        self.stroke.write(builder)?;
        self.fill.write(builder)?;
        builder.end();
        Ok(())
    }
}

// --------------------------------------------------------------------------
// sexp writer
// --------------------------------------------------------------------------

impl Sexp {
    pub fn write(&self, indent: usize, writer: &mut dyn Write) -> Result<bool, Error> {
        let mut has_childs = false;
        writer.write_all(format!("\n{:\t>2$}{}", "(", self.name, indent).as_bytes())?;
        for child in &self.nodes {
            match child {
                super::SexpAtom::Node(node) => {
                    has_childs = true;
                    node.write(indent + 1, writer)?;
                }
                super::SexpAtom::Value(value) => {
                    writer.write_all(format!(" {}", value).as_bytes())?
                }
                super::SexpAtom::Text(text) => {
                    writer.write_all(format!(" \"{}\"", text).as_bytes())?
                }
            }
        }
        if has_childs {
            writer.write_all(format!("\n{:\t>1$}", ")", indent).as_bytes())?;
        } else {
            writer.write_all(")".as_bytes())?;
        }

        Ok(has_childs)
    }
}

impl SexpTree {
    pub fn write(&self, writer: &mut dyn Write) -> Result<(), Error> {
        let node = self.root().unwrap();

        writer.write_all(format!("({}", node.name).as_bytes())?;
        for child in &node.nodes {
            match child {
                super::SexpAtom::Node(node) => {
                    node.write(2, writer)?;
                }
                super::SexpAtom::Value(value) => {
                    writer.write_all(format!(" {}", value).as_bytes())?
                }
                super::SexpAtom::Text(text) => {
                    writer.write_all(format!(" \"{}\"", text).as_bytes())?
                }
            }
        }
        writer.write_all("\n)".as_bytes())?;

        Ok(())
    }
}

