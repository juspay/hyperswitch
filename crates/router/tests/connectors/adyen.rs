use std::str::FromStr;

use api_models::payments::{Address, AddressDetails};
use masking::Secret;
use router::types::{self, api, storage::enums, PaymentAddress};

use crate::{
    connector_auth,
    utils::{self, ConnectorActions, PaymentInfo},
};

#[derive(Clone, Copy)]
struct AdyenTest;
impl ConnectorActions for AdyenTest {}
impl utils::Connector for AdyenTest {
    fn get_data(&self) -> types::api::ConnectorData {
        use router::connector::Adyen;
        types::api::ConnectorData {
            connector: Box::new(&Adyen),
            connector_name: types::Connector::Adyen,
            get_token: types::api::GetToken::Connector,
            merchant_connector_id: None,
        }
    }

    fn get_payout_data(&self) -> Option<types::api::PayoutConnectorData> {
        use router::connector::Adyen;
        Some(types::api::PayoutConnectorData {
            connector: Box::new(&Adyen),
            connector_name: types::PayoutConnectors::Adyen,
            get_token: types::api::GetToken::Connector,
        })
    }

    fn get_auth_token(&self) -> types::ConnectorAuthType {
        utils::to_connector_auth_type(
            connector_auth::ConnectorAuthentication::new()
                .adyen_uk
                .expect("Missing connector authentication configuration")
                .into(),
        )
    }

    fn get_name(&self) -> String {
        "adyen".to_string()
    }
}

