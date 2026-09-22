//! The entity model: what a 2D CAD drawing *is* once it has been taken out
//! of its file format. A parser fills it, and everything downstream -- a
//! summarizer, an editor, a differ, a renderer -- reads it. Nothing here knows
//! which parser produced the values or which consumer will read them.
//!
//! Every type derives `serde::Serialize`/`Deserialize`; [`Entity`] is
//! internally tagged with `"type"` using the same DXF names
//! [`Entity::type_name`] reports, so JSON consumers can dispatch on `type`
//! without knowing the Rust enum (see [`crate::json`]).
//!
//! The shape follows the DXF reference: field names and units are the DXF
//! ones (angles in radians where DXF stores degrees is the one deliberate
//! departure, noted on each field). Where a consumer has to approximate --
//! a curve drawn as chords, a solid drawn as its wireframe -- that is the
//! consumer's decision and is not encoded here.

use serde::{Deserialize, Serialize};

/// A point or vector in the drawing's 3D coordinate space (DXF group codes
/// 10/20/30 and their siblings). Plain data: consumers read and construct
/// it directly.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point3D {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// A point or vector in a 2D coordinate space (DXF group codes 10/20 for
/// entities that are planar by definition, such as TEXT and LWPOLYLINE).
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Point2D {
    pub x: f64,
    pub y: f64,
}

/// A value this model reached by following what the source file points with
/// -- a layer name, a block name, a style name.
///
/// A file points either with a handle (DWG, and DXF once it has handles) or
/// with the name itself (a DXF INSERT names its block). Both are references
/// and both take the same three states.
///
/// Three states rather than an `Option`, because two different things used
/// to collapse into one empty string: a field the file points at nothing for
/// at all (normal for some fields), and a reference that nothing in the
/// drawing answers to (always a defect in the file or in the read). The
/// unresolved case keeps what the file wrote -- the handle, or the name: it
/// is the only thing that tells one missing table row (many entities point
/// at the same dead reference) from references broken wholesale (every
/// entity points somewhere different). A reader that drops the name the file
/// wrote and reports `Absent` instead has erased what the drawing said.
///
/// Serialized adjacently tagged, like [`HatchBoundaryPath`]:
/// `{"type":"RESOLVED","data":"0"}`, `{"type":"ABSENT"}`,
/// `{"type":"UNRESOLVED","data":"2A"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "UPPERCASE")]
pub enum Ref<T> {
    /// The reference resolved. `T` is what it resolved to (a name, today).
    Resolved(T),
    /// The file points at nothing for this field.
    Absent,
    /// The file carries a reference, but nothing in the drawing answers to
    /// it. The value is what the file wrote: the handle as a hex string, the
    /// same form as [`EntityCommon::handle`]; for a pre-R13 drawing, which
    /// points at its tables by index rather than by handle, the index as
    /// `idx:<n>`; and where the file points by name, the name.
    Unresolved(String),
}

impl<T> Ref<T> {
    /// The resolved value, if there is one.
    pub fn resolved(&self) -> Option<&T> {
        match self {
            Ref::Resolved(value) => Some(value),
            Ref::Absent | Ref::Unresolved(_) => None,
        }
    }

    /// `true` for [`Ref::Resolved`].
    pub fn is_resolved(&self) -> bool {
        matches!(self, Ref::Resolved(_))
    }
}

impl Ref<String> {
    /// The resolved name as a `&str`, or `""` when there is none. For
    /// consumers that only need a lookup key and treat "no name" and "no such
    /// name" alike; anything that must tell them apart matches on the enum.
    pub fn name(&self) -> &str {
        self.resolved().map_or("", String::as_str)
    }
}

/// The name by which an entity is pointed at: one scheme, shared by every
/// consumer. Minted by whoever puts the entity into the model -- a parser, a
/// recognizer, an editor -- at the moment it is put there, never by a reader.
/// Unique within a model, opaque to consumers, and the same for the same
/// inputs. A file handle is not a reference ID (see
/// [`EntityCommon::source_handle`]).
///
/// Serialized as a plain integer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EntityId(u64);

impl EntityId {
    /// Mints an ID. Only a producer of entities calls this; a consumer
    /// carries IDs it received and never invents one.
    pub const fn new(value: u64) -> Self {
        EntityId(value)
    }

    /// The ID's value, for producers that derive IDs from one another or
    /// keep them in a table. It carries no meaning for a consumer.
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// Where an entity came from. A closed set: a fourth kind is a discussion,
/// not an addition. Consumers do not branch on it -- they act on
/// [`Confidence`] alone (the model's invariant 6).
///
/// Serialized as `"VECTOR"` / `"RASTER"` / `"DERIVED"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Origin {
    /// Read from a vector source (a DWG/DXF parser).
    Vector,
    /// Recognized from a raster source (an image of a drawing).
    Raster,
    /// Produced by an operation on the model (an editor).
    Derived,
}

