//! The wireframe of an ACIS body, from its SAT text or its SAB bytes.
//!
//! A 3DSOLID or a REGION stores its shape as an ACIS body -- a boundary
//! representation written in ACIS's own save format, either as text (SAT) or
//! in the binary form of the same records (SAB). Both carry records pointing
//! at each other by position (`$N` in SAT). How a file stores the body around
//! those bytes (a DXF record's obfuscated groups, a DXF `ACDSDATA` section's
//! hex chunks, a DWG object's data) is the reader's to undo; this module
//! starts from the SAT text ([`wireframe`]) or the SAB bytes
//! ([`wireframe_sab`]) themselves.
//!
//! It is not an ACIS parser. It reads exactly what [`Solid3DEntity`] carries:
//! for every `edge` record, its two endpoint `vertex` records resolved through
//! their `point` records, as one straight segment. A curved edge becomes its
//! chord; faces and surfaces are not interpreted. The record semantics follow
//! Spatial's public "SAT Save File Format" description.
//!
//! [`Solid3DEntity`]: crate::model::Solid3DEntity

use crate::model::Point3D;

struct SatRecord {
    type_name: String,
    tokens: Vec<String>,
}

/// One straight segment per `edge` record of the SAT text `sat`, and the
/// number of edges that could not be turned into one.
///
/// The second value is what [`Solid3DEntity::skipped_edges`] carries: `0`
/// for a fully read body, and when it is not `0` and there are no segments,
/// the body was not read rather than empty. A text whose `edge` or `vertex`
/// records point past its last record is not read at all -- every edge counts
/// as skipped and none is drawn -- because records are addressed by position,
/// and following such a pointer would attach an edge to whatever record
/// happens to sit there.
///
/// [`Solid3DEntity::skipped_edges`]: crate::model::Solid3DEntity::skipped_edges
pub fn wireframe(sat: &str) -> (Vec<[Point3D; 2]>, usize) {
    extract_wireframe_segments(&parse_sat_records(sat))
}

/// [`wireframe`] for a body stored as SAB, the binary form of the same
/// records; `None` when `sab` cannot be decoded as one.
///
/// Decoding stops at the end-of-data marker, or at the start of a history
/// section. A body is not decoded at all -- rather than decoded up to the
/// point of trouble -- when its signature is not a SAB one, when it ends
/// before its end-of-data marker, or when it holds a tag whose encoding is
/// not known: a guessed width would shift every byte after it, and the
/// records it produced would point at the wrong records.
pub fn wireframe_sab(sab: &[u8]) -> Option<(Vec<[Point3D; 2]>, usize)> {
    Some(extract_wireframe_segments(&parse_sab_records(sab)?))
}

/// Splits SAT (v1, ASCII) text into records, indexed exactly as the text's
/// own `$N` pointers refer to them: 0-based, in order, with the header lines
/// and the `End-of-ACIS-data` marker excluded.
// The fixed 3-line header skip is not verified against every SAT variant.
// On an old R14-era body it did parse every record including 18 real `edge`
// ones, yet none of their vertices resolved -- whether that is this
// assumption being wrong for that ACIS version or some other cause was never
// established.
fn parse_sat_records(sat_text: &str) -> Vec<SatRecord> {
    // The first 3 lines are the header (version, product/version/date
    // string, tolerances); records start on line 3 (0-based).
    let lines: Vec<&str> = sat_text.split('\n').collect();
    let body = if lines.len() > 3 {
        lines[3..].join("\n")
    } else {
        String::new()
    };
    let records_text = match body.find("End-of-ACIS-data") {
        Some(idx) => &body[..idx],
        None => body.as_str(),
    };

    let mut records = Vec::new();
    for chunk in records_text.split('#') {
        let trimmed = chunk.trim();
        if trimmed.is_empty() {
            continue;
        }
        let mut parts = trimmed.split_whitespace();
        let Some(type_name) = parts.next() else {
            continue;
        };
        records.push(SatRecord {
            type_name: type_name.to_string(),
            tokens: parts.map(|s| s.to_string()).collect(),
        });
    }
    records
}

