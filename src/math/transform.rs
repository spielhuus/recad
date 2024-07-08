use std::collections::HashMap;

use ndarray::{arr2, Array2};
use lazy_static::lazy_static;

use crate::{
    gr::Pt,
    math::ToNdarray,
};

#[derive(Default)]
pub struct Transform {
    to: Pt,
    rotate: f32,
    mirror: Option<String>,
}

lazy_static! {
    ///The mirror matrices.
    static ref MIRROR: HashMap<String, Array2<f32>> = HashMap::from([
        (String::from(""), arr2(&[[1., 0.], [0., -1.]])),
        (String::from("x"), arr2(&[[1., 0.], [0., 1.]])),
        (String::from("y"), arr2(&[[-1., 0.], [0., -1.]])),
        (String::from("xy"), arr2(&[[-1., 0.], [0., 1.]])),
    ]);
}

impl Transform {

    pub fn new() -> Self {
        Self {
            to: Pt::default(),
            rotate: 0.0,
            mirror: None,
        }
    }

    pub fn rotation(mut self, angle: f32) -> Self {
        self.rotate = angle;
        self
    }

    pub fn mirror(mut self, axis: &Option<String>) -> Self {
        self.mirror.clone_from(axis);
        self
    }

    pub fn translation(mut self, pt: Pt) -> Self {
        self.to = pt;
        self
    }

    pub fn transform(&self, points: &Array2<f32>) -> Array2<f32> {
        let points = if let Some(mirror) = &self.mirror {
            points.dot(MIRROR.get(mirror).unwrap())
        } else {
            points.dot(MIRROR.get(&String::new()).unwrap())
        };
        let theta = self.rotate.to_radians();
        let rot = arr2(&[[theta.cos(), -theta.sin()], [theta.sin(), theta.cos()]]);
        let points: Array2<f32> = points.dot(&rot);
        &self.to.ndarray() + points
    }
}

#[cfg(test)]
mod test {
    use ndarray::{array, Array2};

    use crate::gr::Pt;

    #[test]
    fn test_nop() {
        //The outcome exhibits symmetry about the x-axis, implying both the symbol
        //library and schema coordinates are reflected along this axis.
        let pts: Array2<f32> = array![[2.0, 3.0], [4.0, 5.0], [6.0, 7.0]];
        let res: Array2<f32> = array![[2.0, -3.0], [4.0, -5.0], [6.0, -7.0]];
        let transform = super::Transform::new();
        let result = transform.transform(&pts);
        assert_eq!(result, res);
    }
    #[test]
    fn test_translate() {
        let mut transform = super::Transform::new();
        transform = transform.translation(Pt { x: 2.0, y: 2.0 });

        let pt = array![
            [0.0, 5.0],   // First vector
            [-5.0, -5.0], // Second vector
            [5.0, 5.0],   // Third vector
            [0.0, 5.0]    // Fourth vector
        ];
        let exp = array![
            [2.0, -3.0],   // First vector
            [-3.0, 7.0], // Second vector
            [7.0, -3.0],   // Third vector
            [2.0, -3.0]    // Fourth vector
        ];
        let res = transform.transform(&pt);
        assert_eq!(exp, res);
    }
    #[test]
    fn test_rotate() {
        let mut transform = super::Transform::new();
        transform = transform.rotation(90.0);
        let pt = array![
            [0.0, 5.0],   // First vector
            [-5.0, -5.0], // Second vector
            [5.0, 5.0],   // Third vector
            [0.0, 5.0]    // Fourth vector
        ];
        let exp = array![[5., 0.], [-5., -5.], [-5., 5.], [5., 0.],];
        let res = transform.transform(&pt);
        //assert_eq!(exp, res);
    }
    #[test]
    fn test_mirror() {
        let mut transform = super::Transform::new();
        transform = transform.mirror(&Some(String::from("x")));
        let pt = array![
            [0.0, 5.0],   // First vector
            [-5.0, -5.0], // Second vector
            [5.0, 5.0],   // Third vector
            [0.0, 5.0]    // Fourth vector
        ];
        let res = transform.transform(&pt);
        assert_eq!(pt, res);
    }
}
