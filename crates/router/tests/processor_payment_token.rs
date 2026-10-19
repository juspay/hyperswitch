use api_models::mandates::ProcessorPaymentToken;
use hyperswitch_domain_models::mandates::ConnectorMandateReferenceId;
use router::types::transformers::ForeignTryFrom;
use serde_json::json;
use unified_connector_service_client::payments;

#[test]
fn token_only_requests_keep_the_existing_ucs_reference() {
    let token: ProcessorPaymentToken = serde_json::from_value(json!({
        "processor_payment_token": "processor-token"
    }))
    .unwrap();
    let reference = ConnectorMandateReferenceId::from(&token);
    let wire = payments::ConnectorMandateReferenceId::foreign_try_from(&reference).unwrap();

    assert_eq!(
        wire.connector_mandate_id.as_deref(),
        Some("processor-token")
    );
    assert!(wire.mandate_metadata.is_none());
}

#[test]
fn ucs_rejects_details_that_its_interface_cannot_carry() {
    for details in [
        json!({}),
        json!({"network_transaction_id": "network-reference"}),
    ] {
        let token: ProcessorPaymentToken = serde_json::from_value(json!({
            "processor_payment_token": "processor-token",
            "processor_payment_token_details": details
        }))
        .unwrap();
        let reference = ConnectorMandateReferenceId::from(&token);
        let error =
            payments::ConnectorMandateReferenceId::foreign_try_from(&reference).unwrap_err();

        assert!(matches!(
            error.current_context(),
            external_services::grpc_client::unified_connector_service::UnifiedConnectorServiceError::NotImplemented(_)
        ));
    }
}
