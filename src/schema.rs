use std::path::Path;

use pyo3::{
    exceptions::PyIOError,
    prelude::*,
    types::{IntoPyDict, PyDict, PyList},
};
use recad_core::{
    draw::{At, Attribute, Direction},
    gr::Pt,
    plot::{
        theme::{Theme, Themes},
        Plotter,
    },
    Drawable, Drawer, Plot,
};

/// The Schema
#[pyclass]
pub struct Schema {
    pub schema: recad_core::Schema,
}

#[pymethods]
impl Schema {
    /// create a new Schema
    ///
    /// :param project: the project name
    #[new]
    fn new(project: &str) -> Self {
        Schema {
            schema: recad_core::Schema::new(project),
        }
    }

    /// Load a new Schema from a file.
    ///
    /// :param path: the file path
    #[staticmethod]
    pub fn load(path: &str) -> PyResult<Schema> {
        if let Ok(s) = recad_core::Schema::load(Path::new(path)) {
            Ok(Schema { schema: s })
        } else {
            Err(PyErr::new::<PyIOError, _>(format!(
                "unable to open schema file '{}'",
                path
            )))
        }
    }

    /// Write a new Schema from to file.
    ///
    /// :param path: the file path
    pub fn write(&self, path: &str) -> PyResult<()> {
        let mut writer = std::fs::File::create(path).unwrap();
        self.schema.write(&mut writer).unwrap();
        Ok(())
    }

    /// Plot a schema
    ///
    /// :param path: the file path
    #[pyo3(signature = (**kwargs))]
    pub fn plot(&self, kwargs: Option<Bound<PyDict>>) -> PyResult<Option<Py<PyAny>>> {
        let mut path: Option<String> = None;

        if let Some(kwargs) = kwargs {
            if let Ok(Some(raw_item)) = kwargs.get_item("path") {
                let item: Result<String, PyErr> = raw_item.extract();
                if let Ok(item) = item {
                    path = Some(item.to_string());
                }
            }
        }

        let mut svg = recad_core::plot::SvgPlotter::new(); //TODO select plotter
        self.schema.plot(&mut svg, &Theme::from(Themes::Kicad2020)); //TODO select theme

        Ok(if let Some(path) = path {
            let mut file = std::fs::File::create(path).unwrap();
            svg.write(&mut file).unwrap();
            None
        } else {
            let mut buffer = Vec::new();
            svg.write(&mut buffer).unwrap();

            let res = Python::with_gil(|py| {
                let svg_path: Py<PyAny> = py
                    .import_bound("IPython")
                    .unwrap()
                    .getattr("display")
                    .unwrap()
                    .getattr("SVG")
                    .unwrap()
                    .into();
                let kwargs = [("data", String::from_utf8(buffer).unwrap())].into_py_dict_bound(py);
                svg_path
                    .call_bound(py, (), Some(&kwargs.into_py_dict_bound(py)))
                    .unwrap()
            });
            Some(res)
        })
    }

    pub fn move_to(mut instance: PyRefMut<'_, Self>, item: (f32, f32)) -> PyRefMut<'_, Self> {
        instance.schema.move_to(At::Pt(Pt {
            x: item.0,
            y: item.1,
        }));
        instance
    }

    pub fn draw<'a>(mut instance: PyRefMut<'a, Self>, item: &Bound<PyAny>) -> PyRefMut<'a, Self> {
        let label: Result<LocalLabel, PyErr> = item.extract();
        if let Ok(label) = label {
            let mut final_label = recad_core::schema::LocalLabel::new(&label.name)
                .attr(Attribute::Rotate(label.rotate));
            final_label = final_label.attr(Attribute::Rotate(label.rotate));
            if let Some(at) = label.at {
                final_label = final_label.attr(Attribute::At(at));
            }
            instance.schema.draw(final_label).unwrap(); //TODO
            return instance;
        }

        let symbol: Result<Symbol, PyErr> = item.extract();
        if let Ok(symbol) = symbol {
            let mut final_symbol =
                recad_core::schema::Symbol::new(&symbol.reference, &symbol.value, &symbol.lib_id);
            final_symbol = final_symbol.attr(Attribute::Rotate(symbol.rotate));
            if let Some(anchor) = symbol.anchor {
                final_symbol = final_symbol.attr(Attribute::Anchor(anchor));
            }
            if let Some(mirror) = symbol.mirror {
                final_symbol = final_symbol.attr(Attribute::Mirror(mirror));
            }
            if let Some(tox) = symbol.tox {
                final_symbol = final_symbol.attr(Attribute::Tox(tox));
            }
            if let Some(toy) = symbol.toy {
                final_symbol = final_symbol.attr(Attribute::Toy(toy));
            }
            if let Some(at) = symbol.at {
                final_symbol = final_symbol.attr(Attribute::At(at));
            }
            instance.schema.draw(final_symbol).unwrap(); //TODO
            return instance;
        }

