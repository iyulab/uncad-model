# Changelog

Notable changes to this project are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versioning follows
[Semantic Versioning](https://semver.org/). While the crate is in 0.x, a breaking change
bumps the minor version.

## [Unreleased]

## [0.7.0] - 2026-10-07

### Added

- `MultiLeaderEntity::arrow_size`, an `Option<f64>`: the size every arrowhead of a multileader is
  drawn at -- its context data's arrowhead size (DXF 140, the content scale already applied), or,
  from R2010, a line's own (40 in its `LEADER_LINE{` block) when that line's override flags (93)
  set bit 0x10. `None` when the context data states no size, a size is not a finite number of zero
  or more, or the lines come out with different sizes. `MultiLeaderEntity::resolve_arrow_size`
  works it out from those groups.

## [0.6.0] - 2026-10-07

### Added

- `AcadTableEntity::flow`, an `Option<TableFlow>`: which way a table's rows run from its insertion
  point -- `Down` (the first row at the top, the insertion point the upper-left corner) or `Up`
  (the first row at the bottom, the insertion point the lower-left corner). It is the table's own
  flow direction (DXF 70) when it overrides its style, otherwise its TABLESTYLE's; `None` when
  neither states it or the reader did not read it. `TableFlow::from_code` reads the DXF 70 code.

### Changed

- `TableGrid`'s rows are documented as running from the first row, which is the top row only when
  the table's flow runs down.

## [0.5.0] - 2026-10-05

### Added

- `AcadTableEntity::grid`, an `Option<TableGrid>`: a table's column widths and its rows, each row
  with its height and one `TableCell` per column -- the cell's kind (`TableCellKind`: text or
  block), its text in the file's format codes, whether another cell's span covers it, and how many
  columns and rows it spans. An empty cell has no text however the writing version spelled it (an
  empty string or no value), so two drawings do not differ only in that spelling. `None` when the
  reader did not read the cells or they do not form a whole grid, and for a document written
  before the field existed. A table's cells were not carried at all before.

### Changed

- **Breaking:** `AcadTableEntity` has a new field, `grid`; code that builds it as a struct literal
  adds `grid: None` (or the grid its source states). JSON written before the field existed reads
  as "not read".

## [0.4.0] - 2026-10-04

### Added

- `MultiLeaderEntity::content`, an `Option<MultiLeaderContent>`: what a multileader points out, as
  its context data states it -- `MTEXT` (`MultiLeaderText`: the text in MTEXT's format codes, its
  style, location, direction, plane, character height, line spacing, rotation, column width,
  content scale and attachment point) or `BLOCK` (`MultiLeaderBlock`: the block, location, scale,
  rotation and plane). `None` for a leader with no content, and for a document written before the
  field existed. A multileader's text was not carried at all before.
- `MultiLeaderEntity::line_type`, a `LeaderLineType` (invisible, straight or spline): how the
  entity's leader lines are drawn between their points. The format settles it in layers -- the
  entity's own type when its override flags say so, else its MLEADERSTYLE's, and from R2010 a
  line's own where that line overrides -- and `LeaderLineType::resolve` applies them, so every
  reader settles it the same way. `None` when the layers do not settle one type (the style is
  missing, a code is undefined, or the lines differ) and for documents written before the
  field existed.
- `HeaderVariables::fingerprintguid` and `HeaderVariables::versionguid`: the `$FINGERPRINTGUID`
  and `$VERSIONGUID` a drawing states (R2000 and later), as stated. The fingerprint is kept
  from the drawing's creation, also by copies and drawings made from the same template, so it
  says where a drawing came from rather than which drawing it is; the version identifier
  changes with a save that changes the drawing.
