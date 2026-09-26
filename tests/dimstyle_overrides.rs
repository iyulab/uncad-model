//! A dimension's own overrides applied to its style: each replaces the
//! variable of its group, a value of the wrong kind makes that variable
//! unknown, and anything the style record does not carry is skipped.

use uncad_model::model::{OverrideValue, StyleOverride};
use uncad_model::tables::{DimStyleRecord, LinearUnitFormat};

fn iso25() -> DimStyleRecord {
    DimStyleRecord {
        name: "ISO-25".to_string(),
        post: Some(String::new()),
        length_factor: Some(1.0),
        tolerances: Some(false),
        tolerance_upper: Some(0.0),
        decimal_places: Some(2),
        text_height: Some(2.5),
        linear_unit_format: Some(LinearUnitFormat::Decimal),
        ..DimStyleRecord::default()
    }
}

fn o(variable: u16, value: OverrideValue) -> StyleOverride {
    StyleOverride { variable, value }
}

#[test]
fn no_overrides_is_the_style() {
    assert_eq!(iso25().overridden(&[]), iso25());
}

#[test]
fn each_override_replaces_the_variable_of_its_group() {
    let s = iso25().overridden(&[
        o(271, OverrideValue::Integer(0)),
        o(144, OverrideValue::Real(25.4)),
        o(3, OverrideValue::Text("<> mm".into())),
        o(71, OverrideValue::Integer(1)),
        o(47, OverrideValue::Real(0.1)),
        o(277, OverrideValue::Integer(4)),
    ]);
    assert_eq!(s.decimal_places, Some(0));
    assert_eq!(s.length_factor, Some(25.4));
    assert_eq!(s.post.as_deref(), Some("<> mm"));
    assert_eq!(s.tolerances, Some(true));
    assert_eq!(s.tolerance_upper, Some(0.1));
    assert_eq!(s.linear_unit_format, Some(LinearUnitFormat::Architectural));
    // Not overridden: the style's.
    assert_eq!(s.text_height, Some(2.5));
    assert_eq!(s.name, "ISO-25");
}

#[test]
fn a_value_of_the_wrong_kind_is_not_the_styles_value_either() {
    let s = iso25().overridden(&[
        o(271, OverrideValue::Real(2.0)),
        o(140, OverrideValue::Text("big".into())),
        o(277, OverrideValue::Integer(9)),
    ]);
    assert_eq!(s.decimal_places, None);
    assert_eq!(s.text_height, None);
    assert_eq!(s.linear_unit_format, None, "9 is no DIMLUNIT");
}

#[test]
fn a_variable_the_record_does_not_carry_is_skipped_and_the_last_override_counts() {
    let s = iso25().overridden(&[
        o(5, OverrideValue::Handle("2A".into())),
        o(176, OverrideValue::Integer(1)),
        o(271, OverrideValue::Integer(4)),
        o(271, OverrideValue::Integer(1)),
    ]);
    let mut expected = iso25();
    expected.decimal_places = Some(1);
    assert_eq!(s, expected);
}
