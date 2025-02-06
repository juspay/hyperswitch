use api_models::payments;
use base64::Engine;
use common_enums::{enums, FutureUsage};
use common_utils::{
    consts, pii,
    types::{SemanticVersion, StringMajorUnit},
};
use hyperswitch_domain_models::{
    payment_method_data::{
        ApplePayWalletData, BankDebitData, GooglePayWalletData, PaymentMethodData, WalletData,
    },
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ApplePayPredecryptData, ConnectorAuthType,
        ConnectorResponseData, ErrorResponse, PaymentMethodToken, RouterData,
    },
    router_flow_types::{
        payments::Authorize,
        refunds::{Execute, RSync},
        SetupMandate,
    },
    router_request_types::{
        PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData, PaymentsSyncData,
        ResponseId, SetupMandateRequestData,
    },
    router_response_types::{MandateReference, PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsIncrementalAuthorizationRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{api, errors};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    constants,
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::{
        self, AddressDetailsData, ApplePayDecrypt, CardData, PaymentsAuthorizeRequestData,
        PaymentsSetupMandateRequestData, PaymentsSyncRequestData, RecurringMandateData,
        RouterData as OtherRouterData,
    },
};

#[derive(Debug, Serialize)]
pub struct WellsfargoRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for WellsfargoRouterData<T> {
    fn from((amount, router_data): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoZeroMandateRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
}

impl TryFrom<&SetupMandateRouterData> for WellsfargoZeroMandateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        let email = item.request.get_email()?;
        let bill_to = build_bill_to(item.get_optional_billing(), email)?;

        let order_information = OrderInformationWithBill {
            amount_details: Amount {
                total_amount: StringMajorUnit::zero(),
                currency: item.request.currency,
            },
            bill_to: Some(bill_to),
        };
        let (action_list, action_token_types, authorization_options) = (
            Some(vec![WellsfargoActionsList::TokenCreate]),
            Some(vec![
                WellsfargoActionsTokenType::PaymentInstrument,
                WellsfargoActionsTokenType::Customer,
            ]),
            Some(WellsfargoAuthorizationOptions {
                initiator: Some(WellsfargoPaymentInitiator {
                    initiator_type: Some(WellsfargoPaymentInitiatorTypes::Customer),
                    credential_stored_on_file: Some(true),
                    stored_credential_used: None,
                }),
                merchant_intitiated_transaction: None,
            }),
        );

        let client_reference_information = ClientReferenceInformation {
            code: Some(item.connector_request_reference_id.clone()),
        };

