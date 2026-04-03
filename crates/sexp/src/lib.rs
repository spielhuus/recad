pub mod builder;
pub mod parser;
mod writer;

use std::borrow::Cow;

use types::{
    constants::el,
    error::RecadError,
    gr::{Arc, Circle, Color, Effects, FillType, Font, Justify, Line, Polyline, Pos, Pt, Pts, Rectangle, Stroke, StrokeType, TitleBlock},
};

use crate::builder::Builder;


pub trait SexpWrite {
    fn write(&self, builder: &mut Builder) -> Result<(), RecadError>;
}

/// Helper to unescape strings parsed from the S-expression file.
fn unescape_string(s: &str) -> Cow<'_, str> {
    if !s.contains('\\') {
        return Cow::Borrowed(s);
    }

    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(escaped) = chars.next() {
                match escaped {
                    'n' => result.push('\n'),
                    'r' => result.push('\r'),
                    't' => result.push('\t'),
                    '\\' => result.push('\\'),
                    '"' => result.push('"'),
                    _ => {
                        result.push('\\');
                        result.push(escaped);
                    }
                }
            } else {
                result.push('\\');
            }
        } else {
            result.push(c);
        }
    }
    Cow::Owned(result)
}

///The sexp element types.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum SexpAtom<'a> {
    ///Child node.
    Node(Sexp<'a>),
    ///Value
    Value(Cow<'a, str>),
    ///Text surrounded with quotes.
    Text(Cow<'a, str>),
}

///Sexp Element
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Sexp<'a> {
    ///name of the node
    pub name: Cow<'a, str>,
    ///Children of the node.
    nodes: Vec<SexpAtom<'a>>,
    pub line: usize,
    pub column: usize,
}

impl<'a> Sexp<'a> {
    ///Create a new sexp node with name.
    pub fn from(name: Cow<'a, str>, line: usize, column: usize) -> Self {
        Sexp {
            name,
            nodes: Vec::new(),
            line,
            column,
        }
    }

    ///get the nodes.
    pub fn nodes(&self) -> impl Iterator<Item = &Sexp<'_>> {
        self.nodes.iter().filter_map(|n| {
            if let SexpAtom::Node(node) = n {
                Some(node)
            } else {
                None
            }
        })
    }

    ///query child nodes for elements by name.
    pub fn query<'b>(&'b self, q: &'b str) -> impl Iterator<Item = &'b Sexp<'a>> {
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

    pub fn value_iter(&self) -> impl Iterator<Item = &str> {
        self.nodes.iter().filter_map(|n| match n {
            SexpAtom::Value(v) | SexpAtom::Text(v) => Some(v.as_ref()),
            _ => None,
        })
    }
}

///Sexp document.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SexpTree<'a> {
    tree: Sexp<'a>,
}

impl<'a> SexpTree<'a> {
    ///parse a sexp document for SexpParser Iterator.
    pub fn from<I>(mut iter: I) -> Result<Self, RecadError>
    where
        I: Iterator<Item = State<'a>>,
    {
        let mut stack: Vec<Sexp> = Vec::new();
        if let Some(State::StartSymbol(name, line, col)) = iter.next() {
            stack.push(Sexp::from(Cow::Borrowed(name), line, col));
        } else {
            return Err(RecadError::Sexp {
                line: 0,
                col: 0,
                msg: "Document does not start with a start symbol.".to_string(),
            });
        };
        loop {
            match iter.next() {
                Some(State::Values(value, _line, _col)) => {
                    let len = stack.len();
                    if let Some(parent) = stack.get_mut(len - 1) {
                        parent.nodes.push(SexpAtom::Value(Cow::Borrowed(value)));
                    }
                }
                Some(State::Text(value, _line, _col)) => {
                    let len = stack.len();
                    if let Some(parent) = stack.get_mut(len - 1) {
                        parent.nodes.push(SexpAtom::Text(unescape_string(value)));
                    }
                }
                Some(State::EndSymbol(_line, _col)) => {
                    let len = stack.len();
                    if len > 1 {
                        let i = stack.pop().unwrap();
                        if let Some(parent) = stack.get_mut(len - 2) {
                            parent.nodes.push(SexpAtom::Node(i));
                        }
                    }
                }
                Some(State::StartSymbol(name, line, col)) => {
                    stack.push(Sexp::from(Cow::Borrowed(name), line, col));
                }
                None => break,
            }
        }
        let i = stack.pop().unwrap();
        Ok(SexpTree { tree: i })
    }

    ///Get the root element.
    pub fn root(&self) -> &Sexp<'_> {
        &self.tree
    }
}

