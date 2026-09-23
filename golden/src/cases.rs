//! The named golden cases. Each is a function returning its spec, so a
//! consumer can pick the ones its role is measured by.

use crate::spec::{
    AttribSpec, BlockSpec, Codepage, DimStyleSpec, EntitySpec, LayerSpec, Spec, TextAlign, Vertex,
    Xy,
};

/// G1, a general machined part: a closed outline, four holes, three linear
/// dimensions and one diameter dimension, and a title block inserted with
/// three attribute values (drawing number, revision, material).
///
/// The reading / summarizing / pointing / editing / verifying cases of the
/// coverage matrix all start from this drawing.
pub fn g1_general_part() -> Spec {
    let outline = "OUTLINE".to_string();
    let holes = "HOLES".to_string();
    let dims = "DIMS".to_string();
    let title = "TITLE".to_string();

    let hole_centers = [
        Xy::new(20.0, 20.0),
        Xy::new(180.0, 20.0),
        Xy::new(20.0, 80.0),
        Xy::new(180.0, 80.0),
    ];

    let mut entities = vec![EntitySpec::LwPolyline {
        layer: outline.clone(),
        vertices: vec![
            Xy::new(0.0, 0.0).into(),
            Xy::new(200.0, 0.0).into(),
            Xy::new(200.0, 100.0).into(),
            Xy::new(0.0, 100.0).into(),
        ],
        closed: true,
        mirrored: false,
    }];
    for c in hole_centers {
        entities.push(EntitySpec::Circle {
            layer: holes.clone(),
            center: c,
            radius: 5.0,
            mirrored: false,
        });
    }
    entities.push(EntitySpec::LinearDimension {
        layer: dims.clone(),
        from: Xy::new(0.0, 0.0),
        to: Xy::new(200.0, 0.0),
        line_point: Xy::new(0.0, -15.0),
        text: "200".to_string(),
        measurement: None,
        style: None,
    });
    entities.push(EntitySpec::LinearDimension {
        layer: dims.clone(),
        from: Xy::new(0.0, 0.0),
        to: Xy::new(0.0, 100.0),
        line_point: Xy::new(-15.0, 0.0),
        text: "100".to_string(),
        measurement: None,
        style: None,
    });
    entities.push(EntitySpec::LinearDimension {
        layer: dims.clone(),
        from: Xy::new(20.0, 20.0),
        to: Xy::new(180.0, 20.0),
        line_point: Xy::new(20.0, 35.0),
        text: "160".to_string(),
        measurement: None,
        style: None,
    });
    entities.push(EntitySpec::DiameterDimension {
        layer: dims.clone(),
        first: Xy::new(15.0, 20.0),
        second: Xy::new(25.0, 20.0),
        text: "%%C10".to_string(),
        measurement: None,
        style: None,
    });
    entities.push(EntitySpec::Insert {
        layer: title.clone(),
        block: "TITLEBLOCK".to_string(),
        insert: Xy::new(120.0, -60.0),
        scale: 1.0,
        rotation_deg: 0.0,
        attribs: vec![
            AttribSpec {
                tag: "DWGNO".to_string(),
                value: "BP-1042".to_string(),
                insert: Xy::new(125.0, -45.0),
                height: 3.5,
                align: None,
                width_factor: 1.0,
            },
            AttribSpec {
                tag: "REV".to_string(),
                value: "B".to_string(),
                insert: Xy::new(125.0, -52.0),
                height: 3.5,
                align: None,
                width_factor: 1.0,
            },
            AttribSpec {
                tag: "MATERIAL".to_string(),
                value: "SS400".to_string(),
                insert: Xy::new(125.0, -59.0),
                height: 3.5,
                align: None,
                width_factor: 1.0,
            },
        ],
        mirrored: false,
    });

    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ascii,
        layers: vec![
            LayerSpec {
                name: outline,
                color_index: 7,
            },
            LayerSpec {
                name: holes,
                color_index: 1,
            },
            LayerSpec {
                name: dims,
                color_index: 3,
            },
            LayerSpec {
                name: title.clone(),
                color_index: 2,
            },
        ],
        blocks: vec![BlockSpec {
            name: "TITLEBLOCK".to_string(),
            entities: vec![
                EntitySpec::LwPolyline {
                    layer: "0".to_string(),
                    vertices: vec![
                        Xy::new(0.0, 0.0).into(),
                        Xy::new(80.0, 0.0).into(),
                        Xy::new(80.0, 20.0).into(),
                        Xy::new(0.0, 20.0).into(),
                    ],
                    closed: true,
                    mirrored: false,
                },
                EntitySpec::Attdef {
                    layer: "0".to_string(),
                    insert: Xy::new(5.0, 15.0),
                    height: 3.5,
                    tag: "DWGNO".to_string(),
                    prompt: "Drawing number".to_string(),
                    default: "-".to_string(),
                },
                EntitySpec::Attdef {
                    layer: "0".to_string(),
                    insert: Xy::new(5.0, 8.0),
                    height: 3.5,
                    tag: "REV".to_string(),
                    prompt: "Revision".to_string(),
                    default: "-".to_string(),
                },
                EntitySpec::Attdef {
                    layer: "0".to_string(),
                    insert: Xy::new(5.0, 1.0),
                    height: 3.5,
                    tag: "MATERIAL".to_string(),
                    prompt: "Material".to_string(),
                    default: "-".to_string(),
                },
            ],
        }],
        entities,
    }
}

