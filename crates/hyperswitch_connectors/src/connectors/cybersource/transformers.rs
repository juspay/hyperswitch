use api_models::payments;
#[cfg(feature = "payouts")]
use api_models::payouts::PayoutMethodData;
use base64::Engine;
use common_enums::{enums, FutureUsage};
use common_utils::{
    consts,
    ext_traits::{OptionExt, ValueExt},
    pii,
    types::{SemanticVersion, StringMajorUnit},
};
use error_stack::ResultExt;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    address::{AddressDetails, PhoneDetails},
    router_flow_types::PoFulfill,
    router_response_types::PayoutsResponseData,
    types::PayoutsRouterData,
};
use hyperswitch_domain_models::{
    network_tokenization::NetworkTokenNumber,
    payment_method_data::{
        ApplePayWalletData, GooglePayWalletData, NetworkTokenData, PaymentMethodData,
        SamsungPayWalletData, WalletData,
    },
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ApplePayPredecryptData, ConnectorAuthType,
        ConnectorResponseData, ErrorResponse, GooglePayDecryptedData, PaymentMethodToken,
        RouterData,
    },
    router_flow_types::{
        payments::Authorize,
        refunds::{Execute, RSync},
        SetupMandate,
    },
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsCancelData, PaymentsCaptureData,
        PaymentsPreProcessingData, PaymentsSyncData, ResponseId, SetupMandateRequestData,
    },
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsIncrementalAuthorizationRouterData,
        PaymentsPreProcessingRouterData, RefundsRouterData, SetupMandateRouterData,
    },
};
use hyperswitch_interfaces::{api, errors};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utils::ForeignTryFrom;

#[cfg(feature = "payouts")]
use crate::types::PayoutsResponseRouterData;
#[cfg(feature = "payouts")]
use crate::utils::PayoutsData;
use crate::{
    constants,
    types::{RefundsResponseRouterData, ResponseRouterData},
    unimplemented_payment_method,
    utils::{
        self, AddressDetailsData, ApplePayDecrypt, CardData, CardIssuer, NetworkTokenData as _,
        PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        PaymentsPreProcessingRequestData, PaymentsSetupMandateRequestData, PaymentsSyncRequestData,
        RecurringMandateData, RouterData as OtherRouterData,
    },
};

#[derive(Debug, Serialize)]
pub struct CybersourceRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for CybersourceRouterData<T> {
    fn from((amount, router_data): (StringMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data,
        }
    }
}

impl From<CardIssuer> for String {
    fn from(card_issuer: CardIssuer) -> Self {
        let card_type = match card_issuer {
            CardIssuer::AmericanExpress => "003",
            CardIssuer::Master => "002",
            //"042" is the type code for Masetro Cards(International). For Maestro Cards(UK-Domestic) the mapping should be "024"
            CardIssuer::Maestro => "042",
            CardIssuer::Visa => "001",
            CardIssuer::Discover => "004",
            CardIssuer::DinersClub => "005",
            CardIssuer::CarteBlanche => "006",
            CardIssuer::JCB => "007",
        };
        card_type.to_string()
    }
}
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CybersourceConnectorMetadataObject {
    pub disable_avs: Option<bool>,
    pub disable_cvn: Option<bool>,
}

impl TryFrom<&Option<pii::SecretSerdeValue>> for CybersourceConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: &Option<pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata = utils::to_connector_meta_from_secret::<Self>(meta_data.clone())
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;
        Ok(metadata)
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceZeroMandateRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
}

impl TryFrom<&SetupMandateRouterData> for CybersourceZeroMandateRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &SetupMandateRouterData) -> Result<Self, Self::Error> {
        let email = item.get_billing_email().or(item.request.get_email())?;
        let bill_to = build_bill_to(item.get_optional_billing(), email)?;

        let order_information = OrderInformationWithBill {
            amount_details: Amount {
                total_amount: StringMajorUnit::zero(),
                currency: item.request.currency,
            },
            bill_to: Some(bill_to),
        };
        let connector_merchant_config =
            CybersourceConnectorMetadataObject::try_from(&item.connector_meta_data)?;

        let (action_list, action_token_types, authorization_options) = (
            Some(vec![CybersourceActionsList::TokenCreate]),
            Some(vec![
                CybersourceActionsTokenType::PaymentInstrument,
                CybersourceActionsTokenType::Customer,
            ]),
            Some(CybersourceAuthorizationOptions {
                initiator: Some(CybersourcePaymentInitiator {
                    initiator_type: Some(CybersourcePaymentInitiatorTypes::Customer),
                    credential_stored_on_file: Some(true),
                    stored_credential_used: None,
                }),
                merchant_intitiated_transaction: None,
                ignore_avs_result: connector_merchant_config.disable_avs,
                ignore_cv_result: connector_merchant_config.disable_cvn,
            }),
        );

        let client_reference_information = ClientReferenceInformation {
            code: Some(item.connector_request_reference_id.clone()),
        };

