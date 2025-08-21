use common_enums::{enums, CaptureMethod, PaymentChannel};
use common_utils::{
    crypto::{self, GenerateDigest},
    date_time,
    ext_traits::{Encode, OptionExt},
    fp_utils,
    id_type::CustomerId,
    pii::{Email, IpAddress},
    request::Method,
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    mandates::{MandateData, MandateDataType},
    payment_method_data::{
        self, ApplePayWalletData, BankRedirectData, GooglePayWalletData, PayLaterData,
        PaymentMethodData, WalletData,
    },
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        ErrorResponse, RouterData,
    },
    router_flow_types::{
        refunds::{Execute, RSync},
        Authorize, Capture, CompleteAuthorize, PSync, PostCaptureVoid, Void,
    },
    router_request_types::{
        authentication::MessageExtensionAttribute, BrowserInformation, PaymentsAuthorizeData,
        PaymentsPreProcessingData, ResponseId,
    },
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types,
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{
    types::{
        PaymentsPreprocessingResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        self, missing_field_err, AddressDetailsData, BrowserInformationData, ForeignTryFrom,
        PaymentsAuthorizeRequestData, PaymentsCancelRequestData, PaymentsPreProcessingRequestData,
        RouterData as _,
    },
};

fn to_boolean(string: String) -> bool {
    let str = string.as_str();
    match str {
        "true" => true,
        "false" => false,
        "yes" => true,
        "no" => false,
        _ => false,
    }
}

// The dimensions of the challenge window for full screen.
const CHALLENGE_WINDOW_SIZE: &str = "05";
// The challenge preference for the challenge flow.
const CHALLENGE_PREFERNCE: &str = "01";

trait NuveiAuthorizePreprocessingCommon {
    fn get_browser_info(&self) -> Option<BrowserInformation>;
    fn get_related_transaction_id(&self) -> Option<String>;
    fn get_setup_mandate_details(&self) -> Option<MandateData>;
    fn get_complete_authorize_url(&self) -> Option<String>;
    fn get_is_moto(&self) -> Option<bool>;
    fn get_connector_mandate_id(&self) -> Option<String>;
    fn get_return_url_required(
        &self,
    ) -> Result<String, error_stack::Report<errors::ConnectorError>>;
    fn get_capture_method(&self) -> Option<CaptureMethod>;
    fn get_amount_required(&self) -> Result<i64, error_stack::Report<errors::ConnectorError>>;
    fn get_customer_id_required(&self) -> Option<CustomerId>;
    fn get_email_required(&self) -> Result<Email, error_stack::Report<errors::ConnectorError>>;
    fn get_currency_required(
        &self,
    ) -> Result<enums::Currency, error_stack::Report<errors::ConnectorError>>;
    fn get_payment_method_data_required(
        &self,
    ) -> Result<PaymentMethodData, error_stack::Report<errors::ConnectorError>>;
    fn get_order_tax_amount(
        &self,
    ) -> Result<Option<i64>, error_stack::Report<errors::ConnectorError>>;
}

impl NuveiAuthorizePreprocessingCommon for PaymentsAuthorizeData {
    fn get_browser_info(&self) -> Option<BrowserInformation> {
        self.browser_info.clone()
    }

    fn get_related_transaction_id(&self) -> Option<String> {
        self.related_transaction_id.clone()
    }
    fn get_is_moto(&self) -> Option<bool> {
        match self.payment_channel {
            Some(PaymentChannel::MailOrder) | Some(PaymentChannel::TelephoneOrder) => Some(true),
            _ => None,
        }
    }

    fn get_customer_id_required(&self) -> Option<CustomerId> {
        self.customer_id.clone()
    }

    fn get_setup_mandate_details(&self) -> Option<MandateData> {
        self.setup_mandate_details.clone()
    }

    fn get_complete_authorize_url(&self) -> Option<String> {
        self.complete_authorize_url.clone()
    }

    fn get_connector_mandate_id(&self) -> Option<String> {
        self.connector_mandate_id().clone()
    }

    fn get_return_url_required(
        &self,
    ) -> Result<String, error_stack::Report<errors::ConnectorError>> {
        self.get_router_return_url()
    }

    fn get_capture_method(&self) -> Option<CaptureMethod> {
        self.capture_method
    }

    fn get_amount_required(&self) -> Result<i64, error_stack::Report<errors::ConnectorError>> {
        Ok(self.amount)
    }

    fn get_currency_required(
        &self,
    ) -> Result<enums::Currency, error_stack::Report<errors::ConnectorError>> {
        Ok(self.currency)
    }
    fn get_payment_method_data_required(
        &self,
    ) -> Result<PaymentMethodData, error_stack::Report<errors::ConnectorError>> {
        Ok(self.payment_method_data.clone())
    }
    fn get_order_tax_amount(
        &self,
    ) -> Result<Option<i64>, error_stack::Report<errors::ConnectorError>> {
        Ok(self.order_tax_amount.map(|tax| tax.get_amount_as_i64()))
    }

    fn get_email_required(&self) -> Result<Email, error_stack::Report<errors::ConnectorError>> {
        self.get_email()
    }
}

impl NuveiAuthorizePreprocessingCommon for PaymentsPreProcessingData {
    fn get_browser_info(&self) -> Option<BrowserInformation> {
        self.browser_info.clone()
    }

    fn get_related_transaction_id(&self) -> Option<String> {
        self.related_transaction_id.clone()
    }

    fn get_is_moto(&self) -> Option<bool> {
        None
    }

    fn get_customer_id_required(&self) -> Option<CustomerId> {
        None
    }
    fn get_email_required(&self) -> Result<Email, error_stack::Report<errors::ConnectorError>> {
        self.get_email()
    }
    fn get_setup_mandate_details(&self) -> Option<MandateData> {
        self.setup_mandate_details.clone()
    }

    fn get_complete_authorize_url(&self) -> Option<String> {
        self.complete_authorize_url.clone()
    }

    fn get_connector_mandate_id(&self) -> Option<String> {
        self.connector_mandate_id()
    }

    fn get_return_url_required(
        &self,
    ) -> Result<String, error_stack::Report<errors::ConnectorError>> {
        self.get_router_return_url()
    }

    fn get_capture_method(&self) -> Option<CaptureMethod> {
        self.capture_method
    }

    fn get_amount_required(&self) -> Result<i64, error_stack::Report<errors::ConnectorError>> {
        self.get_amount()
    }

    fn get_currency_required(
        &self,
    ) -> Result<enums::Currency, error_stack::Report<errors::ConnectorError>> {
        self.get_currency()
    }
    fn get_payment_method_data_required(
        &self,
    ) -> Result<PaymentMethodData, error_stack::Report<errors::ConnectorError>> {
        self.payment_method_data.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_data",
            }
            .into(),
        )
    }
    fn get_order_tax_amount(
        &self,
    ) -> Result<Option<i64>, error_stack::Report<errors::ConnectorError>> {
        Ok(None)
    }
}

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct NuveiMeta {
    pub session_token: Secret<String>,
}

