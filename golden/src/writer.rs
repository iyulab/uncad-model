//! Spec -> DXF text (R2000, ASCII DXF).
//!
//! Independent of every parser by construction: the output is built from the
//! DXF reference's group codes alone, as (code, value) pairs. Handles are
//! issued deterministically, in the order things are written, so the same
//! spec always produces the same bytes -- and the same handles the oracle
//! expects to see on the entities that come back.
//!
//! The output is bytes, not a `String`: an R2000 DXF stores text in the
//! drawing's codepage ([`crate::spec::Codepage`]), so a case with non-ASCII
//! text is not UTF-8 on disk.

use crate::spec::{
    AttribSpec, BlockSpec, Codepage, DimStyleSpec, EntitySpec, LayerSpec, LayerState, LayoutSpec,
    Spec, Xy,
};
use uncad_model::model::{HorizontalJustification, OrdinateAxis, VerticalJustification};
use uncad_model::tables::{
    AngularUnitFormat, FractionFormat, LinearUnitFormat, PlotPaperUnits, PlotRotation,
};

/// First handle issued to an entity. Table entries and block records come
/// before it, so that a reader listing entities by handle sees them in file
/// order.
const FIRST_ENTITY_HANDLE: u32 = 0x100;

/// Bit 0x20000 of a VIEWPORT's status flags (DXF 90): the viewport is off.
const VIEWPORT_OFF: u32 = 0x2_0000;

/// The status flags (DXF 90) the writer gives every viewport besides the
/// off bit: the ones an application sets on a plain paper-space viewport.
const VIEWPORT_FLAGS: u32 = 32864;

/// The handles a written drawing carries, so an oracle can name them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Handles {
    /// Handle of each top-level entity, in file order. A dimension's
    /// anonymous block does not appear here; its entities are in `blocks`.
    pub entities: Vec<u32>,
    /// Handle of each paper-space entity ([`Spec::paper_space`]), in file
    /// order.
    pub paper_entities: Vec<u32>,
    /// Per block definition (named blocks first, in spec order, then the
    /// dimension blocks `*D1`, `*D2`, ...): the handles of its entities.
    pub blocks: Vec<(String, Vec<u32>)>,
    /// Attribute handles per top-level INSERT that has any, keyed by the
    /// INSERT's own handle.
    pub attribs: Vec<(u32, Vec<u32>)>,
}

/// A written drawing: the DXF bytes plus the handles it issued.
#[derive(Debug, Clone, PartialEq)]
pub struct Written {
    /// The file's bytes: 7-bit ASCII for [`Codepage::Ascii`], otherwise the
    /// spec's strings encoded in the declared codepage.
    pub dxf: Vec<u8>,
    pub handles: Handles,
}

/// Writes `spec` as an R2000 ASCII DXF.
///
/// # Panics
/// When a string cannot be encoded in the spec's codepage: a non-ASCII
/// string under [`Codepage::Ascii`], or a character the codepage has no
/// byte sequence for. A case that cannot be written faithfully is not an
/// oracle for anything. Likewise when a spec asks for something the format
/// cannot say: a mirrored entity of a kind that has no object coordinate
/// system, or a viewport frozen on a layer the spec does not declare.
pub fn write(spec: &Spec) -> Written {
    let mut w = Writer {
        codepage: spec.codepage,
        ..Writer::default()
    };
    w.write(spec);
    Written {
        dxf: w.out,
        handles: w.handles,
    }
}

#[derive(Default)]
struct Writer {
    out: Vec<u8>,
    codepage: Codepage,
    next_handle: u32,
    handles: Handles,
    /// Anonymous dimension blocks collected while writing entities; emitted
    /// in the BLOCKS section, which is written after the entities have been
    /// planned.
    dim_blocks: Vec<(String, Vec<EntitySpec>)>,
    /// Block name -> the handle of its BLOCK_RECORD, which owns the block's
    /// entities (DXF 330 on each of them).
    block_records: Vec<(String, u32)>,
    /// Layer name -> the handle of its LAYER entry, which a viewport's
    /// frozen layers point at (DXF 341).
    layer_handles: Vec<(String, u32)>,
    /// Set while writing paper-space entities, which carry DXF 67.
    paper_space: bool,
    /// Set while writing the entity inside [`EntitySpec::Mirrored`]: its
    /// elevation, the z of its points in its (mirrored) OCS.
    mirrored: Option<f64>,
}

impl Writer {
    fn pair(&mut self, code: u16, value: impl std::fmt::Display) {
        // The DXF reference right-aligns the code in a three-character
        // field; the value follows on its own line.
        self.out
            .extend_from_slice(format!("{code:>3}\n").as_bytes());
        let encoded = self.encode(&value.to_string());
        self.out.extend_from_slice(&encoded);
        self.out.push(b'\n');
    }

    /// A string as the file stores it. Every value goes through here, so a
    /// layer name is encoded the same way as a text value.
    fn encode(&self, value: &str) -> Vec<u8> {
        match self.codepage {
            Codepage::Ascii => {
                assert!(
                    value.is_ascii(),
                    "{value:?} is not ASCII; give the spec a codepage that can encode it"
                );
                value.as_bytes().to_vec()
            }
            Codepage::Ansi949 => {
                // WHATWG's euc-kr is the CP949 superset (Unified Hangul
                // Code), which is what `ANSI_949` names.
                let (bytes, _, had_errors) = encoding_rs::EUC_KR.encode(value);
                assert!(!had_errors, "{value:?} has a character CP949 cannot encode");
                bytes.into_owned()
            }
        }
    }

