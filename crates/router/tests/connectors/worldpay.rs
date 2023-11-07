use futures::future::OptionFuture;
use router::types::{self, api, storage::enums};
use serde_json::json;
use serial_test::serial;
use wiremock::{
    matchers::{body_json, method, path},
    Mock, ResponseTemplate,
};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, LocalMock, MockConfig},
};

struct Worldpay;

impl LocalMock for Worldpay {}
impl ConnectorActions for Worldpay {}
impl utils::Connector for Worldpay {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Worldpay;
        types::api::ConnectorData {
            connector: Box::new(&Worldpay),
            connector_name: types::Connector::Worldpay,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .worldpay
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "worldpay".to_string()
    }
}

#[actix_web::test]
#[serial]
async fn should_authorize_card_payment() {
    let conn = Worldpay {};
    let _mock = conn.start_server(get_mock_config()).await;
    let response = conn.authorize_payment(None, None).await.unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    assert_eq!(
        utils::get_connector_transaction_id(response.response),
        Some("123456".to_string())
    );
}

#[actix_web::test]
#[serial]
async fn should_authorize_gpay_payment() {
    let conn = Worldpay {};
    let _mock = conn.start_server(get_mock_config()).await;
    let response = conn
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Wallet(
                    api::WalletData::GooglePay(api_models::payments::GooglePayWalletData {
                        pm_type: "CARD".to_string(),
                        description: "Visa1234567890".to_string(),
                        info: api_models::payments::GooglePayPaymentMethodInfo {
                            card_network: "VISA".to_string(),
                            card_details: "1234".to_string(),
                        },
                        tokenization_data: api_models::payments::GpayTokenizationData {
                            token_type: "worldpay".to_string(),
                            token: "someToken".to_string(),
                        },
                    }),
                ),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    assert_eq!(
        utils::get_connector_transaction_id(response.response),
        Some("123456".to_string())
    );
}

#[actix_web::test]
#[serial]
async fn should_authorize_applepay_payment() {
    let conn = Worldpay {};
    let _mock = conn.start_server(get_mock_config()).await;
    let response = conn
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Wallet(
                    api::WalletData::ApplePay(api_models::payments::ApplePayWalletData {
                        payment_data: "someData".to_string(),
                        transaction_identifier: "someId".to_string(),
                        payment_method: api_models::payments::ApplepayPaymentMethod {
                            display_name: "someName".to_string(),
                            network: "visa".to_string(),
                            pm_type: "card".to_string(),
                        },
                    }),
                ),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
    assert_eq!(
        utils::get_connector_transaction_id(response.response),
        Some("123456".to_string())
    );
}

#[actix_web::test]
#[serial]
async fn should_capture_already_authorized_payment() {
    let connector = Worldpay {};
    let _mock = connector.start_server(get_mock_config()).await;
    let authorize_response = connector.authorize_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response: OptionFuture<_> = txn_id
        .map(|transaction_id| async move {
            connector
                .capture_payment(transaction_id, None, None)
                .await
                .unwrap()
                .status
        })
        .into();
    assert_eq!(response.await, Some(enums::AttemptStatus::Charged));
}