/// G2, block nesting three deep with a rotation and a scale at each level:
/// block `C` holds one LINE, `B` inserts `C` rotated 90 degrees at (5, 0),
/// `A` inserts `B` at scale 2, and the drawing inserts `A` at (100, 100).
/// The line's absolute position is therefore (110, 100) to (110, 120) --
/// the value a renderer's composed transform must produce.
pub fn g2_nested_blocks() -> Spec {
    let layer = "0".to_string();
    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ascii,
        layers: Vec::new(),
        blocks: vec![
            BlockSpec {
                name: "C".to_string(),
                entities: vec![EntitySpec::Line {
                    layer: layer.clone(),
                    start: Xy::new(0.0, 0.0),
                    end: Xy::new(10.0, 0.0),
                }],
            },
            BlockSpec {
                name: "B".to_string(),
                entities: vec![EntitySpec::Insert {
                    layer: layer.clone(),
                    block: "C".to_string(),
                    insert: Xy::new(5.0, 0.0),
                    scale: 1.0,
                    rotation_deg: 90.0,
                    attribs: Vec::new(),
                    mirrored: false,
                }],
            },
            BlockSpec {
                name: "A".to_string(),
                entities: vec![EntitySpec::Insert {
                    layer: layer.clone(),
                    block: "B".to_string(),
                    insert: Xy::new(0.0, 0.0),
                    scale: 2.0,
                    rotation_deg: 0.0,
                    attribs: Vec::new(),
                    mirrored: false,
                }],
            },
        ],
        entities: vec![EntitySpec::Insert {
            layer,
            block: "A".to_string(),
            insert: Xy::new(100.0, 100.0),
            scale: 1.0,
            rotation_deg: 0.0,
            attribs: Vec::new(),
            mirrored: false,
        }],
    }
}

/// G6, two identical lines on top of each other: a reader keeps both as
/// two entities with two IDs, and a pointer asked for the entity at that
/// place must answer with both candidates rather than pick one.
pub fn g6_overlapping_lines() -> Spec {
    let line = |layer: &str| EntitySpec::Line {
        layer: layer.to_string(),
        start: Xy::new(0.0, 0.0),
        end: Xy::new(50.0, 0.0),
    };
    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ascii,
        layers: Vec::new(),
        blocks: Vec::new(),
        entities: vec![line("0"), line("0")],
    }
}