    fn num(&mut self, code: u16, value: f64) {
        self.pair(code, format!("{value:?}"));
    }

    fn xy(&mut self, base: u16, p: Xy) {
        self.xyz(base, p, 0.0);
    }

    fn xyz(&mut self, base: u16, p: Xy, z: f64) {
        self.num(base, p.x);
        self.num(base + 10, p.y);
        self.num(base + 20, z);
    }

    /// The z of an OCS entity's points: its elevation when it is written
    /// mirrored, 0 otherwise.
    fn elevation(&self) -> f64 {
        self.mirrored.unwrap_or(0.0)
    }

    /// DXF 210/220/230 for an OCS entity written mirrored; nothing for one
    /// in the world's own axes, the reference's default.
    fn normal(&mut self) {
        if self.mirrored.is_some() {
            self.num(210, 0.0);
            self.num(220, 0.0);
            self.num(230, -1.0);
        }
    }

    fn handle(&mut self) -> u32 {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }

    fn write(&mut self, spec: &Spec) {
        // Plan the anonymous dimension blocks first: their names must be
        // known when the tables are written.
        let mut dim_count = 0;
        for e in spec.entities.iter().chain(&spec.paper_space) {
            if let Some((name, entities)) = dimension_block(e, &mut dim_count) {
                self.dim_blocks.push((name, entities));
            }
        }

        // Handles: tables and block records first (from 1), entities later
        // (from FIRST_ENTITY_HANDLE).
        self.next_handle = 1;

        self.pair(0, "SECTION");
        self.pair(2, "HEADER");
        self.pair(9, "$ACADVER");
        self.pair(1, "AC1015");
        if let Some(name) = self.codepage.dxf_name() {
            self.pair(9, "$DWGCODEPAGE");
            self.pair(3, name);
        }
        // Rewritten at the end: DXF wants the seed above every handle used.
        let seed_at = self.out.len();
        self.pair(9, "$HANDSEED");
        self.pair(5, "FFFF");
        self.pair(0, "ENDSEC");

        self.tables(spec);
        self.next_handle = FIRST_ENTITY_HANDLE;
        self.blocks(spec);
        self.entities(spec);
        if !spec.layouts.is_empty() {
            self.objects(spec);
        }
        self.pair(0, "EOF");

        let seed = format!("{:X}", self.next_handle);
        let placeholder = b"  9\n$HANDSEED\n  5\nFFFF\n";
        let fixed = format!("  9\n$HANDSEED\n  5\n{seed}\n");
        debug_assert!(self.out[seed_at..].starts_with(placeholder));
        self.out
            .splice(seed_at..seed_at + placeholder.len(), fixed.bytes());
    }

    fn tables(&mut self, spec: &Spec) {
        self.pair(0, "SECTION");
        self.pair(2, "TABLES");

        self.pair(0, "TABLE");
        self.pair(2, "LTYPE");
        let h = self.handle();
        self.pair(5, format!("{h:X}"));
        self.pair(100, "AcDbSymbolTable");
        self.pair(70, 1);
        self.pair(0, "LTYPE");
        let table = h;
        let h = self.handle();
        self.pair(5, format!("{h:X}"));
        self.pair(330, format!("{table:X}"));
        self.pair(100, "AcDbSymbolTableRecord");
        self.pair(100, "AcDbLinetypeTableRecord");
        self.pair(2, "CONTINUOUS");
        self.pair(70, 0);
        self.pair(3, "Solid line");
        self.pair(72, 65);
        self.pair(73, 0);
        self.num(40, 0.0);
        self.pair(0, "ENDTAB");

        self.pair(0, "TABLE");
        self.pair(2, "LAYER");
        let h = self.handle();
        self.pair(5, format!("{h:X}"));
        self.pair(100, "AcDbSymbolTable");
        self.pair(70, spec.layers.len() + 1);
        let table = h;
        self.layer(
            &LayerSpec {
                name: "0".to_string(),
                color_index: 7,
                state: LayerState::default(),
            },
            table,
        );
        for l in &spec.layers {
            self.layer(l, table);
        }
        self.pair(0, "ENDTAB");

        if !spec.text_styles.is_empty() {
            self.pair(0, "TABLE");
            self.pair(2, "STYLE");
            let h = self.handle();
            self.pair(5, format!("{h:X}"));
            self.pair(100, "AcDbSymbolTable");
            self.pair(70, spec.text_styles.len());
            let table = h;
            for name in &spec.text_styles {
                self.pair(0, "STYLE");
                let h = self.handle();
                self.pair(5, format!("{h:X}"));
                self.pair(330, format!("{table:X}"));
                self.pair(100, "AcDbSymbolTableRecord");
                self.pair(100, "AcDbTextStyleTableRecord");
                self.pair(2, name);
                self.pair(70, 0);
                self.num(40, 0.0);
                self.num(41, 1.0);
                self.num(50, 0.0);
                self.pair(71, 0);
                self.num(42, 2.5);
                self.pair(3, "txt");
                self.pair(4, "");
            }
            self.pair(0, "ENDTAB");
        }

        if !spec.dim_styles.is_empty() {
            self.pair(0, "TABLE");
            self.pair(2, "DIMSTYLE");
            let h = self.handle();
            self.pair(5, format!("{h:X}"));
            self.pair(100, "AcDbSymbolTable");
            self.pair(70, spec.dim_styles.len());
            let table = h;
            for style in &spec.dim_styles {
                self.dim_style(style, table);
            }
            self.pair(0, "ENDTAB");
        }

        self.pair(0, "TABLE");
        self.pair(2, "BLOCK_RECORD");
        let h = self.handle();
        self.pair(5, format!("{h:X}"));
        self.pair(100, "AcDbSymbolTable");
        let names = self.block_names(spec);
        self.pair(70, names.len());
        let table = h;
        for name in names {
            self.pair(0, "BLOCK_RECORD");
            let h = self.handle();
            self.pair(5, format!("{h:X}"));
            self.pair(330, format!("{table:X}"));
            self.pair(100, "AcDbSymbolTableRecord");
            self.pair(100, "AcDbBlockTableRecord");
            self.pair(2, &name);
            self.block_records.push((name, h));
        }
        self.pair(0, "ENDTAB");

        self.pair(0, "ENDSEC");
    }

