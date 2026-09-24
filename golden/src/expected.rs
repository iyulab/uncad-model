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

use crate::spec::{
    AttribSpec, EntitySpec, HatchEdgeSpec, HatchShapeSpec, LayerSpec, LayoutSpec, Spec, TextAlign,
    Xy,
};
use crate::writer::{midpoint, space_names, Written};
use std::collections::BTreeMap;
use uncad_model::model::{
    ArcEntity, AttdefEntity, AttribEntity, AttributeFlags, CircleEntity, Confidence,
    DimensionEntity, DimensionKind, DimensionPoints, Entity, EntityCommon, EntityId,
    EntityLinetype, HatchBoundaryPath, HatchEdge, HatchEntity, HorizontalJustification,
    InsertEntity, LineEntity, LwPolylineEntity, Origin, Point2D, Point3D, PolylineVertex, Ref,
    Solid3DEntity, SolidEntity, TextEntity, TextOverride, VerticalJustification, ViewportEntity,
    ViewportView,
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

/// The reference a text's style name (DXF 7) becomes. A text that names a
/// style resolves when the file declares it and is unresolved, carrying the
/// name, when it does not. A text that names none stands for `STANDARD`,
/// the reference's default: it resolves to that entry when the file
/// declares one (table names are not case-sensitive) and is absent when it
/// does not -- then the drawing has no style it could mean.
fn text_style_ref(style: &Option<String>, spec: &Spec) -> Ref<String> {
    match style {
        Some(name) if spec.text_styles.contains(name) => Ref::Resolved(name.clone()),
        Some(name) => Ref::Unresolved(name.clone()),
        None => spec
            .text_styles
            .iter()
            .find(|s| s.eq_ignore_ascii_case("STANDARD"))
            .map_or(Ref::Absent, |s| Ref::Resolved(s.clone())),
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
use uncad_model::tables::{
    AngularUnitFormat, ArcSymbol, BlockRecord, DimStyleRecord, FractionFormat, LayerRecord,
    LayoutRecord, LinearUnitFormat, PlotSettings, Tables,
};
use uncad_model::{CadDatabase, ReadDiagnostics};

/// The model `written` should read back as. `written` must come from
/// [`crate::writer::write`] on the same `spec`.
pub fn model(spec: &Spec, written: &Written) -> CadDatabase {
    let mut layers = BTreeMap::new();
    let zero = LayerSpec {
        name: "0".to_string(),
        color_index: 7,
        state: Default::default(),
    };
    for l in std::iter::once(&zero).chain(&spec.layers) {
        layers.insert(l.name.clone(), layer(l));
    }

    // Top-level entities, in file order: the model's, then paper space's.
    // An INSERT's attributes are listed both on the INSERT and as top-level
    // ATTRIB entities, as the model's JSON documentation states a parser
    // does.
    let mut dim_index = 0;
    let mut top_level = |specs: &[EntitySpec], handles: &[u32]| {
        let mut entities = Vec::new();
        for (e, &h) in specs.iter().zip(handles) {
            let attribs = written
                .handles
                .attribs
                .iter()
                .find(|(insert, _)| *insert == h)
                .map(|(_, hs)| hs.as_slice())
                .unwrap_or(&[]);
            let dim_block = e.is_dimension().then(|| {
                dim_index += 1;
                format!("*D{dim_index}")
            });
            let entity = convert(e, h, attribs, dim_block.as_deref(), spec);
            if let Entity::Insert(insert) = &entity {
                let extra: Vec<Entity> =
                    insert.attribs.iter().cloned().map(Entity::Attrib).collect();
                entities.push(entity);
                entities.extend(extra);
            } else {
                entities.push(entity);
            }
        }
        entities
    };
    let model_space = top_level(&spec.entities, &written.handles.entities);
    let paper_space = top_level(&spec.paper_space, &written.handles.paper_entities);

    // A block record's own list does not repeat an INSERT's attributes.
    let owned = |entities: &[Entity]| -> Vec<Entity> {
        entities
            .iter()
            .filter(|e| !matches!(e, Entity::Attrib(_)))
            .cloned()
            .collect()
    };
    let mut block_records = BTreeMap::new();
    for name in space_names(spec) {
        let entities = match name.as_str() {
            "*Model_Space" => owned(&model_space),
            "*Paper_Space" => owned(&paper_space),
            _ => Vec::new(),
        };
        block_records.insert(name.clone(), BlockRecord { name, entities });
    }
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

    let mut entities = model_space;
    entities.extend(paper_space);
    CadDatabase {
        entities,
        tables: Tables {
            // Only the variables the spec states are written. An unwritten
            // variable whose starting value every template shares is that
            // value; one whose starting value depends on the template stays
            // "this style does not state it" -- the reader must not fill
            // those in. A spec with no styles declares no table at all,
            // which is itself worth pinning: then a dimension naming a style
            // is an unresolved reference.
            dim_styles: spec
                .dim_styles
                .iter()
                .map(|s| {
                    (
                        s.name.clone(),
                        DimStyleRecord {
                            name: s.name.clone(),
                            post: Some(s.post.clone().unwrap_or_default()),
                            scale: Some(1.0),
                            length_factor: Some(1.0),
                            tolerances: Some(false),
                            limits: Some(false),
                            tolerance_upper: Some(0.0),
                            tolerance_lower: Some(0.0),
                            decimal_places: s.decimal_places,
                            tolerance_decimal_places: None,
                            text_height: s.text_height,
                            arrow_size: s.arrow_size,
                            linear_unit_format: Some(
                                s.linear_unit_format.unwrap_or(LinearUnitFormat::Decimal),
                            ),
                            zero_suppression: s.zero_suppression,
                            rounding: Some(s.rounding.unwrap_or(0.0)),
                            angular_unit_format: Some(
                                s.angular_unit_format
                                    .unwrap_or(AngularUnitFormat::DecimalDegrees),
                            ),
                            angular_decimal_places: Some(s.angular_decimal_places.unwrap_or(0)),
                            fraction_format: Some(
                                s.fraction_format.unwrap_or(FractionFormat::Horizontal),
                            ),
                            // The files are R2000, which has the variable:
                            // unwritten is before the text.
                            arc_symbol: Some(s.arc_symbol.unwrap_or(ArcSymbol::BeforeText)),
                        },
                    )
                })
                .collect(),
            layers,
            block_records,
            mlinestyles: BTreeMap::new(),
            // The writer places no images.
            image_definitions: BTreeMap::new(),
            // A spec without layouts has no OBJECTS section, so there is no
            // LAYOUT object to read.
            layouts: spec
                .layouts
                .iter()
                .map(|l| (l.name.clone(), layout(l, &written.handles.paper_entities)))
                .collect(),
        },
        read_diagnostics: ReadDiagnostics::default(),
    }
}

/// The entities the writer put in the anonymous block `name` (`*D<n>`): the
/// n-th dimension's drawn geometry, counting the model's dimensions first
/// and paper space's after them. Mirrors the writer's construction so the
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
        .chain(&spec.paper_space)
        .filter(|e| e.is_dimension())
        .nth(n - 1)
        .expect("the n-th dimension exists");
    crate::writer::dimension_geometry(dim)
}

/// A layer as the writer declares it: with the CONTINUOUS linetype, and
/// with the state the spec gives it -- off as a negative colour, the plot
/// flag (290) and the lineweight (370) only where the spec states them.
fn layer(l: &LayerSpec) -> LayerRecord {
    LayerRecord {
        name: l.name.clone(),
        color_index: if l.state.off {
            -l.color_index
        } else {
            l.color_index
        },
        off: l.state.off,
        frozen: l.state.frozen,
        locked: l.state.locked,
        plot: l.state.plot,
        lineweight: l.state.lineweight,
        linetype: Ref::Resolved("CONTINUOUS".to_string()),
    }
}

fn layout(l: &LayoutSpec, paper_handles: &[u32]) -> LayoutRecord {
    let [margin_left, margin_bottom, margin_right, margin_top] = l.margins;
    LayoutRecord {
        name: l.name.clone(),
        tab_order: l.tab_order,
        block_name: Ref::Resolved(l.block.clone()),
        limits_min: p2(l.limits_min),
        limits_max: p2(l.limits_max),
        plot_settings: PlotSettings {
            paper_name: l.paper_name.clone(),
            paper_width: l.paper_size.0,
            paper_height: l.paper_size.1,
            margin_left,
            margin_bottom,
            margin_right,
            margin_top,
            plot_origin: p2(l.plot_origin),
            paper_units: Some(l.paper_units),
            rotation: Some(l.rotation),
            scale_numerator: l.scale.0,
            scale_denominator: l.scale.1,
        },
        paper_space_linetype_scaling: l.paper_space_linetype_scaling,
        limits_check: l.limits_check,
        extents_min: Some(p3(l.extents.0)),
        extents_max: Some(p3(l.extents.1)),
        // A model layout's active viewport is a VPORT record, not an
        // entity; a sheet's resolves to the viewport the writer named.
        active_viewport: match l.active_viewport {
            Some(i) if l.block != "*Model_Space" => {
                Ref::Resolved(EntityId::new(u64::from(paper_handles[i])))
            }
            _ => Ref::Absent,
        },
    }
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
        linetype: uncad_model::model::EntityLinetype::ByLayer,
        linetype_scale: 1.0,
        lineweight: Some(-1),
        transparency: None,
    }
}

fn p3(p: Xy) -> Point3D {
    Point3D {
        x: p.x,
        y: p.y,
        z: 0.0,
    }
}

/// The OCS normal of an entity whose file writes no group 210: the
/// reference's default, the world's own z axis.
const Z_AXIS: Point3D = Point3D {
    x: 0.0,
    y: 0.0,
    z: 1.0,
};

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
        EntitySpec::Styled { style, entity } => {
            let mut converted = convert(entity, handle, attrib_handles, dim_block, spec);
            let common = converted.common_mut();
            common.linetype = match style.linetype.as_deref() {
                None => EntityLinetype::ByLayer,
                Some(n) if n.eq_ignore_ascii_case("BYLAYER") => EntityLinetype::ByLayer,
                Some(n) if n.eq_ignore_ascii_case("BYBLOCK") => EntityLinetype::ByBlock,
                // The writer's LTYPE table declares CONTINUOUS alone.
                Some(n) if n == "CONTINUOUS" => EntityLinetype::Named(Ref::Resolved(n.to_string())),
                Some(n) => EntityLinetype::Named(Ref::Unresolved(n.to_string())),
            };
            common.linetype_scale = style.linetype_scale.unwrap_or(1.0);
            common.lineweight = Some(style.lineweight.unwrap_or(-1));
            converted
        }
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
            const_width,
            elevation,
            mirrored,
        } => {
            // Two spellings of one polyline are one model: every segment
            // stated as wide as the constant width, at both ends, is no
            // width of the vertices' own.
            let constant = vertices
                .iter()
                .all(|v| v.start_width == *const_width && v.end_width == *const_width);
            Entity::LwPolyline(LwPolylineEntity {
                common: common(handle, layer),
                vertices: vertices
                    .iter()
                    .map(|v| PolylineVertex {
                        point: p2(v.at),
                        bulge: v.bulge,
                        start_width: if constant { 0.0 } else { v.start_width },
                        end_width: if constant { 0.0 } else { v.end_width },
                    })
                    .collect(),
                closed: *closed,
                const_width: *const_width,
                elevation: *elevation,
                extrusion: extrusion(*mirrored),
            })
        }
        EntitySpec::Text {
            layer,
            insert,
            height,
            text,
            rotation_deg,
            align,
            width_factor,
            oblique_deg,
            style,
            mirrored,
        } => Entity::Text(TextEntity {
            common: common(handle, layer),
            start_point: p2(*insert),
            text_height: *height,
            text: text.clone(),
            rotation: rotation_deg.to_radians(),
            horizontal_justification: horizontal(*align),
            vertical_justification: vertical(*align),
            alignment_point: align.map(|a| p2(a.at)),
            width_factor: *width_factor,
            oblique_angle: oblique_deg.to_radians(),
            style_name: text_style_ref(style, spec),
            elevation: 0.0,
            extrusion: extrusion(*mirrored),
        }),
        EntitySpec::Hatch {
            layer,
            paths,
            style,
        } => Entity::Hatch(HatchEntity {
            common: common(handle, layer),
            boundary_paths: paths.iter().map(|p| hatch_path(&p.shape)).collect(),
            solid_fill: true,
            gradient: None,
            pattern_lines: Vec::new(),
            elevation: 0.0,
            extrusion: Z_AXIS,
            style: Some(*style),
        }),
        EntitySpec::Solid {
            layer,
            corners,
            mirrored,
        } => {
            let [corner1, corner2, corner3, corner4] = corners.map(p2);
            Entity::Solid(SolidEntity {
                common: common(handle, layer),
                corner1,
                corner2,
                corner3,
                corner4,
                elevation: 0.0,
                extrusion: extrusion(*mirrored),
            })
        }
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
            flags: AttributeFlags::default(),
            default_value: default.clone(),
            rotation: 0.0,
            horizontal_justification: HorizontalJustification::Left,
            vertical_justification: VerticalJustification::Baseline,
            alignment_point: None,
            width_factor: 1.0,
            oblique_angle: 0.0,
            style_name: text_style_ref(&None, spec),
            elevation: 0.0,
            extrusion: Z_AXIS,
        }),
        EntitySpec::Insert {
            layer,
            block,
            insert,
            scale,
            rotation_deg,
            attribs,
            mirrored,
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
                .map(|(a, &h)| attrib(a, h, layer, spec))
                .collect(),
            extrusion: extrusion(*mirrored),
        }),
        EntitySpec::PolygonMesh {
            layer,
            m,
            n,
            closed_m,
            closed_n,
            vertices,
        } => Entity::PolylineMesh(Solid3DEntity {
            common: common(handle, layer),
            wireframe_edges: mesh_wireframe(*m, *n, *closed_m, *closed_n, vertices),
            skipped_edges: 0,
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
            ordinate_axis: None,
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
            ordinate_axis: None,
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
            ordinate_axis: None,
        }),
        EntitySpec::OrdinateDimension {
            layer,
            datum,
            feature,
            leader_end,
            axis,
            text,
            measurement,
            style,
        } => Entity::Dimension(DimensionEntity {
            common: common(handle, layer),
            block_name: Ref::Resolved(dim_block.expect("a dimension has a block").to_string()),
            kind: Some(DimensionKind::Ordinate),
            measurement: *measurement,
            text_override: text_override(text),
            // Group 10 of an ordinate dimension is the datum it measures
            // from; its feature is group 13 and its leader's end group 14.
            definition_point: Some(p3(*datum)),
            text_midpoint: p2(*leader_end),
            points: DimensionPoints {
                extension1: Some(p3(*feature)),
                extension2: Some(p3(*leader_end)),
                radial: None,
                arc: None,
            },
            rotation: 0.0,
            text_rotation: 0.0,
            style_name: style_ref(style, spec),
            ordinate_axis: Some(*axis),
        }),
        EntitySpec::Viewport {
            layer,
            center,
            width,
            height,
            on,
            id,
            view_center,
            view_height,
            view_target,
            twist_deg,
            frozen_layers,
        } => Entity::Viewport(ViewportEntity {
            common: common(handle, layer),
            center: p3(*center),
            width: *width,
            height: *height,
            view: Some(ViewportView {
                center: p2(*view_center),
                height: *view_height,
                target: p3(*view_target),
                // The writer writes every view as a plan view with the
                // default lens.
                direction: Z_AXIS,
                twist: twist_deg.to_radians(),
                lens_length: 50.0,
            }),
            on: Some(*on),
            viewport_id: Some(*id),
            frozen_layers: frozen_layers
                .iter()
                .map(|name| Ref::Resolved(name.clone()))
                .collect(),
        }),
    }
}