/// G9, two title blocks with two different drawing numbers: a summarizer
/// asked for *the* drawing number must answer "unknown", not pick one.
pub fn g9_two_drawing_numbers() -> Spec {
    let g1 = g1_general_part();
    let title_block = g1
        .blocks
        .into_iter()
        .find(|b| b.name == "TITLEBLOCK")
        .expect("G1 has a title block");
    let insert = |x: f64, number: &str| EntitySpec::Insert {
        layer: "TITLE".to_string(),
        block: "TITLEBLOCK".to_string(),
        insert: Xy::new(x, -60.0),
        scale: 1.0,
        rotation_deg: 0.0,
        attribs: vec![AttribSpec {
            tag: "DWGNO".to_string(),
            value: number.to_string(),
            insert: Xy::new(x + 5.0, -45.0),
            height: 3.5,
            // Right-aligned on the field's right edge, middle of the row,
            // narrowed: where a drawing number usually sits in a title block.
            align: Some(TextAlign {
                horizontal: 2,
                vertical: 2,
                at: Xy::new(x + 45.0, -43.25),
            }),
            width_factor: 0.9,
        }],
        mirrored: false,
    };
    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: "TITLE".to_string(),
            color_index: 2,
        }],
        blocks: vec![title_block],
        entities: vec![insert(0.0, "BP-1042"), insert(120.0, "BP-2077")],
    }
}

/// G5, a drawing that is mostly dimensions, and the case that carries what
/// no file in any corpus here carries: the two spellings of "show the
/// measurement" that are not just an absent group, the single space that
/// means "show nothing", a literal that disagrees with the measurement the
/// same dimension states, an arc-length dimension (its own entity, with a
/// group 70 that says something else), and a DIMSTYLE table that states
/// three of its variables and stays silent on the rest.
pub fn g5_dense_dimensions() -> Spec {
    let dims = "DIMS".to_string();
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: dims.clone(),
            color_index: 3,
        }],
        blocks: Vec::new(),
        dim_styles: vec![DimStyleSpec {
            name: "ISO-25".to_string(),
            post: Some("<>mm".to_string()),
            decimal_places: Some(2),
            text_height: Some(2.5),
        }],
        entities: vec![
            // "<>" and "" are the same thing said two ways, and a reader
            // that keeps them apart makes two drawings differ over spelling.
            EntitySpec::LinearDimension {
                layer: dims.clone(),
                from: Xy::new(0.0, 0.0),
                to: Xy::new(120.0, 0.0),
                line_point: Xy::new(0.0, -15.0),
                text: "<>".to_string(),
                measurement: Some(120.0),
                style: Some("ISO-25".to_string()),
            },
            EntitySpec::LinearDimension {
                layer: dims.clone(),
                from: Xy::new(0.0, 40.0),
                to: Xy::new(120.0, 40.0),
                line_point: Xy::new(0.0, 55.0),
                text: String::new(),
                measurement: Some(120.0),
                style: Some("ISO-25".to_string()),
            },
            // A single space is the drawing saying "show nothing".
            EntitySpec::LinearDimension {
                layer: dims.clone(),
                from: Xy::new(0.0, 80.0),
                to: Xy::new(120.0, 80.0),
                line_point: Xy::new(0.0, 95.0),
                text: " ".to_string(),
                measurement: Some(120.0),
                style: Some("ISO-25".to_string()),
            },
            // The text and the measurement disagree, which real drawings do:
            // both have to survive, because only one of them is the drawing's
            // claim about the part.
            EntitySpec::LinearDimension {
                layer: dims.clone(),
                from: Xy::new(0.0, 120.0),
                to: Xy::new(120.0, 120.0),
                line_point: Xy::new(0.0, 135.0),
                text: "125".to_string(),
                measurement: Some(120.0),
                style: Some("ISO-25".to_string()),
            },
            // Naming a style the file never declares.
            EntitySpec::LinearDimension {
                layer: dims.clone(),
                from: Xy::new(0.0, 160.0),
                to: Xy::new(120.0, 160.0),
                line_point: Xy::new(0.0, 175.0),
                text: "<>".to_string(),
                measurement: None,
                style: Some("NOT-DECLARED".to_string()),
            },
            EntitySpec::DiameterDimension {
                layer: dims.clone(),
                first: Xy::new(60.0, 200.0),
                second: Xy::new(80.0, 200.0),
                text: "%%C20".to_string(),
                measurement: Some(20.0),
                style: Some("ISO-25".to_string()),
            },
            EntitySpec::ArcDimension {
                layer: dims,
                from: Xy::new(0.0, 240.0),
                to: Xy::new(120.0, 240.0),
                center: Xy::new(60.0, 220.0),
                line_point: Xy::new(60.0, 260.0),
                text: "<>".to_string(),
                measurement: Some(133.5),
                style: Some("ISO-25".to_string()),
            },
        ],
    }
}