#[derive(Debug, Serialize, Default, Deserialize)]
pub struct NuveiMandateMeta {
    pub frequency: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiSessionRequest {
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: String,
    pub time_stamp: date_time::DateTime<date_time::YYYYMMDDHHmmss>,
    pub checksum: Secret<String>,
}

#[derive(Debug, Serialize, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiSessionResponse {
    pub session_token: Secret<String>,
    pub internal_request_id: i64,
    pub status: String,
    pub err_code: i64,
    pub reason: String,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub version: String,
    pub client_request_id: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuvieAmountDetails {
    total_tax: Option<String>,
}
#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentsRequest {
    pub time_stamp: String,
    pub session_token: Secret<String>,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: Secret<String>,
    pub amount: String,
    pub currency: enums::Currency,
    /// This ID uniquely identifies your consumer/user in your system.
    pub user_token_id: Option<CustomerId>,
    //unique transaction id
    pub client_unique_id: String,
    pub transaction_type: TransactionType,
    pub is_rebilling: Option<String>,
    pub payment_option: PaymentOption,
    pub is_moto: Option<bool>,
    pub device_details: DeviceDetails,
    pub checksum: Secret<String>,
    pub billing_address: Option<BillingAddress>,
    pub related_transaction_id: Option<String>,
    pub url_details: Option<UrlDetails>,
    pub amount_details: Option<NuvieAmountDetails>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UrlDetails {
    pub success_url: String,
    pub failure_url: String,
    pub pending_url: String,
}

#[derive(Debug, Serialize, Default)]
pub struct NuveiInitPaymentRequest {
    pub session_token: Secret<String>,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: String,
    pub amount: String,
    pub currency: String,
    pub payment_option: PaymentOption,
    pub checksum: Secret<String>,
}

/// Handles payment request for capture, void and refund flows
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentFlowRequest {
    pub time_stamp: String,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: String,
    pub amount: String,
    pub currency: enums::Currency,
    pub related_transaction_id: Option<String>,
    pub checksum: Secret<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentSyncRequest {
    pub session_token: Secret<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub enum TransactionType {
    Auth,
    #[default]
    Sale,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentOption {
    pub card: Option<Card>,
    pub redirect_url: Option<Url>,
    pub user_payment_option_id: Option<String>,
    pub alternative_payment_method: Option<AlternativePaymentMethod>,
    pub billing_address: Option<BillingAddress>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NuveiBIC {
    #[serde(rename = "ABNANL2A")]
    Abnamro,
    #[serde(rename = "ASNBNL21")]
    ASNBank,
    #[serde(rename = "BUNQNL2A")]
    Bunq,
    #[serde(rename = "INGBNL2A")]
    Ing,
    #[serde(rename = "KNABNL2H")]
    Knab,
    #[serde(rename = "RABONL2U")]
    Rabobank,
    #[serde(rename = "RBRBNL21")]
    RegioBank,
    #[serde(rename = "SNSBNL2A")]
    SNSBank,
    #[serde(rename = "TRIONL2U")]
    TriodosBank,
    #[serde(rename = "FVLBNL22")]
    VanLanschotBankiers,
    #[serde(rename = "MOYONL21")]
    Moneyou,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AlternativePaymentMethod {
    pub payment_method: AlternativePaymentMethodType,
    #[serde(rename = "BIC")]
    pub bank_id: Option<NuveiBIC>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AlternativePaymentMethodType {
    #[default]
    #[serde(rename = "apmgw_expresscheckout")]
    Expresscheckout,
    #[serde(rename = "apmgw_Giropay")]
    Giropay,
    #[serde(rename = "apmgw_Sofort")]
    Sofort,
    #[serde(rename = "apmgw_iDeal")]
    Ideal,
    #[serde(rename = "apmgw_EPS")]
    Eps,
    #[serde(rename = "apmgw_Afterpay")]
    AfterPay,
    #[serde(rename = "apmgw_Klarna")]
    Klarna,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    pub email: Email,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub country: api_models::enums::CountryAlpha2,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub card_number: Option<cards::CardNumber>,
    pub card_holder_name: Option<Secret<String>>,
    pub expiration_month: Option<Secret<String>>,
    pub expiration_year: Option<Secret<String>>,
    #[serde(rename = "CVV")]
    pub cvv: Option<Secret<String>>,
    pub three_d: Option<ThreeD>,
    pub cc_card_number: Option<Secret<String>>,
    pub bin: Option<Secret<String>>,
    pub last4_digits: Option<Secret<String>>,
    pub cc_exp_month: Option<Secret<String>>,
    pub cc_exp_year: Option<Secret<String>>,
    pub acquirer_id: Option<Secret<String>>,
    pub cvv2_reply: Option<String>,
    pub avs_code: Option<String>,
    pub card_type: Option<String>,
    pub card_brand: Option<String>,
    pub issuer_bank_name: Option<String>,
    pub issuer_country: Option<String>,
    pub is_prepaid: Option<String>,
    pub external_token: Option<ExternalToken>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalToken {
    pub external_token_provider: ExternalTokenProvider,
    pub mobile_token: Secret<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ExternalTokenProvider {
    #[default]
    GooglePay,
    ApplePay,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeD {
    pub method_completion_ind: Option<MethodCompletion>,
    pub browser_details: Option<BrowserDetails>,
    pub version: Option<String>,
    #[serde(rename = "notificationURL")]
    pub notification_url: Option<String>,
    #[serde(rename = "merchantURL")]
    pub merchant_url: Option<String>,
    pub acs_url: Option<String>,
    pub acs_challenge_mandate: Option<String>,
    pub c_req: Option<Secret<String>>,
    pub three_d_flow: Option<String>,
    pub external_transaction_id: Option<String>,
    pub transaction_id: Option<String>,
    pub three_d_reason_id: Option<String>,
    pub three_d_reason: Option<String>,
    pub challenge_preference_reason: Option<String>,
    pub challenge_cancel_reason_id: Option<String>,
    pub challenge_cancel_reason: Option<String>,
    pub is_liability_on_issuer: Option<String>,
    pub is_exemption_request_in_authentication: Option<String>,
    pub flow: Option<String>,
    pub acquirer_decision: Option<String>,
    pub decision_reason: Option<String>,
    pub platform_type: Option<PlatformType>,
    pub v2supported: Option<String>,
    pub v2_additional_params: Option<V2AdditionalParams>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum MethodCompletion {
    #[serde(rename = "Y")]
    Success,
    #[serde(rename = "N")]
    Failure,
    #[serde(rename = "U")]
    #[default]
    Unavailable,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub enum PlatformType {
    #[serde(rename = "01")]
    App,
    #[serde(rename = "02")]
    #[default]
    Browser,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserDetails {
    pub accept_header: String,
    pub ip: Secret<String, IpAddress>,
    pub java_enabled: String,
    pub java_script_enabled: String,
    pub language: String,
    pub color_depth: u8,
    pub screen_height: u32,
    pub screen_width: u32,
    pub time_zone: i32,
    pub user_agent: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V2AdditionalParams {
    pub challenge_window_size: Option<String>,
    /// Recurring Expiry in format YYYYMMDD. REQUIRED if isRebilling = 0, We recommend setting rebillExpiry to a value of no more than 5 years from the date of the initial transaction processing date.
    pub rebill_expiry: Option<String>,
    /// Recurring Frequency in days
    pub rebill_frequency: Option<String>,
    pub challenge_preference: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDetails {
    pub ip_address: Secret<String, IpAddress>,
}

impl TransactionType {
    fn get_from_capture_method_and_amount_string(
        capture_method: CaptureMethod,
        amount: &str,
    ) -> Self {
        let amount_value = amount.parse::<f64>();
        if capture_method == CaptureMethod::Manual || amount_value == Ok(0.0) {
            Self::Auth
        } else {
            Self::Sale
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NuveiRedirectionResponse {
    pub cres: Secret<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiACSResponse {
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: Secret<String>,
    #[serde(rename = "acsTransID")]
    pub acs_trans_id: Secret<String>,
    pub message_type: String,
    pub message_version: String,
    pub trans_status: Option<LiabilityShift>,
    pub message_extension: Option<Vec<MessageExtensionAttribute>>,
    pub acs_signed_content: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LiabilityShift {
    #[serde(rename = "Y", alias = "1")]
    Success,
    #[serde(rename = "N", alias = "0")]
    Failed,
}

pub fn encode_payload(
    payload: &[&str],
) -> Result<String, error_stack::Report<errors::ConnectorError>> {
    let data = payload.join("");
    let digest = crypto::Sha256
        .generate_digest(data.as_bytes())
        .change_context(errors::ConnectorError::RequestEncodingFailed)
        .attach_printable("error encoding nuvie payload")?;
    Ok(hex::encode(digest))
}

impl TryFrom<&types::PaymentsAuthorizeSessionTokenRouterData> for NuveiSessionRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &types::PaymentsAuthorizeSessionTokenRouterData,
    ) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = item.connector_request_reference_id.clone();
        let time_stamp = date_time::DateTime::<date_time::YYYYMMDDHHmmss>::from(date_time::now());
        let merchant_secret = connector_meta.merchant_secret;
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: client_request_id.clone(),
            time_stamp: time_stamp.clone(),
            checksum: Secret::new(encode_payload(&[
                merchant_id.peek(),
                merchant_site_id.peek(),
                &client_request_id,
                &time_stamp.to_string(),
                merchant_secret.peek(),
            ])?),
        })
    }
}

impl<F, T> TryFrom<ResponseRouterData<F, NuveiSessionResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NuveiSessionResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::Pending,
            session_token: Some(item.response.session_token.clone().expose()),
            response: Ok(PaymentsResponseData::SessionTokenResponse {
                session_token: item.response.session_token.expose(),
            }),
            ..item.data
        })
    }
}

#[derive(Debug)]
pub struct NuveiCardDetails {
    card: payment_method_data::Card,
    three_d: Option<ThreeD>,
    card_holder_name: Option<Secret<String>>,
}

impl TryFrom<GooglePayWalletData> for NuveiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(gpay_data: GooglePayWalletData) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_option: PaymentOption {
                card: Some(Card {
                    external_token: Some(ExternalToken {
                        external_token_provider: ExternalTokenProvider::GooglePay,
                        mobile_token: Secret::new(
                            utils::GooglePayWalletData::try_from(gpay_data)
                                .change_context(errors::ConnectorError::InvalidDataFormat {
                                    field_name: "google_pay_data",
                                })?
                                .encode_to_string_of_json()
                                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
                        ),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }
}
impl TryFrom<ApplePayWalletData> for NuveiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(apple_pay_data: ApplePayWalletData) -> Result<Self, Self::Error> {
        let apple_pay_encrypted_data = apple_pay_data
            .payment_data
            .get_encrypted_apple_pay_payment_data_mandatory()
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "Apple pay encrypted data",
            })?;
        Ok(Self {
            payment_option: PaymentOption {
                card: Some(Card {
                    external_token: Some(ExternalToken {
                        external_token_provider: ExternalTokenProvider::ApplePay,
                        mobile_token: Secret::new(apple_pay_encrypted_data.clone()),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        })
    }
}

impl TryFrom<enums::BankNames> for NuveiBIC {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(bank: enums::BankNames) -> Result<Self, Self::Error> {
        match bank {
            enums::BankNames::AbnAmro => Ok(Self::Abnamro),
            enums::BankNames::AsnBank => Ok(Self::ASNBank),
            enums::BankNames::Bunq => Ok(Self::Bunq),
            enums::BankNames::Ing => Ok(Self::Ing),
            enums::BankNames::Knab => Ok(Self::Knab),
            enums::BankNames::Rabobank => Ok(Self::Rabobank),
            enums::BankNames::SnsBank => Ok(Self::SNSBank),
            enums::BankNames::TriodosBank => Ok(Self::TriodosBank),
            enums::BankNames::VanLanschot => Ok(Self::VanLanschotBankiers),
            enums::BankNames::Moneyou => Ok(Self::Moneyou),

            enums::BankNames::AmericanExpress
            | enums::BankNames::AffinBank
            | enums::BankNames::AgroBank
            | enums::BankNames::AllianceBank
            | enums::BankNames::AmBank
            | enums::BankNames::BankOfAmerica
            | enums::BankNames::BankOfChina
            | enums::BankNames::BankIslam
            | enums::BankNames::BankMuamalat
            | enums::BankNames::BankRakyat
            | enums::BankNames::BankSimpananNasional
            | enums::BankNames::Barclays
            | enums::BankNames::BlikPSP
            | enums::BankNames::CapitalOne
            | enums::BankNames::Chase
            | enums::BankNames::Citi
            | enums::BankNames::CimbBank
            | enums::BankNames::Discover
            | enums::BankNames::NavyFederalCreditUnion
            | enums::BankNames::PentagonFederalCreditUnion
            | enums::BankNames::SynchronyBank
            | enums::BankNames::WellsFargo
            | enums::BankNames::Handelsbanken
            | enums::BankNames::HongLeongBank
            | enums::BankNames::HsbcBank
            | enums::BankNames::KuwaitFinanceHouse
            | enums::BankNames::Regiobank
            | enums::BankNames::Revolut
            | enums::BankNames::ArzteUndApothekerBank
            | enums::BankNames::AustrianAnadiBankAg
            | enums::BankNames::BankAustria
            | enums::BankNames::Bank99Ag
            | enums::BankNames::BankhausCarlSpangler
            | enums::BankNames::BankhausSchelhammerUndSchatteraAg
            | enums::BankNames::BankMillennium
            | enums::BankNames::BankPEKAOSA
            | enums::BankNames::BawagPskAg
            | enums::BankNames::BksBankAg
            | enums::BankNames::BrullKallmusBankAg
            | enums::BankNames::BtvVierLanderBank
            | enums::BankNames::CapitalBankGraweGruppeAg
            | enums::BankNames::CeskaSporitelna
            | enums::BankNames::Dolomitenbank
            | enums::BankNames::EasybankAg
            | enums::BankNames::EPlatbyVUB
            | enums::BankNames::ErsteBankUndSparkassen
            | enums::BankNames::FrieslandBank
            | enums::BankNames::HypoAlpeadriabankInternationalAg
            | enums::BankNames::HypoNoeLbFurNiederosterreichUWien
            | enums::BankNames::HypoOberosterreichSalzburgSteiermark
            | enums::BankNames::HypoTirolBankAg
            | enums::BankNames::HypoVorarlbergBankAg
            | enums::BankNames::HypoBankBurgenlandAktiengesellschaft
            | enums::BankNames::KomercniBanka
            | enums::BankNames::MBank
            | enums::BankNames::MarchfelderBank
            | enums::BankNames::Maybank
            | enums::BankNames::OberbankAg
            | enums::BankNames::OsterreichischeArzteUndApothekerbank
            | enums::BankNames::OcbcBank
            | enums::BankNames::PayWithING
            | enums::BankNames::PlaceZIPKO
            | enums::BankNames::PlatnoscOnlineKartaPlatnicza
            | enums::BankNames::PosojilnicaBankEGen
            | enums::BankNames::PostovaBanka
            | enums::BankNames::PublicBank
            | enums::BankNames::RaiffeisenBankengruppeOsterreich
            | enums::BankNames::RhbBank
            | enums::BankNames::SchelhammerCapitalBankAg
            | enums::BankNames::StandardCharteredBank
            | enums::BankNames::SchoellerbankAg
            | enums::BankNames::SpardaBankWien
            | enums::BankNames::SporoPay
            | enums::BankNames::SantanderPrzelew24
            | enums::BankNames::TatraPay
            | enums::BankNames::Viamo
            | enums::BankNames::VolksbankGruppe
            | enums::BankNames::VolkskreditbankAg
            | enums::BankNames::VrBankBraunau
            | enums::BankNames::UobBank
            | enums::BankNames::PayWithAliorBank
            | enums::BankNames::BankiSpoldzielcze
            | enums::BankNames::PayWithInteligo
            | enums::BankNames::BNPParibasPoland
            | enums::BankNames::BankNowySA
            | enums::BankNames::CreditAgricole
            | enums::BankNames::PayWithBOS
            | enums::BankNames::PayWithCitiHandlowy
            | enums::BankNames::PayWithPlusBank
            | enums::BankNames::ToyotaBank
            | enums::BankNames::VeloBank
            | enums::BankNames::ETransferPocztowy24
            | enums::BankNames::PlusBank
            | enums::BankNames::EtransferPocztowy24
            | enums::BankNames::BankiSpbdzielcze
            | enums::BankNames::BankNowyBfgSa
            | enums::BankNames::GetinBank
            | enums::BankNames::Blik
            | enums::BankNames::NoblePay
            | enums::BankNames::IdeaBank
            | enums::BankNames::EnveloBank
            | enums::BankNames::NestPrzelew
            | enums::BankNames::MbankMtransfer
            | enums::BankNames::Inteligo
            | enums::BankNames::PbacZIpko
            | enums::BankNames::BnpParibas
            | enums::BankNames::BankPekaoSa
            | enums::BankNames::VolkswagenBank
            | enums::BankNames::AliorBank
            | enums::BankNames::Boz
            | enums::BankNames::BangkokBank
            | enums::BankNames::KrungsriBank
            | enums::BankNames::KrungThaiBank
            | enums::BankNames::TheSiamCommercialBank
            | enums::BankNames::KasikornBank
            | enums::BankNames::OpenBankSuccess
            | enums::BankNames::OpenBankFailure
            | enums::BankNames::OpenBankCancelled
            | enums::BankNames::Aib
            | enums::BankNames::BankOfScotland
            | enums::BankNames::DanskeBank
            | enums::BankNames::FirstDirect
            | enums::BankNames::FirstTrust
            | enums::BankNames::Halifax
            | enums::BankNames::Lloyds
            | enums::BankNames::Monzo
            | enums::BankNames::NatWest
            | enums::BankNames::NationwideBank
            | enums::BankNames::RoyalBankOfScotland
            | enums::BankNames::Starling
            | enums::BankNames::TsbBank
            | enums::BankNames::TescoBank
            | enums::BankNames::Yoursafe
            | enums::BankNames::N26
            | enums::BankNames::NationaleNederlanden
            | enums::BankNames::UlsterBank => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Nuvei"),
            ))?,
        }
    }
}

impl<F, Req>
    ForeignTryFrom<(
        AlternativePaymentMethodType,
        Option<BankRedirectData>,
        &RouterData<F, Req, PaymentsResponseData>,
    )> for NuveiPaymentsRequest
where
    Req: NuveiAuthorizePreprocessingCommon,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        data: (
            AlternativePaymentMethodType,
            Option<BankRedirectData>,
            &RouterData<F, Req, PaymentsResponseData>,
        ),
    ) -> Result<Self, Self::Error> {
        let (payment_method, redirect, item) = data;
        let (billing_address, bank_id) = match (&payment_method, redirect) {
            (AlternativePaymentMethodType::Expresscheckout, _) => (
                Some(BillingAddress {
                    email: item.request.get_email_required()?,
                    country: item.get_billing_country()?,
                    ..Default::default()
                }),
                None,
            ),
            (AlternativePaymentMethodType::Giropay, _) => (
                Some(BillingAddress {
                    email: item.request.get_email_required()?,
                    country: item.get_billing_country()?,
                    ..Default::default()
                }),
                None,
            ),
            (AlternativePaymentMethodType::Sofort, _) | (AlternativePaymentMethodType::Eps, _) => {
                let address = item.get_billing_address()?;
                let first_name = address.get_first_name()?;
                (
                    Some(BillingAddress {
                        first_name: Some(first_name.clone()),
                        last_name: Some(address.get_last_name().unwrap_or(first_name).clone()),
                        email: item.request.get_email_required()?,
                        country: item.get_billing_country()?,
                    }),
                    None,
                )
            }
            (
                AlternativePaymentMethodType::Ideal,
                Some(BankRedirectData::Ideal { bank_name, .. }),
            ) => {
                let address = item.get_billing_address()?;
                let first_name = address.get_first_name()?.clone();
                (
                    Some(BillingAddress {
                        first_name: Some(first_name.clone()),
                        last_name: Some(
                            address.get_last_name().ok().unwrap_or(&first_name).clone(),
                        ),
                        email: item.request.get_email_required()?,
                        country: item.get_billing_country()?,
                    }),
                    bank_name.map(NuveiBIC::try_from).transpose()?,
                )
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("Nuvei"),
            ))?,
        };
        Ok(Self {
            payment_option: PaymentOption {
                alternative_payment_method: Some(AlternativePaymentMethod {
                    payment_method,
                    bank_id,
                }),
                ..Default::default()
            },
            billing_address,
            ..Default::default()
        })
    }
}

fn get_pay_later_info<F, Req>(
    payment_method_type: AlternativePaymentMethodType,
    item: &RouterData<F, Req, PaymentsResponseData>,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>>
where
    Req: NuveiAuthorizePreprocessingCommon,
{
    let address = item
        .get_billing()?
        .address
        .as_ref()
        .ok_or_else(missing_field_err("billing.address"))?;
    let first_name = address.get_first_name()?;
    let payment_method = payment_method_type;
    Ok(NuveiPaymentsRequest {
        payment_option: PaymentOption {
            alternative_payment_method: Some(AlternativePaymentMethod {
                payment_method,
                ..Default::default()
            }),
            billing_address: Some(BillingAddress {
                email: item.request.get_email_required()?,
                first_name: Some(first_name.clone()),
                last_name: Some(address.get_last_name().unwrap_or(first_name).clone()),
                country: address.get_country()?.to_owned(),
            }),
            ..Default::default()
        },
        ..Default::default()
    })
}

impl<F, Req> TryFrom<(&RouterData<F, Req, PaymentsResponseData>, String)> for NuveiPaymentsRequest
where
    Req: NuveiAuthorizePreprocessingCommon,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (&RouterData<F, Req, PaymentsResponseData>, String),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let request_data = match item.request.get_payment_method_data_required()?.clone() {
            PaymentMethodData::Card(card) => get_card_info(item, &card),
            PaymentMethodData::MandatePayment => Self::try_from(item),
            PaymentMethodData::Wallet(wallet) => match wallet {
                WalletData::GooglePay(gpay_data) => Self::try_from(gpay_data),
                WalletData::ApplePay(apple_pay_data) => Ok(Self::try_from(apple_pay_data)?),
                WalletData::PaypalRedirect(_) => Self::foreign_try_from((
                    AlternativePaymentMethodType::Expresscheckout,
                    None,
                    item,
                )),
                WalletData::AliPayQr(_)
                | WalletData::AliPayRedirect(_)
                | WalletData::AliPayHkRedirect(_)
                | WalletData::AmazonPayRedirect(_)
                | WalletData::Paysera(_)
                | WalletData::Skrill(_)
                | WalletData::BluecodeRedirect {}
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
                | WalletData::PaypalSdk(_)
                | WalletData::Paze(_)
                | WalletData::SamsungPay(_)
                | WalletData::TwintRedirect {}
                | WalletData::VippsRedirect {}
                | WalletData::TouchNGoRedirect(_)
                | WalletData::WeChatPayRedirect(_)
                | WalletData::CashappQr(_)
                | WalletData::SwishQr(_)
                | WalletData::WeChatPayQr(_)
                | WalletData::RevolutPay(_)
                | WalletData::Mifinity(_) => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("nuvei"),
                )
                .into()),
            },
            PaymentMethodData::BankRedirect(redirect) => match redirect {
                BankRedirectData::Eps { .. } => Self::foreign_try_from((
                    AlternativePaymentMethodType::Eps,
                    Some(redirect),
                    item,
                )),
                BankRedirectData::Giropay { .. } => Self::foreign_try_from((
                    AlternativePaymentMethodType::Giropay,
                    Some(redirect),
                    item,
                )),
                BankRedirectData::Ideal { .. } => Self::foreign_try_from((
                    AlternativePaymentMethodType::Ideal,
                    Some(redirect),
                    item,
                )),
                BankRedirectData::Sofort { .. } => Self::foreign_try_from((
                    AlternativePaymentMethodType::Sofort,
                    Some(redirect),
                    item,
                )),
                BankRedirectData::BancontactCard { .. }
                | BankRedirectData::Bizum {}
                | BankRedirectData::Blik { .. }
                | BankRedirectData::Eft { .. }
                | BankRedirectData::Interac { .. }
                | BankRedirectData::OnlineBankingCzechRepublic { .. }
                | BankRedirectData::OnlineBankingFinland { .. }
                | BankRedirectData::OnlineBankingPoland { .. }
                | BankRedirectData::OnlineBankingSlovakia { .. }
                | BankRedirectData::Przelewy24 { .. }
                | BankRedirectData::Trustly { .. }
                | BankRedirectData::OnlineBankingFpx { .. }
                | BankRedirectData::OnlineBankingThailand { .. }
                | BankRedirectData::OpenBankingUk { .. }
                | BankRedirectData::LocalBankRedirect {} => {
                    Err(errors::ConnectorError::NotImplemented(
                        utils::get_unimplemented_payment_method_error_message("nuvei"),
                    )
                    .into())
                }
            },
            PaymentMethodData::PayLater(pay_later_data) => match pay_later_data {
                PayLaterData::KlarnaRedirect { .. } => {
                    get_pay_later_info(AlternativePaymentMethodType::Klarna, item)
                }
                PayLaterData::AfterpayClearpayRedirect { .. } => {
                    get_pay_later_info(AlternativePaymentMethodType::AfterPay, item)
                }
                PayLaterData::KlarnaSdk { .. }
                | PayLaterData::FlexitiRedirect {}
                | PayLaterData::AffirmRedirect {}
                | PayLaterData::PayBrightRedirect {}
                | PayLaterData::WalleyRedirect {}
                | PayLaterData::AlmaRedirect {}
                | PayLaterData::AtomeRedirect {}
                | PayLaterData::BreadpayRedirect {} => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("nuvei"),
                )
                .into()),
            },
            PaymentMethodData::BankDebit(_)
            | PaymentMethodData::BankTransfer(_)
            | PaymentMethodData::Crypto(_)
            | PaymentMethodData::Reward
            | PaymentMethodData::RealTimePayment(_)
            | PaymentMethodData::MobilePayment(_)
            | PaymentMethodData::Upi(_)
            | PaymentMethodData::Voucher(_)
            | PaymentMethodData::CardRedirect(_)
            | PaymentMethodData::GiftCard(_)
            | PaymentMethodData::OpenBanking(_)
            | PaymentMethodData::CardToken(_)
            | PaymentMethodData::NetworkToken(_)
            | PaymentMethodData::CardDetailsForNetworkTransactionId(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("nuvei"),
                )
                .into())
            }
        }?;
        let currency = item.request.get_currency_required()?;
        let request = Self::try_from(NuveiPaymentRequestData {
            amount: utils::to_currency_base_unit(item.request.get_amount_required()?, currency)?,
            currency,
            connector_auth_type: item.connector_auth_type.clone(),
            client_request_id: item.connector_request_reference_id.clone(),
            session_token: Secret::new(data.1),
            capture_method: item.request.get_capture_method(),
            ..Default::default()
        })?;
        let return_url = item.request.get_return_url_required()?;

        let amount_details = match item.request.get_order_tax_amount()? {
            Some(tax) => Some(NuvieAmountDetails {
                total_tax: Some(utils::to_currency_base_unit(tax, currency)?),
            }),
            None => None,
        };
        Ok(Self {
            is_rebilling: request_data.is_rebilling,
            user_token_id: item.customer_id.clone(),
            related_transaction_id: request_data.related_transaction_id,
            payment_option: request_data.payment_option,
            billing_address: request_data.billing_address,
            device_details: request_data.device_details,
            url_details: Some(UrlDetails {
                success_url: return_url.clone(),
                failure_url: return_url.clone(),
                pending_url: return_url.clone(),
            }),
            amount_details,

            ..request
        })
    }
}

fn get_card_info<F, Req>(
    item: &RouterData<F, Req, PaymentsResponseData>,
    card_details: &payment_method_data::Card,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>>
where
    Req: NuveiAuthorizePreprocessingCommon,
{
    let browser_information = item.request.get_browser_info().clone();
    let related_transaction_id = if item.is_three_ds() {
        item.request.get_related_transaction_id().clone()
    } else {
        None
    };

    let address = item
        .get_optional_billing()
        .and_then(|billing_details| billing_details.address.as_ref());

    let billing_address = match address {
        Some(address) => {
            let first_name = address.get_first_name()?.clone();
            Some(BillingAddress {
                first_name: Some(first_name.clone()),
                last_name: Some(address.get_last_name().ok().unwrap_or(&first_name).clone()),
                email: item.request.get_email_required()?,
                country: item.get_billing_country()?,
            })
        }
        None => None,
    };
    let (is_rebilling, additional_params, user_token_id) =
        match item.request.get_setup_mandate_details().clone() {
            Some(mandate_data) => {
                let details = match mandate_data
                    .mandate_type
                    .get_required_value("mandate_type")
                    .change_context(errors::ConnectorError::MissingRequiredField {
                        field_name: "mandate_type",
                    })? {
                    MandateDataType::SingleUse(details) => details,
                    MandateDataType::MultiUse(details) => {
                        details.ok_or(errors::ConnectorError::MissingRequiredField {
                            field_name: "mandate_data.mandate_type.multi_use",
                        })?
                    }
                };
                let mandate_meta: NuveiMandateMeta = utils::to_connector_meta_from_secret(Some(
                    details.get_metadata().ok_or_else(missing_field_err(
                        "mandate_data.mandate_type.{multi_use|single_use}.metadata",
                    ))?,
                ))?;
                (
                    Some("0".to_string()), // In case of first installment, rebilling should be 0
                    Some(V2AdditionalParams {
                        rebill_expiry: Some(
                            details
                                .get_end_date(date_time::DateFormat::YYYYMMDD)
                                .change_context(errors::ConnectorError::DateFormattingFailed)?
                                .ok_or_else(missing_field_err(
                                    "mandate_data.mandate_type.{multi_use|single_use}.end_date",
                                ))?,
                        ),
                        rebill_frequency: Some(mandate_meta.frequency),
                        challenge_window_size: None,
                        challenge_preference: None,
                    }),
                    item.request.get_customer_id_required(),
                )
            }
            // non mandate transactions
            _ => (
                None,
                Some(V2AdditionalParams {
                    rebill_expiry: None,
                    rebill_frequency: None,
                    challenge_window_size: Some(CHALLENGE_WINDOW_SIZE.to_string()),
                    challenge_preference: Some(CHALLENGE_PREFERNCE.to_string()),
                }),
                None,
            ),
        };
    let three_d = if item.is_three_ds() {
        let browser_details = match &browser_information {
            Some(browser_info) => Some(BrowserDetails {
                accept_header: browser_info.get_accept_header()?,
                ip: browser_info.get_ip_address()?,
                java_enabled: browser_info.get_java_enabled()?.to_string().to_uppercase(),
                java_script_enabled: browser_info
                    .get_java_script_enabled()?
                    .to_string()
                    .to_uppercase(),
                language: browser_info.get_language()?,
                screen_height: browser_info.get_screen_height()?,
                screen_width: browser_info.get_screen_width()?,
                color_depth: browser_info.get_color_depth()?,
                user_agent: browser_info.get_user_agent()?,
                time_zone: browser_info.get_time_zone()?,
            }),
            None => None,
        };
        Some(ThreeD {
            browser_details,
            v2_additional_params: additional_params,
            notification_url: item.request.get_complete_authorize_url().clone(),
            merchant_url: Some(item.request.get_return_url_required()?),
            platform_type: Some(PlatformType::Browser),
            method_completion_ind: Some(MethodCompletion::Unavailable),
            ..Default::default()
        })
    } else {
        None
    };
    let is_moto = item.request.get_is_moto();
    Ok(NuveiPaymentsRequest {
        related_transaction_id,
        is_rebilling,
        user_token_id,
        device_details: DeviceDetails::foreign_try_from(&item.request.get_browser_info().clone())?,
        payment_option: PaymentOption::from(NuveiCardDetails {
            card: card_details.clone(),
            three_d,
            card_holder_name: item.get_optional_billing_full_name(),
        }),
        billing_address,
        is_moto,
        ..Default::default()
    })
}
impl From<NuveiCardDetails> for PaymentOption {
    fn from(card_details: NuveiCardDetails) -> Self {
        let card = card_details.card;
        Self {
            card: Some(Card {
                card_number: Some(card.card_number),
                card_holder_name: card_details.card_holder_name,
                expiration_month: Some(card.card_exp_month),
                expiration_year: Some(card.card_exp_year),
                three_d: card_details.three_d,
                cvv: Some(card.card_cvc),
                ..Default::default()
            }),
            ..Default::default()
        }
    }
}

impl TryFrom<(&types::PaymentsCompleteAuthorizeRouterData, Secret<String>)>
    for NuveiPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (&types::PaymentsCompleteAuthorizeRouterData, Secret<String>),
    ) -> Result<Self, Self::Error> {
        let item = data.0;
        let request_data = match item.request.payment_method_data.clone() {
            Some(PaymentMethodData::Card(card)) => {
                let device_details = DeviceDetails::foreign_try_from(&item.request.browser_info)?;
                Ok(Self {
                    payment_option: PaymentOption::from(NuveiCardDetails {
                        card,
                        three_d: None,
                        card_holder_name: item.get_optional_billing_full_name(),
                    }),
                    device_details,
                    ..Default::default()
                })
            }
            Some(PaymentMethodData::Wallet(..))
            | Some(PaymentMethodData::PayLater(..))
            | Some(PaymentMethodData::BankDebit(..))
            | Some(PaymentMethodData::BankRedirect(..))
            | Some(PaymentMethodData::BankTransfer(..))
            | Some(PaymentMethodData::Crypto(..))
            | Some(PaymentMethodData::MandatePayment)
            | Some(PaymentMethodData::GiftCard(..))
            | Some(PaymentMethodData::Voucher(..))
            | Some(PaymentMethodData::CardRedirect(..))
            | Some(PaymentMethodData::Reward)
            | Some(PaymentMethodData::RealTimePayment(..))
            | Some(PaymentMethodData::MobilePayment(..))
            | Some(PaymentMethodData::Upi(..))
            | Some(PaymentMethodData::OpenBanking(_))
            | Some(PaymentMethodData::CardToken(..))
            | Some(PaymentMethodData::NetworkToken(..))
            | Some(PaymentMethodData::CardDetailsForNetworkTransactionId(_))
            | None => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("nuvei"),
            )),
        }?;
        let request = Self::try_from(NuveiPaymentRequestData {
            amount: utils::to_currency_base_unit(item.request.amount, item.request.currency)?,
            currency: item.request.currency,
            connector_auth_type: item.connector_auth_type.clone(),
            client_request_id: item.connector_request_reference_id.clone(),
            session_token: data.1,
            capture_method: item.request.capture_method,
            ..Default::default()
        })?;
        Ok(Self {
            related_transaction_id: request_data.related_transaction_id,
            payment_option: request_data.payment_option,
            device_details: request_data.device_details,
            ..request
        })
    }
}

impl TryFrom<NuveiPaymentRequestData> for NuveiPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(request: NuveiPaymentRequestData) -> Result<Self, Self::Error> {
        let session_token = request.session_token;
        fp_utils::when(session_token.clone().expose().is_empty(), || {
            Err(errors::ConnectorError::FailedToObtainAuthType)
        })?;
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&request.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = request.client_request_id;
        let time_stamp =
            date_time::format_date(date_time::now(), date_time::DateFormat::YYYYMMDDHHmmss)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        let transaction_type = TransactionType::get_from_capture_method_and_amount_string(
            request.capture_method.unwrap_or_default(),
            &request.amount,
        );
        Ok(Self {
            merchant_id: merchant_id.clone(),
            merchant_site_id: merchant_site_id.clone(),
            client_request_id: Secret::new(client_request_id.clone()),
            time_stamp: time_stamp.clone(),
            session_token,
            transaction_type,
            checksum: Secret::new(encode_payload(&[
                merchant_id.peek(),
                merchant_site_id.peek(),
                &client_request_id,
                &request.amount.clone(),
                &request.currency.to_string(),
                &time_stamp,
                merchant_secret.peek(),
            ])?),
            amount: request.amount,
            user_token_id: None,
            currency: request.currency,
            ..Default::default()
        })
    }
}

