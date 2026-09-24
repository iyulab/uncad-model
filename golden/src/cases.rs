//! The named golden cases. Each is a function returning its spec, so a
//! consumer can pick the ones its role is measured by.

use crate::spec::{
    AttribSpec, BlockSpec, Codepage, DimStyleSpec, EntitySpec, HatchEdgeSpec, HatchPathSpec,
    HatchShapeSpec, LayerSpec, LayerState, LayoutSpec, LineStyleSpec, Spec, TextAlign, Vertex, Xy,
};
use crate::writer::{horizontal_code, vertical_code};
use uncad_model::model::{
    HatchStyle, HorizontalJustification, OrdinateAxis, VerticalJustification,
};
use uncad_model::tables::{
    AngularUnitFormat, ArcSymbol, FractionFormat, LinearUnitFormat, PlotPaperUnits, PlotRotation,
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
        const_width: 0.0,
        elevation: 0.0,
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
                invisible: false,
            },
            AttribSpec {
                tag: "REV".to_string(),
                value: "B".to_string(),
                insert: Xy::new(125.0, -52.0),
                height: 3.5,
                align: None,
                width_factor: 1.0,
                invisible: false,
            },
            AttribSpec {
                tag: "MATERIAL".to_string(),
                value: "SS400".to_string(),
                insert: Xy::new(125.0, -59.0),
                height: 3.5,
                align: None,
                width_factor: 1.0,
                invisible: false,
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
                state: LayerState::default(),
            },
            LayerSpec {
                name: holes,
                color_index: 1,
                state: LayerState::default(),
            },
            LayerSpec {
                name: dims,
                color_index: 3,
                state: LayerState::default(),
            },
            LayerSpec {
                name: title.clone(),
                color_index: 2,
                state: LayerState::default(),
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
                    const_width: 0.0,
                    elevation: 0.0,
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
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
            invisible: false,
        }],
        mirrored: false,
    };
    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: "TITLE".to_string(),
            color_index: 2,
            state: LayerState::default(),
        }],
        blocks: vec![title_block],
        entities: vec![insert(0.0, "BP-1042"), insert(120.0, "BP-2077")],
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
            state: LayerState::default(),
        }],
        blocks: Vec::new(),
        dim_styles: vec![DimStyleSpec {
            name: "ISO-25".to_string(),
            post: Some("<>mm".to_string()),
            decimal_places: Some(2),
            text_height: Some(2.5),
            // Not the default, so a reader that ignores the group fails.
            arc_symbol: Some(ArcSymbol::AboveText),
            ..DimStyleSpec::default()
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
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
        oblique_deg: 0.0,
        style: None,
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
            const_width: 0.0,
            elevation: 0.0,
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
            const_width: 0.0,
            elevation: 0.0,
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
            oblique_deg: 0.0,
            style: None,
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
            oblique_deg: 0.0,
            style: None,
        },
    ];
    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: title,
            color_index: 2,
            state: LayerState::default(),
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
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
                state: LayerState::default(),
            },
            LayerSpec {
                name: title_layer.clone(),
                color_index: 2,
                state: LayerState::default(),
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
                    const_width: 0.0,
                    elevation: 0.0,
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
                const_width: 0.0,
                elevation: 0.0,
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
                        invisible: false,
                    },
                    AttribSpec {
                        tag: "MATERIAL".to_string(),
                        value: material.to_string(),
                        insert: Xy::new(125.0, -52.0),
                        height: 3.5,
                        align: None,
                        width_factor: 1.0,
                        invisible: false,
                    },
                    AttribSpec {
                        tag: "DRAWN".to_string(),
                        value: drawn_by.to_string(),
                        insert: Xy::new(125.0, -59.0),
                        height: 3.5,
                        align: None,
                        width_factor: 1.0,
                        invisible: false,
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
                oblique_deg: 0.0,
                style: None,
            },
        ],
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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