/// G10, a block reference to a block the file never defines: the reference
/// must come back unresolved and carrying the name the file wrote, never as
/// an empty name, never as absent (the file did point at something), and
/// never as a silently dropped entity.
pub fn g10_unreferenced_insert() -> Spec {
    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ascii,
        layers: Vec::new(),
        blocks: Vec::new(),
        entities: vec![
            EntitySpec::Line {
                layer: "0".to_string(),
                start: Xy::new(0.0, 0.0),
                end: Xy::new(10.0, 10.0),
            },
            EntitySpec::Insert {
                layer: "0".to_string(),
                block: "MISSING".to_string(),
                insert: Xy::new(20.0, 20.0),
                scale: 1.0,
                rotation_deg: 0.0,
                attribs: Vec::new(),
                mirrored: false,
            },
        ],
    }
}

/// G7, a title block drawn as loose TEXT entities rather than as a block
/// with attributes: a frame, then label/value pairs as separate texts.
/// Nothing but position ties a value to its label. It also carries mirror
/// copies -- a circle, an arc, a polyline, a block reference and a text written in
/// their own coordinate systems, extrusion (0, 0, -1).
pub fn g7_loose_text_title_block() -> Spec {
    let title = "TITLE".to_string();
    let text = |x: f64, y: f64, s: &str| EntitySpec::Text {
        layer: title.clone(),
        insert: Xy::new(x, y),
        height: 3.5,
        text: s.to_string(),
        rotation_deg: 0.0,
        align: None,
        width_factor: 1.0,
        mirrored: false,
    };
    let entities = vec![
        EntitySpec::LwPolyline {
            layer: title.clone(),
            // Two arc segments, so a reader that drops the bulge is caught:
            // the right edge bows outwards (counter-clockwise), and the
            // closing segment -- the last vertex back to the first -- bows
            // the other way.
            vertices: vec![
                Xy::new(100.0, -60.0).into(),
                Vertex::bulged(Xy::new(180.0, -60.0), 0.5),
                Xy::new(180.0, -40.0).into(),
                Vertex::bulged(Xy::new(100.0, -40.0), -0.5),
            ],
            closed: true,
            mirrored: false,
        },
        text(102.0, -45.0, "DWG NO"),
        text(130.0, -45.0, "BP-1042"),
        text(102.0, -52.0, "REV"),
        text(130.0, -52.0, "B"),
        text(102.0, -59.0, "MATERIAL"),
        text(130.0, -59.0, "SS400"),
        // A mirror copy: extrusion (0, 0, -1), so the centers are written
        // in a coordinate system whose x runs the other way -- the circle
        // is drawn at (170, -50) and the arc about (110, -50), turning
        // clockwise in the world from its start to its end.
        EntitySpec::Circle {
            layer: title.clone(),
            center: Xy::new(-170.0, -50.0),
            radius: 3.0,
            mirrored: true,
        },
        EntitySpec::Arc {
            layer: title.clone(),
            center: Xy::new(-110.0, -50.0),
            radius: 4.0,
            start_deg: 30.0,
            end_deg: 150.0,
            mirrored: true,
        },
        // A mirrored triangle with one arc segment: drawn with its vertices
        // at (150, -58), (140, -58) and (145, -53), the arc from the second
        // turning the other way in the world than its bulge says in its own
        // system.
        EntitySpec::LwPolyline {
            layer: title.clone(),
            vertices: vec![
                Xy::new(-150.0, -58.0).into(),
                Vertex::bulged(Xy::new(-140.0, -58.0), 0.5),
                Xy::new(-145.0, -53.0).into(),
            ],
            closed: true,
            mirrored: true,
        },
        // A caption centered on (140, -67) both ways, drawn at 0.8 of its
        // normal width: the point the text answers to is the alignment
        // point, and the start point is what the writer computed from its
        // font.
        EntitySpec::Text {
            layer: title.clone(),
            insert: Xy::new(128.8, -68.75),
            height: 3.5,
            text: "PLATE".to_string(),
            rotation_deg: 0.0,
            align: Some(TextAlign {
                horizontal: 1,
                vertical: 2,
                at: Xy::new(140.0, -67.0),
            }),
            width_factor: 0.8,
            mirrored: false,
        },
        // A mirror copy of a block: the INSERT is written at (-175, -66) in
        // a system whose x is the world's -x, turned 30 degrees there. The
        // block's line (0, 0)-(8, 0) is drawn from (175, -66) up and to the
        // left, to (175 - 8 cos 30, -66 + 8 sin 30).
        EntitySpec::Insert {
            layer: title.clone(),
            block: "MARK".to_string(),
            insert: Xy::new(-175.0, -66.0),
            scale: 1.0,
            rotation_deg: 30.0,
            attribs: Vec::new(),
            mirrored: true,
        },
        // A mirror copy of a text: written at (-170, -72) in a system whose
        // x is the world's -x, so drawn from (170, -72), reading right to
        // left.
        EntitySpec::Text {
            layer: title.clone(),
            insert: Xy::new(-170.0, -72.0),
            height: 2.5,
            text: "MIRROR".to_string(),
            rotation_deg: 0.0,
            align: None,
            width_factor: 1.0,
            mirrored: true,
        },
    ];
    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: title,
            color_index: 2,
        }],
        blocks: vec![BlockSpec {
            name: "MARK".to_string(),
            entities: vec![EntitySpec::Line {
                layer: "0".to_string(),
                start: Xy::new(0.0, 0.0),
                end: Xy::new(8.0, 0.0),
            }],
        }],
        entities,
    }
}

