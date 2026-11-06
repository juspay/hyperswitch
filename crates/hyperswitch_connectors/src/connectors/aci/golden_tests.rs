//! Golden wire tests: every request the connector sends, end to end through `build_request`.
//!
//! Each scenario renders the method, URL, header names and the decoded form body of one outbound
//! request, and the whole set is compared against `golden_requests.snap`. The snapshot is the
//! contract a refactor of this connector must keep: if a change alters it, the change alters what
//! ACI receives.
//!
//! Regenerate after an intended wire change with
//! `ACI_GOLDEN_BLESS=1 cargo test -p hyperswitch_connectors --features v1 aci::golden_tests`
//! and review the snapshot diff like any other code change.

use std::fmt::Write as _;

use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    id_type,
    pii::Email,
    request::Request,
    types::{MinorUnit, SemanticVersion},
};
use hyperswitch_domain_models::{
    address::{Address, AddressDetails, PhoneDetails},
    mandates::{self, MandateReferenceId},
    payment_address::PaymentAddress,
    payment_method_data::{
        ApplePayWalletData, BankRedirectData, Card, CardWithOptionalCVC, GooglePayWalletData,
        NetworkTokenData, PayLaterData, PaymentMethodData, SamsungPayWalletData, WalletData,
    },
    router_data::{ConnectorAuthType, PaymentMethodToken, RouterData},
    router_flow_types::{
        authentication::{PostAuthentication, PreAuthentication},
        payments::{Authorize, Capture, PSync, SetupMandate, Void},
        refunds::Execute,
    },
    router_request_types::{
        authentication::{ConnectorPostAuthenticationRequestData, PreAuthNRequestData},
        AuthenticationData, BrowserInformation, PaymentsAuthorizeData, PaymentsCancelData,
        PaymentsCaptureData, PaymentsSyncData, RefundsData, ResponseId, SetupMandateRequestData,
    },
    router_response_types::{
        AuthenticationResponseData, PaymentsResponseData, RefundsResponseData,
    },
};
use hyperswitch_interfaces::{
    api::{ConnectorIntegration, ConnectorSpecifications},
    configs::Connectors,
    errors,
    types::Response,
    webhooks::{IncomingWebhook, IncomingWebhookRequestDetails},
};
use hyperswitch_masking::{PeekInterface, Secret};

use super::{
    cof_record::{AciCofRecord, AciCofStash, CofRole},
    test_fixtures::{cof_authorize_data, cof_router_data, COF_CITI, COF_RG, COF_TLID},
    Aci,
};

const SNAPSHOT_PATH: &str = "src/connectors/aci/golden_requests.snap";
const BASE_URL: &str = "https://eu-test.oppwa.com/";

// ---- rendering ----------------------------------------------------------------------------------

fn connectors() -> Connectors {
    let mut connectors = Connectors::default();
    connectors.aci.base_url = BASE_URL.to_string();
    connectors
}

/// `METHOD url`, sorted header names, then one decoded `key=value` per line.
fn render(request: &Request) -> String {
    let mut out = format!("{:?} {}\n", request.method, request.url);
    let mut headers: Vec<_> = request
        .headers
        .iter()
        .map(|(name, _)| name.clone())
        .collect();
    headers.sort();
    let _ = writeln!(out, "headers: {}", headers.join(", "));
    if let Some(body) = &request.body {
        let encoded = body.get_inner_value();
        let pairs: Vec<(String, String)> =
            serde_urlencoded::from_str(encoded.peek()).expect("body is form-urlencoded");
        for (key, value) in pairs {
            let _ = writeln!(out, "{key}={value}");
        }
    }
    out
}

fn outcome(result: CustomResult<Option<Request>, errors::ConnectorError>) -> String {
    match result {
        Ok(Some(request)) => render(&request),
        Ok(None) => "no request\n".to_string(),
        Err(error) => format!("error: {}\n", error.current_context()),
    }
}

fn build<F, Req, Resp>(router_data: &RouterData<F, Req, Resp>) -> String
where
    Aci: ConnectorIntegration<F, Req, Resp>,
{
    outcome(ConnectorIntegration::<F, Req, Resp>::build_request(
        Aci::new(),
        router_data,
        &connectors(),
    ))
}

// ---- fixtures -----------------------------------------------------------------------------------

fn auth() -> ConnectorAuthType {
    ConnectorAuthType::BodyKey {
        api_key: Secret::new("test_api_key".to_string()),
        key1: Secret::new("8ac7a4c9entity".to_string()),
    }
}

fn billing() -> PaymentAddress {
    let address = Address {
        address: Some(AddressDetails {
            city: Some("Cape Town".to_string()),
            country: Some(enums::CountryAlpha2::ZA),
            line1: Some(Secret::new("1 Long Street".to_string())),
            line2: None,
            line3: None,
            zip: Some(Secret::new("8001".to_string())),
            state: Some(Secret::new("Western Cape".to_string())),
            first_name: Some(Secret::new("Jane".to_string())),
            last_name: Some(Secret::new("Shopper".to_string())),
            origin_zip: None,
        }),
        phone: Some(PhoneDetails {
            number: Some(Secret::new("821234567".to_string())),
            country_code: Some("+27".to_string()),
        }),
        email: Some(Email::try_from("jane@example.com".to_string()).expect("valid email")),
    };
    PaymentAddress::new(None, Some(address), None, None)
}

fn browser_info() -> BrowserInformation {
    BrowserInformation {
        color_depth: Some(24),
        java_enabled: None,
        java_script_enabled: None,
        language: Some("en-ZA".to_string()),
        screen_height: Some(1080),
        screen_width: Some(1920),
        time_zone: Some(-120),
        ip_address: Some("196.25.1.1".parse().expect("valid ip")),
        accept_header: Some("text/html".to_string()),
        user_agent: Some("Mozilla/5.0".to_string()),
        ..Default::default()
    }
}

fn card() -> Card {
    Card {
        card_number: "4111111111111111".parse().expect("valid PAN"),
        card_exp_month: Secret::new("03".to_string()),
        card_exp_year: Secret::new("30".to_string()),
        card_cvc: Secret::new("737".to_string()),
        card_network: Some(common_enums::CardNetwork::Visa),
        card_holder_name: Some(Secret::new("Card Holder".to_string())),
        ..Default::default()
    }
}

fn card_with_optional_cvc(card_cvc: Option<&str>) -> CardWithOptionalCVC {
    CardWithOptionalCVC {
        card_number: "5454545454545454".parse().expect("valid PAN"),
        card_exp_month: Secret::new("11".to_string()),
        card_exp_year: Secret::new("2031".to_string()),
        card_cvc: card_cvc.map(|cvc| Secret::new(cvc.to_string())),
        card_network: Some(common_enums::CardNetwork::Mastercard),
        ..Default::default()
    }
}

fn network_token(
    eci: Option<&str>,
    network: Option<common_enums::CardNetwork>,
) -> NetworkTokenData {
    NetworkTokenData {
        token_number: "4895370012003478".parse().expect("valid token"),
        token_exp_month: Secret::new("12".to_string()),
        token_exp_year: Secret::new("29".to_string()),
        token_cryptogram: Some(Secret::new("AgAAAAAABk4DWZ4C28yUQAAAAAA=".to_string())),
        card_network: network,
        eci: eci.map(String::from),
        ..Default::default()
    }
}

