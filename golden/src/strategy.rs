//! `proptest` strategies over [`Spec`]: random synthetic drawings for
//! property checks. The invariants themselves live with whatever they check
//! -- the writer's and the oracle's in this crate's tests, an editor's or a
//! differ's wherever those are linked together -- so the generator is here,
//! once, where every such check can reach it.
//!
//! Generation is deterministic under `proptest`'s seeding, and a failing case
//! shrinks to a minimal spec.

use proptest::prelude::*;
use uncad_model::model::OrdinateAxis;

use crate::spec::{
    AttribSpec, BlockSpec, Codepage, EntitySpec, LayerSpec, LayerState, Spec, TextAlign, Vertex, Xy,
};

/// A coordinate that stays out of the ranges where `{:?}` formatting would
/// switch to exponent notation and where LibreDWG's own limits bite.
pub fn coord() -> impl Strategy<Value = f64> {
    (-5000i32..=5000, 0u8..=99).prop_map(|(whole, frac)| f64::from(whole) + f64::from(frac) / 100.0)
}

pub fn xy() -> impl Strategy<Value = Xy> {
    (coord(), coord()).prop_map(|(x, y)| Xy::new(x, y))
}

pub fn layer_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("0".to_string()),
        Just("OUTLINE".to_string()),
        Just("HOLES".to_string()),
        Just("DIMS".to_string()),
    ]
}

pub fn text() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .:-]{0,24}"
}

pub fn entity() -> impl Strategy<Value = EntitySpec> {
    prop_oneof![
        (layer_name(), xy(), xy()).prop_map(|(layer, start, end)| EntitySpec::Line {
            layer,
            start,
            end
        }),
        (layer_name(), xy(), 0.01f64..500.0, any::<bool>()).prop_map(
            |(layer, center, radius, mirrored)| EntitySpec::Circle {
                layer,
                center,
                radius,
                mirrored,
            }
        ),
        (
            layer_name(),
            xy(),
            0.01f64..500.0,
            0.0f64..360.0,
            0.0f64..360.0,
            any::<bool>()
        )
            .prop_map(|(layer, center, radius, start_deg, end_deg, mirrored)| {
                EntitySpec::Arc {
                    layer,
                    center,
                    radius,
                    start_deg,
                    end_deg,
                    mirrored,
                }
            }),
        (
            layer_name(),
            prop::collection::vec((xy(), bulge(), width(), width()), 2..12),
            any::<bool>(),
            any::<bool>(),
            any::<bool>(),
            prop_oneof![Just(0.0), width()],
        )
            .prop_map(|(layer, points, closed, curved, wide, const_width)| {
                EntitySpec::LwPolyline {
                    layer,
                    vertices: points
                        .iter()
                        .map(|&(at, bulge, start_width, end_width)| {
                            let vertex = Vertex::bulged(at, if curved { bulge } else { 0.0 });
                            if wide {
                                vertex.wide(start_width, end_width)
                            } else {
                                vertex
                            }
                        })
                        .collect(),
                    closed,
                    const_width,
                    elevation: 0.0,
                    mirrored: false,
                }
            }),
        (
            layer_name(),
            xy(),
            0.5f64..50.0,
            text(),
            0.0f64..360.0,
            prop::option::of((0u8..=5, 0u8..=3, xy())),
            prop_oneof![Just(1.0), 0.25f64..4.0],
        )
            .prop_map(
                |(layer, insert, height, text, rotation_deg, align, width_factor)| {
                    EntitySpec::Text {
                        layer,
                        insert,
                        height,
                        text,
                        rotation_deg,
                        // Left and baseline is no alignment at all: the
                        // format writes no alignment point for it.
                        align: align.filter(|&(h, v, _)| (h, v) != (0, 0)).map(
                            |(horizontal, vertical, at)| TextAlign {
                                horizontal,
                                vertical,
                                at,
                            },
                        ),
                        width_factor,
                        oblique_deg: 0.0,
                        style: None,
                        mirrored: false,
                    }
                },
            ),
        (layer_name(), xy(), xy(), xy(), text()).prop_map(|(layer, from, to, line_point, text)| {
            EntitySpec::LinearDimension {
                layer,
                from,
                to,
                line_point,
                text,
                measurement: None,
                style: None,
            }
        }),
        (layer_name(), xy(), xy(), text()).prop_map(|(layer, first, second, text)| {
            EntitySpec::DiameterDimension {
                layer,
                first,
                second,
                text,
                measurement: None,
                style: None,
            }
        }),
        (
            layer_name(),
            xy(),
            0.1f64..10.0,
            0.0f64..360.0,
            prop::collection::vec(
                (
                    text(),
                    xy(),
                    0.5f64..20.0,
                    prop::option::of((0u8..=5, 0u8..=3, xy())),
                    prop_oneof![Just(1.0), 0.25f64..4.0],
                    any::<bool>(),
                ),
                0..4
            ),
            any::<bool>(),
        )
            .prop_map(|(layer, insert, scale, rotation_deg, attribs, mirrored)| {
                EntitySpec::Insert {
                    layer,
                    block: "PART".to_string(),
                    insert,
                    scale,
                    rotation_deg,
                    attribs: attribs
                        .into_iter()
                        .enumerate()
                        .map(
                            |(i, (value, insert, height, align, width_factor, invisible))| {
                                AttribSpec {
                                    tag: format!("TAG{i}"),
                                    value,
                                    insert,
                                    height,
                                    align: align.filter(|&(h, v, _)| (h, v) != (0, 0)).map(
                                        |(horizontal, vertical, at)| TextAlign {
                                            horizontal,
                                            vertical,
                                            at,
                                        },
                                    ),
                                    width_factor,
                                    invisible,
                                }
                            },
                        )
                        .collect(),
                    mirrored,
                }
            }),
        later_entity(),
    ]
}