/// G11, a mirrored part: the kinds the format states in an object
/// coordinate system -- a circle, an arc, a bulged polyline at an
/// elevation, a text, a solid and a block reference -- each written in the
/// coordinate system whose Z axis (the extrusion, DXF 210) is (0, 0, -1),
/// which is what AutoCAD's MIRROR leaves behind, next to one circle in the
/// world's own axes.
///
/// The case is about what a reader must *not* do: the coordinates are the
/// ones the file states, so the mirrored circle stated at (30, 20) is
/// carried at (30, 20) with its extrusion, not at the (-30, 20) it lies at
/// in the world; the polyline's bulge keeps its stated sign. Taking them to
/// the world is a consumer's step -- except for the block reference, whose
/// placement ([`uncad_model::model::InsertEntity::world_transform`]) the model
/// computes.
pub fn g11_mirrored_part() -> Spec {
    let layer = "MIRROR".to_string();
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: layer.clone(),
            color_index: 4,
            state: LayerState::default(),
        }],
        blocks: vec![BlockSpec {
            name: "MARK".to_string(),
            entities: vec![
                EntitySpec::Line {
                    layer: "0".to_string(),
                    start: Xy::new(0.0, 0.0),
                    end: Xy::new(10.0, 0.0),
                },
                EntitySpec::Circle {
                    layer: "0".to_string(),
                    center: Xy::new(10.0, 0.0),
                    radius: 2.0,
                    mirrored: false,
                },
            ],
        }],
        dim_styles: Vec::new(),
        text_styles: Vec::new(),
        entities: vec![
            EntitySpec::Circle {
                layer: layer.clone(),
                center: Xy::new(30.0, 20.0),
                radius: 5.0,
                mirrored: false,
            },
            EntitySpec::Circle {
                layer: layer.clone(),
                center: Xy::new(30.0, 20.0),
                radius: 5.0,
                mirrored: true,
            },
            EntitySpec::Arc {
                layer: layer.clone(),
                center: Xy::new(30.0, 50.0),
                radius: 10.0,
                start_deg: 0.0,
                end_deg: 90.0,
                mirrored: true,
            },
            // A rectangle whose right side bulges out into a half circle,
            // at an elevation: the bulge (1) keeps its sign and the
            // elevation its value, as stated.
            EntitySpec::LwPolyline {
                layer: layer.clone(),
                vertices: vec![
                    Xy::new(20.0, 0.0).into(),
                    Vertex::bulged(Xy::new(40.0, 0.0), 1.0),
                    Xy::new(40.0, 10.0).into(),
                    Xy::new(20.0, 10.0).into(),
                ],
                closed: true,
                const_width: 0.0,
                elevation: 2.5,
                mirrored: true,
            },
            EntitySpec::Text {
                layer: layer.clone(),
                insert: Xy::new(20.0, -10.0),
                height: 2.5,
                text: "MIRRORED".to_string(),
                rotation_deg: 0.0,
                mirrored: true,
                align: None,
                width_factor: 1.0,
                oblique_deg: 0.0,
                style: None,
            },
            EntitySpec::Solid {
                layer: layer.clone(),
                corners: [
                    Xy::new(20.0, -30.0),
                    Xy::new(30.0, -30.0),
                    Xy::new(20.0, -20.0),
                    Xy::new(30.0, -20.0),
                ],
                mirrored: true,
            },
            // Placed at (50, 0) in the mirrored coordinate system and turned
            // 30 degrees there: in the world the block's line runs from
            // (-50, 0) towards the upper left.
            EntitySpec::Insert {
                layer,
                block: "MARK".to_string(),
                insert: Xy::new(50.0, 0.0),
                scale: 1.0,
                rotation_deg: 30.0,
                attribs: Vec::new(),
                mirrored: true,
            },
        ],
        paper_space: Vec::new(),
        layouts: Vec::new(),
    }
}

