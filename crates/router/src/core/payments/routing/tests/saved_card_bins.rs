use common_utils::types::MinorUnit;
use hyperswitch_domain_models::{
    payment_address::PaymentAddress,
    payment_method_data::{CardToken, CardWithOptionalCVC, PaymentMethodData},
};
use serde_json::json;

use super::super::{make_dsl_input, utils::convert_backend_input_to_routing_eval};
use crate::{core::routing::PaymentsDslInput, types::storage};

fn routing_request(
    payment_method_data: Option<&PaymentMethodData>,
    metadata: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut attempt = attempt();
    attempt.payment_method_data = metadata;
    let intent = intent();
    let address = PaymentAddress::default();
    let input = PaymentsDslInput::new(
        None,
        &attempt,
        &intent,
        payment_method_data,
        &address,
        None,
        common_enums::Currency::CAD,
    );
    let backend_input = make_dsl_input(&input).expect("routing DSL");
    let request = convert_backend_input_to_routing_eval(
        "profile_saved_card".to_owned(),
        None,
        backend_input,
        vec![],
        common_enums::TransactionType::Payment,
    )
    .expect("DE routing request");
    serde_json::to_value(request).expect("routing request JSON")
}

#[test]
fn saved_card_bins_reach_decision_engine_with_or_without_cvc() {
    for card_cvc in [
        None,
        Some(hyperswitch_masking::Secret::new("123".to_owned())),
    ] {
        let card = PaymentMethodData::CardWithOptionalCVC(CardWithOptionalCVC {
            card_number: "5555555555554444".parse().expect("test card number"),
            card_cvc,
            ..Default::default()
        });
        let request = routing_request(Some(&card), Some(json!({"card_token": {}})));
        assert_eq!(request["parameters"]["card_bin"]["value"], "555555");
        assert_eq!(
            request["parameters"]["extended_card_bin"]["value"],
            "55555555"
        );
    }
}

#[test]
fn raw_and_network_token_backed_cards_send_original_card_bins() {
    use hyperswitch_domain_models::payment_method_data::{Card, CardWithNetworkTokenDetails};

    let card_number: cards::CardNumber = "5555555555554444".parse().expect("test card number");
    for card in [
        PaymentMethodData::Card(Card {
            card_number: card_number.clone(),
            ..Default::default()
        }),
        PaymentMethodData::CardWithNetworkTokenDetails(Box::new(CardWithNetworkTokenDetails {
            card_details: CardWithOptionalCVC {
                card_number,
                ..Default::default()
            },
            ..Default::default()
        })),
    ] {
        let request = routing_request(Some(&card), None);
        assert_eq!(request["parameters"]["card_bin"]["value"], "555555");
        assert_eq!(
            request["parameters"]["extended_card_bin"]["value"],
            "55555555"
        );
    }
}

#[test]
fn fetched_card_bins_take_precedence_over_stored_metadata() {
    let card = PaymentMethodData::CardWithOptionalCVC(CardWithOptionalCVC {
        card_number: "5555555555554444".parse().expect("test card number"),
        ..Default::default()
    });
    let request = routing_request(
        Some(&card),
        Some(json!({"card": {"card_isin": "424242", "card_extended_bin": "42424242"}})),
    );
    assert_eq!(request["parameters"]["card_bin"]["value"], "555555");
    assert_eq!(
        request["parameters"]["extended_card_bin"]["value"],
        "55555555"
    );
}

#[test]
fn stored_card_bin_fallback_and_missing_card_behavior_are_preserved() {
    let token = PaymentMethodData::CardToken(CardToken::default());
    let request = routing_request(
        Some(&token),
        Some(json!({"card": {"card_isin": "424242", "card_extended_bin": "42424242"}})),
    );
    assert_eq!(request["parameters"]["card_bin"]["value"], "424242");
    assert_eq!(
        request["parameters"]["extended_card_bin"]["value"],
        "42424242"
    );
    let request = routing_request(None, None);
    assert!(request["parameters"].get("card_bin").is_none());
    assert!(request["parameters"].get("extended_card_bin").is_none());
}

