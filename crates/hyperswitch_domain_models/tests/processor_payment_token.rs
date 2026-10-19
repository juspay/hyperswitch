use api_models::mandates::{ProcessorPaymentToken, RecurringDetails};
use hyperswitch_domain_models::{
    mandates::ConnectorMandateReferenceId,
    payment_method_data::RecurringDetails as DomainRecurringDetails,
};
use hyperswitch_masking::ExposeInterface;
use serde_json::json;

#[test]
fn token_only_input_preserves_the_existing_connector_request() {
    let input = json!({"processor_payment_token": "processor-token"});
    let token: ProcessorPaymentToken = serde_json::from_value(input.clone()).unwrap();
    let reference = ConnectorMandateReferenceId::try_from(&token).unwrap();

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
fn supplied_details_reach_the_connector_metadata_without_changes() {
    for details in [
        json!({}),
        json!({"network_transaction_id": "network-reference"}),
        json!({
            "card_exp_month": "09",
            "card_exp_year": "2035",
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
        let reference = ConnectorMandateReferenceId::try_from(&token).unwrap();

        assert_eq!(
            reference.get_connector_mandate_id().as_deref(),
            Some("processor-token")
        );
        assert_eq!(
            reference.get_mandate_metadata().unwrap().expose(),
            json!({"processor_payment_token_details": details})
        );
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