/// The 15-byte signatures a SAB body opens with: ACIS up to version 21800,
/// ASM (Autodesk's ACIS) after it.
const SAB_SIGNATURES: [&[u8; 15]; 2] = [b"ACIS BinaryFile", b"ASM BinaryFile4"];

/// SAB tags: each token is one tag byte, then a payload whose width the tag
/// fixes. Integers and floats are little-endian.
mod sab_tag {
    /// A 32-bit integer.
    pub const INT: u8 = 0x04;
    /// A 64-bit float.
    pub const DOUBLE: u8 = 0x06;
    /// A string of up to 255 bytes: a one-byte length, then the bytes.
    pub const STRING: u8 = 0x07;
    pub const TRUE: u8 = 0x0A;
    pub const FALSE: u8 = 0x0B;
    /// A record pointer: a 32-bit record index, `-1` for none.
    pub const POINTER: u8 = 0x0C;
    /// The last (or only) part of an identifier: a one-byte length, then the
    /// bytes.
    pub const IDENT: u8 = 0x0D;
    /// A leading part of an identifier, joined to the next part with `-`
    /// (`plane` then `surface` is `plane-surface`).
    pub const IDENT_PART: u8 = 0x0E;
    pub const SUBTYPE_START: u8 = 0x0F;
    pub const SUBTYPE_END: u8 = 0x10;
    pub const RECORD_END: u8 = 0x11;
    /// A string with a 32-bit length.
    pub const LONG_STRING: u8 = 0x12;
    /// A position: three 64-bit floats.
    pub const POSITION: u8 = 0x13;
    /// A direction: three 64-bit floats.
    pub const VECTOR: u8 = 0x14;
    /// An enumeration value: a 32-bit integer.
    pub const ENUM: u8 = 0x15;
}

