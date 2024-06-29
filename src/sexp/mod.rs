mod builder;
pub mod constants;
pub mod parser;
mod writer;

use std::collections::HashMap;

use crate::{
    gr::{
        self, Color, Effects, FillType, Font, Justify, PaperSize, Pos, Property, Pt, Pts, Stroke,
        StrokeType, TitleBlock,
    },
    pcb::{self, Footprint, FootprintType, FpLine, Net, Pad, PadShape, PadType, Segment},
    schema::{self, ElectricalTypes, PinGraphicalStyle, PinProperty},
    Error, Pcb, Schema,
};

use constants::el;

type SexpString = dyn SexpValue<String>;
type SexpStringList = dyn SexpQuery<Vec<String>>;

///The sexp element types.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SexpAtom {
    ///Child node.
    Node(Sexp),
    ///Value
    Value(String),
    ///Text surrounded with quotes.
    Text(String),
}

///Sexp Element
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Sexp {
    ///name of the node
    pub name: String,
    ///Children of the node.
    nodes: Vec<SexpAtom>,
}

impl Sexp {

    ///Create a new sexp node with name.
    pub fn from(name: String) -> Self {
        Sexp {
            name,
            nodes: Vec::new(),
        }
    }

    ///get the nodes.
    fn nodes(&self) -> impl Iterator<Item = &Sexp> {
        self.nodes.iter().filter_map(|n| {
            if let SexpAtom::Node(node) = n {
                Some(node)
            } else {
                None
            }
        })
    }

    ///query child nodes for elements by name.
    pub fn query<'a>(&'a self, q: &'a str) -> impl Iterator<Item = &Sexp> + 'a {
        self.nodes.iter().filter_map(move |n| {
            if let SexpAtom::Node(node) = n {
                if node.name == q {
                    Some(node)
                } else {
                    None
                }
            } else {
                None
            }
        })
    }
}

///Sexp document.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SexpTree {
    tree: Sexp,
}

impl<'a> SexpTree {
    ///parse a sexp document for SexpParser Iterator.
    pub fn from<I>(mut iter: I) -> Result<Self, Error>
    where
        I: Iterator<Item = State<'a>>,
    {
        let mut stack: Vec<(String, Sexp)> = Vec::new();
        if let Some(State::StartSymbol(name)) = iter.next() {
            stack.push((name.to_string(), Sexp::from(name.to_string())));
        } else {
            return Err(Error(
                String::from("Document does not start with a start symbol."),
                String::from("from item"),
            ));
        };
        loop {
            match iter.next() {
                Some(State::Values(value)) => {
                    let len = stack.len();
                    if let Some((_, parent)) = stack.get_mut(len - 1) {
                        parent.nodes.push(SexpAtom::Value(value.to_string()));
                    }
                }
                Some(State::Text(value)) => {
                    let len = stack.len();
                    if let Some((_, parent)) = stack.get_mut(len - 1) {
                        parent.nodes.push(SexpAtom::Text(value.to_string()));
                    }
                }
                Some(State::EndSymbol) => {
                    let len = stack.len();
                    if len > 1 {
                        let (_n, i) = stack.pop().unwrap();
                        if let Some((_, parent)) = stack.get_mut(len - 2) {
                            parent.nodes.push(SexpAtom::Node(i));
                        }
                    }
                }
                Some(State::StartSymbol(name)) => {
                    stack.push((name.to_string(), Sexp::from(name.to_string())));
                }
                None => break,
            }
        }
        let (_n, i) = stack.pop().unwrap();
        Ok(SexpTree { tree: i })
    }

    ///Get the root element.
    pub fn root(&self) -> Result<&Sexp, Error> {
        Ok(&self.tree)
    }
}

trait SexpQuery<E> {
    ///Return the values from a node.
    fn values(&self) -> E;
}

