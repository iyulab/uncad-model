//! Each named case has the shape its description promises, stated once so
//! consumers can rely on it.

use uncad_model::model::{Entity, Point2D, Point3D, Ref, SegmentWidth};
use uncad_model::Affine2;
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
fn g10_keeps_the_name_of_the_block_the_file_never_defines() {
    let spec = cases::g10_unreferenced_insert();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    let Entity::Insert(i) = &model.entities[1] else {
        panic!("the second entity is the INSERT");
    };
    // The file names the block, so the name is what the reference owes
    // back; absent would claim the drawing pointed at nothing.
    assert_eq!(i.block_name, Ref::Unresolved("MISSING".into()));
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

/// G3 at two sizes, and what grows between them.
///
/// Every other case is too small for cost to show. This one asks whether
/// the numbers grow the way they should: ten times the copies should be
/// ten times the entities, and a copy's geometry should land exactly where
/// its index says -- a drawing that is merely *big* proves nothing if its
/// contents drifted on the way out.
#[test]
fn g3_grows_linearly_and_lands_where_its_index_says() {
    let small = cases::g3_many_parts(10);
    let large = cases::g3_many_parts(100);
    let one = cases::g1_general_part();

    assert_eq!(small.entities.len(), one.entities.len() * 10);
    assert_eq!(large.entities.len(), one.entities.len() * 100);

    // A copy is G1 moved, not G1 rewritten: the same kinds in the same
    // order, so a reader meets the same drawing ten times over.
    let kind = |e: &EntitySpec| std::mem::discriminant(e);
    let per_copy = one.entities.len();
    for copy in 0..10 {
        for (i, entity) in one.entities.iter().enumerate() {
            assert_eq!(
                kind(&small.entities[copy * per_copy + i]),
                kind(entity),
                "copy {copy}, entity {i}"
            );
        }
    }

    // The first entity of copy 3 sits exactly one pitch from where the
    // index says -- the grid is predictable, not merely spread out.
    let per_row = (10.0f64).sqrt().ceil() as usize;
    let (dx, dy) = cases::g3_offset(3, per_row);
    match (&one.entities[0], &small.entities[3 * per_copy]) {
        (
            EntitySpec::LwPolyline { vertices: a, .. },
            EntitySpec::LwPolyline { vertices: b, .. },
        ) => {
            assert_eq!(a.len(), b.len());
            assert_eq!(b[0].x, a[0].x + dx);
            assert_eq!(b[0].y, a[0].y + dy);
        }
        (a, b) => panic!("expected the same kind, got {a:?} and {b:?}"),
    }
}

/// The written file and the expected model grow with the drawing, and
/// nothing is lost on the way through the writer at size.
#[test]
fn g3_writes_and_reads_back_every_copy() {
    let spec = cases::g3_many_parts(40);
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    assert!(
        model.entities.len() >= spec.entities.len(),
        "every top-level entity is expected back: {} of {}",
        model.entities.len(),
        spec.entities.len()
    );
    // Every layer the single part declares is still declared once, not
    // forty times: the copies share the drawing's tables.
    let one = cases::g1_general_part();
    assert_eq!(
        model.tables.layers.len(),
        expected::model(&one, &write(&one)).tables.layers.len()
    );
}

/// G11 keeps every mirrored coordinate as the file states it, with the
/// normal beside it, and a mirrored block reference lands where its
/// placement -- the one OCS step the model takes -- puts it.
#[test]
fn g11_keeps_mirrored_coordinates_as_stated_and_places_the_block_through_its_normal() {
    let spec = cases::g11_mirrored_part();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    let mirrored = Point3D {
        x: 0.0,
        y: 0.0,
        z: -1.0,
    };

    // Six entities written with the normal (0, 0, -1), one without it.
    let dxf = String::from_utf8(written.dxf).expect("an ASCII case is UTF-8");
    assert_eq!(dxf.matches("210\n0.0\n220\n0.0\n230\n-1.0\n").count(), 6);

    let Entity::Circle(plain) = &model.entities[0] else {
        panic!("the first entity is the unmirrored circle");
    };
    let Entity::Circle(circle) = &model.entities[1] else {
        panic!("the second entity is the mirrored circle");
    };
    assert_eq!(plain.center, circle.center, "the same stated centre");
    assert_eq!(circle.extrusion, mirrored);
    assert_ne!(plain.extrusion, circle.extrusion);

    let Entity::LwPolyline(polyline) = &model.entities[3] else {
        panic!("the fourth entity is the polyline");
    };
    assert_eq!(polyline.bulges, [0.0, 1.0, 0.0, 0.0], "the stated sign");
    assert_eq!(polyline.elevation, 2.5);

    let Entity::Insert(insert) = &model.entities[6] else {
        panic!("the last entity is the block reference");
    };
    assert_eq!(insert.extrusion, mirrored);
    let placement = Affine2::from_insert(insert);
    let at = |x: f64, y: f64| placement.apply(Point2D { x, y });
    let close = |p: Point2D, x: f64, y: f64| (p.x - x).abs() < 1e-9 && (p.y - y).abs() < 1e-9;
    // The block's origin at the stated (50, 0), mirrored to (-50, 0); its
    // line's end, turned 30 degrees in the OCS, up and to the left of it.
    assert!(close(at(0.0, 0.0), -50.0, 0.0));
    let (sin, cos) = 30f64.to_radians().sin_cos();
    assert!(close(at(10.0, 0.0), -50.0 - 10.0 * cos, 10.0 * sin));
    assert!(placement.determinant() < 0.0, "a mirror image");
}

/// G12's polylines carry their bulges and widths, and the one whose file
/// spells "straight, constant width" out in full reads as the one that
/// states nothing.
#[test]
fn g12_carries_bulges_and_widths_and_folds_their_spellings() {
    let spec = cases::g12_curved_and_wide_polylines();
    let written = write(&spec);
    let model = expected::model(&spec, &written);
    let polylines: Vec<_> = model
        .entities
        .iter()
        .map(|e| match e {
            Entity::LwPolyline(p) => p,
            other => panic!("only polylines here, not {other:?}"),
        })
        .collect();
    assert_eq!(polylines.len(), 5);

    let [slot, arrow, bend, donut, spelled] = polylines.as_slice() else {
        unreachable!()
    };
    assert_eq!(slot.bulges, [0.0, 1.0, 0.0, 1.0]);
    assert!(slot.widths.is_empty());
    assert_eq!(
        arrow.widths[1],
        SegmentWidth {
            start: 4.0,
            end: 0.0
        }
    );
    assert!(arrow.bulges.is_empty());
    assert_eq!(bend.const_width, 1.5);
    assert!(bend.bulges[1] < 0.0, "the corner turns clockwise");
    assert_eq!((donut.vertices.len(), donut.closed), (2, true));
    assert_eq!(donut.bulges, [1.0, 1.0]);

    // The file does state zero bulges and constant widths for the last one.
    let dxf = String::from_utf8(written.dxf).expect("an ASCII case is UTF-8");
    let last = &dxf[dxf.rfind("LWPOLYLINE").unwrap()..];
    assert_eq!(last.matches(" 42\n0.0\n").count(), 2);
    assert_eq!(last.matches(" 40\n1.0\n").count(), 2);
    assert!(spelled.bulges.is_empty() && spelled.widths.is_empty());
    assert_eq!(spelled.const_width, 1.0);
}
