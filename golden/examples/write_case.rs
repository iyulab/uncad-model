//! Writes a golden case to disk, for looking at it or feeding it to a reader
//! by hand: `cargo run -p uncad-model-golden --example write_case -- <case> out.dxf`.
//! With a third argument, the expected model's JSON is written there too.
//!
//! `write_case --all <dir>` writes every case, `<dir>/<case>.dxf` and
//! `<dir>/<case>.expected.json` -- the files a reader's copies of the golden
//! cases are compared with -- in one run.

use uncad_model::ToJsonOptions;
use uncad_model_golden::{cases, expected, write, Spec};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.as_slice() {
        [_, flag, dir] if flag == "--all" => {
            for name in cases::NAMES {
                let spec = cases::by_name(name).expect("every listed case exists");
                let base = std::path::Path::new(dir).join(name);
                write_to(
                    &spec,
                    &base.with_extension("dxf"),
                    Some(&base.with_extension("expected.json")),
                );
            }
        }
        [_, case, out, rest @ ..] if !case.starts_with("--") => {
            let Some(spec) = cases::by_name(case) else {
                eprintln!("unknown case {case:?}; known: {}", cases::NAMES.join(", "));
                std::process::exit(2);
            };
            let json = rest.first().map(std::path::Path::new);
            write_to(&spec, std::path::Path::new(out), json);
        }
        _ => {
            eprintln!("usage: write_case <case> <out.dxf> [expected.json]");
            eprintln!("       write_case --all <dir>");
            std::process::exit(2);
        }
    }
}

fn write_to(spec: &Spec, dxf: &std::path::Path, json: Option<&std::path::Path>) {
    let written = write(spec);
    std::fs::write(dxf, &written.dxf).expect("write the DXF");
    if let Some(json_path) = json {
        let json = expected::model(spec, &written)
            .to_json(ToJsonOptions { pretty: true })
            .expect("serialize");
        std::fs::write(json_path, json).expect("write the JSON");
    }
}
