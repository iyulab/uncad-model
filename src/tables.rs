//! The tables an entity resolves its references against -- LAYER, BLOCK
//! (block definitions), DIMSTYLE and MLINESTYLE -- and the drawing's
//! layouts, which say what each of its tabs shows.
//!
//! The maps are `BTreeMap`s, not hash maps, so iteration -- and therefore
//! JSON key order -- is deterministic: the same drawing serializes to the
//! same bytes on every run and every machine.

use crate::model::{absent, Entity, Point2D, Ref};
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
/// The format writes a style variable only when it differs from the value
/// the application starts from. Where that starting value is the same
/// whichever template a drawing began from, an unwritten variable *is*
/// that value, and a reader fills it in: a tolerance of 0, a factor or
/// scale of 1, a switch that is off, an empty pattern, the decimal and
/// decimal-degree unit formats, a horizontal fraction, no rounding, no
/// decimal places in an angle. Each such field says so below.
///
/// Where the templates start differently -- the text height, the arrow
/// size, the decimal places of the measurement and its tolerances, which
/// zeros are suppressed -- an unwritten variable is "this style does not
/// state it", not a value, and the field is `None`: what to use instead is
/// the consumer's decision, the same way assembling the text a dimension
/// displays is. Writing a plausible number there would make a style claim
/// something its file never said.
///
/// Every field stays `Option` either way: `None` is also what a reader
/// reports when it cannot read the variable at all, and what a file from
/// before the version that introduced a variable says about it.
///
/// The set is the one the displayed text depends on, and the two sizes a
/// dimension of the style is drawn at -- its text height and its arrow
/// size. Everything else the table carries is about how the dimension is
/// drawn, which the drawn block already settles.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct DimStyleRecord {
    /// Style name (DXF 2).
    pub name: String,
    /// DXF 3 (`DIMPOST`): the pattern the measurement is placed into, with
    /// `<>` standing for the measurement -- `"<>mm"`, `"[]"`. Carried
    /// verbatim. Unwritten: empty.
    pub post: Option<String>,
    /// DXF 40 (`DIMSCALE`): the overall scale applied to the dimension's
    /// drawn sizes. Unwritten: 1.
    pub scale: Option<f64>,
    /// DXF 144 (`DIMLFAC`): the factor the measurement is multiplied by
    /// before it is displayed. A drawing measured in one unit and dimensioned
    /// in another states it here. Unwritten: 1.
    pub length_factor: Option<f64>,
    /// DXF 71 (`DIMTOL`): whether tolerances are appended to the text.
    /// Unwritten: no.
    pub tolerances: Option<bool>,
    /// DXF 72 (`DIMLIM`): whether the text is the two limits rather than the
    /// measurement with tolerances. Unwritten: no.
    pub limits: Option<bool>,
    /// DXF 47 (`DIMTP`): the upper tolerance. Unwritten: 0.
    pub tolerance_upper: Option<f64>,
    /// DXF 48 (`DIMTM`): the lower tolerance. Unwritten: 0.
    pub tolerance_lower: Option<f64>,
    /// DXF 271 (`DIMDEC`): decimal places in the measurement.
    pub decimal_places: Option<i32>,
    /// DXF 272 (`DIMTDEC`): decimal places in the tolerances.
    pub tolerance_decimal_places: Option<i32>,
    /// DXF 140 (`DIMTXT`): the text height.
    pub text_height: Option<f64>,
    /// DXF 41 (`DIMASZ`): the size of the arrowheads.
    pub arrow_size: Option<f64>,
    /// DXF 277 (`DIMLUNIT`): how a linear measurement is written. `None`
    /// also when the style states a value outside the format's six.
    /// Unwritten: decimal, in a file from R2000 on; the variable came
    /// with R2000, so an earlier file does not state it.
    pub linear_unit_format: Option<LinearUnitFormat>,
    /// DXF 78 (`DIMZIN`): which zeros are left out of a linear measurement,
    /// as the format encodes it -- 0 to 3 say how zero feet and zero inches
    /// are treated, and 4 (leading zeros) and 8 (trailing zeros) are added
    /// to them.
    pub zero_suppression: Option<i32>,
    /// DXF 45 (`DIMRND`): the step a linear measurement is rounded to; 0 is
    /// no rounding. Unwritten: 0.
    pub rounding: Option<f64>,
    /// DXF 275 (`DIMAUNIT`): how an angle is written. `None` also when the
    /// style states a value outside the format's five. Unwritten: decimal
    /// degrees.
    pub angular_unit_format: Option<AngularUnitFormat>,
    /// DXF 179 (`DIMADEC`): decimal places in an angle. Unwritten: 0, in a
    /// file from R2000 on (as for the linear unit format).
    pub angular_decimal_places: Option<i32>,
    /// DXF 276 (`DIMFRAC`): how a fraction is written, where the linear
    /// format writes fractions. `None` also when the style states a value
    /// outside the format's three. Unwritten: horizontal, in a file from
    /// R2000 on (as for the linear unit format).
    pub fraction_format: Option<FractionFormat>,
}