///Get a single sexp value.
///
///Get a sexp value by name or index.
///There could be multiple values, the first is returned.
pub trait SexpValue<E> {
    fn first(&self, q: &str) -> Result<Option<E>, RecadError>;
    fn get(&self, index: usize) -> Result<Option<E>, RecadError>;
}

impl<'a> SexpValue<String> for Sexp<'a> {
    fn first(&self, q: &str) -> Result<Option<String>, RecadError> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = node.value_iter().next() {
                return Ok(Some(value.to_string()));
            }
        }
        Ok(None)
    }

    fn get(&self, index: usize) -> Result<Option<String>, RecadError> {
        if let Some(value) = self.value_iter().nth(index) {
            return Ok(Some(value.to_string()));
        }
        Ok(None)
    }
}

impl<'a> SexpValue<u8> for Sexp<'a> {
    fn first(&self, q: &str) -> Result<Option<u8>, RecadError> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = node.value_iter().next() {
                match value.parse::<u8>() {
                    Ok(val) => return Ok(Some(val)),
                    Err(_) => {
                        return Err(RecadError::Sexp {
                            line: self.line,
                            col: self.column,
                            msg: format!("Failed to parse '{}' as u8 for {}", value, q),
                        })
                    }
                }
            }
        }
        Ok(None)
    }

    fn get(&self, index: usize) -> Result<Option<u8>, RecadError> {
        if let Some(value) = self.value_iter().nth(index) {
            match value.parse::<u8>() {
                Ok(val) => return Ok(Some(val)),
                Err(_) => {
                    return Err(RecadError::Sexp {
                        line: self.line,
                        col: self.column,
                        msg: format!("Failed to parse '{}' as u8 at {}", value, index),
                    })
                }
            }
        }
        Ok(None)
    }
}

impl<'a> SexpValue<u32> for Sexp<'a> {
    fn first(&self, q: &str) -> Result<Option<u32>, RecadError> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = node.value_iter().next() {
                match value.parse::<u32>() {
                    Ok(val) => return Ok(Some(val)),
                    Err(_) => {
                        return Err(RecadError::Sexp {
                            line: self.line,
                            col: self.column,
                            msg: format!("Failed to parse '{}' as u32 for {}", value, q),
                        })
                    }
                }
            }
        }
        Ok(None)
    }

    fn get(&self, index: usize) -> Result<Option<u32>, RecadError> {
        if let Some(value) = self.value_iter().nth(index) {
            match value.parse::<u32>() {
                Ok(val) => return Ok(Some(val)),
                Err(_) => {
                    return Err(RecadError::Sexp {
                        line: self.line,
                        col: self.column,
                        msg: format!("Failed to parse '{}' as u32 at index {}", value, index),
                    })
                }
            }
        }
        Ok(None)
    }
}

impl<'a> SexpValue<bool> for Sexp<'a> {
    fn first(&self, q: &str) -> Result<Option<bool>, RecadError> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = node.value_iter().next() {
                return Ok(Some(value == "true" || value == el::YES));
            }
            return Ok(Some(true));
        }

        Ok(None)
    }

    fn get(&self, index: usize) -> Result<Option<bool>, RecadError> {
        if let Some(value) = self.value_iter().nth(index) {
            return Ok(Some(value == "true" || value == el::YES));
        }

        Ok(None)
    }
}

impl<'a> SexpValue<f32> for Sexp<'a> {
    fn first(&self, q: &str) -> Result<Option<f32>, RecadError> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = node.value_iter().next() {
                return match value.parse::<f32>() {
                    Ok(v) => Ok(Some(v)),
                    Err(_) => Err(RecadError::Sexp {
                        line: node.line,
                        col: node.column,
                        msg: format!("Failed to parse '{}' as f32", value),
                    }),
                };
            }
        }
        Ok(None)
    }

    fn get(&self, index: usize) -> Result<Option<f32>, RecadError> {
        if let Some(value) = self.value_iter().nth(index) {
            return match value.parse::<f32>() {
                Ok(v) => Ok(Some(v)),
                Err(_) => Err(RecadError::Sexp {
                    line: self.line,
                    col: self.column,
                    msg: format!("Failed to parse '{}' as f32 at index {}", value, index),
                }),
            };
        }
        Ok(None)
    }
}

