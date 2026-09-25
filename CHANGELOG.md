# Changelog

Notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versioning follows
[Semantic Versioning](https://semver.org/). While the crate is in 0.x, a breaking change
bumps the minor version.

## [Unreleased]

### Changed

- **Breaking:** `Affine2::from_insert` and `InsertEntity::transform` take the block's base
  point as a second argument. Pass `tables.block_records[name].base_point` (the new
  `BlockRecord::base_point`); passing the origin gives the 0.1.0 result.
- **Breaking:** polyline vertices are `PolylineVertex { point, bulge, start_width, end_width }`
  instead of `Point2D`, in `LwPolylineEntity::vertices` (LWPOLYLINE and POLYLINE_2D) and
  `HatchBoundaryPath::Polyline`. Build one with `PolylineVertex::straight(point)`; in JSON each
  vertex is an object, not a bare point.
- **Breaking:** `SplineEntity` gains `degree`, `closed`, `periodic`, `knots` and `weights`, the
  fields that define its curve, and `HatchEdge::Spline` gains the same plus `rational`,
  `fit_points` and end tangents. `HatchEdge::Line` gains its `end` point.
- **Breaking:** `LeaderEntity::has_arrowhead` is `Option<bool>` (`None`: the file does not say),
  and `LeaderEntity::annotation_id` is a three-state `Ref<EntityId>` instead of
  `Option<EntityId>`, tagged in JSON like every other reference.
- **Breaking:** `LightEntity::has_target` is removed. Use `light_type: Option<LightType>`
  (distant, point, spot), the type the file states; a light aims at its target when it is
  distant or spot.
- **Breaking:** `Entity` has two new variants, `PolylineMesh` and `Image`, that used to arrive as
  `Entity::Unknown`. `Entity` is exhaustive on purpose: add an arm to every match on it.
- **Breaking:** fields were added to public structs, so struct literals and exhaustive
  destructuring must name them: `EntityCommon`, most entity structs, `Tables`, `BlockRecord`,
  `LayerRecord` and `DimStyleRecord` (see Added). Each has a documented JSON default.
- A document 0.1.0 wrote still loads, the new fields taking their documented defaults, except
  where the shape changed: its SPLINEs, LEADERs, and LWPOLYLINE, POLYLINE_2D and HATCH
  polyline-path vertices no longer deserialize.
- A dimension style variable the file leaves unwritten reads as its value where every template
  starts alike (tolerances 0, factors 1, switches off, decimal formats); text height, arrow
  size, decimal places and zero suppression stay `None`. `None` no longer just means unwritten.

### Added

- `Ocs`: an entity's own coordinate system from its extrusion (the DXF reference's arbitrary
  axis algorithm), with `to_world` and `flat_map`, the exact 2D map of a plane parallel to XY.
- `InsertEntity::world_transform`: a block reference's placement taken through its own plane,
  so a mirrored copy is placed correctly. `None` for a plane tilted out of the world's.
- `BulgeArc` and `bulge::segments`: the arc a polyline vertex's bulge describes, with its
  points, extremes and sweep, and a polyline's segments one by one.
- `Nurbs` (domain, knot spans, `point_at`), `SplineEntity::nurbs`, and `EllipseEntity::point_at`
  and `minor_axis`: points on the curves an entity's own fields define.
- `text::decode_escapes` and `text::decode_caret` undo how a file stored a string (`\U+`, `\M+`,
  caret notation); `text::tokens` splits a text into what its percent and MTEXT codes say.
- `EntityCommon` carries the invisible flag, linetype (`EntityLinetype`), linetype scale,
  lineweight and transparency (`Transparency::from_code` reads the stored value).
- The planar entities (CIRCLE, ARC, LWPOLYLINE, POLYLINE_2D, TEXT, ATTRIB, ATTDEF, INSERT, HATCH,
  SOLID, TRACE) and ELLIPSE carry their extrusion, and those that have one their elevation.
- TEXT, ATTRIB and ATTDEF carry justification, alignment point, width factor, oblique angle and
  style; attributes their flags (`AttributeFlags`). MTEXT carries its attachment point,
  reference width, measured extents and style.
- VIEWPORT carries its view (`ViewportView`), on state, number and frozen layers; a layer its
  off, frozen, locked and plot state, lineweight and linetype.
- `Tables::layouts`: every layout, with its limits, plot settings (`PlotSettings`), stored
  extents, active viewport and paper-space linetype scaling.
- IMAGE entities (`ImageEntity`) and the image definitions they name (`Tables::image_definitions`,
  `ImageDefinition`).
- `Entity::PolylineMesh`: a polygon mesh carried as its grid's wireframe, in a stated edge order.
- Dimension styles carry arrow size, unit and fraction formats, zero suppression, rounding,
  angular decimal places and the arc symbol position. An ordinate dimension says which axis it
  measures (`OrdinateAxis`).
- Smaller fields: an MLINE's scale, which 3DFACE edges are invisible, a SPLINE's end tangents, a
  HATCH's fill style (`HatchStyle`), and `from_code` decoders for the new enums.

### Fixed

- A block reference puts its block's base point on the insertion point. The placement took
  every block as based at the origin, so a block with another base point was drawn off by it.

## [0.1.0] - 2026-09-22

Initial release. A neutral entity model for 2D CAD drawings: `Entity` with one struct per kind,
each carrying a reference ID, a provenance and a confidence, and a three-state `Ref` for every
reference; `Tables` for layers, block definitions, dimension styles and mline styles; `Affine2`,
the placement a block reference applies to its block, composed across nested references; the ACI
colour table and reader diagnostics; and `CadDatabase::to_json`, a direct serde form that
round-trips. The crate parses nothing, renders nothing, and has no native dependencies.