        let (payment_information, solution) = match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(ccard) => {
                let card_issuer = ccard.get_card_issuer();
                let card_type = match card_issuer {
                    Ok(issuer) => Some(String::from(issuer)),
                    Err(_) => None,
                };
                (
                    PaymentInformation::Cards(Box::new(CardPaymentInformation {
                        card: Card {
                            number: ccard.card_number,
                            expiration_month: ccard.card_exp_month,
                            expiration_year: ccard.card_exp_year,
                            security_code: Some(ccard.card_cvc),
                            card_type,
                        },
                    })),
                    None,
                )
            }

            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::ApplePay(apple_pay_data) => match item.payment_method_token.clone() {
                    Some(payment_method_token) => match payment_method_token {
                        PaymentMethodToken::ApplePayDecrypt(decrypt_data) => {
                            let expiration_month = decrypt_data.get_expiry_month()?;
                            let expiration_year = decrypt_data.get_four_digit_expiry_year()?;
                            (
                                PaymentInformation::ApplePay(Box::new(
                                    ApplePayPaymentInformation {
                                        tokenized_card: TokenizedCard {
                                            number: decrypt_data.application_primary_account_number,
                                            cryptogram: decrypt_data
                                                .payment_data
                                                .online_payment_cryptogram,
                                            transaction_type: TransactionType::ApplePay,
                                            expiration_year,
                                            expiration_month,
                                        },
                                    },
                                )),
                                Some(PaymentSolution::ApplePay),
                            )
                        }
                        PaymentMethodToken::Token(_) => Err(unimplemented_payment_method!(
                            "Apple Pay",
                            "Manual",
                            "Wellsfargo"
                        ))?,
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Wellsfargo"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(_) => {
                            Err(unimplemented_payment_method!("Google Pay", "Wellsfargo"))?
                        }
                    },
                    None => (
                        PaymentInformation::ApplePayToken(Box::new(
                            ApplePayTokenPaymentInformation {
                                fluid_data: FluidData {
                                    value: Secret::from(apple_pay_data.payment_data),
                                    descriptor: Some(FLUID_DATA_DESCRIPTOR.to_string()),
                                },
                                tokenized_card: ApplePayTokenizedCard {
                                    transaction_type: TransactionType::ApplePay,
                                },
                            },
                        )),
                        Some(PaymentSolution::ApplePay),
                    ),
                },
                WalletData::GooglePay(google_pay_data) => (
                    PaymentInformation::GooglePay(Box::new(GooglePayPaymentInformation {
                        fluid_data: FluidData {
                            value: Secret::from(
                                consts::BASE64_ENGINE
                                    .encode(google_pay_data.tokenization_data.token),
                            ),
                            descriptor: None,
                        },
                    })),
                    Some(PaymentSolution::GooglePay),
                ),
                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::AmazonPayRedirect(_)
                | WalletData::MomoRedirect(_)
                | WalletData::KakaoPayRedirect(_)
                | WalletData::GoPayRedirect(_)
                | WalletData::GcashRedirect(_)
                | WalletData::ApplePayRedirect(_)
                | WalletData::ApplePayThirdPartySdk(_)
                | WalletData::DanaRedirect {}
                | WalletData::GooglePayRedirect(_)
                | WalletData::GooglePayThirdPartySdk(_)
                | WalletData::MbWayRedirect(_)
                | WalletData::MobilePayRedirect(_)
                | WalletData::PaypalRedirect(_)
                | WalletData::PaypalSdk(_)
                | WalletData::Paze(_)
                | WalletData::SamsungPay(_)
                | WalletData::TwintRedirect {}
                | WalletData::VippsRedirect {}
                | WalletData::TouchNGoRedirect(_)
                | WalletData::WeChatPayRedirect(_)
                | WalletData::WeChatPayQr(_)
                | WalletData::CashappQr(_)
                | WalletData::SwishQr(_)
                | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Wellsfargo"),
                ))?,
            },
            PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::PayLater(_)
            | PaymentMethodData::BankRedirect(_)
            | PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::MandatePayment
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("Wellsfargo"),
                ))?
            }
        };

        let processing_information = ProcessingInformation {
            capture: Some(false),
            capture_options: None,
            action_list,
            action_token_types,
            authorization_options,
            commerce_indicator: String::from("internet"),
            payment_solution: solution.map(String::from),
        };
        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoPaymentsRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    consumer_authentication_information: Option<WellsfargoConsumerAuthInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInformation {
    action_list: Option<Vec<WellsfargoActionsList>>,
    action_token_types: Option<Vec<WellsfargoActionsTokenType>>,
    authorization_options: Option<WellsfargoAuthorizationOptions>,
    commerce_indicator: String,
    capture: Option<bool>,
    capture_options: Option<CaptureOptions>,
    payment_solution: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoConsumerAuthInformation {
    ucaf_collection_indicator: Option<String>,
    cavv: Option<String>,
    ucaf_authentication_data: Option<Secret<String>>,
    xid: Option<String>,
    directory_server_transaction_id: Option<Secret<String>>,
    specification_version: Option<String>,
    /// This field specifies the 3ds version
    pa_specification_version: Option<SemanticVersion>,
    /// Verification response enrollment status.
    ///
    /// This field is supported only on Asia, Middle East, and Africa Gateway.
    ///
    /// For external authentication, this field will always be "Y"
    veres_enrolled: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantDefinedInformation {
    key: u8,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WellsfargoActionsList {
    TokenCreate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WellsfargoActionsTokenType {
    Customer,
    PaymentInstrument,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoAuthorizationOptions {
    initiator: Option<WellsfargoPaymentInitiator>,
    merchant_intitiated_transaction: Option<MerchantInitiatedTransaction>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MerchantInitiatedTransaction {
    reason: Option<String>,
    previous_transaction_id: Option<Secret<String>>,
    //Required for recurring mandates payment
    original_authorized_amount: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoPaymentInitiator {
    #[serde(rename = "type")]
    initiator_type: Option<WellsfargoPaymentInitiatorTypes>,
    credential_stored_on_file: Option<bool>,
    stored_credential_used: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WellsfargoPaymentInitiatorTypes {
    Customer,
    Merchant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOptions {
    capture_sequence_number: u32,
    total_capture_count: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPaymentInformation {
    card: Card,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TokenizedCard {
    number: Secret<String>,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    cryptogram: Secret<String>,
    transaction_type: TransactionType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayTokenizedCard {
    transaction_type: TransactionType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayTokenPaymentInformation {
    fluid_data: FluidData,
    tokenized_card: ApplePayTokenizedCard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplePayPaymentInformation {
    tokenized_card: TokenizedCard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MandatePaymentInformation {
    payment_instrument: WellsfargoPaymentInstrument,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct AchBankAccount {
    account: Account,
    routing_number: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Account {
    #[serde(rename = "type")]
    account_type: AccountType,
    number: Secret<String>,
}
#[derive(Debug, Deserialize, Serialize)]
enum AccountType {
    /// Checking account type.
    C,
    /// General ledger account type. Supported only on Wells Fargo ACH.
    G,
    /// Savings account type.
    S,
    /// Corporate checking account type.
    X,
}

#[derive(Debug, Serialize)]
pub struct AchPaymentInformation {
    bank: AchBankAccount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FluidData {
    value: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    descriptor: Option<String>,
}

pub const FLUID_DATA_DESCRIPTOR: &str = "RklEPUNPTU1PTi5BUFBMRS5JTkFQUC5QQVlNRU5U";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayPaymentInformation {
    fluid_data: FluidData,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentInformation {
    Cards(Box<CardPaymentInformation>),
    GooglePay(Box<GooglePayPaymentInformation>),
    ApplePay(Box<ApplePayPaymentInformation>),
    ApplePayToken(Box<ApplePayTokenPaymentInformation>),
    MandatePayment(Box<MandatePaymentInformation>),
    AchDebitPayment(Box<AchPaymentInformation>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WellsfargoPaymentInstrument {
    id: Secret<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    number: cards::CardNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    security_code: Option<Secret<String>>,
    #[serde(rename = "type")]
    card_type: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationWithBill {
    amount_details: Amount,
    bill_to: Option<BillTo>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformationIncrementalAuthorization {
    amount_details: AdditionalAmount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderInformation {
    amount_details: Amount,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Amount {
    total_amount: StringMajorUnit,
    currency: api_models::enums::Currency,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdditionalAmount {
    additional_amount: StringMajorUnit,
    currency: String,
}

#[derive(Debug, Serialize)]
pub enum PaymentSolution {
    ApplePay,
    GooglePay,
}

#[derive(Debug, Serialize)]
pub enum TransactionType {
    #[serde(rename = "1")]
    ApplePay,
}

impl From<PaymentSolution> for String {
    fn from(solution: PaymentSolution) -> Self {
        let payment_solution = match solution {
            PaymentSolution::ApplePay => "001",
            PaymentSolution::GooglePay => "012",
        };
        payment_solution.to_string()
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BillTo {
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
    address1: Option<Secret<String>>,
    locality: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    administrative_area: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    postal_code: Option<Secret<String>>,
    country: Option<enums::CountryAlpha2>,
    email: pii::Email,
    phone_number: Option<Secret<String>>,
}

impl From<&WellsfargoRouterData<&PaymentsAuthorizeRouterData>> for ClientReferenceInformation {
    fn from(item: &WellsfargoRouterData<&PaymentsAuthorizeRouterData>) -> Self {
        Self {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        }
    }
}

impl
    TryFrom<(
        &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
        Option<PaymentSolution>,
        Option<String>,
    )> for ProcessingInformation
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, solution, network): (
            &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
            Option<PaymentSolution>,
            Option<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let mut commerce_indicator = solution
            .as_ref()
            .map(|pm_solution| match pm_solution {
                PaymentSolution::ApplePay => network
                    .as_ref()
                    .map(|card_network| match card_network.to_lowercase().as_str() {
                        "amex" => "aesk",
                        "discover" => "dipb",
                        "mastercard" => "spa",
                        "visa" => "internet",
                        _ => "internet",
                    })
                    .unwrap_or("internet"),
                PaymentSolution::GooglePay => "internet",
            })
            .unwrap_or("internet")
            .to_string();

        let (action_list, action_token_types, authorization_options) = if item
            .router_data
            .request
            .setup_future_usage
            == Some(FutureUsage::OffSession)
            && (item.router_data.request.customer_acceptance.is_some()
                || item
                    .router_data
                    .request
                    .setup_mandate_details
                    .clone()
                    .is_some_and(|mandate_details| mandate_details.customer_acceptance.is_some()))
        {
            (
                Some(vec![WellsfargoActionsList::TokenCreate]),
                Some(vec![
                    WellsfargoActionsTokenType::PaymentInstrument,
                    WellsfargoActionsTokenType::Customer,
                ]),
                Some(WellsfargoAuthorizationOptions {
                    initiator: Some(WellsfargoPaymentInitiator {
                        initiator_type: Some(WellsfargoPaymentInitiatorTypes::Customer),
                        credential_stored_on_file: Some(true),
                        stored_credential_used: None,
                    }),
                    merchant_intitiated_transaction: None,
                }),
            )
        } else if item.router_data.request.mandate_id.is_some() {
            match item
                .router_data
                .request
                .mandate_id
                .clone()
                .and_then(|mandate_id| mandate_id.mandate_reference_id)
            {
                Some(payments::MandateReferenceId::ConnectorMandateId(_)) => {
                    let original_amount = item
                        .router_data
                        .get_recurring_mandate_payment_data()?
                        .get_original_payment_amount()?;
                    let original_currency = item
                        .router_data
                        .get_recurring_mandate_payment_data()?
                        .get_original_payment_currency()?;
                    (
                        None,
                        None,
                        Some(WellsfargoAuthorizationOptions {
                            initiator: None,
                            merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                                reason: None,
                                original_authorized_amount: Some(utils::get_amount_as_string(
                                    &api::CurrencyUnit::Base,
                                    original_amount,
                                    original_currency,
                                )?),
                                previous_transaction_id: None,
                            }),
                        }),
                    )
                }
                Some(payments::MandateReferenceId::NetworkMandateId(network_transaction_id)) => {
                    let (original_amount, original_currency) = match network
                        .clone()
                        .map(|network| network.to_lowercase())
                        .as_deref()
                    {
                        Some("discover") => {
                            let original_amount = Some(
                                item.router_data
                                    .get_recurring_mandate_payment_data()?
                                    .get_original_payment_amount()?,
                            );
                            let original_currency = Some(
                                item.router_data
                                    .get_recurring_mandate_payment_data()?
                                    .get_original_payment_currency()?,
                            );
                            (original_amount, original_currency)
                        }
                        _ => {
                            let original_amount = item
                                .router_data
                                .recurring_mandate_payment_data
                                .as_ref()
                                .and_then(|recurring_mandate_payment_data| {
                                    recurring_mandate_payment_data
                                        .original_payment_authorized_amount
                                });

                            let original_currency = item
                                .router_data
                                .recurring_mandate_payment_data
                                .as_ref()
                                .and_then(|recurring_mandate_payment_data| {
                                    recurring_mandate_payment_data
                                        .original_payment_authorized_currency
                                });

                            (original_amount, original_currency)
                        }
                    };

                    let original_authorized_amount = match (original_amount, original_currency) {
                        (Some(original_amount), Some(original_currency)) => Some(
                            utils::to_currency_base_unit(original_amount, original_currency)?,
                        ),
                        _ => None,
                    };
                    commerce_indicator = "recurring".to_string();
                    (
                        None,
                        None,
                        Some(WellsfargoAuthorizationOptions {
                            initiator: Some(WellsfargoPaymentInitiator {
                                initiator_type: Some(WellsfargoPaymentInitiatorTypes::Merchant),
                                credential_stored_on_file: None,
                                stored_credential_used: Some(true),
                            }),
                            merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                                reason: Some("7".to_string()),
                                original_authorized_amount,
                                previous_transaction_id: Some(Secret::new(network_transaction_id)),
                            }),
                        }),
                    )
                }
                Some(payments::MandateReferenceId::NetworkTokenWithNTI(_)) | None => {
                    (None, None, None)
                }
            }
        } else {
            (None, None, None)
        };
        // this logic is for external authenticated card
        let commerce_indicator_for_external_authentication = item
            .router_data
            .request
            .authentication_data
            .as_ref()
            .and_then(|authn_data| {
                authn_data
                    .eci
                    .clone()
                    .map(|eci| get_commerce_indicator_for_external_authentication(network, eci))
            });

        Ok(Self {
            capture: Some(matches!(
                item.router_data.request.capture_method,
                Some(enums::CaptureMethod::Automatic)
                    | Some(enums::CaptureMethod::SequentialAutomatic)
                    | None
            )),
            payment_solution: solution.map(String::from),
            action_list,
            action_token_types,
            authorization_options,
            capture_options: None,
            commerce_indicator: commerce_indicator_for_external_authentication
                .unwrap_or(commerce_indicator),
        })
    }
}

fn get_commerce_indicator_for_external_authentication(
    card_network: Option<String>,
    eci: String,
) -> String {
    let card_network_lower_case = card_network
        .as_ref()
        .map(|card_network| card_network.to_lowercase());
    match eci.as_str() {
        "00" | "01" | "02" => {
            if matches!(
                card_network_lower_case.as_deref(),
                Some("mastercard") | Some("maestro")
            ) {
                "spa"
            } else {
                "internet"
            }
        }
        "05" => match card_network_lower_case.as_deref() {
            Some("amex") => "aesk",
            Some("discover") => "dipb",
            Some("mastercard") => "spa",
            Some("visa") => "vbv",
            Some("diners") => "pb",
            Some("upi") => "up3ds",
            _ => "internet",
        },
        "06" => match card_network_lower_case.as_deref() {
            Some("amex") => "aesk_attempted",
            Some("discover") => "dipb_attempted",
            Some("mastercard") => "spa",
            Some("visa") => "vbv_attempted",
            Some("diners") => "pb_attempted",
            Some("upi") => "up3ds_attempted",
            _ => "internet",
        },
        "07" => match card_network_lower_case.as_deref() {
            Some("amex") => "internet",
            Some("discover") => "internet",
            Some("mastercard") => "spa",
            Some("visa") => "vbv_failure",
            Some("diners") => "internet",
            Some("upi") => "up3ds_failure",
            _ => "internet",
        },
        _ => "vbv_failure",
    }
    .to_string()
}

impl
    From<(
        &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
        Option<BillTo>,
    )> for OrderInformationWithBill
{
    fn from(
        (item, bill_to): (
            &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
            Option<BillTo>,
        ),
    ) -> Self {
        Self {
            amount_details: Amount {
                total_amount: item.amount.to_owned(),
                currency: item.router_data.request.currency,
            },
            bill_to,
        }
    }
}

fn get_phone_number(
    item: Option<&hyperswitch_domain_models::address::Address>,
) -> Option<Secret<String>> {
    item.as_ref()
        .and_then(|billing| billing.phone.as_ref())
        .and_then(|phone| {
            phone.number.as_ref().and_then(|number| {
                phone
                    .country_code
                    .as_ref()
                    .map(|cc| Secret::new(format!("{}{}", cc, number.peek())))
            })
        })
}

fn build_bill_to(
    address_details: Option<&hyperswitch_domain_models::address::Address>,
    email: pii::Email,
) -> Result<BillTo, error_stack::Report<errors::ConnectorError>> {
    let phone_number = get_phone_number(address_details);
    let default_address = BillTo {
        first_name: None,
        last_name: None,
        address1: None,
        locality: None,
        administrative_area: None,
        postal_code: None,
        country: None,
        email: email.clone(),
        phone_number: phone_number.clone(),
    };
    let ad = Ok(address_details
        .and_then(|addr| {
            addr.address.as_ref().map(|addr| BillTo {
                first_name: addr.first_name.clone(),
                last_name: addr.last_name.clone(),
                address1: addr.line1.clone(),
                locality: addr.city.clone(),
                administrative_area: addr.to_state_code_as_optional().ok().flatten(),
                postal_code: addr.zip.clone(),
                country: addr.country,
                email,
                phone_number: phone_number.clone(),
            })
        })
        .unwrap_or(default_address));
    ad
}

fn convert_metadata_to_merchant_defined_info(metadata: Value) -> Vec<MerchantDefinedInformation> {
    let hashmap: std::collections::BTreeMap<String, Value> =
        serde_json::from_str(&metadata.to_string()).unwrap_or(std::collections::BTreeMap::new());
    let mut vector = Vec::new();
    let mut iter = 1;
    for (key, value) in hashmap {
        vector.push(MerchantDefinedInformation {
            key: iter,
            value: format!("{key}={value}"),
        });
        iter += 1;
    }
    vector
}

impl
    TryFrom<(
        &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
        hyperswitch_domain_models::payment_method_data::Card,
    )> for WellsfargoPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
            hyperswitch_domain_models::payment_method_data::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));

        let card_issuer = ccard.get_card_issuer();
        let card_type = match card_issuer {
            Ok(issuer) => Some(String::from(issuer)),
            Err(_) => None,
        };

        let payment_information = PaymentInformation::Cards(Box::new(CardPaymentInformation {
            card: Card {
                number: ccard.card_number,
                expiration_month: ccard.card_exp_month,
                expiration_year: ccard.card_exp_year,
                security_code: Some(ccard.card_cvc),
                card_type: card_type.clone(),
            },
        }));

        let processing_information = ProcessingInformation::try_from((item, None, card_type))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);

        let consumer_authentication_information = item
            .router_data
            .request
            .authentication_data
            .as_ref()
            .map(|authn_data| {
                let (ucaf_authentication_data, cavv) =
                    if ccard.card_network == Some(common_enums::CardNetwork::Mastercard) {
                        (Some(Secret::new(authn_data.cavv.clone())), None)
                    } else {
                        (None, Some(authn_data.cavv.clone()))
                    };
                WellsfargoConsumerAuthInformation {
                    ucaf_collection_indicator: None,
                    cavv,
                    ucaf_authentication_data,
                    xid: Some(authn_data.threeds_server_transaction_id.clone()),
                    directory_server_transaction_id: authn_data
                        .ds_trans_id
                        .clone()
                        .map(Secret::new),
                    specification_version: None,
                    pa_specification_version: Some(authn_data.message_version.clone()),
                    veres_enrolled: Some("Y".to_string()),
                }
            });

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information,
            merchant_defined_information,
        })
    }
}

impl
    TryFrom<(
        &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
        Box<ApplePayPredecryptData>,
        ApplePayWalletData,
    )> for WellsfargoPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, apple_pay_data, apple_pay_wallet_data): (
            &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
            Box<ApplePayPredecryptData>,
            ApplePayWalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));
        let processing_information = ProcessingInformation::try_from((
            item,
            Some(PaymentSolution::ApplePay),
            Some(apple_pay_wallet_data.payment_method.network.clone()),
        ))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let expiration_month = apple_pay_data.get_expiry_month()?;
        let expiration_year = apple_pay_data.get_four_digit_expiry_year()?;
        let payment_information =
            PaymentInformation::ApplePay(Box::new(ApplePayPaymentInformation {
                tokenized_card: TokenizedCard {
                    number: apple_pay_data.application_primary_account_number,
                    cryptogram: apple_pay_data.payment_data.online_payment_cryptogram,
                    transaction_type: TransactionType::ApplePay,
                    expiration_year,
                    expiration_month,
                },
            }));
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);
        let ucaf_collection_indicator = match apple_pay_wallet_data
            .payment_method
            .network
            .to_lowercase()
            .as_str()
        {
            "mastercard" => Some("2".to_string()),
            _ => None,
        };
        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information: Some(WellsfargoConsumerAuthInformation {
                ucaf_collection_indicator,
                cavv: None,
                ucaf_authentication_data: None,
                xid: None,
                directory_server_transaction_id: None,
                specification_version: None,
                pa_specification_version: None,
                veres_enrolled: None,
            }),
            merchant_defined_information,
        })
    }
}

impl
    TryFrom<(
        &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
        GooglePayWalletData,
    )> for WellsfargoPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, google_pay_data): (
            &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
            GooglePayWalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));

        let payment_information =
            PaymentInformation::GooglePay(Box::new(GooglePayPaymentInformation {
                fluid_data: FluidData {
                    value: Secret::from(
                        consts::BASE64_ENGINE.encode(google_pay_data.tokenization_data.token),
                    ),
                    descriptor: None,
                },
            }));
        let processing_information =
            ProcessingInformation::try_from((item, Some(PaymentSolution::GooglePay), None))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information: None,
            merchant_defined_information,
        })
    }
}

impl TryFrom<Option<common_enums::BankType>> for AccountType {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(optional_bank_type: Option<common_enums::BankType>) -> Result<Self, Self::Error> {
        match optional_bank_type {
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "bank_type",
            })?,
            Some(bank_type) => match bank_type {
                common_enums::BankType::Checking => Ok(Self::C),
                common_enums::BankType::Savings => Ok(Self::S),
            },
        }
    }
}

