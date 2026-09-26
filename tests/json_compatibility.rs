//! A document an earlier version of this crate wrote still loads.
//!
//! `data/model-0.1.0.json` is the output of `uncad-model` 0.1.0 as published
//! on crates.io, unedited: a drawing with one entity of nearly every kind
//! that version knew, a block, a layer that is off, a dimension style and
//! an mline style. A field added since then is absent from it, and each
//! such field says in its own documentation what a document without it
//! reads as -- this test holds every one of them to that.
//!
//! What is not in the document is what changed shape after 0.1.0, so that
//! a 0.1.0 document carrying it is not expected to load: a SPLINE (it
//! gained the degree and knots that define its curve), a LEADER (its
//! annotation became a three-state reference), and a polyline's vertices
//! -- an LWPOLYLINE's, a POLYLINE_2D's and a HATCH polyline path's -- which
//! became objects of their own carrying each vertex's bulge and widths.
//! The document's HATCH therefore has no boundary path; 0.1.0 wrote it that
//! way for a hatch without one.

use uncad_model::model::{
    AttributeFlags, DimensionKind, Entity, HatchBoundaryPath, HorizontalJustification, Point3D,
    PolylineVertex, Ref, VerticalJustification,
};
use uncad_model::CadDatabase;

const Z_AXIS: Point3D = Point3D {
    x: 0.0,
    y: 0.0,
    z: 1.0,
};

fn load() -> CadDatabase {
    serde_json::from_str(include_str!("data/model-0.1.0.json"))
        .expect("a document 0.1.0 wrote deserializes")
}

#[test]
fn a_document_written_by_0_1_0_loads() {
    let db = load();
    assert_eq!(db.entities.len(), 29, "every entity comes back");
    assert!(db.read_diagnostics.is_clean());
    assert_eq!(db.tables.layers.len(), 2);
    assert_eq!(db.tables.block_records["TITLE"].entities.len(), 1);
    let style = &db.tables.dim_styles["ISO-25"];
    assert_eq!(style.decimal_places, Some(2));
    // The variables added since 0.1.0 are not stated by that document.
    assert_eq!((style.arrow_size, style.rounding), (None, None));
    assert_eq!(style.linear_unit_format, None);
    assert_eq!(style.zero_suppression, None);
    assert_eq!(style.angular_unit_format, None);
    assert_eq!(style.angular_decimal_places, None);
    assert_eq!(style.fraction_format, None);
    assert_eq!(db.tables.mlinestyles["STANDARD"], [0.5, -0.5]);
    assert!(db.tables.layouts.is_empty());
}

#[test]
fn fields_added_since_read_as_their_documentation_says() {
    let db = load();
    for e in &db.entities {
        assert!(!e.common().invisible, "{}", e.type_name());
        if let Entity::Ellipse(ellipse) = e {
            assert_eq!(ellipse.extrusion, Z_AXIS);
        }
        if let Entity::MText(m) = e {
            assert_eq!(m.attachment, None);
        }
        if let Entity::Light(light) = e {
            assert_eq!(light.light_type, None);
        }
    }
}

#[test]
fn an_ocs_entity_without_a_stated_normal_is_in_world_axes_at_elevation_zero() {
    let db = load();
    let mut seen = 0;
    for e in &db.entities {
        let (extrusion, elevation) = match e {
            Entity::Circle(c) => (c.extrusion, 0.0),
            Entity::Arc(a) => (a.extrusion, 0.0),
            Entity::LwPolyline(p) | Entity::Polyline2D(p) => (p.extrusion, p.elevation),
            Entity::Text(t) => (t.extrusion, t.elevation),
            Entity::Attrib(a) => (a.extrusion, a.elevation),
            Entity::Attdef(a) => (a.extrusion, a.elevation),
            Entity::Insert(i) => (i.extrusion, 0.0),
            Entity::Solid(s) | Entity::Trace(s) => (s.extrusion, s.elevation),
            _ => continue,
        };
        assert_eq!((extrusion, elevation), (Z_AXIS, 0.0), "{}", e.type_name());
        seen += 1;
    }
    assert_eq!(seen, 8, "every OCS kind the document holds");
}

#[test]
fn text_without_placement_groups_is_left_baseline_upright_and_unstyled() {
    let db = load();
    let mut seen = 0;
    let mut check = |h, v, ap: Option<_>, wf, oa, style: &Ref<String>| {
        assert_eq!(h, HorizontalJustification::Left);
        assert_eq!(v, VerticalJustification::Baseline);
        assert!(ap.is_none());
        assert_eq!((wf, oa), (1.0, 0.0));
        assert_eq!(*style, Ref::Absent);
        seen += 1;
    };
    for e in &db.entities {
        match e {
            Entity::Text(t) => check(
                t.horizontal_justification,
                t.vertical_justification,
                t.alignment_point,
                t.width_factor,
                t.oblique_angle,
                &t.style_name,
            ),
            Entity::Attrib(a) => check(
                a.horizontal_justification,
                a.vertical_justification,
                a.alignment_point,
                a.width_factor,
                a.oblique_angle,
                &a.style_name,
            ),
            Entity::Attdef(a) => check(
                a.horizontal_justification,
                a.vertical_justification,
                a.alignment_point,
                a.width_factor,
                a.oblique_angle,
                &a.style_name,
            ),
            _ => {}
        }
    }
    assert_eq!(seen, 3);
}

