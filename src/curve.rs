//! The point at a parameter on a curve entity: an ELLIPSE, and a NURBS curve
//! -- a SPLINE or a HATCH boundary's spline edge.
//!
//! The format defines both curves completely by the entity's own fields. An
//! ellipse is its center, its major axis, the ratio of its minor axis, and
//! the normal of its plane; a NURBS curve is its degree, knots, control
//! points and weights. A point at a parameter is arithmetic on those, with
//! nothing to choose.
//!
//! What a consumer does with the curve does choose something, and stays with
//! the consumer: how densely to sample it, how far a sample may stray from
//! the curve, what to draw for a definition that does not add up, and what a
//! spline stored only by its fit points looks like between them -- the file
//! does not store that curve, so there is nothing here to evaluate.
//!
//! Every consumer that draws, measures or points at these curves needs the
//! same point for the same parameter, so the evaluation lives with the model
//! rather than in each of them -- the same reason [`BulgeArc`](crate::BulgeArc)
//! does.

use crate::model::{EllipseEntity, Point3D, SplineEntity};

impl EllipseEntity {
    /// The minor axis as a world vector: the unit normal of the ellipse's
    /// plane crossed with the major axis, scaled by `axis_ratio` -- the
    /// direction the parameter turns towards. With the default normal
    /// (0, 0, 1) that is the major axis turned a quarter turn
    /// counter-clockwise; a mirrored ellipse's (0, 0, -1) turns it the other
    /// way. `None` for a zero or non-finite normal, which names no plane.
    pub fn minor_axis(&self) -> Option<Point3D> {
        let (m, e) = (self.major_axis_endpoint, self.extrusion);
        let len = (e.x * e.x + e.y * e.y + e.z * e.z).sqrt();
        if !(len > 0.0 && len.is_finite()) {
            return None;
        }
        let (nx, ny, nz) = (e.x / len, e.y / len, e.z / len);
        Some(Point3D {
            x: (ny * m.z - nz * m.y) * self.axis_ratio,
            y: (nz * m.x - nx * m.z) * self.axis_ratio,
            z: (nx * m.y - ny * m.x) * self.axis_ratio,
        })
    }

    /// The point at parameter `t` (radians): `center + cos t * major +
    /// sin t * minor`. `start_angle` and `end_angle` are values of this
    /// parameter, not angles seen from the center. `None` where
    /// [`minor_axis`](Self::minor_axis) is.
    pub fn point_at(&self, t: f64) -> Option<Point3D> {
        let (c, m, n) = (self.center, self.major_axis_endpoint, self.minor_axis()?);
        Some(Point3D {
            x: c.x + t.cos() * m.x + t.sin() * n.x,
            y: c.y + t.cos() * m.y + t.sin() * n.y,
            z: c.z + t.cos() * m.z + t.sin() * n.z,
        })
    }
}

impl SplineEntity {
    /// The curve its control points define, or `None` when they do not
    /// define one: a spline stored only by its fit points, or a definition
    /// that does not add up (see [`Nurbs::new`]).
    pub fn nurbs(&self) -> Option<Nurbs> {
        Nurbs::new(
            self.degree,
            &self.knots,
            self.control_points.iter().copied(),
            &self.weights,
        )
    }
}

/// A NURBS curve with a complete definition, ready to evaluate.
#[derive(Debug, Clone, PartialEq)]
pub struct Nurbs {
    degree: usize,
    knots: Vec<f64>,
    /// Control points in homogeneous coordinates: (w x, w y, w z, w).
    control: Vec<[f64; 4]>,
}

