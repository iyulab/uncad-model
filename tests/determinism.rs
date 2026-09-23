//! The same model must serialize to the same bytes, every time.
//!
//! Nothing else in the suite would notice a violation: a value that is right
//! but arrives in a different order on the next run passes every ordinary
//! assertion. The usual culprit is a hash-based collection whose iteration
//! order leaks into a result -- `std`'s hasher is seeded per instance, so the
//! order differs between two calls in one process, which is what makes the
//! repetition below able to catch it. (`clippy.toml` forbids those
//! collections outright; this test is the runtime check behind the lint.)

use std::collections::BTreeMap;
use uncad_model::model::{
    CircleEntity, Confidence, Entity, EntityCommon, EntityId, InsertEntity, LineEntity, Origin,
    Point3D, Ref,
};
use uncad_model::tables::{BlockRecord, LayerRecord, Tables};
use uncad_model::{CadDatabase, ReadDiagnostics, ToJsonOptions};

/// How often each output is regenerated. With four or more entries in a
/// leaked hash set, two consecutive identical orders are already unlikely;
/// this many leave no realistic chance of a false pass.
const RUNS: usize = 24;

fn common(handle: &str, layer: &str) -> EntityCommon {
    EntityCommon {
        id: EntityId::new(u64::from_str_radix(handle, 16).unwrap()),
        origin: Origin::Vector,
        confidence: Confidence::High,
        source_handle: Ref::Resolved(handle.to_string()),
        layer: Ref::Resolved(layer.to_string()),
        color_index: 256,
        true_color: None,
        invisible: false,
    }
}

fn p(x: f64, y: f64) -> Point3D {
    Point3D { x, y, z: 0.0 }
}

/// A small drawing with enough distinct table keys and entities that a
/// leaked hash order would show: six layers, four blocks, three mline
/// styles, and entities that reference them in a non-sorted order.
fn drawing() -> CadDatabase {
    let layer_names = ["Walls", "0", "Dims", "Text", "Hidden", "Center"];
    let mut layers = BTreeMap::new();
    for (i, name) in layer_names.iter().enumerate() {
        layers.insert(
            name.to_string(),
            LayerRecord {
                name: name.to_string(),
                color_index: i as i16 + 1,
                off: false,
                frozen: false,
                locked: false,
                plot: Some(true),
                lineweight: Some(-3),
                linetype: Ref::Resolved("CONTINUOUS".to_string()),
            },
        );
    }

    let mut entities = Vec::new();
    for (i, name) in layer_names.iter().enumerate() {
        let x = i as f64 * 10.0;
        entities.push(Entity::Line(LineEntity {
            common: common(&format!("{:X}", 0x20 + i), name),
            start_point: p(x, 0.0),
            end_point: p(x, 5.0),
        }));
        entities.push(Entity::Circle(CircleEntity {
            common: common(&format!("{:X}", 0x30 + i), name),
            center: p(x, 10.0),
            radius: 2.5,
            extrusion: Point3D {
                x: 0.0,
                y: 0.0,
                z: 1.0,
            },
        }));
    }

    let mut block_records = BTreeMap::new();
    for name in ["TITLE", "*Model_Space", "BOLT", "*Paper_Space"] {
        block_records.insert(
            name.to_string(),
            BlockRecord {
                name: name.to_string(),
                entities: entities.clone(),
            },
        );
    }
    entities.push(Entity::Insert(InsertEntity {
        common: common("40", "0"),
        block_name: Ref::Resolved("TITLE".to_string()),
        insertion_point: p(100.0, 100.0),
        scale: Point3D {
            x: 1.0,
            y: 1.0,
            z: 1.0,
        },
        rotation: 0.0,
        attribs: Vec::new(),
        extrusion: Point3D {
            x: 0.0,
            y: 0.0,
            z: -1.0,
        },
    }));

    let mut mlinestyles = BTreeMap::new();
    for (name, offsets) in [
        ("WALL", vec![-0.5, 0.5]),
        ("STANDARD", vec![0.0]),
        ("CAVITY", vec![-1.0, -0.5, 0.5, 1.0]),
    ] {
        mlinestyles.insert(name.to_string(), offsets);
    }

    CadDatabase {
        entities,
        tables: Tables {
            dim_styles: BTreeMap::new(),
            layers,
            block_records,
            mlinestyles,
            layouts: BTreeMap::new(),
        },
        read_diagnostics: ReadDiagnostics {
            warnings: vec!["UNHANDLEDCLASS".to_string(), "WRONGCRC".to_string()],
        },
    }
}

#[test]
fn repeated_serialization_is_byte_identical() {
    let db = drawing();
    let export = |pretty: bool| {
        db.to_json(ToJsonOptions { pretty })
            .expect("the model should serialize")
    };

    let compact = export(false);
    let pretty = export(true);
    for run in 1..RUNS {
        assert_eq!(compact, export(false), "run {run}: compact JSON changed");
        assert_eq!(pretty, export(true), "run {run}: pretty JSON changed");
    }
}

#[test]
fn a_rebuilt_model_serializes_identically() {
    // Two independently constructed values of the same drawing -- the maps
    // were filled in insertion order both times, and the bytes must not
    // depend on that history.
    let first = drawing().to_json(ToJsonOptions::default()).unwrap();
    for run in 1..RUNS {
        let again = drawing().to_json(ToJsonOptions::default()).unwrap();
        assert_eq!(
            first, again,
            "run {run}: a rebuilt model serialized differently"
        );
    }
}

#[test]
fn table_keys_are_serialized_sorted() {
    let json = drawing().to_json(ToJsonOptions::default()).unwrap();
    let layers = json.find("\"layers\":{").expect("layers object");
    let keys: Vec<usize> = [
        "\"0\":",
        "\"Center\":",
        "\"Dims\":",
        "\"Hidden\":",
        "\"Text\":",
        "\"Walls\":",
    ]
    .iter()
    .map(|k| json[layers..].find(k).expect(k))
    .collect();
    assert!(
        keys.windows(2).all(|w| w[0] < w[1]),
        "layer keys must appear in sorted order: {keys:?}"
    );
}

#[test]
fn the_model_round_trips_through_json() {
    let db = drawing();
    let json = db.to_json(ToJsonOptions::default()).unwrap();
    let back: CadDatabase = serde_json::from_str(&json).expect("the JSON should deserialize");
    assert_eq!(back, db);
}
