//! The tables an entity resolves its references against: LAYER, BLOCK
//! (block definitions) and MLINESTYLE.
//!
//! The three maps are `BTreeMap`s, not hash maps, so iteration -- and
//! therefore JSON key order -- is deterministic: the same drawing serializes
//! to the same bytes on every run and every machine.

use crate::model::{absent, Entity, Ref};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// One LAYER table entry (DXF `LAYER`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerRecord {
    /// Layer name (DXF 2).
    pub name: String,
    /// The layer's own AutoCAD Color Index (DXF 62), with the same raw
    /// semantics as [`crate::model::EntityCommon::color_index`]: negative
    /// means "off" (how a DXF says so -- see [`Self::off`]), otherwise a
    /// palette index. It is never 0 or 256 in a well-formed table
    /// (BYBLOCK/BYLAYER are entity-level values a layer cannot resolve
    /// against itself); a parser that meets such a value recovers or
    /// reports it before it reaches here.
    pub color_index: i16,
    /// Whether the layer is switched off: its entities stay in the drawing
    /// and are not shown. A DXF says so with a negative `color_index`; the
    /// binary format has a flag of its own and keeps the colour positive,
    /// so this field, not the sign, is what says it for every file. A
    /// document written before this field existed reads as `false`.
    #[serde(default)]
    pub off: bool,
    /// DXF 70, bit 1: the layer is frozen -- its entities are not shown and
    /// not regenerated. `false` for a document written before this field
    /// existed.
    #[serde(default)]
    pub frozen: bool,
    /// DXF 70, bit 4: the layer is locked -- its entities are shown but
    /// cannot be edited. `false` for a document written before this field
    /// existed.
    #[serde(default)]
    pub locked: bool,
    /// DXF 290: whether the layer is plotted. `None` when the file does not
    /// state it -- R13 and R14 have no such flag -- or when the reader
    /// cannot tell a stated "do not plot" from an absent group.
    pub plot: Option<bool>,
    /// DXF 370: the layer's lineweight in hundredths of a millimetre, or
    /// -3 for the application's default weight (the format's other codes,
    /// -1 and -2, are entity values a layer cannot take). `None` when the
    /// file does not state one -- R13 and R14 have no lineweights -- or
    /// when the reader cannot tell a stated 0 from an absent group.
    pub lineweight: Option<i16>,
    /// DXF 6: the layer's linetype, an entry of the drawing's LTYPE table
    /// (which the model does not carry), as a reference like
    /// [`crate::model::EntityCommon::layer`]. `Absent` for a document
    /// written before this field existed.
    #[serde(default = "absent")]
    pub linetype: Ref<String>,
}

/// One DIMSTYLE table entry (DXF `DIMSTYLE`): the settings a dimension
/// names rather than carries.
///
/// Every field is `Option` because the format writes a style variable only
/// when it differs from the value the application starts from. An absent
/// variable is "this style does not state it", not a value -- what to use
/// instead is the consumer's decision, the same way assembling the text a
/// dimension displays is. Writing a plausible number here would make a
/// style claim something its file never said.
///
/// The set is the one the displayed text depends on. Everything else the
/// table carries is about how the dimension is drawn, which the drawn block
/// already settles.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DimStyleRecord {
    /// Style name (DXF 2).
    pub name: String,
    /// DXF 3 (`DIMPOST`): the pattern the measurement is placed into, with
    /// `<>` standing for the measurement -- `"<>mm"`, `"[]"`. Carried
    /// verbatim.
    pub post: Option<String>,
    /// DXF 40 (`DIMSCALE`): the overall scale applied to the dimension's
    /// drawn sizes.
    pub scale: Option<f64>,
    /// DXF 144 (`DIMLFAC`): the factor the measurement is multiplied by
    /// before it is displayed. A drawing measured in one unit and dimensioned
    /// in another states it here.
    pub length_factor: Option<f64>,
    /// DXF 71 (`DIMTOL`): whether tolerances are appended to the text.
    pub tolerances: Option<bool>,
    /// DXF 72 (`DIMLIM`): whether the text is the two limits rather than the
    /// measurement with tolerances.
    pub limits: Option<bool>,
    /// DXF 47 (`DIMTP`): the upper tolerance.
    pub tolerance_upper: Option<f64>,
    /// DXF 48 (`DIMTM`): the lower tolerance.
    pub tolerance_lower: Option<f64>,
    /// DXF 271 (`DIMDEC`): decimal places in the measurement.
    pub decimal_places: Option<i32>,
    /// DXF 272 (`DIMTDEC`): decimal places in the tolerances.
    pub tolerance_decimal_places: Option<i32>,
    /// DXF 140 (`DIMTXT`): the text height.
    pub text_height: Option<f64>,
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
    /// DIMSTYLE name -> record. A dimension names its style (DXF 3) rather
    /// than carrying these values, so reading what a dimension displays
    /// means reading this table too.
    pub dim_styles: BTreeMap<String, DimStyleRecord>,
    /// MLINESTYLE name -> each parallel line's offset from the MLINE
    /// centerline (DXF 49), in the style's own element order. There is no
    /// separate line-identity field, so array order is the only
    /// correspondence between a style's lines and an MLINE's vertices.
    /// Per-line color and linetype are not carried.
    pub mlinestyles: BTreeMap<String, Vec<f64>>,
}
