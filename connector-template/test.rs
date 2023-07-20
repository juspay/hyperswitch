use masking::Secret;
use router::{
    core::utils as core_utils,
    types::{self, api, storage::enums,
}};

use crate::utils::{self, ConnectorActions};
use test_utils::connector_auth;

#[derive(Clone, Copy)]
struct {{project-name | downcase | pascal_case}}Test;
impl ConnectorActions for {{project-name | downcase | pascal_case}}Test {}
impl utils::Connector for {{project-name | downcase | pascal_case}}Test {
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
                .{{project-name | downcase}}
                .expect("Missing connector authentication configuration"),
        )
    }

    fn get_name(&self) -> String {
        "{{project-name | downcase}}".to_string()
    }
}

static CONNECTOR: {{project-name | downcase | pascal_case}}Test = {{project-name | downcase | pascal_case}}Test {};

fn get_default_payment_info() -> Option<utils::PaymentInfo> {
    None
}

fn payment_method_details() -> Option<types::PaymentsAuthorizeData> {
    None
}

// Cards Positive Tests
// Creates a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_only_authorize_payment() {
    let response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized);
}

// Captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(payment_method_details(), None, get_default_payment_info())
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Partially captures a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_capture_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_capture_payment(
            payment_method_details(),
            Some(types::PaymentsCaptureData {
                amount_to_capture: 50,
                ..utils::PaymentCaptureType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Capture payment response");
    assert_eq!(response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_authorized_payment() {
    let authorize_response = CONNECTOR
        .authorize_payment(payment_method_details(), get_default_payment_info())
        .await
        .expect("Authorize payment response");
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Authorized,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("PSync response");
    assert_eq!(response.status, enums::AttemptStatus::Authorized,);
}

// Voids a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_void_authorized_payment() {
    let response = CONNECTOR
        .authorize_and_void_payment(
            payment_method_details(),
            Some(types::PaymentsCancelData {
                connector_transaction_id: String::from(""),
                cancellation_reason: Some("requested_by_customer".to_string()),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .expect("Void payment response");
    assert_eq!(response.status, enums::AttemptStatus::Voided);
}

// Refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(payment_method_details(), None, None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_manually_captured_payment() {
    let response = CONNECTOR
        .capture_payment_and_refund(
            payment_method_details(),
            None,
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Synchronizes a refund using the manual capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_manually_captured_refund() {
    let refund_response = CONNECTOR
        .capture_payment_and_refund(payment_method_details(), None, None, get_default_payment_info())
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_make_payment() {
    let authorize_response = CONNECTOR.make_payment(payment_method_details(), get_default_payment_info()).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
}

// Synchronizes a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_auto_captured_payment() {
    let authorize_response = CONNECTOR.make_payment(payment_method_details(), get_default_payment_info()).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let response = CONNECTOR
        .psync_retry_till_status_matches(
            enums::AttemptStatus::Charged,
            Some(types::PaymentsSyncData {
                connector_transaction_id: router::types::ResponseId::ConnectorTransactionId(
                    txn_id.unwrap(),
                ),
                capture_method: Some(enums::CaptureMethod::Automatic),
                ..Default::default()
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(response.status, enums::AttemptStatus::Charged,);
}

// Refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_auto_captured_payment() {
    let response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Partially refunds a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_partially_refund_succeeded_payment() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        refund_response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Creates multiple refunds against a payment using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_refund_succeeded_payment_multiple_times() {
    CONNECTOR
        .make_payment_and_multiple_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 50,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await;
}

// Synchronizes a refund using the automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_sync_refund() {
    let refund_response = CONNECTOR
        .make_payment_and_refund(payment_method_details(), None, get_default_payment_info())
        .await
        .unwrap();
    let response = CONNECTOR
        .rsync_retry_till_status_matches(
            enums::RefundStatus::Success,
            refund_response.response.unwrap().connector_refund_id,
            None,
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap().refund_status,
        enums::RefundStatus::Success,
    );
}

// Cards Negative scenerios
// Creates a payment with incorrect CVC.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_cvc() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_cvc: Secret::new("12345".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Your card's security code is invalid.".to_string(),
    );
}

// Creates a payment with incorrect expiry month.
#[actix_web::test]
async fn should_fail_payment_for_invalid_exp_month() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_month: Secret::new("20".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Your card's expiration month is invalid.".to_string(),
    );
}

// Creates a payment with incorrect expiry year.
#[actix_web::test]
async fn should_fail_payment_for_incorrect_expiry_year() {
    let response = CONNECTOR
        .make_payment(
            Some(types::PaymentsAuthorizeData {
                payment_method_data: types::api::PaymentMethodData::Card(api::Card {
                    card_exp_year: Secret::new("2000".to_string()),
                    ..utils::CCardType::default().0
                }),
                ..utils::PaymentAuthorizeType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Your card's expiration year is invalid.".to_string(),
    );
}

// Voids a payment using automatic capture flow (Non 3DS).
#[actix_web::test]
async fn should_fail_void_payment_for_auto_capture() {
    let authorize_response = CONNECTOR.make_payment(payment_method_details(), get_default_payment_info()).await.unwrap();
    assert_eq!(authorize_response.status, enums::AttemptStatus::Charged);
    let txn_id = utils::get_connector_transaction_id(authorize_response.response);
    assert_ne!(txn_id, None, "Empty connector transaction id");
    let void_response = CONNECTOR
        .void_payment(txn_id.unwrap(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        void_response.response.unwrap_err().message,
        "You cannot cancel this PaymentIntent because it has a status of succeeded."
    );
}

// Captures a payment using invalid connector payment id.
#[actix_web::test]
async fn should_fail_capture_for_invalid_payment() {
    let capture_response = CONNECTOR
        .capture_payment("123456789".to_string(), None, get_default_payment_info())
        .await
        .unwrap();
    assert_eq!(
        capture_response.response.unwrap_err().message,
        String::from("No such payment_intent: '123456789'")
    );
}

// Refunds a payment with refund amount higher than payment amount.
#[actix_web::test]
async fn should_fail_for_refund_amount_higher_than_payment_amount() {
    let response = CONNECTOR
        .make_payment_and_refund(
            payment_method_details(),
            Some(types::RefundsData {
                refund_amount: 150,
                ..utils::PaymentRefundType::default().0
            }),
            get_default_payment_info(),
        )
        .await
        .unwrap();
    assert_eq!(
        response.response.unwrap_err().message,
        "Refund amount (₹1.50) is greater than charge amount (₹1.00)",
    );
}

/******************** Payouts test cases ********************/ 
// Creates a BACS payout at connector's end
#[actix_web::test]
async fn should_create_bacs_payout() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Bank(
                api::payouts::BankPayout::Bacs(api::BacsBankTransfer {
                    bank_sort_code: "231470".to_string(),
                    bank_account_number: "28821822".to_string(),
                    bank_name: "Deutsche Bank".to_string(),
                    bank_country_code: enums::CountryAlpha2::NL,
                    bank_city: "Amsterdam".to_string(),
                }),
            ))
    };
    let response = CONNECTOR
        .create_payout(payment_info)
        .await
        .expect("Payout bank creation response");
    assert_eq!(response.status, enums::PayoutStatus::RequiresFulfillment);
}

// Fulfills an existing BACS payout at connector's end
#[actix_web::test]
async fn should_fulfill_bacs_payout() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Bank(
                api::payouts::BankPayout::Bacs(api::BacsBankTransfer {
                    bank_sort_code: "231470".to_string(),
                    bank_account_number: "28821822".to_string(),
                    bank_name: "Deutsche Bank".to_string(),
                    bank_country_code: enums::CountryAlpha2::NL,
                    bank_city: "Amsterdam".to_string(),
                }),
            ))
    };
    let payout_id = core_utils::get_or_generate_uuid("payout_id", &None)
    .map_or("payout_3154763247".to_string(), |p| p);
    let response = CONNECTOR
        .fulfill_payout(payout_id, payment_info)
        .await
        .expect("Payout bank fulfill response");
    assert_eq!(response.status, enums::PayoutStatus::Success);
}

// Creates and fulfills BACS payout at connector's end
#[actix_web::test]
async fn should_create_and_fulfill_bacs_payout() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Bank(
                api::payouts::BankPayout::Bacs(api::BacsBankTransfer {
                    bank_sort_code: "231470".to_string(),
                    bank_account_number: "28821822".to_string(),
                    bank_name: "Deutsche Bank".to_string(),
                    bank_country_code: enums::CountryAlpha2::NL,
                    bank_city: "Amsterdam".to_string(),
                }),
            ))
    };
    let response = CONNECTOR
        .create_and_fulfill_payout(None, payment_info)
        .await
        .expect("Payout bank creation and fulfill response");
    assert_eq!(response.status, enums::PayoutStatus::Success);
}

