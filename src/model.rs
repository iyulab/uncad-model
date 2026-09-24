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
//!
//! Coordinates are the ones the file states. Most entities state world
//! coordinates; the planar ones the reference defines in an object
//! coordinate system -- CIRCLE, ARC, LWPOLYLINE and POLYLINE_2D, TEXT,
//! ATTRIB, ATTDEF, INSERT, HATCH, SOLID and TRACE -- state points of their
//! own coordinate system, whose Z axis is the entity's `extrusion` (DXF 210;
//! see [`CircleEntity::extrusion`]). Taking those to world coordinates is a
//! consumer's step; the arithmetic for it, which every consumer needs alike,
//! lives with the model: the reference's "arbitrary axis algorithm"
//! ([`crate::Ocs`]) and a block reference's placement
//! ([`InsertEntity::world_transform`]).

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

/// One vertex of a 2D polyline -- an LWPOLYLINE, a 2D POLYLINE, or a HATCH
/// boundary path given as a polyline.
///
/// `bulge` (DXF 42) describes the segment from this vertex to the next one
/// (for the last vertex of a closed polyline, back to the first): `0` is a
/// straight segment; otherwise the segment is a circular arc and `bulge` is
/// the tangent of a quarter of its included angle, positive when the arc
/// turns counter-clockwise from this vertex to the next. A file that does
/// not write the group states a straight segment, so its absence is `0`.
///
/// `start_width` and `end_width` (DXF 40 and 41) are the same segment's
/// width where it leaves this vertex and where it reaches the next, in
/// drawing units; they differ for a tapered segment, such as an arrowhead
/// drawn as a polyline. A file that does not write them gives the vertex no
/// width of its own, so their absence is `0` -- and an LWPOLYLINE none of
/// whose vertices has one is as wide as its
/// [`LwPolylineEntity::const_width`]. A HATCH boundary has no widths; its
/// vertices carry `0`.
#[derive(Debug, Default, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PolylineVertex {
    pub point: Point2D,
    pub bulge: f64,
    /// DXF 40. An absent key reads as `0`.
    #[serde(default)]
    pub start_width: f64,
    /// DXF 41. An absent key reads as `0`.
    #[serde(default)]
    pub end_width: f64,
}

impl PolylineVertex {
    /// A vertex whose outgoing segment is straight and has no width of its
    /// own.
    pub fn straight(point: Point2D) -> Self {
        Self {
            point,
            bulge: 0.0,
            start_width: 0.0,
            end_width: 0.0,
        }
    }
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
    /// it. The value is what the file wrote: the handle as a hex string,
    /// the same form as [`EntityCommon::source_handle`]; for a pre-R13
    /// drawing, which points at its tables by index rather than by handle,
    /// the index as `idx:<n>`; and where the file points by name, the name.
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
    /// DXF 60: the file marks the entity invisible -- it is in the drawing
    /// but not drawn (a dynamic block's hidden visibility states are the
    /// common case). An optional group the file writes only when it is set,
    /// so an absent one is visible.
    #[serde(default)]
    pub invisible: bool,
    /// DXF 6: the linetype the entity is drawn with. The format writes the
    /// group only when it is not BYLAYER, so an absent one is
    /// [`EntityLinetype::ByLayer`]; so is every entity of a drawing too old
    /// to state one.
    #[serde(default)]
    pub linetype: EntityLinetype,
    /// DXF 48: the entity's own scale for its linetype's pattern, on top of
    /// the drawing's. An absent group is 1.
    #[serde(default = "one")]
    pub linetype_scale: f64,
    /// DXF 370: the entity's lineweight in hundredths of a millimetre, or
    /// one of the format's codes -1 (BYLAYER), -2 (BYBLOCK) and -3 (the
    /// application's default weight), as for
    /// [`crate::tables::LayerRecord::lineweight`]. From R2000 an absent
    /// group is -1. `None` when the drawing is older than R2000, which has
    /// no lineweights, or states a code the format does not define.
    #[serde(default)]
    pub lineweight: Option<i16>,
    /// DXF 440: how transparent the entity is drawn, as the file stores it
    /// -- the method in the high byte, an alpha in the low one; read it with
    /// [`Transparency::from_code`]. From R2004 an absent group is 0
    /// (BYLAYER). `None` when the drawing is older than R2004, which has no
    /// transparency.
    #[serde(default)]
    pub transparency: Option<u32>,
}