fn apple_pay(payment_data: serde_json::Value) -> WalletData {
    let wallet: ApplePayWalletData = serde_json::from_value(serde_json::json!({
        "payment_data": payment_data,
        "payment_method": {"display_name": "Visa 0224", "network": "Visa", "pm_type": "debit"},
        "transaction_identifier": "apple-txn-1",
    }))
    .expect("valid apple pay wallet data");
    WalletData::ApplePay(wallet)
}

fn apple_pay_decrypted() -> serde_json::Value {
    serde_json::json!({
        "application_primary_account_number": "4895370012003478",
        "application_expiration_month": "12",
        "application_expiration_year": "29",
        "payment_data": {"online_payment_cryptogram": "AppleCryptogram==", "eci_indicator": "07"},
    })
}

fn google_pay(tokenization_data: serde_json::Value) -> WalletData {
    let wallet: GooglePayWalletData = serde_json::from_value(serde_json::json!({
        "pm_type": "CARD",
        "description": "Mastercard •••• 4444",
        "info": {"card_network": "MASTERCARD", "card_details": "4444"},
        "tokenization_data": tokenization_data,
    }))
    .expect("valid google pay wallet data");
    WalletData::GooglePay(wallet)
}

fn google_pay_decrypted() -> serde_json::Value {
    serde_json::json!({
        "card_exp_month": "10",
        "card_exp_year": "2030",
        "application_primary_account_number": "5204740000001002",
        "cryptogram": "GoogleCryptogram==",
        "eci_indicator": "02",
    })
}

fn samsung_pay() -> WalletData {
    let wallet: SamsungPayWalletData = serde_json::from_value(serde_json::json!({
        "payment_credential": {
            "method": "3DS",
            "recurring_payment": false,
            "card_brand": "visa",
            "card_last4digits": "1234",
            "3_d_s": {"type": "S", "version": "100", "data": "samsung-token"},
        },
    }))
    .expect("valid samsung pay wallet data");
    WalletData::SamsungPay(Box::new(wallet))
}

fn external_authentication(eci: &str) -> AuthenticationData {
    AuthenticationData {
        eci: Some(eci.to_string()),
        cavv: Secret::new("AAABBEg0VhI0VniQEjRWAAAAAAA=".to_string()),
        threeds_server_transaction_id: Some("3ds-server-1".to_string()),
        message_version: Some(SemanticVersion::new(2, 2, 0)),
        ds_trans_id: Some("ds-trans-1".to_string()),
        created_at: common_utils::date_time::now(),
        challenge_code: None,
        challenge_cancel: None,
        challenge_code_reason: None,
        message_extension: None,
        acs_trans_id: Some("acs-trans-1".to_string()),
        authentication_type: Some(enums::DecoupledAuthenticationType::Challenge),
        transaction_status: Some(enums::TransactionStatus::Success),
        cb_network_params: None,
        exemption_indicator: None,
    }
}

fn mandate_data_with_end_date() -> mandates::MandateData {
    mandates::MandateData {
        update_mandate_id: None,
        customer_acceptance: Some(common_types::payments::CustomerAcceptance::default()),
        mandate_type: Some(mandates::MandateDataType::MultiUse(Some(
            mandates::MandateAmountData {
                amount: MinorUnit::new(5000),
                currency: enums::Currency::ZAR,
                start_date: None,
                end_date: Some(time::macros::datetime!(2028-06-30 0:00)),
                metadata: None,
            },
        ))),
    }
}

fn connector_mandate(
    rg: &str,
    metadata: Option<serde_json::Value>,
    citi: Option<&str>,
) -> mandates::MandateIds {
    mandates::MandateIds {
        mandate_id: Some("man_golden".to_string()),
        mandate_reference_id: Some(MandateReferenceId::ConnectorMandateId(
            mandates::ConnectorMandateReferenceId::new(
                Some(rg.to_string()),
                None,
                None,
                metadata.map(Secret::new),
                citi.map(String::from),
                None,
            ),
        )),
    }
}

fn network_mandate(network_transaction_id: &str) -> mandates::MandateIds {
    mandates::MandateIds {
        mandate_id: Some("man_golden_nti".to_string()),
        mandate_reference_id: Some(MandateReferenceId::NetworkMandateId(
            mandates::NetworkMandateIdRef {
                network_transaction_id: network_transaction_id.to_string(),
                transaction_link_id: None,
            },
        )),
    }
}

fn stamped_record() -> serde_json::Value {
    AciCofRecord::new(
        COF_RG.to_string(),
        Some(COF_CITI.to_string()),
        Some(COF_TLID.to_string()),
        Some("RECURRING".to_string()),
    )
    .to_metadata()
    .peek()
    .clone()
}

type AuthorizeRouterData = RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>;

/// A plain one-off e-commerce card payment with a full billing address and browser block, which
/// every scenario adjusts.
fn authorize(payment_method_data: PaymentMethodData) -> AuthorizeRouterData {
    let mut router_data: AuthorizeRouterData = cof_router_data(PaymentsAuthorizeData {
        payment_method_data,
        router_return_url: Some("https://merchant.example/return".to_string()),
        browser_info: Some(browser_info()),
        customer_id: id_type::CustomerId::wrap("cus_golden".to_string()).ok(),
        metadata: Some(serde_json::json!({"order_ref": "ord-123", "skip": null, "n": 7})),
        ..cof_authorize_data()
    });
    router_data.connector_auth_type = auth();
    router_data.address = billing();
    router_data.customer_id = id_type::CustomerId::wrap("cus_golden".to_string()).ok();
    router_data
}

fn with<T>(mut value: T, tweak: impl FnOnce(&mut T)) -> T {
    tweak(&mut value);
    value
}

fn meta(value: serde_json::Value) -> Option<Secret<serde_json::Value>> {
    Some(Secret::new(value))
}

fn three_ds(router_data: &mut AuthorizeRouterData, enrolled: bool) {
    router_data.auth_type = enums::AuthenticationType::ThreeDs;
    router_data.request.enrolled_for_3ds = enrolled;
}

fn initial_cit(router_data: &mut AuthorizeRouterData) {
    router_data.request.setup_future_usage = Some(enums::FutureUsage::OffSession);
    router_data.request.customer_acceptance =
        Some(common_types::payments::CustomerAcceptance::default());
    router_data.request.setup_mandate_details = Some(mandate_data_with_end_date());
}

fn mit(router_data: &mut AuthorizeRouterData, mandate: mandates::MandateIds) {
    router_data.request.mandate_id = Some(mandate);
    router_data.request.off_session = Some(true);
}