/// G12, polylines that are not just their vertices: a slot whose two ends
/// are half circles (bulges of 1), a tapered arrow (per-vertex widths), a
/// constant-width polyline with a quarter-circle corner, a DONUT (two
/// vertices, both bulges 1, a constant width), and a polyline whose file
/// states, on every vertex, widths equal to its constant width.
///
/// The last one is the same polyline as one that states the constant width
/// alone: a reader gives its vertices no width of their own, so that two
/// spellings of one drawing compare equal.
pub fn g12_curved_and_wide_polylines() -> Spec {
    let layer = "OUTLINE".to_string();
    // tan(22.5 degrees): the bulge of a quarter circle, negative for one
    // that turns clockwise.
    let quarter = -0.41421356237309503;
    let polyline = |vertices: Vec<Vertex>, closed: bool, const_width: f64| EntitySpec::LwPolyline {
        layer: layer.clone(),
        vertices,
        closed,
        const_width,
        elevation: 0.0,
        mirrored: false,
    };
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: layer.clone(),
            color_index: 7,
            state: LayerState::default(),
        }],
        blocks: Vec::new(),
        dim_styles: Vec::new(),
        text_styles: Vec::new(),
        entities: vec![
            polyline(
                vec![
                    Xy::new(0.0, 0.0).into(),
                    Vertex::bulged(Xy::new(40.0, 0.0), 1.0),
                    Xy::new(40.0, 10.0).into(),
                    Vertex::bulged(Xy::new(0.0, 10.0), 1.0),
                ],
                true,
                0.0,
            ),
            polyline(
                vec![
                    Vertex::from(Xy::new(0.0, 30.0)).wide(2.0, 2.0),
                    Vertex::from(Xy::new(30.0, 30.0)).wide(4.0, 0.0),
                    Xy::new(40.0, 30.0).into(),
                ],
                false,
                0.0,
            ),
            polyline(
                vec![
                    Xy::new(0.0, 50.0).into(),
                    Vertex::bulged(Xy::new(40.0, 50.0), quarter),
                    Xy::new(50.0, 60.0).into(),
                ],
                false,
                1.5,
            ),
            polyline(
                vec![
                    Vertex::bulged(Xy::new(60.0, 20.0), 1.0),
                    Vertex::bulged(Xy::new(70.0, 20.0), 1.0),
                ],
                true,
                2.0,
            ),
            polyline(
                vec![
                    Vertex::from(Xy::new(0.0, 90.0)).wide(1.0, 1.0),
                    Vertex::from(Xy::new(40.0, 90.0)).wide(1.0, 1.0),
                ],
                false,
                1.0,
            ),
        ],
        paper_space: Vec::new(),
        layouts: Vec::new(),
    }
}

/// G13, text placed every way the format places it: each horizontal and
/// vertical justification the reference defines (the alignment point is
/// then what places the text), a width factor, an oblique angle, a named
/// text style, a style the file never declares, and a text that names no
/// style -- which the reference reads as `STANDARD`, declared here -- and a
/// title block with an invisible attribute next to a visible one.
pub fn g13_justified_text() -> Spec {
    let layer = "TEXT".to_string();
    let justified = |insert: Xy,
                     alignment: Xy,
                     horizontal: HorizontalJustification,
                     vertical: VerticalJustification,
                     text: &str,
                     width_factor: f64,
                     oblique_deg: f64,
                     style: Option<&str>| EntitySpec::Text {
        layer: layer.clone(),
        insert,
        height: 2.5,
        text: text.to_string(),
        rotation_deg: 0.0,
        align: (horizontal != HorizontalJustification::Left
            || vertical != VerticalJustification::Baseline)
            .then_some(TextAlign {
                horizontal: horizontal_code(horizontal),
                vertical: vertical_code(vertical),
                at: alignment,
            }),
        width_factor,
        oblique_deg,
        style: style.map(str::to_string),
        mirrored: false,
    };
    use HorizontalJustification as H;
    use VerticalJustification as V;
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: layer.clone(),
            color_index: 2,
            state: LayerState::default(),
        }],
        blocks: vec![BlockSpec {
            name: "TAG".to_string(),
            entities: vec![
                EntitySpec::Attdef {
                    layer: "0".to_string(),
                    insert: Xy::new(0.0, 0.0),
                    height: 2.5,
                    tag: "NUMBER".to_string(),
                    prompt: "Number".to_string(),
                    default: "-".to_string(),
                },
                EntitySpec::Attdef {
                    layer: "0".to_string(),
                    insert: Xy::new(0.0, -4.0),
                    height: 2.5,
                    tag: "NOTE".to_string(),
                    prompt: "Note".to_string(),
                    default: "-".to_string(),
                },
            ],
        }],
        dim_styles: Vec::new(),
        text_styles: vec!["STANDARD".to_string(), "ROMANS".to_string()],
        entities: vec![
            EntitySpec::Text {
                layer: layer.clone(),
                insert: Xy::new(0.0, 0.0),
                height: 2.5,
                text: "LEFT".to_string(),
                rotation_deg: 0.0,
                mirrored: false,
                align: None,
                width_factor: 1.0,
                oblique_deg: 0.0,
                style: None,
            },
            justified(
                Xy::new(43.0, 0.0),
                Xy::new(50.0, 0.0),
                H::Center,
                V::Baseline,
                "CENTER",
                1.0,
                0.0,
                Some("ROMANS"),
            ),
            justified(
                Xy::new(88.0, -2.5),
                Xy::new(100.0, 0.0),
                H::Right,
                V::Top,
                "RIGHT TOP",
                1.0,
                0.0,
                None,
            ),
            justified(
                Xy::new(44.0, 18.75),
                Xy::new(50.0, 20.0),
                H::Middle,
                V::Baseline,
                "MIDDLE",
                0.8,
                0.0,
                Some("ROMANS"),
            ),
            justified(
                Xy::new(0.0, 40.0),
                Xy::new(60.0, 40.0),
                H::Aligned,
                V::Baseline,
                "ALIGNED",
                1.0,
                15.0,
                None,
            ),
            justified(
                Xy::new(0.0, 60.0),
                Xy::new(60.0, 60.0),
                H::Fit,
                V::Baseline,
                "FIT",
                2.5,
                0.0,
                None,
            ),
            justified(
                Xy::new(0.0, 79.0),
                Xy::new(0.0, 80.0),
                H::Left,
                V::Bottom,
                "LEFT BOTTOM",
                1.0,
                0.0,
                Some("GOST"),
            ),
            EntitySpec::Insert {
                layer: layer.clone(),
                block: "TAG".to_string(),
                insert: Xy::new(120.0, 0.0),
                scale: 1.0,
                rotation_deg: 0.0,
                attribs: vec![
                    AttribSpec {
                        tag: "NUMBER".to_string(),
                        value: "D-101".to_string(),
                        insert: Xy::new(120.0, 0.0),
                        height: 2.5,
                        invisible: false,
                        align: None,
                        width_factor: 1.0,
                    },
                    AttribSpec {
                        tag: "NOTE".to_string(),
                        value: "FIRE RATED".to_string(),
                        insert: Xy::new(120.0, -4.0),
                        height: 2.5,
                        invisible: true,
                        align: None,
                        width_factor: 1.0,
                    },
                ],
                mirrored: false,
            },
        ],
        paper_space: Vec::new(),
        layouts: Vec::new(),
    }
}

