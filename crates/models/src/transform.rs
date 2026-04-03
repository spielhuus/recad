use types::gr::Pt;
use glam::{DMat3, DVec2};

#[derive(Clone, Copy, Debug)]
pub struct Transform {
    matrix: DMat3,
}

impl Default for Transform {
    fn default() -> Self {
        Self::new()
    }
}

impl Transform {
    pub fn new() -> Self {
        Self {
            matrix: DMat3::IDENTITY,
        }
    }

    pub fn rotation(mut self, angle_degrees: f64) -> Self {
        let rot = DMat3::from_angle(angle_degrees.to_radians());
        self.matrix *= rot;
        self
    }

    pub fn translation(mut self, pt: Pt) -> Self {
        let trans = DMat3::from_translation(DVec2::new(pt.x, pt.y));
        self.matrix *= trans;
        self
    }

    pub fn scale(mut self, scale: f64) -> Self {
        let s = DMat3::from_scale(DVec2::splat(scale));
        self.matrix *= s;
        self
    }

    pub fn scale_non_uniform(mut self, x: f64, y: f64) -> Self {
        let s = DMat3::from_scale(DVec2::new(x, y));
        self.matrix *= s;
        self
    }

    pub fn mirror(mut self, axis: &Option<String>) -> Self {
        // KiCad Lib (Y-Up) -> Screen (Y-Down) conversion requires a base scale of (1, -1).
        // We combine this with the requested mirror operation.
        let scale = match axis.as_deref() {
            // Mirror X (Vertical Flip in KiCad):
            // Flips Y in Lib. Combined with Lib->Screen flip, Y becomes positive.
            // Net Scale: (1, 1)
            Some("x") => DVec2::new(1.0, 1.0),

            // Mirror Y (Horizontal Flip in KiCad):
            // Flips X in Lib. Combined with Lib->Screen flip (Y: -1).
            // Net Scale: (-1, -1)
            Some("y") => DVec2::new(-1.0, -1.0),
            Some("xy") => DVec2::new(-1.0, 1.0), // Mirror XY + Global Y-Flip = (-1, 1)

            // Default / No Mirror:
            // Must Flip Y to map Lib Up to Screen Down.
            // Net Scale: (1, -1)
            _ => DVec2::new(1.0, -1.0),
        };

        let m = DMat3::from_scale(scale);
        self.matrix *= m;
        self
    }

    /// Takes a Pt, converts to Vec2, transforms, converts back to Pt
    pub fn transform_point(&self, pt: Pt) -> Pt {
        let v = DVec2::new(pt.x, pt.y);
        let res = self.matrix.transform_point2(v);
        Pt { x: res.x, y: res.y }
    }

    /// Handle a list of points
    pub fn transform_pts(&self, pts: &[Pt]) -> Vec<Pt> {
        pts.iter().map(|&p| self.transform_point(p)).collect()
    }
}