/// An ATTRIB as the writer writes it: in the world's own axes, whatever its
/// INSERT's, with the invisible flag the spec gives it.
fn attrib(a: &AttribSpec, handle: u32, layer: &str, spec: &Spec) -> AttribEntity {
    AttribEntity {
        common: common(handle, layer),
        start_point: p2(a.insert),
        text_height: a.height,
        tag: a.tag.clone(),
        flags: AttributeFlags {
            invisible: a.invisible,
            ..AttributeFlags::default()
        },
        text: a.value.clone(),
        rotation: 0.0,
        horizontal_justification: horizontal(a.align),
        vertical_justification: vertical(a.align),
        alignment_point: a.align.map(|al| p2(al.at)),
        width_factor: a.width_factor,
        oblique_angle: 0.0,
        style_name: text_style_ref(&None, spec),
        elevation: 0.0,
        extrusion: Z_AXIS,
    }
}

/// A polygon mesh's grid lines in the order the model states for
/// [`Entity::PolylineMesh`]: with vertex `i * n + j` at row `i`, column `j`,
/// first `(i, j)-(i + 1, j)` row by row, then `(i, j)-(i, j + 1)` row by
/// row, the closing edges included where the mesh is closed.
fn mesh_wireframe(
    m: u16,
    n: u16,
    closed_m: bool,
    closed_n: bool,
    vertices: &[[f64; 3]],
) -> Vec<[Point3D; 2]> {
    let (m, n) = (usize::from(m), usize::from(n));
    let vertex = |i: usize, j: usize| {
        let [x, y, z] = vertices[i * n + j];
        Point3D { x, y, z }
    };
    let mut edges = Vec::new();
    for i in 0..if closed_m { m } else { m - 1 } {
        for j in 0..n {
            edges.push([vertex(i, j), vertex((i + 1) % m, j)]);
        }
    }
    for i in 0..m {
        for j in 0..if closed_n { n } else { n - 1 } {
            edges.push([vertex(i, j), vertex(i, (j + 1) % n)]);
        }
    }
    edges
}