impl
    TryFrom<(
        &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
        BankDebitData,
    )> for WellsfargoPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, bank_debit_data): (
            &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
            BankDebitData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));
        let payment_information = match bank_debit_data {
            BankDebitData::AchBankDebit {
                account_number,
                routing_number,
                bank_type,
                ..
            } => Ok(PaymentInformation::AchDebitPayment(Box::new(
                AchPaymentInformation {
                    bank: AchBankAccount {
                        account: Account {
                            account_type: AccountType::try_from(bank_type)?,
                            number: account_number,
                        },
                        routing_number,
                    },
                },
            ))),
            BankDebitData::SepaBankDebit { .. }
            | BankDebitData::BacsBankDebit { .. }
            | BankDebitData::BecsBankDebit { .. } => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Wellsfargo"),
            )),
        }?;
        let processing_information =
            ProcessingInformation::try_from((item, Some(PaymentSolution::GooglePay), None))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information: None,
            merchant_defined_information: None,
        })
    }
}

impl TryFrom<&WellsfargoRouterData<&PaymentsAuthorizeRouterData>> for WellsfargoPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WellsfargoRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.connector_mandate_id() {
            Some(connector_mandate_id) => Self::try_from((item, connector_mandate_id)),
            None => {
                match item.router_data.request.payment_method_data.clone() {
                    PaymentMethodData::Card(ccard) => Self::try_from((item, ccard)),
                    PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                        WalletData::ApplePay(apple_pay_data) => {
                            match item.router_data.payment_method_token.clone() {
                                Some(payment_method_token) => match payment_method_token {
                                    PaymentMethodToken::ApplePayDecrypt(decrypt_data) => {
                                        Self::try_from((item, decrypt_data, apple_pay_data))
                                    }
                                    PaymentMethodToken::Token(_) => {
                                        Err(unimplemented_payment_method!(
                                            "Apple Pay",
                                            "Manual",
                                            "Wellsfargo"
                                        ))?
                                    }
                                    PaymentMethodToken::PazeDecrypt(_) => {
                                        Err(unimplemented_payment_method!("Paze", "Wellsfargo"))?
                                    }
                                    PaymentMethodToken::GooglePayDecrypt(_) => Err(
                                        unimplemented_payment_method!("Google Pay", "Wellsfargo"),
                                    )?,
                                },
                                None => {
                                    let email = item.router_data.request.get_email()?;
                                    let bill_to = build_bill_to(
                                        item.router_data.get_optional_billing(),
                                        email,
                                    )?;
                                    let order_information =
                                        OrderInformationWithBill::from((item, Some(bill_to)));
                                    let processing_information =
                                        ProcessingInformation::try_from((
                                            item,
                                            Some(PaymentSolution::ApplePay),
                                            Some(apple_pay_data.payment_method.network.clone()),
                                        ))?;
                                    let client_reference_information =
                                        ClientReferenceInformation::from(item);
                                    let payment_information = PaymentInformation::ApplePayToken(
                                        Box::new(ApplePayTokenPaymentInformation {
                                            fluid_data: FluidData {
                                                value: Secret::from(apple_pay_data.payment_data),
                                                descriptor: Some(FLUID_DATA_DESCRIPTOR.to_string()),
                                            },
                                            tokenized_card: ApplePayTokenizedCard {
                                                transaction_type: TransactionType::ApplePay,
                                            },
                                        }),
                                    );
                                    let merchant_defined_information =
                                        item.router_data.request.metadata.clone().map(|metadata| {
                                            convert_metadata_to_merchant_defined_info(metadata)
                                        });
                                    let ucaf_collection_indicator = match apple_pay_data
                                        .payment_method
                                        .network
                                        .to_lowercase()
                                        .as_str()
                                    {
                                        "mastercard" => Some("2".to_string()),
                                        _ => None,
                                    };
                                    Ok(Self {
                                        processing_information,
                                        payment_information,
                                        order_information,
                                        client_reference_information,
                                        merchant_defined_information,
                                        consumer_authentication_information: Some(
                                            WellsfargoConsumerAuthInformation {
                                                ucaf_collection_indicator,
                                                cavv: None,
                                                ucaf_authentication_data: None,
                                                xid: None,
                                                directory_server_transaction_id: None,
                                                specification_version: None,
                                                pa_specification_version: None,
                                                veres_enrolled: None,
                                            },
                                        ),
                                    })
                                }
                            }
                        }
                        WalletData::GooglePay(google_pay_data) => {
                            Self::try_from((item, google_pay_data))
                        }
                        WalletData::AliPayQr(_)
                        | WalletData::AliPayRedirect(_)
                        | WalletData::AliPayHkRedirect(_)
                        | WalletData::AmazonPayRedirect(_)
                        | WalletData::MomoRedirect(_)
                        | WalletData::KakaoPayRedirect(_)
                        | WalletData::GoPayRedirect(_)
                        | WalletData::GcashRedirect(_)
                        | WalletData::ApplePayRedirect(_)
                        | WalletData::ApplePayThirdPartySdk(_)
                        | WalletData::DanaRedirect {}
                        | WalletData::GooglePayRedirect(_)
                        | WalletData::GooglePayThirdPartySdk(_)
                        | WalletData::MbWayRedirect(_)
                        | WalletData::MobilePayRedirect(_)
                        | WalletData::PaypalRedirect(_)
                        | WalletData::PaypalSdk(_)
                        | WalletData::Paze(_)
                        | WalletData::SamsungPay(_)
                        | WalletData::TwintRedirect {}
                        | WalletData::VippsRedirect {}
                        | WalletData::TouchNGoRedirect(_)
                        | WalletData::WeChatPayRedirect(_)
                        | WalletData::WeChatPayQr(_)
                        | WalletData::CashappQr(_)
                        | WalletData::SwishQr(_)
                        | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message("Wellsfargo"),
                        )
                        .into()),
                    },
                    // If connector_mandate_id is present MandatePayment will be the PMD, the case will be handled in the first `if` clause.
                    // This is a fallback implementation in the event of catastrophe.
                    PaymentMethodData::MandatePayment => {
                        let connector_mandate_id =
                            item.router_data.request.connector_mandate_id().ok_or(
                                errors::ConnectorError::MissingRequiredField {
                                    field_name: "connector_mandate_id",
                                },
                            )?;
                        Self::try_from((item, connector_mandate_id))
                    }
                    PaymentMethodData::BankDebit(bank_debit) => Self::try_from((item, bank_debit)),
                    PaymentMethodData::CardRedirect(_)
                    | PaymentMethodData::PayLater(_)
                    | PaymentMethodData::BankRedirect(_)
                    | PaymentMethodData::BankTransfer(_)
                    | PaymentMethodData::Crypto(_)
                    | PaymentMethodData::Reward
                    | PaymentMethodData::RealTimePayment(_)
                    | PaymentMethodData::MobilePayment(_)
                    | PaymentMethodData::Upi(_)
                    | PaymentMethodData::Voucher(_)
                    | PaymentMethodData::GiftCard(_)
                    | PaymentMethodData::OpenBanking(_)
                    | PaymentMethodData::CardToken(_)
                    | PaymentMethodData::NetworkToken(_)
                    | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                        Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message("Wellsfargo"),
                        )
                        .into())
                    }
                }
            }
        }
    }
}