        let wire: Result<Wire, PyErr> = item.extract();
        if let Ok(wire) = wire {
            let mut final_wire = recad_core::schema::Wire::new();
            final_wire = match wire.direction {
                Direction::Left => final_wire.attr(Attribute::Direction(Direction::Left)),
                Direction::Right => final_wire.attr(Attribute::Direction(Direction::Right)),
                Direction::Up => final_wire.attr(Attribute::Direction(Direction::Up)),
                Direction::Down => final_wire.attr(Attribute::Direction(Direction::Up)),
            };
            final_wire = final_wire.attr(Attribute::Length(wire.length * 2.54)); //make configurable
            if let Some(tox) = wire.tox {
                final_wire = final_wire.attr(Attribute::Tox(tox));
            }
            if let Some(toy) = wire.toy {
                final_wire = final_wire.attr(Attribute::Toy(toy));
            }
            instance.schema.draw(final_wire).unwrap(); //TODO
            return instance;
        }

        let junction: Result<Junction, PyErr> = item.extract();
        if let Ok(junction) = junction {
            let final_junction = recad_core::schema::Junction::new();
            instance.schema.draw(final_junction).unwrap(); //TODO
            return instance;
        }

        println!("ERR: type not found: {}", item);
        instance
    }

    fn __add__<'a>(instance: PyRefMut<'a, Self>, item: &Bound<PyAny>) -> PyRefMut<'a, Self> {
        Schema::draw(instance, item)
    }

    fn __str__(&self) -> PyResult<String> {
        Ok(format!("[__str__] {}", self.schema))
    }

    fn __repr__(&self) -> PyResult<String> {
        Ok(format!("[__str__] {}", self.schema))
    }
}

/// A `GlobalLabel` is a custom identifier that can be assigned to
/// multiple objects or components across the entire design.
#[pyclass]
pub struct GlobalLabel {}

#[pymethods]
impl GlobalLabel {
    #[new]
    fn new() -> Self {
        Self {}
    }
}

/// A junction represents a connection point where multiple wires
/// or components intersect, allowing electrical current to
/// flow between them.
#[pyclass]
#[derive(Clone)]
pub struct Junction {}

#[pymethods]
impl Junction {
    #[new]
    fn new() -> Self {
        Self {}
    }
}

/// A `LocalLabel` refers to an identifier assigned to individual
/// Components or objects within a specific grouping on
/// the same schema page.
#[pyclass]
#[derive(Clone, Default)]
pub struct LocalLabel {
    name: String,
    rotate: f32,
    pub at: Option<At>,
}

#[pymethods]
impl LocalLabel {
    #[new]
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            rotate: 0.0,
            ..Default::default()
        }
    }

    /// Rotate the label
    ///
    /// :param angle: rotation angle in degrees
    pub fn rotate(mut instance: PyRefMut<'_, Self>, angle: f32) -> PyRefMut<'_, Self> {
        instance.rotate = angle;
        instance
    }

    /// place the label.
    ///
    /// :param reference: the Symbol label
    /// :param pin: the pin of the Symbol.
    pub fn at(
        mut instance: PyRefMut<'_, Self>,
        reference: String,
        pin: String,
    ) -> PyRefMut<'_, Self> {
        instance.at = Some(At::Pin(reference, pin));
        instance
    }
}

/// A schematic `Symbol` representing an instance from the [`symbols`] library.
#[pyclass]
#[derive(Clone, Default)]
pub struct Symbol {
    pub reference: String,
    pub value: String,
    pub lib_id: String,
    pub rotate: f32,
    pub anchor: Option<String>,
    pub mirror: Option<String>,
    pub tox: Option<At>,
    pub toy: Option<At>,
    pub at: Option<At>,
}

#[pymethods]
impl Symbol {
    #[new]
    fn new(reference: &str, value: &str, lib_id: &str) -> Self {
        Self {
            reference: reference.to_string(),
            value: value.to_string(),
            lib_id: lib_id.to_string(),
            ..Default::default()
        }
    }