impl TryFrom<NuveiPaymentRequestData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(request: NuveiPaymentRequestData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&request.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id;
        let merchant_site_id = connector_meta.merchant_site_id;
        let client_request_id = request.client_request_id;
        let time_stamp =
            date_time::format_date(date_time::now(), date_time::DateFormat::YYYYMMDDHHmmss)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let merchant_secret = connector_meta.merchant_secret;
        Ok(Self {
            merchant_id: merchant_id.to_owned(),
            merchant_site_id: merchant_site_id.to_owned(),
            client_request_id: client_request_id.clone(),
            time_stamp: time_stamp.clone(),
            checksum: Secret::new(encode_payload(&[
                merchant_id.peek(),
                merchant_site_id.peek(),
                &client_request_id,
                &request.amount.clone(),
                &request.currency.to_string(),
                &request.related_transaction_id.clone().unwrap_or_default(),
                &time_stamp,
                merchant_secret.peek(),
            ])?),
            amount: request.amount,
            currency: request.currency,
            related_transaction_id: request.related_transaction_id,
        })
    }
}

#[derive(Debug, Clone, Default)]
pub struct NuveiPaymentRequestData {
    pub amount: String,
    pub currency: enums::Currency,
    pub related_transaction_id: Option<String>,
    pub client_request_id: String,
    pub connector_auth_type: ConnectorAuthType,
    pub session_token: Secret<String>,
    pub capture_method: Option<CaptureMethod>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Self::try_from(NuveiPaymentRequestData {
            client_request_id: item.connector_request_reference_id.clone(),
            connector_auth_type: item.connector_auth_type.clone(),
            amount: utils::to_currency_base_unit(
                item.request.amount_to_capture,
                item.request.currency,
            )?,
            currency: item.request.currency,
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            ..Default::default()
        })
    }
}
impl TryFrom<&types::RefundExecuteRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundExecuteRouterData) -> Result<Self, Self::Error> {
        Self::try_from(NuveiPaymentRequestData {
            client_request_id: item.connector_request_reference_id.clone(),
            connector_auth_type: item.connector_auth_type.clone(),
            amount: utils::to_currency_base_unit(
                item.request.refund_amount,
                item.request.currency,
            )?,
            currency: item.request.currency,
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            ..Default::default()
        })
    }
}

