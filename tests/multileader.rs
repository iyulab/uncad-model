//! A MULTILEADER is drawn as its roots say: each line through its vertices
//! and on to its root's last leader line point, and each dogleg from there.

use uncad_model::model::{Dogleg, EntityCommon, LeaderLineType, LeaderRoot, MultiLeaderEntity};
use uncad_model::Point3D;

fn p(x: f64, y: f64) -> Point3D {
    Point3D { x, y, z: 0.0 }
}

fn multileader(leaders: Vec<LeaderRoot>) -> MultiLeaderEntity {
    let common: EntityCommon = serde_json::from_value(serde_json::json!({
        "id": 1, "origin": "VECTOR", "confidence": "HIGH",
        "source_handle": {"type": "ABSENT"}, "layer": {"type": "ABSENT"},
        "color_index": 256, "true_color": null, "invisible": false,
        "linetype": {"type": "BY_LAYER"}, "linetype_scale": 1.0,
        "lineweight": -1, "transparency": 0
    }))
    .expect("the reference fields deserialize");
    MultiLeaderEntity {
        common,
        leaders,
        line_type: None,
        arrow_size: None,
        content: None,
    }
}

#[test]
fn a_multileader_line_is_drawn_on_to_its_roots_last_point() {
    let m = multileader(vec![
        LeaderRoot {
            lines: vec![vec![p(0.0, 0.0)], vec![p(0.0, 5.0), p(3.0, 5.0)]],
            last_point: Some(p(10.0, 0.0)),
            dogleg: Some(Dogleg {
                direction: p(1.0, 0.0),
                length: 0.5,
            }),
        },
        // No last point: a line of one vertex draws nothing, and there is
        // no dogleg to draw from.
        LeaderRoot {
            lines: vec![vec![p(7.0, 7.0)], vec![p(1.0, 1.0), p(2.0, 2.0)]],
            last_point: None,
            dogleg: Some(Dogleg {
                direction: p(1.0, 0.0),
                length: 0.5,
            }),
        },
    ]);
    assert_eq!(
        m.drawn_lines(),
        [
            vec![p(0.0, 0.0), p(10.0, 0.0)],
            vec![p(0.0, 5.0), p(3.0, 5.0), p(10.0, 0.0)],
            vec![p(1.0, 1.0), p(2.0, 2.0)],
        ]
    );
    assert_eq!(m.doglegs(), [[p(10.0, 0.0), p(10.5, 0.0)]]);
}

// The line type, layer by layer: the entity's own when its override flag
// says so, otherwise its style's, then each line's own where it overrides.

const STRAIGHT: i64 = 1;
const SPLINE: i64 = 2;

#[test]
fn the_style_settles_the_type_unless_the_entity_overrides_it() {
    use LeaderLineType::*;
    // DXF 90 with bit 0x1 clear: the entity's 170 is not what is drawn.
    assert_eq!(
        LeaderLineType::resolve(Some(0x44400), Some(STRAIGHT), Some(SPLINE), []),
        Some(Spline)
    );
    assert_eq!(
        LeaderLineType::resolve(Some(0x44401), Some(SPLINE), Some(STRAIGHT), []),
        Some(Spline)
    );
    assert_eq!(
        LeaderLineType::resolve(Some(0x1), Some(0), Some(STRAIGHT), []),
        Some(Invisible)
    );
}

#[test]
fn a_line_that_overrides_states_its_own_type() {
    use LeaderLineType::*;
    // Every line overrides to the same type: that is the entity's type.
    assert_eq!(
        LeaderLineType::resolve(Some(0), None, Some(STRAIGHT), [(Some(1), Some(SPLINE))]),
        Some(Spline)
    );
    // A line whose flag is clear, or that states none (before R2010),
    // takes the entity's.
    assert_eq!(
        LeaderLineType::resolve(
            Some(0),
            None,
            Some(SPLINE),
            [(Some(0), Some(STRAIGHT)), (None, None)]
        ),
        Some(Spline)
    );
}

#[test]
fn what_the_layers_cannot_settle_is_unknown() {
    // The style is needed but not in the drawing.
    assert_eq!(
        LeaderLineType::resolve(Some(0), Some(STRAIGHT), None, []),
        None
    );
    // A code the format does not define.
    assert_eq!(
        LeaderLineType::resolve(Some(1), Some(7), Some(STRAIGHT), []),
        None
    );
    // No override flags stated: which layer applies is not known.
    assert_eq!(
        LeaderLineType::resolve(None, Some(STRAIGHT), Some(STRAIGHT), []),
        None
    );
    // Lines that come out different: one entity type cannot describe them.
    assert_eq!(
        LeaderLineType::resolve(
            Some(0),
            None,
            Some(STRAIGHT),
            [(Some(0), None), (Some(1), Some(SPLINE))]
        ),
        None
    );
}

// The arrowhead size, layer by layer: the context data's, then each line's
// own where its override flags (93) set bit 0x10.

#[test]
fn the_context_data_sizes_every_arrowhead_unless_a_line_overrides_it() {
    // No line states its own: the context data's size.
    assert_eq!(
        MultiLeaderEntity::resolve_arrow_size(Some(0.18), [(Some(0), Some(9.0)), (None, None)]),
        Some(0.18)
    );
    assert_eq!(
        MultiLeaderEntity::resolve_arrow_size(Some(4.0), []),
        Some(4.0)
    );
    // Every line overrides, all to the same size.
    assert_eq!(
        MultiLeaderEntity::resolve_arrow_size(
            Some(0.18),
            [(Some(0x10), Some(1.0)), (Some(0x11), Some(1.0))]
        ),
        Some(1.0)
    );
}

#[test]
fn an_arrowhead_size_the_layers_cannot_settle_is_unknown() {
    // The context data states none.
    assert_eq!(MultiLeaderEntity::resolve_arrow_size(None, []), None);
    // A size that is not a finite number of zero or more.
    assert_eq!(MultiLeaderEntity::resolve_arrow_size(Some(-1.0), []), None);
    assert_eq!(
        MultiLeaderEntity::resolve_arrow_size(Some(f64::NAN), []),
        None
    );
    // A line that overrides without stating a size.
    assert_eq!(
        MultiLeaderEntity::resolve_arrow_size(Some(0.18), [(Some(0x10), None)]),
        None
    );
    // Lines that come out different: one entity size cannot describe them.
    assert_eq!(
        MultiLeaderEntity::resolve_arrow_size(
            Some(0.18),
            [(Some(0), None), (Some(0x10), Some(1.0))]
        ),
        None
    );
}
