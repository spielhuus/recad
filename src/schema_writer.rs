use std::io::Write;

use crate::{
    gr::{Color, Property},
    schema::{Bus, BusEntry, Junction, LibrarySymbol, LocalLabel, NoConnect, Pin, Polyline, Symbol, Text, Wire},
    sexp::{builder::Builder, constants::el},
    yes_or_no, Error, Schema,
};

fn sub_lib_id(input: &str) -> Result<String, Error> {
    // Find the position of the colon (':') in the input string
    if let Some(pos) = input.find(':') {
        Ok(input[pos + 1..].to_string())
    } else {
        Err(Error(
            String::from("sexp"),
            format!("can not find a colon in \"{}\"", input),
        ))
    }
}

impl Schema {
    pub fn write(&self, writer: &mut dyn Write) -> Result<(), Error> {
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

        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();

        builder.push(el::PAPER);
        builder.text(&self.paper.to_string());
        builder.end();

        builder.push(el::TITLE_BLOCK);

        if let Some(title) = &self.title_block.title {
            builder.push(el::TITLE_BLOCK_TITLE);
            builder.text(title);
            builder.end();
        }
        if let Some(date) = &self.title_block.date {
            builder.push(el::TITLE_BLOCK_DATE);
            builder.text(date);
            builder.end();
        }
        if let Some(rev) = &self.title_block.revision {
            builder.push(el::TITLE_BLOCK_REV);
            builder.text(rev);
            builder.end();
        }
        for c in &self.title_block.comment {
            builder.push(el::TITLE_BLOCK_COMMENT);
            builder.value(&c.0.to_string());
            builder.text(&c.1);
            builder.end();
        }
        builder.end();

        builder.push(el::LIB_SYMBOLS);
        for symbol in &self.library_symbols {
            symbol.write(&mut builder)?;
        }
        builder.end();

        for junction in &self.junctions {
            junction.write(&mut builder)?;
        }

        for nc in &self.no_connects {
            nc.write(&mut builder)?;
        }
        
        for bus_entry in &self.bus_entries {
            bus_entry.write(&mut builder)?;
        }
        
        for bus in &self.busses {
            bus.write(&mut builder)?;
        }

        for wire in &self.wires {
            wire.write(&mut builder)?;
        }
        
        for text in &self.graphical_texts {
            text.write(&mut builder)?;
        }

        for label in &self.local_labels {
            label.write(&mut builder)?;
        }

        for symbol in &self.symbols {
            symbol.write(&mut builder)?;
        }

        builder.end();

        let sexp = builder.sexp().unwrap();
        sexp.write(writer)?;

        Ok(())
    }
}

impl Property {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::PROPERTY);
        builder.text(&self.key);
        builder.text(&self.value);

        builder.push(el::AT);
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.value(&self.pos.angle.to_string());
        builder.end();

        self.effects.write(builder)?;

        builder.end();

        Ok(())
    }
}

impl Junction {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::JUNCTION);
        builder.push(el::AT);
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.end();
        builder.push(el::DIAMETER);
        builder.value(&self.diameter.to_string());
        builder.end();
        if let Some(color) = self.color {
            color.write(builder)?;
        } else {
            Color::None.write(builder)?;
        }
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

impl NoConnect {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::NO_CONNECT);
        builder.push(el::AT);
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.end();
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

impl LocalLabel {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::LABEL);
        builder.text(&self.text);
        builder.push(el::AT);
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.value(&self.pos.angle.to_string());
        builder.end();
        if self.fields_autoplaced {
            builder.push(el::FIELDS_AUTOPLACED);
            builder.value(el::YES);
            builder.end();
        }
        self.effects.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

impl Text {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::TEXT);
        builder.text(&self.text);
        builder.push(el::EXCLUDE_FROM_SIM);
        builder.value(&yes_or_no(self.exclude_from_sim));
        builder.end();
        builder.push(el::AT);
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.value(&self.pos.angle.to_string());
        builder.end();
        self.effects.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

impl Polyline {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
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
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}
	//(polyline
	//	(pts
	//		(xy 205.74 73.66) (xy 205.74 39.37)
	//	)
	//	(stroke
	//		(width 0)
	//		(type default)
	//	)
	//	(uuid "5ee5a0be-32ed-4d5c-ac29-9a387bc8e2f8")
	//)

impl Bus {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::BUS);
        builder.push(el::PTS);
        for pt in &self.pts.0 {
            builder.push(el::XY);
            builder.value(&pt.x.to_string());
            builder.value(&pt.y.to_string());
            builder.end();
        }
        builder.end();
        self.stroke.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

impl BusEntry {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::BUS_ENTRY);
        builder.push(el::AT);
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.end();
        builder.push(el::SIZE);
        builder.value(&self.size.0.to_string());
        builder.value(&self.size.1.to_string());
        builder.end();
        self.stroke.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