impl TryFrom<&types::PaymentsSyncRouterData> for NuveiPaymentSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let meta: NuveiMeta = utils::to_connector_meta(value.request.connector_meta.clone())?;
        Ok(Self {
            session_token: meta.session_token,
        })
    }
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiVoidRequest {
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_unique_id: String,
    pub related_transaction_id: String,
    pub time_stamp: String,
    pub checksum: Secret<String>,
    pub client_request_id: String,
}

impl TryFrom<&types::PaymentsCancelPostCaptureRouterData> for NuveiVoidRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelPostCaptureRouterData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id.clone();
        let merchant_site_id = connector_meta.merchant_site_id.clone();
        let merchant_secret = connector_meta.merchant_secret.clone();
        let client_unique_id = item.connector_request_reference_id.clone();
        let related_transaction_id = item.request.connector_transaction_id.clone();
        let client_request_id = item.connector_request_reference_id.clone();
        let time_stamp =
            date_time::format_date(date_time::now(), date_time::DateFormat::YYYYMMDDHHmmss)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let checksum = Secret::new(encode_payload(&[
            merchant_id.peek(),
            merchant_site_id.peek(),
            &client_request_id,
            &client_unique_id,
            "", // amount (empty for void)
            "", // currency (empty for void)
            &related_transaction_id,
            "", // authCode (empty)
            "", // comment (empty)
            &time_stamp,
            merchant_secret.peek(),
        ])?);

        Ok(Self {
            merchant_id,
            merchant_site_id,
            client_unique_id,
            related_transaction_id,
            time_stamp,
            checksum,
            client_request_id,
        })
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Self::try_from(NuveiPaymentRequestData {
            client_request_id: item.connector_request_reference_id.clone(),
            connector_auth_type: item.connector_auth_type.clone(),
            amount: utils::to_currency_base_unit(
                item.request.get_amount()?,
                item.request.get_currency()?,
            )?,
            currency: item.request.get_currency()?,
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            ..Default::default()
        })
    }
}

