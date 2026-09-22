# uncad-model

The neutral entity model for 2D CAD drawings: what a drawing *is* once it has been taken out of its file format.

Every entity carries a **reference ID**, a **provenance** and a **confidence** — so that anything built on the model can say where a value came from and how far it can be trusted, including "unknown".

This crate is pure data: types and serialization, plus the one piece of arithmetic every consumer of a block reference needs alike -- the placement an INSERT applies to its block (`Affine2`). It parses nothing, renders nothing, and has no native dependencies.

## Who uses it

| Crate | Relation to the model |
|---|---|
| [uncad](https://github.com/iyulab/uncad) | DWG/DXF → model |
| [iron-scout-cad](https://github.com/iyulab/iron-scout-cad) | model → semantic summary, entity references |
| [iron-hand-cad](https://github.com/iyulab/iron-hand-cad) | (model, reference, verb) → new model |
| [iron-diff-cad](https://github.com/iyulab/iron-diff-cad) | (model, model) → numeric change set |
| [iron-render-cad](https://github.com/iyulab/iron-render-cad) | model → SVG/PNG, change overlays |

Dependencies point **toward** this crate. It depends on none of the above — in particular, it does not depend on `uncad` and does not inherit its license.

## Status

0.x. The crate carries the entity types (`Entity` and one struct per kind, with the reference ID, provenance and confidence markers of [docs/principles.md](docs/principles.md) section 2 on every one, three-state `Ref` for every reference, plain `Point2D`/`Point3D`), the tables (`Tables`: layers, block definitions, mline styles), the ACI palette, the reader diagnostics, and the JSON serialization (`CadDatabase::to_json`, a direct serde form that round-trips). Read the principles before proposing anything.

`golden/` is a test-only crate (not published): synthetic drawing specs, a DXF writer for them, and property checks. Consumers use it as a dev-dependency to run their share of the golden cases.

## License

MIT
