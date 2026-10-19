use api_models::mandates::{ProcessorPaymentToken, RecurringDetails};
use hyperswitch_domain_models::{
    mandates::ConnectorMandateReferenceId,
    payment_method_data::RecurringDetails as DomainRecurringDetails,
};
use serde_json::json;

#[test]
fn token_only_input_preserves_the_existing_connector_request() {
    let input = json!({"processor_payment_token": "processor-token"});
    let token: ProcessorPaymentToken = serde_json::from_value(input.clone()).unwrap();
    let reference = ConnectorMandateReferenceId::from(&token);

    assert_eq!(
        reference.get_connector_mandate_id().as_deref(),
        Some("processor-token")
    );
    assert!(reference.get_mandate_metadata().is_none());
    let serialized = serde_json::to_value(token).unwrap();
    assert_eq!(
        serialized["processor_payment_token"],
        input["processor_payment_token"]
    );
    assert!(serialized.get("processor_payment_token_details").is_none());
}

#[test]
fn supplied_details_reach_the_connector_reference_without_changes() {
    for details in [
        json!({}),
        json!({"network_transaction_id": "network-reference"}),
        json!({
            "card_expiry": {"month": "09", "year": "2035"},
            "card_network": "Visa",
            "network_transaction_id": "network-reference",
            "transaction_link_id": "transaction-link",
            "processor_transaction_id": "processor-reference"
        }),
    ] {
        let token: ProcessorPaymentToken = serde_json::from_value(json!({
            "processor_payment_token": "processor-token",
            "processor_payment_token_details": details
        }))
        .unwrap();
        let reference = ConnectorMandateReferenceId::from(&token);

        assert_eq!(
            reference.get_connector_mandate_id().as_deref(),
            Some("processor-token")
        );
        assert_eq!(
            serde_json::to_value(reference.get_processor_payment_token_details().unwrap()).unwrap(),
            details
        );
        assert!(reference.get_mandate_metadata().is_none());
        let domain = DomainRecurringDetails::from(RecurringDetails::ProcessorPaymentToken(token));
        let DomainRecurringDetails::ProcessorPaymentToken(domain_token) = domain else {
            panic!("The processor token must retain its payment method type");
        };
        assert_eq!(
            serde_json::to_value(domain_token).unwrap()["processor_payment_token_details"],
            details
        );
    }
}

#[test]
fn supplied_expiry_requires_both_month_and_year() {
    for expiry in [json!({}), json!({"month": "09"}), json!({"year": "2035"})] {
        assert!(serde_json::from_value::<ProcessorPaymentToken>(json!({
            "processor_payment_token": "processor-token",
            "processor_payment_token_details": {"card_expiry": expiry}
        }))
        .is_err());
    }
}
