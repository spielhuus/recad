use pyo3::prelude::*;

mod schema;

#[pyfunction]
pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[pyfunction]
pub fn schema(path: 6str) -> schema::Schema {
    recad::Schema::from_path(path)
}

#[pymodule]
fn recad(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(add, m)?)?;
    m.add_class::<schema::GlobalLabel>()?;
    m.add_class::<schema::Junction>()?;
    m.add_class::<schema::LocalLabel>()?;
    m.add_class::<schema::Schema>()?;
    m.add_class::<schema::Symbol>()?;
    m.add_class::<schema::Wire>()?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