// Creates a ACH payout at connector's end
#[actix_web::test]
async fn should_create_ach_payout() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Bank(
                api::payouts::BankPayout::Ach(api::AchBankTransfer {
                    bank_sort_code: "231470".to_string(),
                    bank_account_number: "28821822".to_string(),
                    bank_name: "Deutsche Bank".to_string(),
                    bank_country_code: enums::CountryAlpha2::NL,
                    bank_city: "Amsterdam".to_string(),
                }),
            ))
    };
    let response = CONNECTOR
        .create_payout(payment_info)
        .await
        .expect("Payout bank creation response");
    assert_eq!(response.status, enums::PayoutStatus::RequiresFulfillment);
}

// Fulfills an existing ACH payout at connector's end
#[actix_web::test]
async fn should_fulfill_ach_payout() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Bank(
                api::payouts::BankPayout::Ach(api::AchBankTransfer {
                    bank_sort_code: "231470".to_string(),
                    bank_account_number: "28821822".to_string(),
                    bank_name: "Deutsche Bank".to_string(),
                    bank_country_code: enums::CountryAlpha2::NL,
                    bank_city: "Amsterdam".to_string(),
                }),
            ))
    };
    let payout_id = core_utils::get_or_generate_uuid("payout_id", &None)
    .map_or("payout_3154763247".to_string(), |p| p);
    let response = CONNECTOR
        .fulfill_payout(payout_id, payment_info)
        .await
        .expect("Payout bank fulfill response");
    assert_eq!(response.status, enums::PayoutStatus::Success);
}