/// The extrusion a mirrored entity writes, or the default.
/// A boundary path in the model's terms: angles in radians, as the
/// reader converts the file's degrees; a polyline path's vertices with their
/// bulges and no widths.
fn hatch_path(shape: &HatchShapeSpec) -> HatchBoundaryPath {
    match shape {
        HatchShapeSpec::Polyline(vertices) => HatchBoundaryPath::Polyline(
            vertices
                .iter()
                .map(|v| PolylineVertex {
                    point: p2(v.at),
                    bulge: v.bulge,
                    ..PolylineVertex::default()
                })
                .collect(),
        ),
        HatchShapeSpec::Edges(edges) => HatchBoundaryPath::Edges(
            edges
                .iter()
                .map(|e| match e {
                    HatchEdgeSpec::Line { start, end } => HatchEdge::Line {
                        start: p2(*start),
                        end: p2(*end),
                    },
                    HatchEdgeSpec::Arc {
                        center,
                        radius,
                        start_deg,
                        end_deg,
                        ccw,
                    } => HatchEdge::Arc {
                        center: p2(*center),
                        radius: *radius,
                        start_angle: start_deg.to_radians(),
                        end_angle: end_deg.to_radians(),
                        is_ccw: *ccw,
                    },
                    HatchEdgeSpec::Ellipse {
                        center,
                        major_end,
                        ratio,
                        start_deg,
                        end_deg,
                        ccw,
                    } => HatchEdge::Ellipse {
                        center: p2(*center),
                        end: p2(*major_end),
                        minor_major_ratio: *ratio,
                        start_angle: start_deg.to_radians(),
                        end_angle: end_deg.to_radians(),
                        is_ccw: *ccw,
                    },
                    HatchEdgeSpec::Spline {
                        degree,
                        rational,
                        periodic,
                        knots,
                        control_points,
                        weights,
                    } => HatchEdge::Spline {
                        degree: *degree,
                        rational: *rational,
                        periodic: *periodic,
                        knots: knots.clone(),
                        control_points: control_points.iter().map(|p| p2(*p)).collect(),
                        weights: weights.clone(),
                        fit_points: Vec::new(),
                        start_tangent: None,
                        end_tangent: None,
                    },
                })
                .collect(),
        ),
    }
}

fn extrusion(mirrored: bool) -> Point3D {
    Point3D {
        x: 0.0,
        y: 0.0,
        z: if mirrored { -1.0 } else { 1.0 },
    }
}

/// The model's horizontal justification for a spec's (DXF 72).
fn horizontal(align: Option<TextAlign>) -> HorizontalJustification {
    match align.map(|a| a.horizontal) {
        None | Some(0) => HorizontalJustification::Left,
        Some(1) => HorizontalJustification::Center,
        Some(2) => HorizontalJustification::Right,
        Some(3) => HorizontalJustification::Aligned,
        Some(4) => HorizontalJustification::Middle,
        Some(5) => HorizontalJustification::Fit,
        Some(other) => panic!("a spec states horizontal alignment {other}"),
    }
}

/// The model's vertical justification for a spec's.
fn vertical(align: Option<TextAlign>) -> VerticalJustification {
    match align.map(|a| a.vertical) {
        None | Some(0) => VerticalJustification::Baseline,
        Some(1) => VerticalJustification::Bottom,
        Some(2) => VerticalJustification::Middle,
        Some(3) => VerticalJustification::Top,
        Some(other) => panic!("a spec states vertical alignment {other}"),
    }
}