fn authorize_scenarios() -> Vec<(&'static str, AuthorizeRouterData)> {
    let card = || PaymentMethodData::Card(card());
    let nedbank = || meta(serde_json::json!({"acquirer": "Nedbank"}));
    vec![
        ("card/one_off/auto_capture", authorize(card())),
        (
            "card/one_off/manual_capture",
            with(authorize(card()), |rd| {
                rd.request.capture_method = Some(enums::CaptureMethod::Manual)
            }),
        ),
        (
            "card/one_off/billing_name_absent_uses_card_holder",
            with(authorize(card()), |rd| {
                rd.address = PaymentAddress::default();
                rd.request.browser_info = None;
            }),
        ),
        (
            "card/three_ds/enrolled",
            with(authorize(card()), |rd| three_ds(rd, true)),
        ),
        (
            "card/three_ds/not_enrolled",
            with(authorize(card()), |rd| three_ds(rd, false)),
        ),
        (
            "card/three_ds/enrolled_without_browser_info",
            with(authorize(card()), |rd| {
                three_ds(rd, true);
                rd.request.browser_info = None;
            }),
        ),
        (
            "card/three_ds/external_authentication",
            with(authorize(card()), |rd| {
                rd.request.authentication_data = Some(external_authentication("05"))
            }),
        ),
        (
            "card/moto/mca_category",
            with(authorize(card()), |rd| {
                three_ds(rd, true);
                rd.request.browser_info = None;
                rd.connector_meta_data = meta(serde_json::json!({"transactionCategory": " moto "}));
            }),
        ),
        (
            "card/moto/payment_channel_telephone",
            with(authorize(card()), |rd| {
                rd.request.payment_channel = Some(enums::PaymentChannel::TelephoneOrder)
            }),
        ),
        (
            "card/moto/initial_cit",
            with(authorize(card()), |rd| {
                initial_cit(rd);
                rd.connector_meta_data = meta(serde_json::json!({"transactionCategory": "MO"}));
            }),
        ),
        (
            "card/channel/mca_wins_over_payment_channel",
            with(authorize(card()), |rd| {
                rd.request.payment_channel = Some(enums::PaymentChannel::MailOrder);
                rd.connector_meta_data = meta(serde_json::json!({"transactionCategory": "EC"}));
            }),
        ),
        (
            "card/channel/unknown_mca_category_ignored",
            with(authorize(card()), |rd| {
                rd.connector_meta_data = meta(serde_json::json!({"transactionCategory": "XX"}))
            }),
        ),
        (
            "card/recurring/initial_cit",
            with(authorize(card()), |rd| {
                initial_cit(rd);
                rd.request.mit_category = Some(common_enums::MitCategory::Recurring);
            }),
        ),
        (
            "card/recurring/initial_cit_three_ds",
            with(authorize(card()), |rd| {
                initial_cit(rd);
                three_ds(rd, true);
            }),
        ),
        (
            "card/recurring/off_session_without_acceptance",
            with(authorize(card()), |rd| {
                rd.request.setup_future_usage = Some(enums::FutureUsage::OffSession);
                rd.request.mit_category = Some(common_enums::MitCategory::Installment);
            }),
        ),
        (
            "card/recurring/mit_raw_card",
            with(authorize(card()), |rd| {
                mit(rd, connector_mandate(COF_RG, Some(stamped_record()), None));
                three_ds(rd, true);
                rd.request.authentication_data = Some(external_authentication("05"));
            }),
        ),
        (
            "card/recurring/one_click_repeat",
            with(authorize(card()), |rd| {
                mit(rd, connector_mandate(COF_RG, Some(stamped_record()), None));
                rd.request.off_session = Some(false);
                three_ds(rd, true);
            }),
        ),
        (
            "card/recurring/mit_network_mandate",
            with(authorize(card()), |rd| {
                mit(rd, network_mandate("481234567891234"));
                rd.request.mit_category = Some(common_enums::MitCategory::Resubmission);
            }),
        ),
        (
            "card/recurring/moto_mit",
            with(authorize(card()), |rd| {
                mit(rd, connector_mandate(COF_RG, Some(stamped_record()), None));
                rd.connector_meta_data = meta(serde_json::json!({"transactionCategory": "TO"}));
            }),
        ),
        (
            "card/metadata/descriptor_test_mode_trim",
            with(authorize(card()), |rd| {
                rd.request.billing_descriptor = serde_json::from_value(serde_json::json!({
                    "statement_descriptor": "  PEACH SHOP ",
                    "statement_descriptor_suffix": "ORDER 9",
                }))
                .expect("valid billing descriptor");
                rd.request.merchant_order_reference_id =
                    Some("merchant-order-reference-0001".to_string());
                rd.connector_meta_data = meta(serde_json::json!({
                    "test_mode": "external",
                    "trim_merchant_transaction_id": "true",
                }));
            }),
        ),
        (
            "card/metadata/short_order_reference_and_internal_test_mode",
            with(authorize(card()), |rd| {
                rd.request.merchant_order_reference_id = Some("ord1".to_string());
                rd.connector_request_reference_id = "abc".to_string();
                rd.connector_meta_data = meta(serde_json::json!({
                    "test_mode": "INTERNAL",
                    "trim_merchant_transaction_id": true,
                }));
            }),
        ),
        (
            "card/metadata/reserved_and_invalid_custom_parameters",
            with(authorize(card()), |rd| {
                rd.request.metadata = Some(serde_json::json!({
                    "paymentId": "spoof",
                    "x": "too-short-name",
                    "bad key": "spaces",
                    "nested": {"a": 1},
                    "ok_key": "",
                }));
            }),
        ),
        (
            "card_with_optional_cvc/cvc_present",
            authorize(PaymentMethodData::CardWithOptionalCVC(
                card_with_optional_cvc(Some("123")),
            )),
        ),
        (
            "card_with_optional_cvc/cvc_absent_not_permitted",
            authorize(PaymentMethodData::CardWithOptionalCVC(
                card_with_optional_cvc(None),
            )),
        ),
        (
            "card_with_optional_cvc/cvc_absent_permitted",
            with(
                authorize(PaymentMethodData::CardWithOptionalCVC(
                    card_with_optional_cvc(None),
                )),
                |rd| rd.connector_meta_data = meta(serde_json::json!({"collect_cvv": false})),
            ),
        ),
        (
            "network_token/one_off_external_three_ds_eci_dedupe",
            with(
                authorize(PaymentMethodData::NetworkToken(network_token(
                    Some("05"),
                    Some(common_enums::CardNetwork::Visa),
                ))),
                |rd| rd.request.authentication_data = Some(external_authentication("07")),
            ),
        ),
        (
            "network_token/unknown_network_defaults_to_visa",
            authorize(PaymentMethodData::NetworkToken(network_token(None, None))),
        ),
        (
            "network_token/initial_cit",
            with(
                authorize(PaymentMethodData::NetworkToken(network_token(
                    None,
                    Some(common_enums::CardNetwork::Mastercard),
                ))),
                initial_cit,
            ),
        ),
        (
            "network_token/mit_acquirer_packing",
            with(
                authorize(PaymentMethodData::NetworkToken(network_token(
                    None,
                    Some(common_enums::CardNetwork::Mastercard),
                ))),
                |rd| {
                    mit(rd, connector_mandate(COF_RG, Some(stamped_record()), None));
                    rd.connector_meta_data =
                        meta(serde_json::json!({"acquirer": "CyberSource EMC"}));
                },
            ),
        ),
        (
            "apple_pay/decrypted_token",
            with(
                authorize(PaymentMethodData::Wallet(apple_pay(serde_json::json!(
                    "encrypted-apple"
                )))),
                |rd| {
                    rd.payment_method_token = Some(PaymentMethodToken::ApplePayDecrypt(Box::new(
                        serde_json::from_value(apple_pay_decrypted())
                            .expect("valid apple pay data"),
                    )));
                    rd.request.authentication_data = Some(external_authentication("05"));
                },
            ),
        ),
        (
            "apple_pay/decrypted_payload_initial_cit",
            with(
                authorize(PaymentMethodData::Wallet(apple_pay(apple_pay_decrypted()))),
                initial_cit,
            ),
        ),
        (
            "apple_pay/encrypted_passthrough",
            authorize(PaymentMethodData::Wallet(apple_pay(serde_json::json!(
                "encrypted-apple"
            )))),
        ),
        (
            "apple_pay/decrypted_mit",
            with(
                authorize(PaymentMethodData::Wallet(apple_pay(apple_pay_decrypted()))),
                |rd| {
                    mit(rd, connector_mandate(COF_RG, None, Some(COF_CITI)));
                    rd.connector_meta_data = nedbank();
                },
            ),
        ),
        (
            "google_pay/decrypted_payload",
            with(
                authorize(PaymentMethodData::Wallet(
                    google_pay(google_pay_decrypted()),
                )),
                |rd| rd.request.authentication_data = Some(external_authentication("05")),
            ),
        ),
        (
            "google_pay/encrypted_passthrough",
            authorize(PaymentMethodData::Wallet(google_pay(serde_json::json!({
                "type": "PAYMENT_GATEWAY",
                "token": "{\"signature\":\"sig\"}",
            })))),
        ),
        (
            "samsung_pay/one_off_external_three_ds",
            with(authorize(PaymentMethodData::Wallet(samsung_pay())), |rd| {
                rd.request.authentication_data = Some(external_authentication("05"))
            }),
        ),
        (
            "samsung_pay/mit",
            with(authorize(PaymentMethodData::Wallet(samsung_pay())), |rd| {
                mit(rd, connector_mandate(COF_RG, Some(stamped_record()), None));
                rd.request.authentication_data = Some(external_authentication("05"));
            }),
        ),
        (
            "wallet_redirect/mb_way",
            authorize(PaymentMethodData::Wallet(WalletData::MbWayRedirect(
                Box::new(hyperswitch_domain_models::payment_method_data::MbWayRedirection {}),
            ))),
        ),
        (
            "wallet_redirect/ali_pay_initial_cit_ignored",
            with(
                authorize(PaymentMethodData::Wallet(WalletData::AliPayRedirect(
                    hyperswitch_domain_models::payment_method_data::AliPayRedirection {},
                ))),
                |rd| {
                    initial_cit(rd);
                    rd.request.authentication_data = Some(external_authentication("05"));
                },
            ),
        ),
        (
            "pay_later/klarna",
            authorize(PaymentMethodData::PayLater(PayLaterData::KlarnaRedirect {})),
        ),
        (
            "pay_later/unsupported",
            authorize(PaymentMethodData::PayLater(PayLaterData::AffirmRedirect {})),
        ),
        (
            "bank_redirect/eps",
            authorize(PaymentMethodData::BankRedirect(BankRedirectData::Eps {
                bank_name: None,
                country: None,
            })),
        ),
        (
            "bank_redirect/ideal",
            authorize(PaymentMethodData::BankRedirect(BankRedirectData::Ideal {
                bank_name: Some(common_enums::BankNames::Ing),
            })),
        ),
        (
            "bank_redirect/ideal_without_bank",
            authorize(PaymentMethodData::BankRedirect(BankRedirectData::Ideal {
                bank_name: None,
            })),
        ),
        (
            "bank_redirect/giropay",
            authorize(PaymentMethodData::BankRedirect(BankRedirectData::Giropay {
                bank_account_bic: Some(Secret::new("GENODETT488".to_string())),
                bank_account_iban: Some(Secret::new("DE36444488881234567890".to_string())),
                country: None,
            })),
        ),
        (
            "bank_redirect/trustly",
            with(
                authorize(PaymentMethodData::BankRedirect(BankRedirectData::Trustly {
                    country: None,
                    account_holder_name: None,
                    bank_name: None,
                    additional_details: None,
                    bank_last_digits: None,
                    connector_instrument_id: None,
                })),
                initial_cit,
            ),
        ),
        (
            "bank_redirect/przelewy24",
            authorize(PaymentMethodData::BankRedirect(
                BankRedirectData::Przelewy24 { bank_name: None },
            )),
        ),
        (
            "bank_redirect/interac",
            authorize(PaymentMethodData::BankRedirect(BankRedirectData::Interac {
                country: None,
                email: None,
            })),
        ),
        (
            "bank_redirect/eft",
            authorize(PaymentMethodData::BankRedirect(BankRedirectData::Eft {
                provider: "ozow".to_string(),
            })),
        ),
        (
            "bank_redirect/sofort",
            authorize(PaymentMethodData::BankRedirect(BankRedirectData::Sofort {
                country: None,
                preferred_language: None,
            })),
        ),
        (
            "mandate/mit_stamped_record",
            with(authorize(PaymentMethodData::MandatePayment), |rd| {
                mit(rd, connector_mandate(COF_RG, Some(stamped_record()), None));
                rd.request.authentication_data = Some(external_authentication("05"));
                rd.connector_meta_data = nedbank();
            }),
        ),
        (
            "mandate/mit_legacy_record_payment_method_fallback",
            with(authorize(PaymentMethodData::MandatePayment), |rd| {
                mit(
                    rd,
                    connector_mandate(COF_RG, None, Some("pending-placeholder")),
                );
                rd.recurring_mandate_payment_data = Some(
                    hyperswitch_domain_models::router_data::RecurringMandatePaymentData {
                        payment_method_type: None,
                        original_payment_authorized_amount: None,
                        original_payment_authorized_currency: None,
                        mandate_metadata: None,
                        network_transaction_id: Some("MCC1234560910".to_string()),
                        network_transaction_link_id: Some("TLID-PM".to_string()),
                    },
                );
            }),
        ),
        (
            "mandate/missing_mandate_id",
            authorize(PaymentMethodData::MandatePayment),
        ),
        (
            "unsupported/crypto",
            authorize(PaymentMethodData::Crypto(
                hyperswitch_domain_models::payment_method_data::CryptoData {
                    pay_currency: None,
                    network: None,
                },
            )),
        ),
    ]
}

