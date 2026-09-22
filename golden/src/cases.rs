//! The named golden cases. Each is a function returning its spec, so a
//! consumer can pick the ones its role is measured by.

use crate::spec::{AttribSpec, BlockSpec, Codepage, EntitySpec, LayerSpec, Spec, Xy};

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
            Xy::new(0.0, 0.0),
            Xy::new(200.0, 0.0),
            Xy::new(200.0, 100.0),
            Xy::new(0.0, 100.0),
        ],
        closed: true,
    }];
    for c in hole_centers {
        entities.push(EntitySpec::Circle {
            layer: holes.clone(),
            center: c,
            radius: 5.0,
        });
    }
    entities.push(EntitySpec::LinearDimension {
        layer: dims.clone(),
        from: Xy::new(0.0, 0.0),
        to: Xy::new(200.0, 0.0),
        line_point: Xy::new(0.0, -15.0),
        text: "200".to_string(),
    });
    entities.push(EntitySpec::LinearDimension {
        layer: dims.clone(),
        from: Xy::new(0.0, 0.0),
        to: Xy::new(0.0, 100.0),
        line_point: Xy::new(-15.0, 0.0),
        text: "100".to_string(),
    });
    entities.push(EntitySpec::LinearDimension {
        layer: dims.clone(),
        from: Xy::new(20.0, 20.0),
        to: Xy::new(180.0, 20.0),
        line_point: Xy::new(20.0, 35.0),
        text: "160".to_string(),
    });
    entities.push(EntitySpec::DiameterDimension {
        layer: dims.clone(),
        first: Xy::new(15.0, 20.0),
        second: Xy::new(25.0, 20.0),
        text: "%%C10".to_string(),
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
            },
            AttribSpec {
                tag: "REV".to_string(),
                value: "B".to_string(),
                insert: Xy::new(125.0, -52.0),
                height: 3.5,
            },
            AttribSpec {
                tag: "MATERIAL".to_string(),
                value: "SS400".to_string(),
                insert: Xy::new(125.0, -59.0),
                height: 3.5,
            },
        ],
    });

    Spec {
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
                        Xy::new(0.0, 0.0),
                        Xy::new(80.0, 0.0),
                        Xy::new(80.0, 20.0),
                        Xy::new(0.0, 20.0),
                    ],
                    closed: true,
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
        }],
    };
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: "TITLE".to_string(),
            color_index: 2,
        }],
        blocks: vec![title_block],
        entities: vec![insert(0.0, "BP-1042"), insert(120.0, "BP-2077")],
    }
}

/// G10, a block reference to a block the file never defines: the reference
/// must come back as an absent or unresolved value, never as an empty name
/// and never as a silently dropped entity.
pub fn g10_unreferenced_insert() -> Spec {
    Spec {
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
            },
        ],
    }
}

/// G7, a title block drawn as loose TEXT entities rather than as a block
/// with attributes: a frame, then label/value pairs as separate texts.
/// Nothing but position ties a value to its label.
pub fn g7_loose_text_title_block() -> Spec {
    let title = "TITLE".to_string();
    let text = |x: f64, y: f64, s: &str| EntitySpec::Text {
        layer: title.clone(),
        insert: Xy::new(x, y),
        height: 3.5,
        text: s.to_string(),
        rotation_deg: 0.0,
    };
    let entities = vec![
        EntitySpec::LwPolyline {
            layer: title.clone(),
            vertices: vec![
                Xy::new(100.0, -60.0),
                Xy::new(180.0, -60.0),
                Xy::new(180.0, -40.0),
                Xy::new(100.0, -40.0),
            ],
            closed: true,
        },
        text(102.0, -45.0, "DWG NO"),
        text(130.0, -45.0, "BP-1042"),
        text(102.0, -52.0, "REV"),
        text(130.0, -52.0, "B"),
        text(102.0, -59.0, "MATERIAL"),
        text(130.0, -59.0, "SS400"),
    ];
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: title,
            color_index: 2,
        }],
        blocks: Vec::new(),
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
                        Xy::new(0.0, 0.0),
                        Xy::new(80.0, 0.0),
                        Xy::new(80.0, 20.0),
                        Xy::new(0.0, 20.0),
                    ],
                    closed: true,
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
                    Xy::new(0.0, 0.0),
                    Xy::new(200.0, 0.0),
                    Xy::new(200.0, 100.0),
                    Xy::new(0.0, 100.0),
                ],
                closed: true,
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
                    },
                    AttribSpec {
                        tag: "MATERIAL".to_string(),
                        value: material.to_string(),
                        insert: Xy::new(125.0, -52.0),
                        height: 3.5,
                    },
                    AttribSpec {
                        tag: "DRAWN".to_string(),
                        value: drawn_by.to_string(),
                        insert: Xy::new(125.0, -59.0),
                        height: 3.5,
                    },
                ],
            },
            EntitySpec::Text {
                layer: title_layer,
                insert: Xy::new(0.0, -70.0),
                height: 3.5,
                text: scale.to_string(),
                rotation_deg: 0.0,
            },
        ],
    }
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
        "g10" => g10_unreferenced_insert(),
        _ => return None,
    })
}

/// The names [`by_name`] knows, in order.
pub const NAMES: [&str; 7] = ["g1", "g2", "g6", "g7", "g8", "g9", "g10"];