/// Reads the bytes of a SAB body front to back; every read is `None` past
/// the end.
struct SabBytes<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> SabBytes<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let taken = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(taken)
    }

    fn u8(&mut self) -> Option<u8> {
        Some(self.take(1)?[0])
    }

    fn i32(&mut self) -> Option<i32> {
        Some(i32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn f64(&mut self) -> Option<f64> {
        Some(f64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }

    fn text(&mut self, len: usize) -> Option<String> {
        Some(String::from_utf8_lossy(self.take(len)?).into_owned())
    }

    /// One header value: the tag it must carry, then that tag's payload.
    fn expect_tag(&mut self, tag: u8) -> Option<()> {
        (self.u8()? == tag).then_some(())
    }
}

/// Decodes a SAB body into the records [`parse_sat_records`] makes of the
/// same body's SAT text: same order, same `$N` pointers, numbers as their
/// decimal text. `None` when the body cannot be decoded (see
/// [`wireframe_sab`]).
///
/// A record is its type name (the identifier parts before its first other
/// token), its tokens, and a record-end tag. Words the SAT text spells
/// differently by context -- a boolean is `forward`/`reversed` in one record
/// and `I`/`F` in another -- are written as `TRUE`/`FALSE`: nothing read from
/// the records depends on them.
fn parse_sab_records(sab: &[u8]) -> Option<Vec<SatRecord>> {
    let mut b = SabBytes { bytes: sab, pos: 0 };
    let signature = b.take(15)?;
    if !SAB_SIGNATURES.iter().any(|s| s.as_slice() == signature) {
        return None;
    }
    // Version, record count, entity count, flags: nothing read here depends
    // on them.
    for _ in 0..4 {
        b.i32()?;
    }
    // Product, ACIS version and date strings, then the units and the two
    // tolerances.
    for _ in 0..3 {
        b.expect_tag(sab_tag::STRING)?;
        let len = b.u8()?;
        b.take(usize::from(len))?;
    }
    for _ in 0..3 {
        b.expect_tag(sab_tag::DOUBLE)?;
        b.f64()?;
    }

    let mut records = Vec::new();
    let mut type_name: Option<String> = None;
    let mut tokens: Vec<String> = Vec::new();
    // The leading parts of an identifier still waiting for its last part.
    let mut ident_parts: Vec<String> = Vec::new();
    loop {
        let tag = b.u8()?;
        let token = match tag {
            sab_tag::IDENT_PART => {
                let len = b.u8()?;
                ident_parts.push(b.text(usize::from(len))?);
                continue;
            }
            sab_tag::IDENT => {
                let len = b.u8()?;
                ident_parts.push(b.text(usize::from(len))?);
                let ident = ident_parts.join("-");
                ident_parts.clear();
                if type_name.is_some() {
                    ident
                } else if ident.starts_with("End-of-") || ident.starts_with("Begin-of-") {
                    // `End-of-ACIS-data` / `End-of-ASM-data`, or a history
                    // section starting: the records are over.
                    return Some(records);
                } else {
                    type_name = Some(ident);
                    continue;
                }
            }
            _ if !ident_parts.is_empty() => return None,
            _ if type_name.is_none() => return None,
            sab_tag::RECORD_END => {
                records.push(SatRecord {
                    type_name: type_name.take()?,
                    tokens: std::mem::take(&mut tokens),
                });
                continue;
            }
            sab_tag::INT | sab_tag::ENUM => b.i32()?.to_string(),
            sab_tag::DOUBLE => b.f64()?.to_string(),
            sab_tag::STRING => {
                let len = b.u8()?;
                format!("@{len} {}", b.text(usize::from(len))?)
            }
            sab_tag::LONG_STRING => {
                let len = usize::try_from(b.i32()?).ok()?;
                format!("@{len} {}", b.text(len)?)
            }
            sab_tag::TRUE => "TRUE".to_string(),
            sab_tag::FALSE => "FALSE".to_string(),
            sab_tag::POINTER => format!("${}", b.i32()?),
            sab_tag::SUBTYPE_START => "{".to_string(),
            sab_tag::SUBTYPE_END => "}".to_string(),
            sab_tag::POSITION | sab_tag::VECTOR => {
                for _ in 0..3 {
                    tokens.push(b.f64()?.to_string());
                }
                continue;
            }
            _ => return None,
        };
        tokens.push(token);
    }
}

/// `true` when every `$N` pointer in the records the wireframe walk follows
/// (`edge` and `vertex`) addresses an existing record. Other record types are
/// not checked: `eye_refinement` carries `$`-prefixed values that are not
/// pointers at all.
fn pointers_are_in_range(records: &[SatRecord]) -> bool {
    records
        .iter()
        .filter(|r| r.type_name == "edge" || r.type_name == "vertex")
        .flat_map(|r| r.tokens.iter())
        .filter_map(|t| t.strip_prefix('$')?.parse::<usize>().ok())
        .all(|idx| idx < records.len())
}

fn resolve_pointer<'a>(records: &'a [SatRecord], token: &str) -> Option<(usize, &'a SatRecord)> {
    let rest = token.strip_prefix('$')?;
    let idx: usize = rest.parse().ok()?;
    records.get(idx).map(|r| (idx, r))
}

/// A `point` record's coordinates: its last three numbers. What comes before
/// them depends on the ACIS version -- an older body writes only the
/// attribute pointer (`point $-1 0 0 0 #`), a newer one an integer and a
/// history pointer too (`point $-1 -1 $-1 0 0 0 #`) -- so counting from the
/// front would take that integer for x.
fn point_xyz(point_record: &SatRecord) -> Option<[f64; 3]> {
    let nums: Vec<f64> = point_record
        .tokens
        .iter()
        .filter(|t| !t.starts_with('$'))
        .filter_map(|t| t.parse::<f64>().ok())
        .collect();
    let [.., x, y, z] = nums[..] else {
        return None;
    };
    Some([x, y, z])
}

/// Walks every `edge` record and returns one chord segment per edge, plus
/// the number of edges that were skipped because their two endpoint
/// vertices could not both be resolved (unexpected record shape, e.g. a
/// future/older ACIS version this wasn't written against).
fn extract_wireframe_segments(records: &[SatRecord]) -> (Vec<[Point3D; 2]>, usize) {
    let mut segments = Vec::new();
    let mut skipped = 0usize;

    // Measured on a real R2007 drawing: 62 of 116 bodies came back from a
    // binary-to-SAT conversion with pointers up to 166 records past the end
    // -- the converter dropped records it did not handle without renumbering
    // the rest. See `wireframe` for why such a text is not read at all.
    if !pointers_are_in_range(records) {
        let edges = records.iter().filter(|r| r.type_name == "edge").count();
        return (Vec::new(), edges);
    }
    for record in records {
        if record.type_name != "edge" {
            continue;
        }
        let mut vertex_points = Vec::new();
        for token in &record.tokens {
            let Some((_, resolved)) = resolve_pointer(records, token) else {
                continue;
            };
            if resolved.type_name != "vertex" {
                continue;
            }
            let point = resolved
                .tokens
                .iter()
                .filter_map(|t| resolve_pointer(records, t))
                .find(|(_, r)| r.type_name == "point");
            if let Some((_, point_record)) = point {
                if let Some([x, y, z]) = point_xyz(point_record) {
                    vertex_points.push(Point3D { x, y, z });
                }
            }
        }
        if vertex_points.len() == 2 {
            segments.push([vertex_points[0], vertex_points[1]]);
        } else {
            skipped += 1;
        }
    }
    (segments, skipped)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthetic SAT text: 3 header lines (skipped), 2 `point` records, 2
    /// `vertex` records each pointing at one of them, 1 `edge` record
    /// pointing at both vertices, then the `End-of-ACIS-data` marker (with
    /// trailing content after it that must be ignored). Record indices are
    /// 0-based in order: 0=point, 1=point, 2=vertex, 3=vertex, 4=edge.
    const SAT_TEXT: &str = "700 0 1 0\n\
        9 SomeProduct 9 SomeVersion 24 Mon Jan 01 00:00:00 2024\n\
        1e-06 1e-10\n\
        point 0.0 0.0 0.0 #\n\
        point 1.0 2.0 3.0 #\n\
        vertex $0 #\n\
        vertex $1 #\n\
        edge $2 $3 #\n\
        End-of-ACIS-data\n\
        garbage after the marker #\n";

    #[test]
    fn parses_expected_record_count_and_types() {
        let records = parse_sat_records(SAT_TEXT);
        let types: Vec<&str> = records.iter().map(|r| r.type_name.as_str()).collect();
        assert_eq!(types, vec!["point", "point", "vertex", "vertex", "edge"]);
    }

    #[test]
    fn stops_at_end_of_acis_data_marker() {
        let records = parse_sat_records(SAT_TEXT);
        assert!(records.iter().all(|r| r.type_name != "garbage"));
    }

    #[test]
    fn resolves_edge_to_its_two_endpoint_coordinates() {
        let (segments, skipped) = wireframe(SAT_TEXT);
        assert_eq!(skipped, 0);
        assert_eq!(
            segments,
            vec![[
                Point3D {
                    x: 0.0,
                    y: 0.0,
                    z: 0.0
                },
                Point3D {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0
                }
            ]]
        );
    }

    #[test]
    fn edge_with_unresolvable_vertex_is_skipped_not_panicking() {
        // References a vertex index ($9) that doesn't exist -- but the
        // range guard sees it first, so the whole body is unread.
        let (segments, skipped) =
            wireframe("h\nh\nh\npoint 0.0 0.0 0.0 #\nvertex $0 #\nedge $1 $9 #\nEnd-of-ACIS-data");
        assert_eq!(segments.len(), 0);
        assert_eq!(
            skipped, 1,
            "the unresolvable edge must be counted, not just dropped"
        );
    }

    #[test]
    fn an_edge_whose_vertex_has_no_point_is_skipped() {
        let (segments, skipped) = wireframe(
            "h\nh\nh\npoint 0.0 0.0 0.0 #\nvertex $0 #\nvertex $4 #\nedge $1 $2 #\nline #\nEnd-of-ACIS-data",
        );
        assert_eq!((segments.len(), skipped), (0, 1));
    }

    #[test]
    fn resolve_pointer_rejects_non_dollar_and_out_of_range_tokens() {
        let records = parse_sat_records(SAT_TEXT);
        assert!(resolve_pointer(&records, "not-a-pointer").is_none());
        assert!(resolve_pointer(&records, "$999").is_none());
        assert!(resolve_pointer(&records, "$0").is_some());
    }

    /// A newer body writes an integer and a history pointer before the
    /// coordinates; they are not x.
    #[test]
    fn a_newer_point_record_takes_its_last_three_numbers() {
        let (segments, skipped) = wireframe(
            "h
h
h
point $-1 -1 $-1 4 5 6 #
point $-1 -1 $-1 7 8 9 #
             vertex $-1 -1 $-1 $4 $0 #
vertex $-1 -1 $-1 $4 $1 #
             edge $-1 -1 $-1 $2 0 $3 1 $-1 forward @7 unknown #
End-of-ACIS-data",
        );
        assert_eq!(skipped, 0);
        assert_eq!(
            segments,
            vec![[
                Point3D {
                    x: 4.0,
                    y: 5.0,
                    z: 6.0
                },
                Point3D {
                    x: 7.0,
                    y: 8.0,
                    z: 9.0
                }
            ]]
        );
    }

    #[test]
    fn point_xyz_requires_at_least_three_numeric_tokens() {
        let records = parse_sat_records(SAT_TEXT);
        assert_eq!(point_xyz(&records[0]), Some([0.0, 0.0, 0.0]));
        let too_few = SatRecord {
            type_name: "point".to_string(),
            tokens: vec!["1.0".to_string(), "2.0".to_string()],
        };
        assert_eq!(point_xyz(&too_few), None);
    }

    /// A text whose edge points past the last record: nothing is drawn and
    /// every edge is counted, even the one that would have resolved.
    #[test]
    fn out_of_range_pointers_make_the_whole_body_unread_not_partly_drawn() {
        let sat = "700 0 1 0
            9 SomeProduct 9 SomeVersion 24 Mon Jan 01 00:00:00 2024
            1e-06 1e-10
            point 0.0 0.0 0.0 #
            point 1.0 2.0 3.0 #
            vertex $0 #
            vertex $1 #
            edge $2 $3 #
            edge $2 $99 #
            End-of-ACIS-data
";
        let records = parse_sat_records(sat);
        assert!(!pointers_are_in_range(&records));
        let (segments, skipped) = wireframe(sat);
        assert!(
            segments.is_empty(),
            "no chord may be drawn from an inconsistent text"
        );
        assert_eq!(
            skipped, 2,
            "both edges count as unread, including the resolvable one"
        );
    }

    /// The guard covers the vertex -> point hop too: a vertex whose point
    /// pointer runs past the records makes the whole body unread, rather
    /// than each edge failing to find a point one by one.
    #[test]
    fn a_vertex_pointing_past_the_records_makes_the_whole_body_unread() {
        let sat = "700 0 1 0
            9 SomeProduct 9 SomeVersion 24 Mon Jan 01 00:00:00 2024
            1e-06 1e-10
            point 0.0 0.0 0.0 #
            vertex $0 #
            vertex $77 #
            edge $1 $2 #
            End-of-ACIS-data
";
        let records = parse_sat_records(sat);
        assert!(!pointers_are_in_range(&records));
        let (segments, skipped) = wireframe(sat);
        assert!(segments.is_empty());
        assert_eq!(skipped, 1);
    }

    #[test]
    fn a_dollar_value_inside_eye_refinement_is_not_a_pointer() {
        let sat = "700 0 1 0
            9 SomeProduct 9 SomeVersion 24 Mon Jan 01 00:00:00 2024
            1e-06 1e-10
            eye_refinement $-1 $-1 5 mgrid $3000 #
            point 0.0 0.0 0.0 #
            point 1.0 2.0 3.0 #
            vertex $1 #
            vertex $2 #
            edge $3 $4 #
            End-of-ACIS-data
";
        let records = parse_sat_records(sat);
        assert!(pointers_are_in_range(&records));
        assert_eq!(
            {
                let (s, k) = wireframe(sat);
                (s.len(), k)
            },
            (1, 0)
        );
    }

    #[test]
    fn empty_or_header_only_text_is_an_empty_body() {
        assert_eq!(wireframe(""), (Vec::new(), 0));
        assert_eq!(wireframe("700 0 1 0\nh\n1e-06 1e-10\n"), (Vec::new(), 0));
    }

    /// Writes SAB bytes token by token, the way a body is laid out: the
    /// header, then records, then the end-of-data marker.
    struct Sab(Vec<u8>);

    impl Sab {
        fn new(signature: &[u8; 15]) -> Self {
            let mut s = Sab(signature.to_vec());
            for v in [22300i32, 0, 2, 4] {
                s.0.extend(v.to_le_bytes());
            }
            for text in ["Product", "ASM 223.0", "Mon Jan 01 00:00:00 2024"] {
                s.string(text);
            }
            for v in [1.0, 1e-6, 1e-10] {
                s.double(v);
            }
            s
        }
        fn ident(&mut self, name: &str) -> &mut Self {
            let parts: Vec<&str> = name.split('-').collect();
            for (i, part) in parts.iter().enumerate() {
                let tag = if i + 1 == parts.len() { 0x0D } else { 0x0E };
                self.0.push(tag);
                self.0.push(part.len() as u8);
                self.0.extend(part.as_bytes());
            }
            self
        }
        fn string(&mut self, text: &str) -> &mut Self {
            self.0.push(0x07);
            self.0.push(text.len() as u8);
            self.0.extend(text.as_bytes());
            self
        }
        fn int(&mut self, v: i32) -> &mut Self {
            self.0.push(0x04);
            self.0.extend(v.to_le_bytes());
            self
        }
        fn double(&mut self, v: f64) -> &mut Self {
            self.0.push(0x06);
            self.0.extend(v.to_le_bytes());
            self
        }
        fn ptr(&mut self, v: i32) -> &mut Self {
            self.0.push(0x0C);
            self.0.extend(v.to_le_bytes());
            self
        }
        fn position(&mut self, [x, y, z]: [f64; 3]) -> &mut Self {
            self.0.push(0x13);
            for v in [x, y, z] {
                self.0.extend(v.to_le_bytes());
            }
            self
        }
        fn tag(&mut self, tag: u8) -> &mut Self {
            self.0.push(tag);
            self
        }
        fn end(&mut self) -> &mut Self {
            self.tag(0x11)
        }
        /// The records of one straight edge as an ASM-era body lays them
        /// out: 0 asmheader, 1 point, 2 point, 3 vertex, 4 vertex, 5 edge.
        fn one_edge(signature: &[u8; 15], a: [f64; 3], b: [f64; 3]) -> Self {
            let mut s = Sab::new(signature);
            s.ident("asmheader")
                .ptr(-1)
                .int(-1)
                .string("223.0.1.1930")
                .end();
            for p in [a, b] {
                s.ident("point").ptr(-1).int(-1).ptr(-1).position(p).end();
            }
            for point in [1, 2] {
                s.ident("vertex")
                    .ptr(-1)
                    .int(-1)
                    .ptr(-1)
                    .ptr(5)
                    .int(0)
                    .ptr(point)
                    .end();
            }
            s.ident("edge")
                .ptr(-1)
                .int(-1)
                .ptr(-1)
                .ptr(3)
                .double(0.0)
                .ptr(4)
                .double(1.0)
                .ptr(-1)
                .ptr(-1)
                .tag(0x0B)
                .string("unknown")
                .end();
            s
        }
        fn end_of_data(&mut self) -> &mut Self {
            self.ident("End-of-ASM-data")
        }
    }

    const A: [f64; 3] = [-54.05474532926735, 11789.649090833082, 0.0];
    const B: [f64; 3] = [1.5, 2.25, -3.0];

    fn segment(a: [f64; 3], b: [f64; 3]) -> [Point3D; 2] {
        let p = |[x, y, z]: [f64; 3]| Point3D { x, y, z };
        [p(a), p(b)]
    }

    #[test]
    fn a_sab_edge_resolves_to_its_endpoints_exactly() {
        for signature in SAB_SIGNATURES {
            let mut sab = Sab::one_edge(signature, A, B);
            sab.end_of_data();
            assert_eq!(
                wireframe_sab(&sab.0),
                Some((vec![segment(A, B)], 0)),
                "{}",
                String::from_utf8_lossy(signature)
            );
        }
    }

    #[test]
    fn sab_records_are_numbered_like_the_sat_text() {
        let mut sab = Sab::one_edge(SAB_SIGNATURES[1], A, B);
        sab.end_of_data();
        let records = parse_sab_records(&sab.0).unwrap();
        let types: Vec<&str> = records.iter().map(|r| r.type_name.as_str()).collect();
        assert_eq!(
            types,
            ["asmheader", "point", "point", "vertex", "vertex", "edge"]
        );
        assert_eq!(
            records[5].tokens,
            [
                "$-1",
                "-1",
                "$-1",
                "$3",
                "0",
                "$4",
                "1",
                "$-1",
                "$-1",
                "FALSE",
                "@7 unknown"
            ]
        );
    }

    #[test]
    fn identifier_parts_join_into_one_type_name() {
        let mut sab = Sab::one_edge(SAB_SIGNATURES[1], A, B);
        sab.ident("persubent-acadSolidHistory-attrib")
            .ptr(-1)
            .int(-1)
            .end();
        sab.end_of_data();
        let records = parse_sab_records(&sab.0).unwrap();
        assert_eq!(
            records.last().unwrap().type_name,
            "persubent-acadSolidHistory-attrib"
        );
        assert_eq!(wireframe_sab(&sab.0), Some((vec![segment(A, B)], 0)));
    }

    #[test]
    fn a_history_section_ends_the_records() {
        let mut sab = Sab::one_edge(SAB_SIGNATURES[0], A, B);
        sab.ident("Begin-of-ACIS-History-data").tag(0x99);
        assert_eq!(wireframe_sab(&sab.0), Some((vec![segment(A, B)], 0)));
    }

    #[test]
    fn an_unknown_tag_leaves_the_body_undecoded() {
        let mut sab = Sab::one_edge(SAB_SIGNATURES[1], A, B);
        sab.ident("point").ptr(-1).tag(0x16).end();
        sab.end_of_data();
        assert_eq!(wireframe_sab(&sab.0), None);
    }

    #[test]
    fn a_body_without_its_end_marker_is_undecoded() {
        let sab = Sab::one_edge(SAB_SIGNATURES[1], A, B);
        assert_eq!(wireframe_sab(&sab.0), None);
        let mut cut = Sab::one_edge(SAB_SIGNATURES[1], A, B);
        cut.end_of_data();
        cut.0.truncate(cut.0.len() - 20);
        assert_eq!(wireframe_sab(&cut.0), None);
    }

    #[test]
    fn bytes_that_are_not_sab_are_undecoded() {
        assert_eq!(wireframe_sab(b""), None);
        assert_eq!(wireframe_sab(b"700 0 1 0\npoint 0 0 0 #\n"), None);
        let mut sab = Sab::one_edge(b"ACIS BinaryFilX", A, B);
        sab.end_of_data();
        assert_eq!(wireframe_sab(&sab.0), None);
    }

    #[test]
    fn a_token_outside_any_record_leaves_the_body_undecoded() {
        let mut sab = Sab::new(SAB_SIGNATURES[1]);
        sab.int(3).end();
        sab.end_of_data();
        assert_eq!(wireframe_sab(&sab.0), None);
    }

    #[test]
    fn an_empty_sab_body_is_empty() {
        let mut sab = Sab::new(SAB_SIGNATURES[1]);
        sab.end_of_data();
        assert_eq!(wireframe_sab(&sab.0), Some((Vec::new(), 0)));
    }

    /// The range guard applies to decoded records as it does to text.
    #[test]
    fn a_sab_pointer_past_the_records_leaves_every_edge_unread() {
        let mut sab = Sab::one_edge(SAB_SIGNATURES[1], A, B);
        sab.ident("edge")
            .ptr(-1)
            .int(-1)
            .ptr(-1)
            .ptr(3)
            .ptr(40)
            .end();
        sab.end_of_data();
        assert_eq!(wireframe_sab(&sab.0), Some((Vec::new(), 2)));
    }
}