impl<'a> SexpValue<f64> for Sexp<'a> {
    fn first(&self, q: &str) -> Result<Option<f64>, RecadError> {
        if let Some(node) = self.query(q).next() {
            if let Some(value) = node.value_iter().next() {
                return match value.parse::<f64>() {
                    Ok(v) => Ok(Some(v)),
                    Err(_) => Err(RecadError::Sexp {
                        line: node.line,
                        col: node.column,
                        msg: format!("Failed to parse '{}' as f64", value),
                    }),
                };
            }
        }
        Ok(None)
    }

    fn get(&self, index: usize) -> Result<Option<f64>, RecadError> {
        if let Some(value) = self.value_iter().nth(index) {
            return match value.parse::<f64>() {
                Ok(v) => Ok(Some(v)),
                Err(_) => Err(RecadError::Sexp {
                    line: self.line,
                    col: self.column,
                    msg: format!("Failed to parse '{}' as f64 at index {}", value, index),
                }),
            };
        }
        Ok(None)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum State<'a> {
    StartSymbol(&'a str, usize, usize),
    EndSymbol(usize, usize),
    Values(&'a str, usize, usize),
    Text(&'a str, usize, usize),
}

#[derive(Debug, PartialEq, Clone)]
enum IntState {
    NotStarted,
    Symbol,
    Values,
    BeforeEndSymbol(usize, usize),
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Pos {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let at = sexp.require_node(el::AT)?;
        Ok(Pos {
            x: at.require_get(0)?,
            y: at.require_get(1)?,
            angle: at.get(2)?.unwrap_or(0.0),
        })
    }
}

pub trait SexpExt {
    /// Requires a child node to exist, returning a RecadError if missing.
    fn require_node(&self, q: &str) -> Result<&Sexp<'_>, RecadError>;
}

impl<'a> SexpExt for Sexp<'a> {
    fn require_node(&self, q: &str) -> Result<&Sexp<'_>, RecadError> {
        self.nodes()
            .find(|node| node.name == q)
            .ok_or_else(|| RecadError::Sexp {
                line: self.line,
                col: self.column,
                msg: format!("Missing required node '{}'", q),
            })
    }
}

pub trait SexpValueExt {
    /// Requires a positional value to exist
    fn require_get<T>(&self, index: usize) -> Result<T, RecadError>
    where
        Self: SexpValue<T>;

    /// Requires a first query value to exist
    fn require_first<T>(&self, q: &str) -> Result<T, RecadError>
    where
        Self: SexpValue<T>;
}

impl<'a> SexpValueExt for Sexp<'a> {
    fn require_get<T>(&self, index: usize) -> Result<T, RecadError>
    where
        Self: SexpValue<T>,
    {
        <Self as SexpValue<T>>::get(self, index)?.ok_or_else(|| RecadError::Sexp {
            line: self.line,
            col: self.column,
            msg: format!("Missing required value at index {}", index),
        })
    }

    fn require_first<T>(&self, q: &str) -> Result<T, RecadError>
    where
        Self: SexpValue<T>,
    {
        <Self as SexpValue<T>>::first(self, q)?.ok_or_else(|| RecadError::Sexp {
            line: self.line,
            col: self.column,
            msg: format!("Missing required value for '{}'", q),
        })
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Pt {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let x: f64 = sexp.require_get(0)?;
        let y: f64 = sexp.require_get(1)?;
        Ok(Pt { x, y })
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Pts {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let mut pts: Vec<Pt> = Vec::new();
        for pt in sexp.query(el::PTS) {
            for xy in pt.query(el::XY) {
                let x: f64 = xy.require_get(0)?;
                let y: f64 = xy.require_get(1)?;
                pts.push(Pt { x, y });
            }
        }
        Ok(Pts(pts))
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Color {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let s = sexp.require_node(el::COLOR)?;

        let r: Option<u8> = s.get(0)?;
        let g: Option<u8> = s.get(1)?;
        let b: Option<u8> = s.get(2)?;
        let a: Option<f64> = s.get(3)?;

        if let (Some(r), Some(g), Some(b)) = (r, g, b) {
            let alpha = a.unwrap_or(1.0);
            if r == 0 && g == 0 && b == 0 && alpha == 0.0 {
                Ok(Color::None)
            } else {
                Ok(Color::Rgba(r, g, b, (alpha * 255.0).round() as u8))
            }
        } else {
            Err(RecadError::Sexp {
                line: sexp.line,
                col: sexp.column,
                msg: format!("Incomplete Color value: {:?}", sexp),
            })
        }
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Stroke {
    type Error = RecadError;
    fn try_from(value: &Sexp) -> Result<Self, Self::Error> {
        let Some(stroke) = value.query(el::STROKE).next() else {
            return Ok(Stroke::default());
        };
        let color: Option<Color> = stroke.try_into().ok();
        let stroke_type: Option<String> = stroke.first(el::TYPE)?;
        Ok(Stroke {
            width: stroke.first(el::WIDTH)?.unwrap_or(0.0),
            stroke_type: stroke_type.map(|s| StrokeType::from(s.as_str())),
            color,
        })
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Font {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let font = sexp.require_node(el::FONT)?;
        let size = font.require_node(el::SIZE)?;
        Ok(Font {
            face: font.first(el::FACE)?,
            size: (size.require_get(0)?, size.require_get(1)?),
            thickness: font.first("thickness")?,
            bold: font.value_iter().any(|v| v == el::BOLD),
            italic: if let Some(italic) = font.query(el::ITALIC).next() {
                italic.value_iter().next() == Some(el::YES)
            } else {
                font.value_iter().any(|v| v == el::ITALIC)
            },
            line_spacing: font.first("spacing")?,
            color: None,
        })
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for FillType {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        if let Some(fill) = sexp.query(el::FILL).next() {
            let fill_type: Option<String> = fill.first(el::TYPE)?;
            if let Some(filltype) = fill_type {
                if filltype == el::COLOR {
                    Ok(FillType::Color(fill.try_into()?))
                } else {
                    Ok(FillType::from(filltype.as_str()))
                }
            } else {
                Ok(FillType::None)
            }
        } else {
            Ok(FillType::None)
        }
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Arc {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Arc {
            start: sexp.require_node(el::START)?.try_into()?,
            mid: sexp.require_node(el::MID)?.try_into()?,
            end: sexp.require_node(el::END)?.try_into()?,
            stroke: sexp.try_into()?,
            fill: sexp.try_into()?,
            uuid: sexp.first(el::UUID)?,
        })
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Circle {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Circle {
            center: sexp.require_node(el::CENTER)?.try_into()?,
            radius: sexp.require_first(el::RADIUS)?,
            stroke: sexp.try_into()?,
            fill: sexp.try_into()?,
            uuid: sexp.first(el::UUID)?,
        })
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Polyline {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Polyline {
            pts: sexp.try_into()?,
            stroke: sexp.try_into()?,
            fill: if sexp.query(el::FILL).next().is_some() {
                Some(FillType::try_from(sexp)?)
            } else {
                None
            },
            uuid: sexp.first(el::UUID)?,
        })
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Line {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Line {
            pts: sexp.try_into()?,
            stroke: sexp.try_into()?,
            fill: sexp.try_into()?,
            uuid: sexp.first(el::UUID)?,
        })
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Rectangle {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(Rectangle {
            start: sexp.require_node(el::START)?.try_into()?,
            end: sexp.require_node(el::END)?.try_into()?,
            stroke: sexp.try_into()?,
            fill: sexp.try_into()?,
            uuid: sexp.first(el::UUID)?,
        })
    }
}

fn hide(node: &Sexp) -> bool {
    let new_visible: Option<String> = node.first(el::HIDE).unwrap_or_default();
    if let Some(new_visible) = new_visible {
        new_visible == el::YES
    } else {
        node.value_iter().any(|v| v == el::HIDE)
    }
}

fn justify(node: &Sexp) -> Result<Vec<Justify>, RecadError> {
    if let Some(j) = node.query(el::JUSTIFY).next() {
        j.value_iter()
            .map(|v| Justify::try_from(v.to_string()))
            .collect()
    } else {
        Ok(Vec::new())
    }
}

impl<'a> std::convert::TryFrom<&Sexp<'a>> for Effects {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        let effects = sexp.require_node(el::EFFECTS)?;
        Ok(Effects {
            justify: justify(effects)?,
            hide: hide(effects),
            font: effects.try_into()?,
            ..Default::default()
        })
    }
}

///extract a title block section, root must the the title_block itself.
impl<'a> std::convert::TryFrom<&Sexp<'a>> for TitleBlock {
    type Error = RecadError;
    fn try_from(sexp: &Sexp) -> Result<Self, Self::Error> {
        Ok(TitleBlock {
            title: sexp.first(el::TITLE_BLOCK_TITLE)?,
            date: sexp.first(el::TITLE_BLOCK_DATE)?,
            revision: sexp.first(el::TITLE_BLOCK_REV)?,
            company_name: sexp.first(el::TITLE_BLOCK_COMPANY)?,
            comment: sexp
                .query(el::TITLE_BLOCK_COMMENT)
                .map(|c| -> Result<_, RecadError> { Ok((c.require_get(0)?, c.require_get(1)?)) })
                .collect::<Result<Vec<_>, RecadError>>()?,
        })
    }
}