// Auth Struct
pub struct NuveiAuthType {
    pub(super) merchant_id: Secret<String>,
    pub(super) merchant_site_id: Secret<String>,
    pub(super) merchant_secret: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for NuveiAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::SignatureKey {
            api_key,
            key1,
            api_secret,
        } = auth_type
        {
            Ok(Self {
                merchant_id: api_key.to_owned(),
                merchant_site_id: key1.to_owned(),
                merchant_secret: api_secret.to_owned(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NuveiPaymentStatus {
    Success,
    Failed,
    Error,
    #[default]
    Processing,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum NuveiTransactionStatus {
    Approved,
    Declined,
    Error,
    Redirect,
    Pending,
    #[default]
    Processing,
}

impl From<NuveiTransactionStatus> for enums::AttemptStatus {
    fn from(item: NuveiTransactionStatus) -> Self {
        match item {
            NuveiTransactionStatus::Approved => Self::Charged,
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => Self::Failure,
            _ => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentsResponse {
    pub order_id: Option<String>,
    pub user_token_id: Option<Secret<String>>,
    pub payment_option: Option<PaymentOption>,
    pub transaction_status: Option<NuveiTransactionStatus>,
    pub gw_error_code: Option<i64>,
    pub gw_error_reason: Option<String>,
    pub gw_extended_error_code: Option<i64>,
    pub issuer_decline_code: Option<String>,
    pub issuer_decline_reason: Option<String>,
    pub transaction_type: Option<NuveiTransactionType>,
    pub transaction_id: Option<String>,
    pub external_transaction_id: Option<String>,
    pub auth_code: Option<String>,
    pub custom_data: Option<String>,
    pub fraud_details: Option<FraudDetails>,
    pub external_scheme_transaction_id: Option<Secret<String>>,
    pub session_token: Option<Secret<String>>,
    //The ID of the transaction in the merchant’s system.
    pub client_unique_id: Option<String>,
    pub internal_request_id: Option<i64>,
    pub status: NuveiPaymentStatus,
    pub err_code: Option<i64>,
    pub reason: Option<String>,
    pub merchant_id: Option<Secret<String>>,
    pub merchant_site_id: Option<Secret<String>>,
    pub version: Option<String>,
    pub client_request_id: Option<String>,
    pub merchant_advice_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NuveiTransactionType {
    Auth,
    Sale,
    Credit,
    Auth3D,
    InitAuth3D,
    Settle,
    Void,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FraudDetails {
    pub final_decision: String,
}

fn get_payment_status(
    response: &NuveiPaymentsResponse,
    amount: Option<i64>,
) -> enums::AttemptStatus {
    // ZERO dollar authorization
    if amount == Some(0) && response.transaction_type.clone() == Some(NuveiTransactionType::Auth) {
        return match response.transaction_status.clone() {
            Some(NuveiTransactionStatus::Approved) => enums::AttemptStatus::Charged,
            Some(NuveiTransactionStatus::Declined) | Some(NuveiTransactionStatus::Error) => {
                enums::AttemptStatus::AuthorizationFailed
            }
            Some(NuveiTransactionStatus::Pending) | Some(NuveiTransactionStatus::Processing) => {
                enums::AttemptStatus::Pending
            }
            Some(NuveiTransactionStatus::Redirect) => enums::AttemptStatus::AuthenticationPending,
            None => match response.status {
                NuveiPaymentStatus::Failed | NuveiPaymentStatus::Error => {
                    enums::AttemptStatus::Failure
                }
                _ => enums::AttemptStatus::Pending,
            },
        };
    }
    match response.transaction_status.clone() {
        Some(status) => match status {
            NuveiTransactionStatus::Approved => match response.transaction_type {
                Some(NuveiTransactionType::Auth) => enums::AttemptStatus::Authorized,
                Some(NuveiTransactionType::Sale) | Some(NuveiTransactionType::Settle) => {
                    enums::AttemptStatus::Charged
                }
                Some(NuveiTransactionType::Void) => enums::AttemptStatus::Voided,
                _ => enums::AttemptStatus::Pending,
            },
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => {
                match response.transaction_type {
                    Some(NuveiTransactionType::Auth) => enums::AttemptStatus::AuthorizationFailed,
                    Some(NuveiTransactionType::Void) => enums::AttemptStatus::VoidFailed,
                    Some(NuveiTransactionType::Auth3D) => {
                        enums::AttemptStatus::AuthenticationFailed
                    }
                    _ => enums::AttemptStatus::Failure,
                }
            }
            NuveiTransactionStatus::Processing | NuveiTransactionStatus::Pending => {
                enums::AttemptStatus::Pending
            }
            NuveiTransactionStatus::Redirect => enums::AttemptStatus::AuthenticationPending,
        },
        None => match response.status {
            NuveiPaymentStatus::Failed | NuveiPaymentStatus::Error => enums::AttemptStatus::Failure,
            _ => enums::AttemptStatus::Pending,
        },
    }
}

fn build_error_response<T>(
    response: &NuveiPaymentsResponse,
    http_code: u16,
) -> Option<Result<T, error_stack::Report<errors::ConnectorError>>> {
    match response.status {
        NuveiPaymentStatus::Error => Some(
            get_error_response(
                response.err_code,
                &response.reason,
                http_code,
                &response.merchant_advice_code,
                &response.gw_error_code.map(|e| e.to_string()),
                &response.gw_error_reason,
            )
            .map_err(|_err| error_stack::report!(errors::ConnectorError::ResponseHandlingFailed)),
        ),
        _ => {
            let err = Some(
                get_error_response(
                    response.gw_error_code,
                    &response.gw_error_reason,
                    http_code,
                    &response.merchant_advice_code,
                    &response.gw_error_code.map(|e| e.to_string()),
                    &response.gw_error_reason,
                )
                .map_err(|_err| {
                    error_stack::report!(errors::ConnectorError::ResponseHandlingFailed)
                }),
            );
            match response.transaction_status {
                Some(NuveiTransactionStatus::Error) | Some(NuveiTransactionStatus::Declined) => err,
                _ => match response
                    .gw_error_reason
                    .as_ref()
                    .map(|r| r.eq("Missing argument"))
                {
                    Some(true) => err,
                    _ => None,
                },
            }
        }
    }
}

pub trait NuveiPaymentsGenericResponse {}

impl NuveiPaymentsGenericResponse for CompleteAuthorize {}
impl NuveiPaymentsGenericResponse for Void {}
impl NuveiPaymentsGenericResponse for PSync {}
impl NuveiPaymentsGenericResponse for Capture {}
impl NuveiPaymentsGenericResponse for PostCaptureVoid {}

// Helper function to process Nuvei payment response

fn process_nuvei_payment_response<F, T>(
    item: &ResponseRouterData<F, NuveiPaymentsResponse, T, PaymentsResponseData>,
    amount: Option<i64>,
) -> Result<
    (
        enums::AttemptStatus,
        Option<RedirectForm>,
        Option<ConnectorResponseData>,
    ),
    error_stack::Report<errors::ConnectorError>,
>
where
    F: std::fmt::Debug,
    T: std::fmt::Debug,
{
    let redirection_data = match item.data.payment_method {
        enums::PaymentMethod::Wallet | enums::PaymentMethod::BankRedirect => item
            .response
            .payment_option
            .as_ref()
            .and_then(|po| po.redirect_url.clone())
            .map(|base_url| RedirectForm::from((base_url, Method::Get))),
        _ => item
            .response
            .payment_option
            .as_ref()
            .and_then(|o| o.card.clone())
            .and_then(|card| card.three_d)
            .and_then(|three_ds| three_ds.acs_url.zip(three_ds.c_req))
            .map(|(base_url, creq)| RedirectForm::Form {
                endpoint: base_url,
                method: Method::Post,
                form_fields: std::collections::HashMap::from([("creq".to_string(), creq.expose())]),
            }),
    };

    let connector_response_data =
        convert_to_additional_payment_method_connector_response(&item.response)
            .map(ConnectorResponseData::with_additional_payment_method_data);

    let status = get_payment_status(&item.response, amount);

    Ok((status, redirection_data, connector_response_data))
}

// Helper function to create transaction response
fn create_transaction_response(
    response: &NuveiPaymentsResponse,
    redirection_data: Option<RedirectForm>,
    http_code: u16,
) -> Result<PaymentsResponseData, error_stack::Report<errors::ConnectorError>> {
    if let Some(err) = build_error_response(response, http_code) {
        return err;
    }

    Ok(PaymentsResponseData::TransactionResponse {
        resource_id: response
            .transaction_id
            .clone()
            .map_or(response.order_id.clone(), Some) // For paypal there will be no transaction_id, only order_id will be present
            .map(ResponseId::ConnectorTransactionId)
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
        redirection_data: Box::new(redirection_data),
        mandate_reference: Box::new(
            response
                .payment_option
                .as_ref()
                .and_then(|po| po.user_payment_option_id.clone())
                .map(|id| MandateReference {
                    connector_mandate_id: Some(id),
                    payment_method_id: None,
                    mandate_metadata: None,
                    connector_mandate_request_reference_id: None,
                }),
        ),
        // we don't need to save session token for capture, void flow so ignoring if it is not present
        connector_metadata: if let Some(token) = response.session_token.clone() {
            Some(
                serde_json::to_value(NuveiMeta {
                    session_token: token,
                })
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
            )
        } else {
            None
        },
        network_txn_id: None,
        connector_response_reference_id: response.order_id.clone(),
        incremental_authorization_allowed: None,
        charges: None,
    })
}

// Specialized implementation for Authorize
impl
    TryFrom<
        ResponseRouterData<
            Authorize,
            NuveiPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    > for RouterData<Authorize, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            Authorize,
            NuveiPaymentsResponse,
            PaymentsAuthorizeData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        // Get amount directly from the authorize data
        let amount = Some(item.data.request.amount);

        let (status, redirection_data, connector_response_data) =
            process_nuvei_payment_response(&item, amount)?;

        Ok(Self {
            status,
            response: Ok(create_transaction_response(
                &item.response,
                redirection_data,
                item.http_code,
            )?),
            connector_response: connector_response_data,
            ..item.data
        })
    }
}

// Generic implementation for other flow types
impl<F, T> TryFrom<ResponseRouterData<F, NuveiPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
where
    F: NuveiPaymentsGenericResponse + std::fmt::Debug,
    T: std::fmt::Debug,
    F: std::any::Any,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NuveiPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let amount = item
            .data
            .minor_amount_capturable
            .map(|amount| amount.get_amount_as_i64());

        let (status, redirection_data, connector_response_data) =
            process_nuvei_payment_response(&item, amount)?;

        Ok(Self {
            status,
            response: Ok(create_transaction_response(
                &item.response,
                redirection_data,
                item.http_code,
            )?),
            connector_response: connector_response_data,
            ..item.data
        })
    }
}

impl TryFrom<PaymentsPreprocessingResponseRouterData<NuveiPaymentsResponse>>
    for types::PaymentsPreProcessingRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsPreprocessingResponseRouterData<NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let response = item.response;
        let is_enrolled_for_3ds = response
            .clone()
            .payment_option
            .and_then(|po| po.card)
            .and_then(|c| c.three_d)
            .and_then(|t| t.v2supported)
            .map(to_boolean)
            .unwrap_or_default();
        Ok(Self {
            status: get_payment_status(&response, item.data.request.amount),
            response: Ok(PaymentsResponseData::ThreeDSEnrollmentResponse {
                enrolled_v2: is_enrolled_for_3ds,
                related_transaction_id: response.transaction_id,
            }),

            ..item.data
        })
    }
}

impl From<NuveiTransactionStatus> for enums::RefundStatus {
    fn from(item: NuveiTransactionStatus) -> Self {
        match item {
            NuveiTransactionStatus::Approved => Self::Success,
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => Self::Failure,
            NuveiTransactionStatus::Processing
            | NuveiTransactionStatus::Pending
            | NuveiTransactionStatus::Redirect => Self::Pending,
        }
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, NuveiPaymentsResponse>>
    for types::RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = item
            .response
            .transaction_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;

        let refund_response =
            get_refund_response(item.response.clone(), item.http_code, transaction_id)?;

        Ok(Self {
            response: Ok(refund_response),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, NuveiPaymentsResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, NuveiPaymentsResponse>,
    ) -> Result<Self, Self::Error> {
        let transaction_id = item
            .response
            .transaction_id
            .clone()
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;

        let refund_response =
            get_refund_response(item.response.clone(), item.http_code, transaction_id)?;

        Ok(Self {
            response: Ok(refund_response),
            ..item.data
        })
    }
}

impl<F, Req> TryFrom<&RouterData<F, Req, PaymentsResponseData>> for NuveiPaymentsRequest
where
    Req: NuveiAuthorizePreprocessingCommon,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(data: &RouterData<F, Req, PaymentsResponseData>) -> Result<Self, Self::Error> {
        {
            let item = data;
            let connector_mandate_id = &item.request.get_connector_mandate_id();
            let customer_id = item
                .request
                .get_customer_id_required()
                .ok_or(missing_field_err("customer_id")())?;
            let related_transaction_id = if item.is_three_ds() {
                item.request.get_related_transaction_id().clone()
            } else {
                None
            };
            Ok(Self {
                related_transaction_id,
                device_details: DeviceDetails::foreign_try_from(
                    &item.request.get_browser_info().clone(),
                )?,
                is_rebilling: Some("1".to_string()), // In case of second installment, rebilling should be 1
                user_token_id: Some(customer_id),
                payment_option: PaymentOption {
                    user_payment_option_id: connector_mandate_id.clone(),
                    ..Default::default()
                },
                ..Default::default()
            })
        }
    }
}

impl ForeignTryFrom<&Option<BrowserInformation>> for DeviceDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(browser_info: &Option<BrowserInformation>) -> Result<Self, Self::Error> {
        let browser_info = browser_info
            .as_ref()
            .ok_or_else(missing_field_err("browser_info"))?;
        Ok(Self {
            ip_address: browser_info.get_ip_address()?,
        })
    }
}

fn get_refund_response(
    response: NuveiPaymentsResponse,
    http_code: u16,
    txn_id: String,
) -> Result<RefundsResponseData, error_stack::Report<errors::ConnectorError>> {
    let refund_status = response
        .transaction_status
        .clone()
        .map(enums::RefundStatus::from)
        .unwrap_or(enums::RefundStatus::Failure);
    match response.status {
        NuveiPaymentStatus::Error => get_error_response(
            response.err_code,
            &response.reason,
            http_code,
            &response.merchant_advice_code,
            &response.gw_error_code.map(|e| e.to_string()),
            &response.gw_error_reason,
        )
        .map_err(|_err| error_stack::report!(errors::ConnectorError::ResponseHandlingFailed)),
        _ => match response.transaction_status {
            Some(NuveiTransactionStatus::Error) => get_error_response(
                response.err_code,
                &response.reason,
                http_code,
                &response.merchant_advice_code,
                &response.gw_error_code.map(|e| e.to_string()),
                &response.gw_error_reason,
            )
            .map_err(|_err| error_stack::report!(errors::ConnectorError::ResponseHandlingFailed)),
            _ => Ok(RefundsResponseData {
                connector_refund_id: txn_id,
                refund_status,
            }),
        },
    }
}

fn get_error_response<T>(
    error_code: Option<i64>,
    error_msg: &Option<String>,
    http_code: u16,
    network_advice_code: &Option<String>,
    network_decline_code: &Option<String>,
    network_error_message: &Option<String>,
) -> Result<T, Box<ErrorResponse>> {
    Err(Box::new(ErrorResponse {
        code: error_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
        message: error_msg
            .clone()
            .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
        reason: None,
        status_code: http_code,
        attempt_status: None,
        connector_transaction_id: None,
        network_advice_code: network_advice_code.clone(),
        network_decline_code: network_decline_code.clone(),
        network_error_message: network_error_message.clone(),
        connector_metadata: None,
    }))
}

/// Represents any possible webhook notification from Nuvei.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NuveiWebhook {
    PaymentDmn(PaymentDmnNotification),
    Chargeback(ChargebackNotification),
}

/// Represents the status of a chargeback event.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChargebackStatus {
    RetrievalRequest,
    Chargeback,
    Representment,
    SecondChargeback,
    Arbitration,
    #[serde(other)]
    Unknown,
}

/// Represents a Chargeback webhook notification from the Nuvei Control Panel.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChargebackNotification {
    #[serde(rename = "ppp_TransactionID")]
    pub ppp_transaction_id: Option<String>,
    pub merchant_unique_id: Option<String>,
    pub merchant_id: Option<String>,
    pub merchant_site_id: Option<String>,
    pub request_version: Option<String>,
    pub message: Option<String>,
    pub status: Option<ChargebackStatus>,
    pub reason: Option<String>,
    pub case_id: Option<String>,
    pub processor_case_id: Option<String>,
    pub arn: Option<String>,
    pub retrieval_request_date: Option<String>,
    pub chargeback_date: Option<String>,
    pub chargeback_amount: Option<String>,
    pub chargeback_currency: Option<String>,
    pub original_amount: Option<String>,
    pub original_currency: Option<String>,
    #[serde(rename = "transactionID")]
    pub transaction_id: Option<String>,
    pub user_token_id: Option<String>,
}

/// Represents the overall status of the DMN.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DmnStatus {
    Success,
    Error,
    Pending,
}

/// Represents the status of the transaction itself.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum TransactionStatus {
    Approved,
    Declined,
    Error,
    Cancelled,
    Pending,
    #[serde(rename = "Settle")]
    Settled,
}

