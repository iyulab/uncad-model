//! The entity ID is an opaque token: a decimal string, canonical, read back exactly.

use uncad_model::model::EntityId;

#[test]
fn an_id_is_written_as_a_decimal_string() {
    assert_eq!(
        serde_json::to_string(&EntityId::new(42)).unwrap(),
        r#""42""#
    );
    assert_eq!(serde_json::to_string(&EntityId::new(0)).unwrap(), r#""0""#);
}

#[test]
fn ids_above_the_double_precision_limit_stay_distinct() {
    // 2^53 + 1 and 2^53 are one number to a consumer whose numbers are IEEE doubles; as tokens they are two.
    let above = [1u64 << 53, (1 << 53) + 1, 1 << 63, (1 << 63) + 4, u64::MAX];
    let texts: Vec<String> = above
        .iter()
        .map(|v| serde_json::to_string(&EntityId::new(*v)).unwrap())
        .collect();
    let mut unique = texts.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), above.len(), "{texts:?}");
    for v in above {
        let back: EntityId =
            serde_json::from_str(&serde_json::to_string(&EntityId::new(v)).unwrap()).unwrap();
        assert_eq!(back.value(), v);
    }
}

#[test]
fn only_the_canonical_string_is_read() {
    for bad in [
        "42",                        // a number: the form this replaced
        r#""+42""#,                  // a sign
        r#""042""#,                  // a leading zero
        r#""-1""#,                   // negative
        r#""4.0""#,                  // not an integer
        r#""""#,                     // empty
        r#"" 42""#,                  // a space
        r#""18446744073709551616""#, // u64::MAX + 1
        "null",
    ] {
        assert!(
            serde_json::from_str::<EntityId>(bad).is_err(),
            "{bad} was read"
        );
    }
}

#[test]
fn an_id_is_a_usable_map_key() {
    use std::collections::BTreeMap;
    let mut by_id = BTreeMap::new();
    by_id.insert(EntityId::new(1 << 63), "handleless");
    by_id.insert(EntityId::new(7), "handle");
    let json = serde_json::to_string(&by_id).unwrap();
    let back: BTreeMap<EntityId, &str> = serde_json::from_str(&json).unwrap();
    assert_eq!(back, by_id);
}
