#[test]
fn test_backwards_compatibility() {
    let serialized_payment_attempt = r#"{
    "payment_id": "PMT123456789",
    "merchant_id": "M123456789",
    "attempt_id": "ATMPT123456789",
    "status": "pending",
    "amount": 10000,
    "currency": "USD",
    "save_to_locker": true,
    "connector": "stripe",
    "error_message": null,
    "offer_amount": 9500,
    "surcharge_amount": 500,
    "tax_amount": 800,
    "payment_method_id": "CRD123456789",
    "payment_method": "card",
    "connector_transaction_id": "CNTR123456789",
    "capture_method": "automatic",
    "capture_on": "2022-09-10T10:11:12Z",
    "confirm": false,
    "authentication_type": "no_three_ds",
    "created_at": "2024-02-26T12:00:00Z",
    "modified_at": "2024-02-26T12:00:00Z",
    "last_synced": null,
    "cancellation_reason": null,
    "amount_to_capture": 10000,
    "mandate_id": null,
    "browser_info": {
        "user_agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/70.0.3538.110 Safari/537.36",
        "accept_header": "text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,image/apng,*/*;q=0.8",
        "language": "nl-NL",
        "color_depth": 24,
        "screen_height": 723,
        "screen_width": 1536,
        "time_zone": 0,
        "java_enabled": true,
        "java_script_enabled": true,
        "ip_address": "127.0.0.1"
    },
    "error_code": null,
    "payment_token": "TOKEN123456789",
    "connector_metadata": null,
    "payment_experience": "redirect_to_url",
    "payment_method_type": "credit",
    "payment_method_data": {
        "card": {
            "card_number": "4242424242424242",
            "card_exp_month": "10",
            "card_cvc": "123",
            "card_exp_year": "2024",
            "card_holder_name": "John Doe"
        }
    },
    "business_sub_label": "Premium",
    "straight_through_algorithm": null,
    "preprocessing_step_id": null,
    "mandate_details": null,
    "error_reason": null,
    "multiple_capture_count": 0,
    "connector_response_reference_id": null,
    "amount_capturable": 10000,
    "updated_by": "redis_kv",
    "merchant_connector_id": "MCN123456789",
    "authentication_data": null,
    "encoded_data": null,
    "unified_code": null,
    "unified_message": null,
    "net_amount": 10200,
    "mandate_data": {
    "customer_acceptance": {
        "acceptance_type": "offline",
        "accepted_at": "1963-05-03T04:07:52.723Z",
        "online": {
            "ip_address": "127.0.0.1",
            "user_agent": "amet irure esse"
        }
    },
        "single_use": {
            "amount": 6540,
            "currency": "USD"
        }
    }
},
    "fingerprint_id": null
}"#;
    let deserialized = serde_json::from_str::<super::PaymentAttempt>(serialized_payment_attempt);

    assert!(deserialized.is_ok());
}
