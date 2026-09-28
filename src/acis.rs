//! The wireframe of an ACIS body, from its SAT text.
//!
//! A 3DSOLID or a REGION stores its shape as an ACIS body -- a boundary
//! representation written in ACIS's own save format. Both file formats carry
//! that body the same way once it is text: the SAT (v1, ASCII) records, one
//! per `#`-terminated entry, pointing at each other by position (`$N`). How a
//! file stores the text around it (a DXF record's obfuscated groups, a binary
//! body a DWG reader converts) is the reader's to undo; this module starts
//! from the SAT text itself.
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
}
