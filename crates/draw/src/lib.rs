//! Drawers for building schemas.
use std::collections::HashMap;

use models::schema::Property;
use models::{geometry::Bbox, schema::SchemaItem};
use types::{
    constants::el,
    error::RecadError,
    gr::{Effects, Justify, Pos, Pt, Pts, Rect},
};

const SPACING: f64 = 2.0 * 0.635;

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

    // Helper to resolve positions
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
        let max = self.schema
            .items
            .iter()
            .filter_map(|item| {
                if let SchemaItem::Symbol(s) = item {
                    if let Some(prop) = s.property(el::PROPERTY_REFERENCE) {
                        prop.strip_prefix(prefix)?
                        .parse::<u32>()
                        .ok()
                    } else { None }
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
    /// Triggered right before saving to auto-place deferred items
    pub fn finalize(&mut self) -> Result<&models::schema::Schema, RecadError> {
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

        // 2. Iterate and place symbol properties contextually
        let num_items = self.schema.items.len();
        for i in 0..num_items {
            let mut sym_bbox = {
                if let models::schema::SchemaItem::Symbol(s) = &self.schema.items[i] {
                    let lib_symbol = self.schema.library_symbol(&s.lib_id).ok_or_else(|| {
                        RecadError::Schema(format!("Library symbol not found: {}", s.lib_id))
                    })?;
                    s.outline(lib_symbol).unwrap_or_default()
                } else {
                    continue; // Skip non-symbols
                }
            };
            let props_to_place: Vec<&mut Property> =
                if let SchemaItem::Symbol(symbol) = &mut self.schema.items[i] {
                    spdlog::debug!("Load Properties");
                    symbol
                        .props
                        .iter_mut()
                        .filter(|p| {
                            spdlog::debug!("Prop: {} {}", p.value, p.visible());
                            p.visible() && !p.value.is_empty()
                        })
                        .collect()
                } else {
                    continue;
                };

            let bbox_properties: Vec<Rect> = dbg!(props_to_place
                .iter()
                .map(|p| (*p).clone())
                .map(|mut p| {
                    p.pos.angle = 0.0;
                    p.outline()
                })
                .collect::<Result<Vec<Rect>, _>>()?);

            let mut max_width = 0.0_f64;
            let mut sum_height = 0.0_f64;
            let mut heights = vec![];
            for bbox in &bbox_properties {
                // Calculate width and height (using .abs() to ensure they are positive)
                let width = (bbox.end.x - bbox.start.x).abs();
                let height = (bbox.end.y - bbox.start.y).abs();
                heights.push(height);
                // Update max width
                max_width = max_width.max(width);

                // Add to total height
                sum_height += height + SPACING;
            }

            spdlog::debug!("width/height: {}x{}", max_width, sum_height);

            //search the directions
            //TOP

            // Ensure start coordinates are always smaller than end coordinates
            if sym_bbox.start.x > sym_bbox.end.x {
                std::mem::swap(&mut sym_bbox.start.x, &mut sym_bbox.end.x);
            }
            if sym_bbox.start.y > sym_bbox.end.y {
                std::mem::swap(&mut sym_bbox.start.y, &mut sym_bbox.end.y);
            }
            assert!(
                sym_bbox.start.x <= sym_bbox.end.x,
                "end x is smaller then start x"
            );
            assert!(
                sym_bbox.start.y <= sym_bbox.end.y,
                "end y is smaller then start y"
            );

            let top_start = Pt {
                x: sym_bbox.start.x + ((sym_bbox.end.x - sym_bbox.start.x) / 2.0) - max_width / 2.0,
                y: sym_bbox.start.y - sum_height,
            };
            let top_end = top_start
                + Pt {
                    x: max_width,
                    y: sum_height,
                };
            let rect = Rect {
                start: top_start,
                end: top_end,
            };

            let collision = static_bboxes.iter().any(|b| Self::rect_intersect(&rect, b));

            if !collision {
                let mut spacing = 0.0;
                assert!(
                    props_to_place.len() == heights.len(),
                    "props and heights do not have the same size"
                );
                for (prop, height) in props_to_place.into_iter().zip(heights.iter()) {
                    println!("HEIGHT: {}", height);
                    prop.pos.x = sym_bbox.start.x + ((sym_bbox.end.x - sym_bbox.start.x) / 2.0);
                    prop.pos.y = top_start.y + spacing;
                    prop.effects.justify.clear();
                    spacing += SPACING + height;
                }
            }
            //RIGHT
            //BOTTOM
            //LEFT

            // let cx = sym_bbox.start.x + (sym_bbox.end.x - sym_bbox.start.x) / 2.0;
            // let cy = sym_bbox.start.y + (sym_bbox.end.y - sym_bbox.start.y) / 2.0;
            // let base_margin = 1.27; // Grid spacing / safety gap
            //
            // for p_idx in props_to_place {
            //     let mut placed_bbox = None;
            //     let mut best_fallback = None;
            //
            //     // Test iteratively outwards up to 6 steps to allow stacking properties nicely
            //     for step in 1..=6 {
            //         let margin = base_margin * (step as f64);
            //
            //         // Search order: Above, Left, Below, Right
            //         // Justify limits it to grow OUTWARD from the symbol to avoid intersections natively.
            //         let candidates = [
            //             (cx, sym_bbox.start.y - margin, vec![Justify::Bottom]), // Above (grows UP)
            //             (sym_bbox.start.x - margin, cy, vec![Justify::Right]),  // Left (grows LEFT)
            //             (cx, sym_bbox.end.y + margin, vec![Justify::Top]),      // Below (grows DOWN)
            //             (sym_bbox.end.x + margin, cy, vec![Justify::Left]),     // Right (grows RIGHT)
            //         ];
            //
            //         for (cand_x, cand_y, justify) in candidates {
            //             // Mutate property briefly to test its outline/collision
            //             if let models::schema::SchemaItem::Symbol(s) = &mut self.schema.items[i] {
            //                 s.props[p_idx].pos.x = cand_x;
            //                 s.props[p_idx].pos.y = cand_y;
            //                 s.props[p_idx].pos.angle = 0.0; // Place standard upright for readability
            //                 s.props[p_idx].effects.justify = justify.clone();
            //             }
            //
            //             // Generate bounding box for this candidate position
            //             let prop_bbox = if let models::schema::SchemaItem::Symbol(s) = &self.schema.items[i] {
            //                 s.props[p_idx].outline(&self.schema).unwrap_or_default()
            //             } else {
            //                 Rect::default()
            //             };
            //
            //             let collision = static_bboxes.iter().any(|b| Self::rect_intersect(&prop_bbox, b));
            //
            //             if !collision {
            //                 placed_bbox = Some((prop_bbox, cand_x, cand_y, justify));
            //                 break;
            //             } else if best_fallback.is_none() {
            //                 // Cache the first candidate (Above, step 1) as our fallback if nothing fits
            //                 best_fallback = Some((prop_bbox, cand_x, cand_y, justify));
            //             }
            //         }
            //
            //         if placed_bbox.is_some() {
            //             break;
            //         }
            //     }
            //
            //     // If no spot is totally free, use our safest bet
            //     let (final_bbox, final_x, final_y, final_justify) = placed_bbox.unwrap_or_else(|| best_fallback.unwrap());
            //
            //     // Apply the final chosen position coordinates
            //     if let models::schema::SchemaItem::Symbol(s) = &mut self.schema.items[i] {
            //         s.props[p_idx].pos.x = final_x;
            //         s.props[p_idx].pos.y = final_y;
            //         s.props[p_idx].pos.angle = 0.0;
            //         s.props[p_idx].effects.justify = final_justify;
            //     }
            //
            //     // Add to our collision list so the next property properly clears it
            //     static_bboxes.push(final_bbox);
            // }
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
                    x: pt.x - cmd.attrs.length().unwrap_or(self.grid),
                    y: pt.y,
                },
                Direction::Right => Pt {
                    x: pt.x + cmd.attrs.length().unwrap_or(self.grid),
                    y: pt.y,
                },
                Direction::Up => Pt {
                    x: pt.x,
                    y: pt.y - cmd.attrs.length().unwrap_or(self.grid),
                },
                Direction::Down => Pt {
                    x: pt.x,
                    y: pt.y + cmd.attrs.length().unwrap_or(self.grid),
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
}
