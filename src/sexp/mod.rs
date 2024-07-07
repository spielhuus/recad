pub mod builder;
pub mod constants;
pub mod parser;
mod writer;

use std::collections::HashMap;

use crate::{
    gr::{
        Color, Effects, Font, Justify, Pos, Pt, Pts, Stroke,
        StrokeType, TitleBlock,
    }, pcb::{self, Footprint, FootprintType, FpLine, Net, Pad, PadShape, PadType, Segment}, Error, Pcb
};

use constants::el;

pub type SexpString = dyn SexpValue<String>;
pub type SexpStringList = dyn SexpQuery<Vec<String>>;

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
    pub fn nodes(&self) -> impl Iterator<Item = &Sexp> {
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
                String::from("sexp-parse"),
                String::from("Document does not start with a start symbol."),
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

pub trait SexpQuery<E> {
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
                return Some(value == "true" || value == el::YES);
            }
        }
        Some(false)
    }
    fn get(&self, index: usize) -> Option<bool> {
        if let Some(value) = SexpStringList::values(self).get(index) {
            return Some(value == "true" || value == el::YES);
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
        let Some(s) = sexp.query(el::COLOR).next() else {
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
            panic!("no stroke found in {:?}", value);
        };
        let color: Result<Color, Error> = stroke.into();
        let stroke_type: Option<String> = stroke.first(el::TYPE);
        Stroke {
            width: stroke.first(el::WIDTH).unwrap_or(0.0),
            stroke_type: stroke_type.map(|s| StrokeType::from(s.as_str())),
            color: color.ok(), //the error get consumed and converted to None
        }
    }
}

impl std::convert::From<&Sexp> for Font {
    fn from(sexp: &Sexp) -> Self {
        let font = sexp.query(el::FONT).next().unwrap();
        let size = font.query(el::SIZE).next().unwrap();
        Font {
            face: font.first(el::FACE),
            size: (size.get(0).unwrap(), size.get(1).unwrap()),
            thickness: font.first("tickness"),
            bold: SexpStringList::values(font).contains(&el::BOLD.to_string()), //TODO is an element
            italic: if let Some(italic) = font.query(el::ITALIC).next() {
                SexpString::get(italic, 0).unwrap() == el::YES
            } else {
                SexpStringList::values(font).contains(&el::ITALIC.to_string())
            },
            line_spacing: font.first("spacing"), //TODO check name in sexp file.
            color: None,                         //TODO
        }
    }
}

fn hide(node: &Sexp) -> bool {
    let new_visible: Option<String> = node.first(el::HIDE);
    if let Some(new_visible) = new_visible {
        new_visible == el::YES
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
        let effects = sexp.query(el::EFFECTS).next().unwrap();
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








impl std::convert::From<&Sexp> for FpLine {
    fn from(sexp: &Sexp) -> Self {
        Self {
            start: sexp.query(el::START).next().unwrap().into(),
            end: sexp.query(el::END).next().unwrap().into(),
            layer: sexp.first(el::LAYER).unwrap(),
            //width: sexp.first(el::WIDTH).unwrap(),
            stroke: sexp.into(),
            locked: SexpStringList::values(sexp).contains(&"locked".to_string()),
            tstamp: sexp.first(el::TSTAMP).expect("mandatory"),
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
                _ => {}, //TODO log::error!("unknown root node: {:?}", node.name),
            }
        }
        pcb
    }
}

impl std::convert::From<&Sexp> for Segment {
    fn from(sexp: &Sexp) -> Self {
        Self {
            start: sexp.query(el::START).next().unwrap().into(),
            end: sexp.query(el::END).next().unwrap().into(),
            width: sexp.first(el::WIDTH).expect("mandatory"),
            layer: sexp.first(el::LAYER).expect("mandarory"),
            locked: SexpStringList::values(sexp).contains(&"locked".to_string()),
            net: sexp.first("net").unwrap(),
            tstamp: sexp.first(el::TSTAMP).unwrap(),
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
                sexp.query(el::SIZE).next().unwrap().get(0).unwrap(),
                sexp.query(el::SIZE).nth(1).unwrap().get(0).unwrap(),
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
            tstamp: sexp.first(el::TSTAMP).expect("mandatory"),
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
                _ => {}, //TODO log::error!("unknown graphic_item: {:?}", n),
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
            layer: sexp.first(el::LAYER).expect("mandatory"),
            tedit: sexp.first("tedit"), //TODO not seen in a pcb file.
            tstamp: sexp.first(el::TSTAMP),
            pos: sexp.into(),
            descr: sexp.first(el::DESC),
            tags: sexp.first(el::TAGS),
            property: sexp
                .query(el::PROPERTY)
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
                SexpString::first(sexp, "attr").expect("mandatory"),
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

