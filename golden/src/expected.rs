//! Spec -> the model a parser is expected to produce from the written DXF.
//!
//! This is the oracle in the model's own terms: a consumer parses the DXF the
//! writer produced and compares against this value (with the tolerance its
//! own principles state for floating-point fields). Handles are the ones the
//! writer issued, layers resolve by name, and colors are BYLAYER throughout
//! because the writer never sets an entity color.
//!
//! The three markers a parser must state: the reference ID is the file
//! handle's value (a parser reading a file with unique handles mints IDs
//! from them, so the same entity gets the same ID whether the drawing is read
//! as DWG or as DXF), the origin is `Vector`, the confidence `High`.

use crate::spec::{EntitySpec, Spec, Xy};
use crate::writer::{midpoint, Written};
use std::collections::BTreeMap;
use uncad_model::model::{
    ArcEntity, AttdefEntity, AttribEntity, CircleEntity, Confidence, DimensionEntity,
    DimensionKind, DimensionPoints, Entity, EntityCommon, EntityId, InsertEntity, LineEntity,
    LwPolylineEntity, Origin, Point2D, Point3D, PolylineVertex, Ref, TextEntity, TextOverride,
};

/// The reference a dimension's style name becomes: resolved when the file
/// declares that style, unresolved -- carrying the name -- when it names one
/// the file never declares, absent when it names none at all.
fn style_ref(style: &Option<String>, spec: &Spec) -> Ref<String> {
    match style {
        None => Ref::Absent,
        Some(name) if spec.dim_styles.iter().any(|s| &s.name == name) => {
            Ref::Resolved(name.clone())
        }
        Some(name) => Ref::Unresolved(name.clone()),
    }
}

/// The spec's dimension text, read the way the format reads group 1.
fn text_override(text: &str) -> TextOverride {
    match text {
        "" | "<>" => TextOverride::Measured,
        " " => TextOverride::Suppressed,
        other => TextOverride::Literal(other.to_string()),
    }
}
use uncad_model::tables::{BlockRecord, DimStyleRecord, LayerRecord, Tables};
use uncad_model::{CadDatabase, ReadDiagnostics};

/// The model `written` should read back as. `written` must come from
/// [`crate::writer::write`] on the same `spec`.
pub fn model(spec: &Spec, written: &Written) -> CadDatabase {
    let mut layers = BTreeMap::new();
    layers.insert(
        "0".to_string(),
        LayerRecord {
            name: "0".to_string(),
            color_index: 7,
        },
    );
    for l in &spec.layers {
        layers.insert(
            l.name.clone(),
            LayerRecord {
                name: l.name.clone(),
                color_index: l.color_index,
            },
        );
    }

    // Top-level entities, in file order. An INSERT's attributes are listed
    // both on the INSERT and as top-level ATTRIB entities, as the model's
    // JSON documentation states a parser does.
    let mut entities = Vec::new();
    let mut dim_index = 0;
    for (e, &h) in spec.entities.iter().zip(&written.handles.entities) {
        let attribs = written
            .handles
            .attribs
            .iter()
            .find(|(insert, _)| *insert == h)
            .map(|(_, hs)| hs.as_slice())
            .unwrap_or(&[]);
        let dim_block = match e {
            EntitySpec::LinearDimension { .. }
            | EntitySpec::ArcDimension { .. }
            | EntitySpec::DiameterDimension { .. } => {
                dim_index += 1;
                Some(format!("*D{dim_index}"))
            }
            _ => None,
        };
        let entity = convert(e, h, attribs, dim_block.as_deref(), spec);
        if let Entity::Insert(insert) = &entity {
            let extra: Vec<Entity> = insert.attribs.iter().cloned().map(Entity::Attrib).collect();
            entities.push(entity);
            entities.extend(extra);
        } else {
            entities.push(entity);
        }
    }

    let mut block_records = BTreeMap::new();
    block_records.insert(
        "*Model_Space".to_string(),
        BlockRecord {
            name: "*Model_Space".to_string(),
            entities: entities
                .iter()
                .filter(|e| !matches!(e, Entity::Attrib(_)))
                .cloned()
                .collect(),
        },
    );
    block_records.insert(
        "*Paper_Space".to_string(),
        BlockRecord {
            name: "*Paper_Space".to_string(),
            entities: Vec::new(),
        },
    );
    for (name, handles) in &written.handles.blocks {
        let specs: Vec<EntitySpec> = if let Some(b) = spec.blocks.iter().find(|b| &b.name == name) {
            b.entities.clone()
        } else {
            dimension_block_entities(spec, name)
        };
        let block_entities = specs
            .iter()
            .zip(handles)
            .map(|(e, &h)| convert(e, h, &[], None, spec))
            .collect();
        block_records.insert(
            name.clone(),
            BlockRecord {
                name: name.clone(),
                entities: block_entities,
            },
        );
    }

    CadDatabase {
        entities,
        tables: Tables {
            // Only the variables the spec states are written, so everything
            // else stays "this style does not state it" -- the reader must
            // not fill those in. A spec with no styles declares no table at
            // all, which is itself worth pinning: then a dimension naming a
            // style is an unresolved reference.
            dim_styles: spec
                .dim_styles
                .iter()
                .map(|s| {
                    (
                        s.name.clone(),
                        DimStyleRecord {
                            name: s.name.clone(),
                            post: s.post.clone(),
                            decimal_places: s.decimal_places,
                            text_height: s.text_height,
                            ..DimStyleRecord::default()
                        },
                    )
                })
                .collect(),
            layers,
            block_records,
            mlinestyles: BTreeMap::new(),
        },
        read_diagnostics: ReadDiagnostics::default(),
    }
}

