//! The named golden cases. Each is a function returning its spec, so a
//! consumer can pick the ones its role is measured by.

use crate::spec::{
    AttribSpec, BlockSpec, Codepage, DimStyleSpec, EntitySpec, LayerSpec, LayerState, Spec, Xy,
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
            Xy::new(0.0, 0.0),
            Xy::new(200.0, 0.0),
            Xy::new(200.0, 100.0),
            Xy::new(0.0, 100.0),
        ],
        closed: true,
        bulges: Vec::new(),
        widths: Vec::new(),
        const_width: 0.0,
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
                invisible: false,
            },
            AttribSpec {
                tag: "REV".to_string(),
                value: "B".to_string(),
                insert: Xy::new(125.0, -52.0),
                height: 3.5,
                invisible: false,
            },
            AttribSpec {
                tag: "MATERIAL".to_string(),
                value: "SS400".to_string(),
                insert: Xy::new(125.0, -59.0),
                height: 3.5,
                invisible: false,
            },
        ],
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
                        Xy::new(0.0, 0.0),
                        Xy::new(80.0, 0.0),
                        Xy::new(80.0, 20.0),
                        Xy::new(0.0, 20.0),
                    ],
                    closed: true,
                    bulges: Vec::new(),
                    widths: Vec::new(),
                    const_width: 0.0,
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
            invisible: false,
        }],
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
            },
        ],
        text_styles: Vec::new(),
        paper_space: Vec::new(),
        layouts: Vec::new(),
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
            bulges: Vec::new(),
            widths: Vec::new(),
            const_width: 0.0,
        },
        text(102.0, -45.0, "DWG NO"),
        text(130.0, -45.0, "BP-1042"),
        text(102.0, -52.0, "REV"),
        text(130.0, -52.0, "B"),
        text(102.0, -59.0, "MATERIAL"),
        text(130.0, -59.0, "SS400"),
    ];
    Spec {
        dim_styles: Vec::new(),
        codepage: Codepage::Ascii,
        layers: vec![LayerSpec {
            name: title,
            color_index: 2,
            state: LayerState::default(),
        }],
        blocks: Vec::new(),
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
                        Xy::new(0.0, 0.0),
                        Xy::new(80.0, 0.0),
                        Xy::new(80.0, 20.0),
                        Xy::new(0.0, 20.0),
                    ],
                    closed: true,
                    bulges: Vec::new(),
                    widths: Vec::new(),
                    const_width: 0.0,
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
                bulges: Vec::new(),
                widths: Vec::new(),
                const_width: 0.0,
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
                        invisible: false,
                    },
                    AttribSpec {
                        tag: "MATERIAL".to_string(),
                        value: material.to_string(),
                        insert: Xy::new(125.0, -52.0),
                        height: 3.5,
                        invisible: false,
                    },
                    AttribSpec {
                        tag: "DRAWN".to_string(),
                        value: drawn_by.to_string(),
                        insert: Xy::new(125.0, -59.0),
                        height: 3.5,
                        invisible: false,
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
/// OCS whose normal is (0, 0, -1), which is what AutoCAD's MIRROR leaves
/// behind, next to one circle in the world's own axes.
///
/// The case is about what a reader must *not* do: the coordinates are the
/// ones the file states, so the mirrored circle stated at (30, 20) is
/// carried at (30, 20) with its normal, not at the (-30, 20) it lies at in
/// the world; the polyline's bulge keeps its stated sign. Taking them to
/// the world is a consumer's step -- except for the block reference, whose
/// placement ([`uncad_model::Affine2::from_insert`]) the model computes.
pub fn g11_mirrored_part() -> Spec {
    let layer = "MIRROR".to_string();
    let mirrored = |elevation: f64, entity: EntitySpec| EntitySpec::Mirrored {
        elevation,
        entity: Box::new(entity),
    };
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
            },
            mirrored(
                0.0,
                EntitySpec::Circle {
                    layer: layer.clone(),
                    center: Xy::new(30.0, 20.0),
                    radius: 5.0,
                },
            ),
            mirrored(
                0.0,
                EntitySpec::Arc {
                    layer: layer.clone(),
                    center: Xy::new(30.0, 50.0),
                    radius: 10.0,
                    start_deg: 0.0,
                    end_deg: 90.0,
                },
            ),
            // A rectangle whose right side bulges out into a half circle,
            // at an elevation: the bulge (1) keeps its sign and the
            // elevation its value, as stated.
            mirrored(
                2.5,
                EntitySpec::LwPolyline {
                    layer: layer.clone(),
                    vertices: vec![
                        Xy::new(20.0, 0.0),
                        Xy::new(40.0, 0.0),
                        Xy::new(40.0, 10.0),
                        Xy::new(20.0, 10.0),
                    ],
                    closed: true,
                    bulges: vec![0.0, 1.0, 0.0, 0.0],
                    widths: Vec::new(),
                    const_width: 0.0,
                },
            ),
            mirrored(
                0.0,
                EntitySpec::Text {
                    layer: layer.clone(),
                    insert: Xy::new(20.0, -10.0),
                    height: 2.5,
                    text: "MIRRORED".to_string(),
                    rotation_deg: 0.0,
                },
            ),
            mirrored(
                0.0,
                EntitySpec::Solid {
                    layer: layer.clone(),
                    corners: [
                        Xy::new(20.0, -30.0),
                        Xy::new(30.0, -30.0),
                        Xy::new(20.0, -20.0),
                        Xy::new(30.0, -20.0),
                    ],
                },
            ),
            // Placed at (50, 0) in the mirrored OCS and turned 30 degrees
            // there: in the world the block's line runs from (-50, 0)
            // towards the upper left.
            mirrored(
                0.0,
                EntitySpec::Insert {
                    layer,
                    block: "MARK".to_string(),
                    insert: Xy::new(50.0, 0.0),
                    scale: 1.0,
                    rotation_deg: 30.0,
                    attribs: Vec::new(),
                },
            ),
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
        _ => return None,
    })
}

/// The names [`by_name`] knows, in order.
///
/// [`g3_many_parts`] is deliberately absent: it takes a size, and its
/// fixture would be checked-in megabytes whose exact bytes answer no
/// question the case asks.
pub const NAMES: [&str; 9] = ["g1", "g2", "g5", "g6", "g7", "g8", "g9", "g10", "g11"];