type SetupRouterData = RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>;

fn setup_mandate(tweak: impl FnOnce(&mut SetupRouterData)) -> SetupRouterData {
    let authorize = cof_authorize_data();
    let mut router_data: SetupRouterData = cof_router_data(SetupMandateRequestData {
        enable_avs_check: None,
        currency: enums::Currency::ZAR,
        payment_method_data: PaymentMethodData::Card(card()),
        amount: 0,
        confirm: true,
        customer_acceptance: Some(common_types::payments::CustomerAcceptance::default()),
        mandate_id: None,
        setup_future_usage: Some(enums::FutureUsage::OffSession),
        off_session: None,
        setup_mandate_details: Some(mandate_data_with_end_date()),
        router_return_url: Some("https://merchant.example/return".to_string()),
        webhook_url: None,
        feature_metadata: None,
        browser_info: Some(browser_info()),
        email: None,
        customer_name: None,
        return_url: None,
        payment_method_type: None,
        request_incremental_authorization: false,
        metadata: Some(Secret::new(serde_json::json!({"order_ref": "setup-1"}))),
        complete_authorize_url: None,
        capture_method: None,
        enrolled_for_3ds: false,
        related_transaction_id: None,
        minor_amount: MinorUnit::zero(),
        shipping_cost: None,
        connector_testing_data: None,
        customer_id: authorize.customer_id,
        enable_partial_authorization: None,
        payment_channel: None,
        is_stored_credential: None,
        billing_descriptor: None,
        split_payments: None,
        tokenization: None,
        partner_merchant_identifier_details: None,
        authentication_data: None,
        connector_intent_metadata: None,
        merchant_order_reference_id: Some("setup-order-0001".to_string()),
        mit_category: Some(common_enums::MitCategory::Recurring),
        is_account_funded_transaction: None,
        recipient_details: None,
        business_country: None,
    });
    router_data.connector_auth_type = auth();
    router_data.address = billing();
    tweak(&mut router_data);
    router_data
}

