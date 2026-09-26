//! None of the provenance markers has a default: an entity whose JSON omits
//! `origin`, `confidence` or `source_handle` does not load. A default would
//! let "vector" or "high" leak in for a producer that never said so.

use serde_json::Value;
use uncad_model::model::Entity;
use uncad_model::CadDatabase;

/// Every entity of a document an earlier version wrote, as JSON.
fn entities() -> Vec<Value> {
    let doc: Value =
        serde_json::from_str(include_str!("data/model-0.1.0.json")).expect("the document is JSON");
    doc["entities"].as_array().expect("an entity list").clone()
}

#[test]
fn every_entity_loads_with_its_markers() {
    for entity in entities() {
        serde_json::from_value::<Entity>(entity.clone())
            .unwrap_or_else(|e| panic!("{e}: {entity}"));
    }
}

#[test]
fn an_entity_without_a_marker_does_not_load() {
    for marker in ["origin", "confidence", "source_handle"] {
        for mut entity in entities() {
            let kind = entity["type"].clone();
            entity["common"]
                .as_object_mut()
                .expect("every entity has common")
                .remove(marker);
            assert!(
                serde_json::from_value::<Entity>(entity).is_err(),
                "a {kind} without {marker} loaded"
            );
        }
    }
}

/// The same holds for a whole document: one entity missing its confidence
/// makes the drawing fail to load, rather than load with a guess in it.
#[test]
fn a_document_with_one_unmarked_entity_does_not_load() {
    let mut doc: Value =
        serde_json::from_str(include_str!("data/model-0.1.0.json")).expect("the document is JSON");
    doc["entities"][0]["common"]
        .as_object_mut()
        .expect("common")
        .remove("confidence");
    assert!(serde_json::from_value::<CadDatabase>(doc).is_err());
}
