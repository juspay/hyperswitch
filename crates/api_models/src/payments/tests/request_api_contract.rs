use std::str::FromStr;

use super::super::*;

#[test]
fn test_successful_card_deser() {
    let payments_request = r#"
        {
            "amount": 6540,
            "currency": "USD",
            "payment_method": "card",
            "payment_method_data": {
                "card": {
                    "card_number": "4242424242424242",
                    "card_exp_month": "10",
                    "card_exp_year": "25",
                    "card_holder_name": "joseph Doe",
                    "card_cvc": "123"
                }
            }
        }
        "#;

    let expected_card_number_string = "4242424242424242";
    let expected_card_number = CardNumber::from_str(expected_card_number_string).unwrap();

    let payments_request = serde_json::from_str::<PaymentsRequest>(payments_request);
    assert!(payments_request.is_ok());

    if let Some(PaymentMethodData::Card(card_data)) = payments_request
        .unwrap()
        .payment_method_data
        .unwrap()
        .payment_method_data
    {
        assert_eq!(card_data.card_number, expected_card_number);
    } else {
        panic!("Received unexpected response")
    }
}

#[test]
fn test_successful_payment_method_reward() {
    let payments_request = r#"
        {
            "amount": 6540,
            "currency": "USD",
            "payment_method": "reward",
            "payment_method_data": "reward",
            "payment_method_type": "evoucher"
        }
        "#;

    let payments_request = serde_json::from_str::<PaymentsRequest>(payments_request);
    assert!(payments_request.is_ok());
    assert_eq!(
        payments_request
            .unwrap()
            .payment_method_data
            .unwrap()
            .payment_method_data,
        Some(PaymentMethodData::Reward)
    );
}

#[test]
fn test_payment_method_data_with_payment_method_billing() {
    let payments_request = r#"
        {
            "amount": 6540,
            "currency": "USD",
            "payment_method_data": {
                "billing": {
                    "address": {
                        "line1": "1467",
                        "line2": "Harrison Street",
                        "city": "San Fransico",
                        "state": "California",
                        "zip": "94122",
                        "country": "US",
                        "first_name": "Narayan",
                        "last_name": "Bhat"
                    }
                }
            }
        }
        "#;

    let payments_request = serde_json::from_str::<PaymentsRequest>(payments_request);
    assert!(payments_request.is_ok());
    assert!(payments_request
        .unwrap()
        .payment_method_data
        .unwrap()
        .billing
        .is_some());
}
