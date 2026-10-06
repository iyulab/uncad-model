//! A golden case is a *spec*: the entities and values a synthetic drawing
//! contains, written as Rust data. The writer turns a spec into a DXF, and
//! the spec itself is the oracle for whatever a parser reads back out.
//!
//! Only what the model carries is representable here -- a spec that could
//! say more than the model would be an oracle nothing can be checked
//! against.

use uncad_model::model::{HatchStyle, OrdinateAxis};
use uncad_model::tables::{
    AngularUnitFormat, ArcSymbol, FractionFormat, LinearUnitFormat, PlotPaperUnits, PlotRotation,
};

/// How the writer encodes every string it emits, and what the file declares
/// in `$DWGCODEPAGE`. An R2000 DXF stores text as 8-bit bytes in the
/// drawing's codepage, not as UTF-8, so a case with non-ASCII text has to
/// say which codepage that is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Codepage {
    /// No `$DWGCODEPAGE` is written and every string must be 7-bit ASCII;
    /// the writer panics on anything else, so a case cannot silently depend
    /// on a reader's default codepage.
    #[default]
    Ascii,
    /// `$DWGCODEPAGE = ANSI_949`: Korean (Unified Hangul Code, the CP949
    /// superset of EUC-KR). Strings are encoded as CP949 bytes.
    Ansi949,
}

impl Codepage {
    /// The `$DWGCODEPAGE` value, or `None` when the header variable is not
    /// written.
    pub fn dxf_name(self) -> Option<&'static str> {
        match self {
            Codepage::Ascii => None,
            Codepage::Ansi949 => Some("ANSI_949"),
        }
    }
}

/// A whole synthetic drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    /// The text encoding of the written file. The spec's own strings are
    /// always UTF-8 `String`s -- the expected model carries them as such --
    /// and the writer encodes them on the way out.
    pub codepage: Codepage,
    /// The `$INSUNITS` code the header states; `None` writes no such
    /// variable.
    pub insunits: Option<u16>,
    /// Layers besides `0`, which always exists. Names must be unique.
    pub layers: Vec<LayerSpec>,
    /// Named block definitions an INSERT (or a DIMENSION) can refer to.
    pub blocks: Vec<BlockSpec>,
    /// DIMSTYLE table entries the file declares. A dimension naming one of
    /// these resolves; a dimension naming anything else does not, and a file
    /// with no entries here declares no table at all.
    pub dim_styles: Vec<DimStyleSpec>,
    /// STYLE (text style) table entries the file declares, by name. A text
    /// naming one of these resolves; one naming anything else does not. A
    /// text that names no style stands for the one called `STANDARD`, so
    /// it resolves when this list has that name and is absent otherwise --
    /// and with no entries here the file declares no table at all.
    pub text_styles: Vec<String>,
    /// The drawing's own entities, in file order.
    pub entities: Vec<EntitySpec>,
    /// The entities of paper space (`*Paper_Space`, the first sheet), in
    /// file order. The writer puts them after `entities`, marked as paper
    /// space (DXF 67).
    pub paper_space: Vec<EntitySpec>,
    /// The LAYOUT objects the file declares, each naming the block it shows.
    /// With none the file has no OBJECTS section at all -- what a reader of
    /// a drawing without layouts sees.
    pub layouts: Vec<LayoutSpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerSpec {
    pub name: String,
    /// AutoCAD Color Index, 1..=255. A layer that is off is written with
    /// this negated -- how a DXF says "off".
    pub color_index: i16,
    /// Everything else the file says about the layer.
    pub state: LayerState,
}

/// A layer's state beyond its name and colour. The default is the state
/// the writer has always written: on, thawed, unlocked, and neither a plot
/// flag (DXF 290) nor a lineweight (370) stated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayerState {
    /// Written as a negative colour (DXF 62).
    pub off: bool,
    /// DXF 70, bit 1.
    pub frozen: bool,
    /// DXF 70, bit 4.
    pub locked: bool,
    /// DXF 290 when set.
    pub plot: Option<bool>,
    /// DXF 370 when set: hundredths of a millimetre, or -3 for the default.
    pub lineweight: Option<i16>,
}

