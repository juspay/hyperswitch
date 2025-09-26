use common_enums::GooglePayCardFundingSource;
use masking::PeekInterface;
use router::types::{self, domain, storage::enums, AccessToken, ConnectorAuthType};

use crate::{
    connector_auth,
    utils::{self, Connector, ConnectorActions, PaymentAuthorizeType},
};
struct Payu;
impl ConnectorActions for Payu {}
impl Connector for Payu {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Payu;
        utils::construct_connector_data_old(
            Box::new(Payu::new()),
            types::Connector::Payu,
            types::api::GetToken::Connector,
            None,
        )
    }

    fn get_auth_token(&self) -> ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .payu
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "payu".to_string()
    }
}

fn get_access_token() -> Option<AccessToken> {
    let connector = Payu {};
    match connector.get_auth_token() {
        ConnectorAuthType::BodyKey { api_key, key1 } => Some(AccessToken {
            token: api_key,
            expires: key1.peek().parse::<i64>().unwrap(),
        }),
        _ => None,
    }
}
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        access_token: get_access_token(),
        ..Default::default()
    })
}

#[actix_web::test]
#[ignore]
async fn should_authorize_card_payment() {
    //Authorize Card Payment in PLN currency
    let authorize_response = Payu {}
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    // in Payu need Psync to get status therefore set to pending
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response.response) {
        let sync_response = Payu {}
            .psync_retry_till_status_matches(
                enums::AttemptStatus::Authorized,
                Some(types::PaymentsSyncData {
                    connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                        transaction_id.clone(),
                    ),
                    ..Default::default()
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();
        // Assert the sync response, it will be authorized in case of manual capture, for automatic it will be Completed Success
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    }
}

#[actix_web::test]
async fn should_authorize_gpay_payment() {
    let authorize_response = Payu {}
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: domain::PaymentMethodData::Wallet(
                    domain::WalletData::GooglePay(domain::GooglePayWalletData {
                        pm_type: "CARD".to_string(),
                        description: "Visa1234567890".to_string(),
                        info: domain::GooglePayPaymentMethodInfo {
                            card_network: "VISA".to_string(),
                            card_details: "1234".to_string(),
                            assurance_details: None,
                            card_funding_source: Some(GooglePayCardFundingSource::Unknown),
                        },
                        tokenization_data: common_types::payments::GpayTokenizationData::Encrypted(
                            common_types::payments::GpayEcryptedTokenizationData {
                                token_type: "worldpay".to_string(),
                                token: "someToken".to_string(),
                            },
                        ),
                    }),
                ),
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response.response) {
        let sync_response = Payu {}
            .sync_payment(
                Some(types::PaymentsSyncData {
                    connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                        transaction_id.clone(),
                    ),
                    ..Default::default()
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    }
}

#[actix_web::test]
#[ignore]
async fn should_capture_already_authorized_payment() {
    let connector = Payu {};
    let authorize_response = connector
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response.response) {
        let sync_response = connector
            .psync_retry_till_status_matches(
                enums::AttemptStatus::Authorized,
                Some(types::PaymentsSyncData {
                    connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                        transaction_id.clone(),
                    ),
                    ..Default::default()
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
        let capture_response = connector
            .capture_payment(transaction_id.clone(), None, get_default_payment_info())
            .await
            .unwrap();
        assert_eq!(capture_response.status, enums::AttemptStatus::Pending);
        let response = connector
            .psync_retry_till_status_matches(
                enums::AttemptStatus::Charged,
                Some(types::PaymentsSyncData {
                    connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                        transaction_id,
                    ),
                    ..Default::default()
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();
        assert_eq!(response.status, enums::AttemptStatus::Charged,);
    }
}

#[actix_web::test]
#[ignore]
async fn should_sync_payment() {
    let connector = Payu {};
    // Authorize the payment for manual capture
    let authorize_response = connector
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response.response) {
        // Sync the Payment Data
        let response = connector
            .psync_retry_till_status_matches(
                enums::AttemptStatus::Authorized,
                Some(types::PaymentsSyncData {
                    connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                        transaction_id,
                    ),
                    ..Default::default()
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();

        assert_eq!(response.status, enums::AttemptStatus::Authorized);
    }
}

#[actix_web::test]
#[ignore]
async fn should_void_already_authorized_payment() {
    let connector = Payu {};
    //make a successful payment
    let authorize_response = connector
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    //try CANCEL for previous payment
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response.response) {
        let void_response = connector
            .void_payment(transaction_id.clone(), None, get_default_payment_info())
            .await
            .unwrap();
        assert_eq!(void_response.status, enums::AttemptStatus::Pending);

        let sync_response = connector
            .psync_retry_till_status_matches(
                enums::AttemptStatus::Voided,
                Some(types::PaymentsSyncData {
                    connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                        transaction_id,
                    ),
                    ..Default::default()
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();
        assert_eq!(sync_response.status, enums::AttemptStatus::Voided,);
    }
}

#[actix_web::test]
#[ignore]
async fn should_refund_succeeded_payment() {
    let connector = Payu {};
    let authorize_response = connector
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response.response) {
        let sync_response = connector
            .psync_retry_till_status_matches(
                enums::AttemptStatus::Authorized,
                Some(types::PaymentsSyncData {
                    connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                        transaction_id.clone(),
                    ),
                    ..Default::default()
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
        //Capture the payment in case of Manual Capture
        let capture_response = connector
            .capture_payment(transaction_id.clone(), None, get_default_payment_info())
            .await
            .unwrap();
        assert_eq!(capture_response.status, enums::AttemptStatus::Pending);

        let sync_response = connector
            .psync_retry_till_status_matches(
                enums::AttemptStatus::Charged,
                Some(types::PaymentsSyncData {
                    connector_transaction_id: types::ResponseId::ConnectorTransactionId(
                        transaction_id.clone(),
                    ),
                    ..Default::default()
                }),
                get_default_payment_info(),
            )
            .await
            .unwrap();
        assert_eq!(sync_response.status, enums::AttemptStatus::Charged);
        //Refund the payment
        let refund_response = connector
            .refund_payment(transaction_id.clone(), None, get_default_payment_info())
            .await
            .unwrap();
        assert_eq!(
            refund_response.response.unwrap().connector_refund_id.len(),
            10
        );
    }
}

#[actix_web::test]
async fn should_sync_succeeded_refund_payment() {
    let connector = Payu {};

    //Currently hardcoding the order_id because RSync is not instant, change it accordingly
    let sync_refund_response = connector
        .sync_refund(
            "6DHQQN3T57230110GUEST000P01".to_string(),
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        sync_refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success
    );
}

#[actix_web::test]
async fn should_fail_already_refunded_payment() {
    let connector = Payu {};
    //Currently hardcoding the order_id, change it accordingly
    let response = connector
        .refund_payment(
            "5H1SVX6P7W230112GUEST000P01".to_string(),
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.reason.unwrap(), "PAID".to_string());
}
