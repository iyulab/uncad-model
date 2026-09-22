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
use uncad_model::ToJsonOptions;
use uncad_model_golden::spec::{AttribSpec, BlockSpec, Codepage, EntitySpec, LayerSpec, Spec, Xy};
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
        (layer_name(), xy(), 0.01f64..500.0).prop_map(|(layer, center, radius)| {
            EntitySpec::Circle {
                layer,
                center,
                radius,
            }
        }),
        (
            layer_name(),
            xy(),
            0.01f64..500.0,
            0.0f64..360.0,
            0.0f64..360.0
        )
            .prop_map(
                |(layer, center, radius, start_deg, end_deg)| EntitySpec::Arc {
                    layer,
                    center,
                    radius,
                    start_deg,
                    end_deg
                }
            ),
        (
            layer_name(),
            prop::collection::vec(xy(), 2..12),
            any::<bool>()
        )
            .prop_map(|(layer, vertices, closed)| EntitySpec::LwPolyline {
                layer,
                vertices,
                closed
            }),
        (layer_name(), xy(), 0.5f64..50.0, text(), 0.0f64..360.0).prop_map(
            |(layer, insert, height, text, rotation_deg)| EntitySpec::Text {
                layer,
                insert,
                height,
                text,
                rotation_deg
            }
        ),
        (layer_name(), xy(), xy(), xy(), text()).prop_map(|(layer, from, to, line_point, text)| {
            EntitySpec::LinearDimension {
                layer,
                from,
                to,
                line_point,
                text,
            }
        }),
        (layer_name(), xy(), xy(), text()).prop_map(|(layer, first, second, text)| {
            EntitySpec::DiameterDimension {
                layer,
                first,
                second,
                text,
            }
        }),
        (
            layer_name(),
            xy(),
            0.1f64..10.0,
            0.0f64..360.0,
            prop::collection::vec((text(), xy(), 0.5f64..20.0), 0..4)
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
                        .map(|(i, (value, insert, height))| AttribSpec {
                            tag: format!("TAG{i}"),
                            value,
                            insert,
                            height,
                        })
                        .collect(),
                }
            ),
    ]
}

fn spec() -> impl Strategy<Value = Spec> {
    prop::collection::vec(entity(), 0..24).prop_map(|entities| Spec {
        codepage: Codepage::Ascii,
        layers: vec![
            LayerSpec {
                name: "OUTLINE".to_string(),
                color_index: 7,
            },
            LayerSpec {
                name: "HOLES".to_string(),
                color_index: 1,
            },
            LayerSpec {
                name: "DIMS".to_string(),
                color_index: 3,
            },
        ],
        blocks: vec![BlockSpec {
            name: "PART".to_string(),
            entities: vec![EntitySpec::Circle {
                layer: "0".to_string(),
                center: Xy::new(0.0, 0.0),
                radius: 1.0,
            }],
        }],
        entities,
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