/// G14, a sheet: layers in every state the format gives one (off, frozen,
/// locked, plotted and not, with and without a lineweight), a model drawn
/// on them, and a paper-space sheet showing it -- a border, a title, the
/// sheet's own overall viewport, a detail viewport at half scale, turned
/// 30 degrees and with a layer frozen in it alone, and a viewport that is
/// off -- set up as two layouts with their plot settings besides the model
/// tab.
///
/// The detail viewport shows the model around (100, 50) in its view's own
/// coordinates, measured from the target (10, 5): 300 drawing units of
/// height in a 150 mm frame.
pub fn g14_sheet_with_viewports() -> Spec {
    let state = |f: fn(&mut LayerState)| {
        let mut s = LayerState::default();
        f(&mut s);
        s
    };
    let layer = |name: &str, color_index: i16, state: LayerState| LayerSpec {
        name: name.to_string(),
        color_index,
        state,
    };
    let line = |layer: &str, y: f64| EntitySpec::Line {
        layer: layer.to_string(),
        start: Xy::new(0.0, y),
        end: Xy::new(200.0, y),
    };
    let viewport = |center: Xy,
                    size: (f64, f64),
                    on: bool,
                    id: i32,
                    view: (Xy, f64, Xy, f64),
                    frozen_layers: &[&str]| EntitySpec::Viewport {
        layer: "0".to_string(),
        center,
        width: size.0,
        height: size.1,
        on,
        id,
        view_center: view.0,
        view_height: view.1,
        view_target: view.2,
        twist_deg: view.3,
        frozen_layers: frozen_layers.iter().map(|s| s.to_string()).collect(),
    };
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![
            layer("WALLS", 7, LayerState::default()),
            layer("HIDDEN", 1, state(|s| s.off = true)),
            layer("FROZEN", 3, state(|s| s.frozen = true)),
            layer("LOCKED", 4, state(|s| s.locked = true)),
            layer("NOPLOT", 5, state(|s| s.plot = Some(false))),
            layer(
                "PLOT",
                6,
                state(|s| {
                    s.plot = Some(true);
                    s.lineweight = Some(50);
                }),
            ),
            layer("DEFAULTWT", 8, state(|s| s.lineweight = Some(-3))),
        ],
        blocks: Vec::new(),
        dim_styles: Vec::new(),
        text_styles: Vec::new(),
        entities: vec![
            line("WALLS", 0.0),
            line("HIDDEN", 10.0),
            line("FROZEN", 20.0),
            line("LOCKED", 30.0),
            line("NOPLOT", 40.0),
            line("PLOT", 50.0),
            line("DEFAULTWT", 60.0),
        ],
        paper_space: vec![
            EntitySpec::LwPolyline {
                layer: "0".to_string(),
                vertices: vec![
                    Xy::new(0.0, 0.0).into(),
                    Xy::new(420.0, 0.0).into(),
                    Xy::new(420.0, 297.0).into(),
                    Xy::new(0.0, 297.0).into(),
                ],
                closed: true,
                const_width: 0.0,
                elevation: 0.0,
                mirrored: false,
            },
            EntitySpec::Text {
                layer: "0".to_string(),
                insert: Xy::new(320.0, 20.0),
                height: 5.0,
                text: "SHEET 1".to_string(),
                rotation_deg: 0.0,
                mirrored: false,
                align: None,
                width_factor: 1.0,
                oblique_deg: 0.0,
                style: None,
            },
            viewport(
                Xy::new(210.0, 148.5),
                (420.0, 297.0),
                true,
                1,
                (Xy::new(210.0, 148.5), 297.0, Xy::new(0.0, 0.0), 0.0),
                &[],
            ),
            viewport(
                Xy::new(150.0, 150.0),
                (200.0, 150.0),
                true,
                2,
                (Xy::new(100.0, 50.0), 300.0, Xy::new(10.0, 5.0), 30.0),
                &["WALLS"],
            ),
            viewport(
                Xy::new(350.0, 60.0),
                (100.0, 80.0),
                false,
                3,
                (Xy::new(0.0, 0.0), 80.0, Xy::new(0.0, 0.0), 0.0),
                &["FROZEN", "NOPLOT"],
            ),
        ],
        layouts: vec![
            LayoutSpec {
                name: "Model".to_string(),
                tab_order: 0,
                block: "*Model_Space".to_string(),
                limits_min: Xy::new(0.0, 0.0),
                limits_max: Xy::new(420.0, 297.0),
                paper_name: String::new(),
                paper_size: (0.0, 0.0),
                margins: [0.0; 4],
                plot_origin: Xy::new(0.0, 0.0),
                paper_units: PlotPaperUnits::Millimeters,
                rotation: PlotRotation::Unrotated,
                scale: (1.0, 1.0),
                paper_space_linetype_scaling: true,
                limits_check: false,
                extents: (Xy::new(-5.0, -5.0), Xy::new(405.0, 290.0)),
                // A model layout names a VPORT record here, never an entity.
                active_viewport: None,
            },
            // ISO A3 is stated portrait (297 x 420) and turned a quarter,
            // with the usual "origin at the paper's corner" page setup: the
            // plot origin is minus the left and bottom margins.
            LayoutSpec {
                name: "Layout1".to_string(),
                tab_order: 1,
                block: "*Paper_Space".to_string(),
                limits_min: Xy::new(0.0, 0.0),
                limits_max: Xy::new(420.0, 297.0),
                paper_name: "ISO_A3_(420.00_x_297.00_MM)".to_string(),
                paper_size: (297.0, 420.0),
                margins: [7.5, 20.0, 7.5, 20.0],
                plot_origin: Xy::new(-7.5, -20.0),
                paper_units: PlotPaperUnits::Millimeters,
                rotation: PlotRotation::Counterclockwise90,
                scale: (1.0, 1.0),
                paper_space_linetype_scaling: true,
                limits_check: true,
                extents: (Xy::new(0.0, 0.0), Xy::new(420.0, 297.0)),
                // The second viewport: the one someone last worked in.
                active_viewport: Some(3),
            },
            // A second sheet, empty, in inches: its block is one more paper
            // space the file declares.
            LayoutSpec {
                name: "Layout2".to_string(),
                tab_order: 2,
                block: "*Paper_Space0".to_string(),
                limits_min: Xy::new(0.0, 0.0),
                limits_max: Xy::new(11.0, 8.5),
                paper_name: "ANSI_A_(8.50_x_11.00_Inches)".to_string(),
                paper_size: (215.9, 279.4),
                margins: [6.35, 19.05, 6.35, 19.05],
                plot_origin: Xy::new(0.0, 0.0),
                paper_units: PlotPaperUnits::Inches,
                rotation: PlotRotation::Clockwise90,
                scale: (1.0, 1.0),
                // Never activated: no viewport, and the reversed extents an
                // application writes for a space it has not measured.
                paper_space_linetype_scaling: false,
                limits_check: false,
                extents: (Xy::new(1e20, 1e20), Xy::new(-1e20, -1e20)),
                active_viewport: None,
            },
        ],
    }
}