/// Which linetype an entity is drawn with (DXF 6): its layer's, its block
/// reference's, or one it names from the drawing's LTYPE table (which the
/// model does not carry), as a reference like [`EntityCommon::layer`].
///
/// Serialized adjacently tagged, like [`Ref`]: `{"type":"BY_LAYER"}`,
/// `{"type":"BY_BLOCK"}`, `{"type":"NAMED","data":{"type":"RESOLVED",
/// "data":"HIDDEN"}}`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EntityLinetype {
    #[default]
    ByLayer,
    ByBlock,
    Named(Ref<String>),
}

/// How transparent an entity is drawn, read from
/// [`EntityCommon::transparency`]: as its layer says, as its block reference
/// says, or by its own alpha -- 0 fully transparent, 255 opaque.
///
/// Serialized adjacently tagged: `{"type":"BY_LAYER"}`,
/// `{"type":"BY_BLOCK"}`, `{"type":"ALPHA","data":204}`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Transparency {
    ByLayer,
    ByBlock,
    Alpha(u8),
}

impl Transparency {
    /// The transparency a stored value states: its high byte says how to
    /// read it (0 BYLAYER, 1 BYBLOCK, 2 by the alpha in the low byte). `None`
    /// for a method the format does not define.
    pub fn from_code(code: u32) -> Option<Transparency> {
        match code >> 24 {
            0 => Some(Transparency::ByLayer),
            1 => Some(Transparency::ByBlock),
            2 => Some(Transparency::Alpha((code & 0xFF) as u8)),
            _ => None,
        }
    }
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
    /// In the circle's own coordinate system, whose Z axis is `extrusion`
    /// (DXF: the center is an OCS point). With the default extrusion that
    /// system is the world's; a mirrored circle has (0, 0, -1), and its world
    /// center is found through the format's arbitrary axis algorithm.
    pub center: Point3D,
    pub radius: f64,
    /// The normal of the circle's plane (DXF 210). An absent group is the
    /// default (0, 0, 1).
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TextEntity {
    pub common: EntityCommon,
    /// DXF 10: the text's first alignment point -- where its baseline
    /// starts -- in the OCS [`Self::extrusion`] defines.
    pub start_point: Point2D,
    pub text_height: f64,
    pub text: String,
    /// Radians (DXF 50). TEXT stores this as a plain angle, unlike MTEXT,
    /// whose rotation is a direction vector (see [`MTextEntity::rotation`]).
    pub rotation: f64,
    /// DXF 72, with the format's own default ([`HorizontalJustification::Left`])
    /// when the group is absent, and for a document written before this
    /// field existed.
    #[serde(default)]
    pub horizontal_justification: HorizontalJustification,
    /// DXF 73, with the format's own default
    /// ([`VerticalJustification::Baseline`]) when the group is absent, and
    /// for a document written before this field existed.
    #[serde(default)]
    pub vertical_justification: VerticalJustification,
    /// DXF 11: the point the text is justified at, in the OCS like
    /// `start_point` -- for [`HorizontalJustification::Aligned`] and
    /// [`HorizontalJustification::Fit`], the other end of its baseline.
    /// `None` for left/baseline justification, which the file places by
    /// `start_point` alone and states no such point for. With it,
    /// `start_point` is where the writing program computed the text to
    /// begin from its own font, and this point is the one the text answers
    /// to.
    pub alignment_point: Option<Point2D>,
    /// DXF 41: the characters' width relative to the width their style
    /// draws them at. Optional in the reference, with 1 as its default --
    /// a ratio -- so an absent group, and a document written before this
    /// field existed, reads as 1.
    #[serde(default = "one")]
    pub width_factor: f64,
    /// DXF 51: how far the characters slant from upright, radians.
    /// Optional in the reference, with 0 as its default, so an absent group,
    /// and a document written before this field existed, reads as 0.
    #[serde(default)]
    pub oblique_angle: f64,
    /// DXF 7: the text style (an entry of the drawing's STYLE table, which
    /// the model does not carry) the text is drawn in. The reference's
    /// default for an absent group is the style named `STANDARD`, so a
    /// reader resolves an absent group to that entry when the drawing has
    /// one; [`Ref::Absent`] when it has none, and for a document written
    /// before this field existed.
    #[serde(default = "absent")]
    pub style_name: Ref<String>,
    /// The text's points are points of its own coordinate system, whose Z
    /// axis is `extrusion`; this is their z there (DXF 30 -- the binary
    /// format stores it once, as the text's elevation). An absent group is
    /// `0`.
    #[serde(default)]
    pub elevation: f64,
    /// The normal of the text's plane (DXF 210). An absent group is the
    /// default (0, 0, 1). A mirror copy writes (0, 0, -1): its points'
    /// world x is reversed and the text reads mirrored.
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
}

/// LWPOLYLINE, and POLYLINE_2D ([`Entity::Polyline2D`]), which has the same
/// shape: a POLYLINE_2D's vertices are its VERTEX records (DXF 10/20, 42 and
/// 40/41 on each) -- a DXF's POLYLINE record states default widths (its own
/// 40/41) for the vertices that state none, and a reader gives those
/// vertices the defaults -- and its elevation is the z of its own group 10
/// (DXF 30), where an LWPOLYLINE has a group of its own for it (DXF 38).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LwPolylineEntity {
    pub common: EntityCommon,
    /// The vertices in order, each with the bulge and the widths of the
    /// segment that leaves it (see [`PolylineVertex`]). Points of the
    /// polyline's own coordinate system, whose Z axis is `extrusion`; with
    /// the default extrusion that system is the world's.
    pub vertices: Vec<PolylineVertex>,
    /// Whether the last vertex connects back to the first (DXF 70, bit 1).
    pub closed: bool,
    /// DXF 43: the width of every segment when no vertex has a width of its
    /// own (every vertex's `start_width` and `end_width` is `0`) -- the
    /// reference does not use it once a vertex states one. `0` is a line
    /// with no width. An absent group is `0`.
    ///
    /// A file that states this width again on every vertex, at both ends,
    /// draws the same polyline as one that states it only here, and a
    /// reader gives the two the same model: vertices with no width of their
    /// own. A POLYLINE_2D has no such group -- its widths are its vertices'
    /// -- and carries `0` here.
    #[serde(default)]
    pub const_width: f64,
    /// The z of every vertex in the polyline's own coordinate system (DXF 38
    /// for an LWPOLYLINE, the POLYLINE record's 30 for a 2D POLYLINE). An
    /// absent group is `0`.
    #[serde(default)]
    pub elevation: f64,
    /// The normal of the polyline's plane (DXF 210). An absent group is the
    /// default (0, 0, 1). A mirror copy writes (0, 0, -1): its vertices'
    /// world x is reversed, and so is the turn of every bulge.
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArcEntity {
    pub common: EntityCommon,
    /// In the arc's own coordinate system, whose Z axis is `extrusion`, as
    /// for [`CircleEntity::center`].
    pub center: Point3D,
    pub radius: f64,
    /// Radians, measured counter-clockwise about `extrusion` in the arc's own
    /// coordinate system -- a mirrored arc runs the other way in the world.
    pub start_angle: f64,
    /// Radians, as `start_angle`.
    pub end_angle: f64,
    /// The normal of the arc's plane (DXF 210). An absent group is the
    /// default (0, 0, 1).
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
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
    /// The normal of the ellipse's plane (DXF 210). `center` and
    /// `major_axis_endpoint` are world coordinates; the normal says which
    /// way the minor axis points (normal x major) and so which way the
    /// parameters run. A mirrored ellipse has (0, 0, -1). An absent group is
    /// the default (0, 0, 1).
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
}

/// The default extrusion direction (DXF 210): the world Z axis.
fn z_axis() -> Point3D {
    Point3D {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    }
}

/// A ratio's default, 1: no departure from what it is a ratio to.
fn one() -> f64 {
    1.0
}

/// What a document written before a reference field existed reads as.
pub(crate) fn absent<T>() -> Ref<T> {
    Ref::Absent
}

/// How a TEXT, ATTRIB or ATTDEF is justified along its baseline (DXF 72,
/// 0 to 5 in this order). The format's own default is [`Self::Left`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HorizontalJustification {
    /// The text starts at its start point.
    #[default]
    Left,
    /// Centred on its alignment point.
    Center,
    /// Ends at its alignment point.
    Right,
    /// Fills the baseline from its start point to its alignment point, its
    /// height scaled with its width.
    Aligned,
    /// Centred on its alignment point, horizontally and vertically.
    Middle,
    /// Fills the baseline from its start point to its alignment point at
    /// its own height, only its width scaled.
    Fit,
}