impl AdyenTest {
    fn get_payment_info() -> Option<PaymentInfo> {
        Some(PaymentInfo {
            address: Some(PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        country: Some(api_models::enums::CountryAlpha2::US),
                        state: Some(Secret::new("California".to_string())),
                        city: Some("San Francisco".to_string()),
                        zip: Some(Secret::new("94122".to_string())),
                        line1: Some(Secret::new("1467".to_string())),
                        line2: Some(Secret::new("Harrison Street".to_string())),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    fn get_payout_info(payout_type: enums::PayoutType) -> Option<PaymentInfo> {
        Some(PaymentInfo {
            country: Some(api_models::enums::CountryAlpha2::NL),
            currency: Some(enums::Currency::EUR),
            address: Some(PaymentAddress {
                billing: Some(Address {
                    address: Some(AddressDetails {
                        country: Some(api_models::enums::CountryAlpha2::US),
                        state: Some(Secret::new("California".to_string())),
                        city: Some("San Francisco".to_string()),
                        zip: Some(Secret::new("94122".to_string())),
                        line1: Some(Secret::new("1467".to_string())),
                        line2: Some(Secret::new("Harrison Street".to_string())),
                        ..Default::default()
                    }),
                    phone: None,
                }),
                ..Default::default()
            }),
            payout_method_data: match payout_type {
                enums::PayoutType::Card => {
                    Some(api::PayoutMethodData::Card(api::payouts::CardPayout {
                        card_number: cards::CardNumber::from_str("4111111111111111").unwrap(),
                        expiry_month: Secret::new("3".to_string()),
                        expiry_year: Secret::new("2030".to_string()),
                        card_holder_name: Secret::new("John Doe".to_string()),
                    }))
                }
                enums::PayoutType::Bank => Some(api::PayoutMethodData::Bank(
                    api::payouts::BankPayout::Sepa(api::SepaBankTransfer {
                        iban: "NL46TEST0136169112".to_string().into(),
                        bic: Some("ABNANL2A".to_string().into()),
                        bank_name: "Deutsche Bank".to_string(),
                        bank_country_code: enums::CountryAlpha2::NL,
                        bank_city: "Amsterdam".to_string(),
                    }),
                )),
            },
            ..Default::default()
        })
    }

    fn get_payment_authorize_data(
        card_number: &str,
        card_exp_month: &str,
        card_exp_year: &str,
        card_cvc: &str,
        capture_method: enums::CaptureMethod,
    ) -> Option<types::PaymentsAuthorizeData> {
        Some(types::PaymentsAuthorizeData {
            amount: 3500,
            currency: enums::Currency::USD,
            payment_method_data: types::api::PaymentMethodData::Card(types::api::Card {
                card_number: cards::CardNumber::from_str(card_number).unwrap(),
                card_exp_month: Secret::new(card_exp_month.to_string()),
                card_exp_year: Secret::new(card_exp_year.to_string()),
                card_holder_name: Secret::new("John Doe".to_string()),
                card_cvc: Secret::new(card_cvc.to_string()),
                card_issuer: None,
                card_network: None,
                card_type: None,
                card_issuing_country: None,
                bank_code: None,
                nick_name: Some(masking::Secret::new("nick_name".into())),
            }),
            confirm: true,
            statement_descriptor_suffix: None,
            statement_descriptor: None,
            setup_future_usage: None,
            mandate_id: None,
            off_session: None,
            setup_mandate_details: None,
            capture_method: Some(capture_method),
            browser_info: None,
            order_details: None,
            order_category: None,
            email: None,
            payment_experience: None,
            payment_method_type: None,
            session_token: None,
            enrolled_for_3ds: false,
            related_transaction_id: None,
            router_return_url: Some(String::from("http://localhost:8080")),
            webhook_url: None,
            complete_authorize_url: None,
            customer_id: None,
            surcharge_details: None,
        })
    }
}

static CONNECTOR: AdyenTest = AdyenTest {};

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(
            AdyenTest::get_payment_authorize_data(
                "4111111111111111",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            AdyenTest::get_payment_info(),
        )
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            AdyenTest::get_payment_authorize_data(
                "370000000000002",
                "03",
                "2030",
                "7373",
                enums::CaptureMethod::Manual,
            ),
            None,
            AdyenTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            AdyenTest::get_payment_authorize_data(
                "4293189100000008",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Pending);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            AdyenTest::get_payment_authorize_data(
                "4293189100000008",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            AdyenTest::get_payment_authorize_data(
                "370000000000002",
                "03",
                "2030",
                "7373",
                enums::CaptureMethod::Manual,
            ),
            None,
            Some(types::RefundsData {
                refund_amount: 1500,
                reason: Some("CUSTOMER REQUEST".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Pending,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            AdyenTest::get_payment_authorize_data(
                "2222400070000005",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            None,
            Some(types::RefundsData {
                refund_amount: 1500,
                reason: Some("CUSTOMER REQUEST".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Pending,
    );
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR
        .make_payment(
            AdyenTest::get_payment_authorize_data(
                "2222400070000005",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Manual,
            ),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(
            AdyenTest::get_payment_authorize_data(
                "2222400070000005",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Automatic,
            ),
            Some(types::RefundsData {
                refund_amount: 1000,
                reason: Some("CUSTOMER REQUEST".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Pending,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            AdyenTest::get_payment_authorize_data(
                "4293189100000008",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Automatic,
            ),
            Some(types::RefundsData {
                refund_amount: 500,
                reason: Some("CUSTOMER REQUEST".to_string()),
                ..utils::PaymentRefundType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Pending,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    let payment_info = AdyenTest::get_payment_info();
    //make a successful payment
    let response = CONNECTOR
        .make_payment(
            AdyenTest::get_payment_authorize_data(
                "2222400070000005",
                "03",
                "2030",
                "737",
                enums::CaptureMethod::Automatic,
            ),
            payment_info.clone(),
        )
        .await
        .unwrap();

    //try refund for previous payment
    let transaction_id = utils::get_connector_transaction_id(response.response).unwrap();
    for _x in 0..2 {
        let refund_response = CONNECTOR
            .refund_payment(
                transaction_id.clone(),
                Some(types::RefundsData {
                    refund_amount: 100,
                    reason: Some("CUSTOMER REQUEST".to_string()),
                    ..utils::PaymentRefundType::default().0
                }),
                payment_info.clone(),
            )
            .await
            .unwrap();
        assert_eq!(
            refund_response.response.unwrap().refund_status,
            enums::RefundStatus::Pending,
        );
    }
}

// Cards Negative scenerios
// Creates a payment with incorrect card number.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_card_number() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                router_return_url: Some(String::from("http://localhost:8080")),
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_number: cards::CardNumber::from_str("4024007134364842").unwrap(),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.response.unwrap_err().message, "Refused",);
}

// Creates a payment with incorrect CVC.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                router_return_url: Some(String::from("http://localhost:8080")),
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("12345".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "CVC is not the right length",
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                router_return_url: Some(String::from("http://localhost:8080")),
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_month: Secret::new("20".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    let errors = ["The provided Expiry Date is not valid.: Expiry month should be between 1 and 12 inclusive: 20","Refused"];
    assert!(errors.contains(&response.response.unwrap_err().message.as_str()))
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                router_return_url: Some(String::from("http://localhost:8080")),
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("2000".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            AdyenTest::get_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.response.unwrap_err().message, "Expired Card",);
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, AdyenTest::get_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("Original pspReference required for this operation")
    );
}

/******************** Payouts test cases ********************/
// Create SEPA payout
#[ignore]
#[cfg(feature = "payouts")]
#[actix_web::test]
async fn should_create_sepa_payout() {
    let payout_type = enums::PayoutType::Bank;
    let payout_info = AdyenTest::get_payout_info(payout_type);
    let response = CONNECTOR
        .create_payout(None, payout_type, payout_info)
        .await
        .expect("Payout bank creation response");
    assert_eq!(
        response.status.unwrap(),
        enums::PayoutStatus::RequiresFulfillment
    );
}

// Create and fulfill SEPA payout
#[ignore]
#[cfg(feature = "payouts")]
#[actix_web::test]
async fn should_create_and_fulfill_sepa_payout() {
    let payout_type = enums::PayoutType::Bank;
    let payout_info = AdyenTest::get_payout_info(payout_type);
    let response = CONNECTOR
        .create_and_fulfill_payout(None, payout_type, payout_info)
        .await
        .expect("Payout bank creation and fulfill response");
    assert_eq!(response.status.unwrap(), enums::PayoutStatus::Success);
}

// Verifies if card is eligible for payout
#[ignore]
#[cfg(feature = "payouts")]
#[actix_web::test]
async fn should_verify_payout_eligibility() {
    let payout_type = enums::PayoutType::Card;
    let payout_info = AdyenTest::get_payout_info(payout_type);
    let response = CONNECTOR
        .verify_payout_eligibility(payout_type, payout_info)
        .await
        .expect("Payout eligibility response");
    assert_eq!(
        response.status.unwrap(),
        enums::PayoutStatus::RequiresFulfillment
    );
}

// Fulfills card payout
#[ignore]
#[cfg(feature = "payouts")]
#[actix_web::test]
async fn should_fulfill_card_payout() {
    let payout_type = enums::PayoutType::Card;
    let payout_info: Option<PaymentInfo> = AdyenTest::get_payout_info(payout_type);
    let response = CONNECTOR
        .fulfill_payout(None, payout_type, payout_info)
        .await
        .expect("Payout fulfill response");
    assert_eq!(response.status.unwrap(), enums::PayoutStatus::Success);
}

// Cancels a created bank payout
#[ignore]
#[cfg(feature = "payouts")]
#[actix_web::test]
async fn should_create_and_cancel_created_payout() {
    let payout_type = enums::PayoutType::Bank;
    let payout_info = AdyenTest::get_payout_info(payout_type);
    let response = CONNECTOR
        .create_and_cancel_payout(None, payout_type, payout_info)
        .await
        .expect("Payout cancel response");
    assert_eq!(response.status.unwrap(), enums::PayoutStatus::Cancelled);
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
