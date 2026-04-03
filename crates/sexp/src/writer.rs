use std::io::Write;


use types::{
    gr::{Arc, Circle, Color, Effects, FillType, Polyline, Rectangle, Stroke},
    error::RecadError,
    constants::el,
};

use crate::SexpWrite;

use super::{builder::Builder, Sexp, SexpTree};

pub fn write_uuid(builder: &mut Builder, uuid: &Option<String>) {
    if let Some(uuid) = uuid {
        builder.push(el::UUID);
        builder.text(uuid);
        builder.end();
    }
}

/// Helper to safely escape strings before writing them to the file.
fn escape_string(s: &str) -> String {
    if !s.contains(['\"', '\\', '\n', '\r', '\t']) {
        return s.to_string();
    }

    let mut result = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            _ => result.push(c),
        }
    }
    result
}

impl SexpWrite for Arc {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::ARC);
        builder.push(el::START);
        builder.value(self.start.x);
        builder.value(self.start.y);
        builder.end();
        builder.push(el::MID);
        builder.value(self.mid.x);
        builder.value(self.mid.y);
        builder.end();
        builder.push(el::END);
        builder.value(self.end.x);
        builder.value(self.end.y);
        builder.end();
        self.stroke.write(builder)?;
        self.fill.write(builder)?;
        write_uuid(builder, &self.uuid);
        builder.end();
        Ok(())
    }
}

impl SexpWrite for Circle {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::CIRCLE);
        builder.push(el::CENTER);
        builder.value(self.center.x);
        builder.value(self.center.y);
        builder.end();
        builder.push(el::RADIUS);
        builder.value(self.radius);
        builder.end();
        self.stroke.write(builder)?;
        self.fill.write(builder)?;
        write_uuid(builder, &self.uuid);
        builder.end();
        Ok(())
    }
}

impl SexpWrite for Polyline {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::POLYLINE);
        builder.push(el::PTS);
        for pt in &self.pts.0 {
            builder.push(el::XY);
            builder.value(pt.x);
            builder.value(pt.y);
            builder.end();
        }
        builder.end();
        self.stroke.write(builder)?;
        if let Some(fill) = &self.fill {
            fill.write(builder)?;
        }
        write_uuid(builder, &self.uuid);
        builder.end();
        Ok(())
    }
}

impl SexpWrite for Rectangle {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::RECTANGLE);
        builder.push(el::START);
        builder.value(self.start.x);
        builder.value(self.start.y);
        builder.end();
        builder.push(el::END);
        builder.value(self.end.x);
        builder.value(self.end.y);
        builder.end();
        self.stroke.write(builder)?;
        self.fill.write(builder)?;
        write_uuid(builder, &self.uuid);
        builder.end();
        Ok(())
    }
}

impl SexpWrite for Stroke {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::STROKE);
        builder.push(el::WIDTH);
        builder.value(self.width);
        builder.end();
        if let Some(stroketype) = &self.stroke_type {
            builder.push(el::TYPE);
            builder.value(stroketype);
            builder.end();
        }
        if let Some(color) = &self.color {
            color.write(builder)?;
        }
        builder.end();
        Ok(())
    }
}

impl SexpWrite for FillType {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::FILL);
        builder.push(el::TYPE);
        builder.value(self);
        builder.end();
        if let FillType::Color(c) = self {
            c.write(builder)?;
        }
        builder.end();
        Ok(())
    }
}

impl SexpWrite for Color {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        match self {
            Color::None => {
                builder.push(el::COLOR);
                builder.value("0");
                builder.value("0");
                builder.value("0");
                builder.value("0");
                builder.end();
            }
            Color::Rgb(r, g, b) => {
                builder.push(el::COLOR);
                builder.value(r);
                builder.value(g);
                builder.value(b);
                builder.end();
            }
            Color::Rgba(r, g, b, a) => {
                builder.push(el::COLOR);
                builder.value(r);
                builder.value(g);
                builder.value(b);
                builder.value(*a as f64 / 255.0);
                builder.end();
            }
        }
        Ok(())
    }
}

impl SexpWrite for Effects {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError> {
        builder.push(el::EFFECTS);
        builder.push(el::FONT);
        if let Some(face) = &self.font.face {
            builder.push(el::FACE);
            builder.text(face);
            builder.end();
        }
        builder.push(el::SIZE);
        builder.value(self.font.size.0);
        builder.value(self.font.size.1);
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
                builder.value(j);
            }
            builder.end();
        }

        if self.hide {
            builder.push(el::HIDE);
            builder.value(types::yes_or_no(self.hide));
            builder.end();
        }

        builder.end();
        Ok(())
    }
}

// --------------------------------------------------------------------------
// sexp writer
// --------------------------------------------------------------------------

impl<'a> Sexp<'a> {
    pub fn write(&self, indent: usize, writer: &mut dyn Write) -> Result<bool, RecadError> {
        let mut has_children = false;
        writer.write_all(format!("\n{:\t>2$}{}", "(", self.name, indent).as_bytes())?;
        for child in &self.nodes {
            match child {
                super::SexpAtom::Node(node) => {
                    has_children = true;
                    node.write(indent + 1, writer)?;
                }
                super::SexpAtom::Value(value) => {
                    writer.write_all(format!(" {}", value).as_bytes())?
                }
                super::SexpAtom::Text(text) => write!(writer, " \"{}\"", escape_string(text))?,
            }
        }
        if has_children {
            writer.write_all(format!("\n{:\t>1$}", ")", indent).as_bytes())?;
        } else {
            writer.write_all(")".as_bytes())?;
        }

        Ok(has_children)
    }
}

impl<'a> SexpTree<'a> {
    pub fn write(&self, writer: &mut dyn Write) -> Result<(), RecadError> {
        let node = self.root();

        writer.write_all(format!("({}", node.name).as_bytes())?;
        for child in &node.nodes {
            match child {
                super::SexpAtom::Node(node) => {
                    node.write(2, writer)?;
                }
                super::SexpAtom::Value(value) => {
                    writer.write_all(format!(" {}", value).as_bytes())?
                }
                super::SexpAtom::Text(text) => write!(writer, " \"{}\"", escape_string(text))?,
            }
        }
        writer.write_all("\n)".as_bytes())?;

        Ok(())
    }
}