/// G19, [`g14_sheet_with_viewports`] as a DXF writes a layout that is not
/// the current one: from R2000 on, group 69 of every one of its viewports is
/// 0 rather than the viewport's number. The model carries the 0 as written;
/// a consumer that takes it for a number finds no overall viewport, and the
/// sheet itself -- whose view covers the lines of the model -- becomes a
/// window drawing the model over the whole paper. What tells the overall
/// viewport then is its view: its own frame, untwisted.
pub fn g19_sheet_of_a_layout_not_current() -> Spec {
    let mut spec = g14_sheet_with_viewports();
    for entity in &mut spec.paper_space {
        if let EntitySpec::Viewport { id, .. } = entity {
            *id = 0;
        }
    }
    spec
}

/// G15, a polygon mesh: three rows of four vertices, closed in N (each row
/// wraps back to its first vertex, as a tube does) and open in M, its
/// heights varying like a patch of terrain. The model carries it as its
/// grid lines, in the order the model states for a mesh: 8 edges between
/// the rows (two gaps, four columns), then 12 along them (three rows of
/// four, each closed).
pub fn g15_polygon_mesh() -> Spec {
    let layer = "MESH".to_string();
    let mut vertices = Vec::new();
    for (i, z) in [
        [0.0, 1.0, 2.0, 1.0],
        [0.5, 1.5, 2.5, 1.5],
        [1.0, 2.0, 3.0, 2.0],
    ]
    .iter()
    .enumerate()
    {
        for (j, z) in z.iter().enumerate() {
            vertices.push([10.0 * j as f64, 10.0 * i as f64, *z]);
        }
    }
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: layer.clone(),
            color_index: 5,
            state: LayerState::default(),
        }],
        blocks: Vec::new(),
        dim_styles: Vec::new(),
        text_styles: Vec::new(),
        entities: vec![EntitySpec::PolygonMesh {
            layer,
            m: 3,
            n: 4,
            closed_m: false,
            closed_n: true,
            vertices,
        }],
        paper_space: Vec::new(),
        layouts: Vec::new(),
    }
}

