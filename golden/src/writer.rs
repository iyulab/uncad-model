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

use crate::spec::{AttribSpec, BlockSpec, Codepage, EntitySpec, Spec, Xy};

/// First handle issued to an entity. Table entries and block records come
/// before it, so that a reader listing entities by handle sees them in file
/// order.
const FIRST_ENTITY_HANDLE: u32 = 0x100;

/// The handles a written drawing carries, so an oracle can name them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Handles {
    /// Handle of each top-level entity, in file order. A dimension's
    /// anonymous block does not appear here; its entities are in `blocks`.
    pub entities: Vec<u32>,
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
/// oracle for anything.
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

    /// The extrusion group (210/220/230), written only when it is not the
    /// default -- as AutoCAD writes it.
    fn extrusion(&mut self, mirrored: bool) {
        if mirrored {
            self.num(210, 0.0);
            self.num(220, 0.0);
            self.num(230, -1.0);
        }
    }

    fn xy(&mut self, base: u16, p: Xy) {
        self.num(base, p.x);
        self.num(base + 10, p.y);
        self.num(base + 20, 0.0);
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
        for e in &spec.entities {
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
        self.layer("0", 7, table);
        for l in &spec.layers {
            self.layer(&l.name, l.color_index, table);
        }
        self.pair(0, "ENDTAB");

        if !spec.dim_styles.is_empty() {
            self.pair(0, "TABLE");
            self.pair(2, "DIMSTYLE");
            let h = self.handle();
            self.pair(5, format!("{h:X}"));
            self.pair(100, "AcDbSymbolTable");
            self.pair(70, spec.dim_styles.len());
            let table = h;
            for style in &spec.dim_styles {
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
                if let Some(height) = style.text_height {
                    self.num(140, height);
                }
                if let Some(places) = style.decimal_places {
                    self.pair(271, places);
                }
            }
            self.pair(0, "ENDTAB");
        }

        self.pair(0, "TABLE");
        self.pair(2, "BLOCK_RECORD");
        let h = self.handle();
        self.pair(5, format!("{h:X}"));
        self.pair(100, "AcDbSymbolTable");
        self.pair(70, spec.blocks.len() + self.dim_blocks.len() + 2);
        let table = h;
        for name in self.block_names(spec) {
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

    fn layer(&mut self, name: &str, color_index: i16, table: u32) {
        self.pair(0, "LAYER");
        let h = self.handle();
        self.pair(5, format!("{h:X}"));
        self.pair(330, format!("{table:X}"));
        self.pair(100, "AcDbSymbolTableRecord");
        self.pair(100, "AcDbLayerTableRecord");
        self.pair(2, name);
        self.pair(70, 0);
        self.pair(62, color_index);
        self.pair(6, "CONTINUOUS");
    }

    fn block_names(&self, spec: &Spec) -> Vec<String> {
        let mut names = vec!["*Model_Space".to_string(), "*Paper_Space".to_string()];
        names.extend(spec.blocks.iter().map(|b| b.name.clone()));
        names.extend(self.dim_blocks.iter().map(|(n, _)| n.clone()));
        names
    }

    fn blocks(&mut self, spec: &Spec) {
        self.pair(0, "SECTION");
        self.pair(2, "BLOCKS");
        for name in ["*Model_Space", "*Paper_Space"] {
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
        let owner = self.block_record("*Model_Space");
        let mut dim_index = 0;
        for e in &spec.entities {
            let dim_name = match e {
                EntitySpec::LinearDimension { .. }
                | EntitySpec::ArcDimension { .. }
                | EntitySpec::DiameterDimension { .. } => {
                    dim_index += 1;
                    Some(format!("*D{dim_index}"))
                }
                _ => None,
            };
            let h = self.entity(e, dim_name.as_deref(), owner);
            self.handles.entities.push(h);
        }
        self.pair(0, "ENDSEC");
    }

    /// Writes one entity and returns its handle. `dim_block` names the
    /// anonymous block a dimension refers to; `owner` is the BLOCK_RECORD
    /// that owns the entity (DXF 330).
    fn entity(&mut self, e: &EntitySpec, dim_block: Option<&str>, owner: u32) -> u32 {
        let h = self.handle();
        let hex = format!("{h:X}");
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
                mirrored,
            } => {
                self.pair(0, "CIRCLE");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbCircle");
                self.xy(10, *center);
                self.num(40, *radius);
                self.extrusion(*mirrored);
            }
            EntitySpec::Arc {
                layer,
                center,
                radius,
                start_deg,
                end_deg,
                mirrored,
            } => {
                self.pair(0, "ARC");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbCircle");
                self.xy(10, *center);
                self.num(40, *radius);
                self.extrusion(*mirrored);
                self.pair(100, "AcDbArc");
                self.num(50, *start_deg);
                self.num(51, *end_deg);
            }
            EntitySpec::LwPolyline {
                layer,
                vertices,
                closed,
                mirrored,
            } => {
                self.pair(0, "LWPOLYLINE");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbPolyline");
                self.pair(90, vertices.len());
                self.pair(70, u16::from(*closed));
                for v in vertices {
                    self.num(10, v.at.x);
                    self.num(20, v.at.y);
                    // Written only when the segment is an arc, as AutoCAD
                    // writes it: an absent 42 is a straight segment.
                    if v.bulge != 0.0 {
                        self.num(42, v.bulge);
                    }
                }
                self.extrusion(*mirrored);
            }
            EntitySpec::Text {
                layer,
                insert,
                height,
                text,
                rotation_deg,
                align,
                width_factor,
                mirrored,
            } => {
                self.pair(0, "TEXT");
                self.common(&hex, layer, owner);
                self.pair(100, "AcDbText");
                self.xy(10, *insert);
                self.num(40, *height);
                self.pair(1, text);
                self.num(50, *rotation_deg);
                if *width_factor != 1.0 {
                    self.num(41, *width_factor);
                }
                if let Some(a) = align {
                    self.pair(72, a.horizontal);
                    self.xy(11, a.at);
                }
                self.extrusion(*mirrored);
                self.pair(100, "AcDbText");
                if let Some(a) = align {
                    self.pair(73, a.vertical);
                }
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
                mirrored,
            } => {
                self.pair(0, "INSERT");
                self.common(&hex, layer, owner);
                if !attribs.is_empty() {
                    self.pair(66, 1);
                }
                self.pair(100, "AcDbBlockReference");
                self.pair(2, block);
                self.xy(10, *insert);
                self.num(41, *scale);
                self.num(42, *scale);
                self.num(43, *scale);
                self.num(50, *rotation_deg);
                self.extrusion(*mirrored);
                if !attribs.is_empty() {
                    // The attributes and the SEQEND are owned by the INSERT.
                    let mut attrib_handles = Vec::new();
                    for a in attribs {
                        attrib_handles.push(self.attrib(a, layer, h));
                    }
                    self.pair(0, "SEQEND");
                    let sh = self.handle();
                    self.pair(5, format!("{sh:X}"));
                    self.pair(330, hex.as_str());
                    self.pair(100, "AcDbEntity");
                    self.pair(8, layer);
                    self.handles.attribs.push((h, attrib_handles));
                }
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
        if a.width_factor != 1.0 {
            self.num(41, a.width_factor);
        }
        if let Some(al) = a.align {
            self.pair(72, al.horizontal);
            self.xy(11, al.at);
        }
        self.pair(100, "AcDbAttribute");
        self.pair(2, &a.tag);
        self.pair(70, 0);
        if let Some(al) = a.align {
            self.pair(74, al.vertical);
        }
        h
    }

    fn common(&mut self, handle_hex: &str, layer: &str, owner: u32) {
        self.pair(5, handle_hex);
        self.pair(330, format!("{owner:X}"));
        self.pair(100, "AcDbEntity");
        self.pair(8, layer);
    }
}

pub(crate) fn midpoint(a: Xy, b: Xy) -> Xy {
    Xy::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
}

/// The drawn geometry of a dimension, as the anonymous block a DXF carries
/// for it: the dimension line, two extension lines and the text. This is
/// what a reader that only sees the block (the model carries a dimension as
/// its block reference) gets back.
fn dimension_block(e: &EntitySpec, count: &mut usize) -> Option<(String, Vec<EntitySpec>)> {
    match e {
        EntitySpec::LinearDimension { .. }
        | EntitySpec::ArcDimension { .. }
        | EntitySpec::DiameterDimension { .. } => {
            *count += 1;
            Some((format!("*D{count}"), dimension_geometry(e)))
        }
        _ => None,
    }
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
                    align: None,
                    width_factor: 1.0,
                    mirrored: false,
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
                    align: None,
                    width_factor: 1.0,
                    mirrored: false,
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
                    align: None,
                    width_factor: 1.0,
                    mirrored: false,
                },
            ];
            entities
        }
        _ => Vec::new(),
    }
}
