//! A MULTILEADER is drawn as its roots say: each line through its vertices
//! and on to its root's last leader line point, and each dogleg from there.

use uncad_model::model::{Dogleg, EntityCommon, LeaderRoot, MultiLeaderEntity};
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
    MultiLeaderEntity { common, leaders }
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