/// How far an entity's values can be trusted. A total order --
/// `Unknown < Low < High` -- so that whichever layer merges values can take
/// the lower one, and never raises it: a value that entered low never
/// leaves high (invariant 2). Finer grades and numeric scores are not part
/// of the model.
///
/// Serialized as `"UNKNOWN"` / `"LOW"` / `"HIGH"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Confidence {
    /// Nothing is known about how far the values can be trusted.
    Unknown,
    /// The values may be wrong; a consumer presents them as such.
    Low,
    /// The values are what the source states.
    High,
}

/// Fields common to every entity, regardless of type.
///
/// None of the three markers has a default: a producer states them or the
/// entity cannot be built. That is what keeps "vector" and "high" from
/// leaking in unstated.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EntityCommon {
    /// The reference ID -- see [`EntityId`].
    pub id: EntityId,
    /// Where the entity came from -- see [`Origin`].
    pub origin: Origin,
    /// How far its values can be trusted -- see [`Confidence`].
    pub confidence: Confidence,
    /// The handle the source file gave this entity, as a hex string
    /// (e.g. `"2A"`): provenance, present only for entities that came from a
    /// file, kept because it is what a file-level tool (or the file's own
    /// cross-references) names the entity by. [`Ref::Absent`] for an entity
    /// that has no file behind it -- recognized or derived.
    pub source_handle: Ref<String>,
    /// Owning layer's name (DXF 8), resolved from the entity's layer
    /// reference. [`Ref::Unresolved`] when the reference points at nothing;
    /// never an empty string standing in for "could not read".
    pub layer: Ref<String>,
    /// AutoCAD Color Index (DXF 62): negative means "layer off" (the sign
    /// is a visibility flag, not a different color), 0 is BYBLOCK, 256 is
    /// BYLAYER, anything else is a direct index into
    /// [`crate::color::ACI_PALETTE`].
    pub color_index: i16,
    /// Explicit 24-bit true color (DXF 420), present only when the entity
    /// opted into a direct RGB color; it then takes precedence over
    /// `color_index`.
    pub true_color: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LineEntity {
    pub common: EntityCommon,
    pub start_point: Point3D,
    pub end_point: Point3D,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CircleEntity {
    pub common: EntityCommon,
    pub center: Point3D,
    pub radius: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEntity {
    pub common: EntityCommon,
    pub start_point: Point2D,
    pub text_height: f64,
    pub text: String,
    /// Radians (DXF 50). TEXT stores this as a plain angle, unlike MTEXT,
    /// whose rotation is a direction vector (see [`MTextEntity::rotation`]).
    pub rotation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LwPolylineEntity {
    pub common: EntityCommon,
    pub vertices: Vec<Point2D>,
    /// Whether the last vertex connects back to the first (DXF 70, bit 1).
    pub closed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArcEntity {
    pub common: EntityCommon,
    pub center: Point3D,
    pub radius: f64,
    /// Radians.
    pub start_angle: f64,
    /// Radians.
    pub end_angle: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EllipseEntity {
    pub common: EntityCommon,
    pub center: Point3D,
    /// Major-axis endpoint, relative to `center` (DXF 11).
    pub major_axis_endpoint: Point3D,
    /// Minor/major radius ratio (DXF 40).
    pub axis_ratio: f64,
    pub start_angle: f64,
    pub end_angle: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PointEntity {
    pub common: EntityCommon,
    pub position: Point3D,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SolidEntity {
    pub common: EntityCommon,
    /// Corners in DXF order (group codes 10, 11, 12, 13). The 1-2-4-3
    /// reordering a filled rendering needs is the renderer's concern, not
    /// part of the data.
    pub corner1: Point2D,
    pub corner2: Point2D,
    pub corner3: Point2D,
    pub corner4: Point2D,
}

/// Shared by RAY and XLINE: both are a base point and a direction, and
/// differ only in extent (a RAY extends one way from `point`, an XLINE
/// both ways).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RayEntity {
    pub common: EntityCommon,
    pub point: Point3D,
    pub vector: Point3D,
}

/// ATTRIB shares TEXT's exact field shape: it is a block-attribute value
/// attached to an INSERT, but geometrically just another piece of text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttribEntity {
    pub common: EntityCommon,
    pub start_point: Point2D,
    pub text_height: f64,
    /// The attribute's tag (DXF 2): the name its value answers to, as the
    /// block definition's ATTDEF declared it. What a reader of a title
    /// block looks a value up by; the value alone says nothing about which
    /// field it fills.
    pub tag: String,
    pub text: String,
    /// Radians (DXF 50).
    pub rotation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertEntity {
    pub common: EntityCommon,
    /// Referenced block's name (DXF 2). Consumers look it up in
    /// [`crate::tables::Tables::block_records`] for the block's own
    /// entities.
    pub block_name: Ref<String>,
    pub insertion_point: Point3D,
    /// Per-axis scale factors (DXF 41/42/43); (1,1,1) if never set.
    pub scale: Point3D,
    /// Radians.
    pub rotation: f64,
    /// Attribute values attached to this INSERT (the ATTRIB records between
    /// the INSERT and its SEQEND). A parser also lists them as top-level
    /// [`Entity::Attrib`] entries in `CadDatabase::entities`; a consumer that
    /// draws `entities` gets them from there.
    pub attribs: Vec<AttribEntity>,
}

/// TOLERANCE, a GD&T feature control frame. `text_value` keeps the raw
/// DXF feature-control-frame string (DXF 1) with its symbol escapes
/// unstripped; interpreting the symbols is a consumer's job.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToleranceEntity {
    pub common: EntityCommon,
    pub insertion_point: Point3D,
    /// DXF 40. `None` when the file does not state it -- a frame of zero
    /// height is not a height, so a reader that cannot tell an absent group
    /// from a zeroed one reports nothing rather than a size no file gave.
    pub text_height: Option<f64>,
    pub text_value: String,
    /// DXF 11: the direction the frame is written along. `None` when the
    /// reader cannot say -- a zero vector is not a direction, so a backend
    /// that cannot tell an absent group from a zeroed one reports nothing.
    pub direction: Option<Point3D>,
    /// DXF 3: the DIMSTYLE this frame names, which settles how its
    /// tolerances are formatted.
    pub style_name: Ref<String>,
}

/// How a LEADER's line runs between its vertices (DXF 72).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LeaderPath {
    /// 0: straight segments.
    Straight,
    /// 1: a spline through the vertices.
    Spline,
}

/// What a LEADER points at (DXF 73). The format's own default is
/// [`Self::Nothing`], so a file that omits the group is a leader with no
/// annotation rather than an unknown one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LeaderAnnotation {
    /// 0: an MTEXT.
    MText,
    /// 1: a TOLERANCE frame.
    Tolerance,
    /// 2: a block reference.
    Insert,
    /// 3: nothing.
    #[default]
    Nothing,
}

/// ACAD_TABLE, carried the way [`InsertEntity`] is minus `attribs`: a
/// table references a block that holds its rendered cell geometry, so
/// consumers draw it through the same block-reference path. Cell contents
/// (rows, columns, widths) are not part of the model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcadTableEntity {
    pub common: EntityCommon,
    pub block_name: Ref<String>,
    pub insertion_point: Point3D,
    pub scale: Point3D,
    /// Radians.
    pub rotation: f64,
}

/// Same field shape as ATTRIB -- the *template* stored in a block
/// definition, versus ATTRIB, the value attached to an INSERT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttdefEntity {
    pub common: EntityCommon,
    pub start_point: Point2D,
    pub text_height: f64,
    /// The tag (DXF 2) every ATTRIB made from this definition carries.
    pub tag: String,
    pub default_value: String,
    /// Radians (DXF 50). Kept for parity with ATTRIB; a template is not
    /// normally drawn.
    pub rotation: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewportEntity {
    pub common: EntityCommon,
    pub center: Point3D,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Face3DEntity {
    pub common: EntityCommon,
    /// Already-sequential order (unlike SOLID, no 1-2-4-3 reorder needed).
    pub corner1: Point3D,
    pub corner2: Point3D,
    pub corner3: Point3D,
    pub corner4: Point3D,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplineEntity {
    pub common: EntityCommon,
    /// Points that lie exactly on the curve -- preferred over
    /// `control_points` when present.
    pub fit_points: Vec<Point3D>,
    pub control_points: Vec<Point3D>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MTextEntity {
    pub common: EntityCommon,
    pub insertion_point: Point3D,
    pub text: String,
    pub text_height: f64,
    /// Radians. A parser that reads MTEXT's rotation as a direction vector
    /// (DXF 11) has to derive the angle; one that does not leaves `0.0` and
    /// says so in its own documentation.
    pub rotation: f64,
    pub line_spacing_factor: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolylineEntity {
    pub common: EntityCommon,
    pub vertices: Vec<Point3D>,
    pub closed: bool,
}

/// One edge of a non-polyline HATCH boundary path.
// JSON: internally tagged like `Entity`, upper-case like its tags --
// `{"type":"ARC","center":...}` -- but these are edge kinds, not DXF entity
// names (see `crate::json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "UPPERCASE")]
pub enum HatchEdge {
    Line {
        start: Point2D,
    },
    Arc {
        center: Point2D,
        radius: f64,
        start_angle: f64,
        end_angle: f64,
        is_ccw: bool,
    },
    Ellipse {
        center: Point2D,
        /// Major-axis endpoint, relative to `center`.
        end: Point2D,
        /// Minor/major axis length ratio.
        minor_major_ratio: f64,
        start_angle: f64,
        end_angle: f64,
        is_ccw: bool,
    },
    Spline {
        control_points: Vec<Point2D>,
    },
}

/// One HATCH boundary path -- either an explicit polyline (vertices only;
/// bulge/arc segments are dropped) or a list of curved/straight edges.
// JSON: adjacently tagged, because the payload is a sequence rather than a
// struct -- `{"type":"POLYLINE","data":[pt,..]}` / `{"type":"EDGES","data":
// [edge,..]}` -- keeping the `type` key every other tagged object uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "UPPERCASE")]
pub enum HatchBoundaryPath {
    Polyline(Vec<Point2D>),
    Edges(Vec<HatchEdge>),
}

/// One pattern-fill "definition line" (DXF 53/43/44/45/46/49): a family
/// of parallel lines that hatch a non-solid-fill boundary rather than just
/// outlining it.
///
/// The values are final, in the HATCH's own local coordinate space: the
/// HATCH's pattern angle and scale (DXF 52/41) are already applied, so a
/// consumer uses them as they are.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HatchPatternLine {
    /// Radians.
    pub angle: f64,
    /// A point that lies on one of the family's lines.
    pub base_point: Point2D,
    /// Displacement from one line to the next: the component perpendicular
    /// to `angle` is the spacing, a parallel component staggers the lines.
    pub offset: Point2D,
    /// Dash lengths along each line: positive = drawn, negative = gap,
    /// empty = continuous.
    pub dash_pattern: Vec<f64>,
}

/// A HATCH gradient fill, reduced to its colors and a shape. The gradient
/// name (DXF 470: `SPHERICAL`/`HEMISPHERICAL`/`CURVED`/`LINEAR`/`CYLINDER`)
/// collapses to `is_radial`: the two spherical names are radial, everything
/// else linear. The gradient shift (DXF 461) is not carried.
///
/// Colors are what the file states, as packed 24-bit RGB (`0xRRGGBB`), the
/// same form as [`EntityCommon::true_color`]: an ACI stop is resolved
/// through the palette, a true-color stop is taken as is. What the fill
/// looks like -- how a single-color gradient fades, whether white is
/// flipped for a white background -- is a renderer's decision and is not
/// baked into these values.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HatchGradient {
    pub is_radial: bool,
    /// Radians.
    pub angle: f64,
    /// The first stop's color (DXF 421 of the first color record).
    pub color1: u32,
    /// The second stop's color, `None` for a single-color gradient (DXF 452
    /// set), whose second stop a renderer derives from `color1` and `tint`.
    pub color2: Option<u32>,
    /// Single-color gradient tint (DXF 462, `0.0`-`1.0`): how far the fill
    /// fades from `color1` toward white. `0.0` when the gradient has two
    /// colors.
    pub tint: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HatchEntity {
    pub common: EntityCommon,
    pub boundary_paths: Vec<HatchBoundaryPath>,
    pub solid_fill: bool,
    /// `Some` when the HATCH is a gradient fill; a gradient takes
    /// precedence over `pattern_lines`.
    pub gradient: Option<HatchGradient>,
    /// Empty for solid fills, for gradient fills, or when the file's pattern
    /// data could not be read. When non-empty, these are the lines to tile
    /// inside `boundary_paths`.
    pub pattern_lines: Vec<HatchPatternLine>,
}

/// What a dimension measures, as the file states it (DXF 70, low three
/// bits). A file that does not state it leaves [`DimensionEntity::kind`]
/// `None` -- the format defines no default, so there is nothing to fall back
/// to and nothing to guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DimensionKind {
    /// 0: a linear dimension, measured along `points.rotation`.
    Rotated,
    /// 1: aligned with the two extension line origins.
    Aligned,
    /// 2: the angle between two lines.
    Angular2Line,
    /// 3: a diameter.
    Diameter,
    /// 4: a radius.
    Radius,
    /// 5: the angle through three points.
    Angular3Point,
    /// 6: an ordinate (a single coordinate from the origin).
    Ordinate,
    /// The length along an arc. The format gives this one its own entity
    /// (`ARC_DIMENSION`) rather than a value of group 70 -- that group says
    /// 5 on such a dimension, which would read as a three-point angular one.
    /// The entity's name is what decides it.
    ArcLength,
}

/// The dimension's text, as the file states it (DXF 1).
///
/// The format spells "just show what you measured" three ways -- the group
/// absent, an empty string, or the literal `<>` -- and readers of different
/// files (or of the same file through different libraries) see different
/// ones of the three. They are one meaning, so this type folds them into one
/// value: a consumer comparing two drawings must not see a difference that
/// is only a spelling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TextOverride {
    /// Show the measurement. The file wrote nothing, `""`, or `<>`.
    Measured,
    /// Show no text at all. The file wrote a single space.
    Suppressed,
    /// Show this instead. It may itself contain `<>`, standing for the
    /// measurement inside a longer string (`"<> H7"`); substituting it is a
    /// consumer's job, not this model's.
    Literal(String),
}

/// The points a dimension is built from, each one the DXF group the file
/// carries it in. `None` is "the file did not carry that group" -- normal,
/// since which groups a dimension uses depends on what it measures.
///
/// They are named per group rather than per subtype on purpose. The same
/// group number means the same thing across subtypes in the format, while a
/// library's own field names do not: LibreDWG's `def_pt`, for one, is group
/// 10 for most dimensions but is not written as a DXF group at all for a
/// two-line angular dimension, whose group 16 is its second extension line's
/// end. A backend that passes its own "definition point" through would put
/// two different points in one field depending on the file it read.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub struct DimensionPoints {
    /// DXF 13: the first extension line's origin.
    pub extension1: Option<Point3D>,
    /// DXF 14: the second extension line's origin.
    pub extension2: Option<Point3D>,
    /// DXF 15: the point a diameter, radius or angular dimension turns on.
    pub radial: Option<Point3D>,
    /// DXF 16: the point defining an angular dimension's arc.
    pub arc: Option<Point3D>,
}

/// A DIMENSION carries its drawn geometry (lines, arrows, text) as an
/// anonymous block (DXF 2) already in world coordinates, so drawing one is
/// drawing that block with an identity transform. All dimension subtypes
/// (aligned, angular, diameter, linear, ordinate, arc) share this shape and
/// fold into [`Entity::Dimension`].
///
/// Beyond that block it carries what the file says the dimension *is*: what
/// it measures, the measurement, the text, and the points it was built from.
/// Two of those can be missing from a file, and are `None` then rather than
/// filled in with a plausible value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DimensionEntity {
    pub common: EntityCommon,
    /// The anonymous block holding the drawn geometry (DXF 2).
    pub block_name: Ref<String>,
    /// DXF 70, low three bits. `None` when the file does not state it.
    pub kind: Option<DimensionKind>,
    /// DXF 42, the measurement the drawing recorded, in the drawing's units
    /// (radians for an angular dimension). `None` when the file does not
    /// carry it -- drawings older than R2000 often do not, and the format
    /// defines no default, so the only honest value is "not stated". It can
    /// also disagree with [`Self::text_override`], which is why both are
    /// carried.
    pub measurement: Option<f64>,
    /// DXF 1, folded to one value per meaning.
    pub text_override: TextOverride,
    /// DXF 10. What it locates depends on [`Self::kind`] -- the dimension
    /// line for a linear dimension, the far chord for a diameter, the
    /// feature for an ordinate.
    ///
    /// `None` when the reader cannot say which of its own points is this
    /// group. Every DXF dimension carries group 10, but a backend reading
    /// the binary format may hold a different point under its own
    /// "definition point" for some subtypes; saying nothing is better than
    /// putting two different points in one field.
    pub definition_point: Option<Point3D>,
    /// DXF 11: the middle of the dimension text.
    pub text_midpoint: Point2D,
    /// The rest of the points, by DXF group.
    pub points: DimensionPoints,
    /// DXF 50: the angle the dimension is measured along, radians. The
    /// format's default is 0, so an absent group is 0 rather than unknown.
    pub rotation: f64,
    /// DXF 53: the text's own rotation, radians. Default 0, like
    /// [`Self::rotation`].
    pub text_rotation: f64,
    /// DXF 3: the DIMSTYLE this dimension names.
    pub style_name: Ref<String>,
}

/// A 3DSOLID reduced to a wireframe: straight chords between the two
/// endpoint vertices of each edge of its boundary representation, in the
/// solid's own local 3D space. The solid's surfaces are not carried.
///
/// Empty when the wireframe could not be extracted (empty solid, unreadable
/// or unrecognized solid data) -- consumers should treat that like an
/// unsupported entity type, and `skipped_edges` says whether it was
/// "empty" or "not read".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Solid3DEntity {
    pub common: EntityCommon,
    pub wireframe_edges: Vec<[Point3D; 2]>,
    /// Edges in the solid data that could not be turned into a wireframe
    /// segment (endpoint vertices not resolvable). `0` for a fully read
    /// solid; a solid with no `wireframe_edges` and a non-zero count here
    /// was not read, not empty.
    #[serde(default)]
    pub skipped_edges: usize,
}

