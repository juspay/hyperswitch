use super::*;

/// Set of tests to extract billing details from payment method data.
/// These are required for backwards compatibility.
mod billing_from_payment_method_data;
mod null_object;
#[cfg(feature = "v1")]
mod request;
#[cfg(feature = "v1")]
mod request_api_contract;
mod response_api_contract;

#[test]
fn test_mandate_type() {
    let mandate_type = MandateType::default();
    assert_eq!(
        serde_json::to_string(&mandate_type).unwrap(),
        r#"{"multi_use":null}"#
    )
}