/// How a dimension style writes a linear measurement (`DIMLUNIT`, DXF 277,
/// 1 to 6 in this order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LinearUnitFormat {
    /// `1.55E+01`.
    Scientific,
    /// `15.50`.
    Decimal,
    /// Feet and decimal inches: `1'-3.50"`.
    Engineering,
    /// Feet and fractional inches: `1'-3 1/2"`.
    Architectural,
    /// `15 1/2`.
    Fractional,
    /// The operating system's own decimal format.
    WindowsDesktop,
}

/// How a dimension style writes an angle (`DIMAUNIT`, DXF 275, 0 to 4 in
/// this order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AngularUnitFormat {
    /// `45.5` degrees.
    DecimalDegrees,
    /// `45d30'0"`.
    DegreesMinutesSeconds,
    /// `50.5556g`.
    Gradians,
    /// `0.7941r`.
    Radians,
    /// A bearing: `N 44d30' E`.
    SurveyorsUnits,
}

/// How a dimension style writes a fraction (`DIMFRAC`, DXF 276, 0 to 2 in
/// this order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum FractionFormat {
    /// Numerator over denominator, with a horizontal bar.
    Horizontal,
    /// Numerator over denominator, with a diagonal bar.
    Diagonal,
    /// On one line: `1/2`.
    NotStacked,
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

/// One LAYOUT object: a tab of the drawing -- the model tab, or a sheet of
/// paper space -- the block it shows, and the paper it is set up to be
/// printed on.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayoutRecord {
    /// DXF 1: the layout's name, as its tab shows it (`Model`, `Layout1`).
    pub name: String,
    /// DXF 71: the tab's place in the order of tabs; the model tab's is 0.
    pub tab_order: i32,
    /// DXF 330 of the layout's own part: the block record whose entities
    /// the layout shows -- `*Model_Space` for the model tab, a
    /// `*Paper_Space` block for a sheet -- which, resolved, is a key of
    /// [`Tables::block_records`].
    pub block_name: Ref<String>,
    /// DXF 10: the lower-left corner of the layout's limits, in its own
    /// space (paper space for a sheet).
    pub limits_min: Point2D,
    /// DXF 11: the upper-right corner of the layout's limits.
    pub limits_max: Point2D,
    /// How the layout is set up to print.
    pub plot_settings: PlotSettings,
}

/// The plot settings a layout carries (the object's `AcDbPlotSettings`
/// part): the paper, and where the layout is put on it. The reference
/// states every distance here in millimetres, whatever
/// [`Self::paper_units`] says.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlotSettings {
    /// DXF 4: the paper size's name (`ISO_A4_(210.00_x_297.00_MM)`);
    /// empty when none is set.
    pub paper_name: String,
    /// DXF 44: the paper's width as its size defines it, before
    /// [`Self::rotation`]; 0 when no paper is set.
    pub paper_width: f64,
    /// DXF 45: the paper's height, likewise.
    pub paper_height: f64,
    /// DXF 40: the unprintable margin at the paper's left edge.
    pub margin_left: f64,
    /// DXF 41: the unprintable margin at the paper's bottom edge.
    pub margin_bottom: f64,
    /// DXF 42: the unprintable margin at the paper's right edge.
    pub margin_right: f64,
    /// DXF 43: the unprintable margin at the paper's top edge.
    pub margin_top: f64,
    /// DXF 46 and 47: the plot origin's offset.
    pub plot_origin: Point2D,
    /// DXF 72: the unit the layout is drawn in on paper. `None` when the
    /// file states a value outside the format's three.
    pub paper_units: Option<PlotPaperUnits>,
    /// DXF 73: how the layout is turned on the paper. `None` when the file
    /// states a value outside the format's four.
    pub rotation: Option<PlotRotation>,
    /// DXF 142: the paper-units side of the custom print scale.
    pub scale_numerator: f64,
    /// DXF 143: the drawing-units side of the custom print scale.
    pub scale_denominator: f64,
}

/// The unit a layout is drawn in on paper (DXF 72, 0 to 2 in this order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlotPaperUnits {
    /// 0: inches.
    Inches,
    /// 1: millimetres.
    Millimeters,
    /// 2: pixels, for a raster device.
    Pixels,
}

/// How a layout is turned on its paper (DXF 73, 0 to 3 in this order).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PlotRotation {
    /// 0: not turned.
    Unrotated,
    /// 1: a quarter turn counter-clockwise.
    Counterclockwise90,
    /// 2: upside down.
    UpsideDown,
    /// 3: a quarter turn clockwise.
    Clockwise90,
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
    /// Layout name -> record: every LAYOUT object, the model tab's
    /// included. Empty for a file that has none -- a DXF without an OBJECTS
    /// section, or a drawing older than R2000 that no application with
    /// layouts saved (R13 and R14 have none of their own; a later AutoCAD
    /// saving to R14 keeps them) -- whose paper space blocks are still in
    /// [`Self::block_records`], and for a document written before this
    /// field existed.
    #[serde(default)]
    pub layouts: BTreeMap<String, LayoutRecord>,
}
