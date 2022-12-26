use futures::future::OptionFuture;
use masking::Secret;
use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

struct {{project-name | downcase | pascal_case}};
impl utils::ConnectorActions for {{project-name | downcase | pascal_case}} {}
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

#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = {{project-name | downcase | pascal_case}} {}.authorize_payment(None).await;
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

#[actix_web::test]
async fn should_authorize_and_capture_payment() {
    let response = {{project-name | downcase | pascal_case}} {}.make_payment(None).await;
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let connector = {{project-name | downcase | pascal_case}} {};
    let authorize_response = connector.authorize_payment(None).await;
    assert_eq!(authorize_response.status, enums::AttemptStatus::Authorized);
    let txn_id = utils::get_connector_transaction_id(authorize_response);
    let response: OptionFuture<_> = txn_id
        .map(|transaction_id| async move {
            connector.capture_payment(transaction_id, None).await.status
        })
        .into();
    assert_eq!(response.await, Some(enums::AttemptStatus::Charged));
}

#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = {{project-name | downcase | pascal_case}} {}.make_payment(Some(types::PaymentsAuthorizeData {
            payment_method_data: types::api::PaymentMethod::Card(api::CCard {
                card_number: Secret::new("4024007134364842".to_string()),
                ..utils::CCardType::default().0
            }),
            ..utils::PaymentAuthorizeType::default().0
        }))
        .await;
    let x = response.response.unwrap_err();
    assert_eq!(
        x.message,
        "The card's security code failed verification.".to_string(),
    );
}

#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let connector = {{project-name | downcase | pascal_case}} {};
    //make a successful payment
    let response = connector.make_payment(None).await;

    //try refund for previous payment
    if let Some(transaction_id) = utils::get_connector_transaction_id(response) {
        let response = connector.refund_payment(transaction_id, None).await;
        assert_eq!(
            response.response.unwrap().refund_status,
            enums::RefundStatus::Success,
        );
    }
}