/// G8, a title block with Korean text throughout -- layer and block names,
/// attribute values and defaults, a loose text -- written in the CP949
/// codepage an R2000 DXF stores such text in. The strings are Unicode
/// escapes so this source stays ASCII; the expected model carries them as
/// UTF-8, which is what a reader has to produce from the CP949 bytes.
pub fn g8_korean_title_block() -> Spec {
    // "title block", "outline", "title block" (the block's name).
    let title_layer = "\u{D45C}\u{C81C}\u{B780}".to_string();
    let outline_layer = "\u{C678}\u{D615}\u{C120}".to_string();
    let block = "\u{D45C}\u{C81C}\u{BE14}\u{B85D}".to_string();
    // "SS400 general structural rolled steel", "Hong Gildong" (a stock
    // placeholder name), "undecided", "scale 1:1".
    let material =
        "SS400 \u{C77C}\u{BC18}\u{AD6C}\u{C870}\u{C6A9} \u{C555}\u{C5F0}\u{AC15}\u{C7AC}";
    let drawn_by = "\u{D64D}\u{AE38}\u{B3D9}";
    let undecided = "\u{BBF8}\u{C815}";
    let scale = "\u{CD95}\u{CC99} 1:1";

    let attdef = |y: f64, tag: &str, default: &str| EntitySpec::Attdef {
        layer: "0".to_string(),
        insert: Xy::new(5.0, y),
        height: 3.5,
        tag: tag.to_string(),
        prompt: tag.to_string(),
        default: default.to_string(),
    };
    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ansi949,
        layers: vec![
            LayerSpec {
                name: outline_layer.clone(),
                color_index: 7,
            },
            LayerSpec {
                name: title_layer.clone(),
                color_index: 2,
            },
        ],
        blocks: vec![BlockSpec {
            name: block.clone(),
            entities: vec![
                EntitySpec::LwPolyline {
                    layer: "0".to_string(),
                    vertices: vec![
                        Xy::new(0.0, 0.0).into(),
                        Xy::new(80.0, 0.0).into(),
                        Xy::new(80.0, 20.0).into(),
                        Xy::new(0.0, 20.0).into(),
                    ],
                    closed: true,
                    mirrored: false,
                },
                attdef(15.0, "DWGNO", ""),
                attdef(8.0, "MATERIAL", "SS400"),
                attdef(1.0, "DRAWN", undecided),
            ],
        }],
        entities: vec![
            EntitySpec::LwPolyline {
                layer: outline_layer,
                vertices: vec![
                    Xy::new(0.0, 0.0).into(),
                    Xy::new(200.0, 0.0).into(),
                    Xy::new(200.0, 100.0).into(),
                    Xy::new(0.0, 100.0).into(),
                ],
                closed: true,
                mirrored: false,
            },
            EntitySpec::Insert {
                layer: title_layer.clone(),
                block,
                insert: Xy::new(120.0, -60.0),
                scale: 1.0,
                rotation_deg: 0.0,
                attribs: vec![
                    AttribSpec {
                        tag: "DWGNO".to_string(),
                        value: "BP-1042".to_string(),
                        insert: Xy::new(125.0, -45.0),
                        height: 3.5,
                        align: None,
                        width_factor: 1.0,
                    },
                    AttribSpec {
                        tag: "MATERIAL".to_string(),
                        value: material.to_string(),
                        insert: Xy::new(125.0, -52.0),
                        height: 3.5,
                        align: None,
                        width_factor: 1.0,
                    },
                    AttribSpec {
                        tag: "DRAWN".to_string(),
                        value: drawn_by.to_string(),
                        insert: Xy::new(125.0, -59.0),
                        height: 3.5,
                        align: None,
                        width_factor: 1.0,
                    },
                ],
                mirrored: false,
            },
            EntitySpec::Text {
                layer: title_layer,
                insert: Xy::new(0.0, -70.0),
                height: 3.5,
                text: scale.to_string(),
                rotation_deg: 0.0,
                align: None,
                width_factor: 1.0,
                mirrored: false,
            },
        ],
    }
}

