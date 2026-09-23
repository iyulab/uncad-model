//! An entity's own coordinate system (OCS): the plane a CIRCLE, ARC,
//! polyline, SOLID, TRACE or HATCH is written in, given by its extrusion direction.
//!
//! The format fixes the system's axes from the extrusion alone (the
//! "arbitrary axis algorithm"): the X axis is the world Y axis crossed with
//! the extrusion when the extrusion is within 1/64 of the world Z axis, and
//! the world Z axis crossed with it otherwise; the Y axis completes a
//! right-handed system. A mirror copy's extrusion (0, 0, -1) gives an X axis
//! of (-1, 0, 0) -- the world x reversed.
//!
//! Every consumer that takes such an entity to the drawing's coordinates
//! needs the same axes for the same extrusion, so they live with the model
//! rather than in each of them -- the same reason [`Affine2`](crate::Affine2)
//! does. It is arithmetic on the entity's own extrusion; there is no guessing
//! in it.

use crate::model::Point3D;

/// The format's threshold for the arbitrary axis algorithm: an extrusion
/// whose x and y are both below it is "near the world Z axis".
const ARBITRARY_AXIS_LIMIT: f64 = 1.0 / 64.0;

/// Relative tolerance under which an extrusion counts as parallel to the
/// world Z axis (see [`Ocs::is_flat`]). Files write the default extrusion
/// with rounding residues of this order; a real tilt is many orders larger.
/// This one is ours, not the format's.
const FLAT_TOLERANCE: f64 = 1e-9;

/// The axes of the coordinate system whose Z axis is an entity's extrusion,
/// as world unit vectors.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ocs {
    x: Point3D,
    y: Point3D,
    z: Point3D,
}

impl Ocs {
    /// The world's own system: what the default extrusion (0, 0, 1) names.
    pub const WORLD: Ocs = Ocs {
        x: Point3D {
            x: 1.0,
            y: 0.0,
            z: 0.0,
        },
        y: Point3D {
            x: 0.0,
            y: 1.0,
            z: 0.0,
        },
        z: Point3D {
            x: 0.0,
            y: 0.0,
            z: 1.0,
        },
    };

    /// The system whose Z axis is `extrusion`, or `None` for a zero or
    /// non-finite extrusion, which names no plane.
    pub fn of(extrusion: Point3D) -> Option<Ocs> {
        let z = normalized(extrusion)?;
        let world = if z.x.abs() < ARBITRARY_AXIS_LIMIT && z.y.abs() < ARBITRARY_AXIS_LIMIT {
            Point3D {
                x: 0.0,
                y: 1.0,
                z: 0.0,
            }
        } else {
            Point3D {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            }
        };
        let x = normalized(cross(world, z))?;
        let y = cross(z, x);
        Some(Ocs { x, y, z })
    }

    /// The system's X axis, a world unit vector.
    pub fn x_axis(self) -> Point3D {
        self.x
    }

    /// The system's Y axis, a world unit vector.
    pub fn y_axis(self) -> Point3D {
        self.y
    }

    /// The system's Z axis -- the extrusion, normalized.
    pub fn z_axis(self) -> Point3D {
        self.z
    }

    /// `true` when the plane is the world's own -- the default extrusion,
    /// give or take the rounding files write it with.
    pub fn is_world(self) -> bool {
        self.is_flat() && self.z.z > 0.0
    }

    /// `true` when the plane is parallel to the world XY plane, facing
    /// either way (a mirror copy's plane is flat but not the world's).
    pub fn is_flat(self) -> bool {
        self.z.x.abs().max(self.z.y.abs()) <= FLAT_TOLERANCE * self.z.z.abs()
    }

    /// A point of this system in world coordinates.
    pub fn to_world(self, p: Point3D) -> Point3D {
        Point3D {
            x: self.x.x * p.x + self.y.x * p.y + self.z.x * p.z,
            y: self.x.y * p.x + self.y.y * p.y + self.z.y * p.z,
            z: self.x.z * p.x + self.y.z * p.y + self.z.z * p.z,
        }
    }
}

fn cross(a: Point3D, b: Point3D) -> Point3D {
    Point3D {
        x: a.y * b.z - a.z * b.y,
        y: a.z * b.x - a.x * b.z,
        z: a.x * b.y - a.y * b.x,
    }
}

fn normalized(v: Point3D) -> Option<Point3D> {
    let len = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    (len > 0.0 && len.is_finite()).then(|| Point3D {
        x: v.x / len,
        y: v.y / len,
        z: v.z / len,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p3(x: f64, y: f64, z: f64) -> Point3D {
        Point3D { x, y, z }
    }

    #[test]
    fn the_default_extrusion_is_the_world() {
        let o = Ocs::of(p3(0.0, 0.0, 1.0)).unwrap();
        assert!(o.is_world());
        assert_eq!(o, Ocs::WORLD);
        assert_eq!(o.to_world(p3(3.0, 4.0, 5.0)), p3(3.0, 4.0, 5.0));
    }

    #[test]
    fn a_mirror_copy_reverses_the_world_x() {
        let o = Ocs::of(p3(0.0, 0.0, -1.0)).unwrap();
        assert!(o.is_flat() && !o.is_world());
        assert_eq!(o.to_world(p3(3.0, 4.0, 2.0)), p3(-3.0, 4.0, -2.0));
    }

    #[test]
    fn a_tilted_plane_follows_the_arbitrary_axis_algorithm() {
        // Extrusion along world X: far from Z, so X axis = Z x N = (0, 1, 0)
        // and Y axis = N x X = (0, 0, 1). An OCS point (2, 3, 0) is at world
        // (0, 2, 3).
        let o = Ocs::of(p3(1.0, 0.0, 0.0)).unwrap();
        assert!(!o.is_flat());
        assert_eq!(o.x_axis(), p3(0.0, 1.0, 0.0));
        assert_eq!(o.y_axis(), p3(0.0, 0.0, 1.0));
        assert_eq!(o.to_world(p3(2.0, 3.0, 0.0)), p3(0.0, 2.0, 3.0));
    }

    #[test]
    fn no_plane_without_a_direction() {
        assert!(Ocs::of(p3(0.0, 0.0, 0.0)).is_none());
        assert!(Ocs::of(p3(f64::NAN, 0.0, 1.0)).is_none());
    }
}
