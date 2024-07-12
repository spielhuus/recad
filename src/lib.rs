use pyo3::prelude::*;

mod schema;

/// recad main function.
#[pyfunction]
pub fn main() -> PyResult<()> {
    env_logger::init();
    Ok(())
}

#[pymodule]
fn recad(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(main, m)?)?;
    m.add_class::<schema::GlobalLabel>()?;
    m.add_class::<schema::Junction>()?;
    m.add_class::<schema::LocalLabel>()?;
    m.add_class::<schema::Schema>()?;
    m.add_class::<schema::Symbol>()?;
    m.add_class::<schema::Wire>()?;
    Ok(())
}
