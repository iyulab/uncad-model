//! Property checks over randomly generated specs.
//!
//! The negative cells of the coverage matrix ("must never happen") cannot be
//! measured by one case: what must not happen must not happen for *any*
//! input. So these are invariants over generated specs, shrunk to a minimal
//! failing case by `proptest` when they fail. Generation is seeded and
//! deterministic, so a failure is reproducible.
//!
//! P3, determinism: the same spec always writes the same bytes and always
//! states the same expected model, byte for byte in JSON.

use proptest::prelude::*;
use uncad_model::ToJsonOptions;
use uncad_model_golden::strategy::spec;
use uncad_model_golden::{expected, write};

proptest! {
    /// P3 for the writer: the same spec writes the same bytes, every time.
    #[test]
    fn writing_is_deterministic(spec in spec()) {
        let first = write(&spec);
        for _ in 0..8 {
            prop_assert_eq!(&write(&spec), &first);
        }
    }

    /// P3 for the oracle: the expected model serializes to the same bytes,
    /// every time, and round-trips through JSON.
    #[test]
    fn the_expected_model_is_deterministic_and_round_trips(spec in spec()) {
        let written = write(&spec);
        let model = expected::model(&spec, &written);
        let json = model.to_json(ToJsonOptions::default()).unwrap();
        for _ in 0..8 {
            let again = expected::model(&spec, &written).to_json(ToJsonOptions::default()).unwrap();
            prop_assert_eq!(&again, &json);
        }
        let back: uncad_model::CadDatabase = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(back, model);
    }

    /// Every handle the writer issues is unique, and every entity of the
    /// expected model carries one of them.
    #[test]
    fn handles_are_unique_and_accounted_for(spec in spec()) {
        let written = write(&spec);
        let mut all: Vec<u32> = written.handles.entities.clone();
        for (_, hs) in &written.handles.blocks {
            all.extend(hs);
        }
        for (_, hs) in &written.handles.attribs {
            all.extend(hs);
        }
        let mut sorted = all.clone();
        sorted.sort_unstable();
        sorted.dedup();
        prop_assert_eq!(sorted.len(), all.len(), "duplicate handle");

        let model = expected::model(&spec, &written);
        for e in &model.entities {
            let handle = e.common().source_handle.resolved().expect("a file entity has a handle");
            let h = u32::from_str_radix(handle, 16).unwrap();
            prop_assert!(all.contains(&h), "entity handle {:X} was never issued", h);
            prop_assert_eq!(e.common().id.value(), u64::from(h), "the ID is the handle's value");
        }
        // Each issued handle appears in the DXF text as a code-5 pair.
        let dxf = String::from_utf8_lossy(&written.dxf);
        for h in &all {
            let needle = format!("  5\n{h:X}\n");
            prop_assert!(dxf.contains(&needle), "handle {:X} not written", h);
        }
    }

    /// The DXF text is well formed at the level a reader checks first: it
    /// alternates (code, value) lines, opens and closes every section, and
    /// declares every layer an entity uses.
    #[test]
    fn the_dxf_is_well_formed(spec in spec()) {
        let written = write(&spec);
        let dxf = String::from_utf8(written.dxf).expect("an ASCII case is UTF-8");
        let lines: Vec<&str> = dxf.lines().collect();
        prop_assert_eq!(lines.len() % 2, 0, "odd number of lines");
        for pair in lines.chunks(2) {
            prop_assert!(pair[0].trim().parse::<u16>().is_ok(), "bad group code {:?}", pair[0]);
        }
        prop_assert_eq!(dxf.matches("  0\nSECTION\n").count(), 4);
        prop_assert_eq!(dxf.matches("  0\nENDSEC\n").count(), 4);
        prop_assert!(dxf.ends_with("  0\nEOF\n"));
        for e in &spec.entities {
            let declared = format!("  2\n{}\n", e.layer());
            prop_assert!(dxf.contains(&declared), "layer {} not declared", e.layer());
        }
    }
}
