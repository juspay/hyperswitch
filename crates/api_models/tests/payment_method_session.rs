#![cfg(feature = "v2")]

use api_models::payment_methods::{
    PaymentMethodSessionConfirmRequest, PaymentMethodSessionData, PaymentMethodSessionWalletData,
};
use hyperswitch_masking::PeekInterface;
use serde_json::json;

#[test]
fn session_apple_pay_accepts_encrypted_input_but_rejects_decrypted_credentials(
) -> Result<(), serde_json::Error> {
    let mut request = json!({
        "payment_method_type": "wallet",
        "payment_method_subtype": "apple_pay",
        "payment_method_data": {
            "wallet": {
                "apple_pay": {
                    "payment_data": "encrypted-apple-pay-token",
                    "payment_method": {
                        "display_name": "Visa 1234",
                        "network": "Visa",
                        "type": "debit"
                    },
                    "transaction_identifier": "apple-transaction"
                }
            }
        }
    });
    let parsed: PaymentMethodSessionConfirmRequest = serde_json::from_value(request.clone())?;
    assert!(matches!(
        parsed.payment_method_data.payment_method_data,
        PaymentMethodSessionData::Wallet(PaymentMethodSessionWalletData::ApplePay(data))
            if data.payment_data.peek() == "encrypted-apple-pay-token"
                && data.payment_method.network == "Visa"
    ));

    request["payment_method_data"]["wallet"]["apple_pay"]["payment_data"] = json!({
        "application_primary_account_number": "4242424242424242",
        "application_expiration_month": "12",
        "application_expiration_year": "30",
        "payment_data": { "online_payment_cryptogram": "cryptogram", "eci_indicator": "05" }
    });
    assert!(serde_json::from_value::<PaymentMethodSessionConfirmRequest>(request).is_err());
    Ok(())
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
fn raw_apple_pay_without_cryptogram_returns_explicit_null() -> Result<(), serde_json::Error> {
    let raw: api_models::payment_methods::RawWalletDetail = serde_json::from_value(json!({
        "apple_pay_decrypted_data": {
            "application_primary_account_number": "4242424242424242",
            "expiry_month": "12",
            "expiry_year": "2030"
        }
    }))?;

    let response = serde_json::to_value(raw)?;
    assert_eq!(
        response["apple_pay_decrypted_data"].get("payment_data"),
        Some(&serde_json::Value::Null)
    );
    Ok(())
}
