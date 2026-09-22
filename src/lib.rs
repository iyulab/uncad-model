//! The neutral entity model for 2D CAD drawings: what a drawing *is* once it
//! has been taken out of its file format.
//!
//! Pure data. This crate parses nothing, renders nothing, and has no native
//! dependencies: a parser fills a [`CadDatabase`], and a summarizer, an
//! editor, a differ or a renderer reads it. What it offers beyond the data
//! is serialization -- [`CadDatabase::to_json`] and the `serde` derives on
//! every type, the public contract (see [`json`]) -- and the arithmetic
//! every consumer of a block reference needs alike: the placement an INSERT
//! applies to its block ([`Affine2`]), so that nested references compose
//! the same way for all of them.
//!
//! The rules the model follows are in `docs/principles.md` alongside this
//! crate; the shape of each type follows the DXF reference.

#![forbid(unsafe_code)]

pub mod color;
pub mod json;
pub mod model;
pub mod tables;
pub mod transform;

use serde::{Deserialize, Serialize};

pub use json::{JsonError, ToJsonOptions};
pub use model::{Confidence, Entity, EntityCommon, EntityId, Origin, Point2D, Point3D, Ref};
pub use tables::Tables;
pub use transform::Affine2;

/// A drawing: the model, and nothing else.
///
/// `entities` holds what the drawing shows (everything owned by the
/// `*Model_Space`/`*Paper_Space*` blocks, see [`model`]) and `tables` the
/// LAYER / BLOCK / MLINESTYLE tables it resolves against. This is what
/// [`to_json`](Self::to_json) serializes verbatim.
///
/// It is a plain Rust value: `Clone`/`PartialEq`/`Send`/`Sync` without
/// ceremony, constructible directly or deserialized from the JSON `to_json`
/// produced. It is deliberately *not* a round-trip representation of a file
/// (no linetypes, lineweights, styles, dictionaries, header variables): it
/// keeps what consumers of the drawing's content need.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CadDatabase {
    pub entities: Vec<Entity>,
    pub tables: Tables,
    /// What the reader reported while reading but did not fail on. Defaults
    /// to "nothing reported" when absent from JSON written before this field
    /// existed.
    #[serde(default)]
    pub read_diagnostics: ReadDiagnostics,
}

impl CadDatabase {
    /// Every entity the drawing holds, wherever it is owned: the top level
    /// first, then each block record's entities in block-name order. What
    /// model space owns is listed both at the top level and under its
    /// block record (see [`tables::BlockRecord`]), so a consumer keyed by
    /// reference ID meets those IDs twice.
    pub fn all_entities(&self) -> impl Iterator<Item = &Entity> {
        self.entities.iter().chain(
            self.tables
                .block_records
                .values()
                .flat_map(|b| b.entities.iter()),
        )
    }

    /// [`all_entities`](Self::all_entities), mutably.
    pub fn all_entities_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.entities.iter_mut().chain(
            self.tables
                .block_records
                .values_mut()
                .flat_map(|b| b.entities.iter_mut()),
        )
    }
}

/// Non-fatal problems the reader reported while producing the model.
///
/// "Read with warnings" and "read cleanly" are different outcomes: a file
/// that came back with a warning may be missing objects the reader did not
/// know how to decode, and nothing else in the model says so. The names are
/// the reader's own (a parser documents its vocabulary), listed in a stable
/// order so the same read always reports the same way.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ReadDiagnostics {
    /// The reader's warning names, in the reader's stable order; empty when
    /// the read was clean.
    pub warnings: Vec<String>,
}

impl ReadDiagnostics {
    /// `true` when the reader reported nothing at all.
    pub fn is_clean(&self) -> bool {
        self.warnings.is_empty()
    }
}