fn setup_mandate_scenarios() -> Vec<(&'static str, SetupRouterData)> {
    vec![
        ("setup_mandate/card", setup_mandate(|_| {})),
        (
            "setup_mandate/card_three_ds_external",
            setup_mandate(|rd| {
                rd.auth_type = enums::AuthenticationType::ThreeDs;
                rd.request.enrolled_for_3ds = true;
                rd.request.authentication_data = Some(external_authentication("05"));
            }),
        ),
        (
            "setup_mandate/card_moto",
            setup_mandate(|rd| {
                rd.connector_meta_data = meta(serde_json::json!({"transactionCategory": "MOTO"}))
            }),
        ),
        (
            "setup_mandate/card_with_optional_cvc_permitted",
            setup_mandate(|rd| {
                rd.request.payment_method_data =
                    PaymentMethodData::CardWithOptionalCVC(card_with_optional_cvc(None));
                rd.connector_meta_data = meta(serde_json::json!({"collect_cvv": "false"}));
            }),
        ),
        (
            "setup_mandate/carrying_a_mandate_id",
            setup_mandate(|rd| {
                rd.auth_type = enums::AuthenticationType::ThreeDs;
                rd.request.mandate_id =
                    Some(connector_mandate(COF_RG, Some(stamped_record()), None));
                rd.request.off_session = Some(true);
                rd.request.authentication_data = Some(external_authentication("05"));
            }),
        ),
        (
            "setup_mandate/wallet_not_supported",
            setup_mandate(|rd| {
                rd.request.payment_method_data = PaymentMethodData::Wallet(samsung_pay())
            }),
        ),
    ]
}

fn capture() -> RouterData<Capture, PaymentsCaptureData, PaymentsResponseData> {
    let mut router_data = cof_router_data(PaymentsCaptureData {
        amount_to_capture: 1050,
        minor_amount_to_capture: MinorUnit::new(1050),
        currency: enums::Currency::ZAR,
        connector_transaction_id: "8ac7a4a1capture".to_string(),
        payment_amount: 2000,
        minor_payment_amount: MinorUnit::new(2000),
        ..Default::default()
    });
    router_data.connector_auth_type = auth();
    router_data
}

fn sync() -> RouterData<PSync, PaymentsSyncData, PaymentsResponseData> {
    let mut router_data = cof_router_data(PaymentsSyncData {
        connector_transaction_id: ResponseId::ConnectorTransactionId("8ac7a4a1sync".to_string()),
        ..Default::default()
    });
    router_data.connector_auth_type = auth();
    router_data
}

fn void() -> RouterData<Void, PaymentsCancelData, PaymentsResponseData> {
    let mut router_data = cof_router_data(PaymentsCancelData {
        connector_transaction_id: "8ac7a4a1void".to_string(),
        ..Default::default()
    });
    router_data.connector_auth_type = auth();
    router_data
}

fn refund() -> RouterData<Execute, RefundsData, RefundsResponseData> {
    let mut router_data: RouterData<Execute, RefundsData, RefundsResponseData> =
        cof_router_data(RefundsData {
            refund_id: "ref_golden".to_string(),
            connector_transaction_id: "8ac7a4a1refund".to_string(),
            connector_refund_id: None,
            currency: enums::Currency::ZAR,
            payment_amount: 2000,
            reason: None,
            webhook_url: None,
            refund_amount: 750,
            connector_metadata: None,
            refund_connector_metadata: None,
            browser_info: None,
            split_refunds: None,
            minor_payment_amount: MinorUnit::new(2000),
            minor_refund_amount: MinorUnit::new(750),
            integrity_object: None,
            refund_status: enums::RefundStatus::Pending,
            merchant_account_id: None,
            merchant_config_currency: None,
            capture_method: None,
            additional_payment_method_data: None,
            payment_connector_request_reference_id: None,
            payment_method_data: None,
            mandate_id: None,
            feature_metadata: None,
            payment_feature_metadata: None,
        });
    router_data.connector_auth_type = auth();
    router_data
}

fn pre_authentication(
) -> RouterData<PreAuthentication, PreAuthNRequestData, AuthenticationResponseData> {
    let mut router_data = cof_router_data(PreAuthNRequestData { card: card() });
    router_data.connector_auth_type = auth();
    router_data
}

fn post_authentication() -> RouterData<
    PostAuthentication,
    ConnectorPostAuthenticationRequestData,
    AuthenticationResponseData,
> {
    let mut router_data = cof_router_data(ConnectorPostAuthenticationRequestData {
        threeds_server_transaction_id: "8ac7a4a1threeds".to_string(),
    });
    router_data.connector_auth_type = auth();
    router_data
}

// ---- webhooks and connector specifications ---------------------------------------------------

fn webhook_payload(event_type: &str, payment_type: &str, code: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "type": event_type,
        "action": "CREATED",
        "payload": {
            "id": "8ac7a4a1webhook",
            "paymentType": payment_type,
            "paymentBrand": "VISA",
            "amount": "10.00",
            "currency": "ZAR",
            "result": {"code": code, "description": "described"},
            "timestamp": "2026-10-12 10:00:00+0000",
            "ndc": "ndc",
        },
    }))
    .expect("webhook payload serializes")
}

/// Reference id and event type for a decrypted notification, the way the router asks for them.
fn webhook(body: &[u8]) -> String {
    let headers = actix_web::http::header::HeaderMap::new();
    let request = IncomingWebhookRequestDetails {
        method: http::Method::POST,
        uri: http::Uri::from_static("https://example.com/webhooks/aci"),
        headers: &headers,
        body,
        query_params: String::new(),
    };
    let reference = Aci::new()
        .get_webhook_object_reference_id(&request)
        .map(|id| format!("{id:?}"))
        .unwrap_or_else(|error| format!("error: {}", error.current_context()));
    let event = Aci::new()
        .get_webhook_event_type(&request, None)
        .map(|event| format!("{event:?}"))
        .unwrap_or_else(|error| format!("error: {}", error.current_context()));
    let resource = Aci::new()
        .get_webhook_resource_object(&request)
        .map(|_| "payment payload".to_string())
        .unwrap_or_else(|error| format!("error: {}", error.current_context()));
    format!("reference: {reference}\nevent: {event}\nresource: {resource}\n")
}

fn webhook_scenarios() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        (
            "payment/success",
            webhook_payload("PAYMENT", "DB", "000.000.000"),
        ),
        (
            "payment/pending",
            webhook_payload("PAYMENT", "PA", "000.200.000"),
        ),
        (
            "payment/failure",
            webhook_payload("PAYMENT", "DB", "800.100.151"),
        ),
        (
            "payment/unknown_code",
            webhook_payload("PAYMENT", "DB", "999.999.999"),
        ),
        (
            "refund/success",
            webhook_payload("PAYMENT", "rf", "000.100.110"),
        ),
        (
            "refund/pending",
            webhook_payload("PAYMENT", "RF", "000.200.000"),
        ),
        (
            "refund/failure",
            webhook_payload("PAYMENT", "RF", "800.100.151"),
        ),
        (
            "registration",
            webhook_payload("REGISTRATION", "DB", "000.000.000"),
        ),
        ("risk", webhook_payload("RISK", "DB", "000.000.000")),
        (
            "test_ping",
            serde_json::to_vec(&serde_json::json!({
                "type": "test",
                "action": "webhook activation",
                "payload": {},
            }))
            .expect("ping serializes"),
        ),
        ("not_json", b"not json".to_vec()),
    ]
}