/// G16, ordinate dimensions and a dimension style that states every
/// variable the displayed text depends on. A plate's features are
/// dimensioned from its lower-left corner: three X-type ordinates (the
/// feature's x distance from the datum) along the bottom and two Y-type
/// ones along the left edge. One states its measurement, one states a
/// literal text, the rest state neither -- and a second style states only
/// its unit formats, leaving everything else unsaid.
pub fn g16_ordinate_dimensions() -> Spec {
    let dims = "DIMS".to_string();
    let ordinate = |feature: Xy,
                    leader_end: Xy,
                    axis: OrdinateAxis,
                    text: &str,
                    measurement: Option<f64>,
                    style: &str| EntitySpec::OrdinateDimension {
        layer: dims.clone(),
        datum: Xy::new(0.0, 0.0),
        feature,
        leader_end,
        axis,
        text: text.to_string(),
        measurement,
        style: Some(style.to_string()),
    };
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: dims.clone(),
            color_index: 3,
            state: LayerState::default(),
        }],
        blocks: Vec::new(),
        dim_styles: vec![
            DimStyleSpec {
                name: "ORD".to_string(),
                post: Some("<>".to_string()),
                decimal_places: Some(2),
                text_height: Some(2.5),
                arrow_size: Some(2.5),
                linear_unit_format: Some(LinearUnitFormat::Decimal),
                zero_suppression: Some(8),
                rounding: Some(0.5),
                angular_unit_format: Some(AngularUnitFormat::DegreesMinutesSeconds),
                angular_decimal_places: Some(1),
                fraction_format: Some(FractionFormat::NotStacked),
                arc_symbol: Some(ArcSymbol::Suppressed),
            },
            DimStyleSpec {
                name: "ARCH".to_string(),
                linear_unit_format: Some(LinearUnitFormat::Architectural),
                fraction_format: Some(FractionFormat::Diagonal),
                ..DimStyleSpec::default()
            },
        ],
        text_styles: Vec::new(),
        entities: vec![
            EntitySpec::LwPolyline {
                layer: "0".to_string(),
                vertices: vec![
                    Xy::new(0.0, 0.0).into(),
                    Xy::new(120.0, 0.0).into(),
                    Xy::new(120.0, 60.0).into(),
                    Xy::new(0.0, 60.0).into(),
                ],
                closed: true,
                const_width: 0.0,
                elevation: 0.0,
                mirrored: false,
            },
            ordinate(
                Xy::new(0.0, 0.0),
                Xy::new(0.0, -15.0),
                OrdinateAxis::X,
                "<>",
                None,
                "ORD",
            ),
            ordinate(
                Xy::new(30.0, 20.0),
                Xy::new(30.0, -15.0),
                OrdinateAxis::X,
                "",
                Some(30.0),
                "ORD",
            ),
            ordinate(
                Xy::new(120.0, 0.0),
                Xy::new(120.0, -15.0),
                OrdinateAxis::X,
                "120.00",
                None,
                "ARCH",
            ),
            ordinate(
                Xy::new(30.0, 20.0),
                Xy::new(-15.0, 20.0),
                OrdinateAxis::Y,
                "<>",
                None,
                "ORD",
            ),
            ordinate(
                Xy::new(0.0, 60.0),
                Xy::new(-15.0, 60.0),
                OrdinateAxis::Y,
                "<>",
                None,
                "ORD",
            ),
        ],
        paper_space: Vec::new(),
        layouts: Vec::new(),
    }
}