/// The entities the writer put in the anonymous block `name` (`*D<n>`): the
/// n-th dimension's drawn geometry. Mirrors the writer's construction so the
/// oracle and the file agree without sharing code paths that could both be
/// wrong the same way -- the two are compared by the property tests.
fn dimension_block_entities(spec: &Spec, name: &str) -> Vec<EntitySpec> {
    let n: usize = name
        .strip_prefix("*D")
        .and_then(|s| s.parse().ok())
        .expect("an anonymous dimension block is named *D<n>");
    let dim = spec
        .entities
        .iter()
        .filter(|e| {
            matches!(
                e,
                EntitySpec::LinearDimension { .. }
                    | EntitySpec::ArcDimension { .. }
                    | EntitySpec::DiameterDimension { .. }
            )
        })
        .nth(n - 1)
        .expect("the n-th dimension exists");
    crate::writer::dimension_geometry(dim)
}

fn common(handle: u32, layer: &str) -> EntityCommon {
    EntityCommon {
        id: EntityId::new(u64::from(handle)),
        origin: Origin::Vector,
        confidence: Confidence::High,
        source_handle: Ref::Resolved(format!("{handle:X}")),
        layer: Ref::Resolved(layer.to_string()),
        color_index: 256,
        true_color: None,
        invisible: false,
    }
}

fn p3(p: Xy) -> Point3D {
    Point3D {
        x: p.x,
        y: p.y,
        z: 0.0,
    }
}

fn p2(p: Xy) -> Point2D {
    Point2D { x: p.x, y: p.y }
}