// Creates and fulfills ACH payout at connector's end
#[actix_web::test]
async fn should_create_and_fulfill_ach_payout() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Bank(
                api::payouts::BankPayout::Ach(api::AchBankTransfer {
                    bank_sort_code: "231470".to_string(),
                    bank_account_number: "28821822".to_string(),
                    bank_name: "Deutsche Bank".to_string(),
                    bank_country_code: enums::CountryAlpha2::NL,
                    bank_city: "Amsterdam".to_string(),
                }),
            ))
    };
    let response = CONNECTOR
        .create_and_fulfill_payout(None, payment_info)
        .await
        .expect("Payout bank creation and fulfill response");
    assert_eq!(response.status, enums::PayoutStatus::Success);
}

// Creates a recipient at connector's end
#[actix_web::test]
async fn should_create_payout_recipient() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Bank(
                api::payouts::BankPayout::Ach(api::AchBankTransfer {
                    bank_sort_code: "231470".to_string(),
                    bank_account_number: "28821822".to_string(),
                    bank_name: "Deutsche Bank".to_string(),
                    bank_country_code: enums::CountryAlpha2::NL,
                    bank_city: "Amsterdam".to_string(),
                }),
            ))
    };
    let response = CONNECTOR
        .create_payout_recipient(payment_info)
        .await
        .expect("Payout recipient creation response");
    assert_eq!(response.status, enums::PayoutStatus::RequiresCreation);
}

// Checks eligibility of given card details at connector's end
#[actix_web::test]
async fn should_verify_payout_eligibility() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Card(api::payouts::CardPayout {
            card_number: CardNumber::from_str("4111111111111111").unwrap(),
            expiry_month: Secret::new("3".to_string()),
            expiry_year: Secret::new("2030".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
        }))
    };
    let response = CONNECTOR
        .verify_payout_eligibility(payment_info)
        .await
        .expect("Payout eligibility response");
    assert_eq!(response.status.unwrap(), enums::PayoutStatus::RequiresCreation);
}

// Fulfills card payout at connector's end
#[actix_web::test]
async fn should_fulfill_card_payout() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Card(api::payouts::CardPayout {
            card_number: CardNumber::from_str("4111111111111111").unwrap(),
            expiry_month: Secret::new("3".to_string()),
            expiry_year: Secret::new("2030".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
        }))
    };
    let response = CONNECTOR
        .fulfill_payout(payment_info)
        .await
        .expect("Payout fulfill response");
    assert_eq!(response.status.unwrap(), enums::PayoutStatus::RequiresCreation);
}

// Attempts cancellation of a created payout at connector's end
#[actix_web::test]
async fn should_create_and_cancel_created_payout() {
    let payment_info = PaymentInfo {
        payout_method_data: Some(api::PayoutMethodData::Bank(
                api::payouts::BankPayout::Ach(api::AchBankTransfer {
                    bank_sort_code: "231470".to_string(),
                    bank_account_number: "28821822".to_string(),
                    bank_name: "Deutsche Bank".to_string(),
                    bank_country_code: enums::CountryAlpha2::NL,
                    bank_city: "Amsterdam".to_string(),
                }),
            ))
    };
    let response = CONNECTOR
        .create_and_cancel_payout(None, payment_info)
        .await
        .expect("Payout cancel response");
    assert_eq!(response.status.unwrap(), enums::PayoutStatus::Success);
}

// Connector dependent test cases goes here

// [#478]: add unit tests for non 3DS, wallets & webhooks in connector tests
