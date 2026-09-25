# uncad-model

The neutral entity model for 2D CAD drawings: what a drawing *is* once it has been taken out of its file format.

Every entity carries a **reference ID**, a **provenance** and a **confidence** — so that anything built on the model can say where a value came from and how far it can be trusted, including "unknown".

This crate is pure data: types and serialization, plus the coordinate arithmetic the format defines on an entity's own fields, which every consumer needs alike -- the placement an INSERT applies to its block (`Affine2`), the coordinate system an entity is written in (`Ocs`), the arc a polyline vertex's bulge describes (`BulgeArc`), the point at a parameter on an ellipse or a NURBS curve (`EllipseEntity::point_at`, `Nurbs`), and how far an arc or an ellipse runs (`ArcEntity::sweep`, `EllipseEntity::sweep`) -- and the unit a `$INSUNITS` code names (`Units`). It parses nothing, renders nothing, and has no native dependencies.

## Dependencies

The dependency tree is permissive-only (MIT / Apache-2.0 / BSD). This crate depends on no
parser, no renderer and no native code, so anything that needs only to *describe* a drawing can
depend on it alone.

## Status

0.x. The crate provides:

- **Entities** — `Entity` and one struct per kind, each carrying a reference ID, a provenance and
  a confidence, with a three-state `Ref` for every reference and plain `Point2D` / `Point3D`.
- **Tables** — `Tables`: layers, block definitions, dimension styles, mline styles, and the
  layouts with their plot settings.
- **Placement** — `Affine2`, the transform a block reference applies to its block, composed across
  nested references; `InsertEntity::world_transform` places a mirror copy's block too.
- **Palette and diagnostics** — the ACI colour table and the reader diagnostics a drawing carries.
- **Serialization** — `CadDatabase::to_json`, a direct serde form that round-trips.

The design rules are in [docs/principles.md](docs/principles.md).

`golden/` is a test-only crate (not published): synthetic drawing specs, a DXF writer for them,
and property checks, usable as a dev-dependency.

## License

MIT
