use super::*;

#[test]
fn test_mandate_type() {
    let mandate_type = MandateType::default();
    assert_eq!(
        serde_json::to_string(&mandate_type).unwrap(),
        r#"{"multi_use":null}"#
    )
}