/// MULTILEADER's leader-line geometry only: the lines connecting the
/// content to its landing point, as polylines. The text or block content
/// itself is not carried, the same narrow scope as 3DSOLID's wireframe and
/// VIEWPORT's frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiLeaderEntity {
    pub common: EntityCommon,
    pub lines: Vec<Vec<Point3D>>,
}

/// One MLINE vertex: the centerline `point` plus `miter_direction`, a vector
/// that already accounts for the miter angle at this vertex, such that
/// `point + miter_direction * offset` is that vertex's position on the
/// parallel line `offset` away from the centerline.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct MLineVertex {
    pub point: Point3D,
    pub miter_direction: Point3D,
}

/// An MLINE is a set of parallel offset lines (wall-style multi-line). The
/// per-line offsets live in the referenced MLINESTYLE, looked up in
/// [`crate::tables::Tables::mlinestyles`]; the model carries the centerline
/// vertices and the style name.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MLineEntity {
    pub common: EntityCommon,
    pub vertices: Vec<MLineVertex>,
    pub closed: bool,
    pub mlinestyle_name: Ref<String>,
}

/// WIPEOUT's clip boundary, resolved to 2D points in the entity's own local
/// space (the image-entity pixel-to-world transform, DXF 10/11/12, is
/// already applied; block nesting composes on top).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WipeoutEntity {
    pub common: EntityCommon,
    pub boundary: Vec<Point2D>,
}