/// Which line of a TEXT, ATTRIB or ATTDEF sits on its alignment point (DXF
/// 73 on TEXT, 74 on ATTRIB and ATTDEF; 0 to 3 in this order). The
/// format's own default is [`Self::Baseline`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum VerticalJustification {
    /// The baseline.
    #[default]
    Baseline,
    /// The bottom of the descenders.
    Bottom,
    /// The middle of the text.
    Middle,
    /// The top of the text.
    Top,
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
    /// The corners are points of the entity's own coordinate system, whose Z
    /// axis is `extrusion`; this is their z there (DXF 30 -- the four corners
    /// share it). An absent group is `0`.
    #[serde(default)]
    pub elevation: f64,
    /// The normal of the entity's plane (DXF 210). An absent group is the
    /// default (0, 0, 1).
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
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
    /// DXF 10, in the OCS [`Self::extrusion`] defines.
    pub start_point: Point2D,
    pub text_height: f64,
    /// The attribute's tag (DXF 2): the name its value answers to, as the
    /// block definition's ATTDEF declared it. What a reader of a title
    /// block looks a value up by; the value alone says nothing about which
    /// field it fills.
    pub tag: String,
    /// DXF 70. A document written before this field existed reads as no
    /// flag set.
    #[serde(default)]
    pub flags: AttributeFlags,
    pub text: String,
    /// Radians (DXF 50).
    pub rotation: f64,
    /// DXF 72, with the format's own default ([`HorizontalJustification::Left`])
    /// when the group is absent, and for a document written before this
    /// field existed.
    #[serde(default)]
    pub horizontal_justification: HorizontalJustification,
    /// DXF 74 (73 is the field length here), with the format's own default
    /// ([`VerticalJustification::Baseline`]) when the group is absent, and
    /// for a document written before this field existed.
    #[serde(default)]
    pub vertical_justification: VerticalJustification,
    /// DXF 11: the point the value is justified at, in the OCS like
    /// `start_point` -- for [`HorizontalJustification::Aligned`] and
    /// [`HorizontalJustification::Fit`], the other end of its baseline.
    /// `None` for left/baseline justification, which the file places by
    /// `start_point` alone and states no such point for.
    pub alignment_point: Option<Point2D>,
    /// DXF 41: the characters' width relative to the width their style
    /// draws them at. Optional in the reference, with 1 as its default --
    /// a ratio -- so an absent group, and a document written before this
    /// field existed, reads as 1.
    #[serde(default = "one")]
    pub width_factor: f64,
    /// DXF 51: how far the characters slant from upright, radians.
    /// Optional in the reference, with 0 as its default, so an absent group,
    /// and a document written before this field existed, reads as 0.
    #[serde(default)]
    pub oblique_angle: f64,
    /// DXF 7: the text style (an entry of the drawing's STYLE table, which
    /// the model does not carry) the value is drawn in. The reference's
    /// default for an absent group is the style named `STANDARD`, so a
    /// reader resolves an absent group to that entry when the drawing has
    /// one; [`Ref::Absent`] when it has none, and for a document written
    /// before this field existed.
    #[serde(default = "absent")]
    pub style_name: Ref<String>,
    /// The z of the attribute's points in its own coordinate system, as
    /// [`TextEntity::elevation`]. An absent group is `0`.
    #[serde(default)]
    pub elevation: f64,
    /// The normal of the attribute's plane (DXF 210). An absent group is
    /// the default (0, 0, 1).
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
}