    fn layer(&mut self, l: &LayerSpec, table: u32) {
        self.pair(0, "LAYER");
        let h = self.handle();
        self.layer_handles.push((l.name.clone(), h));
        self.pair(5, format!("{h:X}"));
        self.pair(330, format!("{table:X}"));
        self.pair(100, "AcDbSymbolTableRecord");
        self.pair(100, "AcDbLayerTableRecord");
        self.pair(2, &l.name);
        // The DXF layout of group 70: 1 frozen, 4 locked.
        self.pair(
            70,
            u16::from(l.state.frozen) | u16::from(l.state.locked) << 2,
        );
        // A DXF says "off" with the sign of the colour.
        self.pair(
            62,
            if l.state.off {
                -l.color_index
            } else {
                l.color_index
            },
        );
        self.pair(6, "CONTINUOUS");
        if let Some(plot) = l.state.plot {
            self.pair(290, u8::from(plot));
        }
        if let Some(weight) = l.state.lineweight {
            self.pair(370, weight);
        }
    }

    fn dim_style(&mut self, style: &DimStyleSpec, table: u32) {
        self.pair(0, "DIMSTYLE");
        let h = self.handle();
        // A DIMSTYLE entry's own handle is group 105, not 5: the
        // format's one exception, because 5 was already taken.
        self.pair(105, format!("{h:X}"));
        self.pair(330, format!("{table:X}"));
        self.pair(100, "AcDbSymbolTableRecord");
        self.pair(100, "AcDbDimStyleTableRecord");
        self.pair(2, &style.name);
        self.pair(70, 0);
        if let Some(post) = &style.post {
            self.pair(3, post);
        }
        if let Some(size) = style.arrow_size {
            self.num(41, size);
        }
        if let Some(step) = style.rounding {
            self.num(45, step);
        }
        if let Some(zin) = style.zero_suppression {
            self.pair(78, zin);
        }
        if let Some(height) = style.text_height {
            self.num(140, height);
        }
        if let Some(places) = style.angular_decimal_places {
            self.pair(179, places);
        }
        if let Some(places) = style.decimal_places {
            self.pair(271, places);
        }
        if let Some(format) = style.angular_unit_format {
            self.pair(275, angular_unit_code(format));
        }
        if let Some(format) = style.fraction_format {
            self.pair(276, fraction_code(format));
        }
        if let Some(format) = style.linear_unit_format {
            self.pair(277, linear_unit_code(format));
        }
    }

    /// Every block the file declares, in the order its records are
    /// written: the spaces (see [`space_names`]), the named blocks, then the
    /// dimension blocks.
    fn block_names(&self, spec: &Spec) -> Vec<String> {
        let mut names = space_names(spec);
        names.extend(spec.blocks.iter().map(|b| b.name.clone()));
        names.extend(self.dim_blocks.iter().map(|(n, _)| n.clone()));
        names
    }

    fn blocks(&mut self, spec: &Spec) {
        self.pair(0, "SECTION");
        self.pair(2, "BLOCKS");
        for name in &space_names(spec) {
            let owner = self.block_record(name);
            self.block_begin(name, 0, owner);
            self.block_end(owner);
        }
        let named: Vec<BlockSpec> = spec.blocks.clone();
        for b in &named {
            let owner = self.block_record(&b.name);
            self.block_begin(&b.name, 0, owner);
            let mut handles = Vec::new();
            for e in &b.entities {
                let h = self.entity(e, None, owner);
                handles.push(h);
            }
            self.block_end(owner);
            self.handles.blocks.push((b.name.clone(), handles));
        }
        let dims = std::mem::take(&mut self.dim_blocks);
        for (name, entities) in &dims {
            let owner = self.block_record(name);
            // Bit 0 of the block flags marks an anonymous block.
            self.block_begin(name, 1, owner);
            let mut handles = Vec::new();
            for e in entities {
                handles.push(self.entity(e, None, owner));
            }
            self.block_end(owner);
            self.handles.blocks.push((name.clone(), handles));
        }
        self.dim_blocks = dims;
        self.pair(0, "ENDSEC");
    }

