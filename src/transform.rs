//! The placement of a block reference: the 2D affine map that takes a point
//! in a block definition's own coordinates to where an INSERT of that block
//! puts it in the drawing.
//!
//! Every consumer that follows an INSERT into its block -- a renderer, a
//! pointer, an editor -- needs this same map and needs nested references to
//! compose the same way, so it lives with the model rather than in each of
//! them. It is arithmetic on the INSERT's own fields (DXF 10/20/30, 41/42,
//! 50, 210) and nothing else; there is no guessing in it.

use crate::model::{InsertEntity, Point2D, Point3D};
use serde::{Deserialize, Serialize};

/// Relative tolerance under which a placement's linear part counts as a
/// similarity (see [`Affine2::similarity_scale`]). Composing exact
/// similarities in floating point leaves residues of this order; a real
/// shear or per-axis scale is many orders larger.
const SIMILARITY_TOLERANCE: f64 = 1e-9;

/// The world's z axis: the OCS normal whose object coordinates are world
/// coordinates.
const WORLD_Z: Point3D = Point3D {
    x: 0.0,
    y: 0.0,
    z: 1.0,
};

/// The DXF reference's threshold in its arbitrary axis algorithm: a normal
/// this close to the world z axis takes its OCS x axis from the world y
/// axis, any other from the world z axis.
const ARBITRARY_AXIS_THRESHOLD: f64 = 1.0 / 64.0;

/// A 2D affine map, `world = [a c; b d] · local + [e f]`: the six numbers of
/// an SVG `matrix(a b c d e f)`, in that order and with that meaning.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Affine2 {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Affine2 {
    /// Leaves every point where it is.
    pub const IDENTITY: Affine2 = Affine2 {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    /// The placement an INSERT applies to its block's entities: scale by
    /// the per-axis factors, rotate, then translate to the insertion point
    /// -- the DXF order, all of it in the INSERT's object coordinate system
    /// -- and then take that system to the world's (x, y) by the DXF
    /// reference's arbitrary axis algorithm. That last step is where the
    /// normal (210) and the insertion point's OCS z (30) come in: it is the
    /// identity for the usual normal (0, 0, 1), a mirror across the y axis
    /// for (0, 0, -1), and for a tilted normal the view of the tilted plane
    /// from above. The z scale (43) is not part of a 2D placement and is
    /// ignored. A normal of zero length, or one that is not finite, is not
    /// a direction; it places the block as (0, 0, 1) would.
    pub fn from_insert(insert: &InsertEntity) -> Affine2 {
        let in_ocs = Affine2::placement(
            Point2D {
                x: insert.insertion_point.x,
                y: insert.insertion_point.y,
            },
            insert.scale.x,
            insert.scale.y,
            insert.rotation,
        );
        match ocs_to_world(insert.extrusion, insert.insertion_point.z) {
            Some(to_world) => in_ocs.then(&to_world),
            None => in_ocs,
        }
    }

    /// Scale by `(x_scale, y_scale)`, rotate by `rotation` radians, then
    /// translate to `insertion_point`.
    pub fn placement(
        insertion_point: Point2D,
        x_scale: f64,
        y_scale: f64,
        rotation: f64,
    ) -> Affine2 {
        let (sin, cos) = rotation.sin_cos();
        Affine2 {
            a: x_scale * cos,
            b: x_scale * sin,
            c: -y_scale * sin,
            d: y_scale * cos,
            e: insertion_point.x,
            f: insertion_point.y,
        }
    }

    /// Where this map puts `p`.
    pub fn apply(&self, p: Point2D) -> Point2D {
        Point2D {
            x: self.a * p.x + self.c * p.y + self.e,
            y: self.b * p.x + self.d * p.y + self.f,
        }
    }

    /// First this map, then `outer`. Exact for every pair of affine maps:
    /// a nested block's placement is its own INSERT's placement followed by
    /// the placement of the INSERT that placed its parent, and so on up.
    pub fn then(&self, outer: &Affine2) -> Affine2 {
        Affine2 {
            a: outer.a * self.a + outer.c * self.b,
            b: outer.b * self.a + outer.d * self.b,
            c: outer.a * self.c + outer.c * self.d,
            d: outer.b * self.c + outer.d * self.d,
            e: outer.a * self.e + outer.c * self.f + outer.e,
            f: outer.b * self.e + outer.d * self.f + outer.f,
        }
    }

    /// The determinant of the linear part: negative for a mirrored
    /// placement, zero for one that flattens the block.
    pub fn determinant(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }

    /// `Some(s)` when the linear part is a rotation scaled uniformly by
    /// `s > 0` -- no mirroring, no shear, no per-axis scale -- which is
    /// exactly when a circle placed through this map is still a circle (of
    /// radius `r · s`) and an angle is still an angle (offset by
    /// [`rotation`](Self::rotation)). `None` otherwise: what such a
    /// placement does to a circle is not this type's to say.
    pub fn similarity_scale(&self) -> Option<f64> {
        let det = self.determinant();
        if det <= 0.0 {
            return None;
        }
        let la = (self.a * self.a + self.b * self.b).sqrt();
        let lb = (self.c * self.c + self.d * self.d).sqrt();
        let dot = self.a * self.c + self.b * self.d;
        if dot.abs() > SIMILARITY_TOLERANCE * la * lb
            || (la - lb).abs() > SIMILARITY_TOLERANCE * la.max(lb)
        {
            return None;
        }
        Some(det.sqrt())
    }

    /// The rotation of the linear part, in radians, as the angle the local
    /// x axis is turned by.
    pub fn rotation(&self) -> f64 {
        self.b.atan2(self.a)
    }
}