- `MTextAttachment::from_code`: the attachment point a format code states (MTEXT's DXF 71, a
  multileader text's 171), `None` for a code the format does not define.
- Golden case `g20`: multileaders whose line type each of those layers settles, written as an
  R2010 file. The golden writer now writes R2010 when a case holds a multileader, and the
  expected model follows the version it writes.

### Changed

- **Breaking:** `MultiLeaderEntity` has two new fields, `line_type` and `content`, and
  `HeaderVariables` two, `fingerprintguid` and `versionguid`; code that builds them as struct
  literals adds `line_type: None, content: None` (or what its source states), and
  `..HeaderVariables::default()` or the two values. JSON written before the fields existed
  reads as "not stated".

## [0.3.0] - 2026-10-02

### Added

- `CadDatabase::header`, a `HeaderVariables`: the header variables that say what the
  drawing's numbers mean, each as the file states it (`None` when it does not). It carries
  `insunits`, the `$INSUNITS` code, and `HeaderVariables::units` names the unit that code
  stands for. JSON written before the field existed reads as "none stated".
- `MultiLeaderEntity::drawn_lines` and `MultiLeaderEntity::doglegs`: the leader lines as the
  format draws them -- each line on to its root's `last_point` -- and each root's dogleg from
  there, so every consumer draws and measures the same strokes.

### Changed

- **Breaking:** `CadDatabase` has a new field, `header`; code that builds one as a struct
  literal adds `header: HeaderVariables::default()` (or the values its source states).
- **Breaking:** `MultiLeaderEntity::lines` moves under the leader roots the lines run to:
  `MultiLeaderEntity::leaders` is a list of `LeaderRoot { lines, last_point, dogleg }`. A
  line is drawn through its vertices and then to its root's `last_point` (DXF 10 of the
  `LEADER{` block); the `Dogleg { direction, length }` (DXF 11, 40) runs from there towards
  the content. Each is `None` when the root's own flag (DXF 290, 291) says the file has none.
  A line of a single vertex -- the usual case -- was a point before; with its root it is the
  line the drawing shows. Callers: read `leaders[].lines` for what `lines` held, and draw each
  line on to its root's `last_point`.

## [0.2.1] - 2026-09-30

### Added

- `acis::wireframe_sab`: the wireframe of an ACIS body stored as SAB, the binary form of the
  SAT records -- the same edges `acis::wireframe` reads from the text. `None` when the bytes do
  not decode as a SAB body (wrong signature, no end-of-data marker, or a tag whose encoding is
  not known), rather than a partly decoded body.

### Fixed

- `acis::wireframe` reads a SAT text from before ACIS 2.0 (version below 200, as R13 and R14
  write it): its header is one line, not three, and skipping three lines lost the first two
  records, so every pointer after them landed two records off and no edge resolved.

## [0.2.0] - 2026-09-29

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

- `acis::wireframe`: the wireframe a `Solid3DEntity` carries, from the SAT text of an ACIS
  body -- one straight segment per edge and the count of edges it could not resolve -- so that
  every reader of a 3DSOLID or a REGION computes it the same way.
- `DimStyleRecord::overridden(&[StyleOverride])`: the style as one dimension or leader sees it,
  each of its own overrides replacing the variable of its group. An override of the wrong kind
  leaves that variable `None`; one the record does not carry is skipped.
- `LinearUnitFormat::from_code`, `AngularUnitFormat::from_code` and
  `FractionFormat::from_code`, beside `ArcSymbol::from_code`.
- `DimensionEntity::style_overrides` and `LeaderEntity::style_overrides`: the dimension-style
  variables one dimension or leader sets for itself (its extended data's `DSTYLE` list), as
  `StyleOverride { variable, value }` with `OverrideValue` in the kind the file states. `None`
  when the reader did not look -- and for a document written before the field; an empty list
  when it looked and there is none.
- `ArcEntity::sweep`: how far an ARC runs counter-clockwise from its start angle to its end
  angle, within one turn. Angles a whole turn apart are the whole circle
  (`WHOLE_TURN_TOLERANCE`). Equal angles give `None`, because the format does not say whether
  they mean the whole circle or nothing.
- `EllipseEntity::sweep` and `EllipseEntity::extremes`: how far an ELLIPSE runs in its
  parameter, and the points where its arc turns in world x or y. Together with the arc's two
  ends, these points give its extent.
- `Units` (module `units`): the unit a `$INSUNITS` code names, with its length in
  millimetres. This is the DXF reference's table.
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

- `TextKind` documents a dimension's text and a tolerance frame's text as written in MTEXT
  codes (`\X`, `\S…;`, `{\Fgdt;…}`), not in percent codes only.
- A block reference puts its block's base point on the insertion point. The placement took
  every block as based at the origin, so a block with another base point was drawn off by it.

## [0.1.0] - 2026-09-22

Initial release. A neutral entity model for 2D CAD drawings: `Entity` with one struct per kind,
each carrying a reference ID, a provenance and a confidence, and a three-state `Ref` for every
reference; `Tables` for layers, block definitions, dimension styles and mline styles; `Affine2`,
the placement a block reference applies to its block, composed across nested references; the ACI
colour table and reader diagnostics; and `CadDatabase::to_json`, a direct serde form that
round-trips. The crate parses nothing, renders nothing, and has no native dependencies.
