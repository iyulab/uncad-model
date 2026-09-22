//! Writes a golden case to disk, for looking at it or feeding it to a reader
//! by hand: `cargo run -p uncad-model-golden --example write_case -- g1 out.dxf`.
//! With a third argument, the expected model's JSON is written there too.

use uncad_model::ToJsonOptions;
use uncad_model_golden::{cases, expected, write};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let (case, out) = match args.as_slice() {
        [_, case, out, ..] => (case.as_str(), out.as_str()),
        _ => {
            eprintln!("usage: write_case <g1> <out.dxf> [expected.json]");
            std::process::exit(2);
        }
    };
    let spec = match case {
        "g1" => cases::g1_general_part(),
        other => {
            eprintln!("unknown case {other:?}; known: g1");
            std::process::exit(2);
        }
    };
    let written = write(&spec);
    std::fs::write(out, &written.dxf).expect("write the DXF");
    if let Some(json_path) = args.get(3) {
        let json = expected::model(&spec, &written)
            .to_json(ToJsonOptions { pretty: true })
            .expect("serialize");
        std::fs::write(json_path, json).expect("write the JSON");
    }
}