/// One LAYOUT object: a tab, the block it shows and its plot settings.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutSpec {
    /// DXF 1 of the layout part: the tab's name.
    pub name: String,
    /// DXF 71.
    pub tab_order: i32,
    /// The block record the layout shows (DXF 330): `*Model_Space`,
    /// `*Paper_Space`, or another `*Paper_Space<n>` the writer then
    /// declares, empty.
    pub block: String,
    /// DXF 10 and 11.
    pub limits_min: Xy,
    pub limits_max: Xy,
    /// DXF 4.
    pub paper_name: String,
    /// DXF 44 and 45, millimetres.
    pub paper_size: (f64, f64),
    /// DXF 40, 41, 42 and 43 in that order (left, bottom, right, top),
    /// millimetres.
    pub margins: [f64; 4],
    /// DXF 46 and 47, millimetres.
    pub plot_origin: Xy,
    /// DXF 72.
    pub paper_units: PlotPaperUnits,
    /// DXF 73.
    pub rotation: PlotRotation,
    /// DXF 70 of the layout part: bit 1 (PSLTSCALE) and bit 2 (LIMCHECK).
    pub paper_space_linetype_scaling: bool,
    pub limits_check: bool,
    /// DXF 14 and 15, z 0.
    pub extents: (Xy, Xy),
    /// DXF 331: the paper-space viewport last active in the layout, by
    /// index into [`Spec::paper_space`]; `None` writes no group.
    pub active_viewport: Option<usize>,
    /// DXF 142 and 143: the custom print scale's two sides.
    pub scale: (f64, f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockSpec {
    pub name: String,
    /// The definition's entities. An `Insert` inside a block is allowed
    /// (nesting); an `Attrib` is not, since attributes belong to an INSERT.
    pub entities: Vec<EntitySpec>,
}

/// What a multileader points out ([`EntitySpec::MultiLeader`]), in the plane
/// of the drawing: horizontal, unrotated.
#[derive(Debug, Clone, PartialEq)]
pub enum MultiLeaderContentSpec {
    /// A text at `location` (its top-left corner), `height` high, in the
    /// text style `style` -- one of [`Spec::text_styles`].
    Text {
        text: String,
        location: Xy,
        height: f64,
        style: String,
    },
    /// A reference to block `block` -- one of [`Spec::blocks`] -- at
    /// `location`, scaled by `scale` on every axis.
    Block {
        block: String,
        location: Xy,
        scale: f64,
    },
}

/// A 2D coordinate; every entity here is planar, so `z` is always `0`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Xy {
    pub x: f64,
    pub y: f64,
}

impl Xy {
    pub const fn new(x: f64, y: f64) -> Self {
        Xy { x, y }
    }

    /// This point moved by `dx`, `dy`.
    pub const fn moved(self, dx: f64, dy: f64) -> Self {
        Xy {
            x: self.x + dx,
            y: self.y + dy,
        }
    }
}

/// One polyline vertex: where it is and the bulge of the segment that
/// leaves it (DXF 42 -- `0` is straight, otherwise the tangent of a quarter
/// of the arc's included angle, positive counter-clockwise), and that
/// segment's widths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub at: Xy,
    pub bulge: f64,
    /// DXF 40 and 41: the segment's width where it leaves this vertex and
    /// where it reaches the next. The writer states them on every vertex of
    /// a polyline any vertex of which has a width, and on none otherwise.
    pub start_width: f64,
    pub end_width: f64,
}

impl Vertex {
    pub const fn bulged(at: Xy, bulge: f64) -> Self {
        Vertex {
            at,
            bulge,
            start_width: 0.0,
            end_width: 0.0,
        }
    }

    /// This vertex with its segment `start_width` wide where it leaves the
    /// vertex and `end_width` wide where it reaches the next.
    pub const fn wide(self, start_width: f64, end_width: f64) -> Self {
        Vertex {
            at: self.at,
            bulge: self.bulge,
            start_width,
            end_width,
        }
    }
}

