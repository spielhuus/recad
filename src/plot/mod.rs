//!Plot the recad drawings.
use std::{fmt, io::Write};

use crate::
    gr::{Color, Pt, Pts, Rect}
;

mod svg;
pub mod theme;

pub use svg::SvgPlotter;

///The paint for the plotter.
#[derive(Clone)]
pub struct Paint {
    pub(crate) color: Color,
    pub(crate) fill: Option<Color>,
    pub(crate) width: f32,
}

impl Paint {
    pub fn black() -> Self {
        Self {
            color: Color::black(),
            fill: None,
            width: 0.25,
        }
    }
    pub fn red() -> Self {
        Self {
            color: Color::red(),
            fill: None,
            width: 0.25,
        }
    }
    pub fn green() -> Self {
        Self {
            color: Color::green(),
            fill: None,
            width: 0.25,
        }
    }
    pub fn blue() -> Self {
        Self {
            color: Color::blue(),
            fill: None,
            width: 0.25,
        }
    }
    pub fn grey() -> Self {
        Self {
            color: Color::grey(),
            fill: None,
            width: 0.25,
        }
    }
    pub fn outline() -> Self {
        Self {
            color: Color::red(),
            fill: None,
            width: 0.08,
        }
    }
}

///The fot effects for the drawings.
pub struct FontEffects {
    pub angle: f32,
    pub anchor: String,
    pub baseline: String,
    pub face: String,
    pub size: f32,
    pub color: Color,
}

#[derive(Debug)]
//Line CAP, endings.
pub enum LineCap {
    Butt,
    Round,
    Square,
}

impl fmt::Display for LineCap {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LineCap::Butt => write!(f, "butt"),
            LineCap::Round => write!(f, "round"),
            LineCap::Square => write!(f, "square"),
        }
    }
}

pub trait Plotter {
    fn open(&self);

    ///set the view box.
    fn set_view_box(&mut self, rect: Rect);

    ///Move the path cursor to position.
    fn move_to(&mut self, pt: Pt);
    ///Draw a line to position.
    fn line_to(&mut self, pt: Pt);
    ///Close the path.
    fn close(&mut self);
    ///Sroke the path.
    fn stroke(&mut self, stroke: Paint);

    ///Draw a rectancle with stroke.
    fn rect(&mut self, r: Rect, stroke: Paint);
    fn arc(&mut self, center: Pt, radius: f32, stroke: Paint);
    fn circle(&mut self, center: Pt, radius: f32, stroke: Paint);
    fn text(&mut self, text: &str, pt: Pt, effects: FontEffects);

    ///Draw a polyline with the given Pts.
    fn polyline(&mut self, pts: Pts, stroke: Paint);

    ///Write the result to a Writer.
    fn write<W: Write>(self, writer: &mut W) -> std::io::Result<()>;
}

