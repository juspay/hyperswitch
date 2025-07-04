use crate::connector_auth::ConnectorAuthentication;

#[test]
fn should_use_correct_auth_type() {
    let connector = super::Mpgs::new();
    let auth = ConnectorAuthentication::from(&connector.id());
    assert_eq!(
        auth,
        ConnectorAuthentication::HeaderKey
    );
}

#[cfg(test)]
mod transformer_tests {
    use super::*;
    use crate::connectors::mpgs::transformers::*;
    use common_utils::types::StringMinorUnit;
    use hyperswitch_domain_models::{
        payment_method_data::{Card, PaymentMethodData},
        router_data::ConnectorAuthType,
        router_request_types::{CaptureMethod, PaymentsAuthorizeData},
    };
    use masking::Secret;

    #[test]
    fn test_auth_type_transformation() {
        let auth = ConnectorAuthType::HeaderKey {
            api_key: Secret::new("merchant.TEST_MERCHANT:password123".to_string()),
        };

        let result = MpgsAuthType::try_from(&auth);
        assert!(result.is_ok());
        let auth_type = result.unwrap();
        assert_eq!(auth_type.api_key.peek(), "merchant.TEST_MERCHANT:password123");
    }

    #[test]
    fn test_payment_request_transformation() {
        let card = Card {
            card_number: cards::CardNumber::from("4111111111111111"),
            card_exp_month: Secret::new("12".to_string()),
            card_exp_year: Secret::new("25".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_issuer: None,
            card_network: None,
            bank_code: None,
            nick_name: None,
            card_issuing_country: None,
            card_holder_name: None,
            card_type: None,
        };

        let router_data = MpgsRouterData {
            amount: "100.00".to_string(),
            router_data: &PaymentsAuthorizeRouterData {
                request: PaymentsAuthorizeData {
                    amount: 10000, // $100.00 in cents
                    currency: common_enums::Currency::USD,
                    payment_method_data: PaymentMethodData::Card(card),
                    capture_method: Some(CaptureMethod::Automatic),
                    email: Some(common_utils::pii::Email::from("test@example.com")),
                    ..Default::default()
                },
                connector_request_reference_id: "ORDER123".to_string(),
                payment_id: "PAY123".to_string(),
                ..Default::default()
            },
        };

        let result = MpgsPaymentsRequest::try_from(&router_data);
        assert!(result.is_ok());
        
        let request = result.unwrap();
        match request.api_operation {
            MpgsApiOperation::Pay => {},
            _ => panic!("Expected PAY operation for automatic capture"),
        }
        assert_eq!(request.order.amount, "100.00");
        assert_eq!(request.order.currency, "USD");
        assert_eq!(request.source_of_funds.source_type, "CARD");
    }

    #[test]
    fn test_capture_request_transformation() {
        use hyperswitch_domain_models::router_request_types::PaymentsCaptureData;
        use crate::connectors::mpgs::transformers::MpgsCaptureRequest;

        let router_data = MpgsRouterData {
            amount: "50.00".to_string(),
            router_data: &PaymentsCaptureRouterData {
                request: PaymentsCaptureData {
                    amount_to_capture: 5000, // $50.00 in cents
                    currency: common_enums::Currency::USD,
                    connector_transaction_id: "TXN123".to_string(),
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let result = MpgsCaptureRequest::try_from(&router_data);
        assert!(result.is_ok());
        
        let request = result.unwrap();
        match request.api_operation {
            MpgsApiOperation::Capture => {},
            _ => panic!("Expected CAPTURE operation"),
        }
        assert_eq!(request.transaction.amount, "50.00");
        assert_eq!(request.transaction.currency, "USD");
    }

    #[test]
    fn test_refund_request_transformation() {
        use hyperswitch_domain_models::router_request_types::RefundsData;

        let router_data = MpgsRouterData {
            amount: "25.00".to_string(),
            router_data: &RefundsRouterData {
                request: RefundsData {
                    refund_amount: 2500, // $25.00 in cents
                    currency: common_enums::Currency::USD,
                    connector_transaction_id: Some("TXN123".to_string()),
                    ..Default::default()
                },
                ..Default::default()
            },
        };

        let result = MpgsRefundRequest::try_from(&router_data);
        assert!(result.is_ok());
        
        let request = result.unwrap();
        match request.api_operation {
            MpgsApiOperation::Refund => {},
            _ => panic!("Expected REFUND operation"),
        }
        assert_eq!(request.transaction.amount, "25.00");
        assert_eq!(request.transaction.currency, "USD");
        assert_eq!(request.transaction.target_transaction_id, Some("TXN123".to_string()));
    }

    #[test]
    fn test_payment_response_transformation() {
        use crate::types::ResponseRouterData;
        use hyperswitch_domain_models::router_data::RouterData;
        use hyperswitch_domain_models::router_response_types::PaymentsResponseData;

        let response = MpgsPaymentsResponse {
            result: "SUCCESS".to_string(),
            merchant: Some("TEST_MERCHANT".to_string()),
            order: MpgsOrderResponse {
                amount: 100.00,
                currency: "USD".to_string(),
                id: "ORDER123".to_string(),
                status: "CAPTURED".to_string(),
                total_authorized_amount: Some(100.00),
                total_captured_amount: Some(100.00),
            },
            transaction: MpgsTransactionResponse {
                id: "TXN123".to_string(),
                transaction_type: "PAYMENT".to_string(),
                authorization_code: Some("AUTH123".to_string()),
                acquirer_reference: Some("ACQ123".to_string()),
            },
            response: MpgsGatewayResponse {
                gateway_code: "APPROVED".to_string(),
                acquirer_code: Some("00".to_string()),
                acquirer_message: Some("Approved".to_string()),
            },
            authentication: None,
        };

        let router_data = ResponseRouterData {
            response,
            data: PaymentsAuthorizeRouterData::default(),
            http_code: 200,
        };

        let result: Result<RouterData<_, _, PaymentsResponseData>, _> = router_data.try_into();
        assert!(result.is_ok());
        
        let transformed = result.unwrap();
        assert_eq!(transformed.status, common_enums::AttemptStatus::Charged);
    }

    #[test]
    fn test_url_building() {
        let auth = ConnectorAuthType::HeaderKey {
            api_key: Secret::new("merchant.TEST_MERCHANT:password123".to_string()),
        };

        // Test payment URL
        let payment_url = get_payment_url(&auth, "PAY123", Some(CaptureMethod::Automatic));
        assert!(payment_url.is_ok());
        let url = payment_url.unwrap();
        assert!(url.contains("/api/rest/version/73/merchant/TEST_MERCHANT"));
        assert!(url.contains("/order/PAY123/transaction/"));
        assert!(url.contains("?operation=pay"));

        // Test payment sync URL
        let sync_url = get_payment_sync_url(&auth, "ORDER123");
        assert!(sync_url.is_ok());
        let url = sync_url.unwrap();
        assert_eq!(url, "/api/rest/version/73/merchant/TEST_MERCHANT/order/ORDER123");

        // Test capture URL
        let capture_url = get_capture_url(&auth, "ORDER123", "TXN123");
        assert!(capture_url.is_ok());
        let url = capture_url.unwrap();
        assert_eq!(url, "/api/rest/version/73/merchant/TEST_MERCHANT/order/ORDER123/transaction/TXN123?operation=capture");

        // Test refund URL
        let refund_url = get_refund_url(&auth, "ORDER123", "REFUND123");
        assert!(refund_url.is_ok());
        let url = refund_url.unwrap();
        assert_eq!(url, "/api/rest/version/73/merchant/TEST_MERCHANT/order/ORDER123/transaction/REFUND123?operation=refund");
    }

    #[test]
    fn test_error_response_transformation() {
        let error = MpgsErrorResponse {
            error: MpgsError {
                cause: "INVALID_REQUEST".to_string(),
                explanation: "The card number is invalid".to_string(),
                field: Some("sourceOfFunds.provided.card.number".to_string()),
                validation_error: Some("Card number failed validation".to_string()),
            },
        };

        assert_eq!(error.error.cause, "INVALID_REQUEST");
        assert_eq!(error.error.explanation, "The card number is invalid");
        assert_eq!(error.error.field, Some("sourceOfFunds.provided.card.number".to_string()));
    }

    #[test]
    fn test_status_mapping() {
        // Test SUCCESS with APPROVED
        let response = MpgsPaymentsResponse {
            result: "SUCCESS".to_string(),
            response: MpgsGatewayResponse {
                gateway_code: "APPROVED".to_string(),
                acquirer_code: None,
                acquirer_message: None,
            },
            transaction: MpgsTransactionResponse {
                transaction_type: "AUTHORIZATION".to_string(),
                id: "TXN123".to_string(),
                authorization_code: None,
                acquirer_reference: None,
            },
            merchant: None,
            order: MpgsOrderResponse {
                amount: 100.0,
                currency: "USD".to_string(),
                id: "ORDER123".to_string(),
                status: "AUTHORIZED".to_string(),
                total_authorized_amount: None,
                total_captured_amount: None,
            },
            authentication: None,
        };

        let router_data = ResponseRouterData {
            response,
            data: PaymentsAuthorizeRouterData::default(),
            http_code: 200,
        };

        let result: Result<RouterData<_, _, PaymentsResponseData>, _> = router_data.try_into();
        assert!(result.is_ok());
        assert_eq!(result.unwrap().status, common_enums::AttemptStatus::Authorized);
    }

    #[test]
    fn test_authentication_required_status() {
        let response = MpgsPaymentsResponse {
            result: "PENDING".to_string(),
            response: MpgsGatewayResponse {
                gateway_code: "AUTHENTICATION_REQUIRED".to_string(),
                acquirer_code: None,
                acquirer_message: None,
            },
            authentication: Some(MpgsAuthenticationResponse {
                redirect_url: Some("https://3ds.example.com/auth".to_string()),
            }),
            transaction: MpgsTransactionResponse {
                transaction_type: "PAYMENT".to_string(),
                id: "TXN123".to_string(),
                authorization_code: None,
                acquirer_reference: None,
            },
            merchant: None,
            order: MpgsOrderResponse {
                amount: 100.0,
                currency: "USD".to_string(),
                id: "ORDER123".to_string(),
                status: "PENDING".to_string(),
                total_authorized_amount: None,
                total_captured_amount: None,
            },
        };

        let router_data = ResponseRouterData {
            response,
            data: PaymentsAuthorizeRouterData::default(),
            http_code: 200,
        };

        let result: Result<RouterData<_, _, PaymentsResponseData>, _> = router_data.try_into();
        assert!(result.is_ok());
        let transformed = result.unwrap();
        assert_eq!(transformed.status, common_enums::AttemptStatus::AuthenticationPending);
        
        // Verify redirect data is populated
        if let Ok(PaymentsResponseData::TransactionResponse { redirection_data, .. }) = &transformed.response {
            assert!(redirection_data.is_some());
        } else {
            panic!("Expected TransactionResponse");
        }
    }
}
