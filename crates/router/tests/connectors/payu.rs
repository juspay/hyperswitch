use masking::PeekInterface;
use router::types::{self, api, storage::enums, AccessToken, ConnectorAuthType};

use crate::{
    connector_auth,
    utils::{self, Connector, ConnectorActions, PaymentAuthorizeType},
};
struct Payu;
impl ConnectorActions for Payu {}
impl Connector for Payu {
        /// This method returns a ConnectorData object containing information about the Payu connector,
    /// including the connector itself, the connector name, the method to get a token, and the merchant connector ID.
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Payu;
        types::api::ConnectorData {
            connector: Box::new(&Payu),
            connector_name: types::Connector::Payu,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

        /// This method retrieves the authentication token for the connector. It accesses the connector authentication configuration for the PayU authentication method, then converts it into the appropriate ConnectorAuthType.
    fn get_auth_token(&self) -> ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .payu
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

        /// This method returns the name "payu" as a String.
    fn get_name(&self) -> String {
        "payu".to_string()
    }
}

/// Retrieves an access token from the Payu API connector. 
/// If successful, returns Some(AccessToken) containing the token and its expiration time.
/// If unsuccessful, returns None.
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
/// Retrieves the default payment information, including the access token retrieved from the `get_access_token` method.
fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    Some(utils::PaymentInfo {
        access_token: get_access_token(),
        ..Default::default()
    })
}

#[actix_web::test]
#[ignore]
/// Asynchronously authorizes a card payment in PLN currency using the Payu API.
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
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
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
/// Asynchronously authorizes a payment using Google Pay wallet data. 
/// This method creates an authorization request with Google Pay wallet data, currency, and default payment information. 
/// It then awaits the authorization response and asserts that the status is pending. 
/// If a transaction ID is returned from the authorization response, it uses the transaction ID to sync the payment and asserts that the status is authorized.
async fn should_authorize_gpay_payment() {
    let authorize_response = Payu {}
        .authorize_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Wallet(api::WalletData::GooglePay(
                    api_models::payments::GooglePayWalletData {
                        pm_type: "CARD".to_string(),
                        description: "Visa1234567890".to_string(),
                        info: api_models::payments::GooglePayPaymentMethodInfo {
                            card_network: "VISA".to_string(),
                            card_details: "1234".to_string(),
                        },
                        tokenization_data: api_models::payments::GpayTokenizationData {
                            token_type: "payu".to_string(),
                            token: r#"{"signature":"MEUCIQD7Ta+d9+buesrH2KKkF+03AqTen+eHHN8KFleHoKaiVAIgGvAXyI0Vg3ws8KlF7agW/gmXJhpJOOPkqiNVbn/4f0Y\u003d","protocolVersion":"ECv1","signedMessage":"{\"encryptedMessage\":\"UcdGP9F/1loU0aXvVj6VqGRPA5EAjHYfJrXD0N+5O13RnaJXKWIjch1zzjpy9ONOZHqEGAqYKIcKcpe5ppN4Fpd0dtbm1H4u+lA+SotCff3euPV6sne22/Pl/MNgbz5QvDWR0UjcXvIKSPNwkds1Ib7QMmH4GfZ3vvn6s534hxAmcv/LlkeM4FFf6py9crJK5fDIxtxRJncfLuuPeAXkyy+u4zE33HmT34Oe5MSW/kYZVz31eWqFy2YCIjbJcC9ElMluoOKSZ305UG7tYGB1LCFGQLtLxphrhPu1lEmGEZE1t2cVDoCzjr3rm1OcfENc7eNC4S+ko6yrXh1ZX06c/F9kunyLn0dAz8K5JLIwLdjw3wPADVSd3L0eM7jkzhH80I6nWkutO0x8BFltxWl+OtzrnAe093OUncH6/DK1pCxtJaHdw1WUWrzULcdaMZmPfA\\u003d\\u003d\",\"ephemeralPublicKey\":\"BH7A1FUBWiePkjh/EYmsjY/63D/6wU+4UmkLh7WW6v7PnoqQkjrFpc4kEP5a1Op4FkIlM9LlEs3wGdFB8xIy9cM\\u003d\",\"tag\":\"e/EOsw2Y2wYpJngNWQqH7J62Fhg/tzmgDl6UFGuAN+A\\u003d\"}"}"# .to_string()//Generate new GooglePay token this is bound to expire
                        },
                    },
                )),
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
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
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
/// Asynchronously captures an already authorized payment by performing the following steps:
/// 1. Authorize the payment with the provided currency and default payment info, and assert that the status is pending.
/// 2. Retrieve the transaction ID from the authorize response and use it to sync the payment status until it is authorized.
/// 3. Capture the payment using the transaction ID and default payment info, and assert that the status is pending.
/// 4. Sync the payment status until it is charged using the transaction ID and default payment info, and assert that the status is charged.
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
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
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
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
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
/// Asynchronously checks if a payment should be synced. It authorizes the payment for manual capture,
/// checks the status, and syncs the payment data if the status is Authorized.
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
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
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
/// Asynchronously voids an already authorized payment by making a successful payment, then attempting to cancel the payment and verifying the void status.
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
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
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
/// Asynchronously performs a series of payment operations including authorization, capture, and refund. 
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
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
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
                    connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
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
/// Asynchronously performs a refund payment synchronization and asserts that the refund status is a success.
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
/// This method attempts to refund a payment that has already been refunded. It first creates a Payu connector, then calls the refund_payment method with a hardcoded order_id (which should be changed accordingly), None for the refund amount, and default payment information. It awaits the response and unwraps it, then checks if the response has unwrapped error and asserts that the reason for the error is "PAID".
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