/// The flags of an ATTRIB or ATTDEF (DXF 70), one per bit, named as the
/// reference names them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AttributeFlags {
    /// Bit 1: the attribute is invisible -- the drawing holds its value but
    /// does not show it. Not the same flag as the one any entity can carry
    /// ([`EntityCommon::invisible`], DXF 60).
    pub invisible: bool,
    /// Bit 2: the attribute is constant -- its value is the definition's,
    /// the same in every block reference, and no ATTRIB carries it.
    pub constant: bool,
    /// Bit 4: the value is verified when it is entered.
    pub verify: bool,
    /// Bit 8: the value is preset -- entered without a prompt.
    pub preset: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertEntity {
    pub common: EntityCommon,
    /// Referenced block's name (DXF 2). Consumers look it up in
    /// [`crate::tables::Tables::block_records`] for the block's own
    /// entities.
    pub block_name: Ref<String>,
    /// A point of the INSERT's own coordinate system, whose Z axis is
    /// `extrusion`; with the default extrusion that system is the world's.
    pub insertion_point: Point3D,
    /// Per-axis scale factors (DXF 41/42/43); (1,1,1) if never set.
    pub scale: Point3D,
    /// Radians, measured counter-clockwise about `extrusion` in the
    /// INSERT's own coordinate system.
    pub rotation: f64,
    /// Attribute values attached to this INSERT (the ATTRIB records between
    /// the INSERT and its SEQEND). A parser also lists them as top-level
    /// [`Entity::Attrib`] entries in `CadDatabase::entities`; a consumer that
    /// draws `entities` gets them from there.
    pub attribs: Vec<AttribEntity>,
    /// The normal of the plane the block is placed in (DXF 210). An absent
    /// group is the default (0, 0, 1). A block mirrored with the reference
    /// writes (0, 0, -1) here and its scale as stated: the block's world x
    /// is reversed. [`InsertEntity::world_transform`] is the placement that
    /// follows from all of them.
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
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
    /// Placement as the binary format stores it, where a table shares an
    /// INSERT's placement fields. The DXF reference writes no scale for a
    /// table, and a horizontal direction vector (group 11) in place of a
    /// rotation, so a reader of text drawings has these to derive or to
    /// leave at identity -- and says which in its own documentation.
    pub scale: Point3D,
    /// Radians. See [`Self::scale`].
    pub rotation: f64,
}

