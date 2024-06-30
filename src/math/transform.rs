use ndarray::{array, concatenate, s, Array2, Axis};

use crate::{gr::Pt, sexp::constants::el};

#[derive(Default)]
pub struct Transform {
    matrix: Array2<f32>,
}

impl Transform {

    pub fn new() -> Self {
        Self {
            matrix: Array2::eye(3),
        }
    }

    pub fn rotation(mut self, angle: f32) -> Self {
        if angle != 0.0 && angle != 360.0 {
            let theta = angle.to_radians();
            let cos = crate::round(theta.cos());
            let sin = crate::round(theta.sin());
            self.matrix =
                self.matrix
                    .dot(&array![
                        [cos, -sin, 0.0], 
                        [sin,  cos, 0.0], 
                        [0.0,  0.0, 1.0]]);
        }

        self
    }

    pub fn mirror(mut self, axis: &Option<String>) -> Self {
        if let Some(axis) = axis {
            if axis == "x" {
                self.matrix =
                    self.matrix
                        .dot(&array![
                            [1.0,  0.0, 0.0],
                            [0.0, -1.0, 0.0],
                            [0.0,  0.0, 1.0]
                        ]
                    );
            } else if axis == "y" {
                self.matrix =
                    self.matrix
                        .dot(&array![
                            [-1.0, 0.0, 0.0],
                            [ 0.0, 1.0, 0.0],
                            [ 0.0, 0.0, 1.0]
                        ]
                    );
            } else if axis == el::XY {
                self.matrix =
                    self.matrix
                        .dot(&array![
                            [-1.0,  0.0, 0.0],
                            [ 0.0, -1.0, 0.0],
                            [ 0.0,  0.0, 1.0]
                        ]
                    );
            }
        }
        self
    }

    pub fn translation(mut self, pt: Pt) -> Self {
        self.matrix = self
            .matrix
            .dot(&array![
                [1.0, 0.0, pt.x],
                [0.0, 1.0, pt.y],
                [0.0, 0.0, 1.0]
            ]);
        self
    }

    pub fn transform(&self, points: &Array2<f32>) -> Array2<f32> {
        // Create a column of ones with the same number of rows as the original array
        let ones = Array2::ones((points.shape()[0], 1));
        let vectors = concatenate![Axis(1), points.view(), ones.view()];
        let rotated_vectors = vectors.dot(&self.matrix.t());
        // Remove the third column from the result
        rotated_vectors.slice_move(s![.., 0..2])
    }
}

#[cfg(test)]
mod test {
    use ndarray::{arr2, array, Array2};

    use crate::gr::Pt;

    #[test]
    fn test_nop() {
        let pts: Array2<f32> = array![[2.0, 3.0], [4.0, 5.0], [6.0, 7.0]];
        let transform = super::Transform::new();
        let result = transform.transform(&pts);
        assert_eq!(result, pts);
    }
    #[test]
    fn test_translate() {
        let mut transform = super::Transform::new();
        transform = transform.translation(Pt { x: 2.0, y: 2.0 });
        let pts = arr2(&[[1.0, 0.0, 2.0], [0.0, 1.0, 2.0], [0.0, 0.0, 1.0]]);
        assert_eq!(pts, transform.matrix);

        let pt = array![
            [0.0, 5.0],   // First vector
            [-5.0, -5.0], // Second vector
            [5.0, 5.0],   // Third vector
            [0.0, 5.0]    // Fourth vector
        ];
        let exp = array![
            [2.0, 7.0],   // First vector
            [-3.0, -3.0], // Second vector
            [7.0, 7.0],   // Third vector
            [2.0, 7.0]    // Fourth vector
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
        let exp = array![[-5., 0.], [5., -5.], [-5., 5.], [-5., 0.],];
        let res = transform.transform(&pt);
        assert_eq!(exp, res);
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
        let exp = array![
            [0.0, -5.0], // First vector
            [-5.0, 5.0], // Second vector
            [5.0, -5.0], // Third vector
            [0.0, -5.0]  // Fourth vector
        ];
        let res = transform.transform(&pt);
        assert_eq!(exp, res);
    }
}