/// A light source: its position, its target, and what kind of light the
/// file says it is. It has no drawable shape of its own (it is invisible in
/// a plan view); what a consumer shows for it is the consumer's placeholder.
///
/// Whether the light *aims* at `target` is not carried: it is not something
/// the file states, but something that follows from `light_type` (distant
/// and spot lights aim, point lights do not). The model carries what the
/// file said and leaves that step to whoever needs it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LightEntity {
    pub common: EntityCommon,
    pub position: Point3D,
    pub target: Point3D,
    /// DXF 70. `None` when the file does not state it, or states a value
    /// outside the format's three.
    pub light_type: Option<LightType>,
}

/// What kind of light a LIGHT entity is (DXF 70).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LightType {
    /// 1: parallel rays from a direction.
    Distant,
    /// 2: radiates in every direction from `position`.
    Point,
    /// 3: a cone from `position` towards `target`.
    Spot,
}

/// Simple (non-MULTILEADER) LEADER: a polyline of `vertices` plus an
/// optional arrowhead at the first vertex. Style name, spline flag and
/// text-box size are not carried.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LeaderEntity {
    pub common: EntityCommon,
    pub vertices: Vec<Point3D>,
    /// DXF 71. `None` when the file does not state it -- a binary drawing
    /// always stores the flag, a text one may omit it, and which way the
    /// format reads an absent flag is not something this model can source.
    pub has_arrowhead: Option<bool>,
    /// DXF 72. `None` when the file does not state it: which way the format
    /// reads an absent group here is not something this model can source.
    pub path_type: Option<LeaderPath>,
    /// DXF 73, with the format's own default when the group is absent.
    pub annotation: LeaderAnnotation,
    /// DXF 340: the entity the leader points at, when the file names one.
    /// This is a connection the *file* states, not one anybody computed --
    /// which is why it belongs in the model rather than in a consumer.
    ///
    /// Three states, like every other reference: [`Ref::Resolved`] names an
    /// entity of this drawing; [`Ref::Unresolved`] keeps the handle the file
    /// wrote (hex) when no entity of the drawing answers to it; and
    /// [`Ref::Absent`] is a leader whose file names nothing.
    pub annotation_id: Ref<EntityId>,
    /// DXF 3: the DIMSTYLE this leader names.
    pub style_name: Ref<String>,
}