/// Same field shape as ATTRIB -- the *template* stored in a block
/// definition, versus ATTRIB, the value attached to an INSERT.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AttdefEntity {
    pub common: EntityCommon,
    /// DXF 10, in the OCS [`Self::extrusion`] defines.
    pub start_point: Point2D,
    pub text_height: f64,
    /// The tag (DXF 2) every ATTRIB made from this definition carries.
    pub tag: String,
    /// DXF 70: the flags every ATTRIB made from this definition starts
    /// with. A document written before this field existed reads as no flag
    /// set.
    #[serde(default)]
    pub flags: AttributeFlags,
    pub default_value: String,
    /// Radians (DXF 50). Kept for parity with ATTRIB; a template is not
    /// normally drawn.
    pub rotation: f64,
    /// DXF 72, with the format's own default ([`HorizontalJustification::Left`])
    /// when the group is absent, and for a document written before this
    /// field existed.
    #[serde(default)]
    pub horizontal_justification: HorizontalJustification,
    /// DXF 74 (73 is the field length here), with the format's own default
    /// ([`VerticalJustification::Baseline`]) when the group is absent, and
    /// for a document written before this field existed.
    #[serde(default)]
    pub vertical_justification: VerticalJustification,
    /// DXF 11: the point the value is justified at, in the OCS like
    /// `start_point` -- for [`HorizontalJustification::Aligned`] and
    /// [`HorizontalJustification::Fit`], the other end of its baseline.
    /// `None` for left/baseline justification, which the file places by
    /// `start_point` alone and states no such point for.
    pub alignment_point: Option<Point2D>,
    /// DXF 41: the characters' width relative to the width their style
    /// draws them at. Optional in the reference, with 1 as its default --
    /// a ratio -- so an absent group, and a document written before this
    /// field existed, reads as 1.
    #[serde(default = "one")]
    pub width_factor: f64,
    /// DXF 51: how far the characters slant from upright, radians.
    /// Optional in the reference, with 0 as its default, so an absent group,
    /// and a document written before this field existed, reads as 0.
    #[serde(default)]
    pub oblique_angle: f64,
    /// DXF 7: the text style (an entry of the drawing's STYLE table, which
    /// the model does not carry) the value is drawn in. The reference's
    /// default for an absent group is the style named `STANDARD`, so a
    /// reader resolves an absent group to that entry when the drawing has
    /// one; [`Ref::Absent`] when it has none, and for a document written
    /// before this field existed.
    #[serde(default = "absent")]
    pub style_name: Ref<String>,
    /// The z of the definition's points in its own coordinate system, as
    /// [`TextEntity::elevation`]. An absent group is `0`.
    #[serde(default)]
    pub elevation: f64,
    /// The normal of the definition's plane (DXF 210). An absent group is
    /// the default (0, 0, 1).
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
}

/// A paper-space viewport: a frame on a layout's sheet, and the view of the
/// model it shows through that frame.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ViewportEntity {
    pub common: EntityCommon,
    /// DXF 10: the centre of the frame on the sheet, in paper space.
    pub center: Point3D,
    /// DXF 40: the frame's width, in paper space units.
    pub width: f64,
    /// DXF 41: the frame's height, in paper space units.
    pub height: f64,
    /// What the frame shows of the model. `None` when the record does not
    /// carry it: a viewport from a file older than R2000 keeps its view in
    /// extended data, which the model does not read. A document written
    /// before this field existed reads as `None` too.
    pub view: Option<ViewportView>,
    /// Whether the viewport is on, showing its view: bit 0x20000 of the
    /// viewport's status flags (DXF 90), set when it is off, which both
    /// formats state from R2000 on. A DXF older than that says it with
    /// group 68, where 0 is off; a later DXF writes 68 as well, but there 0
    /// is also the value of every viewport of a layout that is not the
    /// current one, on or not, so it is not what says it. `None` when the
    /// file does not state it.
    pub on: Option<bool>,
    /// DXF 69: the viewport's number within its layout. 1 is the layout's
    /// own overall viewport -- the one that is the sheet itself rather than
    /// a window onto the model. `None` when the file does not state it: the
    /// binary format stores no such number. A DXF from R2000 on writes 0 for
    /// every viewport of a layout that is not the current one; that 0 is
    /// carried as written, and it is no number -- numbers start at 1.
    pub viewport_id: Option<i32>,
    /// The layers frozen in this viewport alone (DXF 341; 331 in a DXF
    /// from R2004 on), in file order, each a reference like
    /// [`EntityCommon::layer`]. Empty when there are none, and for a
    /// document written before this field existed.
    #[serde(default)]
    pub frozen_layers: Vec<Ref<String>>,
}

