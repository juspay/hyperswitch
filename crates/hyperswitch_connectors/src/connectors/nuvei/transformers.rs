use common_enums::{enums, CaptureMethod, FutureUsage, GooglePayCardFundingSource, PaymentChannel};
use common_types::{
    payments::{
        ApplePayPaymentData, ApplePayPredecryptData, BillingDescriptor, GPayPredecryptData,
        GpayTokenizationData,
    },
    primitive_wrappers,
};
use common_utils::{
    crypto::{self, GenerateDigest},
    date_time,
    ext_traits::Encode,
    fp_utils,
    id_type::CustomerId,
    pii::{self, Email, IpAddress},
    request::Method,
    types::{FloatMajorUnit, MinorUnit, StringMajorUnit, StringMajorUnitForConnector},
};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    address::Address,
    payment_method_data::{
        self, ApplePayWalletData, BankRedirectData, CardDetailsForNetworkTransactionId,
        GooglePayWalletData, PayLaterData, PaymentMethodData, WalletData,
    },
    router_data::{
        AdditionalPaymentMethodConnectorResponse, ConnectorAuthType, ConnectorResponseData,
        ErrorResponse, L2L3Data, PaymentMethodToken, RouterData,
    },
    router_flow_types::{
        refunds::{Execute, RSync},
        Authorize, Capture, CompleteAuthorize, PSync, PostCaptureVoid, SetupMandate, Void,
    },
    router_request_types::{
        authentication::MessageExtensionAttribute, AuthenticationData, BrowserInformation,
        CompleteAuthorizeData, PaymentsAuthorizeData, ResponseId, SetupMandateRequestData,
    },
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types,
};
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    router_flow_types::payouts::PoFulfill, router_response_types::PayoutsResponseData,
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors::{self},
};
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(feature = "payouts")]
use crate::{types::PayoutsResponseRouterData, utils::PayoutsData as _};
use crate::{
    types::{
        PaymentsPreprocessingResponseRouterData, RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        self, convert_amount, missing_field_err, AddressData, AddressDetailsData,
        BrowserInformationData, CardData, ForeignTryFrom, PaymentsAuthorizeRequestData,
        PaymentsCancelRequestData, PaymentsCompleteAuthorizeRequestData,
        PaymentsPreProcessingRequestData, PaymentsSetupMandateRequestData, RouterData as _,
    },
};

fn to_boolean(string: String) -> bool {
    let str = string.as_str();
    matches!(str, "true" | "yes")
}

// The dimensions of the challenge window for full screen.
const CHALLENGE_WINDOW_SIZE: &str = "05";
// The challenge preference for the challenge flow.
const CHALLENGE_PREFERENCE: &str = "01";

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiThreeDSInitPaymentRequest {
    pub session_token: Secret<String>,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_unique_id: String,
    pub client_request_id: Secret<String>,
    pub amount: StringMajorUnit,
    pub payment_option: CardPaymentOption,
    pub device_details: DeviceDetails,
    pub currency: enums::Currency,
    pub user_token_id: Option<CustomerId>,
    pub billing_address: Option<BillingAddress>,
    pub url_details: UrlDetails,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardPaymentOption {
    pub card: Card,
}

impl TryFrom<(&types::PaymentsPreProcessingRouterData, String)> for NuveiThreeDSInitPaymentRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (item, session_token): (&types::PaymentsPreProcessingRouterData, String),
    ) -> Result<Self, Self::Error> {
        let currency = item.request.get_currency()?;
        let connector_auth: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let amount = item.request.get_minor_amount()?.to_nuvei_amount(currency)?;
        let payment_method_data = item.request.get_payment_method_data()?.clone();
        let card = match payment_method_data {
            PaymentMethodData::Card(card) => card,
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("nuvei"),
            ))?,
        };

        let browser_info = item
            .request
            .browser_info
            .clone()
            .ok_or_else(missing_field_err("browser_info"))?;

        let return_url = item
            .request
            .router_return_url
            .clone()
            .ok_or_else(missing_field_err("return_url"))?;

        let billing_address = item.get_billing().ok().map(|billing| billing.into());

        Ok(Self {
            session_token: session_token.into(),
            merchant_id: connector_auth.merchant_id,
            merchant_site_id: connector_auth.merchant_site_id,
            client_request_id: item.connector_request_reference_id.clone().into(),
            client_unique_id: item.connector_request_reference_id.clone(),
            amount,
            currency,
            payment_option: CardPaymentOption {
                card: Card {
                    card_number: Some(card.card_number),
                    card_holder_name: item.get_optional_billing_full_name(),
                    expiration_month: Some(card.card_exp_month),
                    expiration_year: Some(card.card_exp_year),
                    cvv: Some(card.card_cvc),
                    ..Default::default()
                },
            },
            device_details: DeviceDetails {
                ip_address: browser_info.get_ip_address()?,
            },
            user_token_id: item.customer_id.clone(),
            billing_address,
            url_details: UrlDetails {
                success_url: return_url.clone(),
                failure_url: return_url.clone(),
                pending_url: return_url,
            },
        })
    }
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
#[derive(Debug, Serialize, Default, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiSessionResponse {
    pub session_token: Secret<String>,
    pub status: String,
    pub err_code: i64,
    pub reason: String,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: String,
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
#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentBaseRequest {
    pub time_stamp: String,
    pub session_token: Secret<String>,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: Secret<String>,
    pub client_unique_id: String,
    pub amount: StringMajorUnit,
    pub currency: enums::Currency,
    pub checksum: Secret<String>,
    pub transaction_type: TransactionType,
    pub dynamic_descriptor: Option<NuveiDynamicDescriptor>,
    pub is_partial_approval: Option<PartialApprovalFlag>,
    pub items: Option<Vec<NuveiItem>>,
    pub amount_details: Option<NuveiAmountDetails>,
    pub user_token_id: Option<CustomerId>,
    pub is_rebilling: Option<IsRebilling>,
}

impl<F, Req> TryFrom<(&RouterData<F, Req, PaymentsResponseData>, String)>
    for NuveiPaymentBaseRequest
