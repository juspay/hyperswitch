use router::types::{self, api, storage::enums};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentAuthorizeType},
};

struct Payu;
impl ConnectorActions for Payu {}
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
    //Authorize Card Payment in PLN currency
    let authorize_response = Payu {}
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    // in Payu need Psync to get status therefore set to pending
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let sync_response = Payu {}
            .sync_payment(
                Some(types::PaymentsSyncData {
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                        transaction_id.clone(),
                    ),
                    encoded_data: None,
                    capture_method: None,
                }),
                None,
            )
            .await
            .unwrap();
        // Assert the sync response, it will be authorized in case of manual capture, for automatic it will be Completed Success
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    }
}

#[actix_web::test]
async fn should_authorize_gpay_payment() {
    let authorize_response = Payu {}.authorize_payment(Some(types::PaymentsAuthorizeData{
            payment_method_data: types::api::PaymentMethod::Wallet(api::WalletData{
                    issuer_name: api_models::enums::WalletIssuer::GooglePay,
                    token: Some(r#"{"signature":"MEUCIQD7Ta+d9+buesrH2KKkF+03AqTen+eHHN8KFleHoKaiVAIgGvAXyI0Vg3ws8KlF7agW/gmXJhpJOOPkqiNVbn/4f0Y\u003d","protocolVersion":"ECv1","signedMessage":"{\"encryptedMessage\":\"UcdGP9F/1loU0aXvVj6VqGRPA5EAjHYfJrXD0N+5O13RnaJXKWIjch1zzjpy9ONOZHqEGAqYKIcKcpe5ppN4Fpd0dtbm1H4u+lA+SotCff3euPV6sne22/Pl/MNgbz5QvDWR0UjcXvIKSPNwkds1Ib7QMmH4GfZ3vvn6s534hxAmcv/LlkeM4FFf6py9crJK5fDIxtxRJncfLuuPeAXkyy+u4zE33HmT34Oe5MSW/kYZVz31eWqFy2YCIjbJcC9ElMluoOKSZ305UG7tYGB1LCFGQLtLxphrhPu1lEmGEZE1t2cVDoCzjr3rm1OcfENc7eNC4S+ko6yrXh1ZX06c/F9kunyLn0dAz8K5JLIwLdjw3wPADVSd3L0eM7jkzhH80I6nWkutO0x8BFltxWl+OtzrnAe093OUncH6/DK1pCxtJaHdw1WUWrzULcdaMZmPfA\\u003d\\u003d\",\"ephemeralPublicKey\":\"BH7A1FUBWiePkjh/EYmsjY/63D/6wU+4UmkLh7WW6v7PnoqQkjrFpc4kEP5a1Op4FkIlM9LlEs3wGdFB8xIy9cM\\u003d\",\"tag\":\"e/EOsw2Y2wYpJngNWQqH7J62Fhg/tzmgDl6UFGuAN+A\\u003d\"}"}"#
                    .to_string()) //Generate new GooglePay token this is bound to expire
                }),
            currency: enums::Currency::PLN,
            ..PaymentAuthorizeType::default().0
    }), None).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let sync_response = Payu {}
            .sync_payment(
                Some(types::PaymentsSyncData {
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                        transaction_id.clone(),
                    ),
                    encoded_data: None,
                    capture_method: None,
                }),
                None,
            )
            .await
            .unwrap();
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
    }
}

#[actix_web::test]
async fn should_capture_already_authorized_payment() {
    let connector = Payu {};
    let authorize_response = connector
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let sync_response = connector
            .sync_payment(
                Some(types::PaymentsSyncData {
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                        transaction_id.clone(),
                    ),
                    encoded_data: None,
                    capture_method: None,
                }),
                None,
            )
            .await
            .unwrap();
        assert_eq!(sync_response.status, enums::AttemptStatus::Authorized);
        let capture_response = connector
            .capture_payment(transaction_id.clone(), None, None)
            .await
            .unwrap();
        assert_eq!(capture_response.status, enums::AttemptStatus::Pending);
        let response = connector
            .sync_payment(
                Some(types::PaymentsSyncData {
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                        transaction_id,
                    ),
                    encoded_data: None,
                    capture_method: None,
                }),
                None,
            )
            .await
            .unwrap();
        assert_eq!(response.status, enums::AttemptStatus::Charged,);
    }
}

#[actix_web::test]
async fn should_sync_payment() {
    let connector = Payu {};
    // Authorize the payment for manual capture
    let authorize_response = connector
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        // Sync the Payment Data
        let response = connector
            .sync_payment(
                Some(types::PaymentsSyncData {
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                        transaction_id,
                    ),
                    encoded_data: None,
                    capture_method: None,
                }),
                None,
            )
            .await
            .unwrap();

        assert_eq!(response.status, enums::AttemptStatus::Authorized);
    }
}

#[actix_web::test]
async fn should_void_already_authorized_payment() {
    let connector = Payu {};
    //make a successful payment
    let authorize_response = connector
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    //try CANCEL for previous payment
    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        let void_response = connector
            .void_payment(transaction_id.clone(), None, None)
            .await
            .unwrap();
        assert_eq!(void_response.status, enums::AttemptStatus::Pending);

        let sync_response = connector
            .sync_payment(
                Some(types::PaymentsSyncData {
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                        transaction_id,
                    ),
                    encoded_data: None,
                    capture_method: None,
                }),
                None,
            )
            .await
            .unwrap();
        assert_eq!(sync_response.status, enums::AttemptStatus::Voided,);
    }
}

#[actix_web::test]
async fn should_refund_succeeded_payment() {
    let connector = Payu {};
    //make a successful payment
    let authorize_response = connector
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                currency: enums::Currency::PLN,
                ..PaymentAuthorizeType::default().0
            }),
            None,
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Pending);

    if let Some(transaction_id) = utils::get_connector_transaction_id(authorize_response) {
        //Capture the payment in case of Manual Capture
        let capture_response = connector
            .capture_payment(transaction_id.clone(), None, None)
            .await
            .unwrap();
        assert_eq!(capture_response.status, enums::AttemptStatus::Pending);

        let sync_response = connector
            .sync_payment(
                Some(types::PaymentsSyncData {
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                        transaction_id.clone(),
                    ),
                    encoded_data: None,
                    capture_method: None,
                }),
                None,
            )
            .await
            .unwrap();
        assert_eq!(sync_response.status, enums::AttemptStatus::Charged);

        //Refund the payment
        let refund_response = connector
            .refund_payment(transaction_id.clone(), None, None)
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
        .sync_refund("6DHQQN3T57230110GUEST000P01".to_string(), None, None)
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
        .refund_payment("5H1SVX6P7W230112GUEST000P01".to_string(), None, None)
        .await
        .unwrap();
    let x = response.response.unwrap_err();
    assert_eq!(x.reason.unwrap(), "PAID".to_string());
}