/// What a paper-space viewport shows of the model: a view, stated the way
/// the format states one -- a target point, a direction to look along and
/// a twist, which together define the view's own display coordinates, and
/// the part of that plane the frame shows.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ViewportView {
    /// DXF 12: the centre of the view, in its display coordinates.
    pub center: Point2D,
    /// DXF 45: the height of the model the frame shows, in drawing units.
    /// The frame's own height is in paper units, so the two together are
    /// the scale the model is shown at.
    pub height: f64,
    /// DXF 17: the point of the model the view looks at, in world
    /// coordinates; display coordinates are measured from it.
    pub target: Point3D,
    /// DXF 16: the direction from the target towards the viewer, in world
    /// coordinates. (0, 0, 1) is a plan view.
    pub direction: Point3D,
    /// DXF 51: how far the view is turned about its direction, radians.
    pub twist: f64,
    /// DXF 42: the lens length of a perspective view, in millimetres; it
    /// has no effect on a parallel one.
    pub lens_length: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Face3DEntity {
    pub common: EntityCommon,
    /// Already-sequential order (unlike SOLID, no 1-2-4-3 reorder needed).
    pub corner1: Point3D,
    pub corner2: Point3D,
    pub corner3: Point3D,
    pub corner4: Point3D,
    /// Which edges the file marks invisible (DXF 70, bits 1, 2, 4 and 8):
    /// edge `i` runs from corner `i + 1` to the next, the fourth back to the
    /// first. A mesh of faces hides the edges it shares inside so that only
    /// its outline shows. An absent group is every edge visible.
    #[serde(default)]
    pub invisible_edges: [bool; 4],
}

