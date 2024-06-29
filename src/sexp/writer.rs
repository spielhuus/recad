use std::io::Write;
use itertools::Itertools;

use crate::{gr::{Arc, Color, Effects, FillType, Polyline, Property, Rectangle, Stroke, StrokeType}, round, schema::{Junction, LibrarySymbol, NoConnect, Pin, Symbol, Wire}, Error, Schema};

use super::{builder::Builder, constants::el, justify, Sexp, SexpTree};

fn yes_or_no(input: bool) -> String {
    if input {
        String::from("yes")
    } else {
        String::from("no")
    }
}


fn sub_lib_id(input: &str) -> Result<String, Error> {
    // Find the position of the colon (':') in the input string
    if let Some(pos) = input.find(':') {
        Ok(input[pos + 1..].to_string())
    } else {
        Err(Error(
                String::from("sexp-libid"), 
                format!("can not find a colon in \"{}\"", input)
            )
        )
    }
}

macro_rules! txt {
    ($fmt:expr, $($arg:expr),*) => {
        format!($fmt, $($arg),*).as_bytes()
    };
}

impl Stroke {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
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
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        match self {
            Color::None => {
                builder.push("color");
                builder.value(&0.to_string());
                builder.value(&0.to_string());
                builder.value(&0.to_string());
                builder.value(&0.to_string());
                builder.end();
            },
            Color::Rgb(r, g, b) => {
                builder.push("color");
                builder.value(&r.to_string());
                builder.value(&g.to_string());
                builder.value(&b.to_string());
                builder.end();
            },
            Color::Rgba(r, g, b, a) => {
                builder.push("color");
                builder.value(&r.to_string());
                builder.value(&g.to_string());
                builder.value(&b.to_string());
                builder.value(&a.to_string());
                builder.end();
            },
        }
        Ok(())
    }
}

///write the property to a string
impl Effects {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push("effects");
        builder.push("font");
        builder.push("size");
        builder.value(&self.font.size.0.to_string());
        builder.value(&self.font.size.1.to_string());
        builder.end();
        builder.end();

        if self.hide {
            builder.push("hide");
            builder.value(&yes_or_no(self.hide));
            builder.end();
        }

        if !self.justify.is_empty() {
            builder.push("justify");
            for j in &self.justify {
                builder.value(&j.to_string());
            }
            builder.end();
        }
        builder.end();
        Ok(())
    }
}

///write the property to a string
impl Property {
    fn write2(&self, builder: &mut Builder) -> Result<(), Error> {

        builder.push("property");
        builder.text(&self.key);
        builder.text(&self.value);

        builder.push("at");
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.value(&self.pos.angle.to_string());
        builder.end();

        self.effects.write(builder)?;

        builder.end();

        Ok(())
    }

    fn write(&self, write: &mut dyn Write, indent: usize) -> Result<(), Error> {
        write.write_all(
            format!("{:>6$}(property \"{}\" \"{}\" (at {} {} {})\n",
                " ",
                self.key,
                self.value,
                round(self.pos.x),
                round(self.pos.y),
                round(self.pos.angle),
                indent
            ).as_bytes()
        )?;
        write.write_all(
            format!("{:>1$}(effects (font (size 1.27 1.27))",
                " ", 
                indent+2
            ).as_bytes()
        )?;
        if !self.effects.justify.is_empty() { 
            write.write_all(
                format!(" (justify {}))\n", 
                    self.effects.justify.iter().join(" "),
                ).as_bytes()
            )?;
        }
        if self.effects.hide {
            write.write_all(" hide".as_bytes())?;
        }
        write.write_all(")\n".as_bytes())?;
        write.write_all(txt!("{:>1$})\n", " ", indent))?;
        Ok(())
    }
}

///Implement the write method for a Symbol
impl Symbol {
    fn write(&self, write: &mut dyn Write) -> Result<(), Error> {
        let txt = format!(r#"  (symbol (lib_id "{}") (at {} {} {}) (unit {})
    (in_bom {}) (on_board {}) (dnp no)
    (uuid {})"#,
            self.lib_id,
            round(self.pos.x),
            round(self.pos.y),
            round(self.pos.angle),
            self.unit,
            yes_or_no(self.in_bom),
            yes_or_no(self.on_board),
            self.uuid,
        );
        write.write_all(txt.as_bytes())?;
        write.write_all("\n".as_bytes())?;
        
        //the properties
        for prop in &self.props {
            prop.write(write, 4)?;
        }

        for pin in &self.pins {
            write.write_all(format!("    (pin \"{}\" (uuid {}))\n", pin.0, pin.1).as_bytes())?;
        }
        write.write_all("  )\n".as_bytes())?;
        Ok(())
    }
}

