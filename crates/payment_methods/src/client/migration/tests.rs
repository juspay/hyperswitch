//! Verify migration conversion preserves supported fields without inventing CVC.
use super::*;

// Preserve JSON indexing's missing-field-as-null behavior using checked access.
fn json_at(value: &serde_json::Value, pointer: &str) -> serde_json::Value {
    value.pointer(pointer).cloned().unwrap_or_default()
}

#[test]
fn migration_listing_includes_new_cards_without_a_pagination_limit() {
    let request = super::super::list::ListCustomerPaymentMethodsV1Request {
        customer_id: id_type::GlobalCustomerId::new_unchecked(
            "0a_cus_01926c58bc6e77c09e809964e72af8c8".to_owned(),
        ),
        query_params: api_models::payment_methods::PaymentMethodListRequest::default(),
        modular_service_prefix: "v1".to_owned(),
    };
    assert_eq!(
        ListMigrationCustomerPaymentMethods.query(&request),
        vec![("include_new", "true".to_owned())],
    );
    assert_eq!(
        ListMigrationCustomerPaymentMethods
            .path(&request)
            .first()
            .cloned()
            .expect("prefix path"),
        ("prefix", "v1".to_owned())
    );
}

fn record() -> PaymentMethodRecord {
    serde_json::from_value(serde_json::json!({
        "customer_id": "45630_source_customer", "name": "Customer Name", "email": "customer@example.com",
        "raw_card_number": "4242424242424242", "card_expiry_month": "7", "card_expiry_year": "35",
        "card_holder_name": "Cardholder Name", "nick_name": "Original Nickname", "card_scheme": "visa",
        "payment_method": "card", "payment_method_type": "credit",
        "billing_address_line1": "Address line one", "billing_address_line2": "Address line two",
        "billing_address_line3": "Address line three", "billing_address_city": "City", "billing_address_country": "CA",
        "billing_address_zip": "Zip", "billing_address_state": "State",
        "billing_address_first_name": "Billing First", "billing_address_last_name": "Billing Last"
    })).expect("fixture")
}

#[test]
fn migration_card_payload_uses_global_customer_and_never_fabricates_cvc() {
    let card = MigrationCardInput::try_from(&record()).expect("card");
    let customer = id_type::GlobalCustomerId::new_unchecked(
        "0a_cus_01926c58bc6e77c09e809964e72af8c8".to_string(),
    );
    let payload = serde_json::to_value(card.payload(customer.clone())).expect("payload");
    let fixture: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/card.json")).expect("wire fixture");
    fn assert_fixture(expected: &serde_json::Value, actual: &serde_json::Value) {
        if let serde_json::Value::Object(fields) = expected {
            for (key, value) in fields {
                let actual_field = actual.get(key).cloned().unwrap_or_default();
                assert_fixture(value, &actual_field);
            }
        } else {
            assert_eq!(expected, actual);
        }
    }
    assert_fixture(&fixture, &payload);
    assert_eq!(
        json_at(&payload, "/customer_id"),
        customer.get_string_repr()
    );
    assert_eq!(json_at(&payload, "/storage_type"), "persistent");
    assert_eq!(json_at(&payload, "/payment_method_type"), "card");
    assert_eq!(json_at(&payload, "/payment_method_subtype"), "credit");
    assert_eq!(
        json_at(&payload, "/payment_method_data/card/card_cvc"),
        serde_json::Value::Null
    );
    assert_eq!(
        json_at(&payload, "/payment_method_data/card/card_number"),
        "4242424242424242"
    );
    assert_eq!(
        json_at(&payload, "/payment_method_data/card/card_exp_month"),
        "07"
    );
    assert_eq!(
        json_at(&payload, "/payment_method_data/card/card_exp_year"),
        "2035"
    );
    assert_eq!(
        json_at(&payload, "/payment_method_data/card/card_holder_name"),
        "Cardholder Name"
    );
    assert_eq!(
        json_at(&payload, "/payment_method_data/card/nick_name"),
        "Original Nickname"
    );
    assert_eq!(
        json_at(&payload, "/billing/address/first_name"),
        "Billing First"
    );
    assert_eq!(
        json_at(&payload, "/billing/address/line3"),
        "Address line three"
    );
    assert_eq!(json_at(&payload, "/billing/address/country"), "CA");
    assert_eq!(
        json_at(&payload, "/network_tokenization"),
        serde_json::Value::Null
    );
}

