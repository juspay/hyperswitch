use super::ConnectorCustomerResponseData;

/// Connectors parse the customer email into a masked `Email`; this domain
/// type used to strip it back to a plain `String`, so the address reached
/// any log that serializes the struct via `masked_serialize`.
#[test]
fn connector_customer_email_is_masked_for_logging() {
    let email = common_utils::pii::Email::try_from("jane.doe@example.com".to_string())
        .expect("valid email");
    let data = ConnectorCustomerResponseData::new(
        "cus_1".to_string(),
        Some("Jane Doe".to_string()),
        Some(email),
        None,
    );

    let masked = hyperswitch_masking::masked_serialize(&data)
        .expect("masked serialization")
        .to_string();

    assert!(
        !masked.contains("jane.doe@example.com"),
        "email leaked into the log view: {masked}"
    );
    assert!(
        masked.contains("@example.com"),
        "EmailStrategy should retain the domain, got: {masked}"
    );
}
