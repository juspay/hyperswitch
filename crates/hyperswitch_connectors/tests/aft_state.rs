//! AFT request mapping and serialization only; no PSP credentials or network calls.

use std::{borrow::Cow, marker::PhantomData};

use common_enums::{
    AttemptStatus, AuthenticationType, CaptureMethod, CountryAlpha2, Currency, PaymentMethod,
};
use common_utils::{
    id_type::{MerchantId, TenantId},
    types::MinorUnit,
};
use hyperswitch_connectors::connectors::{
    checkout::transformers::{CheckoutRouterData, PaymentsRequest},
    worldpayxml::transformers::{PaymentService, WorldpayxmlRouterData},
};
use hyperswitch_domain_models::{
    address::{Address, AddressDetails},
    payment_address::PaymentAddress,
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_request_types::PaymentsAuthorizeData,
    types::PaymentsAuthorizeRouterData,
};
use hyperswitch_masking::Secret;
use serde_json::{json, Value};

fn aft_payment(
    sender_state: Option<&str>,
    recipient_state: Option<&str>,
) -> PaymentsAuthorizeRouterData {
    let request = PaymentsAuthorizeData {
        payment_method_data: PaymentMethodData::Card(Card {
            card_number: "4111111111111111".parse().unwrap(),
            card_exp_month: Secret::new("12".into()),
            card_exp_year: Secret::new("2030".into()),
            card_cvc: Secret::new("123".into()),
            ..Default::default()
        }),
        amount: 100,
        order_tax_amount: None,
        email: None,
        customer_name: None,
        currency: Currency::EUR,
        confirm: true,
        capture_method: Some(CaptureMethod::Manual),
        router_return_url: None,
        webhook_url: None,
        complete_authorize_url: None,
        setup_future_usage: None,
        mandate_id: None,
        off_session: None,
        customer_acceptance: None,
        setup_mandate_details: None,
        browser_info: None,
        order_details: None,
        order_category: None,
        session_token: None,
        enrolled_for_3ds: false,
        related_transaction_id: None,
        payment_experience: None,
        payment_method_type: None,
        surcharge_details: None,
        customer_id: None,
        request_incremental_authorization: false,
        metadata: None,
        authentication_data: None,
        ucs_authentication_data: None,
        force_3ds_challenge: None,
        request_extended_authorization: None,
        split_payments: None,
        guest_customer: None,
        minor_amount: MinorUnit::new(100),
        merchant_order_reference_id: None,
        integrity_object: None,
        shipping_cost: None,
        additional_payment_method_data: None,
        merchant_account_id: None,
        merchant_config_currency: None,
        connector_testing_data: None,
        order_id: None,
        locale: None,
        payment_channel: None,
        enable_partial_authorization: None,
        enable_overcapture: None,
        is_stored_credential: None,
        mit_category: None,
        billing_descriptor: None,
        tokenization: None,
        partner_merchant_identifier_details: None,
        feature_metadata: None,
        installment_details: None,
        connector_intent_metadata: Some(
            serde_json::from_value(json!({
                "checkout": { "purpose_of_payment": "financial_services" },
                "worldpayxml": {
                    "funding_transaction_type": "liquid_and_crypto_stored_value_wallet_load",
                    "payment_purpose": "crypto_currency"
                }
            }))
            .unwrap(),
        ),
        is_account_funded_transaction: Some(true),
        recipient_details: Some(
            serde_json::from_value(json!({
                "account": { "type": "phone", "phone_number": "+33123456789" },
                "address": {
                    "first_name": "Jane", "last_name": "Doe",
                    "line1": "10 Rue de Rivoli", "city": "Paris", "zip": "75001",
                    "country": "FR", "state": recipient_state
                }
            }))
            .unwrap(),
        ),
        business_country: None,
    };
    RouterData {
        flow: PhantomData,
        merchant_id: MerchantId::try_from(Cow::Borrowed("aft_state_test")).unwrap(),
        customer_id: None,
        connector_customer: None,
        connector: "aft_state_test".into(),
        payment_id: "pay_aft_state".into(),
        attempt_id: "attempt_aft_state".into(),
        tenant_id: TenantId::try_from_string("public".into()).unwrap(),
        status: AttemptStatus::Started,
        payment_method: PaymentMethod::Card,
        payment_method_type: None,
        connector_auth_type: ConnectorAuthType::SignatureKey {
            api_key: Secret::new("test_key".into()),
            key1: Secret::new("test_channel".into()),
            api_secret: Secret::new("test_merchant".into()),
        },
        description: Some("AFT state regression".into()),
        address: PaymentAddress::new(
            None,
            Some(Address {
                address: Some(AddressDetails {
                    first_name: Some(Secret::new("Jane".into())),
                    last_name: Some(Secret::new("Doe".into())),
                    line1: Some(Secret::new("10 Rue de Rivoli".into())),
                    city: Some("Paris".into()),
                    zip: Some(Secret::new("75001".into())),
                    country: Some(CountryAlpha2::FR),
                    state: sender_state.map(|value| Secret::new(value.into())),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            None,
            None,
        ),
        auth_type: AuthenticationType::NoThreeDs,
        connector_meta_data: None,
        connector_wallets_details: None,
        amount_captured: None,
        access_token: None,
        session_token: None,
        reference_id: None,
        payment_method_token: None,
        recurring_mandate_payment_data: None,
        preprocessing_id: None,
        payment_method_balance: None,
        connector_api_version: None,
        request: request,
        response: Err(ErrorResponse::default()),
        connector_request_reference_id: "aft_state_test".into(),
        #[cfg(feature = "payouts")]
        payout_method_data: None,
        #[cfg(feature = "payouts")]
        quote_id: None,
        test_mode: None,
        connector_http_status_code: None,
        external_latency: None,
        apple_pay_flow: None,
        frm_metadata: None,
        dispute_id: None,
        refund_id: None,
        payout_id: None,
        connector_response: None,
        payment_method_status: None,
        minor_amount_captured: None,
        minor_amount_capturable: None,
        authorized_amount: None,
        integrity_check: Ok(()),
        additional_merchant_data: None,
        header_payload: None,
        connector_mandate_request_reference_id: None,
        l2_l3_data: None,
        authentication_id: None,
        psd2_sca_exemption_type: None,
        raw_connector_response: None,
        is_payment_id_from_merchant: None,
        customer_document_details: None,
        customer_date_of_birth: Some(Secret::new(
            time::Date::from_calendar_date(1980, time::Month::January, 1).unwrap(),
        )),
        feature_data: None,
        sender_payment_instrument_id: None,
        connector_returned_payment_method_details: None,
    }
}

fn checkout_body(sender_state: Option<&str>, recipient_state: Option<&str>) -> Value {
    let data = aft_payment(sender_state, recipient_state);
    let request =
        PaymentsRequest::try_from(&CheckoutRouterData::from((MinorUnit::new(100), &data)))
            .expect("AFT mapping should accept an absent state");
    serde_json::to_value(request).unwrap()
}

fn worldpayxml_body(sender_state: Option<&str>, recipient_state: Option<&str>) -> String {
    let data = aft_payment(sender_state, recipient_state);
    let amount = serde_json::from_value(json!("100")).unwrap();
    let request = PaymentService::try_from(&WorldpayxmlRouterData::from((amount, &data)))
        .expect("AFT mapping should accept an absent state");
    quick_xml::se::to_string(&request).unwrap()
}

#[test]
fn checkout_aft_omits_missing_states() {
    for (sender_state, recipient_state) in [(None, None), (Some("IDF"), None), (None, Some("IDF"))]
    {
        let body = checkout_body(sender_state, recipient_state);
        assert_eq!(body["processing"]["aft"], true);
        for (party, state) in [("sender", sender_state), ("recipient", recipient_state)] {
            let address = &body[party]["address"];
            assert_eq!(
                address.get("state"),
                state.map(|value| json!(value)).as_ref()
            );
            assert_eq!(address["country"], "FR");
            assert_eq!(address["city"], "Paris");
        }
    }
}

#[test]
fn checkout_aft_preserves_supplied_states() {
    let body = checkout_body(Some("IDF"), Some("Normandie"));
    assert_eq!(body["sender"]["address"]["state"], "IDF");
    assert_eq!(body["recipient"]["address"]["state"], "Normandie");
}

#[test]
fn worldpayxml_aft_omits_missing_states() {
    for (sender_state, recipient_state) in [(None, None), (Some("IDF"), None), (None, Some("IDF"))]
    {
        let xml = worldpayxml_body(sender_state, recipient_state);
        let document = roxmltree::Document::parse(&xml).unwrap();
        for (party_type, state) in [("sender", sender_state), ("recipient", recipient_state)] {
            let party = document
                .descendants()
                .find(|node| {
                    node.has_tag_name("fundingParty") && node.attribute("type") == Some(party_type)
                })
                .unwrap();
            let state_element = party.descendants().find(|node| node.has_tag_name("state"));
            assert_eq!(state_element.is_some(), state.is_some());
            assert_eq!(state_element.and_then(|node| node.text()), state);
            assert_eq!(
                party
                    .descendants()
                    .find(|node| node.has_tag_name("countryCode"))
                    .and_then(|node| node.text()),
                Some("FR")
            );
        }
    }
}

#[test]
fn worldpayxml_aft_preserves_supplied_states() {
    let xml = worldpayxml_body(Some("IDF"), Some("Normandie"));
    let document = roxmltree::Document::parse(&xml).unwrap();
    for (party_type, expected) in [("sender", "IDF"), ("recipient", "Normandie")] {
        let party = document
            .descendants()
            .find(|node| {
                node.has_tag_name("fundingParty") && node.attribute("type") == Some(party_type)
            })
            .unwrap();
        assert_eq!(
            party
                .descendants()
                .find(|node| node.has_tag_name("state"))
                .and_then(|node| node.text()),
            Some(expected)
        );
    }
}