#[test]
fn an_attribute_without_flags_is_visible_and_variable() {
    let db = load();
    let mut seen = 0;
    for e in &db.entities {
        let flags = match e {
            Entity::Attrib(a) => a.flags,
            Entity::Attdef(a) => a.flags,
            Entity::Insert(i) => i.attribs[0].flags,
            _ => continue,
        };
        assert_eq!(flags, AttributeFlags::default());
        seen += 1;
    }
    assert_eq!(seen, 3);
}

#[test]
fn an_mtext_without_width_or_extents_is_unwrapped_and_unmeasured() {
    let db = load();
    let Some(Entity::MText(m)) = db.entities.iter().find(|e| e.type_name() == "MTEXT") else {
        panic!("the document has an MTEXT");
    };
    assert_eq!(m.reference_width, 0.0);
    assert_eq!((m.extents_width, m.extents_height), (None, None));
    assert_eq!(m.style_name, Ref::Absent);
}

#[test]
fn a_viewport_without_view_fields_states_no_view() {
    let db = load();
    let Some(Entity::Viewport(v)) = db.entities.iter().find(|e| e.type_name() == "VIEWPORT") else {
        panic!("the document has a VIEWPORT");
    };
    assert_eq!((v.view, v.on, v.viewport_id), (None, None, None));
    assert!(v.frozen_layers.is_empty());
}

#[test]
fn a_layer_without_state_fields_is_on_thawed_unlocked_and_unstated() {
    let db = load();
    for layer in db.tables.layers.values() {
        assert!(
            !layer.off && !layer.frozen && !layer.locked,
            "{}",
            layer.name
        );
        assert_eq!((layer.plot, layer.lineweight), (None, None));
        assert_eq!(layer.linetype, Ref::Absent);
    }
    // The document's HIDDEN layer is off the way 0.1.0 said it: by the
    // sign of its colour, which is still there to read.
    assert_eq!(db.tables.layers["HIDDEN"].color_index, -1);
}

#[test]
fn an_ordinate_dimension_without_its_axis_does_not_claim_one() {
    let db = load();
    let Some(Entity::Dimension(d)) = db.entities.iter().find(|e| e.type_name() == "DIMENSION")
    else {
        panic!("the document has a DIMENSION");
    };
    assert_eq!(d.kind, Some(DimensionKind::Ordinate));
    assert_eq!(d.ordinate_axis, None);
    // Written before a reader looked at a dimension's own style variables:
    // not "none", but "not looked at".
    assert_eq!(d.style_overrides, None);
}

#[test]
fn an_mline_without_its_scale_does_not_claim_one() {
    let db = load();
    let Some(Entity::MLine(m)) = db.entities.iter().find(|e| e.type_name() == "MLINE") else {
        panic!("the document has an MLINE");
    };
    assert_eq!(m.scale, None);
}

/// A polyline as 0.1.0 wrote it -- its vertices bare points -- does not
/// load: a vertex is an object of its own now. Pinned so that the day it
/// loads again is noticed, and the document above can carry polylines
/// again.
#[test]
fn a_polyline_0_1_0_wrote_does_not_load() {
    let lwpolyline = r#"{"type":"LWPOLYLINE","common":{"id":35,"origin":"VECTOR",
        "confidence":"HIGH","source_handle":{"type":"RESOLVED","data":"23"},
        "layer":{"type":"RESOLVED","data":"0"},"color_index":256,"true_color":null},
        "vertices":[{"x":0.0,"y":0.0},{"x":10.0,"y":0.0},{"x":10.0,"y":5.0}],
        "closed":true}"#;
    assert!(serde_json::from_str::<Entity>(lwpolyline).is_err());
    let path = r#"{"type":"POLYLINE","data":[{"x":0.0,"y":0.0},{"x":1.0,"y":0.0}]}"#;
    assert!(serde_json::from_str::<HatchBoundaryPath>(path).is_err());
}

/// A polyline written before a vertex carried its widths: the vertex has
/// none of its own.
#[test]
fn a_vertex_without_widths_has_none_of_its_own() {
    let vertex: PolylineVertex =
        serde_json::from_str(r#"{"point":{"x":1.0,"y":2.0},"bulge":0.5}"#).unwrap();
    assert_eq!((vertex.start_width, vertex.end_width), (0.0, 0.0));
    assert_eq!(vertex.bulge, 0.5);
}