impl Face3DEntity {
    /// The four edge flags of DXF group 70, bit `i` for edge `i`.
    pub fn invisible_edges_from_bits(bits: u32) -> [bool; 4] {
        [bits & 1 != 0, bits & 2 != 0, bits & 4 != 0, bits & 8 != 0]
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SplineEntity {
    pub common: EntityCommon,
    /// Degree of the curve's polynomial pieces (DXF 71).
    pub degree: u32,
    /// Whether the file declares the curve closed (DXF 70, bit 1). `None`
    /// when the record does not say -- a spline stored by its fit points
    /// does not, in the formats that predate the flag on that form.
    pub closed: Option<bool>,
    /// Whether the file declares the curve periodic (DXF 70, bit 2). `None`
    /// under the same condition as `closed`.
    pub periodic: Option<bool>,
    /// The knot vector, in file order (DXF 40). Empty when the file defines
    /// the spline by its fit points and stores no knots.
    pub knots: Vec<f64>,
    /// One weight per control point (DXF 41). Empty when the file gives no
    /// weights, which means every weight is 1 -- the curve is not rational.
    pub weights: Vec<f64>,
    /// Points that lie exactly on the curve (DXF 11). Empty when the file
    /// defines the spline by its control points alone.
    pub fit_points: Vec<Point3D>,
    /// Control points (DXF 10). With `degree`, `knots` and `weights` they
    /// define the curve; they do not lie on it.
    pub control_points: Vec<Point3D>,
    /// DXF 12: the curve's direction at its first fit point, which a spline
    /// defined by fit points may state. `None` where the file states none.
    #[serde(default)]
    pub start_tangent: Option<Point3D>,
    /// DXF 13: the direction at its last fit point, likewise.
    #[serde(default)]
    pub end_tangent: Option<Point3D>,
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
    /// DXF 44: the line spacing as a fraction of the default spacing.
    /// Optional in the DXF reference, which writes optional groups only when
    /// they differ from the default -- and a fraction *of* the default is 1
    /// there, so an absent group is 1.
    pub line_spacing_factor: f64,
    /// DXF 71: which point of the text block `insertion_point` is. Without
    /// it the insertion point does not say where the text goes -- the same
    /// point is the block's top-left corner, its middle or its bottom-right,
    /// depending on this. `None` when the file does not state it.
    pub attachment: Option<MTextAttachment>,
    /// The width of the box the text is laid out in (DXF 41, the reference
    /// rectangle's width), in drawing units: the writing program wraps each
    /// paragraph at it. `0` -- also what an absent group, and a document
    /// written before this field existed, means -- is no box, and each
    /// paragraph is one line. The box's height is not stated by the format
    /// for a drawn text; it follows from the lines.
    #[serde(default)]
    pub reference_width: f64,
    /// DXF 42: the width of the text block as the application that wrote
    /// the file last laid it out -- a measurement, which the reference
    /// calls read-only, of the text in its font, so a consumer without the
    /// font still knows how far the text reaches. `None` when the file does
    /// not state it; a zero measures no text, and reads as `None` too.
    pub extents_width: Option<f64>,
    /// DXF 43: the height of the text block, stated and read like
    /// [`Self::extents_width`].
    pub extents_height: Option<f64>,
    /// DXF 7: the text style the text is drawn in, as
    /// [`TextEntity::style_name`] -- inline format codes in `text` can
    /// still change the font of a part of it.
    #[serde(default = "absent")]
    pub style_name: Ref<String>,
}

/// Which point of an MTEXT block its insertion point is (DXF 71, 1 to 9 in
/// this order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MTextAttachment {
    TopLeft,
    TopCenter,
    TopRight,
    MiddleLeft,
    MiddleCenter,
    MiddleRight,
    BottomLeft,
    BottomCenter,
    BottomRight,
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
    /// A straight edge from `start` to `end` (DXF 10/20 and 11/21). The
    /// format states both; in a closed path the end usually meets the next
    /// edge's start, but nothing in the format requires the edges to be
    /// listed in order.
    Line { start: Point2D, end: Point2D },
    Arc {
        center: Point2D,
        radius: f64,
        /// Radians, as the file states it. For a counter-clockwise edge it is
        /// measured counter-clockwise from +x; for a clockwise edge
        /// (`is_ccw` false) the format measures it clockwise -- it writes the
        /// complement of the counter-clockwise angle -- so the edge starts at
        /// `-start_angle` counter-clockwise.
        start_angle: f64,
        /// Radians, measured the same way as `start_angle`.
        end_angle: f64,
        /// Whether the edge runs counter-clockwise from its start to its end
        /// (DXF 73).
        is_ccw: bool,
    },
    Ellipse {
        center: Point2D,
        /// Major-axis endpoint, relative to `center`.
        end: Point2D,
        /// Minor/major axis length ratio.
        minor_major_ratio: f64,
        /// Radians, the ellipse's parameter, measured as for an arc edge's
        /// `start_angle` -- clockwise for a clockwise edge.
        start_angle: f64,
        end_angle: f64,
        is_ccw: bool,
    },
    /// A spline edge, as the format states it: its degree (DXF 94), whether
    /// it is rational (73) and periodic (74), its knots (40), control points
    /// (10/20) and -- for a rational one -- their weights (42, empty
    /// otherwise). From R2010 on the format may also state the points it was
    /// fitted through (97, 11/21) and, with them, the tangents at its two
    /// ends (12/22, 13/23); `None` where it does not.
    Spline {
        degree: u32,
        rational: bool,
        periodic: bool,
        knots: Vec<f64>,
        control_points: Vec<Point2D>,
        weights: Vec<f64>,
        fit_points: Vec<Point2D>,
        start_tangent: Option<Point2D>,
        end_tangent: Option<Point2D>,
    },
}

/// One HATCH boundary path -- either an explicit polyline (its vertices
/// carry their bulge, see [`PolylineVertex`]) or a list of curved/straight
/// edges. A polyline path is a closed loop.
// JSON: adjacently tagged, because the payload is a sequence rather than a
// struct -- `{"type":"POLYLINE","data":[pt,..]}` / `{"type":"EDGES","data":
// [edge,..]}` -- keeping the `type` key every other tagged object uses.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "UPPERCASE")]
pub enum HatchBoundaryPath {
    Polyline(Vec<PolylineVertex>),
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
    /// The boundary paths and pattern lines are points and directions of the
    /// hatch's own coordinate system, whose Z axis is `extrusion`; this is
    /// their z there (DXF 30 of the elevation point). An absent group is `0`.
    #[serde(default)]
    pub elevation: f64,
    /// The normal of the hatch's plane (DXF 210). An absent group is the
    /// default (0, 0, 1). A mirror copy writes (0, 0, -1): its boundary's
    /// world x is reversed, and so is the turn of every arc edge and bulge.
    #[serde(default = "z_axis")]
    pub extrusion: Point3D,
    /// Which of the areas the boundary paths enclose are filled (DXF 75).
    /// `None` when the file does not say, or says a value the format does
    /// not define.
    #[serde(default)]
    pub style: Option<HatchStyle>,
}

/// Which areas a HATCH fills among nested boundary paths (DXF 75).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HatchStyle {
    /// 0: alternate areas from the outside in -- an island is left empty,
    /// an island within it filled again.
    Normal,
    /// 1: only the outermost area; everything inside the first island is
    /// left empty.
    Outer,
    /// 2: the whole outermost area, islands ignored.
    Ignore,
}