///get sexp values as Strings.
impl SexpQuery<Vec<String>> for Sexp {
    ///Return values from a node.
    fn values(&self) -> Vec<String> {
        self.nodes
            .iter()
            .filter_map(|n| {
                if let SexpAtom::Value(value) = n {
                    Some(value.clone())
                } else if let SexpAtom::Text(value) = n {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .collect()
    }
}

///get sexp values as u8.
impl SexpQuery<Vec<u8>> for Sexp {
    ///Return a single value from a node.
    fn values(&self) -> Vec<u8> {
        let vals: Vec<String> = self
            .nodes
            .iter()
            .filter_map(|n| {
                if let SexpAtom::Value(value) = n {
                    Some(value.clone())
                } else if let SexpAtom::Text(value) = n {
                    Some(value.clone())
                } else {
                    None
                }
            })
            .collect();

        vals.iter()
            .map(|v| v.parse::<u8>().unwrap())
            .collect::<Vec<u8>>()
    }
}

///Get a single sexp value.
///
///Get a sexp value by name or index.
///There could be multiple values, the
///first is returned.
pub trait SexpValue<E> {
    ///Return the first value from a node by name.
    fn first(&self, q: &str) -> Option<E>;
    ///get value at index.
    fn get(&self, index: usize) -> Option<E>;
}

impl SexpValue<String> for Sexp {
    ///Return a single value from a node.
    fn first(&self, q: &str) -> Option<String> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = SexpStringList::values(node).first() {
                return Some(value.to_string());
            }
        }
        None
    }

    ///Return a positional value from the node.
    fn get(&self, index: usize) -> Option<String> {
        if let Some(value) = SexpStringList::values(self).get(index) {
            return Some(value.to_string());
        }
        None
    }
}

impl SexpValue<u8> for Sexp {
    fn first(&self, q: &str) -> Option<u8> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = SexpStringList::values(node).first() {
                return Some(value.parse::<u8>().unwrap());
            }
        }
        None
    }

    fn get(&self, index: usize) -> Option<u8> {
        if let Some(value) = SexpStringList::values(self).get(index) {
            return Some(value.parse::<u8>().unwrap());
        }
        None
    }
}

impl SexpValue<u32> for Sexp {
    fn first(&self, q: &str) -> Option<u32> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = SexpStringList::values(node).first() {
                return Some(value.parse::<u32>().unwrap());
            }
        }
        None
    }
    fn get(&self, index: usize) -> Option<u32> {
        if let Some(value) = SexpStringList::values(self).get(index) {
            return Some(value.parse::<u32>().unwrap());
        }
        None
    }
}

impl SexpValue<bool> for Sexp {
    fn first(&self, q: &str) -> Option<bool> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = SexpStringList::values(node).first() {
                return Some(value == "true" || value == "yes");
            }
        }
        Some(false)
    }
    fn get(&self, index: usize) -> Option<bool> {
        if let Some(value) = SexpStringList::values(self).get(index) {
            return Some(value == "true" || value == "yes");
        }
        Some(false)
    }
}

impl SexpValue<f32> for Sexp {

    fn first(&self, q: &str) -> Option<f32> {
        let node = self.query(q).next();
        if let Some(node) = node {
            if let Some(value) = SexpStringList::values(node).first() {
                return Some(value.parse::<f32>().unwrap());
            }
        }
        None
    }

