use super::*;

/// Set of tests to extract billing details from payment method data.
/// These are required for backwards compatibility.
mod billing_from_payment_method_data;
mod null_object;
#[cfg(feature = "v1")]
mod request;
#[cfg(feature = "v1")]
mod request_api_contract;
mod response_api_contract;

#[test]
fn test_mandate_type() {
    let mandate_type = MandateType::default();
    assert_eq!(
        serde_json::to_string(&mandate_type).unwrap(),
        r#"{"multi_use":null}"#
    )
}

#[test]
fn apple_pay_connector_configured_for_decrypted_token() {
    let payment_request_data = PaymentRequestMetadata {
        supported_networks: vec![],
        merchant_capabilities: vec![],
        label: "Test merchant".to_string(),
    };
    let manual = |payment_processing_details_at| ApplePayCombinedMetadata::Manual {
        payment_request_data: payment_request_data.clone(),
        session_token_data: SessionTokenInfo {
            certificate: Secret::new("test-certificate".to_string()),
            certificate_keys: Secret::new("test-key".to_string()),
            merchant_identifier: "test-merchant".to_string(),
            display_name: "Test merchant".to_string(),
            initiative: ApplepayInitiative::Web,
            initiative_context: None,
            merchant_business_country: None,
            payment_processing_details_at,
            payment_processing_detail_input_type: None,
        },
    };
    let simplified = ApplePayCombinedMetadata::Simplified {
        payment_request_data: payment_request_data.clone(),
        session_token_data: SessionTokenForSimplifiedApplePay {
            initiative_context: "example.com".to_string(),
            merchant_business_country: None,
        },
    };
    let hyperswitch = PaymentProcessingDetailsAt::Hyperswitch(PaymentProcessingDetails {
        payment_processing_certificate: Secret::new("test-processing-certificate".to_string()),
        payment_processing_certificate_key: Secret::new("test-processing-key".to_string()),
    });
    let connector = PaymentProcessingDetailsAt::Connector;

    for (data, support_predecrypted_token, expected) in [
        (None, None, false),
        (None, Some(true), true),
        (Some(simplified), None, true),
        (Some(manual(None)), None, false),
        (Some(manual(Some(connector.clone()))), Some(false), false),
        (Some(manual(Some(connector))), Some(true), true),
        (Some(manual(Some(hyperswitch))), Some(false), true),
    ] {
        let wrapper = ApplePayCombinedWrapper {
            data,
            support_predecrypted_token,
        };
        assert_eq!(wrapper.connector_configured_for_decrypted_token(), expected);
    }
}

#[test]
fn google_pay_connector_configured_for_decrypted_token() {
    use serde_json::json;

    for (tokenization_type, parameters, expected) in [
        ("PAYMENT_GATEWAY", json!({}), false),
        ("INTERNAL_GATEWAY", json!({}), true),
        ("DIRECT", json!({}), false),
        ("DIRECT", json!({ "private_key": "test-key" }), false),
        ("DIRECT", json!({ "recipient_id": "test-recipient" }), false),
        (
            "DIRECT",
            json!({ "private_key": "test-key", "recipient_id": "test-recipient" }),
            true,
        ),
    ] {
        let details: GooglePayWalletDetails = serde_json::from_value(json!({
            "google_pay": {
                "provider_details": {
                    "merchant_info": {
                        "merchant_name": "Test merchant",
                        "tokenization_specification": {
                            "type": tokenization_type,
                            "parameters": parameters,
                        },
                    },
                },
                "cards": {
                    "allowed_auth_methods": ["PAN_ONLY"],
                    "allowed_card_networks": ["VISA"],
                },
            },
        }))
        .unwrap();
        assert_eq!(
            details.connector_configured_for_decrypted_token(),
            expected,
            "{tokenization_type}",
        );
    }
}
