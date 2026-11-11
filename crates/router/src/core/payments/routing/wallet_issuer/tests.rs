use std::cell::Cell;

use api_models::routing::ValueType;
use common_enums::{GooglePayAuthMethod, TransactionType};
use serde_json::json;

use super::*;
use crate::core::payments::routing::utils::convert_backend_input_to_routing_eval;

fn apple_pay(decrypted: bool) -> WalletData {
    let payment_data = if decrypted {
        json!({
            "application_primary_account_number": "4242424242424242",
            "application_expiration_month": "12",
            "application_expiration_year": "30",
            "payment_data": {"online_payment_cryptogram": "test-cryptogram"}
        })
    } else {
        json!("encrypted-apple-pay-token")
    };
    WalletData::ApplePay(
        serde_json::from_value(json!({
            "payment_data": payment_data,
            "payment_method": {"display_name": "Visa 4242", "network": "Visa", "pm_type": "credit"},
            "transaction_identifier": "test-transaction"
        }))
        .expect("valid Apple Pay fixture"),
    )
}

fn google_pay(auth_method: GooglePayAuthMethod, decrypted: bool) -> WalletData {
    let cryptogram = (auth_method == GooglePayAuthMethod::Cryptogram).then_some("fixture");
    let tokenization_data = if decrypted {
        json!({
            "application_primary_account_number": "5555555555554444",
            "card_exp_month": "12", "card_exp_year": "30", "auth_method": auth_method,
            "cryptogram": cryptogram
        })
    } else {
        json!({"type": "PAYMENT_GATEWAY", "token": "encrypted-google-pay-token"})
    };
    WalletData::GooglePay(
        serde_json::from_value(json!({
            "pm_type": "CARD", "description": "Mastercard 4444",
            "info": {"card_network": "MASTERCARD", "card_details": "4444"},
            "tokenization_data": tokenization_data
        }))
        .expect("valid Google Pay fixture"),
    )
}

fn input(method_type: &str) -> BackendInput {
    serde_json::from_value(json!({
        "payment": {"amount": 1000, "currency": "USD", "billing_country": "UnitedKingdomOfGreatBritainAndNorthernIreland"},
        "payment_method": {"payment_method": "wallet", "payment_method_type": method_type},
        "mandate": {}
    })).expect("valid routing input")
}

fn de_parameters(input: BackendInput) -> std::collections::HashMap<String, Option<ValueType>> {
    convert_backend_input_to_routing_eval(
        "pro_wallet".into(),
        Some("pay_wallet".into()),
        input,
        vec![],
        TransactionType::Payment,
    )
    .expect("routing input converts to DE parameters")
    .parameters
}

#[tokio::test]
async fn apple_pay_country_reaches_de_while_payment_method_remains_wallet() {
    let wallet = apple_pay(true);
    let mut input = input("apple_pay");
    enrich_input_from_wallet(&mut input, &wallet, |bin| async move {
        assert_eq!(bin, "424242");
        Ok::<_, ()>(Some("US".into()))
    })
    .await;
    let params = de_parameters(input);
    assert_eq!(
        params.get("issuer_country"),
        Some(&Some(ValueType::EnumVariant(
            "UnitedStatesOfAmerica".into()
        )))
    );
    assert_eq!(
        params.get("payment_method"),
        Some(&Some(ValueType::EnumVariant("wallet".into())))
    );
    assert_eq!(
        params.get("payment_method_type"),
        Some(&Some(ValueType::EnumVariant("apple_pay".into())))
    );
}

#[tokio::test]
async fn google_pay_pan_only_and_cryptogram_wallets_use_the_decrypted_number() {
    for auth_method in [
        GooglePayAuthMethod::PanOnly,
        GooglePayAuthMethod::Cryptogram,
    ] {
        let wallet = google_pay(auth_method, true);
        let mut input = input("google_pay");
        enrich_input_from_wallet(&mut input, &wallet, |bin| async move {
            assert_eq!(bin, "555555");
            Ok::<_, ()>(Some(" de ".into()))
        })
        .await;
        let params = de_parameters(input);
        assert_eq!(
            params.get("issuer_country"),
            Some(&Some(ValueType::EnumVariant("Germany".into())))
        );
        assert_eq!(
            params.get("payment_method_type"),
            Some(&Some(ValueType::EnumVariant("google_pay".into())))
        );
    }
}