impl TryFrom<(&WellsfargoRouterData<&PaymentsAuthorizeRouterData>, String)>
    for WellsfargoPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, connector_mandate_id): (&WellsfargoRouterData<&PaymentsAuthorizeRouterData>, String),
    ) -> Result<Self, Self::Error> {
        let processing_information = ProcessingInformation::try_from((item, None, None))?;
        let payment_instrument = WellsfargoPaymentInstrument {
            id: connector_mandate_id.into(),
        };
        let bill_to =
            item.router_data.request.get_email().ok().and_then(|email| {
                build_bill_to(item.router_data.get_optional_billing(), email).ok()
            });
        let order_information = OrderInformationWithBill::from((item, bill_to));
        let payment_information =
            PaymentInformation::MandatePayment(Box::new(MandatePaymentInformation {
                payment_instrument,
            }));
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);
        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            merchant_defined_information,
            consumer_authentication_information: None,
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoPaymentsCaptureRequest {
    processing_information: ProcessingInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoPaymentsIncrementalAuthorizationRequest {
    processing_information: ProcessingInformation,
    order_information: OrderInformationIncrementalAuthorization,
}

impl TryFrom<&WellsfargoRouterData<&PaymentsCaptureRouterData>>
    for WellsfargoPaymentsCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WellsfargoRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);
        Ok(Self {
            processing_information: ProcessingInformation {
                capture_options: Some(CaptureOptions {
                    capture_sequence_number: 1,
                    total_capture_count: 1,
                }),
                action_list: None,
                action_token_types: None,
                authorization_options: None,
                capture: None,
                commerce_indicator: String::from("internet"),
                payment_solution: None,
            },
            order_information: OrderInformationWithBill {
                amount_details: Amount {
                    total_amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                },
                bill_to: None,
            },
            client_reference_information: ClientReferenceInformation {
                code: Some(item.router_data.connector_request_reference_id.clone()),
            },
            merchant_defined_information,
        })
    }
}

