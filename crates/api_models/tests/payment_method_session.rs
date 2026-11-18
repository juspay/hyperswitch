#![cfg(feature = "v2")]

use api_models::payment_methods::{
    PaymentMethodSessionConfirmRequest, PaymentMethodSessionData, PaymentMethodSessionWalletData,
};
use hyperswitch_masking::PeekInterface;
use serde_json::json;

fn apple_pay_confirm_request(payment_data: serde_json::Value) -> serde_json::Value {
    json!({
        "payment_method_type": "wallet",
        "payment_method_subtype": "apple_pay",
        "payment_method_data": {
            "wallet": {
                "apple_pay": {
                    "payment_data": payment_data,
                    "payment_method": {
                        "display_name": "Visa 1234",
                        "network": "Visa",
                        "type": "debit"
                    },
                    "transaction_identifier": "apple-transaction"
                }
            }
        }
    })
}

#[test]
fn session_apple_pay_accepts_encrypted_input_but_rejects_decrypted_credentials() {
    let parsed = serde_json::from_value::<PaymentMethodSessionConfirmRequest>(
        apple_pay_confirm_request(json!("encrypted-apple-pay-token")),
    );
    assert!(matches!(
        parsed.map(|request| request.payment_method_data.payment_method_data),
        Ok(PaymentMethodSessionData::Wallet(PaymentMethodSessionWalletData::ApplePay(data)))
            if data.payment_data.peek() == "encrypted-apple-pay-token"
                && data.payment_method.network == "Visa"
    ));

    let request = apple_pay_confirm_request(json!({
        "application_primary_account_number": "4242424242424242",
        "application_expiration_month": "12",
        "application_expiration_year": "30",
        "payment_data": { "online_payment_cryptogram": "cryptogram", "eci_indicator": "05" }
    }));
    assert!(serde_json::from_value::<PaymentMethodSessionConfirmRequest>(request).is_err());
}

#[test]
fn session_rejects_unsupported_instruments_and_missing_instrument_data() {
    for data in [
        json!({"pay_later": {"klarna_redirect": {}}}),
        json!({"bank_redirect": {"ideal": {}}}),
        json!({"wallet": {"apple_pay_decrypted": {}}}),
        json!({"wallet": {"google_pay": {}}}),
        json!({"billing": {}}),
    ] {
        let request = json!({
            "payment_method_type": "wallet",
            "payment_method_subtype": "apple_pay",
            "payment_method_data": data
        });
        assert!(
            serde_json::from_value::<PaymentMethodSessionConfirmRequest>(request).is_err(),
            "Unsupported or missing instrument must not deserialize: {data}"
        );
    }
}

#[test]
fn raw_apple_pay_without_cryptogram_returns_explicit_null() {
    let response = serde_json::from_value::<api_models::payment_methods::RawWalletDetail>(json!({
        "apple_pay_decrypted_data": {
            "application_primary_account_number": "4242424242424242",
            "expiry_month": "12",
            "expiry_year": "2030"
        }
    }))
    .and_then(serde_json::to_value);

    assert_eq!(
        response
            .as_ref()
            .ok()
            .and_then(|value| value.pointer("/apple_pay_decrypted_data/payment_data")),
        Some(&serde_json::Value::Null)
    );
}