#[test]
fn unsupported_fields_are_explicitly_rejected() {
    let fixture = serde_json::to_value(record()).expect("record");
    for (field, value) in [
        (
            "original_transaction_id",
            serde_json::json!("original_transaction"),
        ),
        ("payment_instrument_id", serde_json::json!("token")),
        (
            "connector_customer_id",
            serde_json::json!("connector_customer"),
        ),
        (
            "network_token_number",
            serde_json::json!("4242424242424242"),
        ),
        ("account_number", serde_json::json!("bank_account")),
        ("phone", serde_json::json!("phone_number")),
        ("card_number_masked", serde_json::json!("4242XXXX4242")),
    ] {
        let mut value_with_field = fixture.clone();
        *value_with_field.get_mut(field).expect("fixture field") = value;
        let row: PaymentMethodRecord = serde_json::from_value(value_with_field).expect("record");
        let error = MigrationCardInput::try_from(&row).expect_err("unsupported");
        assert_eq!(error.code, "migration_card_input_unsupported");
        assert!(error.fields.contains(&field));
    }
}

#[test]
fn raw_pan_expiry_and_card_network_are_validated() {
    let mut row = record();
    row.raw_card_number = None;
    assert!(MigrationCardInput::try_from(&row).is_err());
    row = record();
    row.raw_card_number = Some(Secret::new("4242424242424243".to_string()));
    assert!(MigrationCardInput::try_from(&row).is_err());
    row = record();
    row.card_expiry_month = Some(Secret::new("13".to_string()));
    assert!(MigrationCardInput::try_from(&row).is_err());
    row = record();
    row.card_expiry_year = Some(Secret::new("2000".to_string()));
    assert!(MigrationCardInput::try_from(&row).is_err());
    row = record();
    row.card_scheme = Some("unsupported_network".to_string());
    assert!(MigrationCardInput::try_from(&row).is_err());
}

#[test]
fn unsupported_payment_method_and_subtype_are_rejected() {
    let mut row = record();
    row.payment_method = Some(common_enums::PaymentMethod::BankDebit);
    assert!(MigrationCardInput::try_from(&row)
        .expect_err("unsupported")
        .fields
        .contains(&"payment_method"));
    row = record();
    row.payment_method_type = Some(common_enums::PaymentMethodType::Ach);
    assert!(MigrationCardInput::try_from(&row)
        .expect_err("unsupported")
        .fields
        .contains(&"payment_method_type"));
}

#[test]
fn migration_transport_uses_existing_modular_path_and_json_contract() {
    use hyperswitch_interfaces::micro_service::ClientOperation;
    assert_eq!(CreateMigrationPaymentMethod::METHOD, Method::Post);
    assert_eq!(
        CreateMigrationPaymentMethod::PATH_TEMPLATE,
        "/{prefix}/payment-methods"
    );
    let request = CreateMigrationPaymentMethodRequest {
        card: MigrationCardInput::try_from(&record()).expect("card"),
        customer_id: id_type::GlobalCustomerId::new_unchecked(
            "0a_cus_01926c58bc6e77c09e809964e72af8c8".to_string(),
        ),
        modular_service_prefix: "v1".to_string(),
    };
    let payload = CreateMigrationPaymentMethod
        .transform_request(&request)
        .expect("conversion");
    assert_eq!(
        payload.storage_type,
        Some(common_enums::StorageType::Persistent)
    );
    assert!(CreateMigrationPaymentMethod.body(payload).is_some());
}