impl From<Xy> for Vertex {
    /// A vertex whose outgoing segment is straight and has no width of its
    /// own.
    fn from(at: Xy) -> Self {
        Vertex::bulged(at, 0.0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EntitySpec {
    Line {
        layer: String,
        start: Xy,
        end: Xy,
    },
    Circle {
        layer: String,
        /// In the circle's own coordinate system: for a mirrored circle
        /// (extrusion (0, 0, -1)) the world x is the negative of this x.
        center: Xy,
        radius: f64,
        /// Extrusion (0, 0, -1) rather than the default (0, 0, 1) -- what a
        /// mirror copy writes.
        mirrored: bool,
    },
    Arc {
        layer: String,
        /// As for `Circle`.
        center: Xy,
        radius: f64,
        /// Degrees, as DXF stores them, counter-clockwise about the extrusion.
        start_deg: f64,
        end_deg: f64,
        mirrored: bool,
    },
    LwPolyline {
        layer: String,
        /// In the polyline's own coordinate system, as for `Circle`.
        vertices: Vec<Vertex>,
        closed: bool,
        /// DXF 43 when not 0.
        const_width: f64,
        /// DXF 38 when not 0: the z of every vertex in the polyline's own
        /// coordinate system.
        elevation: f64,
        mirrored: bool,
    },
    Text {
        layer: String,
        /// In the text's own coordinate system, as for `Circle`.
        insert: Xy,
        height: f64,
        text: String,
        /// Degrees.
        rotation_deg: f64,
        /// An alignment other than left and baseline; `None` writes neither
        /// 72, 73 nor 11.
        align: Option<TextAlign>,
        /// DXF 41, written only when it is not 1.
        width_factor: f64,
        /// In the text's own coordinate system: for a mirrored text
        /// (extrusion (0, 0, -1)) `insert` and the alignment point are
        /// written with their x negated, and the text reads mirrored.
        /// DXF 51 when not 0; degrees.
        oblique_deg: f64,
        /// DXF 7 when set.
        style: Option<String>,
        mirrored: bool,
    },
    /// A SOLID: four corners in DXF order, in its own coordinate system as
    /// for `Circle`.
    Solid {
        layer: String,
        corners: [Xy; 4],
        mirrored: bool,
    },
    /// A polygon mesh (POLYLINE with group 70 bit 16): `m` rows of `n`
    /// vertices, row by row.
    PolygonMesh {
        layer: String,
        m: u16,
        n: u16,
        closed_m: bool,
        closed_n: bool,
        vertices: Vec<[f64; 3]>,
    },
    /// An ordinate dimension: the distance of `feature` from `datum` along
    /// one axis, with its leader running to `leader_end`.
    OrdinateDimension {
        layer: String,
        /// DXF 10.
        datum: Xy,
        /// DXF 13.
        feature: Xy,
        /// DXF 14, and where the text sits (DXF 11).
        leader_end: Xy,
        /// DXF 70, bit 64.
        axis: OrdinateAxis,
        text: String,
        measurement: Option<f64>,
        style: Option<String>,
    },
    /// An entity that states the common properties a plain one leaves to
    /// its layer: the writer writes them in its `AcDbEntity` part. Not a
    /// dimension.
    Styled {
        style: LineStyleSpec,
        entity: Box<EntitySpec>,
    },
    /// A solid-fill HATCH in the world's plane (default extrusion, elevation
    /// 0): its boundary paths and which areas among them are filled.
    Hatch {
        layer: String,
        paths: Vec<HatchPathSpec>,
        /// DXF 75.
        style: HatchStyle,
    },
    /// A paper-space viewport; only meaningful in [`Spec::paper_space`].
    Viewport {
        layer: String,
        /// DXF 10, 40 and 41.
        center: Xy,
        width: f64,
        height: f64,
        /// DXF 68 (0 when off) and the off bit of DXF 90.
        on: bool,
        /// DXF 69.
        id: i32,
        /// DXF 12.
        view_center: Xy,
        /// DXF 45.
        view_height: f64,
        /// DXF 17; its z is 0.
        view_target: Xy,
        /// DXF 51, degrees.
        twist_deg: f64,
        /// DXF 341, by layer name; every name must be a declared layer.
        frozen_layers: Vec<String>,
    },
    /// An attribute definition -- only meaningful inside a block definition.
    Attdef {
        layer: String,
        insert: Xy,
        height: f64,
        tag: String,
        prompt: String,
        default: String,
    },
    Insert {
        layer: String,
        block: String,
        /// In the reference's own coordinate system, as for `Circle`; the
        /// rotation turns about that system's Z axis.
        insert: Xy,
        scale: f64,
        /// Degrees.
        rotation_deg: f64,
        /// Attribute values; each becomes an ATTRIB after the INSERT, in
        /// the world's own axes whatever the INSERT's.
        attribs: Vec<AttribSpec>,
        /// In the INSERT's own coordinate system: for a mirrored INSERT
        /// (extrusion (0, 0, -1)) `insert` is written with its x negated, and
        /// the block's world x runs the other way.
        mirrored: bool,
    },
    /// A linear (rotated) dimension between two definition points, with its
    /// drawn geometry in an anonymous `*D<n>` block the writer produces.
    LinearDimension {
        layer: String,
        /// The measured points (DXF 13/23 and 14/24).
        from: Xy,
        to: Xy,
        /// Where the dimension line sits (DXF 10/20).
        line_point: Xy,
        /// Written verbatim as DXF 1. `""` and `"<>"` both mean "show the
        /// measurement", `" "` means "show nothing", anything else is the
        /// text itself -- and the model folds the first two together.
        text: String,
        /// DXF 42 when set. Left out, the file states no measurement, which
        /// drawings older than R2000 routinely do.
        measurement: Option<f64>,
        /// DXF 3 when set: the style this dimension names.
        style: Option<String>,
    },
    /// A dimension of the length along an arc. The format gives it its own
    /// entity while still writing a group 70 that says "three-point
    /// angular", so it is the case that tells a reader which one it trusts.
    ArcDimension {
        layer: String,
        /// DXF 13/23 and 14/24.
        from: Xy,
        to: Xy,
        /// DXF 15/25, the arc's centre.
        center: Xy,
        /// DXF 10/20.
        line_point: Xy,
        text: String,
        measurement: Option<f64>,
        style: Option<String>,
    },
    /// A diameter dimension across a circle.
    DiameterDimension {
        layer: String,
        /// The two points on the circle the dimension spans (DXF 10/20 and
        /// 15/25).
        first: Xy,
        second: Xy,
        text: String,
        measurement: Option<f64>,
        style: Option<String>,
    },
    /// A MULTILEADER with one leader root and no dogleg. A drawing with one
    /// is written as R2010 (`AC1024`), the first version whose lines can
    /// state their own type.
    MultiLeader {
        layer: String,
        /// The root's lines (`LEADER_LINE{` blocks), each its points and
        /// the line's own type, if it states one (93 bit 0x1 and 170 inside
        /// the block; `None` writes neither).
        lines: Vec<(Vec<Xy>, Option<i16>)>,
        /// The root's last leader line point (DXF 10 of the root): every
        /// line runs on to it.
        last_point: Xy,
        /// DXF 170 on the entity.
        line_type: i16,
        /// DXF 140 of the context data: the size every arrowhead is drawn
        /// at (no line states its own).
        arrow_size: f64,
        /// What it points out, written in its context data; `None` writes
        /// neither a text nor a block.
        content: Option<MultiLeaderContentSpec>,
        /// Whether DXF 90 sets bit 0x1 -- the entity's type, not its
        /// style's, is the one drawn.
        type_overridden: bool,
        /// The MLEADERSTYLE the entity names (DXF 340), by name, and that
        /// style's line type (173). The file declares one style per name
        /// the drawing's multileaders use. `None` names a handle no object
        /// in the drawing has.
        style: Option<(String, i16)>,
    },
}

impl EntitySpec {
    /// This entity moved by `dx`, `dy`, in the drawing's own units.
    ///
    /// Every point the variant carries moves; nothing else changes, so a
    /// dimension still measures the same length and a block reference still
    /// names the same block. Used to build a drawing out of many copies of
    /// a smaller one without restating it.
    pub fn moved(&self, dx: f64, dy: f64) -> EntitySpec {
        let m = |p: &Xy| p.moved(dx, dy);
        match self {
            EntitySpec::Line { layer, start, end } => EntitySpec::Line {
                layer: layer.clone(),
                start: m(start),
                end: m(end),
            },
            EntitySpec::Circle {
                layer,
                center,
                radius,
                mirrored,
            } => EntitySpec::Circle {
                layer: layer.clone(),
                center: moved_ocs(center, *mirrored, dx, dy),
                radius: *radius,
                mirrored: *mirrored,
            },
            EntitySpec::Arc {
                layer,
                center,
                radius,
                start_deg,
                end_deg,
                mirrored,
            } => EntitySpec::Arc {
                layer: layer.clone(),
                center: moved_ocs(center, *mirrored, dx, dy),
                radius: *radius,
                start_deg: *start_deg,
                end_deg: *end_deg,
                mirrored: *mirrored,
            },
            EntitySpec::LwPolyline {
                layer,
                vertices,
                closed,
                const_width,
                elevation,
                mirrored,
            } => EntitySpec::LwPolyline {
                layer: layer.clone(),
                vertices: vertices
                    .iter()
                    .map(|v| Vertex {
                        at: moved_ocs(&v.at, *mirrored, dx, dy),
                        ..*v
                    })
                    .collect(),
                closed: *closed,
                const_width: *const_width,
                elevation: *elevation,
                mirrored: *mirrored,
            },
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
            } => EntitySpec::Text {
                layer: layer.clone(),
                insert: moved_ocs(insert, *mirrored, dx, dy),
                height: *height,
                text: text.clone(),
                rotation_deg: *rotation_deg,
                align: align.map(|a| TextAlign {
                    at: moved_ocs(&a.at, *mirrored, dx, dy),
                    ..a
                }),
                width_factor: *width_factor,
                oblique_deg: *oblique_deg,
                style: style.clone(),
                mirrored: *mirrored,
            },
            EntitySpec::Attdef {
                layer,
                insert,
                height,
                tag,
                prompt,
                default,
            } => EntitySpec::Attdef {
                layer: layer.clone(),
                insert: m(insert),
                height: *height,
                tag: tag.clone(),
                prompt: prompt.clone(),
                default: default.clone(),
            },
            EntitySpec::Insert {
                layer,
                block,
                insert,
                scale,
                rotation_deg,
                attribs,
                mirrored,
            } => EntitySpec::Insert {
                layer: layer.clone(),
                block: block.clone(),
                insert: moved_ocs(insert, *mirrored, dx, dy),
                scale: *scale,
                rotation_deg: *rotation_deg,
                // An attribute's points are written in world coordinates,
                // whatever its reference's extrusion.
                attribs: attribs
                    .iter()
                    .map(|a| AttribSpec {
                        insert: m(&a.insert),
                        align: a.align.map(|t| TextAlign { at: m(&t.at), ..t }),
                        ..a.clone()
                    })
                    .collect(),
                mirrored: *mirrored,
            },
            EntitySpec::Solid {
                layer,
                corners,
                mirrored,
            } => EntitySpec::Solid {
                layer: layer.clone(),
                corners: corners.map(|c| moved_ocs(&c, *mirrored, dx, dy)),
                mirrored: *mirrored,
            },
            EntitySpec::PolygonMesh {
                layer,
                m: rows,
                n,
                closed_m,
                closed_n,
                vertices,
            } => EntitySpec::PolygonMesh {
                layer: layer.clone(),
                m: *rows,
                n: *n,
                closed_m: *closed_m,
                closed_n: *closed_n,
                vertices: vertices
                    .iter()
                    .map(|[x, y, z]| [x + dx, y + dy, *z])
                    .collect(),
            },
            EntitySpec::OrdinateDimension {
                layer,
                datum,
                feature,
                leader_end,
                axis,
                text,
                measurement,
                style,
            } => EntitySpec::OrdinateDimension {
                layer: layer.clone(),
                datum: m(datum),
                feature: m(feature),
                leader_end: m(leader_end),
                axis: *axis,
                text: text.clone(),
                measurement: *measurement,
                style: style.clone(),
            },
            EntitySpec::Styled { style, entity } => EntitySpec::Styled {
                style: style.clone(),
                entity: Box::new(entity.moved(dx, dy)),
            },
            EntitySpec::Hatch {
                layer,
                paths,
                style,
            } => EntitySpec::Hatch {
                layer: layer.clone(),
                paths: paths
                    .iter()
                    .map(|path| HatchPathSpec {
                        shape: match &path.shape {
                            HatchShapeSpec::Polyline(vertices) => HatchShapeSpec::Polyline(
                                vertices
                                    .iter()
                                    .map(|v| Vertex { at: m(&v.at), ..*v })
                                    .collect(),
                            ),
                            HatchShapeSpec::Edges(edges) => HatchShapeSpec::Edges(
                                edges.iter().map(|e| e.moved(dx, dy)).collect(),
                            ),
                        },
                        ..path.clone()
                    })
                    .collect(),
                style: *style,
            },
            // A viewport's own place is its centre on the sheet; what it
            // shows (its view centre and target) is model space and stays.
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
            } => EntitySpec::Viewport {
                layer: layer.clone(),
                center: m(center),
                width: *width,
                height: *height,
                on: *on,
                id: *id,
                view_center: *view_center,
                view_height: *view_height,
                view_target: *view_target,
                twist_deg: *twist_deg,
                frozen_layers: frozen_layers.clone(),
            },
            EntitySpec::LinearDimension {
                layer,
                from,
                to,
                line_point,
                text,
                measurement,
                style,
            } => EntitySpec::LinearDimension {
                layer: layer.clone(),
                from: m(from),
                to: m(to),
                line_point: m(line_point),
                text: text.clone(),
                measurement: *measurement,
                style: style.clone(),
            },
            EntitySpec::ArcDimension {
                layer,
                from,
                to,
                center,
                line_point,
                text,
                measurement,
                style,
            } => EntitySpec::ArcDimension {
                layer: layer.clone(),
                from: m(from),
                to: m(to),
                center: m(center),
                line_point: m(line_point),
                text: text.clone(),
                measurement: *measurement,
                style: style.clone(),
            },
            EntitySpec::DiameterDimension {
                layer,
                first,
                second,
                text,
                measurement,
                style,
            } => EntitySpec::DiameterDimension {
                layer: layer.clone(),
                first: m(first),
                second: m(second),
                text: text.clone(),
                measurement: *measurement,
                style: style.clone(),
            },
            EntitySpec::MultiLeader {
                layer,
                lines,
                last_point,
                line_type,
                arrow_size,
                content,
                type_overridden,
                style,
            } => EntitySpec::MultiLeader {
                layer: layer.clone(),
                lines: lines
                    .iter()
                    .map(|(points, own)| (points.iter().map(m).collect(), *own))
                    .collect(),
                last_point: m(last_point),
                line_type: *line_type,
                arrow_size: *arrow_size,
                content: content.as_ref().map(|c| match c {
                    MultiLeaderContentSpec::Text {
                        text,
                        location,
                        height,
                        style,
                    } => MultiLeaderContentSpec::Text {
                        text: text.clone(),
                        location: m(location),
                        height: *height,
                        style: style.clone(),
                    },
                    MultiLeaderContentSpec::Block {
                        block,
                        location,
                        scale,
                    } => MultiLeaderContentSpec::Block {
                        block: block.clone(),
                        location: m(location),
                        scale: *scale,
                    },
                }),
                type_overridden: *type_overridden,
                style: style.clone(),
            },
        }
    }
}

impl HatchEdgeSpec {
    /// The edge displaced by `(dx, dy)`. An ellipse's major axis endpoint
    /// is relative to its centre (DXF 11 of a hatch edge) and a spline's
    /// knots and weights are not points, so only centres, ends and control
    /// points move.
    pub fn moved(&self, dx: f64, dy: f64) -> HatchEdgeSpec {
        let m = |p: &Xy| p.moved(dx, dy);
        match self {
            HatchEdgeSpec::Line { start, end } => HatchEdgeSpec::Line {
                start: m(start),
                end: m(end),
            },
            HatchEdgeSpec::Arc {
                center,
                radius,
                start_deg,
                end_deg,
                ccw,
            } => HatchEdgeSpec::Arc {
                center: m(center),
                radius: *radius,
                start_deg: *start_deg,
                end_deg: *end_deg,
                ccw: *ccw,
            },
            HatchEdgeSpec::Ellipse {
                center,
                major_end,
                ratio,
                start_deg,
                end_deg,
                ccw,
            } => HatchEdgeSpec::Ellipse {
                center: m(center),
                major_end: *major_end,
                ratio: *ratio,
                start_deg: *start_deg,
                end_deg: *end_deg,
                ccw: *ccw,
            },
            HatchEdgeSpec::Spline {
                degree,
                rational,
                periodic,
                knots,
                control_points,
                weights,
            } => HatchEdgeSpec::Spline {
                degree: *degree,
                rational: *rational,
                periodic: *periodic,
                knots: knots.clone(),
                control_points: control_points.iter().map(m).collect(),
                weights: weights.clone(),
            },
        }
    }
}

/// One DIMSTYLE table entry the file declares. Only the variables a case
/// needs are here; the rest stay unwritten, which is itself what a reader
/// has to report as "this style does not state it".
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DimStyleSpec {
    /// DXF 2.
    pub name: String,
    /// DXF 3.
    pub post: Option<String>,
    /// DXF 271.
    pub decimal_places: Option<i32>,
    /// DXF 140.
    pub text_height: Option<f64>,
    /// DXF 41.
    pub arrow_size: Option<f64>,
    /// DXF 277.
    pub linear_unit_format: Option<LinearUnitFormat>,
    /// DXF 78.
    pub zero_suppression: Option<i32>,
    /// DXF 45.
    pub rounding: Option<f64>,
    /// DXF 275.
    pub angular_unit_format: Option<AngularUnitFormat>,
    /// DXF 179.
    pub angular_decimal_places: Option<i32>,
    /// DXF 276.
    pub fraction_format: Option<FractionFormat>,
    /// DXF 90 (`DIMARCSYM`); unwritten when `None`.
    pub arc_symbol: Option<ArcSymbol>,
}

/// The common properties an entity can state itself; `None` writes no
/// group, which the format reads as BYLAYER (a scale of 1).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct LineStyleSpec {
    /// DXF 6: a linetype name, or `BYLAYER`/`BYBLOCK`.
    pub linetype: Option<String>,
    /// DXF 48.
    pub linetype_scale: Option<f64>,
    /// DXF 370: hundredths of a millimetre, or -1/-2/-3.
    pub lineweight: Option<i16>,
}