impl Arc {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push("arc");
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
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
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
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push("polyline");
        builder.push("pts");
        for pt in &self.pts.0 {
            builder.push("xy");
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

impl Pin {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        
        builder.push(el::PIN);
        builder.value(&self.electrical_type.to_string());
        builder.value(&self.graphical_style.to_string());
        builder.push("at");
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.value(&self.pos.angle.to_string());
        builder.end();
        builder.push("length");
        builder.value(&self.length.to_string());
        builder.end();
        if self.hide {
            builder.value("hide");
        }
        builder.push("name");
        builder.text(&self.name.name.to_string());
        self.name.effects.write(builder)?;
        builder.end();

        builder.push("number");
        builder.text(&self.number.name.to_string());
        self.number.effects.write(builder)?;
        builder.end();

        builder.end();

        Ok(())
    }
}

///Implement the write method for a LibrarySymbol
impl LibrarySymbol {
    fn write2(&self, builder: &mut Builder) -> Result<(), Error> {

        builder.push("symbol");
        builder.text(&self.lib_id);
        if self.pin_names {
            builder.push("pin_names");
            if let Some(offset) = self.pin_names_offset {
                builder.push("offset");
                builder.value(&offset.to_string());
                builder.end()
            }
            builder.end();
        }
        builder.push("exclude_from_sim");
        builder.value(&yes_or_no(self.exclude_from_sim));
        builder.end();
        builder.push("in_bom");
        builder.value(&yes_or_no(self.in_bom));
        builder.end();
        builder.push("on_board");
        builder.value(&yes_or_no(self.on_board));
        builder.end();

        for p in &self.props {
            p.write2(builder)?;
        }

        for subsymbol in &self.units {
            builder.push("symbol");
            builder.text(&format!("{}_{}_{}",  sub_lib_id(self.lib_id.trim_start_matches(':'))?, subsymbol.unit(), subsymbol.style()));

            for graph in &subsymbol.graphics {
                match graph {
                    crate::gr::GraphicItem::Arc(a) => a.write(builder)?,
                    crate::gr::GraphicItem::Circle(_) => {},
                    crate::gr::GraphicItem::Curve(_) => {},
                    crate::gr::GraphicItem::Line(_) => {},
                    crate::gr::GraphicItem::Polyline(p) => p.write(builder)?,
                    crate::gr::GraphicItem::Rectangle(r) => r.write(builder)?,
                    crate::gr::GraphicItem::Text(_) => {},
                }
            }
            for pin in &subsymbol.pins {
                pin.write(builder)?;
            }
            builder.end();
        }

        builder.end();
        Ok(())
    }
}

impl Junction {
    fn write2(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push("junction");
        builder.push("at");
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.end();
        builder.push("diameter");
        builder.value(&self.diameter.to_string());
        builder.end();
        if let Some(color) = self.color {
            color.write(builder)?;
        } else {
            Color::None.write(builder)?;
        }
        builder.push("uuid");
        builder.value(&format!("\"{}\"", &self.uuid));
        builder.end();
        builder.end();
        Ok(())
    }
}

impl NoConnect {
    fn write2(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push("no_connect");
        builder.push("at");
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.end();
        builder.push("uuid");
        builder.value(&format!("\"{}\"", &self.uuid));
        builder.end();
        builder.end();
        Ok(())
    }
}

impl Wire {
    fn write2(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push("wire");
        builder.push("pts");
        for pt in &self.pts.0 {
            builder.push("xy");
            builder.value(&pt.x.to_string());
            builder.value(&pt.y.to_string());
            builder.end();
        }
        builder.end();
        builder.push("uuid");
        builder.value(&format!("\"{}\"", &self.uuid));
        builder.end();
        self.stroke.write(builder)?;
        builder.end();
        Ok(())
    }
}

// --------------------------------------------------------------------------
// new writer

impl Sexp {
    pub fn write(&self, indent: usize, writer: &mut dyn Write) -> Result<bool, Error> {
        let mut has_childs = false;
        writer.write_all(format!("\n{:\t>2$}{}", "(", self.name, indent).as_bytes())?;
        for child in &self.nodes {
            match child {
                super::SexpAtom::Node(node) => {
                    has_childs = true;
                    node.write(indent+1, writer)?;
                },
                super::SexpAtom::Value(value) => {
                    writer.write_all(format!(" {}", value).as_bytes())?
                },
                super::SexpAtom::Text(text) => writer.write_all(format!(" \"{}\"", text).as_bytes())?,
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
                super::SexpAtom::Node(node) => { node.write(2, writer)?; },
                super::SexpAtom::Value(value) => {
                    writer.write_all(format!(" {}", value).as_bytes())?
                },
                super::SexpAtom::Text(text) => writer.write_all(format!(" \"{}\"", text).as_bytes())?,
            }
        }
        writer.write_all("\n)".as_bytes())?;

        Ok(())
    }
}

impl Schema {
    pub fn write2(&self, writer: &mut dyn Write) -> Result<(), Error> {
        
        let mut builder = Builder::new();
        builder.push("kicad_sch");
 
        builder.push("version");
        builder.value(&self.version);
        builder.end();

        builder.push("generator");
        builder.text(&self.generator);
        builder.end();

        if let Some(version) = &self.generator_version {
            builder.push("generator_version");
            builder.text(version);
            builder.end();
        }

        builder.push("uuid");
        builder.text(&self.uuid);
        builder.end();

        builder.push("paper");
        builder.text(&self.paper.to_string());
        builder.end();

        builder.push("title_block");

        if let Some(title) = &self.title_block.title {
            builder.push("title");
            builder.text(title);
            builder.end();
        }
        if let Some(date) = &self.title_block.date {
            builder.push("date");
            builder.text(date);
            builder.end();
        }
        if let Some(rev) = &self.title_block.revision {
            builder.push("rev");
            builder.text(rev);
            builder.end();
        }
        for c in &self.title_block.comment {
            builder.push("comment");
            builder.value(&c.0.to_string());
            builder.text(&c.1);
            builder.end();
        }
        builder.end();

        builder.push("lib_symbols");
        for symbol in &self.library_symbols {
            symbol.write2(&mut builder)?;
        }
        builder.end();

        for junction in &self.junctions {
            junction.write2(&mut builder)?;
        }

        for nc in &self.no_connects {
            nc.write2(&mut builder)?;
        }

        for wire in &self.wires {
            wire.write2(&mut builder)?;
        }








        builder.end();

        let sexp = builder.sexp().unwrap();
        sexp.write(writer)?;

        Ok(())
    }
}
