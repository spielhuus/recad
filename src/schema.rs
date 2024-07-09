use pyo3::prelude::*;

#[pyclass]
pub struct Schema {

}

#[pymethods]
impl Schema {
    #[new]
    fn new() -> Self {
        Schema {}
    }
}

#[pyclass]
pub struct GlobalLabel {

}

#[pymethods]
impl GlobalLabel {
    #[new]
    fn new() -> Self {
        Self {}
    }
}

#[pyclass]
pub struct Junction {

}

#[pymethods]
impl Junction {
    #[new]
    fn new() -> Self {
        Self {}
    }
}

#[pyclass]
pub struct LocalLabel {

}

#[pymethods]
impl LocalLabel {
    #[new]
    fn new() -> Self {
        Self {}
    }
}


#[pyclass]
pub struct Symbol {

}

#[pymethods]
impl Symbol {
    #[new]
    fn new() -> Self {
        Self {}
    }
}

#[pyclass]
pub struct Wire {

}

#[pymethods]
impl Wire {
    #[new]
    fn new() -> Self {
        Self {}
    }
}
