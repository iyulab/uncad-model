//! A golden case is a *spec*: the entities and values a synthetic drawing
//! contains, written as Rust data. The writer turns a spec into a DXF, and
//! the spec itself is the oracle for whatever a parser reads back out.
//!
//! Only what the model carries is representable here -- a spec that could
//! say more than the model would be an oracle nothing can be checked
//! against.

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
    /// Layers besides `0`, which always exists. Names must be unique.
    pub layers: Vec<LayerSpec>,
    /// Named block definitions an INSERT (or a DIMENSION) can refer to.
    pub blocks: Vec<BlockSpec>,
    /// DIMSTYLE table entries the file declares. A dimension naming one of
    /// these resolves; a dimension naming anything else does not, and a file
    /// with no entries here declares no table at all.
    pub dim_styles: Vec<DimStyleSpec>,
    /// The drawing's own entities, in file order.
    pub entities: Vec<EntitySpec>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LayerSpec {
    pub name: String,
    /// AutoCAD Color Index, 1..=255.
    pub color_index: i16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockSpec {
    pub name: String,
    /// The definition's entities. An `Insert` inside a block is allowed
    /// (nesting); an `Attrib` is not, since attributes belong to an INSERT.
    pub entities: Vec<EntitySpec>,
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
/// of the arc's included angle, positive counter-clockwise).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    pub at: Xy,
    pub bulge: f64,
}

impl Vertex {
    pub const fn bulged(at: Xy, bulge: f64) -> Self {
        Vertex { at, bulge }
    }
}

impl From<Xy> for Vertex {
    /// A vertex whose outgoing segment is straight.
    fn from(at: Xy) -> Self {
        Vertex { at, bulge: 0.0 }
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
        mirrored: bool,
    },
    Text {
        layer: String,
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
        insert: Xy,
        scale: f64,
        /// Degrees.
        rotation_deg: f64,
        /// Attribute values; each becomes an ATTRIB after the INSERT.
        attribs: Vec<AttribSpec>,
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
                mirrored,
            } => EntitySpec::LwPolyline {
                layer: layer.clone(),
                vertices: vertices
                    .iter()
                    .map(|v| Vertex::bulged(moved_ocs(&v.at, *mirrored, dx, dy), v.bulge))
                    .collect(),
                closed: *closed,
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
            } => EntitySpec::Text {
                layer: layer.clone(),
                insert: m(insert),
                height: *height,
                text: text.clone(),
                rotation_deg: *rotation_deg,
                align: align.map(|a| TextAlign { at: m(&a.at), ..a }),
                width_factor: *width_factor,
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
            } => EntitySpec::Insert {
                layer: layer.clone(),
                block: block.clone(),
                insert: m(insert),
                scale: *scale,
                rotation_deg: *rotation_deg,
                attribs: attribs.clone(),
            },
            other => other.clone(),
        }
    }
}

/// One DIMSTYLE table entry the file declares. Only the variables a case
/// needs are here; the rest stay unwritten, which is itself what a reader
/// has to report as "this style does not state it".
#[derive(Debug, Clone, PartialEq)]
pub struct DimStyleSpec {
    /// DXF 2.
    pub name: String,
    /// DXF 3.
    pub post: Option<String>,
    /// DXF 271.
    pub decimal_places: Option<i32>,
    /// DXF 140.
    pub text_height: Option<f64>,
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
}

impl EntitySpec {
    pub fn layer(&self) -> &str {
        match self {
            EntitySpec::Line { layer, .. }
            | EntitySpec::Circle { layer, .. }
            | EntitySpec::Arc { layer, .. }
            | EntitySpec::LwPolyline { layer, .. }
            | EntitySpec::Text { layer, .. }
            | EntitySpec::Attdef { layer, .. }
            | EntitySpec::Insert { layer, .. }
            | EntitySpec::LinearDimension { layer, .. }
            | EntitySpec::ArcDimension { layer, .. }
            | EntitySpec::DiameterDimension { layer, .. } => layer,
        }
    }
}

/// An own-coordinate-system center moved by (`dx`, `dy`) in the world: a
/// mirrored entity's own x axis is the world's negative x.
fn moved_ocs(center: &Xy, mirrored: bool, dx: f64, dy: f64) -> Xy {
    if mirrored {
        center.moved(-dx, dy)
    } else {
        center.moved(dx, dy)
    }
}
