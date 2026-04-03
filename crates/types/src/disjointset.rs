use crate::gr::GridPt;
use std::collections::HashMap;

/// A data structure for tracking connected components in a grid using the Union-Find algorithm.
///
/// This struct maintains a collection of disjoint sets, where each set represents a group of
/// connected grid points. It supports two main operations:
/// - `find`: Determine which set a particular point belongs to and return the representative (root) of that set.
/// - `union`: Merge two sets containing different points.
///
/// The implementation uses path compression during `find` to optimize future queries, ensuring
/// nearly constant-time operations on average.
///
/// # Fields
///
/// * `parent` - A mapping from each grid point to its parent in the disjoint set forest.
///   If a point is its own parent (i.e., maps to itself), it is the root of its set.
pub struct DisjointSet {
    parent: HashMap<GridPt, GridPt>,
}

impl DisjointSet {
    pub fn new() -> Self {
        Self {
            parent: HashMap::new(),
        }
    }

    pub fn find(&mut self, pt: GridPt) -> GridPt {
        if *self.parent.get(&pt).unwrap_or(&pt) == pt {
            pt
        } else {
            let p = *self.parent.get(&pt).unwrap();
            let root = self.find(p);
            self.parent.insert(pt, root); // Path compression
            root
        }
    }

    // TODO: is this really not used
    // /// Add a new point to the disjoint set, initializing it as its own parent (a singleton set).
    // ///
    // /// # Arguments
    // ///
    // /// * `pt` - The grid point to add.
    // pub fn add_point(&mut self, pt: GridPt) {
    //     self.parent.entry(pt).or_insert(pt);
    // }
    //
    // /// Check if two points are in the same connected component.
    // ///
    // /// # Arguments
    // ///
    // /// * `a` - First grid point.
    // /// * `b` - Second grid point.
    // ///
    // /// # Returns
    // ///
    // /// `true` if both points belong to the same set, `false` otherwise.
    // pub fn connected(&mut self, a: GridPt, b: GridPt) -> bool {
    //     self.find(a) == self.find(b)
    // }

    // Connect two points
    pub fn union(&mut self, a: GridPt, b: GridPt) {
        let root_a = self.find(a);
        let root_b = self.find(b);
        if root_a != root_b {
            self.parent.insert(root_a, root_b);
        }
    }
}