/// Runs a future that never waits — the webhook trait's async methods do no I/O.
fn ready<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    match future
        .as_mut()
        .poll(&mut std::task::Context::from_waker(std::task::Waker::noop()))
    {
        std::task::Poll::Ready(output) => output,
        std::task::Poll::Pending => panic!("webhook future waited"),
    }
}

const WEBHOOK_KEY: [u8; 32] = [7u8; 32];
const WEBHOOK_IV: [u8; ring::aead::NONCE_LEN] = [9u8; ring::aead::NONCE_LEN];

/// `plaintext` sealed under [`WEBHOOK_KEY`]: the hex ciphertext and the hex tag.
fn seal(plaintext: &[u8]) -> (String, String) {
    use ring::aead;

    let sealing_key = aead::LessSafeKey::new(
        aead::UnboundKey::new(&aead::AES_256_GCM, &WEBHOOK_KEY).expect("valid key"),
    );
    let mut sealed = plaintext.to_vec();
    let tag = sealing_key
        .seal_in_place_separate_tag(
            aead::Nonce::assume_unique_for_key(WEBHOOK_IV),
            aead::Aad::empty(),
            &mut sealed,
        )
        .expect("seals");
    (hex::encode(&sealed), hex::encode(tag.as_ref()))
}

fn webhook_headers(headers: &[(&'static str, String)]) -> actix_web::http::header::HeaderMap {
    let mut map = actix_web::http::header::HeaderMap::new();
    for (name, value) in headers {
        map.insert(
            actix_web::http::header::HeaderName::from_static(name),
            actix_web::http::header::HeaderValue::from_str(value).expect("valid header"),
        );
    }
    map
}

/// A delivery through the router's sequence: decode with the account's webhook secret (`None`:
/// a URL naming only the connector), verify the decoded request, then read it.
fn delivered(headers: &[(&'static str, String)], body: &[u8], secret: Option<[u8; 32]>) -> String {
    let headers = webhook_headers(headers);
    let request = |body| IncomingWebhookRequestDetails {
        method: http::Method::POST,
        uri: http::Uri::from_static("https://example.com/webhooks/merchant/mca_aci"),
        headers: &headers,
        body,
        query_params: String::new(),
    };
    let merchant_id = id_type::MerchantId::default();
    let webhook_details =
        secret.map(|key| Secret::new(serde_json::json!({ "merchant_secret": hex::encode(key) })));
    let decoded = match ready(Aci::new().decode_webhook_body(
        &request(body),
        &merchant_id,
        webhook_details.clone(),
        "aci",
    )) {
        Ok(decoded) => decoded,
        Err(error) => return format!("decode: error: {}\n", error.current_context()),
    };
    let decoded_request = request(&decoded);
    let verified = ready(Aci::new().verify_webhook_source(
        &decoded_request,
        &merchant_id,
        webhook_details,
        common_utils::crypto::Encryptable::new(
            Secret::new(serde_json::Value::Null),
            Secret::new(Vec::new()),
        ),
        "aci",
    ))
    .map(|verified| verified.to_string())
    .unwrap_or_else(|error| format!("error: {}", error.current_context()));
    format!("decode: ok\nverified: {verified}\n{}", webhook(&decoded))
}

fn webhook_delivery_scenarios() -> Vec<(&'static str, String)> {
    let payment = webhook_payload("PAYMENT", "DB", "000.000.000");
    let (ciphertext, tag) = seal(&payment);
    let encrypted = || {
        vec![
            ("x-initialization-vector", hex::encode(WEBHOOK_IV)),
            ("x-authentication-tag", tag.clone()),
        ]
    };
    let (ping_ciphertext, ping_tag) =
        seal(br#"{"type":"test","action":"webhook activation","payload":{}}"#);
    let tampered = match ciphertext.as_bytes().split_first() {
        Some((b'0', rest)) => [b"1", rest].concat(),
        Some((_, rest)) => [b"0", rest].concat(),
        None => Vec::new(),
    };
    vec![
        (
            "encrypted/raw_hex",
            delivered(&encrypted(), ciphertext.as_bytes(), Some(WEBHOOK_KEY)),
        ),
        (
            "encrypted/json_wrapper",
            delivered(
                &encrypted(),
                format!(r#"{{"encryptedBody":"{ciphertext}"}}"#).as_bytes(),
                Some(WEBHOOK_KEY),
            ),
        ),
        (
            "encrypted/activation_ping",
            delivered(
                &[
                    ("x-initialization-vector", hex::encode(WEBHOOK_IV)),
                    ("x-authentication-tag", ping_tag),
                ],
                ping_ciphertext.as_bytes(),
                Some(WEBHOOK_KEY),
            ),
        ),
        (
            "encrypted/wrong_secret",
            delivered(&encrypted(), ciphertext.as_bytes(), Some([8u8; 32])),
        ),
        (
            "encrypted/tampered",
            delivered(&encrypted(), &tampered, Some(WEBHOOK_KEY)),
        ),
        (
            "encrypted/url_without_account",
            delivered(&encrypted(), ciphertext.as_bytes(), None),
        ),
        (
            "encrypted/missing_tag",
            delivered(
                &[("x-initialization-vector", hex::encode(WEBHOOK_IV))],
                ciphertext.as_bytes(),
                Some(WEBHOOK_KEY),
            ),
        ),
        (
            "plaintext/unverified",
            delivered(&[], &payment, Some(WEBHOOK_KEY)),
        ),
    ]
}

fn supported_payment_methods() -> String {
    let methods = Aci::new()
        .get_supported_payment_methods()
        .expect("aci declares its payment methods");
    let mut rows: Vec<String> = methods
        .iter()
        .flat_map(|(method, types)| {
            types.iter().map(move |(method_type, details)| {
                format!("{method:?}/{method_type:?}: {details:?}")
            })
        })
        .collect();
    rows.sort();
    rows.join("\n") + "\n"
}

// ---- responses ----------------------------------------------------------------------------------

fn http(status_code: u16, body: serde_json::Value) -> Response {
    Response {
        headers: None,
        response: serde_json::to_vec(&body).expect("body serializes").into(),
        status_code,
    }
}

/// What a flow makes of a response: the status, response and connector response it records.
fn handled<F, Req, Resp>(router_data: &RouterData<F, Req, Resp>, response: Response) -> String
where
    Aci: ConnectorIntegration<F, Req, Resp>,
    F: Clone,
    Req: Clone,
    Resp: Clone + std::fmt::Debug,
{
    match ConnectorIntegration::<F, Req, Resp>::handle_response(
        Aci::new(),
        router_data,
        None,
        response,
    ) {
        Ok(handled) => format!(
            "status: {:?}\nreference_id: {:?}\nresponse: {:#?}\nconnector_response: {:#?}\n",
            handled.status, handled.reference_id, handled.response, handled.connector_response
        ),
        Err(error) => format!("error: {}\n", error.current_context()),
    }
}

fn result(code: &str) -> serde_json::Value {
    serde_json::json!({"code": code, "description": format!("described {code}")})
}

fn payment_response(code: &str, extra: serde_json::Value) -> serde_json::Value {
    let mut body = serde_json::json!({
        "id": "8ac7a4a1payment",
        "paymentType": "DB",
        "paymentBrand": "VISA",
        "amount": "10.00",
        "currency": "ZAR",
        "result": result(code),
        "buildNumber": "build",
        "timestamp": "2026-10-12 10:00:00+0000",
        "ndc": "ndc",
    });
    if let (Some(body), Some(extra)) = (body.as_object_mut(), extra.as_object()) {
        body.extend(extra.clone());
    }
    body
}

const SUCCESS: &str = "000.100.110";
const PENDING: &str = "000.200.000";
const DECLINED: &str = "800.100.151";

fn nedbank_result_details() -> serde_json::Value {
    serde_json::json!({
        "clearingInstituteName": "NEDBANK",
        "ConnectorTxID2": "000123|612345678901",
        "ConnectorTxID3": "543324|00|0417|075308|MCC00013N0417|000000001816511|",
        "AcquirerResponse": "00",
        "ExtendedDescription": "Approved",
    })
}

fn approved_with_everything() -> serde_json::Value {
    payment_response(
        SUCCESS,
        serde_json::json!({
            "registrationId": COF_RG,
            "result": {"code": SUCCESS, "description": "approved", "cvvResponse": "M"},
            "resultDetails": nedbank_result_details(),
            "threeDSecure": {
                "eci": "05",
                "verificationId": "AAABBEg0VhI0VniQEjRWAAAAAAA=",
                "version": "2.2.0",
                "dsTransactionId": "ds-1",
                "acsTransactionId": "acs-1",
                "authenticationStatus": "Y",
                "flow": "frictionless",
            },
            "standingInstruction": {"agreementId": COF_TLID, "initialTransactionId": COF_CITI},
        }),
    )
}

fn challenge() -> serde_json::Value {
    payment_response(
        PENDING,
        serde_json::json!({
            "registrationId": COF_RG,
            "redirect": {
                "url": "https://acs.example/challenge",
                "parameters": [{"name": "PaReq", "value": "from-redirect"}],
                "preconditions": [{
                    "url": "https://acs.example/method",
                    "method": "POST",
                    "parameters": [{"name": "PaReq", "value": "from-precondition"}],
                }],
            },
        }),
    )
}

fn bank_redirect() -> serde_json::Value {
    payment_response(
        PENDING,
        serde_json::json!({
            "redirect": {
                "url": "https://bank.example/pay",
                "method": "GET",
                "parameters": [{"name": "token", "value": "t1"}],
            },
        }),
    )
}

fn declined() -> serde_json::Value {
    payment_response(
        DECLINED,
        serde_json::json!({
            "resultDetails": {"MerchantAdviceCode": "21", "ExtendedDescription": "Insufficient funds"},
        }),
    )
}

fn response_scenarios() -> Vec<(&'static str, String)> {
    let card = || PaymentMethodData::Card(card());
    let one_off = authorize(card());
    let manual = with(authorize(card()), |rd| {
        rd.request.capture_method = Some(enums::CaptureMethod::Manual)
    });
    let cit = with(authorize(card()), initial_cit);
    let mit_charge = with(authorize(PaymentMethodData::MandatePayment), |rd| {
        mit(rd, connector_mandate(COF_RG, Some(stamped_record()), None))
    });
    let externally_authenticated = with(authorize(card()), |rd| {
        rd.request.authentication_data = Some(external_authentication("05"))
    });
    let cit_sync = with(sync(), |rd| {
        rd.request.connector_meta = Some(
            AciCofStash {
                rg: Some(COF_RG.to_string()),
                si_type: Some("UNSCHEDULED".to_string()),
                ..AciCofStash::role(CofRole::Cit)
            }
            .to_connector_meta(),
        );
    });
    let zero_amount_sync = with(sync(), |rd| {
        rd.request.capture_method = Some(enums::CaptureMethod::Manual);
        rd.request.amount = MinorUnit::zero();
    });
    let mit_sync = with(sync(), |rd| {
        rd.request.connector_meta = Some(AciCofStash::role(CofRole::Mit).to_connector_meta());
    });

    vec![
        (
            "authorize/approved_auto_capture",
            handled(&one_off, http(200, approved_with_everything())),
        ),
        (
            "authorize/approved_manual_capture",
            handled(&manual, http(200, approved_with_everything())),
        ),
        (
            "authorize/pending",
            handled(
                &one_off,
                http(200, payment_response(PENDING, serde_json::json!({}))),
            ),
        ),
        (
            "authorize/declined",
            handled(&one_off, http(200, declined())),
        ),
        (
            "authorize/unknown_result_code",
            handled(
                &one_off,
                http(200, payment_response("123.456.789", serde_json::json!({}))),
            ),
        ),
        (
            "authorize/challenge_redirect",
            handled(&one_off, http(200, challenge())),
        ),
        (
            "authorize/bank_redirect",
            handled(&one_off, http(200, bank_redirect())),
        ),
        (
            "authorize/cit_approved",
            handled(&cit, http(200, approved_with_everything())),
        ),
        (
            "authorize/cit_challenge",
            handled(&cit, http(200, challenge())),
        ),
        (
            "authorize/mit_approved",
            handled(&mit_charge, http(200, approved_with_everything())),
        ),
        (
            "authorize/externally_authenticated_without_echo",
            handled(
                &externally_authenticated,
                http(200, payment_response(SUCCESS, serde_json::json!({}))),
            ),
        ),
        (
            "authorize/not_json",
            handled(
                &one_off,
                Response {
                    headers: None,
                    response: "<html>".into(),
                    status_code: 200,
                },
            ),
        ),
        (
            "sync/cit_finalised",
            handled(&cit_sync, http(200, approved_with_everything())),
        ),
        (
            "sync/cit_still_pending",
            handled(
                &cit_sync,
                http(200, payment_response(PENDING, serde_json::json!({}))),
            ),
        ),
        (
            "sync/mit",
            handled(&mit_sync, http(200, approved_with_everything())),
        ),
        (
            "sync/zero_amount_verification",
            handled(
                &zero_amount_sync,
                http(200, payment_response(SUCCESS, serde_json::json!({}))),
            ),
        ),
        (
            "sync/no_stash_with_registration",
            handled(&sync(), http(200, approved_with_everything())),
        ),
        (
            "setup_mandate/approved",
            handled(
                &setup_mandate(|_| {}),
                http(200, approved_with_everything()),
            ),
        ),
        (
            "setup_mandate/challenge",
            handled(&setup_mandate(|_| {}), http(200, challenge())),
        ),
        (
            "void/approved",
            handled(
                &void(),
                http(
                    200,
                    payment_response(SUCCESS, serde_json::json!({"paymentType": "RV"})),
                ),
            ),
        ),
        ("void/declined", handled(&void(), http(200, declined()))),
        (
            "capture/approved",
            handled(
                &capture(),
                http(
                    200,
                    payment_response(
                        SUCCESS,
                        serde_json::json!({
                            "paymentType": "CP", "referencedId": "8ac7a4a1capture", "descriptor": "desc",
                        }),
                    ),
                ),
            ),
        ),
        (
            "capture/declined",
            handled(
                &capture(),
                http(
                    200,
                    payment_response(
                        DECLINED,
                        serde_json::json!({
                            "paymentType": "CP", "referencedId": "8ac7a4a1capture", "descriptor": "desc",
                            "resultDetails": {"MerchantAdviceCode": "03"},
                        }),
                    ),
                ),
            ),
        ),
        (
            "capture/pending",
            handled(
                &capture(),
                http(
                    200,
                    payment_response(
                        PENDING,
                        serde_json::json!({
                            "paymentType": "CP", "referencedId": "8ac7a4a1capture", "descriptor": "desc",
                        }),
                    ),
                ),
            ),
        ),
        (
            "refund/approved",
            handled(
                &refund(),
                http(
                    200,
                    payment_response(SUCCESS, serde_json::json!({"paymentType": "RF"})),
                ),
            ),
        ),
        (
            "refund/pending",
            handled(
                &refund(),
                http(
                    200,
                    payment_response(PENDING, serde_json::json!({"paymentType": "RF"})),
                ),
            ),
        ),
        ("refund/declined", handled(&refund(), http(200, declined()))),
        (
            "pre_authentication/declined",
            handled(&pre_authentication(), http(200, declined())),
        ),
        (
            "pre_authentication/frictionless",
            handled(&pre_authentication(), http(200, approved_with_everything())),
        ),
        (
            "pre_authentication/challenge",
            handled(&pre_authentication(), http(200, challenge())),
        ),
        (
            "pre_authentication/challenge_without_version",
            handled(&pre_authentication(), http(200, bank_redirect())),
        ),
        (
            "post_authentication/approved",
            handled(
                &post_authentication(),
                http(200, approved_with_everything()),
            ),
        ),
        (
            "post_authentication/pending",
            handled(
                &post_authentication(),
                http(200, payment_response(PENDING, serde_json::json!({}))),
            ),
        ),
        (
            "post_authentication/declined",
            handled(&post_authentication(), http(200, declined())),
        ),
    ]
}

fn error_response(body: serde_json::Value) -> String {
    let response = http(400, body);
    match ConnectorIntegration::<Authorize, PaymentsAuthorizeData, PaymentsResponseData>::get_error_response(
        Aci::new(),
        response,
        None,
    ) {
        Ok(error) => format!("{error:#?}\n"),
        Err(error) => format!("error: {}\n", error.current_context()),
    }
}

fn error_scenarios() -> Vec<(&'static str, String)> {
    vec![
        (
            "parameter_errors",
            error_response(serde_json::json!({
                "result": {
                    "code": "200.300.404",
                    "description": "invalid or missing parameter",
                    "parameterErrors": [
                        {"name": "card.number", "value": "4111", "message": "invalid card number"},
                        {"name": "amount", "message": "missing"},
                    ],
                },
                "buildNumber": "build", "timestamp": "t", "ndc": "ndc",
            })),
        ),
        (
            "acquirer_decline",
            error_response(serde_json::json!({
                "id": "8ac7a4a1declined",
                "result": {"code": DECLINED, "description": "declined"},
                "resultDetails": {"MerchantAdviceCode": "21", "ExtendedDescription": "Do not honour"},
                "buildNumber": "build", "timestamp": "t", "ndc": "ndc",
            })),
        ),
        ("not_json", {
            match ConnectorIntegration::<Authorize, PaymentsAuthorizeData, PaymentsResponseData>::get_error_response(
                Aci::new(),
                Response { headers: None, response: "gateway timeout".into(), status_code: 504 },
                None,
            ) {
                Ok(error) => format!("{error:#?}\n"),
                Err(error) => format!("error: {}\n", error.current_context()),
            }
        }),
    ]
}

// ---- the snapshot -------------------------------------------------------------------------------

fn render_all() -> String {
    let mut out = String::new();
    let mut section = |name: &str, rendered: String| {
        let _ = write!(out, "=== {name}\n{rendered}\n");
    };
    for (name, router_data) in authorize_scenarios() {
        section(name, build(&router_data));
    }
    for (name, router_data) in setup_mandate_scenarios() {
        section(name, build(&router_data));
    }
    section("sync", build(&sync()));
    section("capture", build(&capture()));
    section("void", build(&void()));
    section("refund", build(&refund()));
    section("pre_authentication", build(&pre_authentication()));
    section("post_authentication", build(&post_authentication()));
    for (name, body) in webhook_scenarios() {
        section(&format!("webhook/{name}"), webhook(&body));
    }
    for (name, delivery) in webhook_delivery_scenarios() {
        section(&format!("webhook/delivery/{name}"), delivery);
    }
    section("supported_payment_methods", supported_payment_methods());
    for (name, rendered) in response_scenarios() {
        section(&format!("response/{name}"), rendered);
    }
    for (name, rendered) in error_scenarios() {
        section(&format!("error_response/{name}"), rendered);
    }
    out
}

#[test]
fn outbound_requests_match_the_golden_snapshot() {
    let actual = render_all();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(SNAPSHOT_PATH);
    if std::env::var_os("ACI_GOLDEN_BLESS").is_some() {
        std::fs::write(&path, &actual).expect("snapshot is writable");
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_default();
    if actual != expected {
        let first_difference = actual
            .lines()
            .zip(expected.lines())
            .enumerate()
            .find(|(_, (a, e))| a != e)
            .map(|(line, (a, e))| format!("line {}:\n  actual:   {a}\n  expected: {e}", line + 1))
            .unwrap_or_else(|| "one snapshot is a prefix of the other".to_string());
        panic!(
            "ACI outbound requests changed ({first_difference}).\nIf intended, re-run with \
             ACI_GOLDEN_BLESS=1 and review the diff of {SNAPSHOT_PATH}."
        );
    }
}

// ---- beyond the snapshot ------------------------------------------------------------------------

/// Headers depend on the account alone: a flow whose URL needs an id the request lacks still
/// gets its headers, and only the URL reports the missing id.
#[test]
fn headers_never_need_the_request_ids_the_url_does() {
    let mut router_data = sync();
    router_data.request.connector_transaction_id = ResponseId::NoResponseId;

    let headers =
        ConnectorIntegration::<PSync, PaymentsSyncData, PaymentsResponseData>::get_headers(
            Aci::new(),
            &router_data,
            &connectors(),
        )
        .expect("headers need no transaction id");
    let names: Vec<_> = headers.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(names, ["Content-Type", "Authorization"]);

    assert_eq!(
        build(&router_data),
        "error: Missing connector transaction ID\n"
    );
}

/// Which fault a request faulty twice over reports: the transaction's for every method but a
/// card, whose instrument is validated first — the order each path had before the adapters.
#[test]
fn a_request_faulty_twice_reports_the_same_fault_as_before() {
    let faulty_twice = |payment_method_data| {
        with(authorize(payment_method_data), |rd| {
            rd.request.capture_method = Some(enums::CaptureMethod::Scheduled);
            rd.address = PaymentAddress::default();
        })
    };
    let bank_redirect = faulty_twice(PaymentMethodData::BankRedirect(BankRedirectData::Eps {
        bank_name: None,
        country: None,
    }));
    let cvv_less_card = faulty_twice(PaymentMethodData::CardWithOptionalCVC(
        card_with_optional_cvc(None),
    ));
    assert_eq!(
        build(&bank_redirect),
        "error: Capture method not supported\n"
    );
    assert_eq!(
        build(&cvv_less_card),
        "error: Missing required field: payment_method_data.card.card_cvc\n"
    );
}