/// One entity of a parsed drawing.
///
/// A few variants share a payload type where the entity types are
/// structurally identical ([`Entity::XLine`] reuses [`RayEntity`],
/// [`Entity::Trace`] reuses [`SolidEntity`],
/// [`Entity::Region`]/[`Entity::PolylinePFace`] reuse [`Solid3DEntity`],
/// [`Entity::Polyline2D`] reuses [`LwPolylineEntity`]). They stay distinct
/// variants so [`type_name`](Self::type_name) still reports the real DXF
/// name.
// Internally tagged for JSON: `{"type":"LINE","common":{...},"start_point":...}`.
// Each variant's tag is spelled exactly as `type_name()` reports it -- the
// unit tests in json.rs check every variant for that agreement.
//
// Deliberately not `#[non_exhaustive]`: a new entity kind must reach every
// consumer that matches on `Entity` as a compile error, never as a wildcard
// arm that quietly ignores it. Adding a variant is therefore a breaking
// change (a 0.x minor bump), and that is the intended trade.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Entity {
    #[serde(rename = "LINE")]
    Line(LineEntity),
    #[serde(rename = "CIRCLE")]
    Circle(CircleEntity),
    #[serde(rename = "TEXT")]
    Text(TextEntity),
    #[serde(rename = "LWPOLYLINE")]
    LwPolyline(LwPolylineEntity),
    #[serde(rename = "ARC")]
    Arc(ArcEntity),
    #[serde(rename = "ELLIPSE")]
    Ellipse(EllipseEntity),
    #[serde(rename = "POINT")]
    Point(PointEntity),
    #[serde(rename = "SOLID")]
    Solid(SolidEntity),
    /// Same four corners as SOLID, in the same order.
    #[serde(rename = "TRACE")]
    Trace(SolidEntity),
    #[serde(rename = "RAY")]
    Ray(RayEntity),
    #[serde(rename = "XLINE")]
    XLine(RayEntity),
    #[serde(rename = "INSERT")]
    Insert(InsertEntity),
    #[serde(rename = "ATTRIB")]
    Attrib(AttribEntity),
    #[serde(rename = "ATTDEF")]
    Attdef(AttdefEntity),
    #[serde(rename = "VIEWPORT")]
    Viewport(ViewportEntity),
    #[serde(rename = "3DFACE")]
    Face3D(Face3DEntity),
    #[serde(rename = "SPLINE")]
    Spline(SplineEntity),
    #[serde(rename = "MTEXT")]
    MText(MTextEntity),
    #[serde(rename = "POLYLINE_3D")]
    Polyline3D(PolylineEntity),
    #[serde(rename = "DIMENSION")]
    Dimension(DimensionEntity),
    #[serde(rename = "HATCH")]
    Hatch(HatchEntity),
    #[serde(rename = "3DSOLID")]
    Solid3D(Solid3DEntity),
    #[serde(rename = "LEADER")]
    Leader(LeaderEntity),
    #[serde(rename = "MULTILEADER")]
    MultiLeader(MultiLeaderEntity),
    #[serde(rename = "MLINE")]
    MLine(MLineEntity),
    /// REGION: a planar solid, carried like a 3DSOLID (its outline as a
    /// wireframe).
    #[serde(rename = "REGION")]
    Region(Solid3DEntity),
    /// POLYLINE_PFACE ("polyface mesh"): each face's vertex indices
    /// resolved into wireframe edges, carried like [`Entity::Region`] -- a
    /// polyface mesh is just as inherently 3D as a solid's wireframe.
    #[serde(rename = "POLYLINE_PFACE")]
    PolylinePFace(Solid3DEntity),
    #[serde(rename = "POLYLINE_2D")]
    Polyline2D(LwPolylineEntity),
    #[serde(rename = "TOLERANCE")]
    Tolerance(ToleranceEntity),
    #[serde(rename = "ACAD_TABLE")]
    AcadTable(AcadTableEntity),
    #[serde(rename = "WIPEOUT")]
    Wipeout(WipeoutEntity),
    #[serde(rename = "LIGHT")]
    Light(LightEntity),
    /// An entity type the model does not have a shape for. Carries the DXF
    /// type name the file used, so consumers can still count and report by
    /// type and nothing is silently dropped. In JSON this is the one variant
    /// whose `"type"` tag (`"UNKNOWN"`) is not the DXF name; the real name is
    /// in `type_name`.
    #[serde(rename = "UNKNOWN")]
    Unknown {
        common: EntityCommon,
        type_name: String,
    },
}