    /// The BLOCK_RECORD handle of a block written in the tables.
    fn block_record(&self, name: &str) -> u32 {
        self.block_records
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, h)| *h)
            .expect("every block has a record")
    }

    /// The LAYER handle of a layer written in the tables.
    fn layer_handle(&self, name: &str) -> u32 {
        self.layer_handles
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, h)| *h)
            .unwrap_or_else(|| panic!("layer {name:?} is not declared"))
    }

    fn block_begin(&mut self, name: &str, flags: u16, owner: u32) {
        self.pair(0, "BLOCK");
        let h = self.handle();
        self.pair(5, format!("{h:X}"));
        self.pair(330, format!("{owner:X}"));
        self.pair(100, "AcDbEntity");
        self.pair(8, "0");
        self.pair(100, "AcDbBlockBegin");
        self.pair(2, name);
        self.pair(70, flags);
        self.xy(10, Xy::new(0.0, 0.0));
        self.pair(3, name);
        self.pair(1, "");
    }

    fn block_end(&mut self, owner: u32) {
        self.pair(0, "ENDBLK");
        let h = self.handle();
        self.pair(5, format!("{h:X}"));
        self.pair(330, format!("{owner:X}"));
        self.pair(100, "AcDbEntity");
        self.pair(8, "0");
        self.pair(100, "AcDbBlockEnd");
    }

    fn entities(&mut self, spec: &Spec) {
        self.pair(0, "SECTION");
        self.pair(2, "ENTITIES");
        let mut dim_index = 0;
        let mut dim_name = |e: &EntitySpec| {
            e.is_dimension().then(|| {
                dim_index += 1;
                format!("*D{dim_index}")
            })
        };
        let owner = self.block_record("*Model_Space");
        for e in &spec.entities {
            let name = dim_name(e);
            let h = self.entity(e, name.as_deref(), owner);
            self.handles.entities.push(h);
        }
        let owner = self.block_record("*Paper_Space");
        self.paper_space = true;
        for e in &spec.paper_space {
            let name = dim_name(e);
            let h = self.entity(e, name.as_deref(), owner);
            self.handles.paper_entities.push(h);
        }
        self.paper_space = false;
        self.pair(0, "ENDSEC");
    }

    /// Writes one entity and returns its handle. `dim_block` names the
    /// anonymous block a dimension refers to; `owner` is the BLOCK_RECORD
    /// that owns the entity (DXF 330).
    fn entity(&mut self, e: &EntitySpec, dim_block: Option<&str>, owner: u32) -> u32 {
        if let EntitySpec::Mirrored { elevation, entity } = e {
            assert!(
                matches!(
                    **entity,
                    EntitySpec::Circle { .. }
                        | EntitySpec::Arc { .. }
                        | EntitySpec::LwPolyline { .. }
                        | EntitySpec::Text { .. }
                        | EntitySpec::JustifiedText { .. }
                        | EntitySpec::Solid { .. }
                        | EntitySpec::Insert { .. }
                ),
                "{entity:?} has no object coordinate system to mirror"
            );
            self.mirrored = Some(*elevation);
            let h = self.entity(entity, dim_block, owner);
            self.mirrored = None;
            return h;
        }
        let h = self.handle();
        let hex = format!("{h:X}");
        let z = self.elevation();
        match e {
            EntitySpec::Line { layer, start, end } => {
                self.pair(0, "LINE");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbLine");
                self.xy(10, *start);
                self.xy(11, *end);
            }
            EntitySpec::Circle {
                layer,
                center,
                radius,
            } => {
                self.pair(0, "CIRCLE");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbCircle");
                self.xyz(10, *center, z);
                self.num(40, *radius);
                self.normal();
            }
            EntitySpec::Arc {
                layer,
                center,
                radius,
                start_deg,
                end_deg,
            } => {
                self.pair(0, "ARC");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbCircle");
                self.xyz(10, *center, z);
                self.num(40, *radius);
                self.normal();
                self.pair(100, "AcDbArc");
                self.num(50, *start_deg);
                self.num(51, *end_deg);
            }
            EntitySpec::LwPolyline {
                layer,
                vertices,
                closed,
                bulges,
                widths,
                const_width,
            } => {
                assert!(bulges.is_empty() || bulges.len() == vertices.len());
                assert!(widths.is_empty() || widths.len() == vertices.len());
                self.pair(0, "LWPOLYLINE");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbPolyline");
                self.pair(90, vertices.len());
                self.pair(70, u16::from(*closed));
                if *const_width != 0.0 {
                    self.num(43, *const_width);
                }
                if z != 0.0 {
                    self.num(38, z);
                }
                for (i, v) in vertices.iter().enumerate() {
                    self.num(10, v.x);
                    self.num(20, v.y);
                    if let Some((start, end)) = widths.get(i) {
                        self.num(40, *start);
                        self.num(41, *end);
                    }
                    if let Some(bulge) = bulges.get(i) {
                        self.num(42, *bulge);
                    }
                }
                self.normal();
            }
            EntitySpec::Text {
                layer,
                insert,
                height,
                text,
                rotation_deg,
            } => {
                self.pair(0, "TEXT");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbText");
                self.xyz(10, *insert, z);
                self.num(40, *height);
                self.pair(1, text);
                self.num(50, *rotation_deg);
                self.normal();
                self.pair(100, "AcDbText");
            }
            EntitySpec::JustifiedText {
                layer,
                insert,
                alignment,
                horizontal,
                vertical,
                height,
                text,
                rotation_deg,
                width_factor,
                oblique_deg,
                style,
            } => {
                self.pair(0, "TEXT");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbText");
                self.xyz(10, *insert, z);
                self.num(40, *height);
                self.pair(1, text);
                self.num(50, *rotation_deg);
                if *width_factor != 1.0 {
                    self.num(41, *width_factor);
                }
                if *oblique_deg != 0.0 {
                    self.num(51, *oblique_deg);
                }
                if let Some(style) = style {
                    self.pair(7, style);
                }
                let justified = *horizontal != HorizontalJustification::Left
                    || *vertical != VerticalJustification::Baseline;
                if *horizontal != HorizontalJustification::Left {
                    self.pair(72, horizontal_code(*horizontal));
                }
                if justified {
                    self.xyz(11, *alignment, z);
                }
                self.normal();
                self.pair(100, "AcDbText");
                if *vertical != VerticalJustification::Baseline {
                    self.pair(73, vertical_code(*vertical));
                }
            }
            EntitySpec::Solid { layer, corners } => {
                self.pair(0, "SOLID");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbTrace");
                for (i, corner) in (0u16..).zip(corners) {
                    self.xyz(10 + i, *corner, z);
                }
                self.normal();
            }
            EntitySpec::Attdef {
                layer,
                insert,
                height,
                tag,
                prompt,
                default,
            } => {
                self.pair(0, "ATTDEF");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbText");
                self.xy(10, *insert);
                self.num(40, *height);
                self.pair(1, default);
                self.pair(100, "AcDbAttributeDefinition");
                self.pair(3, prompt);
                self.pair(2, tag);
                self.pair(70, 0);
            }
            EntitySpec::Insert {
                layer,
                block,
                insert,
                scale,
                rotation_deg,
                attribs,
            } => {
                self.pair(0, "INSERT");
                self.common(&hex, layer, owner);
                if !attribs.is_empty() {
                    self.pair(66, 1);
                }
                self.pair(100, "AcDbBlockReference");
                self.pair(2, block);
                self.xyz(10, *insert, z);
                self.num(41, *scale);
                self.num(42, *scale);
                self.num(43, *scale);
                self.num(50, *rotation_deg);
                self.normal();
                if !attribs.is_empty() {
                    // The attributes and the SEQEND are owned by the INSERT,
                    // and are in the world's own axes whatever the INSERT's.
                    let mirrored = self.mirrored.take();
                    let mut attrib_handles = Vec::new();
                    for a in attribs {
                        attrib_handles.push(self.attrib(a, layer, h));
                    }
                    self.mirrored = mirrored;
                    self.seqend(layer, h);
                    self.handles.attribs.push((h, attrib_handles));
                }
            }
            EntitySpec::PolygonMesh {
                layer,
                m,
                n,
                closed_m,
                closed_n,
                vertices,
            } => {
                assert_eq!(vertices.len(), usize::from(*m) * usize::from(*n));
                self.pair(0, "POLYLINE");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbPolygonMesh");
                self.pair(66, 1);
                self.xy(10, Xy::new(0.0, 0.0));
                // 16 = polygon mesh; 1 = closed in M; 32 = closed in N.
                self.pair(70, 16 | u16::from(*closed_m) | u16::from(*closed_n) << 5);
                self.pair(71, m);
                self.pair(72, n);
                for [x, y, vz] in vertices {
                    self.pair(0, "VERTEX");
                    let vh = self.handle();
                    self.common(&format!("{vh:X}"), layer, h);
                    self.pair(100, "AcDbVertex");
                    self.pair(100, "AcDbPolygonMeshVertex");
                    self.xyz(10, Xy::new(*x, *y), *vz);
                    // 64 = a polygon mesh vertex.
                    self.pair(70, 64);
                }
                self.seqend(layer, h);
            }
            EntitySpec::LinearDimension {
                layer,
                from,
                to,
                line_point,
                text,
                measurement,
                style,
            } => {
                self.pair(0, "DIMENSION");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbDimension");
                self.pair(2, dim_block.expect("a dimension has a block"));
                self.xy(10, *line_point);
                self.xy(11, midpoint(*from, *to));
                // 32 = block reference is set, 0 = rotated (linear).
                self.pair(70, 32);
                self.pair(1, text);
                if let Some(m) = measurement {
                    self.num(42, *m);
                }
                if let Some(style) = style {
                    self.pair(3, style);
                }
                self.pair(100, "AcDbAlignedDimension");
                self.xy(13, *from);
                self.xy(14, *to);
                self.pair(100, "AcDbRotatedDimension");
            }
            EntitySpec::ArcDimension {
                layer,
                from,
                to,
                center,
                line_point,
                text,
                measurement,
                style,
            } => {
                self.pair(0, "ARC_DIMENSION");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbDimension");
                self.pair(2, dim_block.expect("a dimension has a block"));
                self.xy(10, *line_point);
                self.xy(11, midpoint(*from, *to));
                // 32 = block reference, 5 = three-point angular -- which is
                // what the format writes on an arc-length dimension, and why
                // the entity's own name has to win.
                self.pair(70, 32 + 5);
                self.pair(1, text);
                if let Some(m) = measurement {
                    self.num(42, *m);
                }
                if let Some(style) = style {
                    self.pair(3, style);
                }
                self.pair(100, "AcDbArcDimension");
                self.xy(13, *from);
                self.xy(14, *to);
                self.xy(15, *center);
            }
            EntitySpec::DiameterDimension {
                layer,
                first,
                second,
                text,
                measurement,
                style,
            } => {
                self.pair(0, "DIMENSION");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbDimension");
                self.pair(2, dim_block.expect("a dimension has a block"));
                self.xy(10, *first);
                self.xy(11, midpoint(*first, *second));
                // 32 = block reference, 3 = diameter.
                self.pair(70, 32 + 3);
                self.pair(1, text);
                if let Some(m) = measurement {
                    self.num(42, *m);
                }
                if let Some(style) = style {
                    self.pair(3, style);
                }
                self.pair(100, "AcDbDiametricDimension");
                self.xy(15, *second);
                self.num(40, 0.0);
            }
            EntitySpec::OrdinateDimension {
                layer,
                datum,
                feature,
                leader_end,
                axis,
                text,
                measurement,
                style,
            } => {
                self.pair(0, "DIMENSION");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbDimension");
                self.pair(2, dim_block.expect("a dimension has a block"));
                self.xy(10, *datum);
                self.xy(11, *leader_end);
                // 32 = block reference, 6 = ordinate, 64 = X-type.
                let x_type = if *axis == OrdinateAxis::X { 64 } else { 0 };
                self.pair(70, 32 + 6 + x_type);
                self.pair(1, text);
                if let Some(m) = measurement {
                    self.num(42, *m);
                }
                if let Some(style) = style {
                    self.pair(3, style);
                }
                self.pair(100, "AcDbOrdinateDimension");
                self.xy(13, *feature);
                self.xy(14, *leader_end);
            }
            EntitySpec::Viewport {
                layer,
                center,
                width,
                height,
                on,
                id,
                view_center,
                view_height,
                view_target,
                twist_deg,
                frozen_layers,
            } => {
                self.pair(0, "VIEWPORT");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbViewport");
                self.xy(10, *center);
                self.num(40, *width);
                self.num(41, *height);
                // 68: 0 is off; a viewport that is on states its place in
                // the stack, which the writer takes from its number.
                self.pair(68, if *on { *id } else { 0 });
                self.pair(69, id);
                self.xy(12, *view_center);
                self.xy(13, Xy::new(0.0, 0.0));
                self.xy(14, Xy::new(10.0, 10.0));
                self.xy(15, Xy::new(10.0, 10.0));
                // A plan view: looking down the world z axis.
                self.num(16, 0.0);
                self.num(26, 0.0);
                self.num(36, 1.0);
                self.xy(17, *view_target);
                self.num(42, 50.0);
                self.num(43, 0.0);
                self.num(44, 0.0);
                self.num(45, *view_height);
                self.num(50, 0.0);
                self.num(51, *twist_deg);
                self.pair(72, 1000);
                for frozen in frozen_layers {
                    let lh = self.layer_handle(frozen);
                    self.pair(341, format!("{lh:X}"));
                }
                self.pair(90, VIEWPORT_FLAGS | if *on { 0 } else { VIEWPORT_OFF });
                self.pair(1, "");
                self.pair(281, 0);
                self.pair(71, 1);
                self.pair(74, 0);
                self.xy(110, Xy::new(0.0, 0.0));
                self.xy(111, Xy::new(1.0, 0.0));
                self.xy(112, Xy::new(0.0, 1.0));
                self.pair(79, 0);
                self.num(146, 0.0);
            }
            EntitySpec::Mirrored { .. } => unreachable!("handled above"),
        }
        h
    }

    fn attrib(&mut self, a: &AttribSpec, layer: &str, owner: u32) -> u32 {
        let h = self.handle();
        self.pair(0, "ATTRIB");
        self.common(&format!("{h:X}"), layer, owner);
        self.pair(100, "AcDbText");
        self.xy(10, a.insert);
        self.num(40, a.height);
        self.pair(1, &a.value);
        self.pair(100, "AcDbAttribute");
        self.pair(2, &a.tag);
        // Bit 1: the attribute is invisible.
        self.pair(70, u8::from(a.invisible));
        h
    }

    /// The SEQEND that closes an INSERT's attributes or a POLYLINE's
    /// vertices; the entity it closes owns it.
    fn seqend(&mut self, layer: &str, owner: u32) {
        self.pair(0, "SEQEND");
        let h = self.handle();
        self.pair(5, format!("{h:X}"));
        self.pair(330, format!("{owner:X}"));
        self.pair(100, "AcDbEntity");
        self.pair(8, layer);
    }

    fn common(&mut self, handle_hex: &str, layer: &str, owner: u32) {
        self.pair(5, handle_hex);
        self.pair(330, format!("{owner:X}"));
        self.pair(100, "AcDbEntity");
        if self.paper_space {
            self.pair(67, 1);
        }
        self.pair(8, layer);
    }

    /// The OBJECTS section: the root dictionary, its layout dictionary and
    /// one LAYOUT object per [`Spec::layouts`] entry.
    fn objects(&mut self, spec: &Spec) {
        self.pair(0, "SECTION");
        self.pair(2, "OBJECTS");
        let root = self.handle();
        let dictionary = self.handle();
        let layouts: Vec<u32> = spec.layouts.iter().map(|_| self.handle()).collect();

        self.pair(0, "DICTIONARY");
        self.pair(5, format!("{root:X}"));
        self.pair(330, 0);
        self.pair(100, "AcDbDictionary");
        self.pair(281, 1);
        self.pair(3, "ACAD_LAYOUT");
        self.pair(350, format!("{dictionary:X}"));

        self.pair(0, "DICTIONARY");
        self.pair(5, format!("{dictionary:X}"));
        self.pair(330, format!("{root:X}"));
        self.pair(100, "AcDbDictionary");
        self.pair(281, 1);
        for (layout, h) in spec.layouts.iter().zip(&layouts) {
            self.pair(3, &layout.name);
            self.pair(350, format!("{h:X}"));
        }

        for (layout, h) in spec.layouts.iter().zip(layouts) {
            self.layout(layout, h, dictionary);
        }
        self.pair(0, "ENDSEC");
    }

    fn layout(&mut self, layout: &LayoutSpec, h: u32, dictionary: u32) {
        self.pair(0, "LAYOUT");
        self.pair(5, format!("{h:X}"));
        self.pair(330, format!("{dictionary:X}"));
        self.pair(100, "AcDbPlotSettings");
        self.pair(1, "");
        self.pair(2, "none_device");
        self.pair(4, &layout.paper_name);
        self.pair(6, "");
        let [left, bottom, right, top] = layout.margins;
        self.num(40, left);
        self.num(41, bottom);
        self.num(42, right);
        self.num(43, top);
        self.num(44, layout.paper_size.0);
        self.num(45, layout.paper_size.1);
        self.num(46, layout.plot_origin.x);
        self.num(47, layout.plot_origin.y);
        self.num(48, 0.0);
        self.num(49, 0.0);
        self.num(140, 0.0);
        self.num(141, 0.0);
        self.num(142, layout.scale.0);
        self.num(143, layout.scale.1);
        self.pair(70, 688);
        self.pair(72, paper_units_code(layout.paper_units));
        self.pair(73, rotation_code(layout.rotation));
        // 5 = plot the layout itself.
        self.pair(74, 5);
        self.pair(7, "");
        // 16 = the 1:1 standard scale.
        self.pair(75, 16);
        self.num(147, 1.0);
        self.num(148, 0.0);
        self.num(149, 0.0);
        self.pair(100, "AcDbLayout");
        self.pair(1, &layout.name);
        self.pair(70, 1);
        self.pair(71, layout.tab_order);
        self.xy(10, layout.limits_min);
        self.xy(11, layout.limits_max);
        self.xy(12, Xy::new(0.0, 0.0));
        self.xy(14, layout.limits_min);
        self.xy(15, layout.limits_max);
        self.num(146, 0.0);
        self.xy(13, Xy::new(0.0, 0.0));
        self.xy(16, Xy::new(1.0, 0.0));
        self.xy(17, Xy::new(0.0, 1.0));
        self.pair(76, 0);
        let block = self.block_record(&layout.block);
        self.pair(330, format!("{block:X}"));
    }
}