impl InsertEntity {
    /// The placement this reference applies to its block's entities. See
    /// [`Affine2::from_insert`].
    pub fn transform(&self) -> Affine2 {
        Affine2::from_insert(self)
    }
}

/// The map from the (x, y) of an object coordinate system -- at height `z`
/// in it -- to the world's (x, y): the DXF reference's arbitrary axis
/// algorithm, seen from above. `None` when there is nothing to map: the
/// normal is the world z axis, or it is not a direction at all.
fn ocs_to_world(normal: Point3D, z: f64) -> Option<Affine2> {
    if normal == WORLD_Z {
        return None;
    }
    let n = normalized(normal)?;
    let seed = if n.x.abs() < ARBITRARY_AXIS_THRESHOLD && n.y.abs() < ARBITRARY_AXIS_THRESHOLD {
        Point3D {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        }
    } else {
        WORLD_Z
    };
    let x_axis = normalized(cross(seed, n))?;
    let y_axis = normalized(cross(n, x_axis))?;
    Some(Affine2 {
        a: x_axis.x,
        b: x_axis.y,
        c: y_axis.x,
        d: y_axis.y,
        e: n.x * z,
        f: n.y * z,
    })
}

fn cross(u: Point3D, v: Point3D) -> Point3D {
    Point3D {
        x: u.y * v.z - u.z * v.y,
        y: u.z * v.x - u.x * v.z,
        z: u.x * v.y - u.y * v.x,
    }
}

