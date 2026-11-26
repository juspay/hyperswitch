//! Check synthetic migration payloads against the real modular customer/card DTOs.
#![cfg(feature = "v2")]

use hyperswitch_masking::PeekInterface;

#[test]
fn v2_accepts_generated_customer_and_card_contracts() {
    if let Ok(directory) = std::env::var("DERIVCO_CONTRACT_DIR") {
        let directory = std::path::Path::new(&directory);
        let customer: api_models::customers::CustomerRequest = serde_json::from_slice(
            &std::fs::read(directory.join("customer_request.json"))
                .expect("exported customer contract"),
        )
        .expect("v2 customer DTO");
        assert_eq!(
            customer
                .merchant_reference_id
                .expect("reference")
                .get_string_repr(),
            "45630_qa_contract"
        );
        assert_eq!(customer.name.peek(), "QA Contract Customer");
        let card: api_models::payment_methods::PaymentMethodCreate = serde_json::from_slice(
            &std::fs::read(directory.join("card_request.json")).expect("exported card contract"),
        )
        .expect("v2 card DTO");
        assert_eq!(card.storage_type, common_enums::StorageType::Persistent);
        assert_eq!(
            card.customer_id.expect("customer").get_string_repr(),
            "0a_cus_01926c58bc6e77c09e809964e72af8c8"
        );
        match card.payment_method_data {
            api_models::payment_methods::PaymentMethodCreateData::Card(card) => {
                assert!(card.card_cvc.is_none())
            }
            _ => panic!("raw card expected"),
        }
    } else {
        let customer: api_models::customers::CustomerRequest = serde_json::from_value(serde_json::json!({"merchant_reference_id": "45630_qa_contract", "name": "QA Contract Customer", "email": "qa.contract@example.com"})).expect("v2 customer DTO");
        assert_eq!(customer.name.peek(), "QA Contract Customer");
    }
}
#[test]
fn v2_accepts_the_v1_migration_wire_contract_without_cvc() {
    let request: api_models::payment_methods::PaymentMethodCreate = serde_json::from_str(
        include_str!("../../payment_methods/src/client/migration/fixtures/card.json"),
    )
    .expect("v2 contract");
    assert_eq!(request.storage_type, common_enums::StorageType::Persistent);
    assert_eq!(
        request
            .customer_id
            .as_ref()
            .expect("customer")
            .get_string_repr(),
        "0a_cus_01926c58bc6e77c09e809964e72af8c8"
    );
    match request.payment_method_data {
        api_models::payment_methods::PaymentMethodCreateData::Card(card) => {
            assert!(card.card_cvc.is_none());
            assert_eq!(
                card.card_holder_name.as_ref().expect("holder").peek(),
                "Cardholder Name"
            );
        }
        _ => panic!("migration must be a card"),
    }
    assert_eq!(
        request
            .billing
            .expect("billing")
            .address
            .expect("address")
            .line3
            .expect("line3")
            .peek(),
        "Address line three"
    );
}