/// The blocks a layout can show, in the order the file declares them: the
/// model and the first paper space, which every drawing has, then any other
/// paper space a layout of the spec names. They hold no entities of their
/// own here -- a drawing's entities are written in the ENTITIES section.
pub fn space_names(spec: &Spec) -> Vec<String> {
    let mut names = vec!["*Model_Space".to_string(), "*Paper_Space".to_string()];
    for layout in &spec.layouts {
        if !names.contains(&layout.block) {
            names.push(layout.block.clone());
        }
    }
    names
}

pub(crate) fn midpoint(a: Xy, b: Xy) -> Xy {
    Xy::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

/// DXF 72 of a TEXT.
fn horizontal_code(h: HorizontalJustification) -> u8 {
    match h {
        HorizontalJustification::Left => 0,
        HorizontalJustification::Center => 1,
        HorizontalJustification::Right => 2,
        HorizontalJustification::Aligned => 3,
        HorizontalJustification::Middle => 4,
        HorizontalJustification::Fit => 5,
    }
}

/// DXF 73 of a TEXT.
fn vertical_code(v: VerticalJustification) -> u8 {
    match v {
        VerticalJustification::Baseline => 0,
        VerticalJustification::Bottom => 1,
        VerticalJustification::Middle => 2,
        VerticalJustification::Top => 3,
    }
}

/// DXF 277 (`DIMLUNIT`).
fn linear_unit_code(f: LinearUnitFormat) -> u8 {
    match f {
        LinearUnitFormat::Scientific => 1,
        LinearUnitFormat::Decimal => 2,
        LinearUnitFormat::Engineering => 3,
        LinearUnitFormat::Architectural => 4,
        LinearUnitFormat::Fractional => 5,
        LinearUnitFormat::WindowsDesktop => 6,
    }
}

/// DXF 275 (`DIMAUNIT`).
fn angular_unit_code(f: AngularUnitFormat) -> u8 {
    match f {
        AngularUnitFormat::DecimalDegrees => 0,
        AngularUnitFormat::DegreesMinutesSeconds => 1,
        AngularUnitFormat::Gradians => 2,
        AngularUnitFormat::Radians => 3,
        AngularUnitFormat::SurveyorsUnits => 4,
    }
}

/// DXF 276 (`DIMFRAC`).
fn fraction_code(f: FractionFormat) -> u8 {
    match f {
        FractionFormat::Horizontal => 0,
        FractionFormat::Diagonal => 1,
        FractionFormat::NotStacked => 2,
    }
}

/// DXF 72 of a layout's plot settings.
fn paper_units_code(u: PlotPaperUnits) -> u8 {
    match u {
        PlotPaperUnits::Inches => 0,
        PlotPaperUnits::Millimeters => 1,
        PlotPaperUnits::Pixels => 2,
    }
}

/// DXF 73 of a layout's plot settings.
fn rotation_code(r: PlotRotation) -> u8 {
    match r {
        PlotRotation::Unrotated => 0,
        PlotRotation::Counterclockwise90 => 1,
        PlotRotation::UpsideDown => 2,
        PlotRotation::Clockwise90 => 3,
    }
}

/// The drawn geometry of a dimension, as the anonymous block a DXF carries
/// for it: the dimension line, two extension lines and the text. This is
/// what a reader that only sees the block (the model carries a dimension as
/// its block reference) gets back.
fn dimension_block(e: &EntitySpec, count: &mut usize) -> Option<(String, Vec<EntitySpec>)> {
    e.is_dimension().then(|| {
        *count += 1;
        (format!("*D{count}"), dimension_geometry(e))
    })
}

/// The drawn geometry of one dimension (see [`dimension_block`]); public so
/// the oracle can state the same entities.
pub fn dimension_geometry(e: &EntitySpec) -> Vec<EntitySpec> {
    let layer = "0".to_string();
    match e {
        EntitySpec::ArcDimension { from, to, text, .. } => {
            // An arc-length dimension's drawn block: the two measured points
            // joined, with the text between them. The arc itself is not
            // drawn -- what a dimension's block holds is what a reader gets
            // back, and this case is about the values, not the picture.
            vec![
                EntitySpec::Line {
                    layer: layer.clone(),
                    start: *from,
                    end: *to,
                },
                EntitySpec::Text {
                    layer,
                    insert: midpoint(*from, *to),
                    height: 2.5,
                    text: text.clone(),
                    rotation_deg: 0.0,
                },
            ]
        }
        EntitySpec::LinearDimension {
            from,
            to,
            line_point,
            text,
            ..
        } => {
            // The dimension line runs parallel to the measured span, offset
            // to the line point; extension lines join each measured point
            // to it. A span that is taller than it is wide is a vertical
            // dimension: the line sits at `line_point.x` and the text is
            // rotated 90 degrees. (Drawing every dimension as horizontal
            // gave a vertical one a zero-length dimension line and two
            // coincident zero-length extension lines.)
            let vertical = (to.y - from.y).abs() > (to.x - from.x).abs();
            let (line_start, line_end, ext_from, ext_to, text_insert, text_rotation) = if vertical {
                let x = line_point.x;
                (
                    Xy::new(x, from.y),
                    Xy::new(x, to.y),
                    Xy::new(x, from.y),
                    Xy::new(x, to.y),
                    Xy::new(x - 1.0, (from.y + to.y) / 2.0),
                    90.0,
                )
            } else {
                let y = line_point.y;
                (
                    Xy::new(from.x, y),
                    Xy::new(to.x, y),
                    Xy::new(from.x, y),
                    Xy::new(to.x, y),
                    Xy::new((from.x + to.x) / 2.0, y + 1.0),
                    0.0,
                )
            };
            vec![
                EntitySpec::Line {
                    layer: layer.clone(),
                    start: line_start,
                    end: line_end,
                },
                EntitySpec::Line {
                    layer: layer.clone(),
                    start: *from,
                    end: ext_from,
                },
                EntitySpec::Line {
                    layer: layer.clone(),
                    start: *to,
                    end: ext_to,
                },
                EntitySpec::Text {
                    layer,
                    insert: text_insert,
                    height: 2.5,
                    text: text.clone(),
                    rotation_deg: text_rotation,
                },
            ]
        }
        EntitySpec::DiameterDimension {
            first,
            second,
            text,
            ..
        } => {
            let mid = midpoint(*first, *second);
            let entities = vec![
                EntitySpec::Line {
                    layer: layer.clone(),
                    start: *first,
                    end: *second,
                },
                EntitySpec::Text {
                    layer,
                    insert: Xy::new(mid.x, mid.y + 1.0),
                    height: 2.5,
                    text: text.clone(),
                    rotation_deg: 0.0,
                },
            ];
            entities
        }
        EntitySpec::OrdinateDimension {
            feature,
            leader_end,
            text,
            ..
        } => {
            // The leader from the feature to its end, and the text there.
            vec![
                EntitySpec::Line {
                    layer: layer.clone(),
                    start: *feature,
                    end: *leader_end,
                },
                EntitySpec::Text {
                    layer,
                    insert: leader_end.moved(1.0, 0.0),
                    height: 2.5,
                    text: text.clone(),
                    rotation_deg: 0.0,
                },
            ]
        }
        _ => Vec::new(),
    }
}