/// G3, one drawing holding many copies of G1 laid out on a grid.
///
/// The case the others cannot be: every other case is small enough that a
/// reader's cost does not show, and the questions this one asks are about
/// cost -- whether a read stays linear in the entities, and whether a
/// comparison between two drawings does. A defect that only appears at ten
/// thousand entities is invisible in a drawing of thirty.
///
/// **Generated rather than checked in**, unlike the cases in [`NAMES`]:
/// its fixture would be megabytes of DXF whose exact bytes nobody reads,
/// and the oracle here is not the bytes but how the numbers grow. Callers
/// choose `copies`, so the same case can be built at two sizes and the two
/// compared -- which is what a cost question actually needs.
///
/// The copies are laid out in a square-ish grid with a gap wider than the
/// part, so no copy touches another and a coordinate can be predicted from
/// the copy's index alone.
pub fn g3_many_parts(copies: usize) -> Spec {
    let one = g1_general_part();
    let per_row = (copies as f64).sqrt().ceil().max(1.0) as usize;
    let mut entities = Vec::with_capacity(one.entities.len() * copies);
    for i in 0..copies {
        let (dx, dy) = g3_offset(i, per_row);
        entities.extend(one.entities.iter().map(|e| e.moved(dx, dy)));
    }
    Spec {
        codepage: one.codepage,
        layers: one.layers,
        blocks: one.blocks,
        dim_styles: one.dim_styles,
        entities,
    }
}

/// Where copy `i` of [`g3_many_parts`] sits. The pitch is wider than the
/// part in G1 (200 x 100 units), so copies never overlap and a test can
/// predict a coordinate from the index.
pub fn g3_offset(i: usize, per_row: usize) -> (f64, f64) {
    const PITCH_X: f64 = 300.0;
    const PITCH_Y: f64 = 200.0;
    let row = i / per_row.max(1);
    let col = i % per_row.max(1);
    (col as f64 * PITCH_X, row as f64 * PITCH_Y)
}

/// A case by its name (`"g1"`, `"g2"`, ...), or `None`.
pub fn by_name(name: &str) -> Option<Spec> {
    Some(match name {
        "g1" => g1_general_part(),
        "g2" => g2_nested_blocks(),
        "g6" => g6_overlapping_lines(),
        "g7" => g7_loose_text_title_block(),
        "g8" => g8_korean_title_block(),
        "g9" => g9_two_drawing_numbers(),
        "g5" => g5_dense_dimensions(),
        "g10" => g10_unreferenced_insert(),
        _ => return None,
    })
}

/// The names [`by_name`] knows, in order.
///
/// [`g3_many_parts`] is deliberately absent: it takes a size, and its
/// fixture would be checked-in megabytes whose exact bytes answer no
/// question the case asks.
pub const NAMES: [&str; 8] = ["g1", "g2", "g5", "g6", "g7", "g8", "g9", "g10"];