        let (payment_information, solution) = match item.request.payment_method_data.clone() {
            PaymentMethodData::Card(ccard) => {
                let card_type = match ccard
                    .card_network
                    .clone()
                    .and_then(get_cybersource_card_type)
                {
                    Some(card_network) => Some(card_network.to_string()),
                    None => ccard.get_card_issuer().ok().map(String::from),
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
                                            cryptogram: Some(
                                                decrypt_data.payment_data.online_payment_cryptogram,
                                            ),
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
                            "Cybersource"
                        ))?,
                        PaymentMethodToken::PazeDecrypt(_) => {
                            Err(unimplemented_payment_method!("Paze", "Cybersource"))?
                        }
                        PaymentMethodToken::GooglePayDecrypt(_) => {
                            Err(unimplemented_payment_method!("Google Pay", "Cybersource"))?
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
                    PaymentInformation::GooglePayToken(Box::new(
                        GooglePayTokenPaymentInformation {
                            fluid_data: FluidData {
                                value: Secret::from(
                                    consts::BASE64_ENGINE
                                        .encode(google_pay_data.tokenization_data.token),
                                ),
                                descriptor: None,
                            },
                        },
                    )),
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
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
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
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
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
pub struct CybersourcePaymentsRequest {
    processing_information: ProcessingInformation,
    payment_information: PaymentInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    consumer_authentication_information: Option<CybersourceConsumerAuthInformation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessingInformation {
    action_list: Option<Vec<CybersourceActionsList>>,
    action_token_types: Option<Vec<CybersourceActionsTokenType>>,
    authorization_options: Option<CybersourceAuthorizationOptions>,
    commerce_indicator: String,
    capture: Option<bool>,
    capture_options: Option<CaptureOptions>,
    payment_solution: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformation {
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
pub enum CybersourceActionsList {
    TokenCreate,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CybersourceActionsTokenType {
    Customer,
    PaymentInstrument,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceAuthorizationOptions {
    initiator: Option<CybersourcePaymentInitiator>,
    merchant_intitiated_transaction: Option<MerchantInitiatedTransaction>,
    ignore_avs_result: Option<bool>,
    ignore_cv_result: Option<bool>,
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
pub struct CybersourcePaymentInitiator {
    #[serde(rename = "type")]
    initiator_type: Option<CybersourcePaymentInitiatorTypes>,
    credential_stored_on_file: Option<bool>,
    stored_credential_used: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CybersourcePaymentInitiatorTypes {
    Customer,
    Merchant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureOptions {
    capture_sequence_number: u32,
    total_capture_count: u32,
    is_final: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkTokenizedCard {
    number: NetworkTokenNumber,
    expiration_month: Secret<String>,
    expiration_year: Secret<String>,
    cryptogram: Option<Secret<String>>,
    transaction_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkTokenPaymentInformation {
    tokenized_card: NetworkTokenizedCard,
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
    cryptogram: Option<Secret<String>>,
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
    payment_instrument: CybersoucrePaymentInstrument,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FluidData {
    value: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    descriptor: Option<String>,
}

pub const FLUID_DATA_DESCRIPTOR: &str = "RklEPUNPTU1PTi5BUFBMRS5JTkFQUC5QQVlNRU5U";

pub const FLUID_DATA_DESCRIPTOR_FOR_SAMSUNG_PAY: &str = "FID=COMMON.SAMSUNG.INAPP.PAYMENT";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayTokenPaymentInformation {
    fluid_data: FluidData,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GooglePayPaymentInformation {
    tokenized_card: TokenizedCard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SamsungPayTokenizedCard {
    transaction_type: TransactionType,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SamsungPayPaymentInformation {
    fluid_data: FluidData,
    tokenized_card: SamsungPayTokenizedCard,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SamsungPayFluidDataValue {
    public_key_hash: Secret<String>,
    version: String,
    data: Secret<String>,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum PaymentInformation {
    Cards(Box<CardPaymentInformation>),
    GooglePayToken(Box<GooglePayTokenPaymentInformation>),
    GooglePay(Box<GooglePayPaymentInformation>),
    ApplePay(Box<ApplePayPaymentInformation>),
    ApplePayToken(Box<ApplePayTokenPaymentInformation>),
    MandatePayment(Box<MandatePaymentInformation>),
    SamsungPay(Box<SamsungPayPaymentInformation>),
    NetworkToken(Box<NetworkTokenPaymentInformation>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CybersoucrePaymentInstrument {
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
    SamsungPay,
}

#[derive(Debug, Serialize)]
pub enum TransactionType {
    #[serde(rename = "1")]
    ApplePay,
    #[serde(rename = "1")]
    SamsungPay,
    #[serde(rename = "1")]
    GooglePay,
}

impl From<PaymentSolution> for String {
    fn from(solution: PaymentSolution) -> Self {
        let payment_solution = match solution {
            PaymentSolution::ApplePay => "001",
            PaymentSolution::GooglePay => "012",
            PaymentSolution::SamsungPay => "008",
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
}

impl From<&CybersourceRouterData<&PaymentsAuthorizeRouterData>> for ClientReferenceInformation {
    fn from(item: &CybersourceRouterData<&PaymentsAuthorizeRouterData>) -> Self {
        Self {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        }
    }
}

impl From<&CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>>
    for ClientReferenceInformation
{
    fn from(item: &CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>) -> Self {
        Self {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        }
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        Option<PaymentSolution>,
        Option<String>,
    )> for ProcessingInformation
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, solution, network): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            Option<PaymentSolution>,
            Option<String>,
        ),
    ) -> Result<Self, Self::Error> {
        let mut commerce_indicator = solution
            .as_ref()
            .map(|pm_solution| match pm_solution {
                PaymentSolution::ApplePay | PaymentSolution::SamsungPay => network
                    .as_ref()
                    .map(|card_network| match card_network.to_lowercase().as_str() {
                        "amex" => "internet",
                        "discover" => "internet",
                        "mastercard" => "spa",
                        "visa" => "internet",
                        _ => "internet",
                    })
                    .unwrap_or("internet"),
                PaymentSolution::GooglePay => "internet",
            })
            .unwrap_or("internet")
            .to_string();

        let connector_merchant_config =
            CybersourceConnectorMetadataObject::try_from(&item.router_data.connector_meta_data)?;

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
                Some(vec![CybersourceActionsList::TokenCreate]),
                Some(vec![
                    CybersourceActionsTokenType::PaymentInstrument,
                    CybersourceActionsTokenType::Customer,
                ]),
                Some(CybersourceAuthorizationOptions {
                    initiator: Some(CybersourcePaymentInitiator {
                        initiator_type: Some(CybersourcePaymentInitiatorTypes::Customer),
                        credential_stored_on_file: Some(true),
                        stored_credential_used: None,
                    }),
                    ignore_avs_result: connector_merchant_config.disable_avs,
                    ignore_cv_result: connector_merchant_config.disable_cvn,
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
                        .recurring_mandate_payment_data
                        .as_ref()
                        .and_then(|recurring_mandate_payment_data| {
                            recurring_mandate_payment_data.original_payment_authorized_amount
                        });

                    let original_currency = item
                        .router_data
                        .recurring_mandate_payment_data
                        .as_ref()
                        .and_then(|recurring_mandate_payment_data| {
                            recurring_mandate_payment_data.original_payment_authorized_currency
                        });

                    let original_authorized_amount = match original_amount.zip(original_currency) {
                        Some((original_amount, original_currency)) => {
                            Some(utils::get_amount_as_string(
                                &api::CurrencyUnit::Base,
                                original_amount,
                                original_currency,
                            )?)
                        }
                        None => None,
                    };
                    (
                        None,
                        None,
                        Some(CybersourceAuthorizationOptions {
                            initiator: None,
                            merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                                reason: None,
                                original_authorized_amount,
                                previous_transaction_id: None,
                            }),
                            ignore_avs_result: connector_merchant_config.disable_avs,
                            ignore_cv_result: connector_merchant_config.disable_cvn,
                        }),
                    )
                }
                Some(payments::MandateReferenceId::NetworkMandateId(network_transaction_id)) => {
                    let (original_amount, original_currency) = match network
                        .clone()
                        .map(|network| network.to_lowercase())
                        .as_deref()
                    {
                        //This is to make original_authorized_amount mandatory for discover card networks in NetworkMandateId flow
                        Some("004") => {
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
                    let original_authorized_amount = match original_amount.zip(original_currency) {
                        Some((original_amount, original_currency)) => Some(
                            utils::to_currency_base_unit(original_amount, original_currency)?,
                        ),
                        None => None,
                    };
                    commerce_indicator = "recurring".to_string();
                    (
                        None,
                        None,
                        Some(CybersourceAuthorizationOptions {
                            initiator: Some(CybersourcePaymentInitiator {
                                initiator_type: Some(CybersourcePaymentInitiatorTypes::Merchant),
                                credential_stored_on_file: None,
                                stored_credential_used: Some(true),
                            }),
                            merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                                reason: Some("7".to_string()),
                                original_authorized_amount,
                                previous_transaction_id: Some(Secret::new(network_transaction_id)),
                            }),
                            ignore_avs_result: connector_merchant_config.disable_avs,
                            ignore_cv_result: connector_merchant_config.disable_cvn,
                        }),
                    )
                }
                Some(payments::MandateReferenceId::NetworkTokenWithNTI(mandate_data)) => {
                    let (original_amount, original_currency) = match network
                        .clone()
                        .map(|network| network.to_lowercase())
                        .as_deref()
                    {
                        //This is to make original_authorized_amount mandatory for discover card networks in NetworkMandateId flow
                        Some("004") => {
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
                    let original_authorized_amount = match original_amount.zip(original_currency) {
                        Some((original_amount, original_currency)) => Some(
                            utils::to_currency_base_unit(original_amount, original_currency)?,
                        ),
                        None => None,
                    };
                    commerce_indicator = "recurring".to_string(); //
                    (
                        None,
                        None,
                        Some(CybersourceAuthorizationOptions {
                            initiator: Some(CybersourcePaymentInitiator {
                                initiator_type: Some(CybersourcePaymentInitiatorTypes::Merchant),
                                credential_stored_on_file: None,
                                stored_credential_used: Some(true),
                            }),
                            merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                                reason: Some("7".to_string()), // 7 is for MIT using NTI
                                original_authorized_amount,
                                previous_transaction_id: Some(Secret::new(
                                    mandate_data.network_transaction_id,
                                )),
                            }),
                            ignore_avs_result: connector_merchant_config.disable_avs,
                            ignore_cv_result: connector_merchant_config.disable_cvn,
                        }),
                    )
                }
                None => (None, None, None),
            }
        } else {
            (
                None,
                None,
                Some(CybersourceAuthorizationOptions {
                    initiator: None,
                    merchant_intitiated_transaction: None,
                    ignore_avs_result: connector_merchant_config.disable_avs,
                    ignore_cv_result: connector_merchant_config.disable_cvn,
                }),
            )
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
                Some(enums::CaptureMethod::Automatic) | None
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
    TryFrom<(
        &CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>,
        Option<PaymentSolution>,
        &CybersourceConsumerAuthValidateResponse,
    )> for ProcessingInformation
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, solution, three_ds_data): (
            &CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>,
            Option<PaymentSolution>,
            &CybersourceConsumerAuthValidateResponse,
        ),
    ) -> Result<Self, Self::Error> {
        let connector_merchant_config =
            CybersourceConnectorMetadataObject::try_from(&item.router_data.connector_meta_data)?;

        let (action_list, action_token_types, authorization_options) =
            if item.router_data.request.setup_future_usage == Some(FutureUsage::OffSession)
            //TODO check for customer acceptance also
            {
                (
                    Some(vec![CybersourceActionsList::TokenCreate]),
                    Some(vec![
                        CybersourceActionsTokenType::PaymentInstrument,
                        CybersourceActionsTokenType::Customer,
                    ]),
                    Some(CybersourceAuthorizationOptions {
                        initiator: Some(CybersourcePaymentInitiator {
                            initiator_type: Some(CybersourcePaymentInitiatorTypes::Customer),
                            credential_stored_on_file: Some(true),
                            stored_credential_used: None,
                        }),
                        merchant_intitiated_transaction: None,
                        ignore_avs_result: connector_merchant_config.disable_avs,
                        ignore_cv_result: connector_merchant_config.disable_cvn,
                    }),
                )
            } else {
                (None, None, None)
            };
        Ok(Self {
            capture: Some(matches!(
                item.router_data.request.capture_method,
                Some(enums::CaptureMethod::Automatic) | None
            )),
            payment_solution: solution.map(String::from),
            action_list,
            action_token_types,
            authorization_options,
            capture_options: None,
            commerce_indicator: three_ds_data
                .indicator
                .to_owned()
                .unwrap_or(String::from("internet")),
        })
    }
}

impl
    From<(
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        Option<BillTo>,
    )> for OrderInformationWithBill
{
    fn from(
        (item, bill_to): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
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

impl
    From<(
        &CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>,
        BillTo,
    )> for OrderInformationWithBill
{
    fn from(
        (item, bill_to): (
            &CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>,
            BillTo,
        ),
    ) -> Self {
        Self {
            amount_details: Amount {
                total_amount: item.amount.to_owned(),
                currency: item.router_data.request.currency,
            },
            bill_to: Some(bill_to),
        }
    }
}

// for cybersource each item in Billing is mandatory
// fn build_bill_to(
//     address_details: &payments::Address,
//     email: pii::Email,
// ) -> Result<BillTo, error_stack::Report<errors::ConnectorError>> {
//     let address = address_details
//         .address
//         .as_ref()
//         .ok_or_else(utils::missing_field_err("billing.address"))?;

//     let country = address.get_country()?.to_owned();
//     let first_name = address.get_first_name()?;

//     let (administrative_area, postal_code) =
//         if country == api_enums::CountryAlpha2::US || country == api_enums::CountryAlpha2::CA {
//             let mut state = address.to_state_code()?.peek().clone();
//             state.truncate(20);
//             (
//                 Some(Secret::from(state)),
//                 Some(address.get_zip()?.to_owned()),
//             )
//         } else {
//             let zip = address.zip.clone();
//             let mut_state = address.state.clone().map(|state| state.expose());
//             match mut_state {
//                 Some(mut state) => {
//                     state.truncate(20);
//                     (Some(Secret::from(state)), zip)
//                 }
//                 None => (None, zip),
//             }
//         };
//     Ok(BillTo {
//         first_name: first_name.clone(),
//         last_name: address.get_last_name().unwrap_or(first_name).clone(),
//         address1: address.get_line1()?.to_owned(),
//         locality: address.get_city()?.to_owned(),
//         administrative_area,
//         postal_code,
//         country,
//         email,
//     })
// }

fn build_bill_to(
    address_details: Option<&hyperswitch_domain_models::address::Address>,
    email: pii::Email,
) -> Result<BillTo, error_stack::Report<errors::ConnectorError>> {
    let default_address = BillTo {
        first_name: None,
        last_name: None,
        address1: None,
        locality: None,
        administrative_area: None,
        postal_code: None,
        country: None,
        email: email.clone(),
    };
    Ok(address_details
        .and_then(|addr| {
            addr.address.as_ref().map(|addr| BillTo {
                first_name: addr.first_name.remove_new_line(),
                last_name: addr.last_name.remove_new_line(),
                address1: addr.line1.remove_new_line(),
                locality: addr.city.remove_new_line(),
                administrative_area: addr
                    .to_state_code_as_optional()
                    .ok()
                    .flatten()
                    .remove_new_line(),
                postal_code: addr.zip.remove_new_line(),
                country: addr.country,
                email,
            })
        })
        .unwrap_or(default_address))
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
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        hyperswitch_domain_models::payment_method_data::Card,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            hyperswitch_domain_models::payment_method_data::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item
            .router_data
            .get_billing_email()
            .or(item.router_data.request.get_email())?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));

        let card_type = match ccard
            .card_network
            .clone()
            .and_then(get_cybersource_card_type)
        {
            Some(card_network) => Some(card_network.to_string()),
            None => ccard.get_card_issuer().ok().map(String::from),
        };

        let security_code = if item
            .router_data
            .request
            .get_optional_network_transaction_id()
            .is_some()
        {
            None
        } else {
            Some(ccard.card_cvc)
        };

        let payment_information = PaymentInformation::Cards(Box::new(CardPaymentInformation {
            card: Card {
                number: ccard.card_number,
                expiration_month: ccard.card_exp_month,
                expiration_year: ccard.card_exp_year,
                security_code,
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
                CybersourceConsumerAuthInformation {
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
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        hyperswitch_domain_models::payment_method_data::CardDetailsForNetworkTransactionId,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            hyperswitch_domain_models::payment_method_data::CardDetailsForNetworkTransactionId,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item
            .router_data
            .get_billing_email()
            .or(item.router_data.request.get_email())?;
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
                security_code: None,
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
                CybersourceConsumerAuthInformation {
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
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        NetworkTokenData,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, token_data): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            NetworkTokenData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));

        let card_issuer = token_data.get_card_issuer();
        let card_type = match card_issuer {
            Ok(issuer) => Some(String::from(issuer)),
            Err(_) => None,
        };

        let payment_information =
            PaymentInformation::NetworkToken(Box::new(NetworkTokenPaymentInformation {
                tokenized_card: NetworkTokenizedCard {
                    number: token_data.get_network_token(),
                    expiration_month: token_data.get_network_token_expiry_month(),
                    expiration_year: token_data.get_network_token_expiry_year(),
                    cryptogram: token_data.get_cryptogram().clone(),
                    transaction_type: "1".to_string(),
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
                    if token_data.card_network == Some(common_enums::CardNetwork::Mastercard) {
                        (Some(Secret::new(authn_data.cavv.clone())), None)
                    } else {
                        (None, Some(authn_data.cavv.clone()))
                    };
                CybersourceConsumerAuthInformation {
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
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        Box<hyperswitch_domain_models::router_data::PazeDecryptedData>,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, paze_data): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            Box<hyperswitch_domain_models::router_data::PazeDecryptedData>,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item.router_data.request.get_email()?;
        let (first_name, last_name) = match paze_data.billing_address.name {
            Some(name) => {
                let (first_name, last_name) = name
                    .peek()
                    .split_once(' ')
                    .map(|(first, last)| (first.to_string(), last.to_string()))
                    .ok_or(errors::ConnectorError::MissingRequiredField {
                        field_name: "billing_address.name",
                    })?;
                (Secret::from(first_name), Secret::from(last_name))
            }
            None => (
                item.router_data.get_billing_first_name()?,
                item.router_data.get_billing_last_name()?,
            ),
        };
        let bill_to = BillTo {
            first_name: Some(first_name),
            last_name: Some(last_name),
            address1: paze_data.billing_address.line1,
            locality: paze_data.billing_address.city.map(|city| city.expose()),
            administrative_area: Some(Secret::from(
                //Paze wallet is currently supported in US only
                common_enums::UsStatesAbbreviation::foreign_try_from(
                    paze_data
                        .billing_address
                        .state
                        .ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "billing_address.state",
                        })?
                        .peek()
                        .to_owned(),
                )?
                .to_string(),
            )),
            postal_code: paze_data.billing_address.zip,
            country: paze_data.billing_address.country_code,
            email,
        };
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));

        let payment_information =
            PaymentInformation::NetworkToken(Box::new(NetworkTokenPaymentInformation {
                tokenized_card: NetworkTokenizedCard {
                    number: paze_data.token.payment_token,
                    expiration_month: paze_data.token.token_expiration_month,
                    expiration_year: paze_data.token.token_expiration_year,
                    cryptogram: Some(paze_data.token.payment_account_reference),
                    transaction_type: "1".to_string(),
                },
            }));

        let processing_information = ProcessingInformation::try_from((item, None, None))?;
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

impl
    TryFrom<(
        &CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>,
        hyperswitch_domain_models::payment_method_data::Card,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, ccard): (
            &CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>,
            hyperswitch_domain_models::payment_method_data::Card,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item
            .router_data
            .get_billing_email()
            .or(item.router_data.request.get_email())?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, bill_to));

        let card_type = match ccard
            .card_network
            .clone()
            .and_then(get_cybersource_card_type)
        {
            Some(card_network) => Some(card_network.to_string()),
            None => ccard.get_card_issuer().ok().map(String::from),
        };

        let payment_information = PaymentInformation::Cards(Box::new(CardPaymentInformation {
            card: Card {
                number: ccard.card_number,
                expiration_month: ccard.card_exp_month,
                expiration_year: ccard.card_exp_year,
                security_code: Some(ccard.card_cvc),
                card_type,
            },
        }));
        let client_reference_information = ClientReferenceInformation::from(item);

        let three_ds_info: CybersourceThreeDSMetadata = item
            .router_data
            .request
            .connector_meta
            .clone()
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_meta",
            })?
            .parse_value("CybersourceThreeDSMetadata")
            .change_context(errors::ConnectorError::InvalidConnectorConfig {
                config: "metadata",
            })?;

        let processing_information =
            ProcessingInformation::try_from((item, None, &three_ds_info.three_ds_data))?;

        let consumer_authentication_information = Some(CybersourceConsumerAuthInformation {
            ucaf_collection_indicator: three_ds_info.three_ds_data.ucaf_collection_indicator,
            cavv: three_ds_info.three_ds_data.cavv,
            ucaf_authentication_data: three_ds_info.three_ds_data.ucaf_authentication_data,
            xid: three_ds_info.three_ds_data.xid,
            directory_server_transaction_id: three_ds_info
                .three_ds_data
                .directory_server_transaction_id,
            specification_version: three_ds_info.three_ds_data.specification_version,
            pa_specification_version: None,
            veres_enrolled: None,
        });

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
            consumer_authentication_information,
            merchant_defined_information,
        })
    }
}

impl
    TryFrom<(
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        Box<ApplePayPredecryptData>,
        ApplePayWalletData,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, apple_pay_data, apple_pay_wallet_data): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            Box<ApplePayPredecryptData>,
            ApplePayWalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item
            .router_data
            .get_billing_email()
            .or(item.router_data.request.get_email())?;
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
                    cryptogram: Some(apple_pay_data.payment_data.online_payment_cryptogram),
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
            consumer_authentication_information: Some(CybersourceConsumerAuthInformation {
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
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        GooglePayWalletData,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, google_pay_data): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            GooglePayWalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item
            .router_data
            .get_billing_email()
            .or(item.router_data.request.get_email())?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));

        let payment_information =
            PaymentInformation::GooglePayToken(Box::new(GooglePayTokenPaymentInformation {
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

impl
    TryFrom<(
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        Box<GooglePayDecryptedData>,
        GooglePayWalletData,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, google_pay_decrypted_data, google_pay_data): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            Box<GooglePayDecryptedData>,
            GooglePayWalletData,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item
            .router_data
            .get_billing_email()
            .or(item.router_data.request.get_email())?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));

        let payment_information =
            PaymentInformation::GooglePay(Box::new(GooglePayPaymentInformation {
                tokenized_card: TokenizedCard {
                    number: Secret::new(
                        google_pay_decrypted_data
                            .payment_method_details
                            .pan
                            .get_card_no(),
                    ),
                    cryptogram: google_pay_decrypted_data.payment_method_details.cryptogram,
                    transaction_type: TransactionType::GooglePay,
                    expiration_year: Secret::new(
                        google_pay_decrypted_data
                            .payment_method_details
                            .expiration_year
                            .four_digits(),
                    ),
                    expiration_month: Secret::new(
                        google_pay_decrypted_data
                            .payment_method_details
                            .expiration_month
                            .two_digits(),
                    ),
                },
            }));
        let processing_information = ProcessingInformation::try_from((
            item,
            Some(PaymentSolution::GooglePay),
            Some(google_pay_data.info.card_network.clone()),
        ))?;
        let client_reference_information = ClientReferenceInformation::from(item);
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);

        let ucaf_collection_indicator =
            match google_pay_data.info.card_network.to_lowercase().as_str() {
                "mastercard" => Some("2".to_string()),
                _ => None,
            };

        Ok(Self {
            processing_information,
            payment_information,
            order_information,
            client_reference_information,
            consumer_authentication_information: Some(CybersourceConsumerAuthInformation {
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
        &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
        Box<SamsungPayWalletData>,
    )> for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, samsung_pay_data): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            Box<SamsungPayWalletData>,
        ),
    ) -> Result<Self, Self::Error> {
        let email = item
            .router_data
            .get_billing_email()
            .or(item.router_data.request.get_email())?;
        let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
        let order_information = OrderInformationWithBill::from((item, Some(bill_to)));

        let samsung_pay_fluid_data_value =
            get_samsung_pay_fluid_data_value(&samsung_pay_data.payment_credential.token_data)?;

        let samsung_pay_fluid_data_str = serde_json::to_string(&samsung_pay_fluid_data_value)
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to serialize samsung pay fluid data")?;

        let payment_information =
            PaymentInformation::SamsungPay(Box::new(SamsungPayPaymentInformation {
                fluid_data: FluidData {
                    value: Secret::new(consts::BASE64_ENGINE.encode(samsung_pay_fluid_data_str)),
                    descriptor: Some(
                        consts::BASE64_ENGINE.encode(FLUID_DATA_DESCRIPTOR_FOR_SAMSUNG_PAY),
                    ),
                },
                tokenized_card: SamsungPayTokenizedCard {
                    transaction_type: TransactionType::SamsungPay,
                },
            }));

        let processing_information = ProcessingInformation::try_from((
            item,
            Some(PaymentSolution::SamsungPay),
            Some(samsung_pay_data.payment_credential.card_brand.to_string()),
        ))?;
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

fn get_samsung_pay_fluid_data_value(
    samsung_pay_token_data: &hyperswitch_domain_models::payment_method_data::SamsungPayTokenData,
) -> Result<SamsungPayFluidDataValue, error_stack::Report<errors::ConnectorError>> {
    let samsung_pay_header =
        josekit::jwt::decode_header(samsung_pay_token_data.data.clone().peek())
            .change_context(errors::ConnectorError::RequestEncodingFailed)
            .attach_printable("Failed to decode samsung pay header")?;

    let samsung_pay_kid_optional = samsung_pay_header.claim("kid").and_then(|kid| kid.as_str());

    let samsung_pay_fluid_data_value = SamsungPayFluidDataValue {
        public_key_hash: Secret::new(
            samsung_pay_kid_optional
                .get_required_value("samsung pay public_key_hash")
                .change_context(errors::ConnectorError::RequestEncodingFailed)?
                .to_string(),
        ),
        version: samsung_pay_token_data.version.clone(),
        data: Secret::new(consts::BASE64_ENGINE.encode(samsung_pay_token_data.data.peek())),
    };
    Ok(samsung_pay_fluid_data_value)
}

impl TryFrom<&CybersourceRouterData<&PaymentsAuthorizeRouterData>> for CybersourcePaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
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
                                            "Cybersource"
                                        ))?
                                    }
                                    PaymentMethodToken::PazeDecrypt(_) => {
                                        Err(unimplemented_payment_method!("Paze", "Cybersource"))?
                                    }
                                    PaymentMethodToken::GooglePayDecrypt(_) => Err(
                                        unimplemented_payment_method!("Google Pay", "Cybersource"),
                                    )?,
                                },
                                None => {
                                    let email = item
                                        .router_data
                                        .get_billing_email()
                                        .or(item.router_data.request.get_email())?;
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
                                            CybersourceConsumerAuthInformation {
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
                            match item.router_data.payment_method_token.clone() {
                                Some(payment_method_token) => match payment_method_token {
                                    PaymentMethodToken::GooglePayDecrypt(decrypt_data) => {
                                        Self::try_from((item, decrypt_data, google_pay_data))
                                    }
                                    PaymentMethodToken::Token(_) => {
                                        Err(unimplemented_payment_method!(
                                            "Apple Pay",
                                            "Manual",
                                            "Cybersource"
                                        ))?
                                    }
                                    PaymentMethodToken::PazeDecrypt(_) => {
                                        Err(unimplemented_payment_method!("Paze", "Cybersource"))?
                                    }
                                    PaymentMethodToken::ApplePayDecrypt(_) => {
                                        Err(unimplemented_payment_method!(
                                            "Apple Pay",
                                            "Simplified",
                                            "Cybersource"
                                        ))?
                                    }
                                },
                                None => Self::try_from((item, google_pay_data)),
                            }
                        }
                        WalletData::SamsungPay(samsung_pay_data) => {
                            Self::try_from((item, samsung_pay_data))
                        }
                        WalletData::Paze(_) => {
                            match item.router_data.payment_method_token.clone() {
                                Some(PaymentMethodToken::PazeDecrypt(paze_decrypted_data)) => {
                                    Self::try_from((item, paze_decrypted_data))
                                }
                                _ => Err(errors::ConnectorError::NotImplemented(
                                    utils::get_unimplemented_payment_method_error_message(
                                        "Cybersource",
                                    ),
                                )
                                .into()),
                            }
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
                        | WalletData::TwintRedirect {}
                        | WalletData::VippsRedirect {}
                        | WalletData::TouchNGoRedirect(_)
                        | WalletData::WeChatPayRedirect(_)
                        | WalletData::WeChatPayQr(_)
                        | WalletData::CashappQr(_)
                        | WalletData::SwishQr(_)
                        | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message("Cybersource"),
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
                    PaymentMethodData::NetworkToken(token_data) => {
                        Self::try_from((item, token_data))
                    }
                    PaymentMethodData::CardDetailsForNetworkTransactionId(card) => {
                        Self::try_from((item, card))
                    }
                    PaymentMethodData::CardRedirect(_)
                    | PaymentMethodData::PayLater(_)
                    | PaymentMethodData::BankRedirect(_)
                    | PaymentMethodData::BankDebit(_)
                    | PaymentMethodData::BankTransfer(_)
                    | PaymentMethodData::Crypto(_)
                    | PaymentMethodData::Reward
                    | PaymentMethodData::RealTimePayment(_)
                    | PaymentMethodData::MobilePayment(_)
                    | PaymentMethodData::Upi(_)
                    | PaymentMethodData::Voucher(_)
                    | PaymentMethodData::GiftCard(_)
                    | PaymentMethodData::OpenBanking(_)
                    | PaymentMethodData::CardToken(_) => {
                        Err(errors::ConnectorError::NotImplemented(
                            utils::get_unimplemented_payment_method_error_message("Cybersource"),
                        )
                        .into())
                    }
                }
            }
        }
    }
}

impl TryFrom<(&CybersourceRouterData<&PaymentsAuthorizeRouterData>, String)>
    for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, connector_mandate_id): (
            &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
            String,
        ),
    ) -> Result<Self, Self::Error> {
        let processing_information = ProcessingInformation::try_from((item, None, None))?;
        let payment_instrument = CybersoucrePaymentInstrument {
            id: connector_mandate_id.into(),
        };
        let bill_to = item
            .router_data
            .get_optional_billing_email()
            .or(item.router_data.request.get_optional_email())
            .and_then(|email| build_bill_to(item.router_data.get_optional_billing(), email).ok());
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
pub struct CybersourceAuthSetupRequest {
    payment_information: PaymentInformation,
    client_reference_information: ClientReferenceInformation,
}

impl TryFrom<&CybersourceRouterData<&PaymentsAuthorizeRouterData>> for CybersourceAuthSetupRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(ccard) => {
                let card_type = match ccard
                    .card_network
                    .clone()
                    .and_then(get_cybersource_card_type)
                {
                    Some(card_network) => Some(card_network.to_string()),
                    None => ccard.get_card_issuer().ok().map(String::from),
                };
                let payment_information =
                    PaymentInformation::Cards(Box::new(CardPaymentInformation {
                        card: Card {
                            number: ccard.card_number,
                            expiration_month: ccard.card_exp_month,
                            expiration_year: ccard.card_exp_year,
                            security_code: Some(ccard.card_cvc),
                            card_type,
                        },
                    }));
                let client_reference_information = ClientReferenceInformation::from(item);
                Ok(Self {
                    payment_information,
                    client_reference_information,
                })
            }
            PaymentMethodData::Wallet(_)
            | PaymentMethodData::CardRedirect(_)
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
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsCaptureRequest {
    processing_information: ProcessingInformation,
    order_information: OrderInformationWithBill,
    client_reference_information: ClientReferenceInformation,
    #[serde(skip_serializing_if = "Option::is_none")]
    merchant_defined_information: Option<Vec<MerchantDefinedInformation>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsIncrementalAuthorizationRequest {
    processing_information: ProcessingInformation,
    order_information: OrderInformationIncrementalAuthorization,
}

impl TryFrom<&CybersourceRouterData<&PaymentsCaptureRouterData>>
    for CybersourcePaymentsCaptureRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let merchant_defined_information = item
            .router_data
            .request
            .metadata
            .clone()
            .map(convert_metadata_to_merchant_defined_info);

        let is_final = matches!(
            item.router_data.request.capture_method,
            Some(enums::CaptureMethod::Manual)
        )
        .then_some(true);

        Ok(Self {
            processing_information: ProcessingInformation {
                capture_options: Some(CaptureOptions {
                    capture_sequence_number: 1,
                    total_capture_count: 1,
                    is_final,
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

impl TryFrom<&CybersourceRouterData<&PaymentsIncrementalAuthorizationRouterData>>
    for CybersourcePaymentsIncrementalAuthorizationRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&PaymentsIncrementalAuthorizationRouterData>,
    ) -> Result<Self, Self::Error> {
        let connector_merchant_config =
            CybersourceConnectorMetadataObject::try_from(&item.router_data.connector_meta_data)?;

        Ok(Self {
            processing_information: ProcessingInformation {
                action_list: None,
                action_token_types: None,
                authorization_options: Some(CybersourceAuthorizationOptions {
                    initiator: Some(CybersourcePaymentInitiator {
                        initiator_type: None,
                        credential_stored_on_file: None,
                        stored_credential_used: Some(true),
                    }),
                    merchant_intitiated_transaction: Some(MerchantInitiatedTransaction {
                        reason: Some("5".to_owned()),
                        previous_transaction_id: None,
                        original_authorized_amount: None,
                    }),
                    ignore_avs_result: connector_merchant_config.disable_avs,
                    ignore_cv_result: connector_merchant_config.disable_cvn,
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
pub struct CybersourceVoidRequest {
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

impl TryFrom<&CybersourceRouterData<&PaymentsCancelRouterData>> for CybersourceVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        value: &CybersourceRouterData<&PaymentsCancelRouterData>,
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

pub struct CybersourceAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) merchant_account: Secret<String>,
    pub(super) api_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for CybersourceAuthType {
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
pub enum CybersourcePaymentStatus {
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
pub enum CybersourceIncrementalAuthorizationStatus {
    Authorized,
    Declined,
    AuthorizedPendingReview,
}

pub fn map_cybersource_attempt_status(
    status: CybersourcePaymentStatus,
    capture: bool,
) -> enums::AttemptStatus {
    match status {
        CybersourcePaymentStatus::Authorized => {
            if capture {
                // Because Cybersource will return Payment Status as Authorized even in AutoCapture Payment
                enums::AttemptStatus::Charged
            } else {
                enums::AttemptStatus::Authorized
            }
        }
        CybersourcePaymentStatus::Succeeded | CybersourcePaymentStatus::Transmitted => {
            enums::AttemptStatus::Charged
        }
        CybersourcePaymentStatus::Voided
        | CybersourcePaymentStatus::Reversed
        | CybersourcePaymentStatus::Cancelled => enums::AttemptStatus::Voided,
        CybersourcePaymentStatus::Failed
        | CybersourcePaymentStatus::Declined
        | CybersourcePaymentStatus::AuthorizedRiskDeclined
        | CybersourcePaymentStatus::Rejected
        | CybersourcePaymentStatus::InvalidRequest
        | CybersourcePaymentStatus::ServerError => enums::AttemptStatus::Failure,
        CybersourcePaymentStatus::PendingAuthentication => {
            enums::AttemptStatus::AuthenticationPending
        }
        CybersourcePaymentStatus::PendingReview
        | CybersourcePaymentStatus::StatusNotReceived
        | CybersourcePaymentStatus::Challenge
        | CybersourcePaymentStatus::Accepted
        | CybersourcePaymentStatus::Pending
        | CybersourcePaymentStatus::AuthorizedPendingReview => enums::AttemptStatus::Pending,
    }
}
impl From<CybersourceIncrementalAuthorizationStatus> for common_enums::AuthorizationStatus {
    fn from(item: CybersourceIncrementalAuthorizationStatus) -> Self {
        match item {
            CybersourceIncrementalAuthorizationStatus::Authorized => Self::Success,
            CybersourceIncrementalAuthorizationStatus::AuthorizedPendingReview => Self::Processing,
            CybersourceIncrementalAuthorizationStatus::Declined => Self::Failure,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsResponse {
    id: String,
    status: Option<CybersourcePaymentStatus>,
    client_reference_information: Option<ClientReferenceInformation>,
    processor_information: Option<ClientProcessorInformation>,
    risk_information: Option<ClientRiskInformation>,
    token_information: Option<CybersourceTokenInformation>,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceErrorInformationResponse {
    id: String,
    error_information: CybersourceErrorInformation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformationResponse {
    access_token: String,
    device_data_collection_url: String,
    reference_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientAuthSetupInfoResponse {
    id: String,
    client_reference_information: ClientReferenceInformation,
    consumer_authentication_information: CybersourceConsumerAuthInformationResponse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourceAuthSetupResponse {
    ClientAuthSetupInfo(Box<ClientAuthSetupInfoResponse>),
    ErrorInformation(Box<CybersourceErrorInformationResponse>),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePaymentsIncrementalAuthorizationResponse {
    status: CybersourceIncrementalAuthorizationStatus,
    error_information: Option<CybersourceErrorInformation>,
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
pub struct CybersourceTokenInformation {
    payment_instrument: Option<CybersoucrePaymentInstrument>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CybersourceErrorInformation {
    reason: Option<String>,
    message: Option<String>,
    details: Option<Vec<Details>>,
}

fn get_error_response_if_failure(
    (info_response, status, http_code): (&CybersourcePaymentsResponse, enums::AttemptStatus, u16),
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
    (info_response, status, http_code): (&CybersourcePaymentsResponse, enums::AttemptStatus, u16),
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
            CybersourcePaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Authorize,
            CybersourcePaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = map_cybersource_attempt_status(
            item.response
                .status
                .clone()
                .unwrap_or(CybersourcePaymentStatus::StatusNotReceived),
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

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            CybersourceAuthSetupResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CybersourceAuthSetupResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourceAuthSetupResponse::ClientAuthSetupInfo(info_response) => Ok(Self {
                status: enums::AttemptStatus::AuthenticationPending,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::NoResponseId,
                    redirection_data: Box::new(Some(RedirectForm::CybersourceAuthSetup {
                        access_token: info_response
                            .consumer_authentication_information
                            .access_token,
                        ddc_url: info_response
                            .consumer_authentication_information
                            .device_data_collection_url,
                        reference_id: info_response
                            .consumer_authentication_information
                            .reference_id,
                    })),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(
                        info_response
                            .client_reference_information
                            .code
                            .unwrap_or(info_response.id.clone()),
                    ),
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            }),
            CybersourceAuthSetupResponse::ErrorInformation(error_response) => {
                let detailed_error_info =
                    error_response
                        .error_information
                        .details
                        .to_owned()
                        .map(|details| {
                            details
                                .iter()
                                .map(|details| format!("{} : {}", details.field, details.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });

                let reason = get_error_reason(
                    error_response.error_information.message,
                    detailed_error_info,
                    None,
                );
                let error_message = error_response.error_information.reason;
                Ok(Self {
                    response: Err(ErrorResponse {
                        code: error_message
                            .clone()
                            .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
                        message: error_message.unwrap_or(
                            hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string(),
                        ),
                        reason,
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(error_response.id.clone()),
                    }),
                    status: enums::AttemptStatus::AuthenticationFailed,
                    ..item.data
                })
            }
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformationRequest {
    return_url: String,
    reference_id: String,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceAuthEnrollmentRequest {
    payment_information: PaymentInformation,
    client_reference_information: ClientReferenceInformation,
    consumer_authentication_information: CybersourceConsumerAuthInformationRequest,
    order_information: OrderInformationWithBill,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct CybersourceRedirectionAuthResponse {
    pub transaction_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformationValidateRequest {
    authentication_transaction_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceAuthValidateRequest {
    payment_information: PaymentInformation,
    client_reference_information: ClientReferenceInformation,
    consumer_authentication_information: CybersourceConsumerAuthInformationValidateRequest,
    order_information: OrderInformation,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum CybersourcePreProcessingRequest {
    AuthEnrollment(Box<CybersourceAuthEnrollmentRequest>),
    AuthValidate(Box<CybersourceAuthValidateRequest>),
}

impl TryFrom<&CybersourceRouterData<&PaymentsPreProcessingRouterData>>
    for CybersourcePreProcessingRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&PaymentsPreProcessingRouterData>,
    ) -> Result<Self, Self::Error> {
        let client_reference_information = ClientReferenceInformation {
            code: Some(item.router_data.connector_request_reference_id.clone()),
        };
        let payment_method_data = item.router_data.request.payment_method_data.clone().ok_or(
            errors::ConnectorError::MissingConnectorRedirectionPayload {
                field_name: "payment_method_data",
            },
        )?;
        let payment_information = match payment_method_data {
            PaymentMethodData::Card(ccard) => {
                let card_type = match ccard
                    .card_network
                    .clone()
                    .and_then(get_cybersource_card_type)
                {
                    Some(card_network) => Some(card_network.to_string()),
                    None => ccard.get_card_issuer().ok().map(String::from),
                };
                Ok(PaymentInformation::Cards(Box::new(
                    CardPaymentInformation {
                        card: Card {
                            number: ccard.card_number,
                            expiration_month: ccard.card_exp_month,
                            expiration_year: ccard.card_exp_year,
                            security_code: Some(ccard.card_cvc),
                            card_type,
                        },
                    },
                )))
            }
            PaymentMethodData::Wallet(_)
            | PaymentMethodData::CardRedirect(_)
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
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                ))
            }
        }?;

        let redirect_response = item.router_data.request.redirect_response.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "redirect_response",
            },
        )?;

        let amount_details = Amount {
            total_amount: item.amount.clone(),
            currency: item.router_data.request.currency.ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                },
            )?,
        };

        match redirect_response.params {
            Some(param) if !param.clone().peek().is_empty() => {
                let reference_id = param
                    .clone()
                    .peek()
                    .split_once('=')
                    .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                        field_name: "request.redirect_response.params.reference_id",
                    })?
                    .1
                    .to_string();
                let email = item
                    .router_data
                    .get_billing_email()
                    .or(item.router_data.request.get_email())?;
                let bill_to = build_bill_to(item.router_data.get_optional_billing(), email)?;
                let order_information = OrderInformationWithBill {
                    amount_details,
                    bill_to: Some(bill_to),
                };
                Ok(Self::AuthEnrollment(Box::new(
                    CybersourceAuthEnrollmentRequest {
                        payment_information,
                        client_reference_information,
                        consumer_authentication_information:
                            CybersourceConsumerAuthInformationRequest {
                                return_url: item
                                    .router_data
                                    .request
                                    .get_complete_authorize_url()?,
                                reference_id,
                            },
                        order_information,
                    },
                )))
            }
            Some(_) | None => {
                let redirect_payload: CybersourceRedirectionAuthResponse = redirect_response
                    .payload
                    .ok_or(errors::ConnectorError::MissingConnectorRedirectionPayload {
                        field_name: "request.redirect_response.payload",
                    })?
                    .peek()
                    .clone()
                    .parse_value("CybersourceRedirectionAuthResponse")
                    .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                let order_information = OrderInformation { amount_details };
                Ok(Self::AuthValidate(Box::new(
                    CybersourceAuthValidateRequest {
                        payment_information,
                        client_reference_information,
                        consumer_authentication_information:
                            CybersourceConsumerAuthInformationValidateRequest {
                                authentication_transaction_id: redirect_payload.transaction_id,
                            },
                        order_information,
                    },
                )))
            }
        }
    }
}

impl TryFrom<&CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>>
    for CybersourcePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&PaymentsCompleteAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let payment_method_data = item.router_data.request.payment_method_data.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_data",
            },
        )?;
        match payment_method_data {
            PaymentMethodData::Card(ccard) => Self::try_from((item, ccard)),
            PaymentMethodData::Wallet(_)
            | PaymentMethodData::CardRedirect(_)
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
                    utils::get_unimplemented_payment_method_error_message("Cybersource"),
                )
                .into())
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourceAuthEnrollmentStatus {
    PendingAuthentication,
    AuthenticationSuccessful,
    AuthenticationFailed,
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthValidateResponse {
    ucaf_collection_indicator: Option<String>,
    cavv: Option<String>,
    ucaf_authentication_data: Option<Secret<String>>,
    xid: Option<String>,
    specification_version: Option<String>,
    directory_server_transaction_id: Option<Secret<String>>,
    indicator: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CybersourceThreeDSMetadata {
    three_ds_data: CybersourceConsumerAuthValidateResponse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceConsumerAuthInformationEnrollmentResponse {
    access_token: Option<Secret<String>>,
    step_up_url: Option<String>,
    //Added to segregate the three_ds_data in a separate struct
    #[serde(flatten)]
    validate_response: CybersourceConsumerAuthValidateResponse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientAuthCheckInfoResponse {
    id: String,
    client_reference_information: ClientReferenceInformation,
    consumer_authentication_information: CybersourceConsumerAuthInformationEnrollmentResponse,
    status: CybersourceAuthEnrollmentStatus,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourcePreProcessingResponse {
    ClientAuthCheckInfo(Box<ClientAuthCheckInfoResponse>),
    ErrorInformation(Box<CybersourceErrorInformationResponse>),
}

impl From<CybersourceAuthEnrollmentStatus> for enums::AttemptStatus {
    fn from(item: CybersourceAuthEnrollmentStatus) -> Self {
        match item {
            CybersourceAuthEnrollmentStatus::PendingAuthentication => Self::AuthenticationPending,
            CybersourceAuthEnrollmentStatus::AuthenticationSuccessful => {
                Self::AuthenticationSuccessful
            }
            CybersourceAuthEnrollmentStatus::AuthenticationFailed => Self::AuthenticationFailed,
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            CybersourcePreProcessingResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsPreProcessingData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CybersourcePreProcessingResponse,
            PaymentsPreProcessingData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            CybersourcePreProcessingResponse::ClientAuthCheckInfo(info_response) => {
                let status = enums::AttemptStatus::from(info_response.status);
                let risk_info: Option<ClientRiskInformation> = None;
                if utils::is_payment_failure(status) {
                    let response = Err(get_error_response(
                        &info_response.error_information,
                        &risk_info,
                        Some(status),
                        item.http_code,
                        info_response.id.clone(),
                    ));

                    Ok(Self {
                        status,
                        response,
                        ..item.data
                    })
                } else {
                    let connector_response_reference_id = Some(
                        info_response
                            .client_reference_information
                            .code
                            .unwrap_or(info_response.id.clone()),
                    );

                    let redirection_data = match (
                        info_response
                            .consumer_authentication_information
                            .access_token,
                        info_response
                            .consumer_authentication_information
                            .step_up_url,
                    ) {
                        (Some(token), Some(step_up_url)) => {
                            Some(RedirectForm::CybersourceConsumerAuth {
                                access_token: token.expose(),
                                step_up_url,
                            })
                        }
                        _ => None,
                    };
                    let three_ds_data = serde_json::to_value(
                        info_response
                            .consumer_authentication_information
                            .validate_response,
                    )
                    .change_context(errors::ConnectorError::ResponseHandlingFailed)?;
                    Ok(Self {
                        status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: ResponseId::NoResponseId,
                            redirection_data: Box::new(redirection_data),
                            mandate_reference: Box::new(None),
                            connector_metadata: Some(serde_json::json!({
                                "three_ds_data": three_ds_data
                            })),
                            network_txn_id: None,
                            connector_response_reference_id,
                            incremental_authorization_allowed: None,
                            charges: None,
                        }),
                        ..item.data
                    })
                }
            }
            CybersourcePreProcessingResponse::ErrorInformation(error_response) => {
                let detailed_error_info =
                    error_response
                        .error_information
                        .details
                        .to_owned()
                        .map(|details| {
                            details
                                .iter()
                                .map(|details| format!("{} : {}", details.field, details.reason))
                                .collect::<Vec<_>>()
                                .join(", ")
                        });

                let reason = get_error_reason(
                    error_response.error_information.message,
                    detailed_error_info,
                    None,
                );
                let error_message = error_response.error_information.reason.to_owned();
                let response = Err(ErrorResponse {
                    code: error_message
                        .clone()
                        .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
                    message: error_message
                        .unwrap_or(hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
                    reason,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(error_response.id.clone()),
                });
                Ok(Self {
                    response,
                    status: enums::AttemptStatus::AuthenticationFailed,
                    ..item.data
                })
            }
        }
    }
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            CompleteAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = map_cybersource_attempt_status(
            item.response
                .status
                .clone()
                .unwrap_or(CybersourcePaymentStatus::StatusNotReceived),
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
            CybersourcePaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCaptureData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            PaymentsCaptureData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = map_cybersource_attempt_status(
            item.response
                .status
                .clone()
                .unwrap_or(CybersourcePaymentStatus::StatusNotReceived),
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
        ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsCancelData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CybersourcePaymentsResponse,
            PaymentsCancelData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let status = map_cybersource_attempt_status(
            item.response
                .status
                .clone()
                .unwrap_or(CybersourcePaymentStatus::StatusNotReceived),
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
            CybersourcePaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            SetupMandate,
            CybersourcePaymentsResponse,
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
        let mut mandate_status = map_cybersource_attempt_status(
            item.response
                .status
                .clone()
                .unwrap_or(CybersourcePaymentStatus::StatusNotReceived),
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
            CybersourcePaymentsIncrementalAuthorizationResponse,
            T,
            PaymentsResponseData,
        >,
    > for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CybersourcePaymentsIncrementalAuthorizationResponse,
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
                None => Ok(PaymentsResponseData::IncrementalAuthorizationResponse {
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
pub struct CybersourceTransactionResponse {
    id: String,
    application_information: ApplicationInformation,
    client_reference_information: Option<ClientReferenceInformation>,
    error_information: Option<CybersourceErrorInformation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationInformation {
    status: Option<CybersourcePaymentStatus>,
}

impl<F>
    TryFrom<
        ResponseRouterData<
            F,
            CybersourceTransactionResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    > for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            CybersourceTransactionResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response.application_information.status {
            Some(status) => {
                let status =
                    map_cybersource_attempt_status(status, item.data.request.is_auto_capture()?);
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
pub struct CybersourceRefundRequest {
    order_information: OrderInformation,
    client_reference_information: ClientReferenceInformation,
}

impl<F> TryFrom<&CybersourceRouterData<&RefundsRouterData<F>>> for CybersourceRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &CybersourceRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
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

impl From<CybersourceRefundStatus> for enums::RefundStatus {
    fn from(item: CybersourceRefundStatus) -> Self {
        match item {
            CybersourceRefundStatus::Succeeded | CybersourceRefundStatus::Transmitted => {
                Self::Success
            }
            CybersourceRefundStatus::Cancelled
            | CybersourceRefundStatus::Failed
            | CybersourceRefundStatus::Voided => Self::Failure,
            CybersourceRefundStatus::Pending => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourceRefundStatus {
    Succeeded,
    Transmitted,
    Failed,
    Pending,
    Voided,
    Cancelled,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRefundResponse {
    id: String,
    status: CybersourceRefundStatus,
    error_information: Option<CybersourceErrorInformation>,
}

impl TryFrom<RefundsResponseRouterData<Execute, CybersourceRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, CybersourceRefundResponse>,
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
    status: Option<CybersourceRefundStatus>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRsyncResponse {
    id: String,
    application_information: Option<RsyncApplicationInformation>,
    error_information: Option<CybersourceErrorInformation>,
}

impl TryFrom<RefundsResponseRouterData<RSync, CybersourceRsyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, CybersourceRsyncResponse>,
    ) -> Result<Self, Self::Error> {
        let response = match item
            .response
            .application_information
            .and_then(|application_information| application_information.status)
        {
            Some(status) => {
                let refund_status = enums::RefundStatus::from(status.clone());
                if utils::is_refund_failure(refund_status) {
                    if status == CybersourceRefundStatus::Voided {
                        Err(get_error_response(
                            &Some(CybersourceErrorInformation {
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

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourcePayoutFulfillRequest {
    client_reference_information: ClientReferenceInformation,
    order_information: OrderInformation,
    recipient_information: CybersourceRecipientInfo,
    sender_information: CybersourceSenderInfo,
    processing_information: CybersourceProcessingInfo,
    payment_information: PaymentInformation,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceRecipientInfo {
    first_name: Secret<String>,
    last_name: Secret<String>,
    address1: Secret<String>,
    locality: String,
    administrative_area: Secret<String>,
    postal_code: Secret<String>,
    country: enums::CountryAlpha2,
    phone_number: Option<Secret<String>>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceSenderInfo {
    reference_number: String,
    account: CybersourceAccountInfo,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceAccountInfo {
    funds_source: CybersourcePayoutFundSourceType,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
pub enum CybersourcePayoutFundSourceType {
    #[serde(rename = "05")]
    Disbursement,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceProcessingInfo {
    business_application_id: CybersourcePayoutBusinessType,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
pub enum CybersourcePayoutBusinessType {
    #[serde(rename = "PP")]
    PersonToPerson,
    #[serde(rename = "AA")]
    AccountToAccount,
}

#[cfg(feature = "payouts")]
impl TryFrom<&CybersourceRouterData<&PayoutsRouterData<PoFulfill>>>
    for CybersourcePayoutFulfillRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &CybersourceRouterData<&PayoutsRouterData<PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        let payout_type = item.router_data.request.get_payout_type()?;
        match payout_type {
            enums::PayoutType::Card => {
                let client_reference_information = ClientReferenceInformation {
                    code: Some(item.router_data.request.payout_id.clone()),
                };

                let order_information = OrderInformation {
                    amount_details: Amount {
                        total_amount: item.amount.to_owned(),
                        currency: item.router_data.request.destination_currency,
                    },
                };

                let billing_address = item.router_data.get_billing_address()?;
                let phone_address = item.router_data.get_billing_phone()?;
                let recipient_information =
                    CybersourceRecipientInfo::try_from((billing_address, phone_address))?;

                let sender_information = CybersourceSenderInfo {
                    reference_number: item.router_data.request.payout_id.clone(),
                    account: CybersourceAccountInfo {
                        funds_source: CybersourcePayoutFundSourceType::Disbursement,
                    },
                };

                let processing_information = CybersourceProcessingInfo {
                    business_application_id: CybersourcePayoutBusinessType::PersonToPerson, // this means sender and receiver are different
                };

                let payout_method_data = item.router_data.get_payout_method_data()?;
                let payment_information = PaymentInformation::try_from(payout_method_data)?;

                Ok(Self {
                    client_reference_information,
                    order_information,
                    recipient_information,
                    sender_information,
                    processing_information,
                    payment_information,
                })
            }
            enums::PayoutType::Bank | enums::PayoutType::Wallet => {
                Err(errors::ConnectorError::NotSupported {
                    message: "PayoutType is not supported".to_string(),
                    connector: "Cybersource",
                })?
            }
        }
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<(&AddressDetails, &PhoneDetails)> for CybersourceRecipientInfo {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: (&AddressDetails, &PhoneDetails)) -> Result<Self, Self::Error> {
        let (billing_address, phone_address) = item;
        Ok(Self {
            first_name: billing_address.get_first_name()?.to_owned(),
            last_name: billing_address.get_last_name()?.to_owned(),
            address1: billing_address.get_line1()?.to_owned(),
            locality: billing_address.get_city()?.to_owned(),
            administrative_area: billing_address.get_state()?.to_owned(),
            postal_code: billing_address.get_zip()?.to_owned(),
            country: billing_address.get_country()?.to_owned(),
            phone_number: phone_address.number.clone(),
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<PayoutMethodData> for PaymentInformation {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: PayoutMethodData) -> Result<Self, Self::Error> {
        match item {
            PayoutMethodData::Card(card_details) => {
                let card_issuer = card_details.get_card_issuer().ok();
                let card_type = card_issuer.map(String::from);
                let card = Card {
                    number: card_details.card_number,
                    expiration_month: card_details.expiry_month,
                    expiration_year: card_details.expiry_year,
                    security_code: None,
                    card_type,
                };
                Ok(Self::Cards(Box::new(CardPaymentInformation { card })))
            }
            PayoutMethodData::Bank(_) | PayoutMethodData::Wallet(_) => {
                Err(errors::ConnectorError::NotSupported {
                    message: "PayoutMethod is not supported".to_string(),
                    connector: "Cybersource",
                })?
            }
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceFulfillResponse {
    id: String,
    status: CybersourcePayoutStatus,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CybersourcePayoutStatus {
    Accepted,
    Declined,
    InvalidRequest,
}

#[cfg(feature = "payouts")]
fn map_payout_status(status: CybersourcePayoutStatus) -> enums::PayoutStatus {
    match status {
        CybersourcePayoutStatus::Accepted => enums::PayoutStatus::Success,
        CybersourcePayoutStatus::Declined | CybersourcePayoutStatus::InvalidRequest => {
            enums::PayoutStatus::Failed
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<PayoutsResponseRouterData<F, CybersourceFulfillResponse>> for PayoutsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<F, CybersourceFulfillResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(PayoutsResponseData {
                status: Some(map_payout_status(item.response.status)),
                connector_payout_id: Some(item.response.id),
                payout_eligible: None,
                should_add_next_step_to_process_tracker: false,
                error_code: None,
                error_message: None,
            }),
            ..item.data
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceStandardErrorResponse {
    pub error_information: Option<ErrorInformation>,
    pub status: Option<String>,
    pub message: Option<String>,
    pub reason: Option<String>,
    pub details: Option<Vec<Details>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceNotAvailableErrorResponse {
    pub errors: Vec<CybersourceNotAvailableErrorObject>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceNotAvailableErrorObject {
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CybersourceServerErrorResponse {
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
pub struct CybersourceAuthenticationErrorResponse {
    pub response: AuthenticationErrorInformation,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum CybersourceErrorResponse {
    AuthenticationError(Box<CybersourceAuthenticationErrorResponse>),
    //If the request resource is not available/exists in cybersource
    NotAvailableError(Box<CybersourceNotAvailableErrorResponse>),
    StandardError(Box<CybersourceStandardErrorResponse>),
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
    error_data: &Option<CybersourceErrorInformation>,
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

    let detailed_error_info = error_data.as_ref().and_then(|error_data| {
        error_data.details.as_ref().map(|details| {
            details
                .iter()
                .map(|detail| format!("{} : {}", detail.field, detail.reason))
                .collect::<Vec<_>>()
                .join(", ")
        })
    });

    let reason = get_error_reason(
        error_data
            .as_ref()
            .and_then(|error_info| error_info.message.clone()),
        detailed_error_info,
        avs_message,
    );

    let error_message = error_data
        .as_ref()
        .and_then(|error_info| error_info.reason.clone());

    ErrorResponse {
        code: error_message
            .clone()
            .unwrap_or_else(|| hyperswitch_interfaces::consts::NO_ERROR_CODE.to_string()),
        message: error_message
            .unwrap_or_else(|| hyperswitch_interfaces::consts::NO_ERROR_MESSAGE.to_string()),
        reason,
        status_code,
        attempt_status,
        connector_transaction_id: Some(transaction_id),
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

fn get_cybersource_card_type(card_network: common_enums::CardNetwork) -> Option<&'static str> {
    match card_network {
        common_enums::CardNetwork::Visa => Some("001"),
        common_enums::CardNetwork::Mastercard => Some("002"),
        common_enums::CardNetwork::AmericanExpress => Some("003"),
        common_enums::CardNetwork::JCB => Some("007"),
        common_enums::CardNetwork::DinersClub => Some("005"),
        common_enums::CardNetwork::Discover => Some("004"),
        common_enums::CardNetwork::CartesBancaires => Some("006"),
        common_enums::CardNetwork::UnionPay => Some("062"),
        //"042" is the type code for Masetro Cards(International). For Maestro Cards(UK-Domestic) the mapping should be "024"
        common_enums::CardNetwork::Maestro => Some("042"),
        common_enums::CardNetwork::Interac | common_enums::CardNetwork::RuPay => None,
    }
}

pub trait RemoveNewLine {
    fn remove_new_line(&self) -> Self;
}

impl RemoveNewLine for Option<Secret<String>> {
    fn remove_new_line(&self) -> Self {
        self.clone().map(|masked_value| {
            let new_string = masked_value.expose().replace("\n", " ");
            Secret::new(new_string)
        })
    }
}

impl RemoveNewLine for Option<String> {
    fn remove_new_line(&self) -> Self {
        self.clone().map(|value| value.replace("\n", " "))
    }
}
