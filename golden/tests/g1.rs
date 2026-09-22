//! G1, the general part: the case's own shape, stated once so a consumer
//! can rely on it.

use uncad_model::model::{Entity, Ref};
use uncad_model_golden::cases::g1_general_part;
use uncad_model_golden::{expected, write};

#[test]
fn g1_has_the_advertised_shape() {
    let spec = g1_general_part();
    let written = write(&spec);
    let model = expected::model(&spec, &written);

    let count = |f: fn(&Entity) -> bool| model.entities.iter().filter(|e| f(e)).count();
    assert_eq!(
        count(|e| matches!(e, Entity::LwPolyline(_))),
        1,
        "one outline"
    );
    assert_eq!(count(|e| matches!(e, Entity::Circle(_))), 4, "four holes");
    assert_eq!(
        count(|e| matches!(e, Entity::Dimension(_))),
        4,
        "3 linear + 1 diameter"
    );
    assert_eq!(
        count(|e| matches!(e, Entity::Insert(_))),
        1,
        "one title block"
    );
    assert_eq!(
        count(|e| matches!(e, Entity::Attrib(_))),
        3,
        "three attribute values"
    );

    let insert = model
        .entities
        .iter()
        .find_map(|e| match e {
            Entity::Insert(i) => Some(i),
            _ => None,
        })
        .unwrap();
    assert_eq!(insert.block_name, Ref::Resolved("TITLEBLOCK".to_string()));
    let values: Vec<&str> = insert.attribs.iter().map(|a| a.text.as_str()).collect();
    assert_eq!(values, ["BP-1042", "B", "SS400"]);

    // Every reference resolves: the file declares every layer and block.
    for e in &model.entities {
        assert!(e.common().layer.is_resolved(), "{e:?}");
    }
    for name in ["TITLEBLOCK", "*D1", "*D2", "*D3", "*D4", "*Model_Space"] {
        assert!(model.tables.block_records.contains_key(name), "{name}");
    }
    assert_eq!(model.tables.layers.len(), 5, "0 + four named layers");
}

#[test]
fn g1_writes_a_stable_file() {
    // A change in the writer's output for the fixed case is a change every
    // consumer's expectations are built on; the length pins the shape
    // cheaply, and the handle count pins the entity inventory.
    let written = write(&g1_general_part());
    assert_eq!(written.handles.entities.len(), 10);
    assert_eq!(written.handles.blocks.len(), 5);
    assert_eq!(written.handles.attribs.len(), 1);
    let dxf = String::from_utf8(written.dxf).expect("an ASCII case is UTF-8");
    assert!(dxf.contains("  9\n$ACADVER\n  1\nAC1015\n"));
    assert!(
        !dxf.contains("$DWGCODEPAGE"),
        "an ASCII case declares no codepage"
    );
}

/// Every line a dimension block draws has a length: the vertical dimension
/// used to get a zero-length dimension line and two coincident zero-length
/// extension lines, because every dimension was drawn as a horizontal one.
#[test]
fn dimension_blocks_draw_no_zero_length_lines() {
    let spec = g1_general_part();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    let mut lines = 0;
    for (name, block) in &model.tables.block_records {
        if !name.starts_with("*D") {
            continue;
        }
        for e in &block.entities {
            if let Entity::Line(l) = e {
                lines += 1;
                assert_ne!(l.start_point, l.end_point, "{name}: a zero-length line");
            }
        }
    }
    assert_eq!(
        lines, 10,
        "three linear dimensions of three lines, one diameter of one"
    );
}
