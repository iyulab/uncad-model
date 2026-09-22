//! Each named case has the shape its description promises, stated once so
//! consumers can rely on it.

use uncad_model::model::{Entity, Ref};
use uncad_model_golden::cases;
use uncad_model_golden::{expected, write, EntitySpec};

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
    assert_eq!(
        String::from_utf8_lossy(&written.dxf)
            .matches("\nMISSING\n")
            .count(),
        1
    );
}

#[test]
fn g7_is_a_title_block_of_loose_texts_and_no_block() {
    let spec = cases::g7_loose_text_title_block();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    assert!(
        model.tables.block_records.len() == 2,
        "only the two space records"
    );
    let texts: Vec<&str> = model
        .entities
        .iter()
        .filter_map(|e| match e {
            Entity::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(
        texts,
        ["DWG NO", "BP-1042", "REV", "B", "MATERIAL", "SS400"]
    );
}

#[test]
fn g8_declares_cp949_and_stores_korean_as_cp949_bytes() {
    let spec = cases::g8_korean_title_block();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    // Declared in the header, right after the version.
    assert!(written
        .dxf
        .starts_with(b"  0\nSECTION\n  2\nHEADER\n  9\n$ACADVER\n  1\nAC1015\n  9\n$DWGCODEPAGE\n  3\nANSI_949\n"));
    // The title-block layer's name, as CP949 bytes in the file ...
    let cp949_title_layer: &[u8] = b"\xC7\xA5\xC1\xA6\xB6\xF5";
    assert!(
        written
            .dxf
            .windows(cp949_title_layer.len())
            .any(|w| w == cp949_title_layer),
        "the layer name is not stored as CP949"
    );
    assert!(
        std::str::from_utf8(&written.dxf).is_err(),
        "the file is not UTF-8"
    );
    // ... and as UTF-8 in the expected model.
    let title_layer = "\u{D45C}\u{C81C}\u{B780}";
    assert!(model.tables.layers.contains_key(title_layer));
    let Entity::Insert(insert) = &model.entities[1] else {
        panic!("the second entity is the INSERT");
    };
    assert_eq!(insert.attribs.len(), 3);
    assert_eq!(insert.attribs[2].text, "\u{D64D}\u{AE38}\u{B3D9}");
}

#[test]
#[should_panic(expected = "is not ASCII")]
fn an_ascii_case_refuses_non_ascii_text() {
    let mut spec = cases::g7_loose_text_title_block();
    if let EntitySpec::Text { text, .. } = &mut spec.entities[1] {
        *text = "\u{D45C}".to_string();
    }
    write(&spec);
}