/// Represents the type of transaction.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum PaymentTransactionType {
    Auth,
    Sale,
    Settle,
    Credit,
    Void,
    Auth3D,
    Sale3D,
    Verif,
}

/// Represents a Payment Direct Merchant Notification (DMN) webhook.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDmnNotification {
    #[serde(rename = "PPP_TransactionID")]
    pub ppp_transaction_id: Option<String>,
    #[serde(rename = "TransactionID")]
    pub transaction_id: Option<String>,
    pub status: Option<DmnStatus>,
    #[serde(rename = "ErrCode")]
    pub err_code: Option<String>,
    #[serde(rename = "ExErrCode")]
    pub ex_err_code: Option<String>,
    pub desc: Option<String>,
    pub merchant_unique_id: Option<String>,
    pub custom_data: Option<String>,
    pub product_id: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub email: Option<String>,
    pub total_amount: Option<String>,
    pub currency: Option<String>,
    pub fee: Option<String>,
    #[serde(rename = "AuthCode")]
    pub auth_code: Option<String>,
    pub transaction_status: Option<TransactionStatus>,
    pub transaction_type: Option<PaymentTransactionType>,
    #[serde(rename = "user_token_id")]
    pub user_token_id: Option<String>,
    #[serde(rename = "payment_method")]
    pub payment_method: Option<String>,
    #[serde(rename = "responseTimeStamp")]
    pub response_time_stamp: Option<String>,
    #[serde(rename = "invoice_id")]
    pub invoice_id: Option<String>,
    #[serde(rename = "merchant_id")]
    pub merchant_id: Option<String>,
    #[serde(rename = "merchant_site_id")]
    pub merchant_site_id: Option<String>,
    #[serde(rename = "responsechecksum")]
    pub response_checksum: Option<String>,
    #[serde(rename = "advanceResponseChecksum")]
    pub advance_response_checksum: Option<String>,
}

