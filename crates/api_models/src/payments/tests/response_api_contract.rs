use super::super::*;

#[derive(Debug, serde::Serialize)]
struct TestPaymentsResponse {
    #[serde(serialize_with = "serialize_payment_method_data_response")]
    payment_method_data: Option<PaymentMethodDataResponseWithBilling>,
}

#[test]
fn test_reward_payment_response() {
    let payment_method_response_with_billing = PaymentMethodDataResponseWithBilling {
        payment_method_data: Some(PaymentMethodDataResponse::Reward {}),
        billing: None,
    };

    let payments_response = TestPaymentsResponse {
        payment_method_data: Some(payment_method_response_with_billing),
    };

    let expected_response = r#"{"payment_method_data":"reward"}"#;

    let stringified_payments_response = payments_response.encode_to_string_of_json();
    assert_eq!(stringified_payments_response.unwrap(), expected_response);
}

#[cfg(feature = "v1")]
#[test]
fn test_fingerprint_type_serialization() {
    assert_eq!(
        serde_json::to_value(FingerprintType::Dpan).unwrap(),
        serde_json::json!("dpan")
    );
    assert_eq!(
        serde_json::to_value(FingerprintType::Fpan).unwrap(),
        serde_json::json!("fpan")
    );
}
