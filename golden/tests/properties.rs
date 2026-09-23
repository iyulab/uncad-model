//! Property checks over randomly generated specs.
//!
//! The negative cells of the coverage matrix ("must never happen") cannot be
//! measured by one case: what must not happen must not happen for *any*
//! input. So these are invariants over generated specs, shrunk to a minimal
//! failing case by `proptest` when they fail. Generation is seeded and
//! deterministic, so a failure is reproducible.
//!
//! P3, determinism: the same spec always writes the same bytes and always
//! states the same expected model, byte for byte in JSON.

use proptest::prelude::*;
use uncad_model::model::{HorizontalJustification, OrdinateAxis, VerticalJustification};
use uncad_model::ToJsonOptions;
use uncad_model_golden::spec::{
    AttribSpec, BlockSpec, Codepage, EntitySpec, LayerSpec, LayerState, Spec, Vertex, Xy,
};
use uncad_model_golden::{expected, write};

/// A coordinate that stays out of the ranges where `{:?}` formatting would
/// switch to exponent notation and where LibreDWG's own limits bite.
fn coord() -> impl Strategy<Value = f64> {
    (-5000i32..=5000, 0u8..=99).prop_map(|(whole, frac)| f64::from(whole) + f64::from(frac) / 100.0)
}

fn xy() -> impl Strategy<Value = Xy> {
    (coord(), coord()).prop_map(|(x, y)| Xy::new(x, y))
}

fn layer_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("0".to_string()),
        Just("OUTLINE".to_string()),
        Just("HOLES".to_string()),
        Just("DIMS".to_string()),
    ]
}

fn text() -> impl Strategy<Value = String> {
    "[A-Za-z0-9 .:-]{0,24}"
}

fn entity() -> impl Strategy<Value = EntitySpec> {
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
        (layer_name(), xy(), 0.5f64..50.0, text(), 0.0f64..360.0).prop_map(
            |(layer, insert, height, text, rotation_deg)| EntitySpec::Text {
                layer,
                insert,
                height,
                text,
                rotation_deg,
                mirrored: false,
            }
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
            prop::collection::vec((text(), xy(), 0.5f64..20.0, any::<bool>()), 0..4)
        )
            .prop_map(
                |(layer, insert, scale, rotation_deg, attribs)| EntitySpec::Insert {
                    layer,
                    block: "PART".to_string(),
                    insert,
                    scale,
                    rotation_deg,
                    attribs: attribs
                        .into_iter()
                        .enumerate()
                        .map(|(i, (value, insert, height, invisible))| AttribSpec {
                            tag: format!("TAG{i}"),
                            value,
                            insert,
                            height,
                            invisible,
                        })
                        .collect(),
                    mirrored: false,
                }
            ),
        later_entity(),
    ]
}

/// A bulge, in hundredths: straight segments, arcs of either sense, and
/// half circles (1) among them.
fn bulge() -> impl Strategy<Value = f64> {
    (-200i32..=200).prop_map(|b| f64::from(b) / 100.0)
}

/// A segment width, in tenths.
fn width() -> impl Strategy<Value = f64> {
    (0u8..=50).prop_map(|w| f64::from(w) / 10.0)
}