where
    F: std::fmt::Debug,
    Req: NuveiAuthorizePreprocessingCommon + std::fmt::Debug,
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        value: (&RouterData<F, Req, PaymentsResponseData>, String),
    ) -> Result<Self, Self::Error> {
        let (item, session_token) = value;

        let currency = item.request.get_currency();
        let amount = item
            .request
            .get_minor_amount_required()?
            .to_nuvei_amount(currency)?;

        fp_utils::when(session_token.is_empty(), || {
            Err(errors::ConnectorError::MissingRequiredField {
                field_name: "session_token",
            })
        })?;

        let auth = NuveiAuthType::try_from(&item.connector_auth_type)?;
        let client_request_id = item.connector_request_reference_id.clone();

        let time_stamp =
            date_time::format_date(date_time::now(), date_time::DateFormat::YYYYMMDDHHmmss)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let (is_rebilling, user_token_id) = match (
            item.request.get_payment_method_data_required()?,
            item.request.is_customer_initiated_mandate_payment(),
        ) {
            (PaymentMethodData::Card(_) | PaymentMethodData::Wallet(_), true) => {
                (Some(IsRebilling::False), item.customer_id.clone())
            }
            (
                PaymentMethodData::MandatePayment
                | PaymentMethodData::CardDetailsForNetworkTransactionId(_),
                _,
            ) => (
                Some(IsRebilling::True),
                Some(
                    item.customer_id
                        .clone()
                        .ok_or_else(missing_field_err("customer_id"))?,
                ),
            ),
            _ => (None, None),
        };
        Ok(Self {
            merchant_id: auth.merchant_id.clone(),
            merchant_site_id: auth.merchant_site_id.clone(),
            client_request_id: Secret::new(client_request_id.clone()),
            client_unique_id: client_request_id.clone(),
            time_stamp: time_stamp.clone(),
            session_token: Secret::new(session_token),
            user_token_id,
            is_rebilling,
            amount: amount.clone(),
            currency,
            dynamic_descriptor: item.request.get_dynamic_descriptor()?,
            is_partial_approval: item.request.get_is_partial_approval(),
            items: get_l2_l3_items(&item.l2_l3_data, currency)?,
            amount_details: get_amount_details(&item.l2_l3_data, currency)?,
            transaction_type: TransactionType::get_from_capture_method_and_amount_string(
                item.request.get_capture_method().unwrap_or_default(),
                &amount.get_amount_as_string(),
            ),
            checksum: Secret::new(
                encode_payload(&[
                    auth.merchant_id.peek(),
                    auth.merchant_site_id.peek(),
                    &client_request_id,
                    &amount.get_amount_as_string(),
                    &currency.to_string(),
                    &time_stamp,
                    auth.merchant_secret.peek(),
                ])
                .change_context(errors::ConnectorError::RequestEncodingFailed)?,
            ),
        })
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentsRequest {
    #[serde(flatten)]
    pub base: NuveiPaymentBaseRequest,
    pub payment_option: PaymentOption,
    pub device_details: DeviceDetails,
    pub billing_address: Option<BillingAddress>,
    pub shipping_address: Option<ShippingAddress>,
    pub related_transaction_id: Option<String>,
    pub external_scheme_details: Option<ExternalSchemeDetails>,
    pub is_moto: Option<bool>,
    pub url_details: Option<UrlDetails>,
}

/// Handles payment request for capture, void and refund flows
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentFlowRequest {
    pub time_stamp: String,
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: String,
    pub client_unique_id: String,
    pub amount: StringMajorUnit,
    pub currency: enums::Currency,
    pub related_transaction_id: Option<String>,
    pub checksum: Secret<String>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPaymentSyncRequest {
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_unique_id: String,
    pub time_stamp: String,
    pub checksum: Secret<String>,
    pub transaction_id: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
    pub brand: Option<String>,
    pub issuer_bank_name: Option<String>,
    pub issuer_country: Option<String>,
    pub external_token: Option<ExternalToken>,
    pub stored_credentials: Option<StoredCredentialMode>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalToken {
    pub external_token_provider: ExternalTokenProvider,
    pub mobile_token: Option<Secret<String>>,
    pub cryptogram: Option<Secret<String>>,
    pub eci_provider: Option<String>,
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
pub struct ExternalMpi {
    pub eci: Option<String>,
    pub cavv: Secret<String>,
    #[serde(rename = "dsTransID")]
    pub ds_trans_id: Option<String>,
    pub challenge_preference: Option<String>,
    pub exemption_request_reason: Option<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreeD {
    pub method_completion_ind: Option<MethodCompletion>,
    pub browser_details: Option<BrowserDetails>,
    #[serde(rename = "notificationURL")]
    pub notification_url: Option<String>,
    #[serde(rename = "merchantURL")]
    pub merchant_url: Option<String>,
    pub acs_url: Option<String>,
    pub c_req: Option<Secret<String>>,
    pub external_mpi: Option<ExternalMpi>,
    pub transaction_id: Option<String>,
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
    #[serde(rename = "Y", alias = "1", alias = "y")]
    Success,
    #[serde(rename = "N", alias = "0", alias = "n")]
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

impl From<NuveiPaymentSyncResponse> for NuveiTransactionSyncResponse {
    fn from(value: NuveiPaymentSyncResponse) -> Self {
        match value {
            NuveiPaymentSyncResponse::NuveiDmn(payment_dmn_notification) => {
                Self::from(*payment_dmn_notification)
            }
            NuveiPaymentSyncResponse::NuveiApi(nuvei_transaction_sync_response) => {
                *nuvei_transaction_sync_response
            }
        }
    }
}

#[derive(Debug)]
pub struct NuveiCardDetails {
    card: payment_method_data::Card,
    three_d: Option<ThreeD>,
    card_holder_name: Option<Secret<String>>,
    stored_credentials: Option<StoredCredentialMode>,
}

// Define new structs with camelCase serialization
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GooglePayCamelCase {
    pm_type: Secret<String>,
    description: Secret<String>,
    info: GooglePayInfoCamelCase,
    tokenization_data: GooglePayTokenizationDataCamelCase,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GooglePayInfoCamelCase {
    card_network: Secret<String>,
    card_details: Secret<String>,
    assurance_details: Option<GooglePayAssuranceDetailsCamelCase>,
    card_funding_source: Option<GooglePayCardFundingSource>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExternalSchemeDetails {
    transaction_id: Secret<String>, // This is sensitive information
    brand: Option<NuveiCardType>,
}
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GooglePayAssuranceDetailsCamelCase {
    card_holder_authenticated: bool,
    account_verified: bool,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct GooglePayTokenizationDataCamelCase {
    #[serde(rename = "type")]
    token_type: Secret<String>,
    token: Secret<String>,
}

// Define ApplePay structs with camelCase serialization
#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApplePayCamelCase {
    payment_data: Secret<String>,
    payment_method: ApplePayPaymentMethodCamelCase,
    transaction_identifier: Secret<String>,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ApplePayPaymentMethodCamelCase {
    display_name: Secret<String>,
    network: Secret<String>,
    #[serde(rename = "type")]
    pm_type: Secret<String>,
}

fn get_google_pay_decrypt_data(
    predecrypt_data: &GPayPredecryptData,
    brand: Option<String>,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    Ok(NuveiPaymentsRequest {
        payment_option: PaymentOption {
            card: Some(Card {
                brand,
                card_number: Some(predecrypt_data.application_primary_account_number.clone()),
                last4_digits: Some(Secret::new(
                    predecrypt_data
                        .application_primary_account_number
                        .clone()
                        .get_last4(),
                )),
                expiration_month: Some(predecrypt_data.card_exp_month.clone()),
                expiration_year: Some(predecrypt_data.card_exp_year.clone()),
                external_token: Some(ExternalToken {
                    external_token_provider: ExternalTokenProvider::GooglePay,
                    mobile_token: None,
                    cryptogram: predecrypt_data.cryptogram.clone(),
                    eci_provider: predecrypt_data.eci_indicator.clone(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn get_googlepay_info<F, Req>(
    item: &RouterData<F, Req, PaymentsResponseData>,
    gpay_data: &GooglePayWalletData,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>>
where
    Req: NuveiAuthorizePreprocessingCommon,
{
    if let Ok(PaymentMethodToken::GooglePayDecrypt(ref token)) = item.get_payment_method_token() {
        return get_google_pay_decrypt_data(token, Some(gpay_data.info.card_network.clone()));
    }

    match &gpay_data.tokenization_data {
        GpayTokenizationData::Decrypted(gpay_predecrypt_data) => get_google_pay_decrypt_data(
            gpay_predecrypt_data,
            Some(gpay_data.info.card_network.clone()),
        ),
        GpayTokenizationData::Encrypted(ref encrypted_data) => Ok(NuveiPaymentsRequest {
            payment_option: PaymentOption {
                card: Some(Card {
                    external_token: Some(ExternalToken {
                        external_token_provider: ExternalTokenProvider::GooglePay,

                        mobile_token: {
                            let (token_type, token) = (
                                encrypted_data.token_type.clone(),
                                encrypted_data.token.clone(),
                            );

                            let google_pay: GooglePayCamelCase = GooglePayCamelCase {
                                pm_type: Secret::new(gpay_data.pm_type.clone()),
                                description: Secret::new(gpay_data.description.clone()),
                                info: GooglePayInfoCamelCase {
                                    card_network: Secret::new(gpay_data.info.card_network.clone()),
                                    card_details: Secret::new(gpay_data.info.card_details.clone()),
                                    assurance_details: gpay_data
                                        .info
                                        .assurance_details
                                        .as_ref()
                                        .map(|details| GooglePayAssuranceDetailsCamelCase {
                                            card_holder_authenticated: details
                                                .card_holder_authenticated,
                                            account_verified: details.account_verified,
                                        }),
                                    card_funding_source: gpay_data.info.card_funding_source.clone(),
                                },
                                tokenization_data: GooglePayTokenizationDataCamelCase {
                                    token_type: token_type.into(),
                                    token: token.into(),
                                },
                            };
                            Some(Secret::new(
                                google_pay.encode_to_string_of_json().change_context(
                                    errors::ConnectorError::RequestEncodingFailed,
                                )?,
                            ))
                        },
                        cryptogram: None,
                        eci_provider: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
    }
}

fn get_apple_pay_decrypt_data(
    apple_pay_predecrypt_data: &ApplePayPredecryptData,
    network: String,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    Ok(NuveiPaymentsRequest {
        payment_option: PaymentOption {
            card: Some(Card {
                brand: Some(network),
                card_number: Some(
                    apple_pay_predecrypt_data
                        .application_primary_account_number
                        .clone(),
                ),
                last4_digits: Some(Secret::new(
                    apple_pay_predecrypt_data
                        .application_primary_account_number
                        .get_last4(),
                )),
                expiration_month: Some(
                    apple_pay_predecrypt_data
                        .application_expiration_month
                        .clone(),
                ),
                expiration_year: Some(
                    apple_pay_predecrypt_data
                        .application_expiration_year
                        .clone(),
                ),
                external_token: Some(ExternalToken {
                    external_token_provider: ExternalTokenProvider::ApplePay,
                    mobile_token: None,
                    cryptogram: Some(
                        apple_pay_predecrypt_data
                            .payment_data
                            .online_payment_cryptogram
                            .clone(),
                    ),
                    eci_provider: apple_pay_predecrypt_data.payment_data.eci_indicator.clone(),
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    })
}
fn get_applepay_info<F, Req>(
    item: &RouterData<F, Req, PaymentsResponseData>,
    apple_pay_data: &ApplePayWalletData,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>>
where
    Req: NuveiAuthorizePreprocessingCommon,
{
    if let Ok(PaymentMethodToken::ApplePayDecrypt(ref token)) = item.get_payment_method_token() {
        return get_apple_pay_decrypt_data(token, apple_pay_data.payment_method.network.clone());
    }
    match apple_pay_data.payment_data {
        ApplePayPaymentData::Decrypted(ref apple_pay_predecrypt_data) => {
            get_apple_pay_decrypt_data(
                apple_pay_predecrypt_data,
                apple_pay_data.payment_method.network.clone(),
            )
        }

        ApplePayPaymentData::Encrypted(ref encrypted_data) => Ok(NuveiPaymentsRequest {
            payment_option: PaymentOption {
                card: Some(Card {
                    external_token: Some(ExternalToken {
                        external_token_provider: ExternalTokenProvider::ApplePay,
                        mobile_token: {
                            let apple_pay: ApplePayCamelCase = ApplePayCamelCase {
                                payment_data: encrypted_data.to_string().into(),
                                payment_method: ApplePayPaymentMethodCamelCase {
                                    display_name: Secret::new(
                                        apple_pay_data.payment_method.display_name.clone(),
                                    ),
                                    network: Secret::new(
                                        apple_pay_data.payment_method.network.clone(),
                                    ),
                                    pm_type: Secret::new(
                                        apple_pay_data.payment_method.pm_type.clone(),
                                    ),
                                },
                                transaction_identifier: Secret::new(
                                    apple_pay_data.transaction_identifier.clone(),
                                ),
                            };

                            Some(Secret::new(
                                apple_pay.encode_to_string_of_json().change_context(
                                    errors::ConnectorError::RequestEncodingFailed,
                                )?,
                            ))
                        },
                        cryptogram: None,
                        eci_provider: None,
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }),
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

            _ => Err(errors::ConnectorError::NotImplemented(
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
        let bank_id = match (&payment_method, redirect) {
            (AlternativePaymentMethodType::Expresscheckout, _) => None,
            (AlternativePaymentMethodType::Giropay, _) => None,
            (AlternativePaymentMethodType::Sofort, _) | (AlternativePaymentMethodType::Eps, _) => {
                None
            }
            (
                AlternativePaymentMethodType::Ideal,
                Some(BankRedirectData::Ideal { bank_name, .. }),
            ) => bank_name.map(NuveiBIC::try_from).transpose()?,
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
            billing_address: item.get_billing().ok().map(|billing| billing.into()),
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
    address.get_first_name()?;
    address.get_country()?; //country is necessary check
    item.request.get_email_required()?;
    Ok(NuveiPaymentsRequest {
        payment_option: PaymentOption {
            alternative_payment_method: Some(AlternativePaymentMethod {
                payment_method: payment_method_type,
                ..Default::default()
            }),
            ..Default::default()
        },
        ..Default::default()
    })
}

fn get_ntid_card_info<F, Req>(
    router_data: &RouterData<F, Req, PaymentsResponseData>,
    data: CardDetailsForNetworkTransactionId,
) -> Result<NuveiPaymentsRequest, error_stack::Report<errors::ConnectorError>>
where
    Req: NuveiAuthorizePreprocessingCommon,
{
    let card_type = match data.card_network.clone() {
        Some(card_type) => NuveiCardType::try_from(card_type)?,
        None => NuveiCardType::try_from(&data.get_card_issuer()?)?,
    };

    let external_scheme_details = Some(ExternalSchemeDetails {
        transaction_id: router_data
            .request
            .get_ntid()
            .ok_or_else(missing_field_err("network_transaction_id"))
            .attach_printable("Nuvei unable to find NTID for MIT")?
            .into(),
        brand: Some(card_type),
    });
    let payment_option: PaymentOption = PaymentOption {
        card: Some(Card {
            card_number: Some(data.card_number),
            card_holder_name: data.card_holder_name,
            expiration_month: Some(data.card_exp_month),
            expiration_year: Some(data.card_exp_year),
            ..Default::default() // CVV should be disabled by nuvei
        }),
        ..Default::default()
    };
    Ok(NuveiPaymentsRequest {
        external_scheme_details,
        payment_option,
        ..Default::default()
    })
}

impl<F, Req> TryFrom<(&RouterData<F, Req, PaymentsResponseData>, String)> for NuveiPaymentsRequest
where
    Req: NuveiAuthorizePreprocessingCommon + std::fmt::Debug,
    F: std::fmt::Debug,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (&RouterData<F, Req, PaymentsResponseData>, String),
    ) -> Result<Self, Self::Error> {
        let (item, session_token) = data;
        let base = NuveiPaymentBaseRequest::try_from((item, session_token))?;

        let return_url = item.request.get_return_url_required()?;
        let address = {
            let mut billing_address = item.get_billing()?.clone();
            billing_address.email = Some(
                item.get_billing_email()
                    .or_else(|_| item.request.get_email_required())?,
            );
            Some(billing_address)
        };
        let request_data = match item.request.get_payment_method_data_required()?.clone() {
            PaymentMethodData::Card(card) => get_card_info(item, &card),
            PaymentMethodData::MandatePayment => Self::try_from(item),
            PaymentMethodData::CardDetailsForNetworkTransactionId(data) => {
                get_ntid_card_info(item, data)
            }
            PaymentMethodData::Wallet(wallet) => match wallet {
                WalletData::GooglePay(gpay_data) => get_googlepay_info(item, &gpay_data),
                WalletData::ApplePay(apple_pay_data) => get_applepay_info(item, &apple_pay_data),
                WalletData::PaypalRedirect(_) => Self::foreign_try_from((
                    AlternativePaymentMethodType::Expresscheckout,
                    None,
                    item,
                )),
                _ => Err(errors::ConnectorError::NotImplemented(
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
                _ => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("nuvei"),
                )
                .into()),
            },
            PaymentMethodData::PayLater(pay_later_data) => match pay_later_data {
                PayLaterData::KlarnaRedirect { .. } => {
                    get_pay_later_info(AlternativePaymentMethodType::Klarna, item)
                }
                PayLaterData::AfterpayClearpayRedirect { .. } => {
                    get_pay_later_info(AlternativePaymentMethodType::AfterPay, item)
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("nuvei"),
                )
                .into()),
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("nuvei"),
            )
            .into()),
        }?;
        let device_details = if request_data
            .device_details
            .ip_address
            .clone()
            .expose()
            .is_empty()
        {
            DeviceDetails::foreign_try_from(&item.request.get_browser_info().clone())?
        } else {
            request_data.device_details.clone()
        };
        Ok(Self {
            base,
            related_transaction_id: request_data.related_transaction_id,
            payment_option: request_data.payment_option,
            billing_address: address.clone().map(|ref address| address.into()),
            shipping_address: item.get_optional_shipping().map(|address| address.into()),
            device_details,
            external_scheme_details: request_data.external_scheme_details,
            url_details: Some(UrlDetails {
                success_url: return_url.clone(),
                failure_url: return_url.clone(),
                pending_url: return_url,
            }),
            ..request_data
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
    let additional_params = match item.request.is_customer_initiated_mandate_payment() {
        true => Some(V2AdditionalParams {
            rebill_expiry: Some(
                time::OffsetDateTime::now_utc()
                    .replace_year(time::OffsetDateTime::now_utc().year() + 5)
                    .map_err(|_| errors::ConnectorError::DateFormattingFailed)?
                    .date()
                    .format(&time::macros::format_description!("[year][month][day]"))
                    .map_err(|_| errors::ConnectorError::DateFormattingFailed)?,
            ),
            rebill_frequency: Some("0".to_string()),
            challenge_window_size: Some(CHALLENGE_WINDOW_SIZE.to_string()),
            challenge_preference: Some(CHALLENGE_PREFERENCE.to_string()),
        }),
        // non mandate transactions
        false => Some(V2AdditionalParams {
            rebill_expiry: None,
            rebill_frequency: None,
            challenge_window_size: Some(CHALLENGE_WINDOW_SIZE.to_string()),
            challenge_preference: Some(CHALLENGE_PREFERENCE.to_string()),
        }),
    };
    let three_d = if let Some(auth_data) = item.request.get_auth_data()? {
        Some(ThreeD {
            external_mpi: Some(ExternalMpi {
                eci: auth_data.eci,
                cavv: auth_data.cavv,
                ds_trans_id: auth_data.ds_trans_id,
                challenge_preference: None,
                exemption_request_reason: None,
            }),
            ..Default::default()
        })
    } else if item.is_three_ds() {
        Some(ThreeD {
            browser_details: item
                .request
                .get_browser_info()
                .map(BrowserDetails::try_from)
                .transpose()?,
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
    Ok(NuveiPaymentsRequest {
        related_transaction_id: item.request.get_related_transaction_id().clone(),
        device_details: DeviceDetails::foreign_try_from(&item.request.get_browser_info().clone())?,
        payment_option: PaymentOption::from(NuveiCardDetails {
            card: card_details.clone(),
            three_d,
            card_holder_name: item.get_optional_billing_full_name(),
            stored_credentials: item.request.get_is_stored_credential(),
        }),
        is_moto: item.request.get_is_moto(),
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
                stored_credentials: card_details.stored_credentials,
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
        let (item, session_token) = data;
        let request_data = match item.request.payment_method_data.clone() {
            Some(PaymentMethodData::Card(card)) => Ok(Self {
                payment_option: PaymentOption::from(NuveiCardDetails {
                    card,
                    three_d: None,
                    card_holder_name: item.get_optional_billing_full_name(),
                    stored_credentials: StoredCredentialMode::get_optional_stored_credential(
                        item.request.is_stored_credential,
                    ),
                }),
                device_details: DeviceDetails::foreign_try_from(&item.request.browser_info)?,
                ..Default::default()
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                utils::get_unimplemented_payment_method_error_message("nuvei"),
            )),
        }?;
        Ok(Self {
            related_transaction_id: item.request.connector_transaction_id.clone(),
            payment_option: request_data.payment_option,
            device_details: request_data.device_details,
            base: NuveiPaymentBaseRequest::try_from((item, session_token.peek().to_string()))?,
            ..request_data
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
            client_unique_id: client_request_id.clone(),
            time_stamp: time_stamp.clone(),
            checksum: Secret::new(encode_payload(&[
                merchant_id.peek(),
                merchant_site_id.peek(),
                &client_request_id,
                &client_request_id.clone(),
                &request.amount.get_amount_as_string(),
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
    pub amount: StringMajorUnit,
    pub currency: enums::Currency,
    pub related_transaction_id: Option<String>,
    pub client_request_id: String,
    pub client_unique_id: String,
    pub connector_auth_type: ConnectorAuthType,
    pub session_token: Secret<String>,
    pub capture_method: Option<CaptureMethod>,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for NuveiPaymentFlowRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Self::try_from(NuveiPaymentRequestData {
            client_request_id: item.connector_request_reference_id.clone(),
            client_unique_id: item.connector_request_reference_id.clone(),
            connector_auth_type: item.connector_auth_type.clone(),
            amount: item
                .request
                .minor_amount_to_capture
                .to_nuvei_amount(item.request.currency)?,
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
            client_unique_id: item.connector_request_reference_id.clone(),
            connector_auth_type: item.connector_auth_type.clone(),
            amount: item
                .request
                .minor_refund_amount
                .to_nuvei_amount(item.request.currency)?,
            currency: item.request.currency,
            related_transaction_id: Some(item.request.connector_transaction_id.clone()),
            ..Default::default()
        })
    }
}

impl TryFrom<&types::PaymentsSyncRouterData> for NuveiPaymentSyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(value: &types::PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let connector_meta: NuveiAuthType = NuveiAuthType::try_from(&value.connector_auth_type)?;
        let merchant_id = connector_meta.merchant_id.clone();
        let merchant_site_id = connector_meta.merchant_site_id.clone();
        let merchant_secret = connector_meta.merchant_secret.clone();
        let time_stamp =
            date_time::format_date(date_time::now(), date_time::DateFormat::YYYYMMDDHHmmss)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let client_unique_id = value.connector_request_reference_id.clone();
        let transaction_id = value
            .request
            .connector_transaction_id
            .clone()
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;
        let checksum = Secret::new(encode_payload(&[
            merchant_id.peek(),
            merchant_site_id.peek(),
            &transaction_id,
            &client_unique_id,
            &time_stamp,
            merchant_secret.peek(),
        ])?);

        Ok(Self {
            merchant_id,
            merchant_site_id,
            client_unique_id,
            time_stamp,
            checksum,
            transaction_id,
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
            client_unique_id: item.connector_request_reference_id.clone(),
            connector_auth_type: item.connector_auth_type.clone(),
            amount: item
                .request
                .minor_amount
                .ok_or_else(missing_field_err("amount"))?
                .to_nuvei_amount(item.request.get_currency()?)?,
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

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPayoutRequest {
    pub merchant_id: Secret<String>,
    pub merchant_site_id: Secret<String>,
    pub client_request_id: String,
    pub client_unique_id: String,
    pub amount: StringMajorUnit,
    pub currency: String,
    pub user_token_id: CustomerId,
    pub time_stamp: String,
    pub checksum: Secret<String>,
    pub card_data: NuveiPayoutCardData,
    pub url_details: NuveiPayoutUrlDetails,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPayoutUrlDetails {
    pub notification_url: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPayoutCardData {
    pub card_number: cards::CardNumber,
    pub card_holder_name: Secret<String>,
    pub expiration_month: Secret<String>,
    pub expiration_year: Secret<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NuveiPayoutResponse {
    NuveiPayoutSuccessResponse(NuveiPayoutSuccessResponse),
    NuveiPayoutErrorResponse(NuveiPayoutErrorResponse),
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPayoutSuccessResponse {
    pub transaction_id: String,
    pub user_token_id: CustomerId,
    pub transaction_status: NuveiTransactionStatus,
    pub client_unique_id: String,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPayoutErrorResponse {
    pub status: NuveiPaymentStatus,
    pub err_code: i64,
    pub reason: Option<String>,
}

#[cfg(feature = "payouts")]
impl TryFrom<api_models::payouts::PayoutMethodData> for NuveiPayoutCardData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        payout_method_data: api_models::payouts::PayoutMethodData,
    ) -> Result<Self, Self::Error> {
        match payout_method_data {
            api_models::payouts::PayoutMethodData::Card(card_data) => Ok(Self {
                card_number: card_data.card_number,
                card_holder_name: card_data.card_holder_name.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "customer_id",
                    },
                )?,
                expiration_month: card_data.expiry_month,
                expiration_year: card_data.expiry_year,
            }),
            api_models::payouts::PayoutMethodData::Bank(_)
            | api_models::payouts::PayoutMethodData::Wallet(_)
            | api_models::payouts::PayoutMethodData::BankRedirect(_)
            | api_models::payouts::PayoutMethodData::Passthrough(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "Selected Payout Method is not implemented for Nuvei".to_string(),
                )
                .into())
            }
        }
    }
}

#[cfg(feature = "payouts")]
impl<F> TryFrom<&types::PayoutsRouterData<F>> for NuveiPayoutRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PayoutsRouterData<F>) -> Result<Self, Self::Error> {
        let connector_auth: NuveiAuthType = NuveiAuthType::try_from(&item.connector_auth_type)?;

        let amount = item
            .request
            .minor_amount
            .to_nuvei_amount(item.request.destination_currency)?;

        let time_stamp =
            date_time::format_date(date_time::now(), date_time::DateFormat::YYYYMMDDHHmmss)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

        let checksum = encode_payload(&[
            connector_auth.merchant_id.peek(),
            connector_auth.merchant_site_id.peek(),
            &item.connector_request_reference_id,
            &amount.get_amount_as_string(),
            &item.request.destination_currency.to_string(),
            &time_stamp,
            connector_auth.merchant_secret.peek(),
        ])?;

        let payout_method_data = item.get_payout_method_data()?;

        let card_data = NuveiPayoutCardData::try_from(payout_method_data)?;

        let customer_details = item.request.get_customer_details()?;

        let url_details = NuveiPayoutUrlDetails {
            notification_url: item.request.get_webhook_url()?,
        };

        Ok(Self {
            merchant_id: connector_auth.merchant_id,
            merchant_site_id: connector_auth.merchant_site_id,
            client_request_id: item.connector_request_reference_id.clone(),
            client_unique_id: item.connector_request_reference_id.clone(),
            amount,
            currency: item.request.destination_currency.to_string(),
            user_token_id: customer_details.customer_id.clone().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "customer_id",
                },
            )?,
            time_stamp,
            checksum: Secret::new(checksum),
            card_data,
            url_details,
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<PayoutsResponseRouterData<PoFulfill, NuveiPayoutResponse>>
    for types::PayoutsRouterData<PoFulfill>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<PoFulfill, NuveiPayoutResponse>,
    ) -> Result<Self, Self::Error> {
        let response = &item.response;

        match response {
            NuveiPayoutResponse::NuveiPayoutSuccessResponse(response_data) => Ok(Self {
                response: Ok(PayoutsResponseData {
                    status: Some(enums::PayoutStatus::from(
                        response_data.transaction_status.clone(),
                    )),
                    connector_payout_id: Some(response_data.transaction_id.clone()),
                    payout_eligible: None,
                    should_add_next_step_to_process_tracker: false,
                    error_code: None,
                    error_message: None,
                    payout_connector_metadata: None,
                }),
                ..item.data
            }),
            NuveiPayoutResponse::NuveiPayoutErrorResponse(error_response_data) => Ok(Self {
                response: Ok(PayoutsResponseData {
                    status: Some(enums::PayoutStatus::from(
                        error_response_data.status.clone(),
                    )),
                    connector_payout_id: None,
                    payout_eligible: None,
                    should_add_next_step_to_process_tracker: false,
                    error_code: Some(error_response_data.err_code.to_string()),
                    error_message: error_response_data.reason.clone(),
                    payout_connector_metadata: None,
                }),
                ..item.data
            }),
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
    #[serde(alias = "Approved", alias = "APPROVED")]
    Approved,
    #[serde(alias = "Declined", alias = "DECLINED")]
    Declined,
    #[serde(alias = "Filter Error", alias = "ERROR", alias = "Error")]
    Error,
    #[serde(alias = "Redirect", alias = "REDIRECT")]
    Redirect,
    #[serde(alias = "Pending", alias = "PENDING")]
    Pending,
    #[serde(alias = "Processing", alias = "PROCESSING")]
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

#[cfg(feature = "payouts")]
impl From<NuveiTransactionStatus> for enums::PayoutStatus {
    fn from(item: NuveiTransactionStatus) -> Self {
        match item {
            NuveiTransactionStatus::Approved => Self::Success,
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => Self::Failed,
            NuveiTransactionStatus::Processing | NuveiTransactionStatus::Pending => Self::Pending,
            NuveiTransactionStatus::Redirect => Self::Ineligible,
        }
    }
}

#[cfg(feature = "payouts")]
impl From<NuveiPaymentStatus> for enums::PayoutStatus {
    fn from(item: NuveiPaymentStatus) -> Self {
        match item {
            NuveiPaymentStatus::Success => Self::Success,
            NuveiPaymentStatus::Failed | NuveiPaymentStatus::Error => Self::Failed,
            NuveiPaymentStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiPartialApproval {
    pub requested_amount: StringMajorUnit,
    pub requested_currency: enums::Currency,
    pub processed_amount: StringMajorUnit,
    pub processed_currency: enums::Currency,
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
    pub auth_code: Option<String>,
    // NTID
    pub external_scheme_transaction_id: Option<Secret<String>>,
    pub session_token: Option<Secret<String>>,
    pub partial_approval: Option<NuveiPartialApproval>,
    //The ID of the transaction in the merchant’s system.
    pub client_unique_id: Option<String>,
    pub status: NuveiPaymentStatus,
    pub err_code: Option<i64>,
    pub reason: Option<String>,
    pub merchant_id: Option<Secret<String>>,
    pub merchant_site_id: Option<Secret<String>>,
    pub client_request_id: Option<String>,
    pub merchant_advice_code: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiTxnPartialApproval {
    requested_amount: Option<StringMajorUnit>,
    requested_currency: Option<enums::Currency>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiTransactionSyncResponseDetails {
    gw_error_code: Option<i64>,
    gw_error_reason: Option<String>,
    gw_extended_error_code: Option<i64>,
    transaction_id: Option<String>,
    // Status of the payment
    transaction_status: Option<NuveiTransactionStatus>,
    transaction_type: Option<NuveiTransactionType>,
    auth_code: Option<String>,
    processed_amount: Option<StringMajorUnit>,
    processed_currency: Option<enums::Currency>,
    acquiring_bank_name: Option<String>,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NuveiTransactionSyncResponse {
    pub payment_option: Option<PaymentOption>,
    pub partial_approval: Option<NuveiTxnPartialApproval>,
    pub transaction_details: Option<NuveiTransactionSyncResponseDetails>,
    pub client_unique_id: Option<String>,
    // API response status
    pub status: NuveiPaymentStatus,
    pub err_code: Option<i64>,
    pub reason: Option<String>,
    pub merchant_id: Option<Secret<String>>,
    pub merchant_site_id: Option<Secret<String>>,
    pub client_request_id: Option<String>,
    pub merchant_advice_code: Option<String>,
}
impl NuveiTransactionSyncResponse {
    pub fn get_partial_approval(&self) -> Option<NuveiPartialApproval> {
        match &self.partial_approval {
            Some(partial_approval) => match (
                partial_approval.requested_amount.clone(),
                partial_approval.requested_currency,
                self.transaction_details
                    .as_ref()
                    .and_then(|txn| txn.processed_amount.clone()),
                self.transaction_details
                    .as_ref()
                    .and_then(|txn| txn.processed_currency),
            ) {
                (
                    Some(requested_amount),
                    Some(requested_currency),
                    Some(processed_amount),
                    Some(processed_currency),
                ) => Some(NuveiPartialApproval {
                    requested_amount,
                    requested_currency,
                    processed_amount,
                    processed_currency,
                }),
                _ => None,
            },
            None => None,
        }
    }
}

pub fn get_amount_captured(
    partial_approval_data: Option<NuveiPartialApproval>,
    transaction_type: Option<NuveiTransactionType>,
) -> Result<(Option<i64>, Option<MinorUnit>), error_stack::Report<errors::ConnectorError>> {
    match partial_approval_data {
        Some(partial_approval) => {
            let amount = utils::convert_back_amount_to_minor_units(
                &StringMajorUnitForConnector,
                partial_approval.processed_amount.clone(),
                partial_approval.processed_currency,
            )?;
            match transaction_type {
                None => Ok((None, None)),
                Some(NuveiTransactionType::Sale) => {
                    Ok((Some(MinorUnit::get_amount_as_i64(amount)), None))
                }
                Some(NuveiTransactionType::Auth) => Ok((None, Some(amount))),
                Some(NuveiTransactionType::Auth3D) => {
                    Ok((Some(MinorUnit::get_amount_as_i64(amount)), None))
                }
                Some(NuveiTransactionType::InitAuth3D) => Ok((None, Some(amount))),
                Some(NuveiTransactionType::Credit) => Ok((None, None)),
                Some(NuveiTransactionType::Void) => Ok((None, None)),
                Some(NuveiTransactionType::Settle) => Ok((None, None)),
            }
        }
        None => Ok((None, None)),
    }
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

fn get_payment_status(
    amount: Option<i64>,
    is_post_capture_void: bool,
    transaction_type: Option<NuveiTransactionType>,
    transaction_status: Option<NuveiTransactionStatus>,
    status: NuveiPaymentStatus,
) -> enums::AttemptStatus {
    // ZERO dollar authorization
    if amount == Some(0) && transaction_type == Some(NuveiTransactionType::Auth) {
        return match transaction_status {
            Some(NuveiTransactionStatus::Approved) => enums::AttemptStatus::Charged,
            Some(NuveiTransactionStatus::Declined) | Some(NuveiTransactionStatus::Error) => {
                enums::AttemptStatus::AuthorizationFailed
            }
            Some(NuveiTransactionStatus::Pending) | Some(NuveiTransactionStatus::Processing) => {
                enums::AttemptStatus::Pending
            }
            Some(NuveiTransactionStatus::Redirect) => enums::AttemptStatus::AuthenticationPending,
            None => match status {
                NuveiPaymentStatus::Failed | NuveiPaymentStatus::Error => {
                    enums::AttemptStatus::Failure
                }
                _ => enums::AttemptStatus::Pending,
            },
        };
    }

    match transaction_status {
        Some(status) => match status {
            NuveiTransactionStatus::Approved => match transaction_type {
                Some(NuveiTransactionType::InitAuth3D) | Some(NuveiTransactionType::Auth) => {
                    enums::AttemptStatus::Authorized
                }
                Some(NuveiTransactionType::Sale) | Some(NuveiTransactionType::Settle) => {
                    enums::AttemptStatus::Charged
                }
                Some(NuveiTransactionType::Void) if is_post_capture_void => {
                    enums::AttemptStatus::VoidedPostCharge
                }
                Some(NuveiTransactionType::Void) => enums::AttemptStatus::Voided,
                Some(NuveiTransactionType::Auth3D) => enums::AttemptStatus::AuthenticationPending,
                _ => enums::AttemptStatus::Pending,
            },
            NuveiTransactionStatus::Declined | NuveiTransactionStatus::Error => {
                match transaction_type {
                    Some(NuveiTransactionType::Auth) => enums::AttemptStatus::AuthorizationFailed,
                    Some(NuveiTransactionType::Void) => enums::AttemptStatus::VoidFailed,
                    Some(NuveiTransactionType::Auth3D) | Some(NuveiTransactionType::InitAuth3D) => {
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
        None => match status {
            NuveiPaymentStatus::Failed | NuveiPaymentStatus::Error => enums::AttemptStatus::Failure,
            _ => enums::AttemptStatus::Pending,
        },
    }
}

#[derive(Debug)]
struct ErrorResponseParams {
    http_code: u16,
    status: NuveiPaymentStatus,
    err_code: Option<i64>,
    err_msg: Option<String>,
    merchant_advice_code: Option<String>,
    gw_error_code: Option<i64>,
    gw_error_reason: Option<String>,
    transaction_status: Option<NuveiTransactionStatus>,
    transaction_id: Option<String>,
}

fn build_error_response(params: ErrorResponseParams) -> Option<ErrorResponse> {
    match params.status {
        NuveiPaymentStatus::Error => Some(get_error_response(
            params.err_code,
            params.err_msg.clone(),
            params.http_code,
            params.merchant_advice_code.clone(),
            params.gw_error_code.map(|code| code.to_string()),
            params.gw_error_reason.clone(),
            params.transaction_id.clone(),
        )),

        _ => {
            let err = Some(get_error_response(
                params.gw_error_code,
                params.gw_error_reason.clone(),
                params.http_code,
                params.merchant_advice_code,
                params.gw_error_code.map(|e| e.to_string()),
                params.gw_error_reason.clone(),
                params.transaction_id.clone(),
            ));

            match params.transaction_status {
                Some(NuveiTransactionStatus::Error) | Some(NuveiTransactionStatus::Declined) => err,
                _ => match params
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

pub trait NuveiPaymentsGenericResponse {
    fn is_post_capture_void() -> bool {
        false
    }
}

impl NuveiPaymentsGenericResponse for CompleteAuthorize {}
impl NuveiPaymentsGenericResponse for Void {}
impl NuveiPaymentsGenericResponse for PSync {}
impl NuveiPaymentsGenericResponse for Capture {}
impl NuveiPaymentsGenericResponse for PostCaptureVoid {
    fn is_post_capture_void() -> bool {
        true
    }
}

impl
    TryFrom<
        ResponseRouterData<
            SetupMandate,
            NuveiPaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    > for RouterData<SetupMandate, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            SetupMandate,
            NuveiPaymentsResponse,
            SetupMandateRequestData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        let amount = item.data.request.amount;
        let response = &item.response;
        let (status, redirection_data, connector_response_data) = process_nuvei_payment_response(
            NuveiPaymentResponseData::new(amount, false, item.data.payment_method, response),
        )?;

        let (amount_captured, minor_amount_capturable) = get_amount_captured(
            response.partial_approval.clone(),
            response.transaction_type.clone(),
        )?;

        let ip_address = item
            .data
            .request
            .browser_info
            .as_ref()
            .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                field_name: "browser_info",
            })?
            .ip_address
            .as_ref()
            .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                field_name: "browser_info.ip_address",
            })?
            .to_string();
        let response = &item.response;

        Ok(Self {
            status,
            response: if let Some(err) = build_error_response(ErrorResponseParams {
                http_code: item.http_code,
                status: response.status.clone(),
                err_code: response.err_code,
                err_msg: response.reason.clone(),
                merchant_advice_code: response.merchant_advice_code.clone(),
                gw_error_code: response.gw_error_code,
                gw_error_reason: response.gw_error_reason.clone(),
                transaction_status: response.transaction_status.clone(),
                transaction_id: response.transaction_id.clone(),
            }) {
                Err(err)
            } else {
                let response = &item.response;
                Ok(create_transaction_response(
                    redirection_data,
                    Some(ip_address),
                    response.transaction_id.clone(),
                    response.order_id.clone(),
                    response.session_token.clone(),
                    response.external_scheme_transaction_id.clone(),
                    response.payment_option.clone(),
                )?)
            },
            amount_captured,
            minor_amount_capturable,
            connector_response: connector_response_data,
            ..item.data
        })
    }
}

// Helper function to process Nuvei payment response

/// Struct to encapsulate parameters for processing Nuvei payment responses
#[derive(Debug)]
pub struct NuveiPaymentResponseData {
    pub amount: Option<i64>,
    pub is_post_capture_void: bool,
    pub payment_method: enums::PaymentMethod,
    pub payment_option: Option<PaymentOption>,
    pub transaction_type: Option<NuveiTransactionType>,
    pub transaction_status: Option<NuveiTransactionStatus>,
    pub status: NuveiPaymentStatus,
    pub merchant_advice_code: Option<String>,
}

impl NuveiPaymentResponseData {
    pub fn new(
        amount: Option<i64>,
        is_post_capture_void: bool,
        payment_method: enums::PaymentMethod,
        response: &NuveiPaymentsResponse,
    ) -> Self {
        Self {
            amount,
            is_post_capture_void,
            payment_method,
            payment_option: response.payment_option.clone(),
            transaction_type: response.transaction_type.clone(),
            transaction_status: response.transaction_status.clone(),
            status: response.status.clone(),
            merchant_advice_code: response.merchant_advice_code.clone(),
        }
    }

    pub fn new_from_sync_response(
        amount: Option<i64>,
        is_post_capture_void: bool,
        payment_method: enums::PaymentMethod,
        response: &NuveiTransactionSyncResponse,
    ) -> Self {
        let transaction_details = &response.transaction_details;
        Self {
            amount,
            is_post_capture_void,
            payment_method,
            payment_option: response.payment_option.clone(),
            transaction_type: transaction_details
                .as_ref()
                .and_then(|details| details.transaction_type.clone()),
            transaction_status: transaction_details
                .as_ref()
                .and_then(|details| details.transaction_status.clone()),
            status: response.status.clone(),
            merchant_advice_code: None,
        }
    }
}

fn process_nuvei_payment_response(
    data: NuveiPaymentResponseData,
) -> Result<
    (
        enums::AttemptStatus,
        Option<RedirectForm>,
        Option<ConnectorResponseData>,
    ),
    error_stack::Report<errors::ConnectorError>,
> {
    let redirection_data = match data.payment_method {
        enums::PaymentMethod::Wallet | enums::PaymentMethod::BankRedirect => data
            .payment_option
            .as_ref()
            .and_then(|po| po.redirect_url.clone())
            .map(|base_url| RedirectForm::from((base_url, Method::Get))),
        _ => data
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
        convert_to_additional_payment_method_connector_response(data.payment_option.clone())
            .map(ConnectorResponseData::with_additional_payment_method_data);
    let status = get_payment_status(
        data.amount,
        data.is_post_capture_void,
        data.transaction_type,
        data.transaction_status,
        data.status,
    );

    Ok((status, redirection_data, connector_response_data))
}

// Helper function to create transaction response
fn create_transaction_response(
    redirection_data: Option<RedirectForm>,
    ip_address: Option<String>,
    transaction_id: Option<String>,
    order_id: Option<String>,
    session_token: Option<Secret<String>>,
    external_scheme_transaction_id: Option<Secret<String>>,
    payment_option: Option<PaymentOption>,
) -> Result<PaymentsResponseData, error_stack::Report<errors::ConnectorError>> {
    Ok(PaymentsResponseData::TransactionResponse {
        resource_id: transaction_id
            .clone()
            .map_or(order_id.clone(), Some) // For paypal there will be no transaction_id, only order_id will be present
            .map(ResponseId::ConnectorTransactionId)
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?,
        redirection_data: Box::new(redirection_data),
        mandate_reference: Box::new(
            payment_option
                .as_ref()
                .and_then(|po| po.user_payment_option_id.clone())
                .map(|id| MandateReference {
                    connector_mandate_id: Some(id),
                    payment_method_id: None,
                    mandate_metadata: ip_address
                        .map(|ip| pii::SecretSerdeValue::new(serde_json::Value::String(ip))),
                    connector_mandate_request_reference_id: None,
                }),
        ),
        // we don't need to save session token for capture, void flow so ignoring if it is not present
        connector_metadata: if let Some(token) = session_token {
            Some(
                serde_json::to_value(NuveiMeta {
                    session_token: token,
                })
                .change_context(errors::ConnectorError::ResponseHandlingFailed)?,
            )
        } else {
            None
        },
        network_txn_id: external_scheme_transaction_id
            .as_ref()
            .map(|ntid| ntid.clone().expose()),
        connector_response_reference_id: order_id.clone(),
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
        let response = &item.response;
        let (status, redirection_data, connector_response_data) = process_nuvei_payment_response(
            NuveiPaymentResponseData::new(amount, false, item.data.payment_method, response),
        )?;

        let (amount_captured, minor_amount_capturable) = get_amount_captured(
            response.partial_approval.clone(),
            response.transaction_type.clone(),
        )?;

        let ip_address = item
            .data
            .request
            .browser_info
            .clone()
            .and_then(|browser_info| browser_info.ip_address.map(|ip| ip.to_string()));

        Ok(Self {
            status,
            response: if let Some(err) = build_error_response(ErrorResponseParams {
                http_code: item.http_code,
                status: response.status.clone(),
                err_code: response.err_code,
                err_msg: response.reason.clone(),
                merchant_advice_code: response.merchant_advice_code.clone(),
                gw_error_code: response.gw_error_code,
                gw_error_reason: response.gw_error_reason.clone(),
                transaction_status: response.transaction_status.clone(),
                transaction_id: response.transaction_id.clone(),
            }) {
                Err(err)
            } else {
                let response = &item.response;
                Ok(create_transaction_response(
                    redirection_data,
                    ip_address,
                    response.transaction_id.clone(),
                    response.order_id.clone(),
                    response.session_token.clone(),
                    response.external_scheme_transaction_id.clone(),
                    response.payment_option.clone(),
                )?)
            },
            amount_captured,
            minor_amount_capturable,
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
        let response = &item.response;
        let (status, redirection_data, connector_response_data) =
            process_nuvei_payment_response(NuveiPaymentResponseData::new(
                amount,
                F::is_post_capture_void(),
                item.data.payment_method,
                response,
            ))?;

        let (amount_captured, minor_amount_capturable) = get_amount_captured(
            response.partial_approval.clone(),
            response.transaction_type.clone(),
        )?;
        Ok(Self {
            status,
            response: if let Some(err) = build_error_response(ErrorResponseParams {
                http_code: item.http_code,
                status: response.status.clone(),
                err_code: response.err_code,
                err_msg: response.reason.clone(),
                merchant_advice_code: response.merchant_advice_code.clone(),
                gw_error_code: response.gw_error_code,
                gw_error_reason: response.gw_error_reason.clone(),
                transaction_status: response.transaction_status.clone(),
                transaction_id: response.transaction_id.clone(),
            }) {
                Err(err)
            } else {
                let response = &item.response;
                Ok(create_transaction_response(
                    redirection_data,
                    None,
                    response.transaction_id.clone(),
                    response.order_id.clone(),
                    response.session_token.clone(),
                    response.external_scheme_transaction_id.clone(),
                    response.payment_option.clone(),
                )?)
            },
            amount_captured,
            minor_amount_capturable,
            connector_response: connector_response_data,
            ..item.data
        })
    }
}

// Generic implementation for other flow types
impl<F, T> TryFrom<ResponseRouterData<F, NuveiTransactionSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
where
    F: NuveiPaymentsGenericResponse + std::fmt::Debug,
    T: std::fmt::Debug,
    F: std::any::Any,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, NuveiTransactionSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let amount = item
            .data
            .minor_amount_capturable
            .map(|amount| amount.get_amount_as_i64());
        let response = &item.response;
        let transaction_details = &response.transaction_details;
        let transaction_type = transaction_details
            .as_ref()
            .and_then(|details| details.transaction_type.clone());
        let (status, redirection_data, connector_response_data) =
            process_nuvei_payment_response(NuveiPaymentResponseData::new_from_sync_response(
                amount,
                F::is_post_capture_void(),
                item.data.payment_method,
                response,
            ))?;

        let (amount_captured, minor_amount_capturable) =
            get_amount_captured(response.get_partial_approval(), transaction_type.clone())?;

        if bypass_error_for_no_payments_found(response.err_code) {
            return Ok(item.data);
        };

        Ok(Self {
            status,
            response: if let Some(err) = build_error_response(ErrorResponseParams {
                http_code: item.http_code,
                status: response.status.clone(),
                err_code: response.err_code,
                err_msg: response.reason.clone(),
                merchant_advice_code: None,
                gw_error_code: transaction_details
                    .as_ref()
                    .and_then(|details| details.gw_error_code),
                gw_error_reason: transaction_details
                    .as_ref()
                    .and_then(|details| details.gw_error_reason.clone()),
                transaction_status: transaction_details
                    .as_ref()
                    .and_then(|details| details.transaction_status.clone()),
                transaction_id: transaction_details
                    .as_ref()
                    .and_then(|details| details.transaction_id.clone()),
            }) {
                Err(err)
            } else {
                Ok(create_transaction_response(
                    redirection_data,
                    None,
                    transaction_details
                        .as_ref()
                        .and_then(|data| data.transaction_id.clone()),
                    None,
                    None,
                    None,
                    response.payment_option.clone(),
                )?)
            },
            amount_captured,
            minor_amount_capturable,
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
            status: get_payment_status(
                item.data.request.amount,
                false,
                response.transaction_type,
                response.transaction_status,
                response.status,
            ),
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
            get_refund_response(item.response.clone(), item.http_code, transaction_id);

        Ok(Self {
            response: refund_response.map_err(|err| *err),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, NuveiTransactionSyncResponse>>
    for types::RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, NuveiTransactionSyncResponse>,
    ) -> Result<Self, Self::Error> {
        if bypass_error_for_no_payments_found(item.response.err_code) {
            return Ok(item.data);
        };
        let txn_id = item
            .response
            .transaction_details
            .as_ref()
            .and_then(|details| details.transaction_id.clone())
            .ok_or(errors::ConnectorError::MissingConnectorTransactionID)?;

        let refund_status = item
            .response
            .transaction_details
            .as_ref()
            .and_then(|details| details.transaction_status.clone())
            .map(enums::RefundStatus::from)
            .unwrap_or(enums::RefundStatus::Failure);

        let network_decline_code = item
            .response
            .transaction_details
            .as_ref()
            .and_then(|details| details.gw_error_code.map(|e| e.to_string()));

        let network_error_msg = item
            .response
            .transaction_details
            .as_ref()
            .and_then(|details| details.gw_error_reason.clone());

        let refund_response = match item.response.status {
            NuveiPaymentStatus::Error => Err(Box::new(get_error_response(
                item.response.err_code,
                item.response.reason.clone(),
                item.http_code,
                item.response.merchant_advice_code,
                network_decline_code,
                network_error_msg,
                Some(txn_id.clone()),
            ))),
            _ => match item
                .response
                .transaction_details
                .and_then(|nuvei_response| nuvei_response.transaction_status)
            {
                Some(NuveiTransactionStatus::Error) => Err(Box::new(get_error_response(
                    item.response.err_code,
                    item.response.reason,
                    item.http_code,
                    item.response.merchant_advice_code,
                    network_decline_code,
                    network_error_msg,
                    Some(txn_id.clone()),
                ))),
                _ => Ok(RefundsResponseData {
                    connector_refund_id: txn_id,
                    refund_status,
                }),
            },
        };

        Ok(Self {
            response: refund_response.map_err(|err| *err),
            ..item.data
        })
    }
}

impl<F, Req> TryFrom<&RouterData<F, Req, PaymentsResponseData>> for NuveiPaymentsRequest
where
    Req: NuveiAuthorizePreprocessingCommon,
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RouterData<F, Req, PaymentsResponseData>) -> Result<Self, Self::Error> {
        {
            Ok(Self {
                related_transaction_id: item.request.get_related_transaction_id().clone(),
                device_details: DeviceDetails {
                    ip_address: Secret::new(
                        item.recurring_mandate_payment_data
                            .as_ref()
                            .and_then(|r| r.mandate_metadata.as_ref())
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "browser_info.ip_address",
                            })?
                            .clone()
                            .expose()
                            .as_str()
                            .ok_or(errors::ConnectorError::MissingRequiredField {
                                field_name: "browser_info.ip_address",
                            })?
                            .to_owned(),
                    ),
                },
                payment_option: PaymentOption {
                    user_payment_option_id: item.request.get_connector_mandate_id().clone(),
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
) -> Result<RefundsResponseData, Box<ErrorResponse>> {
    // Check if either status or transaction_status indicates an error
    if matches!(response.status, NuveiPaymentStatus::Error)
        || matches!(
            response.transaction_status,
            Some(NuveiTransactionStatus::Error)
        )
    {
        return Err(Box::new(get_error_response(
            response.err_code,
            response.reason.clone(),
            http_code,
            response.merchant_advice_code,
            response.gw_error_code.map(|e| e.to_string()),
            response.gw_error_reason,
            Some(txn_id),
        )));
    }
    let refund_status = response
        .transaction_status
        .clone()
        .map(enums::RefundStatus::from)
        .unwrap_or(enums::RefundStatus::Failure);
    Ok(RefundsResponseData {
        connector_refund_id: txn_id,
        refund_status,
    })
}

fn get_error_response(
    error_code: Option<i64>,
    error_msg: Option<String>,
    http_code: u16,
    network_advice_code: Option<String>,
    network_decline_code: Option<String>,
    network_error_message: Option<String>,
    transaction_id: Option<String>,
) -> ErrorResponse {
    ErrorResponse {
        code: error_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| NO_ERROR_CODE.to_string()),
        message: error_msg
            .clone()
            .unwrap_or_else(|| NO_ERROR_MESSAGE.to_string()),
        reason: None,
        status_code: http_code,
        attempt_status: None,
        connector_transaction_id: transaction_id,
        network_advice_code: network_advice_code.clone(),
        network_decline_code: network_decline_code.clone(),
        network_error_message: network_error_message.clone(),
        connector_metadata: None,
    }
}

/// Represents any possible webhook notification from Nuvei.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NuveiWebhook {
    PaymentDmn(PaymentDmnNotification),
    Chargeback(ChargebackNotification),
}

/// Represents Psync Response from Nuvei.
#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum NuveiPaymentSyncResponse {
    NuveiDmn(Box<PaymentDmnNotification>),
    NuveiApi(Box<NuveiTransactionSyncResponse>),
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
#[serde(rename_all = "PascalCase")]
pub struct ChargebackNotification {
    pub client_name: Option<String>,
    pub event_date_u_t_c: Option<String>,
    pub event_correlation_id: Option<String>,
    pub chargeback: ChargebackData,
    pub transaction_details: ChargebackTransactionDetails,
    pub event_id: Option<String>,
    pub processing_entity_type: Option<String>,
    pub processing_entity_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChargebackData {
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub date: Option<time::PrimitiveDateTime>,
    pub chargeback_status_category: Option<ChargebackStatusCategory>,
    #[serde(rename = "Type")]
    pub webhook_type: Option<ChargebackType>,
    pub status: Option<String>,
    pub amount: FloatMajorUnit,
    pub currency: String,
    pub reported_amount: FloatMajorUnit,
    pub reported_currency: String,
    pub chargeback_reason: Option<String>,
    pub chargeback_reason_category: Option<String>,
    pub reason_message: Option<String>,
    pub dispute_id: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub dispute_due_date: Option<time::PrimitiveDateTime>,
    pub dispute_event_id: Option<String>,
    pub dispute_unified_status_code: Option<DisputeUnifiedStatusCode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, strum::Display)]
pub enum DisputeUnifiedStatusCode {
    #[serde(rename = "FC")]
    FirstChargebackInitiatedByIssuer,

    #[serde(rename = "CC")]
    CreditChargebackInitiatedByIssuer,

    #[serde(rename = "CC-A-ACPT")]
    CreditChargebackAcceptedAutomatically,

    #[serde(rename = "FC-A-EPRD")]
    FirstChargebackNoResponseExpired,

    #[serde(rename = "FC-M-ACPT")]
    FirstChargebackAcceptedByMerchant,

    #[serde(rename = "FC-A-ACPT")]
    FirstChargebackAcceptedAutomatically,

    #[serde(rename = "FC-A-ACPT-MCOLL")]
    FirstChargebackAcceptedAutomaticallyMcoll,

    #[serde(rename = "FC-M-PART")]
    FirstChargebackPartiallyAcceptedByMerchant,

    #[serde(rename = "FC-M-PART-EXP")]
    FirstChargebackPartiallyAcceptedByMerchantExpired,

    #[serde(rename = "FC-M-RJCT")]
    FirstChargebackRejectedByMerchant,

    #[serde(rename = "FC-M-RJCT-EXP")]
    FirstChargebackRejectedByMerchantExpired,

    #[serde(rename = "FC-A-RJCT")]
    FirstChargebackRejectedAutomatically,

    #[serde(rename = "FC-A-RJCT-EXP")]
    FirstChargebackRejectedAutomaticallyExpired,

    #[serde(rename = "IPA")]
    PreArbitrationInitiatedByIssuer,

    #[serde(rename = "MPA-I-ACPT")]
    MerchantPreArbitrationAcceptedByIssuer,

    #[serde(rename = "MPA-I-RJCT")]
    MerchantPreArbitrationRejectedByIssuer,

    #[serde(rename = "MPA-I-PART")]
    MerchantPreArbitrationPartiallyAcceptedByIssuer,

    #[serde(rename = "FC-CLSD-MF")]
    FirstChargebackClosedMerchantFavour,

    #[serde(rename = "FC-CLSD-CHF")]
    FirstChargebackClosedCardholderFavour,

    #[serde(rename = "FC-CLSD-RCL")]
    FirstChargebackClosedRecall,

    #[serde(rename = "FC-I-RCL")]
    FirstChargebackRecalledByIssuer,

    #[serde(rename = "PA-CLSD-MF")]
    PreArbitrationClosedMerchantFavour,

    #[serde(rename = "PA-CLSD-CHF")]
    PreArbitrationClosedCardholderFavour,

    #[serde(rename = "RDR")]
    Rdr,

    #[serde(rename = "FC-SPCSE")]
    FirstChargebackDisputeResponseNotAllowed,

    #[serde(rename = "MCC")]
    McCollaborationInitiatedByIssuer,

    #[serde(rename = "MCC-A-RJCT")]
    McCollaborationPreviouslyRefundedAuto,

    #[serde(rename = "MCC-M-ACPT")]
    McCollaborationRefundedByMerchant,

    #[serde(rename = "MCC-EXPR")]
    McCollaborationExpired,

    #[serde(rename = "MCC-M-RJCT")]
    McCollaborationRejectedByMerchant,

    #[serde(rename = "MCC-A-ACPT")]
    McCollaborationAutomaticAccept,

    #[serde(rename = "MCC-CLSD-MF")]
    McCollaborationClosedMerchantFavour,

    #[serde(rename = "MCC-CLSD-CHF")]
    McCollaborationClosedCardholderFavour,

    #[serde(rename = "INQ")]
    InquiryInitiatedByIssuer,

    #[serde(rename = "INQ-M-RSP")]
    InquiryRespondedByMerchant,

    #[serde(rename = "INQ-EXPR")]
    InquiryExpired,

    #[serde(rename = "INQ-A-RJCT")]
    InquiryAutomaticallyRejected,

    #[serde(rename = "INQ-A-CNLD")]
    InquiryCancelledAfterRefund,

    #[serde(rename = "INQ-M-RFND")]
    InquiryAcceptedFullRefund,

    #[serde(rename = "INQ-M-P-RFND")]
    InquiryPartialAcceptedPartialRefund,

    #[serde(rename = "INQ-UPD")]
    InquiryUpdated,

    #[serde(rename = "IPA-M-ACPT")]
    PreArbitrationAcceptedByMerchant,

    #[serde(rename = "IPA-M-PART")]
    PreArbitrationPartiallyAcceptedByMerchant,

    #[serde(rename = "IPA-M-PART-EXP")]
    PreArbitrationPartiallyAcceptedByMerchantExpired,

    #[serde(rename = "IPA-M-RJCT")]
    PreArbitrationRejectedByMerchant,

    #[serde(rename = "IPA-M-RJCT-EXP")]
    PreArbitrationRejectedByMerchantExpired,

    #[serde(rename = "IPA-A-ACPT")]
    PreArbitrationAutomaticallyAcceptedByMerchant,

    #[serde(rename = "PA-CLSD-RC")]
    PreArbitrationClosedRecall,

    #[serde(rename = "IPAR-M-ACPT")]
    RejectedPreArbAcceptedByMerchant,

    #[serde(rename = "IPAR-A-ACPT")]
    RejectedPreArbExpiredAutoAccepted,

    #[serde(rename = "CC-I-RCLL")]
    CreditChargebackRecalledByIssuer,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChargebackTransactionDetails {
    pub transaction_id: i64,
    pub transaction_date: Option<String>,
    pub client_unique_id: Option<String>,
    pub acquirer_name: Option<String>,
    pub masked_card_number: Option<String>,
    pub arn: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ChargebackType {
    Chargeback,
    Retrieval,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ChargebackStatusCategory {
    #[serde(rename = "Regular")]
    Regular,
    #[serde(rename = "cancelled")]
    Cancelled,
    #[serde(rename = "Duplicate")]
    Duplicate,
    #[serde(rename = "RDR-Refund")]
    RdrRefund,
    #[serde(rename = "Soft_CB")]
    SoftCb,
}

/// Represents the overall status of the DMN.
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DmnStatus {
    Success,
    Approved,
    Error,
    Pending,
    Declined,
}

/// Represents the transaction status of the DMN
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
pub enum DmnApiTransactionStatus {
    Ok,
    Fail,
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

/// Represents a Payment Direct Merchant Notification (DMN) webhook.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentDmnNotification {
    // Status of the Api transaction
    #[serde(rename = "ppp_status")]
    pub ppp_status: DmnApiTransactionStatus,
    #[serde(rename = "PPP_TransactionID")]
    pub ppp_transaction_id: String,
    pub total_amount: String,
    pub currency: String,
    #[serde(rename = "TransactionID")]
    pub transaction_id: Option<String>,
    // Status of the Payment
    #[serde(rename = "Status")]
    pub status: Option<DmnStatus>,
    pub transaction_type: Option<NuveiTransactionType>,
    #[serde(rename = "ErrCode")]
    pub err_code: Option<String>,
    #[serde(rename = "Reason")]
    pub reason: Option<String>,
    #[serde(rename = "ReasonCode")]
    pub reason_code: Option<String>,
    #[serde(rename = "user_token_id")]
    pub user_token_id: Option<String>,
    #[serde(rename = "payment_method")]
    pub payment_method: Option<String>,
    #[serde(rename = "responseTimeStamp")]
    pub response_time_stamp: String,
    #[serde(rename = "merchant_id")]
    pub merchant_id: Option<Secret<String>>,
    #[serde(rename = "merchant_site_id")]
    pub merchant_site_id: Option<Secret<String>>,
    #[serde(rename = "responsechecksum")]
    pub response_checksum: Option<String>,
    #[serde(rename = "advanceResponseChecksum")]
    pub advance_response_checksum: Option<String>,
    pub product_id: Option<String>,
    pub merchant_advice_code: Option<String>,
    #[serde(rename = "AuthCode")]
    pub auth_code: Option<String>,
    pub acquirer_bank: Option<String>,
    pub client_request_id: Option<String>,
}

// For backward compatibility with existing code
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct NuveiWebhookTransactionId {
    #[serde(rename = "ppp_TransactionID")]
    pub ppp_transaction_id: String,
}

// Convert webhook to payments response for further processing
impl From<PaymentDmnNotification> for NuveiTransactionSyncResponse {
    fn from(notification: PaymentDmnNotification) -> Self {
        Self {
            status: match notification.ppp_status {
                DmnApiTransactionStatus::Ok => NuveiPaymentStatus::Success,
                DmnApiTransactionStatus::Fail => NuveiPaymentStatus::Failed,
                DmnApiTransactionStatus::Pending => NuveiPaymentStatus::Processing,
            },
            err_code: notification
                .err_code
                .and_then(|code| code.parse::<i64>().ok()),
            reason: notification.reason.clone(),
            transaction_details: Some(NuveiTransactionSyncResponseDetails {
                gw_error_code: notification
                    .reason_code
                    .and_then(|code| code.parse::<i64>().ok()),
                gw_error_reason: notification.reason.clone(),
                gw_extended_error_code: None,
                transaction_id: notification.transaction_id,
                transaction_status: notification.status.map(|ts| match ts {
                    DmnStatus::Success | DmnStatus::Approved => NuveiTransactionStatus::Approved,
                    DmnStatus::Declined => NuveiTransactionStatus::Declined,
                    DmnStatus::Pending => NuveiTransactionStatus::Pending,
                    DmnStatus::Error => NuveiTransactionStatus::Error,
                }),
                transaction_type: notification.transaction_type,
                auth_code: notification.auth_code,
                processed_amount: None,
                processed_currency: None,
                acquiring_bank_name: notification.acquirer_bank,
            }),
            merchant_id: notification.merchant_id,
            merchant_site_id: notification.merchant_site_id,
            merchant_advice_code: notification.merchant_advice_code,
            ..Default::default()
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

/// Concatenates a vector of strings without any separator
/// This is useful for creating verification messages for webhooks
pub fn concat_strings(strings: &[String]) -> String {
    strings.join("")
}

fn convert_to_additional_payment_method_connector_response(
    payment_option: Option<PaymentOption>,
) -> Option<AdditionalPaymentMethodConnectorResponse> {
    let card = payment_option.as_ref()?.card.as_ref()?;
    let avs_code = card.avs_code.as_ref();
    let cvv2_code = card.cvv2_reply.as_ref();

    let avs_description = avs_code.and_then(|code| get_avs_response_description(code));
    let cvv_description = cvv2_code.and_then(|code| get_cvv2_response_description(code));

    let payment_checks = serde_json::json!({
        "avs_result": avs_code,
        "avs_description": avs_description,
        "card_validation_result": cvv2_code,
        "card_validation_description": cvv_description,
    });

    let card_network = card.brand.clone();
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

pub fn map_notification_to_event(
    status: DmnStatus,
    transaction_type: NuveiTransactionType,
) -> Result<api_models::webhooks::IncomingWebhookEvent, error_stack::Report<errors::ConnectorError>>
{
    match (status, transaction_type) {
        (DmnStatus::Success | DmnStatus::Approved, NuveiTransactionType::Auth) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationSuccess)
        }
        (DmnStatus::Success | DmnStatus::Approved, NuveiTransactionType::Sale) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess)
        }
        (DmnStatus::Success | DmnStatus::Approved, NuveiTransactionType::Settle) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentCaptureSuccess)
        }
        (DmnStatus::Success | DmnStatus::Approved, NuveiTransactionType::Void) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled)
        }
        (DmnStatus::Success | DmnStatus::Approved, NuveiTransactionType::Credit) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::RefundSuccess)
        }
        (DmnStatus::Error | DmnStatus::Declined, NuveiTransactionType::Auth) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationFailure)
        }
        (DmnStatus::Error | DmnStatus::Declined, NuveiTransactionType::Sale) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure)
        }
        (DmnStatus::Error | DmnStatus::Declined, NuveiTransactionType::Settle) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentCaptureFailure)
        }
        (DmnStatus::Error | DmnStatus::Declined, NuveiTransactionType::Void) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelFailure)
        }
        (DmnStatus::Error | DmnStatus::Declined, NuveiTransactionType::Credit) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::RefundFailure)
        }
        (
            DmnStatus::Pending,
            NuveiTransactionType::Auth | NuveiTransactionType::Sale | NuveiTransactionType::Settle,
        ) => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing),
        _ => Err(errors::ConnectorError::WebhookEventTypeNotFound.into()),
    }
}

#[cfg(feature = "payouts")]
pub fn map_notification_to_event_for_payout(
    status: DmnStatus,
    transaction_type: NuveiTransactionType,
) -> Result<api_models::webhooks::IncomingWebhookEvent, error_stack::Report<errors::ConnectorError>>
{
    match (status, transaction_type) {
        (DmnStatus::Success | DmnStatus::Approved, NuveiTransactionType::Credit) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PayoutSuccess)
        }
        (DmnStatus::Pending, _) => Ok(api_models::webhooks::IncomingWebhookEvent::PayoutProcessing),
        (DmnStatus::Declined | DmnStatus::Error, _) => {
            Ok(api_models::webhooks::IncomingWebhookEvent::PayoutFailure)
        }
        _ => Err(errors::ConnectorError::WebhookEventTypeNotFound.into()),
    }
}

pub fn get_dispute_stage(
    chargeback_data: &ChargebackData,
) -> Result<common_enums::DisputeStage, error_stack::Report<errors::ConnectorError>> {
    let dispute_stage = chargeback_data
        .dispute_unified_status_code
        .clone()
        .map(common_enums::DisputeStage::from)
        .or(match chargeback_data.webhook_type {
            Some(ChargebackType::Retrieval) => Some(common_enums::DisputeStage::PreDispute),
            Some(ChargebackType::Chargeback) | None => None,
        })
        .or(match chargeback_data.chargeback_status_category {
            Some(ChargebackStatusCategory::Cancelled)
            | Some(ChargebackStatusCategory::Duplicate) => {
                Some(common_enums::DisputeStage::DisputeReversal)
            }
            Some(ChargebackStatusCategory::Regular) => Some(common_enums::DisputeStage::Dispute),
            Some(ChargebackStatusCategory::RdrRefund) => {
                Some(common_enums::DisputeStage::PreDispute)
            }
            Some(ChargebackStatusCategory::SoftCb) => {
                Some(common_enums::DisputeStage::PreArbitration)
            }
            None => None,
        });

    dispute_stage.ok_or(errors::ConnectorError::WebhookEventTypeNotFound.into())
}

pub fn map_dispute_notification_to_event(
    chargeback_data: &ChargebackData,
) -> Result<api_models::webhooks::IncomingWebhookEvent, error_stack::Report<errors::ConnectorError>>
{
    let event_code = chargeback_data
        .dispute_unified_status_code
        .as_ref()
        .and_then(|code| match code {
            DisputeUnifiedStatusCode::FirstChargebackInitiatedByIssuer
            | DisputeUnifiedStatusCode::CreditChargebackInitiatedByIssuer
            | DisputeUnifiedStatusCode::McCollaborationInitiatedByIssuer
            | DisputeUnifiedStatusCode::FirstChargebackClosedRecall
            | DisputeUnifiedStatusCode::InquiryInitiatedByIssuer => {
                Some(api_models::webhooks::IncomingWebhookEvent::DisputeOpened)
            }
            DisputeUnifiedStatusCode::CreditChargebackAcceptedAutomatically
            | DisputeUnifiedStatusCode::FirstChargebackAcceptedAutomatically
            | DisputeUnifiedStatusCode::FirstChargebackAcceptedAutomaticallyMcoll
            | DisputeUnifiedStatusCode::FirstChargebackAcceptedByMerchant
            | DisputeUnifiedStatusCode::FirstChargebackDisputeResponseNotAllowed
            | DisputeUnifiedStatusCode::Rdr
            | DisputeUnifiedStatusCode::McCollaborationRefundedByMerchant
            | DisputeUnifiedStatusCode::McCollaborationAutomaticAccept
            | DisputeUnifiedStatusCode::InquiryAcceptedFullRefund
            | DisputeUnifiedStatusCode::PreArbitrationAcceptedByMerchant
            | DisputeUnifiedStatusCode::PreArbitrationPartiallyAcceptedByMerchant
            | DisputeUnifiedStatusCode::PreArbitrationAutomaticallyAcceptedByMerchant
            | DisputeUnifiedStatusCode::RejectedPreArbAcceptedByMerchant
            | DisputeUnifiedStatusCode::RejectedPreArbExpiredAutoAccepted => {
                Some(api_models::webhooks::IncomingWebhookEvent::DisputeAccepted)
            }
            DisputeUnifiedStatusCode::FirstChargebackNoResponseExpired
            | DisputeUnifiedStatusCode::FirstChargebackPartiallyAcceptedByMerchant
            | DisputeUnifiedStatusCode::FirstChargebackClosedCardholderFavour
            | DisputeUnifiedStatusCode::PreArbitrationClosedCardholderFavour
            | DisputeUnifiedStatusCode::McCollaborationClosedCardholderFavour => {
                Some(api_models::webhooks::IncomingWebhookEvent::DisputeLost)
            }
            DisputeUnifiedStatusCode::FirstChargebackRejectedByMerchant
            | DisputeUnifiedStatusCode::FirstChargebackRejectedAutomatically
            | DisputeUnifiedStatusCode::PreArbitrationInitiatedByIssuer
            | DisputeUnifiedStatusCode::MerchantPreArbitrationRejectedByIssuer
            | DisputeUnifiedStatusCode::InquiryRespondedByMerchant
            | DisputeUnifiedStatusCode::PreArbitrationRejectedByMerchant => {
                Some(api_models::webhooks::IncomingWebhookEvent::DisputeChallenged)
            }
            DisputeUnifiedStatusCode::FirstChargebackRejectedAutomaticallyExpired
            | DisputeUnifiedStatusCode::FirstChargebackPartiallyAcceptedByMerchantExpired
            | DisputeUnifiedStatusCode::FirstChargebackRejectedByMerchantExpired
            | DisputeUnifiedStatusCode::McCollaborationExpired
            | DisputeUnifiedStatusCode::InquiryExpired
            | DisputeUnifiedStatusCode::PreArbitrationPartiallyAcceptedByMerchantExpired
            | DisputeUnifiedStatusCode::PreArbitrationRejectedByMerchantExpired => {
                Some(api_models::webhooks::IncomingWebhookEvent::DisputeExpired)
            }
            DisputeUnifiedStatusCode::MerchantPreArbitrationAcceptedByIssuer
            | DisputeUnifiedStatusCode::MerchantPreArbitrationPartiallyAcceptedByIssuer
            | DisputeUnifiedStatusCode::FirstChargebackClosedMerchantFavour
            | DisputeUnifiedStatusCode::McCollaborationClosedMerchantFavour
            | DisputeUnifiedStatusCode::PreArbitrationClosedMerchantFavour => {
                Some(api_models::webhooks::IncomingWebhookEvent::DisputeWon)
            }
            DisputeUnifiedStatusCode::FirstChargebackRecalledByIssuer
            | DisputeUnifiedStatusCode::InquiryCancelledAfterRefund
            | DisputeUnifiedStatusCode::PreArbitrationClosedRecall
            | DisputeUnifiedStatusCode::CreditChargebackRecalledByIssuer => {
                Some(api_models::webhooks::IncomingWebhookEvent::DisputeCancelled)
            }

            DisputeUnifiedStatusCode::McCollaborationPreviouslyRefundedAuto
            | DisputeUnifiedStatusCode::McCollaborationRejectedByMerchant
            | DisputeUnifiedStatusCode::InquiryAutomaticallyRejected
            | DisputeUnifiedStatusCode::InquiryPartialAcceptedPartialRefund
            | DisputeUnifiedStatusCode::InquiryUpdated => None,
        });

    event_code
        .or_else(|| {
            chargeback_data
                .chargeback_status_category
                .as_ref()
                .and_then(|code| match code {
                    ChargebackStatusCategory::Cancelled | ChargebackStatusCategory::Duplicate => {
                        Some(api_models::webhooks::IncomingWebhookEvent::DisputeCancelled)
                    }
                    ChargebackStatusCategory::RdrRefund => {
                        Some(api_models::webhooks::IncomingWebhookEvent::DisputeAccepted)
                    }
                    _ => None,
                })
        })
        .ok_or(errors::ConnectorError::WebhookEventTypeNotFound.into())
}

impl From<DisputeUnifiedStatusCode> for common_enums::DisputeStage {
    fn from(code: DisputeUnifiedStatusCode) -> Self {
        match code {
            // --- PreDispute ---
            DisputeUnifiedStatusCode::Rdr
            | DisputeUnifiedStatusCode::InquiryInitiatedByIssuer
            | DisputeUnifiedStatusCode::InquiryRespondedByMerchant
            | DisputeUnifiedStatusCode::InquiryExpired
            | DisputeUnifiedStatusCode::InquiryAutomaticallyRejected
            | DisputeUnifiedStatusCode::InquiryCancelledAfterRefund
            | DisputeUnifiedStatusCode::InquiryAcceptedFullRefund
            | DisputeUnifiedStatusCode::InquiryPartialAcceptedPartialRefund
            | DisputeUnifiedStatusCode::InquiryUpdated => Self::PreDispute,

            // --- Dispute ---
            DisputeUnifiedStatusCode::FirstChargebackInitiatedByIssuer
            | DisputeUnifiedStatusCode::CreditChargebackInitiatedByIssuer
            | DisputeUnifiedStatusCode::FirstChargebackNoResponseExpired
            | DisputeUnifiedStatusCode::FirstChargebackAcceptedByMerchant
            | DisputeUnifiedStatusCode::FirstChargebackAcceptedAutomatically
            | DisputeUnifiedStatusCode::FirstChargebackAcceptedAutomaticallyMcoll
            | DisputeUnifiedStatusCode::FirstChargebackPartiallyAcceptedByMerchant
            | DisputeUnifiedStatusCode::FirstChargebackPartiallyAcceptedByMerchantExpired
            | DisputeUnifiedStatusCode::FirstChargebackRejectedByMerchant
            | DisputeUnifiedStatusCode::FirstChargebackRejectedByMerchantExpired
            | DisputeUnifiedStatusCode::FirstChargebackRejectedAutomatically
            | DisputeUnifiedStatusCode::FirstChargebackRejectedAutomaticallyExpired
            | DisputeUnifiedStatusCode::FirstChargebackClosedMerchantFavour
            | DisputeUnifiedStatusCode::FirstChargebackClosedCardholderFavour
            | DisputeUnifiedStatusCode::FirstChargebackClosedRecall
            | DisputeUnifiedStatusCode::FirstChargebackRecalledByIssuer
            | DisputeUnifiedStatusCode::FirstChargebackDisputeResponseNotAllowed
            | DisputeUnifiedStatusCode::McCollaborationInitiatedByIssuer
            | DisputeUnifiedStatusCode::McCollaborationPreviouslyRefundedAuto
            | DisputeUnifiedStatusCode::McCollaborationRefundedByMerchant
            | DisputeUnifiedStatusCode::McCollaborationExpired
            | DisputeUnifiedStatusCode::McCollaborationRejectedByMerchant
            | DisputeUnifiedStatusCode::McCollaborationAutomaticAccept
            | DisputeUnifiedStatusCode::McCollaborationClosedMerchantFavour
            | DisputeUnifiedStatusCode::McCollaborationClosedCardholderFavour
            | DisputeUnifiedStatusCode::CreditChargebackAcceptedAutomatically => Self::Dispute,

            // --- PreArbitration ---
            DisputeUnifiedStatusCode::PreArbitrationInitiatedByIssuer
            | DisputeUnifiedStatusCode::MerchantPreArbitrationAcceptedByIssuer
            | DisputeUnifiedStatusCode::MerchantPreArbitrationRejectedByIssuer
            | DisputeUnifiedStatusCode::MerchantPreArbitrationPartiallyAcceptedByIssuer
            | DisputeUnifiedStatusCode::PreArbitrationClosedMerchantFavour
            | DisputeUnifiedStatusCode::PreArbitrationClosedCardholderFavour
            | DisputeUnifiedStatusCode::PreArbitrationAcceptedByMerchant
            | DisputeUnifiedStatusCode::PreArbitrationPartiallyAcceptedByMerchant
            | DisputeUnifiedStatusCode::PreArbitrationPartiallyAcceptedByMerchantExpired
            | DisputeUnifiedStatusCode::PreArbitrationRejectedByMerchant
            | DisputeUnifiedStatusCode::PreArbitrationRejectedByMerchantExpired
            | DisputeUnifiedStatusCode::PreArbitrationAutomaticallyAcceptedByMerchant
            | DisputeUnifiedStatusCode::PreArbitrationClosedRecall
            | DisputeUnifiedStatusCode::RejectedPreArbAcceptedByMerchant
            | DisputeUnifiedStatusCode::RejectedPreArbExpiredAutoAccepted => Self::PreArbitration,

            // --- DisputeReversal ---
            DisputeUnifiedStatusCode::CreditChargebackRecalledByIssuer => Self::DisputeReversal,
        }
    }
}
/// bypass error state when psync is called immediately and psp returns no payments found
/// https://docs.nuvei.com/documentation/integration/response-handling/
fn bypass_error_for_no_payments_found(err_code: Option<i64>) -> bool {
    match err_code {
        //No transaction details returned for the provided ID.
        Some(9146) => true,
        _ => false,
    }
}

impl TryFrom<BrowserInformation> for BrowserDetails {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(browser_info: BrowserInformation) -> Result<Self, Self::Error> {
        Ok(Self {
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
        })
    }
}
trait NuveiAmountExt {
    fn to_nuvei_amount(
        &self,
        currency: enums::Currency,
    ) -> Result<StringMajorUnit, error_stack::Report<errors::ConnectorError>>;
}

impl NuveiAmountExt for MinorUnit {
    fn to_nuvei_amount(
        &self,
        currency: enums::Currency,
    ) -> Result<StringMajorUnit, error_stack::Report<errors::ConnectorError>> {
        convert_amount(&StringMajorUnitForConnector, *self, currency)
    }
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum NuveiCardType {
    Visa,
    MasterCard,
    AmericanExpress,
    Discover,
    DinersClub,
    Interac,
    JCB,
    UnionPay,
    CartesBancaires,
}

impl TryFrom<common_enums::CardNetwork> for NuveiCardType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(card_network: common_enums::CardNetwork) -> Result<Self, Self::Error> {
        match card_network {
            common_enums::CardNetwork::Visa => Ok(Self::Visa),
            common_enums::CardNetwork::Mastercard => Ok(Self::MasterCard),
            common_enums::CardNetwork::AmericanExpress => Ok(Self::AmericanExpress),
            common_enums::CardNetwork::Discover => Ok(Self::Discover),
            common_enums::CardNetwork::DinersClub => Ok(Self::DinersClub),
            common_enums::CardNetwork::JCB => Ok(Self::JCB),
            common_enums::CardNetwork::UnionPay => Ok(Self::UnionPay),
            common_enums::CardNetwork::CartesBancaires => Ok(Self::CartesBancaires),
            common_enums::CardNetwork::Interac => Ok(Self::Interac),
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Card network".to_string(),
                connector: "nuvei",
            }
            .into()),
        }
    }
}

impl TryFrom<&utils::CardIssuer> for NuveiCardType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(card_issuer: &utils::CardIssuer) -> Result<Self, Self::Error> {
        match card_issuer {
            utils::CardIssuer::Visa => Ok(Self::Visa),
            utils::CardIssuer::Master => Ok(Self::MasterCard),
            utils::CardIssuer::AmericanExpress => Ok(Self::AmericanExpress),
            utils::CardIssuer::Discover => Ok(Self::Discover),
            utils::CardIssuer::DinersClub => Ok(Self::DinersClub),
            utils::CardIssuer::JCB => Ok(Self::JCB),
            utils::CardIssuer::CartesBancaires => Ok(Self::CartesBancaires),
            &utils::CardIssuer::UnionPay => Ok(Self::UnionPay),
            _ => Err(errors::ConnectorError::NotSupported {
                message: "Card network".to_string(),
                connector: "nuvei",
            }
            .into()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredCredentialMode {
    pub stored_credentials_mode: Option<StoredCredentialModeType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StoredCredentialModeType {
    #[serde(rename = "0")]
    First,
    #[serde(rename = "1")]
    Used,
}

impl StoredCredentialMode {
    pub fn get_optional_stored_credential(is_stored_credential: Option<bool>) -> Option<Self> {
        is_stored_credential.and_then(|is_stored_credential| {
            if is_stored_credential {
                Some(Self {
                    stored_credentials_mode: Some(StoredCredentialModeType::Used),
                })
            } else {
                None
            }
        })
    }
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum IsRebilling {
    #[serde(rename = "1")]
    True,
    #[serde(rename = "0")]
    False,
}
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
pub enum PartialApprovalFlag {
    #[serde(rename = "1")]
    Enabled,
    #[serde(rename = "0")]
    Disabled,
}

impl From<primitive_wrappers::EnablePartialAuthorizationBool> for PartialApprovalFlag {
    fn from(value: primitive_wrappers::EnablePartialAuthorizationBool) -> Self {
        if value.is_true() {
            Self::Enabled
        } else {
            Self::Disabled
        }
    }
}

trait NuveiAuthorizePreprocessingCommon {
    fn get_browser_info(&self) -> Option<BrowserInformation>;
    fn get_complete_authorize_url(&self) -> Option<String>;
    fn get_related_transaction_id(&self) -> Option<String> {
        None
    }
    fn get_billing_descriptor(&self) -> Option<&BillingDescriptor> {
        None
    }
    fn get_auth_data(
        &self,
    ) -> Result<Option<AuthenticationData>, error_stack::Report<errors::ConnectorError>> {
        Ok(None)
    }
    fn get_is_partial_approval(&self) -> Option<PartialApprovalFlag> {
        None
    }
    fn get_is_moto(&self) -> Option<bool> {
        None
    }
    fn get_ntid(&self) -> Option<String> {
        None
    }
    fn get_connector_mandate_id(&self) -> Option<String> {
        None
    }
    fn get_return_url_required(
        &self,
    ) -> Result<String, error_stack::Report<errors::ConnectorError>>;
    fn get_capture_method(&self) -> Option<CaptureMethod>;
    fn get_minor_amount_required(
        &self,
    ) -> Result<MinorUnit, error_stack::Report<errors::ConnectorError>>;
    fn get_email_required(&self) -> Result<Email, error_stack::Report<errors::ConnectorError>>;
    fn get_currency(&self) -> enums::Currency;
    fn get_payment_method_data_required(
        &self,
    ) -> Result<PaymentMethodData, error_stack::Report<errors::ConnectorError>>;

    fn is_customer_initiated_mandate_payment(&self) -> bool;

    fn get_is_stored_credential(&self) -> Option<StoredCredentialMode>;

    fn get_dynamic_descriptor(
        &self,
    ) -> Result<Option<NuveiDynamicDescriptor>, error_stack::Report<errors::ConnectorError>> {
        if let Some(descriptor) = self.get_billing_descriptor() {
            if let Some(phone) = descriptor.phone.as_ref() {
                if phone.clone().expose().len() > 13 {
                    return Err(errors::ConnectorError::MaxFieldLengthViolated {
                        connector: "Nuvei".to_string(),
                        field_name: "dynamic_descriptor.merchant_phone".to_string(),
                        max_length: 13,
                        received_length: phone.clone().expose().len(),
                    }
                    .into());
                }
            }

            let dynamic_descriptor = NuveiDynamicDescriptor {
                merchant_name: descriptor.name.as_ref().map(|name| {
                    Secret::new(name.clone().expose().trim().chars().take(25).collect())
                }),
                merchant_phone: descriptor.phone.clone(),
            };

            Ok(Some(dynamic_descriptor))
        } else {
            Ok(None)
        }
    }
}
impl NuveiAuthorizePreprocessingCommon for CompleteAuthorizeData {
    fn get_currency(&self) -> enums::Currency {
        self.currency
    }
    fn get_return_url_required(
        &self,
    ) -> Result<String, error_stack::Report<errors::ConnectorError>> {
        self.get_router_return_url()
    }

    fn get_minor_amount_required(
        &self,
    ) -> Result<MinorUnit, error_stack::Report<errors::ConnectorError>> {
        Ok(self.minor_amount)
    }

    fn get_payment_method_data_required(
        &self,
    ) -> Result<PaymentMethodData, error_stack::Report<errors::ConnectorError>> {
        self.payment_method_data
            .clone()
            .ok_or_else(missing_field_err("payment_method_data"))
    }

    fn is_customer_initiated_mandate_payment(&self) -> bool {
        self.mandate_id.is_some()
    }

    fn get_capture_method(&self) -> Option<CaptureMethod> {
        self.capture_method
    }

    fn get_email_required(&self) -> Result<Email, error_stack::Report<errors::ConnectorError>> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }

    fn get_browser_info(&self) -> Option<BrowserInformation> {
        self.browser_info.clone()
    }

    fn get_complete_authorize_url(&self) -> Option<String> {
        self.complete_authorize_url.clone()
    }

    fn get_is_stored_credential(&self) -> Option<StoredCredentialMode> {
        StoredCredentialMode::get_optional_stored_credential(self.is_stored_credential)
    }
}
impl NuveiAuthorizePreprocessingCommon for SetupMandateRequestData {
    fn get_browser_info(&self) -> Option<BrowserInformation> {
        self.browser_info.clone()
    }
    fn get_billing_descriptor(&self) -> Option<&BillingDescriptor> {
        self.billing_descriptor.as_ref()
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
    fn get_complete_authorize_url(&self) -> Option<String> {
        self.complete_authorize_url.clone()
    }

    fn get_connector_mandate_id(&self) -> Option<String> {
        self.mandate_id.as_ref().and_then(|mandate_ids| {
            mandate_ids.mandate_reference_id.as_ref().and_then(
                |mandate_ref_id| match mandate_ref_id {
                    api_models::payments::MandateReferenceId::ConnectorMandateId(id) => {
                        id.get_connector_mandate_id()
                    }
                    _ => None,
                },
            )
        })
    }

    fn get_return_url_required(
        &self,
    ) -> Result<String, error_stack::Report<errors::ConnectorError>> {
        self.get_router_return_url()
    }

    fn get_capture_method(&self) -> Option<CaptureMethod> {
        self.capture_method
    }
    fn get_currency(&self) -> enums::Currency {
        self.currency
    }
    fn get_payment_method_data_required(
        &self,
    ) -> Result<PaymentMethodData, error_stack::Report<errors::ConnectorError>> {
        Ok(self.payment_method_data.clone())
    }

    fn get_minor_amount_required(
        &self,
    ) -> Result<MinorUnit, error_stack::Report<errors::ConnectorError>> {
        self.minor_amount
            .ok_or_else(missing_field_err("minor_amount"))
    }

    fn get_is_partial_approval(&self) -> Option<PartialApprovalFlag> {
        self.enable_partial_authorization
            .map(PartialApprovalFlag::from)
    }

    fn get_email_required(&self) -> Result<Email, error_stack::Report<errors::ConnectorError>> {
        self.email.clone().ok_or_else(missing_field_err("email"))
    }
    fn is_customer_initiated_mandate_payment(&self) -> bool {
        (self.customer_acceptance.is_some() || self.setup_mandate_details.is_some())
            && self.setup_future_usage == Some(FutureUsage::OffSession)
    }
    fn get_is_stored_credential(&self) -> Option<StoredCredentialMode> {
        StoredCredentialMode::get_optional_stored_credential(self.is_stored_credential)
    }
}

impl NuveiAuthorizePreprocessingCommon for PaymentsAuthorizeData {
    fn get_browser_info(&self) -> Option<BrowserInformation> {
        self.browser_info.clone()
    }
    fn get_billing_descriptor(&self) -> Option<&BillingDescriptor> {
        self.billing_descriptor.as_ref()
    }
    fn get_ntid(&self) -> Option<String> {
        self.get_optional_network_transaction_id()
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
    fn get_auth_data(
        &self,
    ) -> Result<Option<AuthenticationData>, error_stack::Report<errors::ConnectorError>> {
        Ok(self.authentication_data.clone())
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

    fn get_complete_authorize_url(&self) -> Option<String> {
        self.complete_authorize_url.clone()
    }

    fn get_minor_amount_required(
        &self,
    ) -> Result<MinorUnit, error_stack::Report<errors::ConnectorError>> {
        Ok(self.minor_amount)
    }

    fn get_currency(&self) -> enums::Currency {
        self.currency
    }
    fn get_payment_method_data_required(
        &self,
    ) -> Result<PaymentMethodData, error_stack::Report<errors::ConnectorError>> {
        Ok(self.payment_method_data.clone())
    }

    fn get_email_required(&self) -> Result<Email, error_stack::Report<errors::ConnectorError>> {
        self.get_email()
    }
    fn is_customer_initiated_mandate_payment(&self) -> bool {
        (self.customer_acceptance.is_some() || self.setup_mandate_details.is_some())
            && self.setup_future_usage == Some(FutureUsage::OffSession)
    }
    fn get_is_partial_approval(&self) -> Option<PartialApprovalFlag> {
        self.enable_partial_authorization
            .map(PartialApprovalFlag::from)
    }

    fn get_is_stored_credential(&self) -> Option<StoredCredentialMode> {
        StoredCredentialMode::get_optional_stored_credential(self.is_stored_credential)
    }
}
#[derive(Debug, Serialize, Default, Deserialize)]
pub struct NuveiMeta {
    pub session_token: Secret<String>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiItem {
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: NuveiItemType,
    pub price: StringMajorUnit,
    pub quantity: String,
    pub group_id: Option<String>,
    pub discount: Option<StringMajorUnit>,
    pub tax: Option<StringMajorUnit>,
    pub tax_rate: Option<String>,
    pub image_url: Option<String>,
}
fn get_l2_l3_items(
    l2_l3_data: &Option<Box<L2L3Data>>,
    currency: enums::Currency,
) -> Result<Option<Vec<NuveiItem>>, error_stack::Report<errors::ConnectorError>> {
    l2_l3_data
        .as_ref()
        .and_then(|data| data.get_order_details())
        .map(|order_details_list| {
            order_details_list
                .iter()
                .map(|order| {
                    Ok(NuveiItem {
                        name: order.product_name.clone(),
                        item_type: order.product_type.clone().into(),
                        price: order.amount.to_nuvei_amount(currency)?,
                        quantity: order.quantity.to_string(),
                        group_id: order.product_id.clone(),
                        discount: order
                            .unit_discount_amount
                            .map(|amount| amount.to_nuvei_amount(currency))
                            .transpose()?,
                        tax: order
                            .total_tax_amount
                            .map(|amount| amount.to_nuvei_amount(currency))
                            .transpose()?,
                        tax_rate: order.tax_rate.map(|rate| rate.to_string()),
                        image_url: order.product_img_link.clone(),
                    })
                })
                .collect::<Result<Vec<_>, _>>()
        })
        .transpose()
}
fn get_amount_details(
    l2_l3_data: &Option<Box<L2L3Data>>,
    currency: enums::Currency,
) -> Result<Option<NuveiAmountDetails>, error_stack::Report<errors::ConnectorError>> {
    let cv = |a| convert_amount(&StringMajorUnitForConnector, a, currency);
    l2_l3_data
        .as_deref()
        .map(|d| {
            Ok(NuveiAmountDetails {
                total_tax: d.get_order_tax_amount().map(cv).transpose()?,
                total_shipping: d.get_shipping_cost().map(cv).transpose()?,
                total_discount: d.get_discount_amount().map(cv).transpose()?,
                total_handling: d.get_duty_amount().map(cv).transpose()?,
            })
        })
        .transpose()
}
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiAmountDetails {
    pub total_tax: Option<StringMajorUnit>,
    pub total_shipping: Option<StringMajorUnit>,
    pub total_handling: Option<StringMajorUnit>,
    pub total_discount: Option<StringMajorUnit>,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct UrlDetails {
    pub success_url: String,
    pub failure_url: String,
    pub pending_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "snake_case")]
pub enum NuveiItemType {
    #[default]
    Physical,
    Discount,
    #[serde(rename = "Shipping_fee")]
    ShippingFee,
    Digital,
    #[serde(rename = "Gift_card")]
    GiftCard,
    #[serde(rename = "Store_credit")]
    StoreCredit,
    Surcharge,
    #[serde(rename = "Sales_tax")]
    SalesTax,
}
impl From<Option<enums::ProductType>> for NuveiItemType {
    fn from(value: Option<enums::ProductType>) -> Self {
        match value {
            Some(enums::ProductType::Digital) => Self::Digital,
            Some(enums::ProductType::Physical) => Self::Physical,
            Some(enums::ProductType::Ride)
            | Some(enums::ProductType::Travel)
            | Some(enums::ProductType::Accommodation) => Self::ShippingFee,
            _ => Self::Physical,
        }
    }
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NuveiDynamicDescriptor {
    pub merchant_name: Option<Secret<String>>,
    pub merchant_phone: Option<Secret<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingAddress {
    pub email: Email,
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub country: api_models::enums::CountryAlpha2,
    pub phone: Option<Secret<String>>,
    pub city: Option<Secret<String>>,
    pub address: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub state: Option<Secret<String>>,
    pub address_line2: Option<Secret<String>>,
    pub address_line3: Option<Secret<String>>,
}

#[serde_with::skip_serializing_none]
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShippingAddress {
    pub first_name: Option<Secret<String>>,
    pub last_name: Option<Secret<String>>,
    pub address: Option<Secret<String>>,
    pub phone: Option<Secret<String>>,
    pub zip: Option<Secret<String>>,
    pub city: Option<Secret<String>>,
    pub country: api_models::enums::CountryAlpha2,
    pub email: Email,
    pub address_line2: Option<Secret<String>>,
    pub address_line3: Option<Secret<String>>,
}

impl From<&Address> for BillingAddress {
    fn from(address: &Address) -> Self {
        let address_details = address.address.as_ref();
        Self {
            email: address.email.clone().unwrap_or_default(),
            first_name: address.get_optional_first_name(),
            last_name: address_details.and_then(|address| address.get_optional_last_name()),
            country: address_details
                .and_then(|address| address.get_optional_country())
                .unwrap_or_default(),
            phone: address
                .phone
                .as_ref()
                .and_then(|phone| phone.number.clone()),
            city: address_details
                .and_then(|address| address.get_optional_city().map(|city| city.into())),
            address: address_details.and_then(|address| address.get_optional_line1()),
            zip: address_details.and_then(|details| details.get_optional_zip()),
            state: address_details.and_then(|details| details.to_state_code_as_optional().ok()?),
            address_line2: address_details.and_then(|address| address.get_optional_line2()),
            address_line3: address_details.and_then(|address| address.get_optional_line3()),
        }
    }
}

impl From<&Address> for ShippingAddress {
    fn from(address: &Address) -> Self {
        let address_details = address.address.as_ref();

        Self {
            email: address.email.clone().unwrap_or_default(),
            first_name: address_details.and_then(|details| details.get_optional_first_name()),
            last_name: address_details.and_then(|details| details.get_optional_last_name()),
            country: address_details
                .and_then(|details| details.get_optional_country())
                .unwrap_or_default(),
            phone: address
                .phone
                .as_ref()
                .and_then(|phone| phone.number.clone()),
            city: address_details
                .and_then(|details| details.get_optional_city().map(|city| city.into())),
            address: address_details.and_then(|details| details.get_optional_line1()),
            zip: address_details.and_then(|details| details.get_optional_zip()),
            address_line2: address_details.and_then(|details| details.get_optional_line2()),
            address_line3: address_details.and_then(|details| details.get_optional_line3()),
        }
    }
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