// For backward compatibility with existing code
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NuveiWebhookTransactionId {
    #[serde(rename = "ppp_TransactionID")]
    pub ppp_transaction_id: String,
}

// Helper struct to extract transaction ID from either webhook type
impl From<&NuveiWebhook> for NuveiWebhookTransactionId {
    fn from(webhook: &NuveiWebhook) -> Self {
        match webhook {
            NuveiWebhook::Chargeback(notification) => Self {
                ppp_transaction_id: notification.ppp_transaction_id.clone().unwrap_or_default(),
            },
            NuveiWebhook::PaymentDmn(notification) => Self {
                ppp_transaction_id: notification.ppp_transaction_id.clone().unwrap_or_default(),
            },
        }
    }
}

// Convert webhook to payments response for further processing
impl From<NuveiWebhook> for NuveiPaymentsResponse {
    fn from(webhook: NuveiWebhook) -> Self {
        match webhook {
            NuveiWebhook::Chargeback(notification) => Self {
                transaction_status: Some(NuveiTransactionStatus::Processing),
                transaction_id: notification.transaction_id,
                transaction_type: Some(NuveiTransactionType::Credit), // Using Credit as placeholder for chargeback
                ..Default::default()
            },
            NuveiWebhook::PaymentDmn(notification) => {
                let transaction_type = notification.transaction_type.map(|tt| match tt {
                    PaymentTransactionType::Auth => NuveiTransactionType::Auth,
                    PaymentTransactionType::Sale => NuveiTransactionType::Sale,
                    PaymentTransactionType::Settle => NuveiTransactionType::Settle,
                    PaymentTransactionType::Credit => NuveiTransactionType::Credit,
                    PaymentTransactionType::Void => NuveiTransactionType::Void,
                    PaymentTransactionType::Auth3D => NuveiTransactionType::Auth3D,
                    PaymentTransactionType::Sale3D => NuveiTransactionType::Auth3D, // Map to closest equivalent
                    PaymentTransactionType::Verif => NuveiTransactionType::Auth, // Map to closest equivalent
                });

                Self {
                    transaction_status: notification.transaction_status.map(|ts| match ts {
                        TransactionStatus::Approved => NuveiTransactionStatus::Approved,
                        TransactionStatus::Declined => NuveiTransactionStatus::Declined,
                        TransactionStatus::Error => NuveiTransactionStatus::Error,
                        TransactionStatus::Settled => NuveiTransactionStatus::Approved,
                        _ => NuveiTransactionStatus::Processing,
                    }),
                    transaction_id: notification.transaction_id,
                    transaction_type,
                    ..Default::default()
                }
            }
        }
    }
}