    fn get(&self, index: usize) -> Option<f32> {
        if let Some(value) = SexpStringList::values(self).get(index) {
            return Some(value.parse::<f32>().unwrap());
        }
        None
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State<'a> {
    StartSymbol(&'a str),
    EndSymbol,
    Values(&'a str),
    Text(&'a str),
}

#[derive(Debug, PartialEq, Clone)]
enum IntState {
    NotStarted,
    Symbol,
    Values,
    BeforeEndSymbol,
}

impl std::convert::From<&Sexp> for Pos {
    fn from(sexp: &Sexp) -> Self {
        let at = sexp.query(el::AT).next().unwrap();
        Pos {
            x: at.get(0).unwrap(),
            y: at.get(1).unwrap(),
            angle: at.get(2).unwrap_or(0.0),
        }
    }
}

impl std::convert::From<&Sexp> for Pt {
    fn from(sexp: &Sexp) -> Self {
        let x: f32 = sexp.get(0).unwrap();
        let y: f32 = sexp.get(1).unwrap();
        Pt { x, y }
    }
}

impl std::convert::From<&Sexp> for Pts {
    fn from(sexp: &Sexp) -> Self {
        let mut pts: Vec<Pt> = Vec::new();
        for pt in sexp.query(el::PTS) {
            for xy in pt.query(el::XY) {
                let x: f32 = xy.get(0).unwrap();
                let y: f32 = xy.get(1).unwrap();
                pts.push(Pt { x, y });
            }
        }
        Pts(pts)
    }
}

//TODO review needed
impl std::convert::From<&Sexp> for Result<Color, Error> {
    fn from(sexp: &Sexp) -> Result<Color, Error> {
        let Some(s) = sexp.query("color").next() else {
            return Err(Error(
                "sexp".to_string(),
                format!("color not found in: {:?}", sexp),
            ));
        };
        let mut colors: Vec<u8> = s.values();
        colors.pop();
        let a: Option<f32> = s.get(3);
        if a.is_none() { //TODO try something
            return Err(Error(
                "sexp".to_string(),
                format!("a value not found: {:?}", sexp),
            ));
        };

        if colors != vec![0, 0, 0, 0] {
            Ok(Color::Rgba(
                colors[0],
                colors[1],
                colors[2],
                (a.unwrap() * 255.0) as u8,
            ))
        } else {
            Err(Error("sexp".to_string(), "no color is set".to_string()))
        }
    }
}

impl std::convert::From<&Sexp> for Stroke {
    fn from(value: &Sexp) -> Self {
        let Some(stroke) = value.query(el::STROKE).next() else {
            panic!("no stroke found in {:?}", value); //TODO
        };
        let color: Result<Color, Error> = stroke.into();
        let stroke_type: Option<String> = stroke.first("type");
        Stroke {
            width: stroke.first(el::WIDTH).unwrap_or(0.0),
            stroke_type: stroke_type.map(|s| StrokeType::from(s.as_str())),
            color: color.ok(), //the error get consumed and converted to None
        }
    }
}

impl std::convert::From<&Sexp> for Font {
    fn from(sexp: &Sexp) -> Self {
        let font = sexp.query("font").next().unwrap();
        let size = font.query("size").next().unwrap();
        Font {
            face: font.first("face"),
            size: (size.get(0).unwrap(), size.get(1).unwrap()),
            thickness: font.first("tickness"),
            bold: SexpStringList::values(font).contains(&"bold".to_string()),
            italic: SexpStringList::values(font).contains(&"italic".to_string()),
            line_spacing: font.first("spacing"), //TODO check name in sexp file.
            color: None,                         //TODO
        }
    }
}

fn hide(node: &Sexp) -> bool {
    let new_visible: Option<String> = node.first("hide");
    if let Some(new_visible) = new_visible {
        &new_visible == "yes"
    } else {
        let visible: Vec<String> = node.values();
        visible.contains(&el::HIDE.to_string())
    }
}

fn justify(node: &Sexp) -> Vec<Justify> {
    let mut j = node.query(el::JUSTIFY);
    if let Some(j) = j.next() {
        SexpStringList::values(j)
            .iter()
            .map(|j| Justify::from(j.to_string()))
            .collect::<Vec<Justify>>()
    } else {
        Vec::new()
    }
}

impl std::convert::From<&Sexp> for Effects {
    fn from(sexp: &Sexp) -> Self {
        let effects = sexp.query("effects").next().unwrap();
        Effects {
            justify: justify(effects),
            hide: hide(effects),
            font: effects.into(),
        }
    }
}

///extract a title block section, root must the the title_block itself.
impl std::convert::From<&Sexp> for TitleBlock {
    fn from(sexp: &Sexp) -> Self {
        TitleBlock {
            title: sexp.first(el::TITLE_BLOCK_TITLE),
            date: sexp.first(el::TITLE_BLOCK_DATE),
            revision: sexp.first(el::TITLE_BLOCK_REV),
            company_name: sexp.first(el::TITLE_BLOCK_COMPANY),
            comment: sexp
                .query(el::TITLE_BLOCK_COMMENT)
                .map(|c| (c.get(0).unwrap(), c.get(1).unwrap()))
                .collect(),
        }
    }
}

///extract a wire section, root must the the wire itself.
impl std::convert::From<&Sexp> for schema::Wire {
    fn from(sexp: &Sexp) -> Self {
        schema::Wire {
            pts: sexp.into(),
            stroke: sexp.into(),
            uuid: sexp.first("uuid").expect("uuid is mandatory."),
        }
    }
}

impl std::convert::From<&Sexp> for schema::LocalLabel {
    fn from(sexp: &Sexp) -> Self {
        let color: Result<Color, Error> = sexp.into();
        schema::LocalLabel {
            text: sexp.get(0).expect("text is mandatory."),
            pos: sexp.into(),
            effects: sexp.into(),
            color: color.ok(),
            uuid: sexp.first(el::UUID).expect("mandatory."),
        }
    }
}

impl std::convert::From<&Sexp> for schema::GlobalLabel {
    fn from(sexp: &Sexp) -> Self {
        schema::GlobalLabel {
            text: sexp.get(0).unwrap(),
            shape: sexp.first("shape"),
            pos: sexp.into(),
            effects: Effects::default(), //todo!(),
            uuid: sexp.first("uuid").unwrap(),
        }
    }
}

impl std::convert::From<&Sexp> for schema::Junction {
    fn from(sexp: &Sexp) -> Self {
        schema::Junction {
            pos: sexp.into(),
            diameter: sexp.first("diameter").unwrap_or(0.0),
            color: None, //TODO
            uuid: sexp.first(el::UUID).unwrap(),
        }
    }
}

impl std::convert::From<&Sexp> for schema::NoConnect {
    fn from(sexp: &Sexp) -> Self {
        schema::NoConnect {
            pos: sexp.into(),
            uuid: sexp.first(el::UUID).unwrap(),
        }
    }
}

fn properties(node: &Sexp) -> Vec<Property> {
    node.query(el::PROPERTY)
        .collect::<Vec<&Sexp>>()
        .iter()
        .map(|x| Property {
            pos: (*x).into(),
            key: x.get(0).unwrap(),
            value: x.get(1).unwrap(),
            effects: (*x).into(),
        })
        .collect()
}

fn pin_numbers(node: &Sexp) -> bool {
    let pin_numbers = node.query("pin_numbers").collect::<Vec<&Sexp>>();
    if pin_numbers.is_empty() {
        true
    } else {
        !<Sexp as SexpQuery<Vec<String>>>::values(
            pin_numbers.first().expect("tested before for is_empty"),
        )
        .contains(&el::HIDE.to_string())
    }
}

fn pin_names(node: &Sexp) -> bool {
    let pin_names = node.query(el::PIN_NAMES).collect::<Vec<&Sexp>>();
    if pin_names.is_empty() {
        true
    } else {
        !<Sexp as SexpQuery<Vec<String>>>::values(pin_names.first().expect("tested before"))
            .contains(&el::HIDE.to_string())
    }
}

pub fn pin_names_offset(sexp: &Sexp) -> Option<f32> {
    if let Some(names) = sexp.query(el::PIN_NAMES).next() {
        if let Some(offset) = <Sexp as SexpValue<f32>>::first(names, el::OFFSET) {
            return Some(offset);
        }
    }
    None
}

impl std::convert::From<&Sexp> for schema::LibrarySymbol {
    fn from(sexp: &Sexp) -> Self {
        schema::LibrarySymbol {

            lib_id: sexp.get(0).unwrap(),
            extends: sexp.first("extends"),
            power: sexp.query("power").next().is_some(),
            //TODO in_bom: <Sexp as SexpValue<String>>::first(sexp, "in_bom").unwrap_or("yes".to_string())
            exclude_from_sim: if let Some(exclude) =
                <Sexp as SexpValue<String>>::first(sexp, "exclude_from_sim")
            {
                exclude == "yes"
            } else {
                false
            },
            in_bom: SexpString::first(sexp, "in_bom").unwrap_or("yes".to_string()) == "yes",
            on_board: SexpString::first(sexp, "on_board").unwrap_or("yes".to_string()) == "yes",
            props: properties(sexp),
            graphics: sexp
                .nodes()
                .filter_map(|node| match node.name.as_str() {
                    "arc" => Some(gr::GraphicItem::Arc(gr::Arc {
                        start: node.query("start").next().unwrap().into(),
                        mid: node.query("mid").next().unwrap().into(),
                        end: node.query("end").next().unwrap().into(),
                        stroke: node.into(),
                        fill: FillType::from(
                            <Sexp as SexpValue<String>>::first(
                                node.query("fill").next().unwrap(),
                                "type",
                            )
                            .unwrap(),
                        ),
                    })),
                    "circle" => Some(gr::GraphicItem::Circle(gr::Circle {
                        center: node.query("center").next().unwrap().into(),
                        radius: node.first("radius").unwrap(),
                        stroke: node.into(),
                        fill: FillType::from(
                            <Sexp as SexpValue<String>>::first(
                                node.query("fill").next().unwrap(),
                                "type",
                            )
                            .unwrap(),
                        ),
                    })),
                    "curve" => Some(gr::GraphicItem::Curve(gr::Curve {
                        pts: node.into(),
                        stroke: node.into(),
                        fill: FillType::from(
                            <Sexp as SexpValue<String>>::first(
                                node.query("fill").next().unwrap(),
                                "type",
                            )
                            .unwrap(),
                        ),
                    })),
                    "polyline" => Some(gr::GraphicItem::Polyline(gr::Polyline {
                        pts: node.into(),
                        stroke: node.into(),
                        fill: FillType::from(
                            <Sexp as SexpValue<String>>::first(
                                node.query("fill").next().unwrap(),
                                "type",
                            )
                            .unwrap(),
                        ),
                    })),
                    "line" => Some(gr::GraphicItem::Line(gr::Line {
                        pts: node.into(),
                        stroke: node.into(),
                        fill: FillType::from(
                            <Sexp as SexpValue<String>>::first(
                                node.query("fill").next().unwrap(),
                                "type",
                            )
                            .unwrap(),
                        ),
                    })),
                    "rectangle" => Some(gr::GraphicItem::Rectangle(gr::Rectangle {
                        start: node.query("start").next().unwrap().into(),
                        end: node.query("end").next().unwrap().into(),
                        stroke: node.into(),
                        fill: FillType::from(
                            <Sexp as SexpValue<String>>::first(
                                node.query("fill").next().unwrap(),
                                "type",
                            )
                            .unwrap(),
                        ),
                    })),
                    "text" => Some(gr::GraphicItem::Text(gr::Text {
                        text: node.get(0).expect("text"),
                        pos: node.into(),
                        effects: node.into(),
                    })),
                    _ => {
                        if node.name != "pin"
                            && node.name != "symbol"
                            && node.name != "power"
                            && node.name != "pin_numbers"
                            && node.name != "pin_names"
                            && node.name != "in_bom"
                            && node.name != "on_board"
                            && node.name != "exclude_from_sim"
                            && node.name != "property"
                            && node.name != "extends"
                        {
                            panic!("unknown graphic type: {}", node.name);
                        }
                        None
                    }
                })
                .collect(),
            pins: sexp
                .nodes()
                .filter_map(|node| match node.name.as_str() {
                    "pin" => Some(schema::Pin {
                        electrical_type: ElectricalTypes::from(
                            <Sexp as SexpValue<String>>::get(node, 0).unwrap().as_str(),
                        ),
                        graphical_style: PinGraphicalStyle::from(
                            <Sexp as SexpValue<String>>::get(node, 1).unwrap().as_str(),
                        ),
                        pos: node.into(),
                        length: <Sexp as SexpValue<f32>>::first(node, "length").expect("required"),
                        hide: SexpStringList::values(node).contains(&"hide".to_string()),
                        name: {
                            let name = node.query("name").next().unwrap();
                            PinProperty {
                                name: name.get(0).unwrap(),
                                effects: name.into(),
                            }
                        },
                        number: {
                            let number = node.query("number").next().unwrap();
                            PinProperty {
                                name: number.get(0).unwrap(),
                                effects: number.into(),
                            }
                        },
                    }),
                    _ => None,
                })
                .collect(),
            pin_numbers: pin_numbers(sexp),
            pin_names: pin_names(sexp),
            pin_names_offset: pin_names_offset(sexp),
            units: sexp.query("symbol").map(|s| s.into()).collect(),
            unit_name: sexp.first("unit_name"), //TODO check name in sexp file.
        }
    }
}

impl std::convert::From<&Sexp> for schema::Symbol {
    fn from(sexp: &Sexp) -> Self {
        schema::Symbol {
            lib_id: sexp.first(el::LIB_ID).unwrap(),
            pos: sexp.into(),
            unit: sexp.first(el::SYMBOL_UNIT).unwrap(),
            mirror: sexp.first(el::MIRROR),
            in_bom: SexpString::first(sexp, el::IN_BOM).expect("required fuild") == "yes",
            on_board: SexpString::first(sexp, "on_board").unwrap() == "yes",
            exclude_from_sim: if let Some(exclude) =
                <Sexp as SexpValue<String>>::first(sexp, "exclude_from_sim")
            {
                exclude == "yes"
            } else {
                false
            },
            uuid: sexp.first("uuid").unwrap(),
            props: properties(sexp),
            pins: sexp
                .query("pin")
                .map(|p| (p.get(0).unwrap(), p.first("uuid").unwrap()))
                .collect(),
        }
    }
}

impl std::convert::From<SexpTree> for Schema {
    fn from(sexp: SexpTree) -> Self {
        let mut schema = Schema::default();
        for node in sexp.root().unwrap().nodes() {
            match node.name.as_str() {
                el::UUID => schema.uuid = node.get(0).unwrap(),
                el::GENERATOR => schema.generator = node.get(0).unwrap(),
                "generator_version" => schema.generator_version = node.get(0),
                "version" => schema.version = node.get(0).unwrap(),
                el::JUNCTION => schema.junctions.push(node.into()),
                "paper" => {
                    schema.paper =
                        PaperSize::from(&<Sexp as SexpValue<String>>::get(node, 0).unwrap())
                }
                el::WIRE => schema.wires.push(node.into()),
                el::LABEL => schema.local_labels.push(node.into()),
                el::GLOBAL_LABEL => schema.global_labels.push(node.into()),
                el::NO_CONNECT => schema.no_connects.push(node.into()),
                el::TITLE_BLOCK => schema.title_block = node.into(),
                el::LIB_SYMBOLS => {
                    schema.library_symbols = node.query(el::SYMBOL).map(|s| s.into()).collect()
                }
                el::SYMBOL => schema.symbols.push(node.into()),
                _ => log::error!("unknown root node: {:?}", node.name),
            }
        }
        schema
    }
}

impl std::convert::From<&Sexp> for FpLine {
    fn from(sexp: &Sexp) -> Self {
        Self {
            start: sexp.query("start").next().unwrap().into(),
            end: sexp.query("end").next().unwrap().into(),
            layer: sexp.first("layer").unwrap(),
            //width: sexp.first("width").unwrap(),
            stroke: sexp.into(),
            locked: SexpStringList::values(sexp).contains(&"locked".to_string()),
            tstamp: sexp.first("tstamp").expect("mandatory"),
        }
    }
}

impl std::convert::From<SexpTree> for Pcb {
    fn from(sexp: SexpTree) -> Self {
        let mut pcb = Pcb::default();
        for node in sexp.root().unwrap().nodes() {
            match node.name.as_str() {
                el::UUID => pcb.uuid = node.get(0).unwrap(),
                el::SEGMENT => pcb.segments.push(node.into()),
                el::NET => pcb.nets.push(node.into()),
                el::FOOTPRINT => pcb.footprints.push(node.into()),
                _ => log::error!("unknown root node: {:?}", node.name),
            }
        }
        pcb
    }
}

impl std::convert::From<&Sexp> for Segment {
    fn from(sexp: &Sexp) -> Self {
        Self {
            start: sexp.query("start").next().unwrap().into(),
            end: sexp.query("end").next().unwrap().into(),
            width: sexp.first("width").expect("mandatory"),
            layer: sexp.first("layer").expect("mandarory"),
            locked: SexpStringList::values(sexp).contains(&"locked".to_string()),
            net: sexp.first("net").unwrap(),
            tstamp: sexp.first("tstamp").unwrap(),
        }
    }
}

impl std::convert::From<&Sexp> for Net {
    fn from(sexp: &Sexp) -> Self {
        Self {
            ordinal: sexp.get(0).expect("mandatory"),
            name: sexp.get(1).expect("mandatory"),
        }
    }
}

impl std::convert::From<&Sexp> for Pad {
    fn from(sexp: &Sexp) -> Self {
        Self {
            number: sexp.get(0).expect("mandatory"),
            pad_type: PadType::from(SexpString::get(sexp, 1).expect("mandatory")),
            shape: PadShape::from(SexpString::get(sexp, 1).expect("shape")),
            pos: sexp.into(),
            //locked: todo!(),
            size: (
                sexp.query("size").next().unwrap().get(0).unwrap(),
                sexp.query("size").nth(1).unwrap().get(0).unwrap(),
            ),
            drill: sexp.first("drill"),
            //canonical_layer_list: todo!(),
            //properties: todo!(),
            //remove_unused_layer: todo!(),
            //keep_end_layers: todo!(),
            //roundrect_rratio: todo!(),
            //chamfer_ratio: todo!(),
            //chamfer: todo!(),
            net: sexp.into(),
            tstamp: sexp.first("tstamp").expect("mandatory"),
            //pinfunction: todo!(),
            //pintype: todo!(),
            //die_length: todo!(),
            //solder_mask_margin: todo!(),
            //solder_paste_margin: todo!(),
            //solder_paste_margin_ratio: todo!(),
            //clearance: todo!(),
            //zone_connect: todo!(),
            //thermal_width: todo!(),
            //thermal_gap: todo!(),
            //custom_pad_options: todo!(),
            //custom_pad_primitives: todo!(),
        }
    }
}

impl std::convert::From<&Sexp> for Vec<pcb::GraphicItem> {
    fn from(sexp: &Sexp) -> Self {
        let mut res = Vec::new();
        for n in sexp.nodes() {
            match n.name.as_str() {
                el::FP_LINE => res.push(pcb::GraphicItem::FpLine(n.into())),
                _ => log::error!("unknwn graphic_item: {:?}", n),
            }
        }
        res
    }
}

impl std::convert::From<&Sexp> for Footprint {
    fn from(sexp: &Sexp) -> Self {
        Self {
            library_link: sexp.get(0).expect("mandatory"),
            locked: SexpStringList::values(sexp).contains(&"locked".to_string()),
            placed: SexpStringList::values(sexp).contains(&"placed".to_string()),
            layer: sexp.first("layer").expect("mandatory"),
            tedit: sexp.first("tedit"), //TODO not seen in a pcb file.
            tstamp: sexp.first("tstamp"),
            pos: sexp.into(),
            descr: sexp.first("desc"),
            tags: sexp.first("tags"),
            property: sexp
                .query("property")
                .fold(HashMap::<String, String>::new(), |mut m, s| {
                    m.insert(s.get(0).expect("mandatory"), s.get(1).expect("mandatory"));
                    m
                }),
            path: sexp.first("path"),
            autoplace_cost90: sexp.first("autoplace_cost90"), //TODO not seen in a pcb file
            autoplace_cost180: sexp.first("autoplace_cost180"), //TODO not seen in a pcb file
            solder_mask_margin: sexp.first("solder_mask_margin"), //TODO not seen in a pcb file
            solder_paste_margin: sexp.first("solder_paste_margin"), //TODO not seen in a pcb file
            solder_paste_ratio: sexp.first("solder_paste_ratio"), //TODO not seen in a pcb file
            clearance: sexp.first("clearance"),
            zone_connect: sexp.first("zone_connect"), //TODO not seen in a pcb file
            thermal_width: sexp.first("thermal_width"),
            thermal_gap: sexp.first("thermal_gap"),
            footprint_type: FootprintType::from(
                <Sexp as SexpValue<String>>::first(sexp, "attr").expect("mandatory"),
            ),
            board_only: SexpStringList::values(sexp.query("attr").next().unwrap())
                .contains(&"board_only".to_string()),
            exclude_from_pos_files: SexpStringList::values(sexp.query("attr").next().unwrap())
                .contains(&"exclude_from_pos_files".to_string()),
            exclude_from_bom: SexpStringList::values(sexp.query("attr").next().unwrap())
                .contains(&"exclude_from_bom".to_string()),
            private_layers: None,     //TODO does this exist, and why optional?
            net_tie_pad_groups: None, //TODO same as above
            graphic_items: sexp.into(),
            pads: Vec::new(),   //todo!(),
            zones: Vec::new(),  //todo!(),
            groups: Vec::new(), //todo!(),
            model_3d: None,     //TODO
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        gr::{Pt, Pts, Stroke, StrokeType, TitleBlock},
        sexp::SexpTree,
        Schema,
    };

    use super::parser::SexpParser;

    #[test]
    fn empty_schema() {
        let schema = r#"
            (kicad_sch (version 20231120) (generator "eeschema") (generator_version "8.0")
              (paper "A4")
              (lib_symbols)
              (symbol_instances)
            )"#;

        let parser = SexpParser::from(schema.to_string());
        let tree = SexpTree::from(parser.iter()).unwrap();
        let schema: Schema = tree.into();

        assert_eq!("20231120", schema.version);
        assert_eq!("eeschema", schema.generator);
        //TODO assert_eq!("8.0", schema.generator_version);
        assert_eq!("A4", schema.paper.to_string());
    }

    #[test]
    fn title_block() {
        let schema = r#"
          (title_block
            (title "summe")
            (date "2021-05-30")
            (rev "R02")
            (company "company")
            (comment 1 "schema for pcb")
            (comment 2 "DC coupled mixer")
            (comment 3 "comment 3")
            (comment 4 "License CC BY 4.0 - Attribution 4.0 International")
            (comment 5 "comment 5")
            (comment 6 "comment 6")
            (comment 7 "comment 7")
            (comment 8 "comment 8")
            (comment 9 "comment 9")
          )"#;

        let parser = SexpParser::from(schema.to_string());
        let tree = SexpTree::from(parser.iter()).unwrap();
        let tb: TitleBlock = tree.root().unwrap().into();
        assert_eq!("summe".to_string(), tb.title.unwrap());
        assert_eq!("2021-05-30".to_string(), tb.date.unwrap());
        assert_eq!("R02".to_string(), tb.revision.unwrap());
        assert_eq!("company".to_string(), tb.company_name.unwrap());
        assert_eq!(9, tb.comment.len());
        assert_eq!(
            (1, "schema for pcb".to_string()),
            *tb.comment.first().unwrap()
        );
    }

    #[test]
    fn into_stroke() {
        let schema = r#"
            (something
                (stroke
                        (width 0.1)
                        (type dash)
                )
            )
        "#;

        let parser = SexpParser::from(schema.to_string());
        let tree = SexpTree::from(parser.iter()).unwrap();
        let stroke: Stroke = tree.root().unwrap().into();

        assert_eq!(0.1, stroke.width);
        assert_eq!(StrokeType::Dash, stroke.stroke_type.unwrap());
    }

    #[test]
    fn wire_schema() {
        let schema = r#"
                (wire
                        (pts
                                (xy 163.83 107.95) (xy 163.83 110.49)
                        )
                        (stroke
                                (width 0)
                                (type default)
                        )
                        (uuid "7a8a9d59-7b8a-4c1b-ab50-9119def7e130")
                )
        )"#;

        let parser = SexpParser::from(schema.to_string());
        let tree = SexpTree::from(parser.iter()).unwrap();
        let wire: crate::schema::Wire = tree.root().unwrap().into();
        assert_eq!(
            Pts(vec![
                Pt {
                    x: 163.83,
                    y: 107.95
                },
                Pt {
                    x: 163.83,
                    y: 110.49
                },
            ]),
            wire.pts
        );

        assert_eq!(0.0, wire.stroke.width);
        assert_eq!(Some(StrokeType::Default), wire.stroke.stroke_type);
    }
}