/// A bulge, in hundredths: straight segments, arcs of either sense, and
/// half circles (1) among them.
pub fn bulge() -> impl Strategy<Value = f64> {
    (-200i32..=200).prop_map(|b| f64::from(b) / 100.0)
}

/// A segment width, in tenths.
pub fn width() -> impl Strategy<Value = f64> {
    (0u8..=50).prop_map(|w| f64::from(w) / 10.0)
}

/// The kinds the golden writer learned after the first cases: justified
/// text, solids, polylines at an elevation, ordinate dimensions and
/// polygon meshes.
pub fn later_entity() -> impl Strategy<Value = EntitySpec> {
    let justified = (
        layer_name(),
        xy(),
        xy(),
        (0u8..6, 0u8..4),
        text(),
        (0.5f64..50.0, 0.0f64..360.0, 0.5f64..2.0, -30.0f64..30.0),
        prop_oneof![Just(None), Just(Some("STANDARD".to_string()))],
    )
        .prop_map(|(layer, insert, alignment, (h, v), text, numbers, style)| {
            let (height, rotation_deg, width_factor, oblique_deg) = numbers;
            EntitySpec::Text {
                layer,
                insert,
                height,
                text,
                rotation_deg,
                // Left and baseline is no justification at all: the format
                // writes no alignment point for it.
                align: ((h, v) != (0, 0)).then_some(TextAlign {
                    horizontal: h,
                    vertical: v,
                    at: alignment,
                }),
                width_factor,
                oblique_deg,
                style,
                mirrored: false,
            }
        });
    let solid = (layer_name(), xy(), xy(), xy(), xy(), any::<bool>()).prop_map(
        |(layer, a, b, c, d, mirrored)| EntitySpec::Solid {
            layer,
            corners: [a, b, c, d],
            mirrored,
        },
    );
    // A mirrored polyline at an elevation: the two fields a polyline states
    // its own coordinate system with.
    let lifted = (
        layer_name(),
        prop::collection::vec((xy(), bulge()), 2..8),
        any::<bool>(),
        -100i32..=100,
    )
        .prop_map(
            |(layer, points, closed, elevation)| EntitySpec::LwPolyline {
                layer,
                vertices: points
                    .into_iter()
                    .map(|(at, bulge)| Vertex::bulged(at, bulge))
                    .collect(),
                closed,
                const_width: 0.0,
                elevation: f64::from(elevation),
                mirrored: true,
            },
        );
    let ordinate = (layer_name(), xy(), xy(), xy(), any::<bool>(), text()).prop_map(
        |(layer, datum, feature, leader_end, x, text)| EntitySpec::OrdinateDimension {
            layer,
            datum,
            feature,
            leader_end,
            axis: if x { OrdinateAxis::X } else { OrdinateAxis::Y },
            text,
            measurement: None,
            style: None,
        },
    );
    let mesh = (2u16..5, 2u16..5)
        .prop_flat_map(|(m, n)| {
            (
                layer_name(),
                Just(m),
                Just(n),
                any::<bool>(),
                any::<bool>(),
                prop::collection::vec(
                    (coord(), coord(), coord()).prop_map(|(x, y, z)| [x, y, z]),
                    usize::from(m * n),
                ),
            )
        })
        .prop_map(
            |(layer, m, n, closed_m, closed_n, vertices)| EntitySpec::PolygonMesh {
                layer,
                m,
                n,
                closed_m,
                closed_n,
                vertices,
            },
        );
    prop_oneof![justified, solid, lifted, ordinate, mesh]
}

/// The name every reference [`undeclared`] rewrites points at: a block, a
/// text style and a dimension style the file never declares.
pub const UNDECLARED: &str = "UNDECLARED";

/// `entity` with the table entry it names by name -- the block a reference
/// inserts, the style a text or a dimension is written in -- replaced by
/// [`UNDECLARED`]. The file names it; the reader owes that name back,
/// unresolved. An entity that names nothing is returned as it is.
pub fn undeclared(entity: EntitySpec) -> EntitySpec {
    let mut e = entity;
    match &mut e {
        EntitySpec::Insert { block, .. } => *block = UNDECLARED.to_string(),
        EntitySpec::Text { style, .. }
        | EntitySpec::LinearDimension { style, .. }
        | EntitySpec::DiameterDimension { style, .. }
        | EntitySpec::ArcDimension { style, .. }
        | EntitySpec::OrdinateDimension { style, .. } => *style = Some(UNDECLARED.to_string()),
        _ => {}
    }
    e
}

/// A spec of up to 23 entities, one in four of them naming a table entry
/// the file never declares ([`undeclared`]).
pub fn spec() -> impl Strategy<Value = Spec> {
    prop::collection::vec(
        (entity(), 0u8..4).prop_map(|(e, k)| if k == 0 { undeclared(e) } else { e }),
        0..24,
    )
    .prop_map(|entities| Spec {
        codepage: Codepage::Ascii,
        insunits: None,
        dim_styles: Vec::new(),
        layers: vec![
            LayerSpec {
                name: "OUTLINE".to_string(),
                color_index: 7,
                state: LayerState::default(),
            },
            LayerSpec {
                name: "HOLES".to_string(),
                color_index: 1,
                state: LayerState::default(),
            },
            LayerSpec {
                name: "DIMS".to_string(),
                color_index: 3,
                state: LayerState::default(),
            },
        ],
        blocks: vec![BlockSpec {
            name: "PART".to_string(),
            entities: vec![EntitySpec::Circle {
                layer: "0".to_string(),
                center: Xy::new(0.0, 0.0),
                radius: 1.0,
                mirrored: false,
            }],
        }],
        entities,
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
    })
}
