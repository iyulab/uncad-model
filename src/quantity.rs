//! What quantity a numeric field of an entity measures.
//!
//! The model documents every number it carries -- "in the drawing's units",
//! "radians", "a ratio" -- but a consumer holding only the serialized form (a
//! JSON path such as `vertices[2].point.y`) cannot read that documentation.
//! [`quantity`] states it as a value, so a consumer that picks a tolerance, a
//! unit suffix or a display format for a number does not guess from the
//! field's name.
//!
//! The answer is a fact about the model, not an interpretation of a drawing:
//! the only per-entity input is a dimension's `measurement`, whose quantity
//! follows from the kind the drawing states for it. Where the model does not
//! say -- a path that is not a number, a name this table does not know, a
//! dimension whose kind is not stated -- the answer is `None`, never a guess.

use crate::model::{DimensionKind, Entity};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// What a numeric field measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Quantity {
    /// A distance or a coordinate, in the drawing's units -- or in the unit the
    /// model documents for the field where it names another (a viewport's lens
    /// length is in millimetres whatever the drawing's units are).
    Length,
    /// An angle, in radians.
    Angle,
    /// A dimensionless magnitude: a scale factor, a ratio, a bulge, a spline
    /// weight or knot, a gradient tint, an image's 0--100 settings.
    Ratio,
    /// A number that names something -- a color index, a packed color, a
    /// handle-like id, a lineweight code. A difference between two of them is
    /// not a size.
    Index,
    /// A component of a direction vector (an extrusion, a ray's direction, a
    /// tangent). Only the direction is meaningful, not the length.
    Direction,
    /// A whole-number count: a spline's degree, a cell's span, pixels.
    Count,
    /// Bit flags packed in an integer.
    Flag,
}

impl Quantity {
    /// The name this quantity serializes as.
    pub fn as_str(self) -> &'static str {
        match self {
            Quantity::Length => "length",
            Quantity::Angle => "angle",
            Quantity::Ratio => "ratio",
            Quantity::Index => "index",
            Quantity::Direction => "direction",
            Quantity::Count => "count",
            Quantity::Flag => "flag",
        }
    }
}

/// The quantity of the number at `path` in `entity`'s JSON form -- the path
/// the serialized entity has, with object keys joined by `.` and array
/// positions in brackets (`center.x`, `vertices[2].point.y`,
/// `common.linetype_scale`). `None` when the path is not a number this model
/// knows the quantity of.
pub fn quantity(entity: &Entity, path: &str) -> Option<Quantity> {
    classify(path, || match entity {
        Entity::Dimension(dimension) => dimension.kind.and_then(of_kind),
        _ => None,
    })
}

/// [`quantity`] for a consumer that holds the entity's JSON form rather than
/// the entity: `entity` is the serialized object, and a dimension's
/// `measurement` is read against the `kind` beside it. The answer is the same
/// one [`quantity`] gives for the entity this JSON came from.
pub fn quantity_of_json(entity: &Value, path: &str) -> Option<Quantity> {
    classify(path, || {
        let kind = entity.get("kind")?;
        of_kind(serde_json::from_value::<DimensionKind>(kind.clone()).ok()?)
    })
}

/// The one rule both entry points share. `measurement` is asked for only when
/// `path` is the top-level `measurement` -- the one number whose quantity
/// depends on the entity rather than on its name.
fn classify(path: &str, measurement: impl FnOnce() -> Option<Quantity>) -> Option<Quantity> {
    let names: Vec<&str> = path
        .split('.')
        .map(|segment| segment.split('[').next().unwrap_or(segment))
        .collect();
    let (&last, parents) = names.split_last()?;
    if matches!(last, "x" | "y" | "z") {
        return parents.last().map(|&parent| component(parent));
    }
    if last == "measurement" && parents.is_empty() {
        return measurement();
    }
    named(last, parents.last().copied())
}

/// A coordinate component (`x`, `y`, `z`): what it measures depends on the
/// vector it belongs to.
fn component(vector: &str) -> Quantity {
    match vector {
        "extrusion" | "direction" | "vector" | "miter_direction" | "start_tangent"
        | "end_tangent" | "normal" => Quantity::Direction,
        "scale" => Quantity::Ratio,
        "size_pixels" => Quantity::Count,
        // Points, offsets, an image's pixel vectors, an ellipse's major-axis
        // endpoint: positions or distances in the drawing's units.
        _ => Quantity::Length,
    }
}