impl Entity {
    pub fn common(&self) -> &EntityCommon {
        match self {
            Entity::Line(e) => &e.common,
            Entity::Circle(e) => &e.common,
            Entity::Text(e) => &e.common,
            Entity::LwPolyline(e) => &e.common,
            Entity::Arc(e) => &e.common,
            Entity::Ellipse(e) => &e.common,
            Entity::Point(e) => &e.common,
            Entity::Solid(e) | Entity::Trace(e) => &e.common,
            Entity::Ray(e) => &e.common,
            Entity::XLine(e) => &e.common,
            Entity::Insert(e) => &e.common,
            Entity::Attrib(e) => &e.common,
            Entity::Attdef(e) => &e.common,
            Entity::Viewport(e) => &e.common,
            Entity::Face3D(e) => &e.common,
            Entity::Spline(e) => &e.common,
            Entity::MText(e) => &e.common,
            Entity::Polyline3D(e) => &e.common,
            Entity::Dimension(e) => &e.common,
            Entity::Hatch(e) => &e.common,
            Entity::Solid3D(e) => &e.common,
            Entity::Leader(e) => &e.common,
            Entity::MultiLeader(e) => &e.common,
            Entity::MLine(e) => &e.common,
            Entity::Region(e) => &e.common,
            Entity::PolylinePFace(e) => &e.common,
            Entity::Polyline2D(e) => &e.common,
            Entity::Tolerance(e) => &e.common,
            Entity::AcadTable(e) => &e.common,
            Entity::Wipeout(e) => &e.common,
            Entity::Light(e) => &e.common,
            Entity::Unknown { common, .. } => common,
        }
    }

