# uncad-model

The neutral entity model for 2D CAD drawings: what a drawing *is* once it has been taken out of its file format.

Every entity carries a **reference ID**, a **provenance** and a **confidence** — so that anything built on the model can say where a value came from and how far it can be trusted, including "unknown".

This crate is pure data: types and serialization. It parses nothing, renders nothing, and has no native dependencies.

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

Pre-implementation. No code yet. The contract is documented in [docs/principles.md](docs/principles.md); read that before proposing anything.

## License

MIT