fn attempt() -> storage::PaymentAttempt {
    storage::PaymentAttempt {
        payment_id: Default::default(),
        merchant_id: Default::default(),
        attempt_id: "attempt_saved_card".to_owned(),
        status: common_enums::AttemptStatus::Started,
        net_amount: Default::default(),
        currency: None,
        save_to_locker: None,
        connector: None,
        error_message: None,
        offer_amount: None,
        payment_method_id: None,
        payment_method: None,
        connector_transaction_id: None,
        capture_method: None,
        capture_on: None,
        confirm: true,
        authentication_type: None,
        created_at: common_utils::date_time::now(),
        modified_at: common_utils::date_time::now(),
        last_synced: None,
        cancellation_reason: None,
        amount_to_capture: None,
        mandate_id: None,
        browser_info: None,
        error_code: None,
        payment_token: None,
        connector_metadata: None,
        payment_experience: None,
        payment_method_type: None,
        payment_method_data: None,
        business_sub_label: None,
        straight_through_algorithm: None,
        preprocessing_step_id: None,
        mandate_details: None,
        error_reason: None,
        multiple_capture_count: None,
        connector_response_reference_id: None,
        amount_capturable: MinorUnit::zero(),
        updated_by: "postgres_only".to_owned(),
        authentication_data: None,
        encoded_data: None,
        merchant_connector_id: None,
        unified_code: None,
        unified_message: None,
        external_three_ds_authentication_attempted: None,
        external_threeds_authentication_type: None,
        authentication_connector: None,
        authentication_id: None,
        mandate_data: None,
        payment_method_billing_address_id: None,
        fingerprint_id: None,
        fingerprint_type: None,
        charge_id: None,
        client_source: None,
        client_version: None,
        customer_acceptance: None,
        profile_id: common_utils::generate_profile_id_of_default_length(),
        organization_id: Default::default(),
        connector_mandate_detail: None,
        tokenization: None,
        request_extended_authorization: None,
        extended_authorization_applied: None,
        extended_authorization_last_applied_at: None,
        capture_before: None,
        card_discovery: None,
        charges: None,
        issuer_error_code: None,
        issuer_error_message: None,
        processor_merchant_id: Default::default(),
        created_by: None,
        setup_future_usage_applied: None,
        routing_approach: None,
        connector_request_reference_id: None,
        debit_routing_savings: None,
        network_transaction_id: None,
        network_transaction_link_id: None,
        is_overcapture_enabled: None,
        network_details: None,
        is_stored_credential: None,
        authorized_amount: None,
        encrypted_payment_method_data: None,
        error_details: None,
        retry_type: None,
        installment_data: None,
        external_surcharge_details: None,
        applied_offer_details: None,
        applied_overrides: None,
        payment_account_reference: None,
        sender_payment_instrument_id: None,
        active_frm_id: None,
    }
}

fn intent() -> storage::PaymentIntent {
    storage::PaymentIntent {
        payment_id: Default::default(),
        merchant_id: Default::default(),
        status: common_enums::IntentStatus::RequiresConfirmation,
        amount: MinorUnit::new(1000),
        shipping_cost: None,
        currency: None,
        amount_captured: None,
        customer_id: None,
        description: None,
        return_url: None,
        metadata: None,
        connector_id: None,
        shipping_address_id: None,
        billing_address_id: None,
        statement_descriptor_name: None,
        statement_descriptor_suffix: None,
        created_at: common_utils::date_time::now(),
        modified_at: common_utils::date_time::now(),
        last_synced: None,
        setup_future_usage: None,
        off_session: None,
        client_secret: None,
        active_attempt: hyperswitch_domain_models::RemoteStorageObject::ForeignID(
            "attempt_saved_card".to_owned(),
        ),
        business_country: None,
        business_label: None,
        order_details: None,
        allowed_payment_method_types: None,
        connector_metadata: None,
        feature_metadata: None,
        attempt_count: 1,
        profile_id: common_utils::generate_profile_id_of_default_length(),
        payment_link_id: None,
        merchant_decision: None,
        payment_confirm_source: None,
        updated_by: "postgres_only".to_owned(),
        surcharge_applicable: None,
        request_incremental_authorization: None,
        incremental_authorization_allowed: None,
        authorization_count: None,
        fingerprint_id: None,
        session_expiry: None,
        request_external_three_ds_authentication: None,
        split_payments: None,
        frm_metadata: None,
        customer_details: None,
        billing_details: None,
        merchant_order_reference_id: None,
        shipping_details: None,
        is_payment_processor_token_flow: None,
        organization_id: Default::default(),
        tax_details: None,
        skip_external_tax_calculation: None,
        request_extended_authorization: None,
        psd2_sca_exemption_type: None,
        processor_merchant_id: Default::default(),
        created_by: None,
        force_3ds_challenge: None,
        force_3ds_challenge_trigger: None,
        is_iframe_redirection_enabled: None,
        is_payment_id_from_merchant: None,
        payment_channel: None,
        tax_status: None,
        discount_amount: None,
        order_date: None,
        shipping_amount_tax: None,
        value_added_tax_amount: None,
        value_added_tax_percent: None,
        duty_amount: None,
        order_commodity_code: None,
        vat_invoice_reference_number: None,
        enable_partial_authorization: None,
        enable_overcapture: None,
        mit_category: None,
        billing_descriptor: None,
        tokenization: None,
        state_metadata: None,
        installment_options: None,
        profile_acquirer_id: None,
        external_surcharge_strategy: None,
        external_surcharge_applicable: None,
        is_account_funded_transaction: None,
        partner_merchant_identifier_details: None,
        recipient_details: None,
        enable_avs_check: None,
        enable_account_name_verification: None,
        settlement_conclusion_applied: None,
        sender_details: None,
    }
}
