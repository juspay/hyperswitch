use serde_json;

use super::super::*;

#[test]
fn test_null_object_serialization() {
    let null_object = NullObject;
    let serialized = serde_json::to_string(&null_object).unwrap();
    assert_eq!(serialized, "null");
}