impl HatchStyle {
    /// The style DXF group 75 names, `None` for a value outside the three.
    pub fn from_code(code: i64) -> Option<HatchStyle> {
        match code {
            0 => Some(HatchStyle::Normal),
            1 => Some(HatchStyle::Outer),
            2 => Some(HatchStyle::Ignore),
            _ => None,
        }
    }
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
    /// datum an ordinate measures from (its feature is group 13).
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
    /// DXF 50: the angle the dimension is measured along, radians. An
    /// absent group is 0. The DXF reference does not mark the group
    /// optional, but writers leave it out of an unrotated dimension; what it
    /// states is a rotation away from the horizontal, so saying nothing
    /// states none.
    pub rotation: f64,
    /// DXF 53: the text's own rotation away from its default orientation,
    /// radians. Optional in the DXF reference, which writes optional groups
    /// only when they differ from the default -- and a rotation *away from*
    /// the default orientation is 0 there, so an absent group is 0.
    pub text_rotation: f64,
    /// DXF 3: the DIMSTYLE this dimension names.
    pub style_name: Ref<String>,
    /// DXF 70, bit 64, on an ordinate dimension: which coordinate of its
    /// feature it measures from the datum -- [`OrdinateAxis::X`] when the
    /// bit is set, [`OrdinateAxis::Y`] when it is clear. `None` on every
    /// other kind, where the bit means nothing, and for a document written
    /// before this field existed.
    pub ordinate_axis: Option<OrdinateAxis>,
}

/// Which coordinate an ordinate dimension measures (DXF 70, bit 64).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrdinateAxis {
    /// The bit is set: the feature's x distance from the datum.
    X,
    /// The bit is clear: the feature's y distance from the datum.
    Y,
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
/// itself is not carried, the same narrow scope as 3DSOLID's wireframe.
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
/// vertices, the style name and the scale the style is drawn at.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MLineEntity {
    pub common: EntityCommon,
    pub vertices: Vec<MLineVertex>,
    pub closed: bool,
    pub mlinestyle_name: Ref<String>,
    /// DXF 40: the factor the style's offsets are multiplied by for this
    /// MLINE -- a wall 200 units thick, drawn in a style whose offsets are
    /// 0.5 and -0.5, states 200 here. The format requires the group; `None`
    /// is a model that was not given it, such as a document written before
    /// this field existed.
    pub scale: Option<f64>,
}

/// WIPEOUT's clip boundary, resolved to 2D points in the entity's own local
/// space (the image-entity pixel-to-world transform, DXF 10/11/12, is
/// already applied; block nesting composes on top). A closed loop: its first
/// point is not repeated at the end, however the file wrote it.
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
/// [`Entity::Region`]/[`Entity::PolylinePFace`]/[`Entity::PolylineMesh`]
/// reuse [`Solid3DEntity`],
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
    /// POLYLINE_MESH ("polygon mesh"): an M by N grid of vertices (DXF 71
    /// and 72), carried like [`Entity::PolylinePFace`] as the wireframe of
    /// its grid lines. The file stores the vertices row by row, so vertex
    /// `i * N + j` is row `i`, column `j`; the edges come in a fixed order
    /// so that two readers of one mesh agree edge for edge: first
    /// `(i, j)-(i + 1, j)` for each row `i` in turn and each column `j`
    /// within it, then `(i, j)-(i, j + 1)` in the same order. Closed in M
    /// (DXF 70, bit 1), `i + 1` wraps from the last row to the first; closed
    /// in N (bit 32), `j + 1` wraps from the last column to the first.
    #[serde(rename = "POLYLINE_MESH")]
    PolylineMesh(Solid3DEntity),
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
            Entity::PolylineMesh(e) => &e.common,
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
            Entity::PolylineMesh(e) => &mut e.common,
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
            Entity::PolylineMesh(_) => "POLYLINE_MESH",
            Entity::Polyline2D(_) => "POLYLINE_2D",
            Entity::Tolerance(_) => "TOLERANCE",
            Entity::AcadTable(_) => "ACAD_TABLE",
            Entity::Wipeout(_) => "WIPEOUT",
            Entity::Light(_) => "LIGHT",
            Entity::Unknown { type_name, .. } => type_name,
        }
    }
}