/// G17: solid-fill HATCHes whose boundaries are edge paths -- every edge
/// kind, a rational and a non-rational spline among them -- and a polyline
/// island with a bulge. The corpus has no spline edge at all, so this case
/// is what tells a reader's spline fields apart from zeros.
///
/// Both hatches are associative: an edge path ending in a spline edge names
/// the polyline it was picked from (97 = 1, then a 330). In an R2000 file a
/// spline edge carries no fit data, so that 97 belongs to the path -- a
/// reader that takes it for the edge's fit-point count loses its place, and
/// the second hatch has another path after it to lose.
pub fn g17_hatch_edge_paths() -> Spec {
    let hatch = "HATCH".to_string();
    let xy = Xy::new;
    Spec {
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: hatch.clone(),
            color_index: 4,
            state: LayerState::default(),
        }],
        blocks: Vec::new(),
        dim_styles: Vec::new(),
        text_styles: Vec::new(),
        entities: vec![
            // The outline both hatches name as their source.
            EntitySpec::LwPolyline {
                layer: "0".to_string(),
                vertices: vec![
                    xy(0.0, 0.0).into(),
                    Vertex::bulged(xy(40.0, 0.0), 1.0),
                    xy(40.0, 20.0).into(),
                    xy(0.0, 20.0).into(),
                ],
                closed: true,
                const_width: 0.0,
                elevation: 0.0,
                mirrored: false,
            },
            EntitySpec::Hatch {
                layer: hatch.clone(),
                paths: vec![HatchPathSpec {
                    shape: HatchShapeSpec::Edges(vec![
                        HatchEdgeSpec::Line {
                            start: xy(0.0, 0.0),
                            end: xy(40.0, 0.0),
                        },
                        HatchEdgeSpec::Arc {
                            center: xy(40.0, 10.0),
                            radius: 10.0,
                            start_deg: 270.0,
                            end_deg: 90.0,
                            ccw: true,
                        },
                        HatchEdgeSpec::Line {
                            start: xy(40.0, 20.0),
                            end: xy(0.0, 20.0),
                        },
                        HatchEdgeSpec::Spline {
                            degree: 2,
                            rational: true,
                            periodic: false,
                            knots: vec![0.0, 0.0, 0.0, 1.0, 1.0, 1.0],
                            control_points: vec![xy(0.0, 20.0), xy(-8.0, 10.0), xy(0.0, 0.0)],
                            weights: vec![1.0, 0.5, 1.0],
                        },
                    ]),
                    external: true,
                    sources: vec![0],
                }],
                style: HatchStyle::Normal,
            },
            EntitySpec::Hatch {
                layer: hatch,
                paths: vec![
                    HatchPathSpec {
                        shape: HatchShapeSpec::Edges(vec![
                            HatchEdgeSpec::Ellipse {
                                center: xy(80.0, 20.0),
                                major_end: xy(20.0, 0.0),
                                ratio: 0.25,
                                start_deg: 0.0,
                                end_deg: 180.0,
                                ccw: true,
                            },
                            HatchEdgeSpec::Arc {
                                center: xy(60.0, 10.0),
                                radius: 10.0,
                                start_deg: 270.0,
                                end_deg: 90.0,
                                ccw: false,
                            },
                            HatchEdgeSpec::Line {
                                start: xy(60.0, 0.0),
                                end: xy(100.0, 0.0),
                            },
                            HatchEdgeSpec::Spline {
                                degree: 3,
                                rational: false,
                                periodic: false,
                                knots: vec![0.0, 0.0, 0.0, 0.0, 0.5, 1.0, 1.0, 1.0, 1.0],
                                control_points: vec![
                                    xy(100.0, 0.0),
                                    xy(110.0, 5.0),
                                    xy(105.0, 12.0),
                                    xy(112.0, 18.0),
                                    xy(100.0, 20.0),
                                ],
                                weights: Vec::new(),
                            },
                        ]),
                        external: true,
                        sources: vec![0],
                    },
                    HatchPathSpec {
                        shape: HatchShapeSpec::Polyline(vec![
                            xy(75.0, 5.0).into(),
                            Vertex::bulged(xy(85.0, 5.0), 0.5),
                            xy(85.0, 15.0).into(),
                            xy(75.0, 15.0).into(),
                        ]),
                        external: false,
                        sources: Vec::new(),
                    },
                ],
                style: HatchStyle::Outer,
            },
        ],
        paper_space: Vec::new(),
        layouts: Vec::new(),
    }
}