    /// [`common`](Self::common), mutably.
    pub fn common_mut(&mut self) -> &mut EntityCommon {
        match self {
            Entity::Line(e) => &mut e.common,
            Entity::Circle(e) => &mut e.common,
            Entity::Text(e) => &mut e.common,
            Entity::LwPolyline(e) => &mut e.common,
            Entity::Arc(e) => &mut e.common,
            Entity::Ellipse(e) => &mut e.common,
            Entity::Point(e) => &mut e.common,
            Entity::Solid(e) | Entity::Trace(e) => &mut e.common,
            Entity::Ray(e) => &mut e.common,
            Entity::XLine(e) => &mut e.common,
            Entity::Insert(e) => &mut e.common,
            Entity::Attrib(e) => &mut e.common,
            Entity::Attdef(e) => &mut e.common,
            Entity::Viewport(e) => &mut e.common,
            Entity::Face3D(e) => &mut e.common,
            Entity::Spline(e) => &mut e.common,
            Entity::MText(e) => &mut e.common,
            Entity::Polyline3D(e) => &mut e.common,
            Entity::Dimension(e) => &mut e.common,
            Entity::Hatch(e) => &mut e.common,
            Entity::Solid3D(e) => &mut e.common,
            Entity::Leader(e) => &mut e.common,
            Entity::MultiLeader(e) => &mut e.common,
            Entity::MLine(e) => &mut e.common,
            Entity::Region(e) => &mut e.common,
            Entity::PolylinePFace(e) => &mut e.common,
            Entity::Polyline2D(e) => &mut e.common,
            Entity::Tolerance(e) => &mut e.common,
            Entity::AcadTable(e) => &mut e.common,
            Entity::Wipeout(e) => &mut e.common,
            Entity::Light(e) => &mut e.common,
            Entity::Unknown { common, .. } => common,
        }
    }

