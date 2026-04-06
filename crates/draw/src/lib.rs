//! Drawers for building schemas.
use std::collections::HashMap;

use models::schema::Property;
use models::symbols::LibrarySymbol;
use models::{geometry::Bbox, schema::SchemaItem};
use types::{
    constants::el,
    error::RecadError,
    gr::{Effects, Justify, Pos, Pt, Pts, Rect},
};

const MARGIN: f64 = 1.27;
const LINE_SPACING: f64 = 4.0 * 0.254;

pub trait Drawer<Command> {
    type Output;
    fn draw(&mut self, cmd: Command) -> Result<Self::Output, RecadError>;
}

/// Label position
#[derive(Debug, Clone, PartialEq)]
pub enum LabelPosition {
    North,
    South,
    West,
    East,
    Offset(f64, f64),
}

impl TryFrom<String> for LabelPosition {
    type Error = RecadError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.to_lowercase().as_str() {
            "north" => Ok(LabelPosition::North),
            "south" => Ok(LabelPosition::South),
            "west" => Ok(LabelPosition::West),
            "east" => Ok(LabelPosition::East),
            _ => Err(RecadError::Schema(format!(
                "unknown label direction: {}",
                value
            ))),
        }
    }
}

/// Attributes for the elements.
#[derive(Debug, Clone, PartialEq)]
pub enum Attribute {
    Anchor(String),
    Direction(Direction),
    Id(String),
    Mirror(String),
    Length(f64),
    Rotate(f64),
    Tox(At),
    Toy(At),
    Property(HashMap<String, String>), // key, value
    Dot(Vec<DotPosition>),
    At(At),
    Unit(u8),
    LabelPosition(LabelPosition),
}

/// Dot position
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum DotPosition {
    Start,
    End,
}

/// Direction enum
#[derive(Debug, Clone, PartialEq, Default)]
pub enum Direction {
    #[default]
    Left,
    Right,
    Up,
    Down,
}

/// Draw a Wire from the actual position to position.
#[derive(Debug, Clone, PartialEq)]
pub struct To {
    /// The Attributes.
    pub attributes: Vec<Attribute>,
}

impl To {
    /// Create a new empty `To`.
    pub fn new() -> Self {
        Self {
            attributes: Vec::new(),
        }
    }

    pub fn push(&mut self, attr: Attribute) {
        self.attributes.push(attr);
    }

    pub fn at(&self) -> Option<At> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::At(at) = attr {
                Some(at.clone())
            } else {
                None
            }
        })
    }

    pub fn anchor(&self) -> Option<String> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::Anchor(pin) = attr {
                Some(pin.to_string())
            } else {
                None
            }
        })
    }

    pub fn mirror(&self) -> Option<String> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::Mirror(m) = attr {
                Some(m.to_string())
            } else {
                None
            }
        })
    }

    pub fn angle(&self) -> Option<f64> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::Rotate(angle) = attr {
                Some(*angle)
            } else {
                None
            }
        })
    }

    pub fn length(&self) -> Option<f64> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::Length(length) = attr {
                Some(*length)
            } else {
                None
            }
        })
    }

    pub fn direction(&self) -> &Direction {
        self.attributes
            .iter()
            .find_map(|attr| {
                if let Attribute::Direction(dir) = attr {
                    Some(dir)
                } else {
                    None
                }
            })
            .unwrap_or(&Direction::Left)
    }

    pub fn tox(&self) -> Option<&At> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::Tox(at) = attr {
                Some(at)
            } else {
                None
            }
        })
    }

    pub fn toy(&self) -> Option<&At> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::Toy(at) = attr {
                Some(at)
            } else {
                None
            }
        })
    }

    pub fn unit(&self) -> Option<u8> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::Unit(unit) = attr {
                Some(*unit)
            } else {
                None
            }
        })
    }

    pub fn dot(&self) -> Option<&Vec<DotPosition>> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::Dot(dot) = attr {
                Some(dot)
            } else {
                None
            }
        })
    }

    pub fn properties(&self) -> Option<&HashMap<String, String>> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::Property(props) = attr {
                Some(props)
            } else {
                None
            }
        })
    }

    pub fn label_position(&self) -> Option<&LabelPosition> {
        self.attributes.iter().find_map(|attr| {
            if let Attribute::LabelPosition(pos) = attr {
                Some(pos)
            } else {
                None
            }
        })
    }
}

impl Default for To {
    fn default() -> Self {
        Self::new()
    }
}

/// Represents different position identifiers
#[derive(Debug, Clone, PartialEq)]
pub enum At {
    /// A simple point with x and y in mm.
    Pt(Pt),
    /// The position of a ```Pin``` by reference and pin number.
    Pin(String, String),
    /// A Junction by id
    Junction(String),
}

