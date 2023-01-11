use futures::future::OptionFuture;
use masking::Secret;
use router::types::{self, api, storage::enums};
use storage_models::schema::temp_card::txn_id;

use crate::{
    connector_auth,
    utils::{self, ConnectorActions},
};

struct Payu;
impl utils::ConnectorActions for Payu {}
impl utils::Connector for Payu {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Payu;
        types::api::ConnectorData {
            connector: Box::new(&Payu),
            connector_name: types::Connector::Payu,
            get_token: types::api::GetToken::Connector,
        }
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        types::ConnectorAuthType::from(
            connector_auth::ConnectorAuthentication::new()
                .payu
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "payu".to_string()
    }
}

#[actix_web::test]
async fn should_authorize_card_payment() {
    let authorize_response = Payu {}.authorize_payment(None).await;
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let sync_response = Payu {}
            .sync_payment(Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                encoded_data: None,
            }))
            .await;
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    }
}

#[actix_web::test]
async fn should_authorize_gpay_payment() {
    let authorize_response = Payu {}.authorize_payment(Some(types::PaymentsAuthorizeData{
            payment_method_data: types::api::PaymentMethod::Wallet(api::WalletData{
                    issuer_name: api_models::enums::WalletIssuer::GooglePay,
                    token: Some("eyJzaWduYXR1cmUiOiJNRVFDSUEwYWN1RGF3SkxYbDZXSEhKMU5oWWJJdHU2cElnaUcwYjlmNHY2Q1ZpMlpBaUJKbE9SR0Z0ME5kVlp0T0h4QTJQa3NDY3ZFSERhVGt4eHFsVnNiTVRnc1dRXHUwMDNkXHUwMDNkIiwicHJvdG9jb2xWZXJzaW9uIjoiRUN2MSIsInNpZ25lZE1lc3NhZ2UiOiJ7XCJlbmNyeXB0ZWRNZXNzYWdlXCI6XCJyRkoxT1haOXNzQjdKTkxvV0pLVklLWGQ2RnZzeWwrUW5naUY3UWU1RlEvZWJMVXdWOGNUdnZmSnM4T0ptcEVWNGt5M2t5MCttNjhlanVXTlhrYm1lWmRmTFdVeEtFREkxVG5MMjYwVWtvZ1NNRDc1VEUyaVYwbFdDY2xKSnl0RXdmR0JmTWZYUVNPSGpUempOYTlTZmtyT21LTk0rTDRsNGlqNFNXWFZaUnlEVmZnajZ6TnNaV0hhbUZjZUZTLzFmOGRheHFSQzRTT2d5SEVjQ0ZrVEZ0RUFONk1HRlFVd2NOY2hRZml1TTliL3lqYmJKVXE1aEtZbXFPMXg1K0hxWE9wVHhkeWFSUTFDeFJoQWJZdi9ZU2xMaU5Ja09PZ1hnRjBkKytkTnhIcHBDTVVnbkRITytSQzZiaXoyZnFiRXFQWUgvVlRNNTFuRmRkRlcwVk1CWUxlcC9hTkRBak9OSUc4WjlJZ1c0ajhnTldBTWlUVm5xM3NjcDVvTDhyMHh5M1VtQnFYMnlPUCtaVHZneGdxYys0ZHhrTWhzWVVBcWpnUmpMa1BzNk1zZnhLaGUyODFpL0pmRlcxY2VSUW9uQkFcXHUwMDNkXFx1MDAzZFwiLFwiZXBoZW1lcmFsUHVibGljS2V5XCI6XCJCTUt0VnozQ3ZZYWNKOWVBN0pwWkVSUVVHMkIvaDFKZU1UQkdVc09wbERjcG50dVEwM0hjRXd3K1ZrRlBXVUlKZlJ3WnZyVjFOaVlGNm9iaWxobTBZNjhcXHUwMDNkXCIsXCJ0YWdcIjpcIlI3MWFVVGVrbzRGZVBibXhkekdtZVpDcS9VckVhK2dHd3VkT2RBUE9ZNEFcXHUwMDNkXCJ9In="
                    .to_string())
                }),
            amount: 100,
            currency: enums::Currency::PLN,
            confirm: true,
            statement_descriptor_suffix: None,
            capture_method: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            browser_info: None,
            order_details: None,
            email: None,
    })).await;
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let sync_response = Payu {}
            .sync_payment(Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                encoded_data: None,
            }))
            .await;
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    }
}

#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let connector = Payu {};
    let authorize_response = connector.authorize_payment(None).await;
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let sync_response = connector
            .sync_payment(Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                encoded_data: None,
            }))
            .await;
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
        let capture_response = connector
            .capture_payment(transaction_id.clone(), None)
            .await;
        assert_eq!(capture_response.status, enums::AttemptStatus::Pending);
        let response = connector
            .sync_payment(Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id,
                ),
                encoded_data: None,
            }))
            .await;
        assert_eq!(response.status, enums::AttemptStatus::Charged,);
    }
}

#[actix_web::test]
async fn should_sync_payment() {
    let connector = Payu {};
    let authorize_response = connector.authorize_payment(None).await;
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let response = connector
            .sync_payment(Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id,
                ),
                encoded_data: None,
            }))
            .await;

        assert_eq!(response.status, enums::AttemptStatus::Authorized,);
    }
}

#[actix_web::test]
async fn should_void_already_authorized_payment() {
    let connector = Payu {};
    //make a successful payment
    let authorize_response = connector.make_payment(None).await;
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    //try CANCEL for previous payment
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let void_response = connector.void_payment(transaction_id.clone(), None).await;
        assert_eq!(void_response.status, enums::AttemptStatus::Pending);

        let sync_response = connector
            .sync_payment(Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id,
                ),
                encoded_data: None,
            }))
            .await;
        assert_eq!(sync_response.status, enums::AttemptStatus::Voided,);
    }
}

#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let connector = Payu {};
    //make a successful payment
    let authorize_response = connector.make_payment(None).await;
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    //try refund for previous payment
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let capture_response = connector
            .capture_payment(transaction_id.clone(), None)
            .await;
        assert_eq!(capture_response.status, enums::AttemptStatus::Pending);

        let sync_response = connector
            .sync_payment(Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    transaction_id.clone(),
                ),
                encoded_data: None,
            }))
            .await;
        assert_eq!(sync_response.status, enums::AttemptStatus::Charged);

        let refund_response = connector.refund_payment(transaction_id.clone(), None).await;
        assert_eq!(
            refund_response.response.unwrap().connector_refund_id.len(),
            10
        );
    }
}

#[actix_web::test]
async fn should_sync_succeeded_refund_payment() {
    let connector = Payu {};

    let sync_refund_response = connector
        .sync_refund("6DHQQN3T57230110GUEST000P01".to_string(), None)
        .await;
    assert_eq!(
        sync_refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success
    );
}

#[actix_web::test]
async fn should_fail_already_refunded_payment() {
    let connector = Payu {};
    let response = connector
        .refund_payment("6DHQQN3T57230110GUEST000P01".to_string(), None)
        .await;
    let x = response.response.unwrap_err();
    assert_eq!(x.reason.unwrap(), "PAID".to_string());
}
