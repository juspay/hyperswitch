use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};
use actix::clock::sleep;
use masking::Secret;
use router::types::{self, api, storage::enums};
use std::{time::Duration};

struct {{project-name | downcase | pascal_case}};
impl ConnectorActions for {{project-name | downcase | pascal_case}} {}
impl utils::Connector for {{project-name | downcase | pascal_case}} {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::{{project-name | downcase | pascal_case}};
        types::api::ConnectorData {
            connector: Box::new(&{{project-name | downcase | pascal_case}}),
            connector_name: types::Connector::{{project-name | downcase | pascal_case}},
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .{{project-name | downcase }}
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "{{project-name | downcase }}".to_string()
    }
}

fn None -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethod::Card(api::CCard {
            card_number: Secret::new("4242424242424242".to_string()),
            ..utils::CCardType::default().0
        }),
        ..utils::PaymentAuthorizeType::default().0
    })
}

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = {{project-name | downcase | pascal_case}} {}
        .authorize_payment(None, None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_authorize_and_capture_payment() {
    let response = {{project-name | downcase | pascal_case}} {}
        .make_payment(None, None)
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let response = connector
        .authorize_and_capture_payment(None, None, None)
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_partially_capture_already_authorized_payment() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let response = connector
        .authorize_and_capture_payment(
            None,
            Some(types::PaymentsCaptureData {
                amount_to_capture: Some(50),
                ..utils::PaymentCaptureType::default().0
            }),
            None,
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_sync_payment() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let authorize_response = connector
        .authorize_payment(None, None)
        .await
        .unwrap();
    let txn_id = utils::get_connector_transaction_id(authorize_response);
    let response = connector
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                encoded_data: None,
                capture_method: None,
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

#[actix_web::test]
async fn should_void_already_authorized_payment() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let response = connector
        .authorize_and_void_payment(
            None,
            Some(types::PaymentsCancelData {
                connector_transaction_id: "".to_string(),
                cancellation_reason: Some("requested_by_customer".to_string()),
            }),
            None,
        )
        .await;
    assert_eq!(response.unwrap().status, enums::AttemptStatus::Voided);
}

#[actix_web::test]
async fn should_fail_payment_for_incorrect_card_number() {
    let response = {{project-name | downcase | pascal_case}} {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("4024007134364842".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(
        x.message,
        r#"connector-error-message"#,
    );
}

#[actix_web::test]
async fn should_fail_payment_for_no_card_number() {
    let response = {{project-name | downcase | pascal_case}} {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_number: Secret::new("".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(
        x.message,
        r#"connector-error-message"#,
    );
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = {{project-name | downcase | pascal_case}} {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_exp_month: Secret::new("13".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, r#"connector-error-message"#,);
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_year() {
    let response = {{project-name | downcase | pascal_case}} {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_exp_year: Secret::new("2022".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, r#"connector-error-message"#,);
}

#[actix_web::test]
async fn should_fail_payment_for_invalid_card_cvc() {
    let response = {{project-name | downcase | pascal_case}} {}
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                    card_cvc: Secret::new("12".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.message, r#"connector-error-message"#,);
}

#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let authorize_response = connector
        .authorize_payment(None, None)
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    sleep(Duration::from_secs(5)).await; // to avoid 404 error
    let response = connector
        .capture_payment("12345".to_string(), None, None)
        .await
        .unwrap();
    let err = response.response.unwrap_err();
    assert_eq!(err.message, "No such payment_intent: '12345'".to_string());
    assert_eq!(err.code, "resource_missing".to_string());
}

#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let response = connector
        .make_payment_and_refund(None, None, None)
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let refund_response = connector
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    let connector = {{project-name | downcase | pascal_case}} {};
    connector
        .make_payment_and_multiple_refund(None, Some(types::RefundsData {
            refund_amount: 50,
            ..utils::PaymentRefundType::default().0
        }), None)
        .await;
}

#[actix_web::test]
async fn should_fail_refund_for_invalid_amount() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let response = connector
        .make_payment_and_refund(
            None,
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        r#"connector-error-message"#,
    );
}

#[actix_web::test]
async fn should_sync_refund() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let refund_response = connector
        .make_payment_and_refund(None, None, None)
        .await
        .unwrap();
    let response = connector
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            None,
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}