impl Default for At {
    fn default() -> Self {
        At::Pt(Pt { x: 0.0, y: 0.0 })
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
struct PinDirections {
    up: usize,
    down: usize,
    left: usize,
    right: usize,
}

impl PinDirections {
    fn single_pin(&self) -> bool {
        let mut count = 0;
        if self.up > 0 {
            count += 1;
        }
        if self.down > 0 {
            count += 1;
        }
        if self.left > 0 {
            count += 1;
        }
        if self.right > 0 {
            count += 1;
        }
        count == 1
    }
    fn free_up(&self) -> bool {
        self.up == 0
    }
    fn free_down(&self) -> bool {
        self.down == 0
    }
    fn free_left(&self) -> bool {
        self.left == 0
    }
    fn free_right(&self) -> bool {
        self.right == 0
    }
}

// ==============================================================================
// SCHEMA BUILDER
// ==============================================================================

#[derive(Default)]
pub struct SchemaBuilder {
    pub schema: models::schema::Schema,
    pub last_pos: At,
    pub grid: f64,
}

impl SchemaBuilder {
    pub fn new(project: &str) -> Self {
        Self {
            schema: models::schema::Schema::new(project, None),
            last_pos: At::default(),
            grid: 2.54,
        }
    }

    pub fn move_to(&mut self, at: At) {
        self.last_pos = at;
    }

    // resolve positions
    pub fn get_pt(&self, at: &At) -> Pt {
        match at {
            At::Pt(pt) => *pt,
            At::Pin(ref_des, pin_num) => {
                // Determine exactly which unit this pin belongs to (e.g., Unit 1, 2, or 3)
                if let Some(unit) = self.schema.pin_unit(ref_des, pin_num) {
                    // Fetch the specific unit instance instead of just the first one found
                    if let Some(symbol) = self.schema.symbol(ref_des, unit) {
                        if let Some(lib) = self.schema.library_symbol(&symbol.lib_id) {
                            if let Some(pin) = lib.pin(pin_num) {
                                return models::geometry::pin_position(symbol, pin);
                            }
                        }
                    }
                }
                Pt { x: 0.0, y: 0.0 }
            }
            At::Junction(id) => {
                for item in &self.schema.items {
                    if let models::schema::SchemaItem::Junction(j) = item {
                        if j.uuid == *id {
                            return Pt {
                                x: j.pos.x,
                                y: j.pos.y,
                            };
                        }
                    }
                }
                Pt { x: 0.0, y: 0.0 }
            }
        }
    }

    /// Generates the next available reference for a given prefix (e.g., "R", "C").
    /// If "R1" and "R2" exist, next_reference("R") returns "R3".
    pub fn next_reference(&self, prefix: &str) -> String {
        let max = self
            .schema
            .items
            .iter()
            .filter_map(|item| {
                if let SchemaItem::Symbol(s) = item {
                    if let Some(prop) = s.property(el::PROPERTY_REFERENCE) {
                        prop.strip_prefix(prefix)?.parse::<u32>().ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .max();

        match max {
            Some(m) => format!("{}{}", prefix, m + 1),
            None => format!("{}1", prefix),
        }
    }

    pub fn last_reference(&self, prefix: &str) -> Option<String> {
        self.schema.items.iter().rev().find_map(|item| {
            if let SchemaItem::Symbol(s) = item {
                if let Some(r) = s.property(el::PROPERTY_REFERENCE) {
                    if r.starts_with(prefix) {
                        return Some(r);
                    }
                }
            }
            None
        })
    }

    /// Extracted helper from previously commented `impl Schema`
    fn apply_symbol_metadata(
        &self,
        new_symbol: &mut models::schema::Symbol,
        cmd_symbol: &Symbol,
        lib: &models::symbols::LibrarySymbol,
        unit: u8,
    ) -> Result<(), RecadError> {
        let new_reference = &cmd_symbol.reference;

        // Handle References (#PWR, #FLG, or Standard)
        let reference = if new_reference.starts_with("#PWR") {
            self.schema.next_power()
        } else if new_reference.starts_with("#FLG") {
            self.schema.next_flag()
        } else {
            new_reference.to_string()
        };

        new_symbol.set_property(el::PROPERTY_REFERENCE, reference.as_str());
        new_symbol.set_property(el::PROPERTY_VALUE, cmd_symbol.value.as_str());

        // Map Pins from library to schematic instance
        for pin in &lib.pins(unit) {
            new_symbol
                .pins
                .push((pin.number.name.clone(), models::uuid!()));
        }

        // Handle custom properties (hidden, etc.)
        if let Some(props) = cmd_symbol.attrs.properties() {
            for (key, value) in props {
                if let Some(prop) = new_symbol.props.iter_mut().find(|p| p.key == *key) {
                    prop.value = value.to_string();
                } else {
                    new_symbol.props.push(Property {
                        key: key.to_string(),
                        value: value.to_string(),
                        effects: Effects {
                            hide: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                }
            }
        }

        // Define Instance path
        new_symbol.instances = vec![models::schema::Instance {
            project: self.schema.project.to_string(),
            path: format!("/{}", self.schema.uuid),
            reference,
            unit,
        }];

        Ok(())
    }

    /// Helper for checking Bounding Box Collisions
    fn rect_intersect(a: &Rect, b: &Rect) -> bool {
        let margin = 0.1; // Small tolerance
        !(a.end.x <= b.start.x + margin
            || a.start.x >= b.end.x - margin
            || a.end.y <= b.start.y + margin
            || a.start.y >= b.end.y - margin)
    }

    fn pin_directions(
        &self,
        lib_symbol: &LibrarySymbol,
        symbol: &models::schema::Symbol,
        sym_bbox: &Rect,
    ) -> PinDirections {
        let mut pin_directions = PinDirections::default();
        let cx = sym_bbox.start.x + (sym_bbox.end.x - sym_bbox.start.x) / 2.0;
        let cy = sym_bbox.start.y + (sym_bbox.end.y - sym_bbox.start.y) / 2.0;
        for pin in lib_symbol.pins(symbol.unit) {
            let pin_global_pt = models::geometry::pin_position(symbol, pin);
            let dx = pin_global_pt.x - cx;
            let dy = pin_global_pt.y - cy;

            // Determine the primary axis of the pin relative to the center
            if dx.abs() > dy.abs() {
                if dx > 0.0 {
                    pin_directions.right += 1;
                } else {
                    pin_directions.left += 1;
                }
            } else if dy > 0.0 {
                pin_directions.down += 1;
            } else {
                pin_directions.up += 1;
            }
        }
        pin_directions
    }

    fn bbox(&self) -> Result<Vec<Rect>, RecadError> {
        let mut static_bboxes = Vec::new();

        for item in &self.schema.items {
            match item {
                models::schema::SchemaItem::Wire(w) => {
                    if let Ok(bbox) = w.outline() {
                        static_bboxes.push(bbox);
                    }
                }
                models::schema::SchemaItem::Junction(j) => {
                    if let Ok(bbox) = j.outline() {
                        static_bboxes.push(bbox);
                    }
                }
                models::schema::SchemaItem::NoConnect(nc) => {
                    if let Ok(bbox) = nc.outline() {
                        static_bboxes.push(bbox);
                    }
                }
                models::schema::SchemaItem::LocalLabel(ll) => {
                    if let Ok(bbox) = ll.outline() {
                        static_bboxes.push(bbox);
                    }
                }
                models::schema::SchemaItem::GlobalLabel(gl) => {
                    if let Ok(bbox) = gl.outline() {
                        static_bboxes.push(bbox);
                    }
                }
                models::schema::SchemaItem::Symbol(s) => {
                    let lib_symbol = self.schema.library_symbol(&s.lib_id).ok_or_else(|| {
                        RecadError::Schema(format!("Library symbol not found: {}", s.lib_id))
                    })?;
                    if let Ok(bbox) = s.outline(lib_symbol) {
                        static_bboxes.push(bbox);
                    }
                }
                _ => {
                    spdlog::warn!("Draw::finalize: unhandled SchemaItem: {:?}", item);
                }
            }
        }
        Ok(static_bboxes)
    }

    /// Triggered right before saving to auto-place deferred items
    pub fn finalize(&mut self) -> Result<&models::schema::Schema, RecadError> {
        let static_bboxes = self.bbox()?;

        let num_items = self.schema.items.len();
        for i in 0..num_items {
            let mut props_updates: Vec<Property> = Vec::new();
            let symbol = if let models::schema::SchemaItem::Symbol(s) = &self.schema.items[i] {
                s
            } else {
                continue; // Skip non-symbols
            };

            let lib_symbol = self.schema.library_symbol(&symbol.lib_id).ok_or_else(|| {
                RecadError::Schema(format!("Library symbol not found: {}", symbol.lib_id))
            })?;

            let mut sym_bbox = symbol.outline(lib_symbol).unwrap_or_default();
            // Ensure start coordinates are always smaller than end coordinates
            if sym_bbox.start.x > sym_bbox.end.x {
                std::mem::swap(&mut sym_bbox.start.x, &mut sym_bbox.end.x);
            }
            if sym_bbox.start.y > sym_bbox.end.y {
                std::mem::swap(&mut sym_bbox.start.y, &mut sym_bbox.end.y);
            }

            let pin_directions = self.pin_directions(lib_symbol, symbol, &sym_bbox);

            let props_to_place: Vec<&Property> = symbol
                .props
                .iter()
                .filter(|p| p.visible() && !p.value.is_empty())
                .collect();

            if props_to_place.is_empty() {
                continue; // Nothing to place, move to next item
            }

            // Determine upright text angle based on standard KiCad readability (0 or 90)
            let sym_angle_norm = ((symbol.pos.angle.round() as i32 % 360) + 360) % 360;
            let text_angle = if sym_angle_norm == 90 || sym_angle_norm == 270 {
                90.0
            } else {
                0.0
            };

            let bbox_properties: Vec<Rect> = props_to_place
                .iter()
                .map(|p| (*p).clone())
                .map(|mut p| {
                    p.pos.angle = 0.0;
                    p.outline()
                })
                .collect::<Result<Vec<Rect>, _>>()?;

            let mut max_width = 0.0_f64;
            let mut block_height = 0.0_f64;
            let mut heights = vec![];
            let mut widths = vec![]; // NEW: Track widths
            for (i, bbox) in bbox_properties.iter().enumerate() {
                let width = (bbox.end.x - bbox.start.x).abs();
                let height = (bbox.end.y - bbox.start.y).abs();
                heights.push(height);
                widths.push(width); // NEW
                max_width = max_width.max(width);
                block_height += height;
                if i < bbox_properties.len() - 1 {
                    block_height += LINE_SPACING;
                }
            }

            // determine the prioritized order of placement directions
            let mut preferred_directions = vec![];

            if pin_directions.single_pin() {
                if !pin_directions.free_up() {
                    // Pin is UP (like GND). Text should go DOWN.
                    preferred_directions = vec![
                        Direction::Down,
                        Direction::Right,
                        Direction::Left,
                        Direction::Up,
                    ];
                } else if !pin_directions.free_down() {
                    // Pin is DOWN. Text should go UP.
                    preferred_directions = vec![
                        Direction::Up,
                        Direction::Right,
                        Direction::Left,
                        Direction::Down,
                    ];
                } else if !pin_directions.free_left() {
                    // Pin is LEFT. Text should go RIGHT.
                    preferred_directions = vec![
                        Direction::Right,
                        Direction::Up,
                        Direction::Down,
                        Direction::Left,
                    ];
                } else if !pin_directions.free_right() {
                    // Pin is RIGHT. Text should go LEFT.
                    preferred_directions = vec![
                        Direction::Left,
                        Direction::Up,
                        Direction::Down,
                        Direction::Right,
                    ];
                }
            } else {
                if pin_directions.free_up() {
                    preferred_directions.push(Direction::Up);
                }
                if pin_directions.free_right() {
                    preferred_directions.push(Direction::Right);
                }
                if pin_directions.free_down() {
                    preferred_directions.push(Direction::Down);
                }
                if pin_directions.free_left() {
                    preferred_directions.push(Direction::Left);
                }
            }

            #[derive(Clone)]
            struct Placement {
                dir: Direction,
                rect: Rect,
                start_x: f64,
                start_y: f64,
            }

            let sym_center_x = sym_bbox.start.x + ((sym_bbox.end.x - sym_bbox.start.x) / 2.0);
            let sym_center_y = sym_bbox.start.y + ((sym_bbox.end.y - sym_bbox.start.y) / 2.0);

            // search for the best collision-free position
            let mut best_placement: Option<Placement> = None;
            let mut first_choice: Option<Placement> = None;

            for dir in &preferred_directions {
                let (start_x, start_y, rect) = match dir {
                    Direction::Up => {
                        let sx = sym_center_x - (max_width / 2.0); // Make sx the Left Edge
                        let sy = sym_bbox.start.y - MARGIN - block_height;
                        let rect = Rect {
                            start: Pt { x: sx, y: sy },
                            end: Pt {
                                x: sx + max_width,
                                y: sy + block_height,
                            },
                        };
                        (sx, sy, rect)
                    }
                    Direction::Down => {
                        let sx = sym_center_x - (max_width / 2.0); // Make sx the Left Edge
                        let sy = sym_bbox.end.y + MARGIN;
                        let rect = Rect {
                            start: Pt { x: sx, y: sy },
                            end: Pt {
                                x: sx + max_width,
                                y: sy + block_height,
                            },
                        };
                        (sx, sy, rect)
                    }
                    Direction::Left => {
                        let sx = sym_bbox.start.x - MARGIN - max_width; // Make sx the Left Edge
                        let sy = sym_center_y - (block_height / 2.0);
                        let rect = Rect {
                            start: Pt { x: sx, y: sy },
                            end: Pt {
                                x: sx + max_width,
                                y: sy + block_height,
                            },
                        };
                        (sx, sy, rect)
                    }
                    Direction::Right => {
                        let sx = sym_bbox.end.x + MARGIN; // Make sx the Left Edge
                        let sy = sym_center_y - (block_height / 2.0);
                        let rect = Rect {
                            start: Pt { x: sx, y: sy },
                            end: Pt {
                                x: sx + max_width,
                                y: sy + block_height,
                            },
                        };
                        (sx, sy, rect)
                    }
                };

                let placement = Placement {
                    dir: dir.clone(),
                    rect,
                    start_x,
                    start_y,
                };

                if first_choice.is_none() {
                    first_choice = Some(placement.clone());
                }

                // Check for collision
                let collision = static_bboxes
                    .iter()
                    .any(|b| Self::rect_intersect(&placement.rect, b));

                if !collision {
                    best_placement = Some(placement);
                    break; // Found a free spot, break out of the SEARCH loop
                }
            }

            // fallback if everything was occupied
            let final_placement = best_placement.or(first_choice).unwrap();

            // generate the property updates to be applied at the end
            let mut current_y = final_placement.start_y;

            for i in 0..props_to_place.len() {
                let prop = props_to_place[i];
                let height = heights[i];
                let width = widths[i];
                let mut new_prop = prop.clone();
                new_prop.effects.justify.clear();
                new_prop.pos.x = match final_placement.dir {
                    Direction::Left => final_placement.start_x + max_width - (width / 2.0),
                    Direction::Right => final_placement.start_x + (width / 2.0),
                    _ => final_placement.start_x + (max_width / 2.0),
                };

                new_prop.pos.y = current_y + (height / 2.0);
                current_y += height + LINE_SPACING;
                props_updates.push(new_prop);
            }

            // write the result to the properties
            if let models::schema::SchemaItem::Symbol(symbol) = &mut self.schema.items[i] {
                for update in props_updates {
                    let prop = symbol
                        .props
                        .iter_mut()
                        .find(|p| p.key == update.key)
                        .unwrap();
                    prop.pos = update.pos;
                    prop.pos.angle = text_angle;
                    prop.effects = update.effects;
                }
            }
        }

        Ok(&self.schema)
    }
}

// ==============================================================================
// CONFIG COLLECTORS (SHADOW STRUCTS)
// ==============================================================================

#[derive(Clone, Default)]
pub struct Symbol {
    pub reference: String,
    pub value: String,
    pub lib_id: String,
    pub attrs: To,
}

impl Symbol {
    pub fn new(reference: &str, value: &str, lib_id: &str) -> Self {
        Self {
            reference: reference.to_string(),
            value: value.to_string(),
            lib_id: lib_id.to_string(),
            attrs: To::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct LocalLabel {
    pub text: String,
    pub attrs: To,
}

impl LocalLabel {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            attrs: To::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct GlobalLabel {
    pub text: String,
    pub attrs: To,
}

impl GlobalLabel {
    pub fn new(text: &str) -> Self {
        Self {
            text: text.to_string(),
            attrs: To::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct Wire {
    pub attrs: To,
}

impl Wire {
    pub fn new() -> Self {
        Self { attrs: To::new() }
    }
}

#[derive(Clone, Default)]
pub struct Junction {
    pub attrs: To,
}

impl Junction {
    pub fn new() -> Self {
        Self { attrs: To::new() }
    }
}

#[derive(Clone, Default)]
pub struct NoConnect {
    pub attrs: To,
}

impl NoConnect {
    pub fn new() -> Self {
        Self { attrs: To::new() }
    }
}

#[derive(Clone, Default)]
pub struct Feedback {
    pub atref: Option<(String, String)>,
    pub toref: Option<(String, String)>,
    pub with: Option<Symbol>,
    pub height: f64,
    pub dot: Option<Vec<DotPosition>>,
    pub attrs: To,
}

impl Feedback {
    pub fn new() -> Self {
        Self {
            height: 5.0 * 2.54,
            ..Default::default()
        }
    }
}

// ==============================================================================
// FLUENT API IMPLEMENTATIONS
// ==============================================================================

pub trait Drawable<T> {
    fn attr(self, attr: Attribute) -> T;
}

impl Drawable<Symbol> for Symbol {
    fn attr(mut self, attr: Attribute) -> Symbol {
        self.attrs.push(attr);
        self
    }
}

impl Drawable<LocalLabel> for LocalLabel {
    fn attr(mut self, attr: Attribute) -> LocalLabel {
        self.attrs.push(attr);
        self
    }
}

impl Drawable<GlobalLabel> for GlobalLabel {
    fn attr(mut self, attr: Attribute) -> GlobalLabel {
        self.attrs.push(attr);
        self
    }
}

impl Drawable<Wire> for Wire {
    fn attr(mut self, attr: Attribute) -> Wire {
        self.attrs.push(attr);
        self
    }
}

impl Drawable<Junction> for Junction {
    fn attr(mut self, attr: Attribute) -> Junction {
        self.attrs.push(attr);
        self
    }
}

impl Drawable<NoConnect> for NoConnect {
    fn attr(mut self, attr: Attribute) -> NoConnect {
        self.attrs.push(attr);
        self
    }
}

impl Drawable<Feedback> for Feedback {
    fn attr(mut self, attr: Attribute) -> Feedback {
        self.attrs.push(attr);
        self
    }
}

// ==============================================================================
// DRAWER IMPLEMENTATIONS
// ==============================================================================

impl Drawer<LocalLabel> for SchemaBuilder {
    type Output = models::schema::LocalLabel;

    fn draw(&mut self, cmd: LocalLabel) -> Result<Self::Output, RecadError> {
        let pt = if let Some(at) = cmd.attrs.at() {
            self.get_pt(&at)
        } else {
            self.get_pt(&self.last_pos)
        };

        let mut label = models::schema::LocalLabel::new(&cmd.text);
        label.pos.x = pt.x;
        label.pos.y = pt.y;

        if let Some(angle) = cmd.attrs.angle() {
            label.pos.angle = angle;
        }

        // Adjust text justification based on angle
        if label.pos.angle == 0.0 || label.pos.angle == 90.0 {
            label.effects.justify = vec![Justify::Left, Justify::Bottom];
        } else if label.pos.angle == 180.0 || label.pos.angle == 270.0 {
            label.effects.justify = vec![Justify::Right, Justify::Bottom];
        }

        self.schema
            .items
            .push(models::schema::SchemaItem::LocalLabel(label.clone()));
        self.last_pos = At::Pt(pt);

        Ok(label)
    }
}

impl Drawer<GlobalLabel> for SchemaBuilder {
    type Output = models::schema::GlobalLabel;

    fn draw(&mut self, cmd: GlobalLabel) -> Result<Self::Output, RecadError> {
        let pt = if let Some(at) = cmd.attrs.at() {
            self.get_pt(&at)
        } else {
            self.get_pt(&self.last_pos)
        };

        let mut label = models::schema::GlobalLabel::new(&cmd.text);
        label.pos.x = pt.x;
        label.pos.y = pt.y;

        if let Some(angle) = cmd.attrs.angle() {
            label.pos.angle = angle;
        }

        self.schema
            .items
            .push(models::schema::SchemaItem::GlobalLabel(label.clone()));
        self.last_pos = At::Pt(pt);

        Ok(label)
    }
}

impl Drawer<Junction> for SchemaBuilder {
    type Output = models::schema::Junction;

    fn draw(&mut self, cmd: Junction) -> Result<Self::Output, RecadError> {
        let pt = if let Some(at) = cmd.attrs.at() {
            self.get_pt(&at)
        } else {
            self.get_pt(&self.last_pos)
        };

        let mut junction = models::schema::Junction::new();
        junction.pos = Pos {
            x: pt.x,
            y: pt.y,
            angle: 0.0,
        };

        self.schema
            .items
            .push(models::schema::SchemaItem::Junction(junction.clone()));
        self.last_pos = At::Pt(pt);

        Ok(junction)
    }
}

impl Drawer<NoConnect> for SchemaBuilder {
    type Output = models::schema::NoConnect;

    fn draw(&mut self, cmd: NoConnect) -> Result<Self::Output, RecadError> {
        let pt = if let Some(at) = cmd.attrs.at() {
            self.get_pt(&at)
        } else {
            self.get_pt(&self.last_pos)
        };

        let mut no_connect = models::schema::NoConnect::new();
        no_connect.pos.x = pt.x;
        no_connect.pos.y = pt.y;

        self.schema
            .items
            .push(models::schema::SchemaItem::NoConnect(no_connect.clone()));
        self.last_pos = At::Pt(pt);

        Ok(no_connect)
    }
}

impl Drawer<Wire> for SchemaBuilder {
    type Output = models::schema::Wire;

    fn draw(&mut self, cmd: Wire) -> Result<Self::Output, RecadError> {
        let pt = if let Some(to) = cmd.attrs.at() {
            self.get_pt(&to)
        } else {
            self.get_pt(&self.last_pos)
        };

        let to_pos = if let Some(tox) = cmd.attrs.tox() {
            let target_pos = self.get_pt(tox);
            Pt {
                x: target_pos.x,
                y: pt.y,
            }
        } else if let Some(toy) = cmd.attrs.toy() {
            let target_pos = self.get_pt(toy);
            Pt {
                x: pt.x,
                y: target_pos.y,
            }
        } else {
            match cmd.attrs.direction() {
                Direction::Left => Pt {
                    x: pt.x - cmd.attrs.length().unwrap_or(1.0),
                    y: pt.y,
                },
                Direction::Right => Pt {
                    x: pt.x + cmd.attrs.length().unwrap_or(1.0),
                    y: pt.y,
                },
                Direction::Up => Pt {
                    x: pt.x,
                    y: pt.y - cmd.attrs.length().unwrap_or(1.0),
                },
                Direction::Down => Pt {
                    x: pt.x,
                    y: pt.y + cmd.attrs.length().unwrap_or(1.0),
                },
            }
        };

        let mut wire = models::schema::Wire::new();
        wire.pts = Pts(vec![pt, to_pos]);

        self.schema
            .items
            .push(models::schema::SchemaItem::Wire(wire.clone()));
        self.last_pos = At::Pt(to_pos);

        Ok(wire)
    }
}

impl Drawer<Symbol> for SchemaBuilder {
    type Output = models::schema::Symbol;

    fn draw(&mut self, cmd: Symbol) -> Result<Self::Output, RecadError> {
        // Resolve library symbol reference
        let lib = self.schema.library_symbol_mut(&cmd.lib_id)?.clone();

        let selected_unit = cmd.attrs.unit().unwrap_or(1);

        // Create the pure model
        let mut new_symbol = lib.symbol(selected_unit);
        new_symbol.pos.angle = cmd.attrs.angle().unwrap_or(0.0);
        new_symbol.mirror = cmd.attrs.mirror();
        new_symbol.unit = selected_unit;

        let start_pt = if let Some(at_attr) = cmd.attrs.at() {
            self.get_pt(&at_attr)
        } else {
            self.get_pt(&self.last_pos)
        };

        // Handle auto-routing along X (tox)
        if let Some(tox_at) = cmd.attrs.tox() {
            let target_pos = self.get_pt(tox_at);

            let p1_rel = models::geometry::pin_position(&new_symbol, lib.pin("1").unwrap());
            let p2_rel = models::geometry::pin_position(&new_symbol, lib.pin("2").unwrap());
            let sym_len_x = (p1_rel.x - p2_rel.x).abs();

            let total_dist_x = (target_pos.x - start_pt.x).abs();
            let wire_length = (total_dist_x - sym_len_x) / 2.0;

            let dir = if target_pos.x > start_pt.x {
                Direction::Right
            } else {
                Direction::Left
            };

            // Draw Lead-in Wire
            self.last_pos = At::Pt(start_pt);
            self.draw(
                Wire::new()
                    .attr(Attribute::Direction(dir.clone()))
                    .attr(Attribute::Length(wire_length)),
            )?;

            // Position symbol at the end of the wire
            let wire_end_pt = self.get_pt(&self.last_pos);
            let anchor_name = cmd.attrs.anchor().unwrap_or_else(|| "1".to_string());
            let anchor_rel =
                models::geometry::pin_position(&new_symbol, lib.pin(&anchor_name).unwrap());

            new_symbol.pos.x = wire_end_pt.x - anchor_rel.x;
            new_symbol.pos.y = wire_end_pt.y - anchor_rel.y;

            self.apply_symbol_metadata(&mut new_symbol, &cmd, &lib, selected_unit)?;
            self.schema
                .items
                .push(models::schema::SchemaItem::Symbol(new_symbol.clone()));

            // Draw Lead-out Wire
            let out_pin_name = if anchor_name == "1" { "2" } else { "1" };
            let out_pin_pos =
                models::geometry::pin_position(&new_symbol, lib.pin(out_pin_name).unwrap());
            self.last_pos = At::Pt(out_pin_pos);

            self.draw(
                Wire::new()
                    .attr(Attribute::Direction(dir))
                    .attr(Attribute::Length(wire_length)),
            )?;

            return Ok(new_symbol);
        }

        // Standard Placement (No auto-wiring)
        let anchor_name = cmd.attrs.anchor().unwrap_or_else(|| "1".to_string());
        let anchor_rel =
            models::geometry::pin_position(&new_symbol, lib.pin(&anchor_name).unwrap());

        new_symbol.pos.x = start_pt.x - anchor_rel.x;
        new_symbol.pos.y = start_pt.y - anchor_rel.y;

        self.apply_symbol_metadata(&mut new_symbol, &cmd, &lib, selected_unit)?;

        let pin_count = lib.pins(selected_unit).len();
        let out_pin = if pin_count == 1 || anchor_name == "2" {
            "1"
        } else {
            "2"
        };

        if let Some(pin) = lib.pin(out_pin) {
            self.last_pos = At::Pt(models::geometry::pin_position(&new_symbol, pin));
        } else {
            self.last_pos = At::Pt(Pt {
                x: new_symbol.pos.x,
                y: new_symbol.pos.y,
            });
        }

        self.schema
            .items
            .push(models::schema::SchemaItem::Symbol(new_symbol.clone()));

        Ok(new_symbol)
    }
}

impl Drawer<Feedback> for SchemaBuilder {
    type Output = Feedback;

    fn draw(&mut self, feedback: Feedback) -> Result<Self::Output, RecadError> {
        if let Some((reference, pin)) = &feedback.atref {
            // move to start
            self.move_to(At::Pin(reference.clone(), pin.clone()));

            // draw the first vertical wire
            let mut w_up = Wire::new()
                .attr(Attribute::Direction(if feedback.height >= 0.0 {
                    Direction::Up
                } else {
                    Direction::Down
                }))
                .attr(Attribute::Length(feedback.height.abs()));

            if let Some(dots) = &feedback.dot {
                if dots.contains(&DotPosition::Start) {
                    w_up = w_up.attr(Attribute::Dot(vec![DotPosition::Start]));
                }
            }
            self.draw(w_up)?;

            // insert component if available
            if let Some(sym) = &feedback.with {
                self.draw(sym.clone())?;
            }

            // draw the horizontal wire over/under
            if let Some((to_reference, to_pin)) = &feedback.toref {
                let w_across = Wire::new().attr(Attribute::Tox(At::Pin(
                    to_reference.clone(),
                    to_pin.clone(),
                )));
                self.draw(w_across)?;

                // close it vertically back to target pin
                let mut w_down = Wire::new().attr(Attribute::Toy(At::Pin(
                    to_reference.clone(),
                    to_pin.clone(),
                )));

                if let Some(dots) = &feedback.dot {
                    if dots.contains(&DotPosition::End) {
                        w_down = w_down.attr(Attribute::Dot(vec![DotPosition::End]));
                    }
                }
                self.draw(w_down)?;
            }
        }

        Ok(feedback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use models::schema::SchemaItem;
    use types::gr::Pt;

    #[test]
    fn test_local_label() {
        let mut builder = SchemaBuilder::new("test");
        let label_cmd = LocalLabel::new("INPUT")
            .attr(Attribute::Rotate(90.0))
            .attr(Attribute::At(At::Pt(Pt { x: 12.5, y: 12.5 })));
        let drawn_label = builder.draw(label_cmd).unwrap();

        assert_eq!("INPUT", drawn_label.text);
        assert_eq!(12.5, drawn_label.pos.x);
        assert_eq!(12.5, drawn_label.pos.y);
        assert_eq!(90.0, drawn_label.pos.angle);

        let Some(SchemaItem::LocalLabel(schema_label)) = builder.schema.items.last() else {
            panic!("label not found in schema items list");
        };

        assert_eq!("INPUT", schema_label.text);
        assert_eq!(12.5, schema_label.pos.x);
        assert_eq!(12.5, schema_label.pos.y);
        assert_eq!(90.0, schema_label.pos.angle);
    }

    #[test]
    fn test_opamp_pin_direction() {
        let mut builder = SchemaBuilder::new("test");
        let op_cmd = Symbol::new("U1", "TL072", "Amplifier_Operational:TL072").attr(Attribute::At(
            At::Pt(Pt {
                x: 2.54 * 20.0,
                y: 2.54 * 10.0,
            }),
        ));
        builder.draw(op_cmd).unwrap();
        let symbol = builder.schema.symbol("U1", 1).unwrap();
        let lib_symbol = builder
            .schema
            .library_symbol("Amplifier_Operational:TL072")
            .unwrap();
        let sym_bbox = symbol.outline(lib_symbol).unwrap();
        let direction = builder.pin_directions(lib_symbol, symbol, &sym_bbox);
        assert_eq!(
            direction,
            PinDirections {
                up: 0,
                down: 0,
                left: 2,
                right: 1
            }
        );
    }

    #[test]
    fn test_opamp_final() {
        let mut builder = SchemaBuilder::new("test");
        let op_cmd = Symbol::new("U1", "TL072", "Amplifier_Operational:TL072").attr(Attribute::At(
            At::Pt(Pt {
                x: 2.54 * 20.0,
                y: 2.54 * 10.0,
            }),
        ));
        builder.draw(op_cmd).unwrap();
        let schema = builder.finalize().unwrap();

        // let mut file = std::fs::File::create("opamp.kicad_sch").unwrap();
        // schema.write(&mut file).unwrap();

        let symbol = schema.items.first().unwrap();
        if let SchemaItem::Symbol(symbol) = symbol {
            for prop in &symbol.props {
                if prop.key == "Reference" {
                    assert_eq!(
                        prop.pos,
                        Pos {
                            x: 43.18,
                            y: 16.12900002861023,
                            angle: 0.0
                        }
                    );
                } else if prop.key == "Value" {
                    assert_eq!(
                        prop.pos,
                        Pos {
                            x: 43.18,
                            y: 18.415000009536744,
                            angle: 0.0
                        }
                    );
                } else {
                    assert!(!prop.visible(), "{:?}", prop);
                }
            }
        }
    }

    #[test]
    fn test_ground_symbol_property_placement() {
        let mut builder = SchemaBuilder::new("test");

        // Command to place a GND symbol.
        // Note: Assuming "power:GND" is the correct lib_id in your library setup.
        let gnd_cmd = Symbol::new("#PWR01", "GND", "power:GND")
            .attr(Attribute::At(At::Pt(Pt { x: 50.8, y: 50.8 })));

        builder.draw(gnd_cmd).unwrap();

        // Finalize will run the auto-placement logic
        let schema = builder.finalize().unwrap();

        // Extract the symbol from the schema
        let item = schema.items.first().unwrap();
        if let SchemaItem::Symbol(symbol) = item {
            // Get the library symbol to find its bounding box
            let lib_symbol = schema
                .library_symbol("power:GND")
                .expect("Failed to find power:GND in library");
            let sym_bbox = symbol.outline(lib_symbol).unwrap();
            let mut checked_props = 0;
            for prop in &symbol.props {
                if prop.visible() && !prop.value.is_empty() {
                    checked_props += 1;

                    assert!(
                        prop.pos.y >= sym_bbox.end.y,
                        "Property {}='{}' is at Y={}, but bounding box ends at Y={}. It should be on the opposite side of the pin (BOTTOM).",
                        prop.key, prop.value, prop.pos.y, sym_bbox.end.y
                    );
                }
            }
            assert!(
                checked_props > 0,
                "No visible properties were found to check."
            );
        } else {
            panic!("Expected the first item to be a Symbol");
        }
    }

    #[test]
    fn test_resistor_placement() {
        let mut builder = SchemaBuilder::new("test");

        // Command to place a GND symbol.
        // Note: Assuming "power:GND" is the correct lib_id in your library setup.
        let r_cmd = Symbol::new("R1", "100k", "Device:R")
            .attr(Attribute::At(At::Pt(Pt { x: 50.8, y: 50.8 })))
            .attr(Attribute::Rotate(90.0));
        builder.draw(r_cmd).unwrap();

        let r_cmd = Symbol::new("R2", "100k", "Device:R")
            .attr(Attribute::At(At::Pt(Pt { x: 70.8, y: 50.8 })))
            .attr(Attribute::Rotate(0.0));
        builder.draw(r_cmd).unwrap();

        let r_cmd = Symbol::new("R3", "100k", "Device:R")
            .attr(Attribute::At(At::Pt(Pt { x: 90.8, y: 50.8 })))
            .attr(Attribute::Rotate(180.0));
        builder.draw(r_cmd).unwrap();

        let r_cmd = Symbol::new("R4", "100k", "Device:R")
            .attr(Attribute::At(At::Pt(Pt { x: 110.8, y: 50.8 })))
            .attr(Attribute::Rotate(270.0));
        builder.draw(r_cmd).unwrap();

        let r_cmd = Symbol::new("R5", "100k", "Device:R")
            .attr(Attribute::At(At::Pt(Pt { x: 130.8, y: 50.8 })))
            .attr(Attribute::Rotate(270.0));
        builder.draw(r_cmd).unwrap();
        let r_cmd = Symbol::new("R6", "100k", "Device:R")
            .attr(Attribute::At(At::Pt(Pt { x: 130.8, y: 60.8 })))
            .attr(Attribute::Rotate(270.0));
        builder.draw(r_cmd).unwrap();

        // Finalize will run the auto-placement logic
        let schema = builder.finalize().unwrap();

        // let mut file = std::fs::File::create("resistor.kicad_sch").unwrap();
        // schema.write(&mut file).unwrap();

        // Extract the symbol from the schema
        let item = schema.items.first().unwrap();
        if let SchemaItem::Symbol(symbol) = item {
            let mut checked_props = 0;
            for prop in &symbol.props {
                if prop.visible() && !prop.value.is_empty() {
                    checked_props += 1;
                    if prop.value == "R1" {
                        assert_eq!(
                            prop.pos,
                            Pos {
                                x: 54.61,
                                y: 45.59300002861023,
                                angle: 90.0
                            }
                        );
                    }
                }
            }
            assert!(
                checked_props > 0,
                "No visible properties were found to check."
            );
        } else {
            panic!("Expected the first item to be a Symbol");
        }
    }
}