fn convert(
    e: &EntitySpec,
    handle: u32,
    attrib_handles: &[u32],
    dim_block: Option<&str>,
    spec: &Spec,
) -> Entity {
    match e {
        EntitySpec::Line { layer, start, end } => Entity::Line(LineEntity {
            common: common(handle, layer),
            start_point: p3(*start),
            end_point: p3(*end),
        }),
        EntitySpec::Circle {
            layer,
            center,
            radius,
            mirrored,
        } => Entity::Circle(CircleEntity {
            common: common(handle, layer),
            center: p3(*center),
            radius: *radius,
            extrusion: extrusion(*mirrored),
        }),
        EntitySpec::Arc {
            layer,
            center,
            radius,
            start_deg,
            end_deg,
            mirrored,
        } => Entity::Arc(ArcEntity {
            extrusion: extrusion(*mirrored),
            common: common(handle, layer),
            center: p3(*center),
            radius: *radius,
            start_angle: start_deg.to_radians(),
            end_angle: end_deg.to_radians(),
        }),
        EntitySpec::LwPolyline {
            layer,
            vertices,
            closed,
            mirrored,
        } => Entity::LwPolyline(LwPolylineEntity {
            common: common(handle, layer),
            vertices: vertices
                .iter()
                .map(|v| PolylineVertex {
                    point: p2(v.at),
                    bulge: v.bulge,
                })
                .collect(),
            closed: *closed,
            elevation: 0.0,
            extrusion: extrusion(*mirrored),
        }),
        EntitySpec::Text {
            layer,
            insert,
            height,
            text,
            rotation_deg,
        } => Entity::Text(TextEntity {
            common: common(handle, layer),
            start_point: p2(*insert),
            text_height: *height,
            text: text.clone(),
            rotation: rotation_deg.to_radians(),
        }),
        EntitySpec::Attdef {
            layer,
            insert,
            height,
            tag,
            default,
            ..
        } => Entity::Attdef(AttdefEntity {
            common: common(handle, layer),
            start_point: p2(*insert),
            text_height: *height,
            tag: tag.clone(),
            default_value: default.clone(),
            rotation: 0.0,
        }),
        EntitySpec::Insert {
            layer,
            block,
            insert,
            scale,
            rotation_deg,
            attribs,
        } => Entity::Insert(InsertEntity {
            common: common(handle, layer),
            // A reference to a block the file never defines. The file names
            // the block (DXF group code 2), so the name is what the reader
            // owes back: unresolved, carrying that name -- never an empty
            // name, and never absent, which would claim the drawing pointed
            // at nothing.
            block_name: if spec.blocks.iter().any(|b| &b.name == block) {
                Ref::Resolved(block.clone())
            } else {
                Ref::Unresolved(block.clone())
            },
            insertion_point: p3(*insert),
            scale: Point3D {
                x: *scale,
                y: *scale,
                z: *scale,
            },
            rotation: rotation_deg.to_radians(),
            attribs: attribs
                .iter()
                .zip(attrib_handles)
                .map(|(a, &h)| AttribEntity {
                    common: common(h, layer),
                    start_point: p2(a.insert),
                    text_height: a.height,
                    tag: a.tag.clone(),
                    text: a.value.clone(),
                    rotation: 0.0,
                })
                .collect(),
        }),
        // The writer states group 2 (the anonymous block), 70, 10, 11, 1 and
        // the subtype's own points -- and deliberately not 42 or 3, so the
        // "the file did not say" values are exercised here too.
        EntitySpec::LinearDimension {
            layer,
            from,
            to,
            line_point,
            text,
            measurement,
            style,
        } => Entity::Dimension(DimensionEntity {
            common: common(handle, layer),
            block_name: Ref::Resolved(dim_block.expect("a dimension has a block").to_string()),
            kind: Some(DimensionKind::Rotated),
            measurement: *measurement,
            text_override: text_override(text),
            definition_point: Some(p3(*line_point)),
            text_midpoint: p2(midpoint(*from, *to)),
            points: DimensionPoints {
                extension1: Some(p3(*from)),
                extension2: Some(p3(*to)),
                radial: None,
                arc: None,
            },
            rotation: 0.0,
            text_rotation: 0.0,
            style_name: style_ref(style, spec),
        }),
        EntitySpec::ArcDimension {
            layer,
            from,
            to,
            center,
            line_point,
            text,
            measurement,
            style,
        } => Entity::Dimension(DimensionEntity {
            common: common(handle, layer),
            block_name: Ref::Resolved(dim_block.expect("a dimension has a block").to_string()),
            // Its group 70 says "three-point angular"; the entity it is says
            // otherwise, and the entity wins.
            kind: Some(DimensionKind::ArcLength),
            measurement: *measurement,
            text_override: text_override(text),
            definition_point: Some(p3(*line_point)),
            text_midpoint: p2(midpoint(*from, *to)),
            points: DimensionPoints {
                extension1: Some(p3(*from)),
                extension2: Some(p3(*to)),
                radial: Some(p3(*center)),
                arc: None,
            },
            rotation: 0.0,
            text_rotation: 0.0,
            style_name: style_ref(style, spec),
        }),
        EntitySpec::DiameterDimension {
            layer,
            first,
            second,
            text,
            measurement,
            style,
        } => Entity::Dimension(DimensionEntity {
            common: common(handle, layer),
            block_name: Ref::Resolved(dim_block.expect("a dimension has a block").to_string()),
            kind: Some(DimensionKind::Diameter),
            measurement: *measurement,
            text_override: text_override(text),
            definition_point: Some(p3(*first)),
            text_midpoint: p2(midpoint(*first, *second)),
            points: DimensionPoints {
                extension1: None,
                extension2: None,
                radial: Some(p3(*second)),
                arc: None,
            },
            rotation: 0.0,
            text_rotation: 0.0,
            style_name: style_ref(style, spec),
        }),
    }
}

/// The extrusion a mirrored entity writes, or the default.
fn extrusion(mirrored: bool) -> Point3D {
    Point3D {
        x: 0.0,
        y: 0.0,
        z: if mirrored { -1.0 } else { 1.0 },
    }
}