/// The kinds the golden writer learned after the first cases: justified
/// text, solids, polylines at an elevation, ordinate dimensions and
/// polygon meshes.
fn later_entity() -> impl Strategy<Value = EntitySpec> {
    const HORIZONTAL: [HorizontalJustification; 6] = [
        HorizontalJustification::Left,
        HorizontalJustification::Center,
        HorizontalJustification::Right,
        HorizontalJustification::Aligned,
        HorizontalJustification::Middle,
        HorizontalJustification::Fit,
    ];
    const VERTICAL: [VerticalJustification; 4] = [
        VerticalJustification::Baseline,
        VerticalJustification::Bottom,
        VerticalJustification::Middle,
        VerticalJustification::Top,
    ];
    let justified = (
        layer_name(),
        xy(),
        xy(),
        (0usize..6, 0usize..4),
        text(),
        (0.5f64..50.0, 0.0f64..360.0, 0.5f64..2.0, -30.0f64..30.0),
        prop_oneof![Just(None), Just(Some("STANDARD".to_string()))],
    )
        .prop_map(|(layer, insert, alignment, (h, v), text, numbers, style)| {
            let (height, rotation_deg, width_factor, oblique_deg) = numbers;
            EntitySpec::JustifiedText {
                layer,
                insert,
                alignment,
                horizontal: HORIZONTAL[h],
                vertical: VERTICAL[v],
                height,
                text,
                rotation_deg,
                width_factor,
                oblique_deg,
                style,
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

fn spec() -> impl Strategy<Value = Spec> {
    prop::collection::vec(entity(), 0..24).prop_map(|entities| Spec {
        codepage: Codepage::Ascii,
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

proptest! {
    /// P3 for the writer: the same spec writes the same bytes, every time.
    #[test]
    fn writing_is_deterministic(spec in spec()) {
        let first = write(&spec);
        for _ in 0..8 {
            prop_assert_eq!(&write(&spec), &first);
        }
    }

    /// P3 for the oracle: the expected model serializes to the same bytes,
    /// every time, and round-trips through JSON.
    #[test]
    fn the_expected_model_is_deterministic_and_round_trips(spec in spec()) {
        let written = write(&spec);
        let model = expected::model(&spec, &written);
        let json = model.to_json(ToJsonOptions::default()).unwrap();
        for _ in 0..8 {
            let again = expected::model(&spec, &written).to_json(ToJsonOptions::default()).unwrap();
            prop_assert_eq!(&again, &json);
        }
        let back: uncad_model::CadDatabase = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back, model);
    }

    /// Every handle the writer issues is unique, and every entity of the
    /// expected model carries one of them.
    #[test]
    fn handles_are_unique_and_accounted_for(spec in spec()) {
        let written = write(&spec);
        let mut all: Vec<u32> = written.handles.entities.clone();
        for (_, hs) in &written.handles.blocks {
            all.extend(hs);
        }
        for (_, hs) in &written.handles.attribs {
            all.extend(hs);
        }
        let mut sorted = all.clone();
        sorted.sort_unstable();
        sorted.dedup();
        prop_assert_eq!(sorted.len(), all.len(), "duplicate handle");

        let model = expected::model(&spec, &written);
        for e in &model.entities {
            let handle = e.common().source_handle.resolved().expect("a file entity has a handle");
            let h = u32::from_str_radix(handle, 16).unwrap();
            prop_assert!(all.contains(&h), "entity handle {:X} was never issued", h);
            prop_assert_eq!(e.common().id.value(), u64::from(h), "the ID is the handle's value");
        }
        // Each issued handle appears in the DXF text as a code-5 pair.
        let dxf = String::from_utf8_lossy(&written.dxf);
        for h in &all {
            let needle = format!("  5\n{h:X}\n");
            prop_assert!(dxf.contains(&needle), "handle {:X} not written", h);
        }
    }

    /// The DXF text is well formed at the level a reader checks first: it
    /// alternates (code, value) lines, opens and closes every section, and
    /// declares every layer an entity uses.
    #[test]
    fn the_dxf_is_well_formed(spec in spec()) {
        let written = write(&spec);
        let dxf = String::from_utf8(written.dxf).expect("an ASCII case is UTF-8");
        let lines: Vec<&str> = dxf.lines().collect();
        prop_assert_eq!(lines.len() % 2, 0, "odd number of lines");
        for pair in lines.chunks(2) {
            prop_assert!(pair[0].trim().parse::<u16>().is_ok(), "bad group code {:?}", pair[0]);
        }
        prop_assert_eq!(dxf.matches("  0\nSECTION\n").count(), 4);
        prop_assert_eq!(dxf.matches("  0\nENDSEC\n").count(), 4);
        prop_assert!(dxf.ends_with("  0\nEOF\n"));
        for e in &spec.entities {
            let declared = format!("  2\n{}\n", e.layer());
            prop_assert!(dxf.contains(&declared), "layer {} not declared", e.layer());
        }
    }
}