#[actix_web::test]
#[serial]
async fn should_sync_payment() {
    let connector = Worldpay {};
    let _mock = connector.start_server(get_mock_config()).await;
    let response = connector
        .sync_payment(
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    "112233".to_string(),
                ),
                ..Default::default()
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

#[actix_web::test]
#[serial]
async fn should_void_already_authorized_payment() {
    let connector = Worldpay {};
    let _mock = connector.start_server(get_mock_config()).await;
    let authorize_response = connector.authorize_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response: OptionFuture<_> = txn_id
        .map(|transaction_id| async move {
            connector
                .void_payment(transaction_id, None, None)
                .await
                .unwrap()
                .status
        })
        .into();
    assert_eq!(response.await, Some(enums::AttemptStatus::Voided));
}

#[actix_web::test]
#[serial]
async fn should_fail_capture_for_invalid_payment() {
    let connector = Worldpay {};
    let _mock = connector.start_server(get_mock_config()).await;
    let authorize_response = connector.authorize_payment(None, None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let response = connector
        .capture_payment("12345".to_string(), None, None)
        .await
        .unwrap();
    let err = response.response.unwrap_err();
    assert_eq!(
        err.message,
        "You must provide valid transaction id to capture payment".to_string()
    );
    assert_eq!(err.code, "invalid-id".to_string());
}

#[actix_web::test]
#[serial]
async fn should_refund_succeeded_payment() {
    let connector = Worldpay {};
    let _mock = connector.start_server(get_mock_config()).await;
    //make a successful payment
    let response = connector.make_payment(None, None).await.unwrap();

    //try refund for previous payment
    let transaction_id = utils::get_connector_transaction_id(response.response).unwrap();
    let response = connector
        .refund_payment(transaction_id, None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
#[serial]
async fn should_sync_refund() {
    let connector = Worldpay {};
    let _mock = connector.start_server(get_mock_config()).await;
    let response = connector
        .sync_refund("654321".to_string(), None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

fn get_mock_config() -> MockConfig {
    let authorized = json!({
        "outcome": "authorized",
        "_links": {
            "payments:cancel": {
                "href": "/payments/authorizations/cancellations/123456"
            },
            "payments:settle": {
                "href": "/payments/settlements/123456"
            },
            "payments:partialSettle": {
                "href": "/payments/settlements/partials/123456"
            },
            "payments:events": {
                "href": "/payments/events/123456"
            },
            "curies": [
                {
                    "name": "payments",
                    "href": "/rels/payments/{rel}",
                    "templated": true
                }
            ]
        }
    });
    let settled = json!({
        "_links": {
            "payments:refund": {
                "href": "/payments/settlements/refunds/full/654321"
            },
            "payments:partialRefund": {
                "href": "/payments/settlements/refunds/partials/654321"
            },
            "payments:events": {
                "href": "/payments/events/654321"
            },
            "curies": [
                {
                    "name": "payments",
                    "href": "/rels/payments/{rel}",
                    "templated": true
                }
            ]
        }
    });
    let error_resp = json!({
        "errorName": "invalid-id",
        "message": "You must provide valid transaction id to capture payment"
    });
    let partial_refund = json!({
        "_links": {
            "payments:events": {
                "href": "https://try.access.worldpay.com/payments/events/eyJrIjoiazNhYjYzMiJ9"
            },
            "curies": [{
                "name": "payments",
                "href": "https://try.access.worldpay.com/rels/payments/{rel}",
                "templated": true
            }]
        }
    });
    let partial_refund_req_body = json!({
        "value": {
            "amount": 100,
            "currency": "USD"
        },
        "reference": "123456"
    });
    let refunded = json!({
        "lastEvent": "refunded",
        "_links": {
            "payments:cancel": "/payments/authorizations/cancellations/654321",
            "payments:settle": "/payments/settlements/full/654321",
            "payments:partialSettle": "/payments/settlements/partials/654321",
            "curies": [
                {
                    "name": "payments",
                    "href": "/rels/payments/{rel}",
                    "templated": true
                }
            ]
        }
    });
    let sync_payment = json!({
        "lastEvent": "authorized",
        "_links": {
            "payments:events": "/payments/authorizations/events/654321",
            "payments:settle": "/payments/settlements/full/654321",
            "payments:partialSettle": "/payments/settlements/partials/654321",
            "curies": [
                {
                    "name": "payments",
                    "href": "/rels/payments/{rel}",
                    "templated": true
                }
            ]
        }
    });

    MockConfig {
        address: Some("127.0.0.1:9090".to_string()),
        mocks: vec![
            Mock::given(method("POST"))
                .and(path("/payments/authorizations".to_string()))
                .respond_with(ResponseTemplate::new(201).set_body_json(authorized)),
            Mock::given(method("POST"))
                .and(path("/payments/settlements/123456".to_string()))
                .respond_with(ResponseTemplate::new(202).set_body_json(settled)),
            Mock::given(method("GET"))
                .and(path("/payments/events/112233".to_string()))
                .respond_with(ResponseTemplate::new(200).set_body_json(sync_payment)),
            Mock::given(method("POST"))
                .and(path("/payments/settlements/12345".to_string()))
                .respond_with(ResponseTemplate::new(400).set_body_json(error_resp)),
            Mock::given(method("POST"))
                .and(path(
                    "/payments/settlements/refunds/partials/123456".to_string(),
                ))
                .and(body_json(partial_refund_req_body))
                .respond_with(ResponseTemplate::new(202).set_body_json(partial_refund)),
            Mock::given(method("GET"))
                .and(path("/payments/events/654321".to_string()))
                .respond_with(ResponseTemplate::new(200).set_body_json(refunded)),
        ],
    }
}
