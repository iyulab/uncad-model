//! Each named case has the shape its description promises, stated once so
//! consumers can rely on it.

use uncad_model::model::{Entity, Ref};
use uncad_model_golden::cases;
use uncad_model_golden::{expected, write};

#[test]
fn every_named_case_writes_and_has_an_expected_model() {
    for name in cases::NAMES {
        let spec = cases::by_name(name).expect(name);
        let written = write(&spec);
        let model = expected::model(&spec, &written);
        assert!(!written.dxf.is_empty(), "{name}");
        assert!(
            model.entities.len() >= spec.entities.len(),
            "{name}: every top-level entity is expected back"
        );
    }
    assert!(cases::by_name("g0").is_none());
}

#[test]
fn g2_nests_three_blocks_with_the_declared_transforms() {
    let spec = cases::g2_nested_blocks();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    for (block, inner) in [("A", "B"), ("B", "C")] {
        let record = &model.tables.block_records[block];
        assert_eq!(record.entities.len(), 1, "{block}");
        let Entity::Insert(i) = &record.entities[0] else {
            panic!("{block} holds an INSERT");
        };
        assert_eq!(i.block_name, Ref::Resolved(inner.to_string()));
    }
    let Entity::Insert(top) = &model.entities[0] else {
        panic!("the drawing inserts A");
    };
    assert_eq!(top.block_name, Ref::Resolved("A".to_string()));
    assert_eq!(
        (top.insertion_point.x, top.insertion_point.y),
        (100.0, 100.0)
    );
    let b = &model.tables.block_records["B"];
    let Entity::Insert(c_ref) = &b.entities[0] else {
        unreachable!()
    };
    assert!((c_ref.rotation - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
}

#[test]
fn g6_keeps_both_overlapping_lines_as_distinct_entities() {
    let spec = cases::g6_overlapping_lines();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    let lines: Vec<_> = model
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Line(l) => Some(l),
            _ => None,
        })
        .collect();
    assert_eq!(lines.len(), 2);
    assert_ne!(lines[0].common.id, lines[1].common.id);
    assert_eq!(lines[0].start_point, lines[1].start_point);
    assert_eq!(lines[0].end_point, lines[1].end_point);
}

#[test]
fn g9_has_two_title_blocks_with_two_drawing_numbers() {
    let spec = cases::g9_two_drawing_numbers();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    let numbers: Vec<&str> = model
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Insert(i) => i.attribs.first().map(|a| a.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(numbers, ["BP-1042", "BP-2077"]);
}

#[test]
fn g10_reports_the_undefined_block_reference_as_absent() {
    let spec = cases::g10_unreferenced_insert();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    let Entity::Insert(i) = &model.entities[1] else {
        panic!("the second entity is the INSERT");
    };
    assert_eq!(i.block_name, Ref::Absent);
    assert!(!model.tables.block_records.contains_key("MISSING"));
    // The writer declares no record and no definition for it: the name
    // appears exactly once, on the INSERT.
    assert_eq!(written.dxf.matches("
MISSING
").count(), 1);
}