impl Nurbs {
    /// The curve of `degree` over `knots`, `control` points and `weights`
    /// (one per control point; empty for a curve that is not rational, every
    /// weight being 1). `None` when those do not define a curve: the degree
    /// must be at least 1 and below the number of control points, the knot
    /// count must be control points + degree + 1, the knots finite and
    /// non-decreasing, and the weights, when given, one per control point.
    pub fn new(
        degree: u32,
        knots: &[f64],
        control: impl IntoIterator<Item = Point3D>,
        weights: &[f64],
    ) -> Option<Nurbs> {
        let p = usize::try_from(degree).ok()?;
        let control: Vec<[f64; 4]> = control
            .into_iter()
            .enumerate()
            .map(|(i, c)| {
                let w = weights.get(i).copied().unwrap_or(1.0);
                [c.x * w, c.y * w, c.z * w, w]
            })
            .collect();
        let n = control.len();
        if p == 0 || n <= p || knots.len() != n + p + 1 {
            return None;
        }
        if !weights.is_empty() && weights.len() != n {
            return None;
        }
        if knots.windows(2).any(|k| k[1] < k[0]) || knots.iter().any(|k| !k.is_finite()) {
            return None;
        }
        Some(Nurbs {
            degree: p,
            knots: knots.to_vec(),
            control,
        })
    }

    /// The parameter range the curve is defined on: from the knot at index
    /// `degree` to the one at index `control points`.
    pub fn domain(&self) -> (f64, f64) {
        (self.knots[self.degree], self.knots[self.control.len()])
    }

    /// The curve's knot spans that are not empty, in order: the pieces it
    /// is made of, each `(from, to)` with `from < to`.
    pub fn spans(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        (self.degree..self.control.len())
            .map(|k| (self.knots[k], self.knots[k + 1]))
            .filter(|(a, b)| a < b)
    }

    /// The point at parameter `u`, by de Boor's algorithm in homogeneous
    /// coordinates, so the weights are exact. `None` when `u` is outside
    /// [`domain`](Self::domain) (or not finite), or when the weights put the
    /// point at infinity there.
    ///
    /// A knot shared by two pieces belongs to the piece that starts at it;
    /// the domain's end belongs to the last piece.
    pub fn point_at(&self, u: f64) -> Option<Point3D> {
        let (from, to) = self.domain();
        if !(from <= u && u <= to) {
            return None;
        }
        let span = (self.degree..self.control.len())
            .rev()
            .find(|&k| self.knots[k] <= u && self.knots[k] < self.knots[k + 1])?;
        let [x, y, z, w] = self.de_boor(span, u);
        if w == 0.0 || !w.is_finite() {
            return None;
        }
        Some(Point3D {
            x: x / w,
            y: y / w,
            z: z / w,
        })
    }