#[tokio::test]
async fn unknown_invalid_and_failed_bin_lookups_leave_country_absent() {
    for result in [
        Ok(None),
        Ok(Some("invalid".into())),
        Ok(Some("".into())),
        Ok(Some("USA".into())),
        Err(()),
    ] {
        let mut input = input("apple_pay");
        enrich_input_from_wallet(&mut input, &apple_pay(true), |_| async move { result }).await;
        assert!(!de_parameters(input).contains_key("issuer_country"));
    }
}

#[tokio::test]
async fn encrypted_and_non_card_wallets_do_not_trigger_bin_lookup_or_use_billing_country() {
    for wallet in [
        apple_pay(false),
        google_pay(GooglePayAuthMethod::PanOnly, false),
        WalletData::PaypalRedirect(
            serde_json::from_value(json!({})).expect("valid PayPal fixture"),
        ),
    ] {
        let called = Cell::new(false);
        let mut input = input("apple_pay");
        enrich_input_from_wallet(&mut input, &wallet, |_| {
            called.set(true);
            async { Ok::<_, ()>(Some("US".into())) }
        })
        .await;
        assert!(!called.get());
        assert!(!de_parameters(input).contains_key("issuer_country"));
    }
}

#[tokio::test]
async fn existing_issuer_information_is_preserved() {
    let mut input = input("apple_pay");
    input.issuer_data = Some(IssuerDataInput {
        name: Some("Known issuer".into()),
        country: Some(Country::Canada),
    });
    let called = Cell::new(false);
    enrich_input_from_wallet(&mut input, &apple_pay(true), |_| {
        called.set(true);
        async { Ok::<_, ()>(Some("US".into())) }
    })
    .await;
    assert!(!called.get());
    assert_eq!(
        input.issuer_data.as_ref().and_then(|issuer| issuer.country),
        Some(Country::Canada)
    );
    assert_eq!(
        input.issuer_data.and_then(|issuer| issuer.name).as_deref(),
        Some("Known issuer")
    );
}

fn routing_info(value: serde_json::Value) -> PaymentRoutingInfo {
    serde_json::from_value(value).expect("valid session routing fixture")
}

#[test]
fn wallet_decryption_account_comes_only_from_the_matching_session_method() {
    let info = routing_info(json!({"pre_routing_results": {
        "apple_pay": {"connector": "stripe", "merchant_connector_id": "mca_apple"},
        "google_pay": {"connector": "adyen", "merchant_connector_id": "mca_google"}
    }}));
    assert_eq!(
        wallet_session_mca(info.clone(), PaymentMethodType::ApplePay)
            .map(|id| id.get_string_repr().to_owned()),
        Some("mca_apple".into())
    );
    assert_eq!(
        wallet_session_mca(info.clone(), PaymentMethodType::GooglePay)
            .map(|id| id.get_string_repr().to_owned()),
        Some("mca_google".into())
    );
    assert!(wallet_session_mca(info, PaymentMethodType::Paypal).is_none());
    assert!(wallet_session_mca(routing_info(json!({})), PaymentMethodType::ApplePay).is_none());
}

#[test]
fn multiple_session_choices_follow_existing_first_choice_semantics() {
    let info = routing_info(json!({"pre_routing_results": {"apple_pay": [
        {"connector": "stripe", "merchant_connector_id": "mca_first"},
        {"connector": "adyen", "merchant_connector_id": "mca_second"}
    ]}}));
    assert_eq!(
        wallet_session_mca(info, PaymentMethodType::ApplePay)
            .map(|id| id.get_string_repr().to_owned()),
        Some("mca_first".into())
    );
    let empty = routing_info(json!({"pre_routing_results": {"apple_pay": []}}));
    assert!(wallet_session_mca(empty, PaymentMethodType::ApplePay).is_none());
}

#[tokio::test]
async fn adding_country_preserves_an_existing_issuer_name() {
    let mut input = input("apple_pay");
    input.issuer_data = Some(IssuerDataInput {
        name: Some("Known issuer".into()),
        country: None,
    });
    enrich_input_from_wallet(&mut input, &apple_pay(true), |_| async {
        Ok::<_, ()>(Some("US".into()))
    })
    .await;
    let issuer = input.issuer_data.expect("issuer data is retained");
    assert_eq!(issuer.name.as_deref(), Some("Known issuer"));
    assert_eq!(issuer.country, Some(Country::UnitedStatesOfAmerica));
}
