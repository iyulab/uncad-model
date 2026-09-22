//! The tables an entity resolves its references against: LAYER, BLOCK
//! (block definitions) and MLINESTYLE.
//!
//! The three maps are `BTreeMap`s, not hash maps, so iteration -- and
//! therefore JSON key order -- is deterministic: the same drawing serializes
//! to the same bytes on every run and every machine.

use crate::model::Entity;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One LAYER table entry (DXF `LAYER`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerRecord {
    /// Layer name (DXF 2).
    pub name: String,
    /// The layer's own AutoCAD Color Index (DXF 62), with the same raw
    /// semantics as [`crate::model::EntityCommon::color_index`]: negative
    /// means "off", otherwise a palette index. It is never 0 or 256 in a
    /// well-formed table (BYBLOCK/BYLAYER are entity-level values a layer
    /// cannot resolve against itself); a parser that meets such a value
    /// recovers or reports it before it reaches here.
    pub color_index: i16,
}

/// One block definition (DXF `BLOCK` ... `ENDBLK`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockRecord {
    /// Block name (DXF 2).
    pub name: String,
    /// Every entity the block definition owns, whether the block is
    /// `*Model_Space`/`*Paper_Space*` or a named definition an INSERT refers
    /// to -- unlike [`crate::CadDatabase::entities`], which holds only the
    /// former. This is what an INSERT's `block_name` resolves against to
    /// find what it draws.
    pub entities: Vec<Entity>,
}

/// The tables of a drawing.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Tables {
    /// Layer name -> record. A layer's true-color field (DXF 420) is
    /// deliberately not carried: only `color_index` is trustworthy for
    /// BYLAYER resolution.
    pub layers: BTreeMap<String, LayerRecord>,
    /// Block name -> record, every block in the file (including
    /// `*Model_Space`/`*Paper_Space*`, which also show up flattened into
    /// [`crate::CadDatabase::entities`] -- see that field's documentation).
    pub block_records: BTreeMap<String, BlockRecord>,
    /// MLINESTYLE name -> each parallel line's offset from the MLINE
    /// centerline (DXF 49), in the style's own element order. There is no
    /// separate line-identity field, so array order is the only
    /// correspondence between a style's lines and an MLINE's vertices.
    /// Per-line color and linetype are not carried.
    pub mlinestyles: BTreeMap<String, Vec<f64>>,
}