    /// The homogeneous point at `u` in knot span `k`
    /// (`knots[k] <= u <= knots[k + 1]`).
    fn de_boor(&self, k: usize, u: f64) -> [f64; 4] {
        let (p, knots) = (self.degree, &self.knots);
        let mut d: Vec<[f64; 4]> = (0..=p).map(|j| self.control[j + k - p]).collect();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = j + k - p;
                let denom = knots[i + p + 1 - r] - knots[i];
                let alpha = if denom == 0.0 {
                    0.0
                } else {
                    (u - knots[i]) / denom
                };
                let prev = d[j - 1];
                for (value, before) in d[j].iter_mut().zip(prev) {
                    *value = (1.0 - alpha) * before + alpha * *value;
                }
            }
        }
        d[p]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Confidence, EntityCommon, EntityId, EntityLinetype, Origin, Ref};
    use std::f64::consts::{FRAC_1_SQRT_2, FRAC_PI_2, PI};

    fn p(x: f64, y: f64) -> Point3D {
        Point3D { x, y, z: 0.0 }
    }

    fn near(a: Point3D, b: Point3D) -> bool {
        (a.x - b.x).abs() < 1e-12 && (a.y - b.y).abs() < 1e-12 && (a.z - b.z).abs() < 1e-12
    }

    fn common() -> EntityCommon {
        EntityCommon {
            id: EntityId::new(1),
            origin: Origin::Vector,
            confidence: Confidence::High,
            source_handle: Ref::Absent,
            layer: Ref::Absent,
            color_index: 7,
            true_color: None,
            invisible: false,
            linetype: EntityLinetype::ByLayer,
            linetype_scale: 1.0,
            lineweight: Some(-1),
            transparency: Some(0),
        }
    }

    fn ellipse(major: Point3D, ratio: f64, extrusion: Point3D) -> EllipseEntity {
        EllipseEntity {
            common: common(),
            center: p(10.0, 20.0),
            major_axis_endpoint: major,
            axis_ratio: ratio,
            start_angle: 0.0,
            end_angle: std::f64::consts::TAU,
            extrusion,
        }
    }

    #[test]
    fn an_ellipse_runs_from_its_major_axis_towards_its_minor_one() {
        let el = ellipse(
            p(4.0, 0.0),
            0.5,
            Point3D {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        );
        assert!(near(el.minor_axis().unwrap(), p(0.0, 2.0)));
        assert!(near(el.point_at(0.0).unwrap(), p(14.0, 20.0)));
        assert!(near(el.point_at(FRAC_PI_2).unwrap(), p(10.0, 22.0)));
        assert!(near(el.point_at(PI).unwrap(), p(6.0, 20.0)));
    }

    #[test]
    fn a_mirrored_ellipse_runs_the_other_way() {
        let el = ellipse(
            p(4.0, 0.0),
            0.5,
            Point3D {
                x: 0.0,
                y: 0.0,
                z: -1.0,
            },
        );
        assert!(near(el.point_at(FRAC_PI_2).unwrap(), p(10.0, 18.0)));
    }

    #[test]
    fn an_ellipse_whose_normal_names_no_plane_has_no_points() {
        let el = ellipse(
            p(4.0, 0.0),
            0.5,
            Point3D {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
        );
        assert_eq!(el.minor_axis(), None);
        assert_eq!(el.point_at(0.0), None);
    }

    #[test]
    fn a_rational_quadratic_with_the_standard_weights_is_an_exact_quarter_circle() {
        let c = Nurbs::new(
            2,
            &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            [p(1.0, 0.0), p(1.0, 1.0), p(0.0, 1.0)],
            &[1.0, FRAC_1_SQRT_2, 1.0],
        )
        .unwrap();
        assert_eq!(c.domain(), (0.0, 1.0));
        for i in 0..=16 {
            let q = c.point_at(i as f64 / 16.0).unwrap();
            assert!((q.x.hypot(q.y) - 1.0).abs() < 1e-12, "{q:?}");
        }
        assert_eq!(c.point_at(0.0), Some(p(1.0, 0.0)));
    }

    #[test]
    fn weights_pull_the_curve_by_the_rational_formula() {
        // A rational quadratic from (0, 20) to (0, 0) over (-8, 10), weights
        // 1, 0.5, 1: at u = 1/2 its x is
        // 2 * 0.25 * 0.5 * (-8) / (0.25 + 2 * 0.25 * 0.5 + 0.25) = -8/3.
        let c = Nurbs::new(
            2,
            &[0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
            [p(0.0, 20.0), p(-8.0, 10.0), p(0.0, 0.0)],
            &[1.0, 0.5, 1.0],
        )
        .unwrap();
        let mid = c.point_at(0.5).unwrap();
        assert!((mid.x - (-8.0 / 3.0)).abs() < 1e-12, "{mid:?}");
        assert!((mid.y - 10.0).abs() < 1e-12, "{mid:?}");
    }

    #[test]
    fn a_clamped_cubic_passes_the_bezier_midpoint() {
        let c = Nurbs::new(
            3,
            &[0.0, 0.0, 0.0, 0.0, 1.0, 1.0, 1.0, 1.0],
            [p(0.0, 0.0), p(1.0, 2.0), p(3.0, 2.0), p(4.0, 0.0)],
            &[],
        )
        .unwrap();
        // (P0 + 3 P1 + 3 P2 + P3) / 8.
        assert!(near(c.point_at(0.5).unwrap(), p(2.0, 1.5)));
        assert_eq!(c.point_at(1.0), Some(p(4.0, 0.0)));
    }

    #[test]
    fn the_curve_is_evaluated_in_three_dimensions() {
        let c = Nurbs::new(
            1,
            &[0.0, 0.0, 1.0, 1.0],
            [
                Point3D {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0,
                },
                Point3D {
                    x: 2.0,
                    y: 4.0,
                    z: 6.0,
                },
            ],
            &[],
        )
        .unwrap();
        assert!(near(
            c.point_at(0.5).unwrap(),
            Point3D {
                x: 1.0,
                y: 2.0,
                z: 3.0
            }
        ));
    }

    #[test]
    fn a_shared_knot_belongs_to_the_piece_that_starts_at_it() {
        // A degree-one curve with two pieces: the corner (2, 0) is where the
        // second piece starts, and the domain's end is the last piece's.
        let c = Nurbs::new(
            1,
            &[0.0, 0.0, 1.0, 2.0, 2.0],
            [p(0.0, 0.0), p(2.0, 0.0), p(2.0, 2.0)],
            &[],
        )
        .unwrap();
        assert_eq!(c.spans().collect::<Vec<_>>(), vec![(0.0, 1.0), (1.0, 2.0)]);
        assert_eq!(c.point_at(1.0), Some(p(2.0, 0.0)));
        assert_eq!(c.point_at(2.0), Some(p(2.0, 2.0)));
    }

    #[test]
    fn a_repeated_interior_knot_leaves_an_empty_span_out() {
        let c = Nurbs::new(
            2,
            &[0.0, 0.0, 0.0, 1.0, 1.0, 2.0, 2.0, 2.0],
            [
                p(0.0, 0.0),
                p(1.0, 1.0),
                p(2.0, 0.0),
                p(3.0, 1.0),
                p(4.0, 0.0),
            ],
            &[],
        )
        .unwrap();
        assert_eq!(c.spans().collect::<Vec<_>>(), vec![(0.0, 1.0), (1.0, 2.0)]);
        // A double knot in a quadratic interpolates its control point.
        assert!(near(c.point_at(1.0).unwrap(), p(2.0, 0.0)));
    }

    #[test]
    fn outside_the_domain_there_is_no_point() {
        let c = Nurbs::new(1, &[0.0, 0.0, 1.0, 1.0], [p(0.0, 0.0), p(1.0, 0.0)], &[]).unwrap();
        assert_eq!(c.point_at(-0.1), None);
        assert_eq!(c.point_at(1.1), None);
        assert_eq!(c.point_at(f64::NAN), None);
    }

    #[test]
    fn a_definition_that_does_not_add_up_is_no_curve() {
        let three = [p(0.0, 0.0), p(1.0, 1.0), p(2.0, 0.0)];
        // Three control points and degree 2 need six knots, not four.
        assert_eq!(Nurbs::new(2, &[0.0, 0.0, 1.0, 1.0], three, &[]), None);
        // Degree 0, and a degree the control points cannot carry.
        assert_eq!(Nurbs::new(0, &[0.0, 1.0, 2.0, 3.0], three, &[]), None);
        assert_eq!(Nurbs::new(3, &[0.0; 7], three, &[]), None);
        // Decreasing and non-finite knots; weights not one per point.
        let knots = [0.0, 0.0, 0.0, 1.0, 1.0, 1.0];
        assert_eq!(
            Nurbs::new(2, &[0.0, 0.0, 1.0, 0.5, 1.0, 1.0], three, &[]),
            None
        );
        assert_eq!(
            Nurbs::new(2, &[0.0, 0.0, 0.0, f64::NAN, 1.0, 1.0], three, &[]),
            None
        );
        assert_eq!(Nurbs::new(2, &knots, three, &[1.0, 1.0]), None);
        assert!(Nurbs::new(2, &knots, three, &[]).is_some());
    }

    #[test]
    fn a_fit_point_spline_has_no_curve_to_evaluate() {
        let s = SplineEntity {
            common: common(),
            degree: 3,
            closed: None,
            periodic: None,
            knots: Vec::new(),
            weights: Vec::new(),
            fit_points: vec![p(0.0, 0.0), p(1.0, 1.0)],
            control_points: Vec::new(),
            start_tangent: None,
            end_tangent: None,
        };
        assert_eq!(s.nurbs(), None);
    }
}