/// G18: lines that state their own linetype, linetype scale and lineweight
/// -- BYBLOCK, the declared CONTINUOUS with a scale and a weight, and a
/// linetype the file never declares -- beside one that states nothing and
/// takes all three from its layer. An R2000 file: no transparency.
pub fn g18_line_styles() -> Spec {
    let line = |y: f64| EntitySpec::Line {
        layer: "0".to_string(),
        start: Xy::new(0.0, y),
        end: Xy::new(50.0, y),
    };
    let styled = |y: f64, linetype: &str, scale: Option<f64>, weight: i16| EntitySpec::Styled {
        style: LineStyleSpec {
            linetype: Some(linetype.to_string()),
            linetype_scale: scale,
            lineweight: Some(weight),
        },
        entity: Box::new(line(y)),
    };
    Spec {
        codepage: Codepage::Ascii,
        layers: Vec::new(),
        blocks: Vec::new(),
        dim_styles: Vec::new(),
        text_styles: Vec::new(),
        entities: vec![
            line(0.0),
            styled(10.0, "BYBLOCK", None, -2),
            styled(20.0, "CONTINUOUS", Some(2.5), 35),
            styled(30.0, "DASHED", None, -3),
        ],
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
        "g5" => g5_dense_dimensions(),
        "g10" => g10_unreferenced_insert(),
        "g11" => g11_mirrored_part(),
        "g12" => g12_curved_and_wide_polylines(),
        "g13" => g13_justified_text(),
        "g14" => g14_sheet_with_viewports(),
        "g15" => g15_polygon_mesh(),
        "g16" => g16_ordinate_dimensions(),
        "g17" => g17_hatch_edge_paths(),
        "g18" => g18_line_styles(),
        "g19" => g19_sheet_of_a_layout_not_current(),
        _ => return None,
    })
}

/// The names [`by_name`] knows, in order.
///
/// [`g3_many_parts`] is deliberately absent: it takes a size, and its
/// fixture would be checked-in megabytes whose exact bytes answer no
/// question the case asks.
pub const NAMES: [&str; 17] = [
    "g1", "g2", "g5", "g6", "g7", "g8", "g9", "g10", "g11", "g12", "g13", "g14", "g15", "g16",
    "g17", "g18", "g19",
];