    /// Rotate the symbol
    ///
    /// :param angle: rotation angle in degrees
    pub fn rotate(mut instance: PyRefMut<'_, Self>, angle: f32) -> PyRefMut<'_, Self> {
        instance.rotate = angle;
        instance
    }

    /// Set an anchor Pin.
    ///
    /// :param pin: the anchor pin.
    pub fn anchor(mut instance: PyRefMut<'_, Self>, pin: String) -> PyRefMut<'_, Self> {
        instance.anchor = Some(pin);
        instance
    }

    /// Mirror the symbol
    ///
    /// :param axis: the mirror axis ['x', 'y', 'xy']
    pub fn mirror(mut instance: PyRefMut<'_, Self>, axis: String) -> PyRefMut<'_, Self> {
        instance.mirror = Some(axis);
        instance
    }

    /// Expand the length to the pin horizontally
    ///
    ///  Draw wires at both the start and finish 
    ///  of the symbol for path completion.
    ///
    /// :param reference: the Symbol label
    /// :param pin: the pin of the Symbol.
    pub fn tox(
        mut instance: PyRefMut<'_, Self>,
        reference: String,
        pin: String,
    ) -> PyRefMut<'_, Self> {
        instance.tox = Some(At::Pin(reference, pin));
        instance
    }

    /// Expand the length to the pin vertically
    ///
    ///  Draw wires at both the start and finish 
    ///  of the symbol for path completion.
    ///
    /// :param reference: the Symbol label
    /// :param pin: the pin of the Symbol.
    pub fn toy(
        mut instance: PyRefMut<'_, Self>,
        reference: String,
        pin: String,
    ) -> PyRefMut<'_, Self> {
        instance.toy = Some(At::Pin(reference, pin));
        instance
    }

    /// place the label.
    ///
    /// :param reference: the Symbol label
    /// :param pin: the pin of the Symbol.
    pub fn at(
        mut instance: PyRefMut<'_, Self>,
        reference: String,
        pin: String,
    ) -> PyRefMut<'_, Self> {
        instance.at = Some(At::Pin(reference, pin));
        instance
    }
}


#[pyclass]
#[derive(Clone, Default)]
pub struct Wire {
    pub direction: Direction,
    pub length: f32,
    pub tox: Option<At>,
    pub toy: Option<At>,
}

/// Wires represent electrical connections between components or points,
/// showing the circuit's interconnections and paths for electric current flow.
#[pymethods]
impl Wire {
    #[new]
    fn new() -> Self {
        Self {
            direction: Direction::Left,
            length: 1.0,
            ..Default::default()
        }
    }

    /// Draw wire to the left.
    ///
    /// This function draws a wire from the current position
    /// to the left side of the canvas.
    pub fn left(mut instance: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        instance.direction = Direction::Left;
        instance
    }

    /// Draw wire to the right.
    ///
    /// This function draws a wire from the current position
    /// to the right side of the canvas.
    pub fn right(mut instance: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        instance.direction = Direction::Right;
        instance
    }

    /// Draw wire upward.
    ///
    /// This function draws a wire from the current position
    /// to the top edge of the canvas.
    pub fn up(mut instance: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        instance.direction = Direction::Up;
        instance
    }

    /// Draw a line downwards.
    ///
    /// This function draws a line from the current position to 
    /// the bottom edge of the canvas.
    pub fn down(mut instance: PyRefMut<'_, Self>) -> PyRefMut<'_, Self> {
        instance.direction = Direction::Down;
        instance
    }

    /// The length of the wire
    ///
    /// The length is in `units` of the canvas.
    /// This is tipically 2.54mm.
    ///
    /// :param length: the wire length in units.
    pub fn length(mut instance: PyRefMut<'_, Self>, length: f32) -> PyRefMut<'_, Self> {
        instance.length = length;
        instance
    }

    /// Expand the length to the pin horizontally
    ///
    /// :param reference: the Symbol label
    /// :param pin: the pin of the Symbol.
    pub fn tox(
        mut instance: PyRefMut<'_, Self>,
        reference: String,
        pin: String,
    ) -> PyRefMut<'_, Self> {
        instance.tox = Some(At::Pin(reference, pin));
        instance
    }
    /// Expand the length to the pin vertically
    ///
    /// :param reference: the Symbol label
    /// :param pin: the pin of the Symbol.
    pub fn toy(
        mut instance: PyRefMut<'_, Self>,
        reference: String,
        pin: String,
    ) -> PyRefMut<'_, Self> {
        instance.toy = Some(At::Pin(reference, pin));
        instance
    }
}