fn get_cvv2_response_description(code: &str) -> Option<&str> {
    match code {
        "M" => Some("CVV2 Match"),
        "N" => Some("CVV2 No Match"),
        "P" => Some("Not Processed. For EU card-on-file (COF) and ecommerce (ECOM) network token transactions, Visa removes any CVV and sends P. If you have fraud or security concerns, Visa recommends using 3DS."),
        "U" => Some("Issuer is not certified and/or has not provided Visa the encryption keys"),
        "S" => Some("CVV2 processor is unavailable."),
        _=> None,
    }
}

fn get_avs_response_description(code: &str) -> Option<&str> {
    match code {
        "A" => Some("The street address matches, the ZIP code does not."),
        "W" => Some("Postal code matches, the street address does not."),
        "Y" => Some("Postal code and the street address match."),
        "X" => Some("An exact match of both the 9-digit ZIP code and the street address."),
        "Z" => Some("Postal code matches, the street code does not."),
        "U" => Some("Issuer is unavailable."),
        "S" => Some("AVS not supported by issuer."),
        "R" => Some("Retry."),
        "B" => Some("Not authorized (declined)."),
        "N" => Some("Both the street address and postal code do not match."),
        _ => None,
    }
}

fn get_merchant_advice_code_description(code: &str) -> Option<&str> {
    match code {
        "01" => Some("New Account Information Available"),
        "02" => Some("Cannot approve at this time, try again later"),
        "03" => Some("Do Not Try Again"),
        "04" => Some("Token requirements not fulfilled for this token type"),
        "21" => Some("Payment Cancellation, do not try again"),
        "24" => Some("Retry after 1 hour"),
        "25" => Some("Retry after 24 hours"),
        "26" => Some("Retry after 2 days"),
        "27" => Some("Retry after 4 days"),
        "28" => Some("Retry after 6 days"),
        "29" => Some("Retry after 8 days"),
        "30" => Some("Retry after 10 days"),
        "40" => Some("Card is a consumer non-reloadable prepaid card"),
        "41" => Some("Card is a consumer single-use virtual card number"),
        "42" => Some("Transaction type exceeds issuer's risk threshold. Please retry with another payment account."),
        "43" => Some("Card is a consumer multi-use virtual card number"),
        _ => None,
    }
}

/// Concatenates a vector of strings without any separator
/// This is useful for creating verification messages for webhooks
pub fn concat_strings(strings: &[String]) -> String {
    strings.join("")
}

fn convert_to_additional_payment_method_connector_response(
    transaction_response: &NuveiPaymentsResponse,
) -> Option<AdditionalPaymentMethodConnectorResponse> {
    let card = transaction_response
        .payment_option
        .as_ref()?
        .card
        .as_ref()?;
    let avs_code = card.avs_code.as_ref();
    let cvv2_code = card.cvv2_reply.as_ref();
    let merchant_advice_code = transaction_response.merchant_advice_code.as_ref();

    let avs_description = avs_code.and_then(|code| get_avs_response_description(code));
    let cvv_description = cvv2_code.and_then(|code| get_cvv2_response_description(code));
    let merchant_advice_description =
        merchant_advice_code.and_then(|code| get_merchant_advice_code_description(code));

    let payment_checks = serde_json::json!({
        "avs_result_code": avs_code,
        "avs_description": avs_description,
        "cvv_2_reply_code": cvv2_code,
        "cvv_2_description": cvv_description,
        "merchant_advice_code": merchant_advice_code,
        "merchant_advice_code_description": merchant_advice_description
    });

    let card_network = card.card_brand.clone();
    let three_ds_data = card
        .three_d
        .clone()
        .map(|three_d| {
            serde_json::to_value(three_d)
                .map_err(|_| errors::ConnectorError::ResponseHandlingFailed)
                .attach_printable("threeDs encoding failed Nuvei")
        })
        .transpose();

    match three_ds_data {
        Ok(authentication_data) => Some(AdditionalPaymentMethodConnectorResponse::Card {
            authentication_data,
            payment_checks: Some(payment_checks),
            card_network,
            domestic_network: None,
        }),
        Err(_) => None,
    }
}
