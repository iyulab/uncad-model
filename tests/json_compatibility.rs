//! A document an earlier version of this crate wrote still loads.
//!
//! `data/model-0.1.0.json` is the output of `uncad-model` 0.1.0 as published
//! on crates.io, unedited: a drawing with one entity of every kind that
//! version knew, a block, a layer that is off, a dimension style and an
//! mline style. A field added since then is absent from it, and each such
//! field says in its own documentation what a document without it reads
//! as -- this test holds every one of them to that.
//!
//! Two kinds are not in the document. A SPLINE's and a LEADER's shape
//! changed after 0.1.0 (a spline gained the degree and knots that define
//! its curve, a leader's annotation became a three-state reference), so a
//! 0.1.0 document carrying either is not expected to load.

use uncad_model::model::{Entity, Point3D};
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
    assert_eq!(db.entities.len(), 31, "every entity comes back");
    assert!(db.read_diagnostics.is_clean());
    assert_eq!(db.tables.layers.len(), 2);
    assert_eq!(db.tables.block_records["TITLE"].entities.len(), 1);
    assert_eq!(db.tables.dim_styles["ISO-25"].decimal_places, Some(2));
    assert_eq!(db.tables.mlinestyles["STANDARD"], [0.5, -0.5]);
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
    assert_eq!(seen, 10, "every OCS kind the document holds");
}