/// A dimension's recorded measurement is a length, or radians for the
/// angular kinds.
fn of_kind(kind: DimensionKind) -> Option<Quantity> {
    Some(match kind {
        DimensionKind::Angular2Line | DimensionKind::Angular3Point => Quantity::Angle,
        DimensionKind::Rotated
        | DimensionKind::Aligned
        | DimensionKind::Diameter
        | DimensionKind::Radius
        | DimensionKind::Ordinate
        | DimensionKind::ArcLength => Quantity::Length,
    })
}

/// A scalar field by its name (and its parent's, where one name serves two
/// meanings).
fn named(name: &str, parent: Option<&str>) -> Option<Quantity> {
    Some(match name {
        "angle" | "start_angle" | "end_angle" | "oblique_angle" | "rotation" | "text_rotation"
        | "twist" => Quantity::Angle,

        "height" | "width" | "text_height" | "start_width" | "end_width" | "const_width"
        | "radius" | "elevation" | "reference_width" | "extents_width" | "extents_height"
        | "lens_length" | "length" | "arrow_size" | "column_widths" | "dash_pattern" => {
            Quantity::Length
        }

        "width_factor"
        | "bulge"
        | "scale"
        | "minor_major_ratio"
        | "axis_ratio"
        | "line_spacing_factor"
        | "linetype_scale"
        | "weights"
        | "knots"
        | "tint"
        | "brightness"
        | "contrast"
        | "fade" => Quantity::Ratio,

        "color_index" | "true_color" | "color1" | "color2" | "lineweight" | "transparency"
        | "id" | "viewport_id" | "variable" => Quantity::Index,
        // A reference's payload is the id it points at.
        "data" if parent == Some("annotation_id") => Quantity::Index,

        "degree" | "span_rows" | "span_columns" | "skipped_edges" => Quantity::Count,
        "display_flags" => Quantity::Flag,

        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::tests::one_of_each;
    use serde_json::Value;

    /// Every number in `value`, as the path the diff and the JSON consumer see.
    fn numeric_paths(value: &Value, path: String, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (key, inner) in map {
                    let next = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    numeric_paths(inner, next, out);
                }
            }
            Value::Array(items) => {
                for (i, inner) in items.iter().enumerate() {
                    numeric_paths(inner, format!("{path}[{i}]"), out);
                }
            }
            Value::Number(_) => out.push(path),
            _ => {}
        }
    }

    /// The table must answer for every number the model can write. A field
    /// added to an entity and forgotten here fails this test -- the one place
    /// the omission would otherwise go quiet.
    #[test]
    fn every_number_of_every_entity_kind_has_a_quantity() {
        let mut missing = Vec::new();
        for entity in one_of_each() {
            if matches!(entity, Entity::Unknown { .. }) {
                continue;
            }
            let value = serde_json::to_value(&entity).expect("serializable");
            let mut paths = Vec::new();
            numeric_paths(&value, String::new(), &mut paths);
            for path in paths {
                if quantity(&entity, &path).is_none() {
                    missing.push(format!("{} {path}", entity.type_name()));
                }
            }
        }
        assert!(missing.is_empty(), "numbers with no quantity: {missing:#?}");
    }

    fn line() -> Entity {
        one_of_each()
            .into_iter()
            .find(|e| matches!(e, Entity::Line(_)))
            .expect("a line")
    }

    #[test]
    fn a_coordinate_is_a_length_a_direction_component_is_a_direction() {
        let e = line();
        assert_eq!(quantity(&e, "start_point.x"), Some(Quantity::Length));
        assert_eq!(quantity(&e, "vertices[2].point.y"), Some(Quantity::Length));
        assert_eq!(quantity(&e, "extrusion.z"), Some(Quantity::Direction));
        assert_eq!(quantity(&e, "content.data.scale.x"), Some(Quantity::Ratio));
        assert_eq!(quantity(&e, "size_pixels.x"), Some(Quantity::Count));
    }

    #[test]
    fn the_common_fields_are_ratios_and_names_not_lengths() {
        let e = line();
        assert_eq!(quantity(&e, "common.linetype_scale"), Some(Quantity::Ratio));
        assert_eq!(quantity(&e, "common.color_index"), Some(Quantity::Index));
        assert_eq!(quantity(&e, "common.lineweight"), Some(Quantity::Index));
        assert_eq!(quantity(&e, "common.id"), Some(Quantity::Index));
        assert_eq!(quantity(&e, "vertices[0].bulge"), Some(Quantity::Ratio));
        assert_eq!(quantity(&e, "start_angle"), Some(Quantity::Angle));
        assert_eq!(quantity(&e, "knots[3]"), Some(Quantity::Ratio));
        assert_eq!(quantity(&e, "degree"), Some(Quantity::Count));
        assert_eq!(quantity(&e, "display_flags"), Some(Quantity::Flag));
        assert_eq!(quantity(&e, "annotation_id.data"), Some(Quantity::Index));
    }

    #[test]
    fn what_the_table_does_not_know_is_none_not_a_guess() {
        let e = line();
        assert_eq!(quantity(&e, "no_such_field"), None);
        assert_eq!(quantity(&e, "layer"), None);
        assert_eq!(quantity(&e, ""), None);
        // Only an annotation id's payload is an id; another `data` is not named.
        assert_eq!(quantity(&e, "content.data"), None);
    }

    #[test]
    fn a_dimension_measurement_follows_the_kind_the_drawing_states() {
        let mut dimension = one_of_each()
            .into_iter()
            .find_map(|e| match e {
                Entity::Dimension(d) => Some(d),
                _ => None,
            })
            .expect("a dimension");
        for (kind, expected) in [
            (Some(DimensionKind::Aligned), Some(Quantity::Length)),
            (Some(DimensionKind::Radius), Some(Quantity::Length)),
            (Some(DimensionKind::ArcLength), Some(Quantity::Length)),
            (Some(DimensionKind::Angular2Line), Some(Quantity::Angle)),
            (Some(DimensionKind::Angular3Point), Some(Quantity::Angle)),
            (None, None),
        ] {
            dimension.kind = kind;
            let entity = Entity::Dimension(dimension.clone());
            assert_eq!(quantity(&entity, "measurement"), expected, "{kind:?}");
        }
        // `measurement` of anything that is not a dimension is not this field.
        assert_eq!(quantity(&line(), "measurement"), None);
    }

    #[test]
    fn the_json_form_answers_as_the_entity_does_for_every_number_of_every_kind() {
        for entity in one_of_each() {
            if matches!(entity, Entity::Unknown { .. }) {
                continue;
            }
            let value = serde_json::to_value(&entity).expect("serializable");
            let mut paths = Vec::new();
            numeric_paths(&value, String::new(), &mut paths);
            for path in paths {
                assert_eq!(
                    quantity_of_json(&value, &path),
                    quantity(&entity, &path),
                    "{} {path}",
                    entity.type_name()
                );
            }
        }
        // A dimension that states no kind does not say, in either form.
        let none = serde_json::json!({ "type": "DIMENSION", "kind": null, "measurement": 1.0 });
        assert_eq!(quantity_of_json(&none, "measurement"), None);
        let angular = serde_json::json!({ "kind": "ANGULAR3_POINT", "measurement": 1.0 });
        assert_eq!(
            quantity_of_json(&angular, "measurement"),
            Some(Quantity::Angle)
        );
    }

    #[test]
    fn every_numeric_field_declared_in_the_model_has_a_quantity() {
        // The sample entities leave optional fields empty, so the test above cannot see
        // them; the declarations can. A scalar numeric field added to `model.rs` with no
        // entry in the table fails here. (Point fields are classified by their vector's
        // name, and `measurement` by its dimension's kind -- neither is a scalar name.)
        const NUMERIC: [&str; 10] = [
            "f64", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "usize", "isize",
        ];
        let mut missing = std::collections::BTreeSet::new();
        for line in include_str!("model.rs").lines() {
            let Some(rest) = line.trim().strip_prefix("pub ") else {
                continue;
            };
            let Some((name, ty)) = rest.split_once(": ") else {
                continue;
            };
            // A field declaration, not a function that happens to start `pub fn name(`.
            if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let numeric = ty
                .split(|c: char| !c.is_alphanumeric() && c != '_')
                .any(|token| NUMERIC.contains(&token));
            if numeric
                && !matches!(name, "x" | "y" | "z" | "measurement")
                && named(name, None).is_none()
            {
                missing.insert(name.to_string());
            }
        }
        assert!(
            missing.is_empty(),
            "numeric fields of the model with no quantity: {missing:?}"
        );
    }

    #[test]
    fn a_quantity_serializes_by_its_name() {
        let json = serde_json::to_string(&Quantity::Direction).expect("serializable");
        assert_eq!(json, "\"direction\"");
        let back: Quantity = serde_json::from_str("\"angle\"").expect("deserializable");
        assert_eq!(back, Quantity::Angle);
        assert_eq!(Quantity::Flag.as_str(), "flag");
    }
}