impl Wire {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::WIRE);
        builder.push(el::PTS);
        for pt in &self.pts.0 {
            builder.push(el::XY);
            builder.value(&pt.x.to_string());
            builder.value(&pt.y.to_string());
            builder.end();
        }
        builder.end();
        self.stroke.write(builder)?;
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();
        builder.end();
        Ok(())
    }
}

impl LibrarySymbol {
    pub fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::SYMBOL);
        builder.text(&self.lib_id);
        if self.power {
            builder.push(el::POWER);
            builder.end();
        }
        if !self.pin_numbers {
            builder.push(el::PIN_NUMBERS);
            builder.value(el::HIDE);
            builder.end();
        }
        if let Some(offset) = self.pin_names_offset {
            builder.push(el::PIN_NAMES);
            builder.push(el::OFFSET);
            builder.value(&offset.to_string());
            builder.end();
            if !self.pin_names {
                builder.value(el::HIDE);
            }
            builder.end();
        } else if !self.pin_names {
            builder.push(el::PIN_NAMES);
            builder.value(el::HIDE);
            builder.end();
        }
        builder.push(el::EXCLUDE_FROM_SIM);
        builder.value(&crate::yes_or_no(self.exclude_from_sim));
        builder.end();
        builder.push(el::IN_BOM);
        builder.value(&crate::yes_or_no(self.in_bom));
        builder.end();
        builder.push(el::ON_BOARD);
        builder.value(&crate::yes_or_no(self.on_board));
        builder.end();

        for p in &self.props {
            p.write(builder)?;
        }

        for subsymbol in &self.units {
            builder.push(el::SYMBOL);
            builder.text(&format!(
                "{}_{}_{}",
                sub_lib_id(self.lib_id.trim_start_matches(':'))?,
                subsymbol.unit(),
                subsymbol.style()
            ));

            for graph in &subsymbol.graphics {
                match graph {
                    crate::gr::GraphicItem::Arc(a) => a.write(builder)?,
                    crate::gr::GraphicItem::Circle(c) => c.write(builder)?,
                    crate::gr::GraphicItem::Curve(_) => {}
                    crate::gr::GraphicItem::Line(_) => {}
                    crate::gr::GraphicItem::Polyline(p) => p.write(builder)?,
                    crate::gr::GraphicItem::Rectangle(r) => r.write(builder)?,
                    crate::gr::GraphicItem::Text(_) => {}
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

impl Pin {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::PIN);
        builder.value(&self.electrical_type.to_string());
        builder.value(&self.graphical_style.to_string());
        builder.push(el::AT);
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.value(&self.pos.angle.to_string());
        builder.end();
        builder.push(el::LENGTH);
        builder.value(&self.length.to_string());
        builder.end();
        if self.hide {
            builder.value(el::HIDE);
        }
        builder.push(el::NAME);
        builder.text(&self.name.name.to_string());
        self.name.effects.write(builder)?;
        builder.end();

        builder.push(el::NUMBER);
        builder.text(&self.number.name.to_string());
        self.number.effects.write(builder)?;
        builder.end();

        builder.end();

        Ok(())
    }
}

impl Symbol {
    fn write(&self, builder: &mut Builder) -> Result<(), Error> {
        builder.push(el::SYMBOL);
        builder.push(el::LIB_ID);
        builder.text(&self.lib_id);
        builder.end();
        builder.push(el::AT);
        builder.value(&self.pos.x.to_string());
        builder.value(&self.pos.y.to_string());
        builder.value(&self.pos.angle.to_string());
        builder.end();
        if let Some(mirror) = &self.mirror {
            builder.push(el::MIRROR);
            builder.value(mirror);
            builder.end();
        }
        builder.push(el::SYMBOL_UNIT);
        builder.value(&self.unit.to_string());
        builder.end();
        builder.push(el::EXCLUDE_FROM_SIM);
        builder.value(&crate::yes_or_no(self.exclude_from_sim));
        builder.end();
        builder.push(el::IN_BOM);
        builder.value(&crate::yes_or_no(self.in_bom));
        builder.end();
        builder.push(el::ON_BOARD);
        builder.value(&crate::yes_or_no(self.on_board));
        builder.end();
        builder.push(el::DNP);
        builder.value(&crate::yes_or_no(self.dnp));
        builder.end();
        builder.push(el::UUID);
        builder.text(&self.uuid);
        builder.end();

        for prop in &self.props {
            prop.write(builder)?;
        }

        for pin in &self.pins {
            builder.push(el::PIN);
            builder.text(&pin.0);
            builder.push(el::UUID);
            builder.text(&pin.1);
            builder.end();
            builder.end();
        }

        for instance in &self.instances {
            builder.push(el::INSTANCES);
            builder.push(el::PROJECT);
            builder.text(&instance.project);
            builder.push(el::PATH);
            builder.text(&instance.path);
            builder.push(el::REFERENCE);
            builder.text(&instance.reference);
            builder.end();
            builder.push(el::SYMBOL_UNIT);
            builder.value(&instance.unit.to_string());
            builder.end();
            builder.end();
            builder.end();
            builder.end();
        }
        builder.end();

        Ok(())
    }
}