/// `v` scaled to unit length, or `None` when it has no direction.
fn normalized(v: Point3D) -> Option<Point3D> {
    let length = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    (length > 0.0 && length.is_finite()).then(|| Point3D {
        x: v.x / length,
        y: v.y / length,
        z: v.z / length,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::FRAC_PI_2;

    fn p(x: f64, y: f64) -> Point2D {
        Point2D { x, y }
    }

    #[test]
    fn the_identity_leaves_a_point_where_it_is() {
        assert_eq!(Affine2::IDENTITY.apply(p(3.0, -4.0)), p(3.0, -4.0));
        assert_eq!(Affine2::IDENTITY.similarity_scale(), Some(1.0));
        assert_eq!(Affine2::IDENTITY.rotation(), 0.0);
    }

    #[test]
    fn a_placement_scales_then_rotates_then_translates() {
        let t = Affine2::placement(p(100.0, 100.0), 2.0, 2.0, FRAC_PI_2);
        let q = t.apply(p(10.0, 0.0));
        assert!((q.x - 100.0).abs() < 1e-12 && (q.y - 120.0).abs() < 1e-12);
        assert!((t.similarity_scale().unwrap() - 2.0).abs() < 1e-12);
        assert!((t.rotation() - FRAC_PI_2).abs() < 1e-12);
    }

    #[test]
    fn three_nested_placements_compose_to_the_declared_world_position() {
        // A line (0,0)-(10,0) in block C; C placed in B at (5,0) turned a
        // quarter; B placed in A at the origin scaled 2; A placed in the
        // drawing at (100,100).
        let c_in_b = Affine2::placement(p(5.0, 0.0), 1.0, 1.0, FRAC_PI_2);
        let b_in_a = Affine2::placement(p(0.0, 0.0), 2.0, 2.0, 0.0);
        let a_in_world = Affine2::placement(p(100.0, 100.0), 1.0, 1.0, 0.0);
        let world = c_in_b.then(&b_in_a).then(&a_in_world);
        let start = world.apply(p(0.0, 0.0));
        let end = world.apply(p(10.0, 0.0));
        assert!((start.x - 110.0).abs() < 1e-12 && (start.y - 100.0).abs() < 1e-12);
        assert!((end.x - 110.0).abs() < 1e-12 && (end.y - 120.0).abs() < 1e-12);
        assert!((world.similarity_scale().unwrap() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn then_is_composition_for_any_pair_of_maps() {
        let inner = Affine2 {
            a: 1.5,
            b: 0.25,
            c: -0.5,
            d: 2.0,
            e: 3.0,
            f: -1.0,
        };
        let outer = Affine2 {
            a: 0.0,
            b: -1.0,
            c: 1.0,
            d: 0.0,
            e: 10.0,
            f: 20.0,
        };
        let q = p(7.0, -2.0);
        let composed = inner.then(&outer).apply(q);
        let stepwise = outer.apply(inner.apply(q));
        assert!((composed.x - stepwise.x).abs() < 1e-12);
        assert!((composed.y - stepwise.y).abs() < 1e-12);
    }

    fn insert(at: Point3D, rotation: f64, extrusion: Point3D) -> InsertEntity {
        use crate::model::{Confidence, EntityCommon, EntityId, Origin, Ref};
        InsertEntity {
            common: EntityCommon {
                id: EntityId::new(1),
                origin: Origin::Vector,
                confidence: Confidence::High,
                source_handle: Ref::Absent,
                layer: Ref::Resolved("0".to_string()),
                color_index: 256,
                true_color: None,
                invisible: false,
            },
            block_name: Ref::Resolved("B".to_string()),
            insertion_point: at,
            scale: Point3D {
                x: 1.0,
                y: 1.0,
                z: 1.0,
            },
            rotation,
            attribs: Vec::new(),
            extrusion,
        }
    }

    fn v(x: f64, y: f64, z: f64) -> Point3D {
        Point3D { x, y, z }
    }

    fn close(a: Point2D, b: Point2D) -> bool {
        (a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12
    }

    #[test]
    fn an_insert_with_the_usual_normal_is_placed_in_the_world_axes() {
        let i = insert(v(10.0, 5.0, 3.0), FRAC_PI_2, WORLD_Z);
        assert_eq!(
            Affine2::from_insert(&i),
            Affine2::placement(p(10.0, 5.0), 1.0, 1.0, FRAC_PI_2)
        );
    }

    #[test]
    fn a_mirrored_insert_lands_across_the_y_axis() {
        // The OCS of normal (0, 0, -1) has its x axis along world -x: the
        // stated insertion point (10, 5) is (-10, 5) in the world, and a
        // block point one unit along the block's x axis goes one unit
        // further left.
        let t = Affine2::from_insert(&insert(v(10.0, 5.0, 0.0), 0.0, v(0.0, 0.0, -1.0)));
        assert!(close(t.apply(p(0.0, 0.0)), p(-10.0, 5.0)));
        assert!(close(t.apply(p(1.0, 0.0)), p(-11.0, 5.0)));
        assert!(t.determinant() < 0.0, "a mirror image");
        // Turned a quarter in its OCS, the block's x axis runs up the world
        // y axis still, since the mirror leaves y alone.
        let t = Affine2::from_insert(&insert(v(10.0, 5.0, 0.0), FRAC_PI_2, v(0.0, 0.0, -1.0)));
        assert!(close(t.apply(p(1.0, 0.0)), p(-10.0, 6.0)));
        // A stated normal that is not of unit length names the same OCS.
        let long = Affine2::from_insert(&insert(v(10.0, 5.0, 0.0), 0.0, v(0.0, 0.0, -4.0)));
        assert!(close(long.apply(p(1.0, 0.0)), p(-11.0, 5.0)));
    }

    #[test]
    fn a_tilted_insert_is_seen_from_above_and_its_elevation_counts() {
        // Normal (1, 0, 0): the OCS x axis is world y, its y axis world z
        // and its z axis world x. From above, a block point (1, 0) placed at
        // OCS height 7 is at world (7, 1); the block's y axis points
        // straight up and flattens away.
        let t = Affine2::from_insert(&insert(v(0.0, 0.0, 7.0), 0.0, v(1.0, 0.0, 0.0)));
        assert!(close(t.apply(p(1.0, 0.0)), p(7.0, 1.0)));
        assert!(close(t.apply(p(0.0, 1.0)), p(7.0, 0.0)));
        assert_eq!(t.similarity_scale(), None);
    }

    #[test]
    fn a_normal_that_is_not_a_direction_places_like_the_usual_one() {
        let usual = Affine2::from_insert(&insert(v(10.0, 5.0, 0.0), 0.3, WORLD_Z));
        for bad in [v(0.0, 0.0, 0.0), v(f64::NAN, 0.0, 1.0)] {
            assert_eq!(
                Affine2::from_insert(&insert(v(10.0, 5.0, 0.0), 0.3, bad)),
                usual
            );
        }
    }

    #[test]
    fn a_mirrored_block_inside_a_mirrored_block_is_the_right_way_round_again() {
        let inner = Affine2::from_insert(&insert(v(1.0, 0.0, 0.0), 0.0, v(0.0, 0.0, -1.0)));
        let outer = Affine2::from_insert(&insert(v(10.0, 0.0, 0.0), 0.0, v(0.0, 0.0, -1.0)));
        let world = inner.then(&outer);
        assert!(world.determinant() > 0.0);
        // Inner: (2, 0) -> OCS (3, 0) -> (-3, 0) in the outer block; outer:
        // -> OCS (7, 0) -> world (-7, 0).
        assert!(close(world.apply(p(2.0, 0.0)), p(-7.0, 0.0)));
    }

    #[test]
    fn per_axis_scale_and_mirroring_are_not_similarities() {
        let stretched = Affine2::placement(p(0.0, 0.0), 2.0, 1.0, 0.3);
        assert_eq!(stretched.similarity_scale(), None);
        let mirrored = Affine2::placement(p(0.0, 0.0), -1.0, 1.0, 0.0);
        assert_eq!(mirrored.similarity_scale(), None);
        assert!(mirrored.determinant() < 0.0);
        let flat = Affine2::placement(p(0.0, 0.0), 0.0, 1.0, 0.0);
        assert_eq!(flat.similarity_scale(), None);
    }
}