impl TryFrom<&WellsfargoRouterData<&PaymentsIncrementalAuthorizationRouterData>>
    for WellsfargoPaymentsIncrementalAuthorizationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WellsfargoRouterData<&PaymentsIncrementalAuthorizationRouterData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            processing_information: ProcessingInformation {
                action_list: None,
                action_token_types: None,
                authorization_options: Some(WellsfargoAuthorizationOptions {
                    initiator: Some(WellsfargoPaymentInitiator {
                        initiator_type: None,
                        credential_stored_on_file: None,
                        stored_credential_used: Some(true),
                    }),
                    merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                        reason: Some("5".to_owned()),
                        previous_transaction_id: None,
                        original_authorized_amount: None,
                    }),
                }),
                commerce_indicator: String::from("internet"),
                capture: None,
                capture_options: None,
                payment_solution: None,
            },
            order_information: OrderInformationIncrementalAuthorization {
                amount_details: AdditionalAmount {
                    additional_amount: item.amount.clone(),
                    currency: item.router_data.request.currency.to_string(),
                },
            },
        })
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoVoidRequest {
    client_reference_information: ClientReferenceInformation,
    reversal_information: ReversalInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
    // The connector documentation does not mention the merchantDefinedInformation field for Void requests. But this has been still added because it works!
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReversalInformation {
    amount_details: Amount,
    reason: String,
}

impl TryFrom<&WellsfargoRouterData<&PaymentsCancelRouterData>> for WellsfargoVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: &WellsfargoRouterData<&PaymentsCancelRouterData>,
    ) -> Result<Self, Self::Error> {
        let merchant_defined_information = value
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);
        Ok(Self {
            client_reference_information: ClientReferenceInformation {
                code: Some(value.router_data.connector_request_reference_id.clone()),
            },
            reversal_information: ReversalInformation {
                amount_details: Amount {
                    total_amount: value.amount.to_owned(),
                    currency: value.router_data.request.currency.ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "Currency",
                        },
                    )?,
                },
                reason: value
                    .router_data
                    .request
                    .cancellation_reason
                    .clone()
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "Cancellation Reason",
                    })?,
            },
            merchant_defined_information,
        })
    }
}