/// One HATCH boundary path, and the entities it was picked from.
#[derive(Debug, Clone, PartialEq)]
pub struct HatchPathSpec {
    pub shape: HatchShapeSpec,
    /// DXF 92, bit 1: the path is on the outside.
    pub external: bool,
    /// The top-level entities (by index into [`Spec::entities`], each before
    /// the hatch) the path was picked from: DXF 97 and one 330 each, and the
    /// hatch is associative (71 = 1) when any path names one. The model does
    /// not carry them -- they are references, not geometry -- so a reader
    /// only has to step over them.
    pub sources: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HatchShapeSpec {
    /// A closed polyline path (92 bit 2): its vertices and their bulges (42,
    /// written for every vertex when any bulge is not 0). Widths are ignored.
    Polyline(Vec<Vertex>),
    Edges(Vec<HatchEdgeSpec>),
}

/// One edge of an edge path, as DXF states it.
#[derive(Debug, Clone, PartialEq)]
pub enum HatchEdgeSpec {
    /// 72 = 1: DXF 10 and 11.
    Line { start: Xy, end: Xy },
    /// 72 = 2: DXF 10, 40, 50 and 51 (degrees) and 73.
    Arc {
        center: Xy,
        radius: f64,
        start_deg: f64,
        end_deg: f64,
        ccw: bool,
    },
    /// 72 = 3: DXF 10, 11 (major axis endpoint relative to the center), 40,
    /// 50 and 51 (degrees) and 73.
    Ellipse {
        center: Xy,
        major_end: Xy,
        ratio: f64,
        start_deg: f64,
        end_deg: f64,
        ccw: bool,
    },
    /// 72 = 4: DXF 94, 73, 74, 95/96, the knots (40), the control points
    /// (10) and, for a rational one, a weight (42) per control point. Fit
    /// data is R2010 and later; this R2000 writer has none to write.
    Spline {
        degree: u32,
        rational: bool,
        periodic: bool,
        knots: Vec<f64>,
        control_points: Vec<Xy>,
        weights: Vec<f64>,
    },
}

/// A TEXT's alignment: its two DXF codes (72 horizontal 0 to 5, 73
/// vertical 0 to 3) and the alignment point (11).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextAlign {
    pub horizontal: u8,
    pub vertical: u8,
    pub at: Xy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttribSpec {
    pub tag: String,
    pub value: String,
    pub insert: Xy,
    pub height: f64,
    /// An alignment other than left and baseline (72, and 74 -- an
    /// attribute's vertical alignment code); `None` writes neither.
    pub align: Option<TextAlign>,
    /// DXF 41, written only when it is not 1.
    pub width_factor: f64,
    /// DXF 70, bit 1: the value is not shown.
    pub invisible: bool,
}

impl EntitySpec {
    pub fn layer(&self) -> &str {
        match self {
            EntitySpec::Line { layer, .. }
            | EntitySpec::Circle { layer, .. }
            | EntitySpec::Arc { layer, .. }
            | EntitySpec::LwPolyline { layer, .. }
            | EntitySpec::Text { layer, .. }
            | EntitySpec::Solid { layer, .. }
            | EntitySpec::Attdef { layer, .. }
            | EntitySpec::Insert { layer, .. }
            | EntitySpec::PolygonMesh { layer, .. }
            | EntitySpec::LinearDimension { layer, .. }
            | EntitySpec::ArcDimension { layer, .. }
            | EntitySpec::DiameterDimension { layer, .. }
            | EntitySpec::MultiLeader { layer, .. }
            | EntitySpec::OrdinateDimension { layer, .. }
            | EntitySpec::Hatch { layer, .. }
            | EntitySpec::Viewport { layer, .. } => layer,
            EntitySpec::Styled { entity, .. } => entity.layer(),
        }
    }

    /// Whether this is a dimension, which the writer gives an anonymous
    /// `*D<n>` block of its drawn geometry.
    pub fn is_dimension(&self) -> bool {
        matches!(
            self,
            EntitySpec::LinearDimension { .. }
                | EntitySpec::ArcDimension { .. }
                | EntitySpec::DiameterDimension { .. }
                | EntitySpec::OrdinateDimension { .. }
        )
    }
}

/// An own-coordinate-system point moved by (`dx`, `dy`) in the world: a
/// mirrored entity's own x axis is the world's negative x.
fn moved_ocs(center: &Xy, mirrored: bool, dx: f64, dy: f64) -> Xy {
    if mirrored {
        center.moved(-dx, dy)
    } else {
        center.moved(dx, dy)
    }
}