    /// The DXF/entity type name, e.g. `"LINE"`.
    pub fn type_name(&self) -> &str {
        match self {
            Entity::Line(_) => "LINE",
            Entity::Circle(_) => "CIRCLE",
            Entity::Text(_) => "TEXT",
            Entity::LwPolyline(_) => "LWPOLYLINE",
            Entity::Arc(_) => "ARC",
            Entity::Ellipse(_) => "ELLIPSE",
            Entity::Point(_) => "POINT",
            Entity::Solid(_) => "SOLID",
            Entity::Trace(_) => "TRACE",
            Entity::Ray(_) => "RAY",
            Entity::XLine(_) => "XLINE",
            Entity::Insert(_) => "INSERT",
            Entity::Attrib(_) => "ATTRIB",
            Entity::Attdef(_) => "ATTDEF",
            Entity::Viewport(_) => "VIEWPORT",
            Entity::Face3D(_) => "3DFACE",
            Entity::Spline(_) => "SPLINE",
            Entity::MText(_) => "MTEXT",
            Entity::Polyline3D(_) => "POLYLINE_3D",
            Entity::Dimension(_) => "DIMENSION",
            Entity::Hatch(_) => "HATCH",
            Entity::Solid3D(_) => "3DSOLID",
            Entity::Leader(_) => "LEADER",
            Entity::MultiLeader(_) => "MULTILEADER",
            Entity::MLine(_) => "MLINE",
            Entity::Region(_) => "REGION",
            Entity::PolylinePFace(_) => "POLYLINE_PFACE",
            Entity::Polyline2D(_) => "POLYLINE_2D",
            Entity::Tolerance(_) => "TOLERANCE",
            Entity::AcadTable(_) => "ACAD_TABLE",
            Entity::Wipeout(_) => "WIPEOUT",
            Entity::Light(_) => "LIGHT",
            Entity::Unknown { type_name, .. } => type_name,
        }
    }
}