pub struct WellsfargoAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for WellsfargoAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                api_key: api_key.to_owned(),
                merchant_account: key1.to_owned(),
                api_secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WellsfargoPaymentStatus {
    Authorized,
    Succeeded,
    Failed,
    Voided,
    Reversed,
    Pending,
    Declined,
    Rejected,
    Challenge,
    AuthorizedPendingReview,
    AuthorizedRiskDeclined,
    Transmitted,
    InvalidRequest,
    ServerError,
    PendingAuthentication,
    PendingReview,
    Accepted,
    Cancelled,
    StatusNotReceived,
    //PartialAuthorized, not being consumed yet.
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WellsfargoIncrementalAuthorizationStatus {
    Authorized,
    Declined,
    AuthorizedPendingReview,
}

fn map_attempt_status(status: WellsfargoPaymentStatus, capture: bool) -> enums::AttemptStatus {
    match status {
        WellsfargoPaymentStatus::Authorized | WellsfargoPaymentStatus::AuthorizedPendingReview => {
            if capture {
                // Because Wellsfargo will return Payment Status as Authorized even in AutoCapture Payment
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        WellsfargoPaymentStatus::Pending => {
            if capture {
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Pending
            }
        }
        WellsfargoPaymentStatus::Succeeded | WellsfargoPaymentStatus::Transmitted => {
            enums::AttemptStatus::Charged
        }
        WellsfargoPaymentStatus::Voided
        | WellsfargoPaymentStatus::Reversed
        | WellsfargoPaymentStatus::Cancelled => enums::AttemptStatus::Voided,
        WellsfargoPaymentStatus::Failed
        | WellsfargoPaymentStatus::Declined
        | WellsfargoPaymentStatus::AuthorizedRiskDeclined
        | WellsfargoPaymentStatus::Rejected
        | WellsfargoPaymentStatus::InvalidRequest
        | WellsfargoPaymentStatus::ServerError => enums::AttemptStatus::Failure,
        WellsfargoPaymentStatus::PendingAuthentication => {
            enums::AttemptStatus::AuthenticationPending
        }
        WellsfargoPaymentStatus::PendingReview
        | WellsfargoPaymentStatus::StatusNotReceived
        | WellsfargoPaymentStatus::Challenge
        | WellsfargoPaymentStatus::Accepted => enums::AttemptStatus::Pending,
    }
}

impl From<WellsfargoIncrementalAuthorizationStatus> for common_enums::AuthorizationStatus {
    fn from(item: WellsfargoIncrementalAuthorizationStatus) -> Self {
        match item {
            WellsfargoIncrementalAuthorizationStatus::Authorized
            | WellsfargoIncrementalAuthorizationStatus::AuthorizedPendingReview => Self::Success,
            WellsfargoIncrementalAuthorizationStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoPaymentsResponse {
    id: String,
    status: Option<WellsfargoPaymentStatus>,
    client_reference_information: Option<ClientReferenceInformation>,
    processor_information: Option<ClientProcessorInformation>,
    risk_information: Option<ClientRiskInformation>,
    token_information: Option<WellsfargoTokenInformation>,
    error_information: Option<WellsfargoErrorInformation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoErrorInformationResponse {
    id: String,
    error_information: WellsfargoErrorInformation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoPaymentsIncrementalAuthorizationResponse {
    status: WellsfargoIncrementalAuthorizationStatus,
    error_information: Option<WellsfargoErrorInformation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientReferenceInformation {
    code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientProcessorInformation {
    network_transaction_id: Option<String>,
    avs: Option<Avs>,
    card_verification: Option<CardVerification>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CardVerification {
    result_code: Option<String>,
    result_code_raw: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Avs {
    code: Option<String>,
    code_raw: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientRiskInformation {
    rules: Option<Vec<ClientRiskInformationRules>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ClientRiskInformationRules {
    name: Option<Secret<String>>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoTokenInformation {
    payment_instrument: Option<WellsfargoPaymentInstrument>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct WellsfargoErrorInformation {
    reason: Option<String>,
    message: Option<String>,
    details: Option<Vec<Details>>,
}

fn get_error_response_if_failure(
    (info_response, status, http_code): (&WellsfargoPaymentsResponse, enums::AttemptStatus, u16),
) -> Option<ErrorResponse> {
    if utils::is_payment_failure(status) {
        Some(get_error_response(
            &info_response.error_information,
            &info_response.risk_information,
            Some(status),
            http_code,
            info_response.id.clone(),
        ))
    } else {
        None
    }
}

fn get_payment_response(
    (info_response, status, http_code): (&WellsfargoPaymentsResponse, enums::AttemptStatus, u16),
) -> Result<PaymentsResponseData, ErrorResponse> {
    let error_response = get_error_response_if_failure((info_response, status, http_code));
    match error_response {
        Some(error) => Err(error),
        None => {
            let incremental_authorization_allowed =
                Some(status == enums::AttemptStatus::Authorized);
            let mandate_reference =
                info_response
                    .token_information
                    .clone()
                    .map(|token_info| MandateReference {
                        connector_mandate_id: token_info
                            .payment_instrument
                            .map(|payment_instrument| payment_instrument.id.expose()),
                        payment_method_id: None,
                        mandate_metadata: None,
                        connector_mandate_request_reference_id: None,
                    });

            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(info_response.id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(mandate_reference),
                connector_metadata: None,
                network_txn_id: info_response.processor_information.as_ref().and_then(
                    |processor_information| processor_information.network_transaction_id.clone(),
                ),
                connector_response_reference_id: Some(
                    info_response
                        .client_reference_information
                        .clone()
                        .and_then(|client_reference_information| client_reference_information.code)
                        .unwrap_or(info_response.id.clone()),
                ),
                incremental_authorization_allowed,
                charges: None,
            })
        }
    }
}

impl
    TryFrom<
        ResponseRouterData<
            Authorize,
            WellsfargoPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Authorize,
            WellsfargoPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = map_attempt_status(
            item.response
                .status
                .clone()
                .unwrap_or(WellsfargoPaymentStatus::StatusNotReceived),
            item.data.request.is_auto_capture()?,
        );
        let response = get_payment_response((&item.response, status, item.http_code));
        let connector_response = item
            .response
            .processor_information
            .as_ref()
            .map(AdditionalPaymentMethodConnectorResponse::from)
            .map(ConnectorResponseData::with_additional_payment_method_data);

        Ok(Self {
            status,
            response,
            connector_response,
            ..item.data
        })
    }
}

impl From<&ClientProcessorInformation> for AdditionalPaymentMethodConnectorResponse {
    fn from(processor_information: &ClientProcessorInformation) -> Self {
        let payment_checks = Some(
            serde_json::json!({"avs_response": processor_information.avs, "card_verification": processor_information.card_verification}),
        );

        Self::Card {
            authentication_data: None,
            payment_checks,
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            WellsfargoPaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            WellsfargoPaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = map_attempt_status(
            item.response
                .status
                .clone()
                .unwrap_or(WellsfargoPaymentStatus::StatusNotReceived),
            true,
        );
        let response = get_payment_response((&item.response, status, item.http_code));
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<F, WellsfargoPaymentsResponse, PaymentsCancelData, PaymentsResponseData>,
    > for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            WellsfargoPaymentsResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = map_attempt_status(
            item.response
                .status
                .clone()
                .unwrap_or(WellsfargoPaymentStatus::StatusNotReceived),
            false,
        );
        let response = get_payment_response((&item.response, status, item.http_code));
        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

// zero dollar response
impl
    TryFrom<
        ResponseRouterData<
            SetupMandate,
            WellsfargoPaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            SetupMandate,
            WellsfargoPaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let mandate_reference =
            item.response
                .token_information
                .clone()
                .map(|token_info| MandateReference {
                    connector_mandate_id: token_info
                        .payment_instrument
                        .map(|payment_instrument| payment_instrument.id.expose()),
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: None,
                });
        let mut mandate_status = map_attempt_status(
            item.response
                .status
                .clone()
                .unwrap_or(WellsfargoPaymentStatus::StatusNotReceived),
            false,
        );
        if matches!(mandate_status, enums::AttemptStatus::Authorized) {
            //In case of zero auth mandates we want to make the payment reach the terminal status so we are converting the authorized status to charged as well.
            mandate_status = enums::AttemptStatus::Charged
        }
        let error_response =
            get_error_response_if_failure((&item.response, mandate_status, item.http_code));

        let connector_response = item
            .response
            .processor_information
            .as_ref()
            .map(AdditionalPaymentMethodConnectorResponse::from)
            .map(ConnectorResponseData::with_additional_payment_method_data);

        Ok(Self {
            status: mandate_status,
            response: match error_response {
                Some(error) => Err(error),
                None => Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(mandate_reference),
                    connector_metadata: None,
                    network_txn_id: item.response.processor_information.as_ref().and_then(
                        |processor_information| {
                            processor_information.network_transaction_id.clone()
                        },
                    ),
                    connector_response_reference_id: Some(
                        item.response
                            .client_reference_information
                            .and_then(|client_reference_information| {
                                client_reference_information.code.clone()
                            })
                            .unwrap_or(item.response.id),
                    ),
                    incremental_authorization_allowed: Some(
                        mandate_status == enums::AttemptStatus::Authorized,
                    ),
                    charges: None,
                }),
            },
            connector_response,
            ..item.data
        })
    }
}

impl<F, T>
    TryFrom<
        ResponseRouterData<
            F,
            WellsfargoPaymentsIncrementalAuthorizationResponse,
            T,
            PaymentsResponseData,
        >,
    > for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            WellsfargoPaymentsIncrementalAuthorizationResponse,
            T,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: match item.response.error_information {
                Some(error) => Ok(PaymentsResponseData::IncrementalAuthorizationResponse {
                    status: common_enums::AuthorizationStatus::Failure,
                    error_code: error.reason,
                    error_message: error.message,
                    connector_authorization_id: None,
                }),
                _ => Ok(PaymentsResponseData::IncrementalAuthorizationResponse {
                    status: item.response.status.into(),
                    error_code: None,
                    error_message: None,
                    connector_authorization_id: None,
                }),
            },
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoTransactionResponse {
    id: String,
    application_information: ApplicationInformation,
    client_reference_information: Option<ClientReferenceInformation>,
    error_information: Option<WellsfargoErrorInformation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInformation {
    status: Option<WellsfargoPaymentStatus>,
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            WellsfargoTransactionResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            WellsfargoTransactionResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.application_information.status {
            Some(status) => {
                let status = map_attempt_status(status, item.data.request.is_auto_capture()?);
                let incremental_authorization_allowed =
                    Some(status == enums::AttemptStatus::Authorized);
                let risk_info: Option<ClientRiskInformation> = None;
                if utils::is_payment_failure(status) {
                    Ok(Self {
                        response: Err(get_error_response(
                            &item.response.error_information,
                            &risk_info,
                            Some(status),
                            item.http_code,
                            item.response.id.clone(),
                        )),
                        status: enums::AttemptStatus::Failure,
                        ..item.data
                    })
                } else {
                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::ConnectorTransactionId(
                                item.response.id.clone(),
                            ),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: item
                                .response
                                .client_reference_information
                                .map(|cref| cref.code)
                                .unwrap_or(Some(item.response.id)),
                            incremental_authorization_allowed,
                            charges: None,
                        }),
                        ..item.data
                    })
                }
            }
            None => Ok(Self {
                status: item.data.status,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(item.response.id.clone()),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(item.response.id),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoRefundRequest {
    order_information: OrderInformation,
    client_reference_information: ClientReferenceInformation,
}

impl<F> TryFrom<&WellsfargoRouterData<&RefundsRouterData<F>>> for WellsfargoRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &WellsfargoRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            order_information: OrderInformation {
                amount_details: Amount {
                    total_amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                },
            },
            client_reference_information: ClientReferenceInformation {
                code: Some(item.router_data.request.refund_id.clone()),
            },
        })
    }
}

impl From<WellsfargoRefundStatus> for enums::RefundStatus {
    fn from(item: WellsfargoRefundStatus) -> Self {
        match item {
            WellsfargoRefundStatus::Succeeded | WellsfargoRefundStatus::Transmitted => {
                Self::Success
            }
            WellsfargoRefundStatus::Cancelled
            | WellsfargoRefundStatus::Failed
            | WellsfargoRefundStatus::Voided => Self::Failure,
            WellsfargoRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WellsfargoRefundStatus {
    Succeeded,
    Transmitted,
    Failed,
    Pending,
    Voided,
    Cancelled,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoRefundResponse {
    id: String,
    status: WellsfargoRefundStatus,
    error_information: Option<WellsfargoErrorInformation>,
}

impl TryFrom<RefundsResponseRouterData<Execute, WellsfargoRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, WellsfargoRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status.clone());
        let response = if utils::is_refund_failure(refund_status) {
            Err(get_error_response(
                &item.response.error_information,
                &None,
                None,
                item.http_code,
                item.response.id.clone(),
            ))
        } else {
            Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status: enums::RefundStatus::from(item.response.status),
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RsyncApplicationInformation {
    status: Option<WellsfargoRefundStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoRsyncResponse {
    id: String,
    application_information: Option<RsyncApplicationInformation>,
    error_information: Option<WellsfargoErrorInformation>,
}

impl TryFrom<RefundsResponseRouterData<RSync, WellsfargoRsyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, WellsfargoRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item
            .response
            .application_information
            .and_then(|application_information| application_information.status)
        {
            Some(status) => {
                let refund_status = enums::RefundStatus::from(status.clone());
                if utils::is_refund_failure(refund_status) {
                    if status == WellsfargoRefundStatus::Voided {
                        Err(get_error_response(
                            &Some(WellsfargoErrorInformation {
                                message: Some(constants::REFUND_VOIDED.to_string()),
                                reason: Some(constants::REFUND_VOIDED.to_string()),
                                details: None,
                            }),
                            &None,
                            None,
                            item.http_code,
                            item.response.id.clone(),
                        ))
                    } else {
                        Err(get_error_response(
                            &item.response.error_information,
                            &None,
                            None,
                            item.http_code,
                            item.response.id.clone(),
                        ))
                    }
                } else {
                    Ok(RefundsResponseData {
                        connector_refund_id: item.response.id,
                        refund_status,
                    })
                }
            }

            None => Ok(RefundsResponseData {
                connector_refund_id: item.response.id.clone(),
                refund_status: match item.data.response {
                    Ok(response) => response.refund_status,
                    Err(_) => common_enums::RefundStatus::Pending,
                },
            }),
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoStandardErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoNotAvailableErrorResponse {
    pub errors: Vec<WellsfargoNotAvailableErrorObject>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoNotAvailableErrorObject {
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WellsfargoServerErrorResponse {
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<Reason>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Reason {
    SystemError,
    ServerTimeout,
    ServiceTimeout,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WellsfargoAuthenticationErrorResponse {
    pub response: AuthenticationErrorInformation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum WellsfargoErrorResponse {
    AuthenticationError(Box<WellsfargoAuthenticationErrorResponse>),
    //If the request resource is not available/exists in wellsfargo
    NotAvailableError(Box<WellsfargoNotAvailableErrorResponse>),
    StandardError(Box<WellsfargoStandardErrorResponse>),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Details {
    pub field: String,
    pub reason: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ErrorInformation {
    pub message: String,
    pub reason: String,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct AuthenticationErrorInformation {
    pub rmsg: String,
}

pub fn get_error_response(
    error_data: &Option<WellsfargoErrorInformation>,
    risk_information: &Option<ClientRiskInformation>,
    attempt_status: Option<enums::AttemptStatus>,
    status_code: u16,
    transaction_id: String,
) -> ErrorResponse {
    let avs_message = risk_information
        .clone()
        .map(|client_risk_information| {
            client_risk_information.rules.map(|rules| {
                rules
                    .iter()
                    .map(|risk_info| {
                        risk_info.name.clone().map_or("".to_string(), |name| {
                            format!(" , {}", name.clone().expose())
                        })
                    })
                    .collect::<Vec<String>>()
                    .join("")
            })
        })
        .unwrap_or(Some("".to_string()));

    let detailed_error_info = error_data
        .clone()
        .map(|error_data| match error_data.details {
            Some(details) => details
                .iter()
                .map(|details| format!("{} : {}", details.field, details.reason))
                .collect::<Vec<_>>()
                .join(", "),
            None => "".to_string(),
        });

    let reason = get_error_reason(
        error_data.clone().and_then(|error_info| error_info.message),
        detailed_error_info,
        avs_message,
    );
    let error_message = error_data.clone().and_then(|error_info| error_info.reason);
    ErrorResponse {
        code: error_message
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
        message: error_message
            .clone()
            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
        reason,
        status_code,
        attempt_status,
        connector_transaction_id: Some(transaction_id.clone()),
    }
}
pub fn get_error_reason(
    error_info: Option<String>,
    detailed_error_info: Option<String>,
    avs_error_info: Option<String>,
) -> Option<String> {
    match (error_info, detailed_error_info, avs_error_info) {
        (Some(message), Some(details), Some(avs_message)) => Some(format!(
            "{}, detailed_error_information: {}, avs_message: {}",
            message, details, avs_message
        )),
        (Some(message), Some(details), None) => Some(format!(
            "{}, detailed_error_information: {}",
            message, details
        )),
        (Some(message), None, Some(avs_message)) => {
            Some(format!("{}, avs_message: {}", message, avs_message))
        }
        (None, Some(details), Some(avs_message)) => {
            Some(format!("{}, avs_message: {}", details, avs_message))
        }
        (Some(message), None, None) => Some(message),
        (None, Some(details), None) => Some(details),
        (None, None, Some(avs_message)) => Some(avs_message),
        (None, None, None) => None,
    }
}
