#[cfg(feature = "payouts")]
use api_models::payouts::{ApplePayDecrypt, CardPayout};
use base64::Engine;
use common_enums::enums;
#[cfg(feature = "payouts")]
use common_utils::pii;
use common_utils::types::StringMinorUnit;
use error_stack::ResultExt;
use http::HeaderMap;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    address::Address,
    router_flow_types::payouts::{PoCancel, PoFulfill, PoSync},
    router_response_types::PayoutsResponseData,
    types::PayoutsRouterData,
};
use hyperswitch_domain_models::{
    payment_method_data::{
        ApplePayWalletData, Card, GooglePayWalletData, PaymentMethodData, WalletData,
    },
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsSyncData, ResponseId,
        SetupMandateRequestData,
    },
    router_response_types::{
        MandateReference, PaymentsResponseData, RedirectForm, RefundsResponseData,
    },
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsSyncRouterData, RefundSyncRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors};
use josekit;
use masking::{ExposeInterface, Secret, WithType};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[cfg(feature = "payouts")]
use crate::types::PayoutsResponseRouterData;
use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        self as connector_utils, AddressDetailsData, CardData, ForeignTryFrom,
        PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        PaymentsSyncRequestData, RouterData as _,
    },
};

pub struct WorldpayxmlRouterData<T> {
    pub amount: StringMinorUnit,
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for WorldpayxmlRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub mod worldpayxml_constants {
    pub const WORLDPAYXML_VERSION: &str = "1.4";
    pub const XML_VERSION: &str = "1.0";
    pub const XML_ENCODING: &str = "UTF-8";
    pub const WORLDPAYXML_DOC_TYPE: &str = r#"paymentService PUBLIC "-//Worldpay//DTD Worldpay PaymentService v1//EN" "http://dtd.worldpay.com/paymentService_v1.dtd""#;
    pub const MAX_PAYMENT_REFERENCE_ID_LENGTH: usize = 64;
    pub const COOKIE: &str = "cookie";
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "paymentService")]
pub struct PaymentService {
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "@merchantCode")]
    merchant_code: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    submit: Option<Submit>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<Reply>,
    #[serde(skip_serializing_if = "Option::is_none")]
    inquiry: Option<Inquiry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    modify: Option<Modify>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Modify {
    order_modification: OrderModification,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderModification {
    #[serde(rename = "@orderCode")]
    order_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    capture: Option<CaptureRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cancel: Option<CancelRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cancel_refund: Option<CancelRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    refund: Option<RefundRequest>,
}

#[derive(Debug, Serialize, Deserialize)]
struct RefundRequest {
    amount: WorldpayXmlAmount,
}

#[derive(Debug, Serialize, Deserialize)]
struct CancelRequest {}

#[derive(Debug, Serialize, Deserialize)]
struct CaptureRequest {
    amount: WorldpayXmlAmount,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Inquiry {
    order_inquiry: OrderInquiry,
}

#[derive(Debug, Serialize, Deserialize)]
struct OrderInquiry {
    #[serde(rename = "@orderCode")]
    order_code: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Submit {
    order: Order,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    order_status: Option<OrderStatus>,
    pub error: Option<WorldpayXmlErrorResponse>,
    ok: Option<OkResponse>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutResponse {
    reply: PayoutReply,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PayoutReply {
    ok: Option<OkPayoutResponse>,
    order_status: Option<OrderStatus>,
    error: Option<WorldpayXmlErrorResponse>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OkPayoutResponse {
    refund_received: Option<ModifyRequestReceived>,
    cancel_received: Option<ModifyRequestReceived>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OkResponse {
    capture_received: Option<ModifyRequestReceived>,
    cancel_received: Option<ModifyRequestReceived>,
    refund_received: Option<ModifyRequestReceived>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ModifyRequestReceived {
    #[serde(rename = "@orderCode")]
    order_code: String,
    amount: Option<WorldpayXmlAmount>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WorldpayXmlErrorResponse {
    #[serde(rename = "@code")]
    pub code: String,
    #[serde(rename = "$value")]
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderStatus {
    #[serde(rename = "@orderCode")]
    order_code: String,
    challenge_required: Option<ChallengeRequired>,
    payment: Option<Payment>,
    token: Option<Token>,
    error: Option<WorldpayXmlErrorResponse>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Token {
    authenticated_shopper_i_d: String,
    token_details: TokenDetails,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct TokenDetails {
    #[serde(rename = "@tokenEvent")]
    token_event: String,
    payment_token_i_d: Secret<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChallengeRequired {
    #[serde(rename = "threeDSChallengeDetails")]
    three_ds_challenge_details: Option<ThreeDSChallengeDetails>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ThreeDSChallengeDetails {
    #[serde(rename = "threeDSVersion")]
    three_ds_version: Option<String>,
    #[serde(rename = "acsURL")]
    acs_url: Option<String>,
    #[serde(rename = "transactionId3DS")]
    transaction_id_3ds: Option<String>,
    payload: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Payment {
    payment_method: String,
    amount: WorldpayXmlAmount,
    pub last_event: LastEvent,
    #[serde(rename = "AuthorisationId")]
    authorisation_id: Option<AuthorisationId>,
    scheme_response: Option<SchemeResponse>,
    payment_method_detail: Option<PaymentMethodDetail>,
    #[serde(rename = "CVCResultCode")]
    cvc_result_code: Option<ResultCode>,
    #[serde(rename = "AVSResultCode")]
    avs_result_code: Option<ResultCode>,
    #[serde(rename = "AAVAddressResultCode")]
    aav_address_result_code: Option<ResultCode>,
    #[serde(rename = "AAVPostcodeResultCode")]
    aav_postcode_result_code: Option<ResultCode>,
    #[serde(rename = "AAVCardholderNameResultCode")]
    aav_cardholder_name_result_code: Option<ResultCode>,
    #[serde(rename = "AAVTelephoneResultCode")]
    aav_telephone_result_code: Option<ResultCode>,
    #[serde(rename = "AAVEmailResultCode")]
    aav_email_result_code: Option<ResultCode>,
    #[serde(rename = "ThreeDSecureResult")]
    three_d_secure_result: Option<ResultCode>,
    issuer_country_code: Option<String>,
    issuer_name: Option<String>,
    balance: Option<Vec<Balance>>,
    card_holder_name: Option<String>,
    fast_funds: Option<bool>,
    #[serde(rename = "ISO8583ReturnCode")]
    return_code: Option<ReturnCode>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReturnCode {
    #[serde(rename = "@description")]
    description: String,
    #[serde(rename = "@code")]
    code: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ResultCode {
    #[serde(rename = "@description")]
    description: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Balance {
    #[serde(rename = "@accountType")]
    account_type: String,
    amount: WorldpayXmlAmount,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct PaymentMethodDetail {
    card: CardResponse,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CardResponse {
    #[serde(rename = "@number")]
    number: Option<Secret<String>>,
    #[serde(rename = "@type")]
    card_type: String,
    expiry_date: Option<ExpiryDate>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AuthorisationId {
    #[serde(rename = "@id")]
    id: Secret<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LastEvent {
    Authorised,
    Refused,
    Cancelled,
    Captured,
    Settled,
    SentForAuthorisation,
    SentForRefund,
    SentForFastRefund,
    Refunded,
    RefundRequested,
    RefundFailed,
    RefundedByMerchant,
    Error,
    QueryRequired,
    CancelReceived,
    RefundReceived,
    PushApproved,
    PushPending,
    PushRequested,
    PushRefused,
    SettledByMerchant,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct SchemeResponse {
    transaction_identifier: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Order {
    #[serde(rename = "@orderCode")]
    order_code: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "@captureDelay")]
    capture_delay: Option<AutoCapture>,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    amount: Option<WorldpayXmlAmount>,
    #[serde(skip_serializing_if = "Option::is_none")]
    payment_details: Option<PaymentDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shopper: Option<WorldpayxmlShopper>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shipping_address: Option<WorldpayxmlPayinAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    billing_address: Option<WorldpayxmlPayinAddress>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "additional3DSData")]
    additional_threeds_data: Option<AdditionalThreeDSData>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "info3DSecure")]
    info_threed_secure: Option<Info3DSecure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<CompleteAuthSession>,
    #[serde(skip_serializing_if = "Option::is_none")]
    create_token: Option<CreateToken>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Info3DSecure {
    completed_authentication: CompletedAuthentication,
}

#[derive(Debug, Serialize, Deserialize)]
struct CompletedAuthentication {}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompleteAuthSession {
    #[serde(rename = "@id")]
    id: Secret<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorldpayxmlShopper {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shopper_email_address: Option<pii::Email>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authenticated_shopper_i_d: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser: Option<WPGBrowserData>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WPGBrowserData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accept_header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent_header: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_accept_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http_referer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_java_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_java_script_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_colour_depth: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_screen_height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub browser_screen_width: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldpayxmlPayinAddress {
    address: WorldpayxmlAddressData,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldpayxmlAddressData {
    #[serde(skip_serializing_if = "Option::is_none")]
    first_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    last_name: Option<Secret<String>>,
    address1: Secret<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address2: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address3: Option<Secret<String>>,
    postal_code: Secret<String>,
    city: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    state: Option<Secret<String>>,
    country_code: common_enums::CountryAlpha2,
}

#[derive(Debug, Serialize, Deserialize)]
struct AdditionalThreeDSData {
    #[serde(rename = "@dfReferenceId")]
    df_reference_id: Option<String>,
    #[serde(rename = "@javaScriptEnabled")]
    javascript_enabled: bool,
    #[serde(rename = "@deviceChannel")]
    device_channel: String,
    #[serde(rename = "@challengePreference")]
    challenge_preference: ChallengePreference,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ChallengePreference {
    NoChallengeRequested,
    ChallengeRequested,
    ChallengeMandated,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "UPPERCASE")]
enum AutoCapture {
    Off,
    #[serde(rename = "0")]
    On,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorldpayXmlAmount {
    #[serde(rename = "@value")]
    value: StringMinorUnit,
    #[serde(rename = "@currencyCode")]
    currency_code: api_models::enums::Currency,
    #[serde(rename = "@exponent")]
    exponent: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentDetails {
    #[serde(skip_serializing_if = "Option::is_none", rename = "@action")]
    action: Option<Action>,
    #[serde(flatten)]
    payment_method: PaymentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<Session>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stored_credentials: Option<StoredCredentials>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredCredentials {
    #[serde(rename = "@usage")]
    usage: UsageType,
    #[serde(
        rename = "@customerInitiatedReason",
        skip_serializing_if = "Option::is_none"
    )]
    customer_initiated_reason: Option<MandateType>,
    #[serde(
        rename = "@merchantInitiatedReason",
        skip_serializing_if = "Option::is_none"
    )]
    merchant_initiated_reason: Option<MandateType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    scheme_transaction_identifier: Option<Secret<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum UsageType {
    First,
    Used,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum MandateType {
    Recurring,
    Unscheduled,
    Instalment,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateToken {
    #[serde(rename = "@tokenScope")]
    token_scope: String,
    token_event_reference: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Session {
    #[serde(rename = "@id")]
    id: String,
    #[serde(rename = "@shopperIPAddress")]
    shopper_ip_address: Secret<String, pii::IpAddress>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum PaymentMethod {
    #[serde(rename = "CARD-SSL")]
    CardSSL(CardSSL),

    #[serde(rename = "FF_DISBURSE-SSL")]
    FastAccessSSL(FastAccessData),

    #[serde(rename = "PAYWITHGOOGLE-SSL")]
    PayWithGoogleSSL(GooglePayData),

    #[serde(rename = "APPLEPAY-SSL")]
    PayWithAppleSSL(ApplePayData),

    #[serde(rename = "TOKEN-SSL")]
    TokenSSL(TokenData),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TokenData {
    #[serde(rename = "@tokenScope")]
    token_scope: Secret<String>,
    payment_token_i_d: Secret<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FastAccessData {
    recipient: Recipient,
    #[serde(skip_serializing_if = "Option::is_none")]
    purpose_of_payment: Option<String>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Recipient {
    payment_instrument: PaymentInstrument,
    address: Option<WorldpayxmlAddressData>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PaymentInstrument {
    card_details: CardDetails,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Serialize, Deserialize)]
struct CardDetails {
    #[serde(flatten)]
    card_ssl: CardSSL,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CardSSL {
    card_number: cards::CardNumber,
    expiry_date: ExpiryDate,
    card_holder_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cvc: Option<Secret<String>>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "expiryDate")]
struct ExpiryDate {
    date: Date,
}

#[derive(Debug, Deserialize, Serialize)]
struct Date {
    #[serde(rename = "@month")]
    month: Secret<String>,
    #[serde(rename = "@year")]
    year: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GooglePayData {
    protocol_version: Secret<String>,
    signature: Secret<String>,
    signed_message: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplePayData {
    header: ApplePayHeader,
    signature: Secret<String>,
    version: Secret<String>,
    data: Secret<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApplePayHeader {
    ephemeral_public_key: Secret<String>,
    public_key_hash: Secret<String>,
    transaction_id: Secret<String>,
}

#[cfg(feature = "payouts")]
impl TryFrom<LastEvent> for enums::PayoutStatus {
    type Error = errors::ConnectorError;
    fn try_from(item: LastEvent) -> Result<Self, Self::Error> {
        match item {
            LastEvent::PushRequested => Ok(Self::Initiated),
            LastEvent::PushPending => Ok(Self::Pending),
            LastEvent::Error | LastEvent::PushRefused => Ok(Self::Failed),
            LastEvent::PushApproved | LastEvent::SettledByMerchant => Ok(Self::Success),
            LastEvent::CancelReceived => Ok(Self::Cancelled),
            _ => Err(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Invalid LastEvent".to_string()),
            )),
        }
    }
}

#[cfg(feature = "payouts")]
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpayxmlPayoutConnectorMetadataObject {
    pub purpose_of_payment: Option<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct WorldpayxmlConnectorMetadataObject {
    pub issuer_id: Option<String>,
    pub organizational_unit_id: Option<String>,
    pub jwt_mac_key: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Action {
    Authorise,
    Sale,
    Refund,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum WorldpayxmlSyncResponse {
    Webhook(Box<WorldpayFormWebhookBody>),
    Payment(Box<PaymentService>),
}

impl TryFrom<(&Card, Option<enums::CaptureMethod>, Option<Session>)> for PaymentDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (card_data, capture_method, session): (
            &Card,
            Option<enums::CaptureMethod>,
            Option<Session>,
        ),
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            action: if connector_utils::is_manual_capture(capture_method) {
                Some(Action::Authorise)
            } else {
                Some(Action::Sale)
            },
            payment_method: PaymentMethod::CardSSL(CardSSL {
                card_number: card_data.card_number.clone(),
                expiry_date: ExpiryDate {
                    date: Date {
                        month: card_data.get_card_expiry_month_2_digit()?,
                        year: card_data.get_expiry_year_4_digit(),
                    },
                },
                card_holder_name: card_data.card_holder_name.to_owned(),
                cvc: Some(card_data.card_cvc.to_owned()),
            }),
            session,
            stored_credentials: None,
        })
    }
}

impl TryFrom<PaymentsAuthorizeData> for PaymentDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: PaymentsAuthorizeData) -> Result<Self, Self::Error> {
        let stored_credentials = Some(StoredCredentials {
            usage: UsageType::Used,
            customer_initiated_reason: None,
            merchant_initiated_reason: Some(get_mandate_type(item.mit_category)),
            scheme_transaction_identifier: Some(
                item.get_connector_mandate_request_reference_id()?.into(),
            ),
        });

        Ok(Self {
            action: None,
            payment_method: PaymentMethod::TokenSSL(TokenData {
                token_scope: Secret::new("shopper".to_string()),
                payment_token_i_d: Secret::new(item.get_connector_mandate_id()?),
            }),
            session: None,
            stored_credentials,
        })
    }
}

impl TryFrom<(&GooglePayWalletData, PaymentsAuthorizeData)> for PaymentDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (gpay_data, item): (&GooglePayWalletData, PaymentsAuthorizeData),
    ) -> Result<Self, Self::Error> {
        let token_string = gpay_data
            .tokenization_data
            .get_encrypted_google_pay_token()
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "gpay wallet_token",
            })?
            .to_owned();

        let parsed_token = serde_json::from_str::<GooglePayData>(&token_string)
            .change_context(errors::ConnectorError::ParsingFailed)?;

        let stored_credentials = if item.is_cit_mandate_payment() {
            Some(StoredCredentials {
                usage: UsageType::First,
                customer_initiated_reason: Some(get_mandate_type(item.mit_category)),
                merchant_initiated_reason: None,
                scheme_transaction_identifier: None,
            })
        } else {
            None
        };

        Ok(Self {
            action: if connector_utils::is_manual_capture(item.capture_method) {
                Some(Action::Authorise)
            } else {
                Some(Action::Sale)
            },
            payment_method: PaymentMethod::PayWithGoogleSSL(GooglePayData {
                protocol_version: parsed_token.protocol_version,
                signature: parsed_token.signature,
                signed_message: parsed_token.signed_message.clone(),
            }),
            session: None,
            stored_credentials,
        })
    }
}

impl TryFrom<(&ApplePayWalletData, PaymentsAuthorizeData)> for PaymentDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (apple_pay_wallet_data, item): (&ApplePayWalletData, PaymentsAuthorizeData),
    ) -> Result<Self, Self::Error> {
        let applepay_encrypt_data = apple_pay_wallet_data
            .payment_data
            .get_encrypted_apple_pay_payment_data_mandatory()
            .change_context(errors::ConnectorError::MissingRequiredField {
                field_name: "Apple pay encrypted data",
            })?;

        let decoded_data = base64::prelude::BASE64_STANDARD
            .decode(applepay_encrypt_data)
            .change_context(errors::ConnectorError::InvalidDataFormat {
                field_name: "apple_pay_encrypted_data",
            })?;

        let apple_pay_token: ApplePayData = serde_json::from_slice(&decoded_data).change_context(
            errors::ConnectorError::InvalidDataFormat {
                field_name: "apple_pay_token_json",
            },
        )?;

        let stored_credentials = if item.is_cit_mandate_payment() {
            Some(StoredCredentials {
                usage: UsageType::First,
                customer_initiated_reason: Some(get_mandate_type(item.mit_category)),
                merchant_initiated_reason: None,
                scheme_transaction_identifier: None,
            })
        } else {
            None
        };

        Ok(Self {
            action: if connector_utils::is_manual_capture(item.capture_method) {
                Some(Action::Authorise)
            } else {
                Some(Action::Sale)
            },
            payment_method: PaymentMethod::PayWithAppleSSL(apple_pay_token),
            session: None,
            stored_credentials,
        })
    }
}

fn get_address_details(data: &Address) -> Option<WorldpayxmlPayinAddress> {
    let address1_option = data
        .address
        .as_ref()
        .and_then(|address| address.get_optional_line1());
    let postal_code_option = data
        .address
        .as_ref()
        .and_then(|address| address.get_optional_zip());
    let country_code_option = data
        .address
        .as_ref()
        .and_then(|address| address.get_optional_country());
    let city_option = data
        .address
        .as_ref()
        .and_then(|address| address.get_optional_city());

    if let (Some(address1), Some(postal_code), Some(country_code), Some(city), Some(address_data)) = (
        address1_option,
        postal_code_option,
        country_code_option,
        city_option,
        data.address.as_ref(),
    ) {
        Some(WorldpayxmlPayinAddress {
            address: WorldpayxmlAddressData {
                first_name: address_data.get_optional_first_name(),
                last_name: address_data.get_optional_last_name(),
                address1,
                address2: address_data.get_optional_line2(),
                address3: address_data.get_optional_line2(),
                postal_code,
                city,
                state: address_data.get_optional_state(),
                country_code,
            },
        })
    } else {
        None
    }
}

fn get_shopper_details(
    item: &PaymentsAuthorizeRouterData,
    accept_header: Option<String>,
    user_agent_header: Option<String>,
) -> Result<Option<WorldpayxmlShopper>, error_stack::Report<errors::ConnectorError>> {
    let shopper_email = item.request.email.clone();
    let browser_info = item
        .request
        .browser_info
        .clone()
        .as_ref()
        .map(|browser_info| WPGBrowserData {
            accept_header,
            http_accept_language: browser_info.accept_language.clone(),
            http_referer: browser_info.referer.clone(),
            browser_language: browser_info.language.clone(),
            browser_java_enabled: browser_info.java_enabled,
            browser_java_script_enabled: browser_info.java_script_enabled,
            browser_colour_depth: browser_info.color_depth,
            browser_screen_height: browser_info.screen_height,
            browser_screen_width: browser_info.screen_width,
            user_agent_header,
            time_zone: browser_info.time_zone,
        });

    let authenticated_shopper_i_d =
        if item.request.payment_method_data == PaymentMethodData::MandatePayment {
            let mandate_data = item.request.get_connector_mandate_data().ok_or(
                errors::ConnectorError::MissingRequiredField {
                    field_name: "mandate_data",
                },
            )?;

            let metadata = mandate_data.get_mandate_metadata().ok_or_else(|| {
                errors::ConnectorError::MissingRequiredField {
                    field_name: "mandate_metadata",
                }
            })?;

            let customer_id = metadata
                .expose()
                .get("customer_id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
                    field_name: "customer_id in metadata",
                })?
                .to_owned();

            Some(Secret::new(customer_id))
        } else {
            item.request
                .is_cit_mandate_payment()
                .then(|| {
                    item.get_customer_id()
                        .change_context(errors::ConnectorError::MissingRequiredField {
                            field_name: "customer_id for authenticatedShopperID",
                        })
                        .map(|cid| cid.get_string_repr().to_owned())
                        .map(Secret::new)
                })
                .transpose()?
        };

    if shopper_email.is_some() || browser_info.is_some() || authenticated_shopper_i_d.is_some() {
        Ok(Some(WorldpayxmlShopper {
            shopper_email_address: shopper_email,
            browser: browser_info,
            authenticated_shopper_i_d,
        }))
    } else {
        Ok(None)
    }
}

impl TryFrom<&WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = WorldpayxmlAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let order_code = if item.router_data.connector_request_reference_id.len()
            <= worldpayxml_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH
        {
            Ok(item.router_data.connector_request_reference_id.clone())
        } else {
            Err(errors::ConnectorError::MaxFieldLengthViolated {
                connector: "Worldpayxml".to_string(),
                field_name: "order_code".to_string(),
                max_length: worldpayxml_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH,
                received_length: item.router_data.connector_request_reference_id.len(),
            })
        }?;

        let capture_delay = if item.router_data.request.is_auto_capture()? {
            Some(AutoCapture::On)
        } else {
            Some(AutoCapture::Off)
        };
        let description = item.router_data.description.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "description",
            },
        )?;

        let is_three_ds = item.router_data.is_three_ds();
        let (additional_threeds_data, session, accept_header, user_agent_header) =
            if is_three_ds && item.router_data.request.is_card() {
                let additional_threeds_data = Some(AdditionalThreeDSData {
                    df_reference_id: None,
                    javascript_enabled: false,
                    device_channel: "Browser".to_string(),
                    challenge_preference: ChallengePreference::ChallengeRequested,
                });
                let browser_info = item.router_data.request.get_browser_info()?;
                let accept_header = browser_info.accept_header.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "browser_info.accept_header",
                    },
                )?;
                let user_agent_header = browser_info.user_agent.ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "browser_info.user_agent",
                    },
                )?;

                let session = Some(Session {
                    id: item.router_data.connector_request_reference_id.clone(),
                    shopper_ip_address: item.router_data.request.get_ip_address()?,
                });

                (
                    additional_threeds_data,
                    session,
                    Some(accept_header),
                    Some(user_agent_header),
                )
            } else {
                let accept_header = item
                    .router_data
                    .request
                    .browser_info
                    .as_ref()
                    .and_then(|info| info.accept_header.clone());
                let user_agent_header = item
                    .router_data
                    .request
                    .browser_info
                    .as_ref()
                    .and_then(|info| info.user_agent.clone());

                (None, None, accept_header, user_agent_header)
            };

        let exponent = item
            .router_data
            .request
            .currency
            .number_of_digits_after_decimal_point()
            .to_string();
        let amount = WorldpayXmlAmount {
            currency_code: item.router_data.request.currency.to_owned(),
            exponent,
            value: item.amount.to_owned(),
        };
        let shopper = get_shopper_details(item.router_data, accept_header, user_agent_header)?;
        let billing_address = item
            .router_data
            .get_optional_billing()
            .and_then(get_address_details);
        let shipping_address = item
            .router_data
            .get_optional_shipping()
            .and_then(get_address_details);

        let payment_details = match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => PaymentDetails::try_from((
                &req_card,
                item.router_data.request.capture_method,
                session,
            ))?,
            PaymentMethodData::Wallet(wallet_data) => match wallet_data {
                WalletData::GooglePay(google_pay_data) => {
                    PaymentDetails::try_from((&google_pay_data, item.router_data.request.clone()))?
                }
                WalletData::ApplePay(apple_pay_data) => {
                    PaymentDetails::try_from((&apple_pay_data, item.router_data.request.clone()))?
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    connector_utils::get_unimplemented_payment_method_error_message("Worldpayxml"),
                ))?,
            },
            PaymentMethodData::MandatePayment => {
                PaymentDetails::try_from(item.router_data.request.clone())?
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                connector_utils::get_unimplemented_payment_method_error_message("Worldpayxml"),
            ))?,
        };

        let create_token = if item.router_data.request.is_cit_mandate_payment() {
            Some(CreateToken {
                token_scope: "shopper".to_string(),
                token_event_reference: item.router_data.connector_request_reference_id.clone(),
            })
        } else {
            None
        };

        let submit = Some(Submit {
            order: Order {
                order_code,
                capture_delay,
                description: Some(description),
                amount: Some(amount),
                payment_details: Some(payment_details),
                shopper,
                shipping_address,
                billing_address,
                additional_threeds_data,
                info_threed_secure: None,
                session: None,
                create_token,
            },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit,
            reply: None,
            inquiry: None,
            modify: None,
        })
    }
}

impl TryFrom<&WorldpayxmlRouterData<&PaymentsCaptureRouterData>> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayxmlRouterData<&PaymentsCaptureRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = WorldpayxmlAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let modify = Some(Modify {
            order_modification: OrderModification {
                order_code: item.router_data.request.connector_transaction_id.clone(),
                capture: Some(CaptureRequest {
                    amount: WorldpayXmlAmount {
                        currency_code: item.router_data.request.currency.to_owned(),
                        exponent: item
                            .router_data
                            .request
                            .currency
                            .number_of_digits_after_decimal_point()
                            .to_string(),
                        value: item.amount.to_owned(),
                    },
                }),
                cancel_refund: None,
                cancel: None,
                refund: None,
            },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit: None,
            reply: None,
            inquiry: None,
            modify,
        })
    }
}

impl TryFrom<&PaymentsCancelRouterData> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        let auth = WorldpayxmlAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let modify = Some(Modify {
            order_modification: OrderModification {
                order_code: item.request.connector_transaction_id.clone(),
                capture: None,
                cancel_refund: None,
                cancel: Some(CancelRequest {}),
                refund: None,
            },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit: None,
            reply: None,
            inquiry: None,
            modify,
        })
    }
}

impl<F> TryFrom<&WorldpayxmlRouterData<&RefundsRouterData<F>>> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &WorldpayxmlRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        let auth = WorldpayxmlAuthType::try_from(&item.router_data.connector_auth_type)?;

        let modify = Some(Modify {
            order_modification: OrderModification {
                order_code: item.router_data.request.connector_transaction_id.clone(),
                capture: None,
                cancel: None,
                cancel_refund: None,
                refund: Some(RefundRequest {
                    amount: WorldpayXmlAmount {
                        currency_code: item.router_data.request.currency.to_owned(),
                        exponent: item
                            .router_data
                            .request
                            .currency
                            .number_of_digits_after_decimal_point()
                            .to_string(),
                        value: item.amount.to_owned(),
                    },
                }),
            },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit: None,
            reply: None,
            inquiry: None,
            modify,
        })
    }
}

impl TryFrom<RefundsResponseRouterData<Execute, PaymentService>> for RefundsRouterData<Execute> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, PaymentService>,
    ) -> Result<Self, Self::Error> {
        let reply = item
            .response
            .reply
            .ok_or(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Missing reply data".to_string()),
            ))?;

        if let Some(refund_received) = reply.ok.and_then(|ok| ok.refund_received) {
            Ok(Self {
                response: Ok(RefundsResponseData {
                    connector_refund_id: refund_received.order_code,
                    refund_status: enums::RefundStatus::Pending,
                }),
                ..item.data
            })
        } else {
            let error = reply
                .error
                .ok_or(errors::ConnectorError::UnexpectedResponseError(
                    bytes::Bytes::from(
                        "Either refund_received or error must be present in the response"
                            .to_string(),
                    ),
                ))?;

            Ok(Self {
                response: Err(ErrorResponse {
                    code: error.code,
                    message: error.message.clone(),
                    reason: Some(error.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })
        }
    }
}

pub struct WorldpayxmlAuthType {
    pub(super) api_username: Secret<String>,
    pub(super) api_password: Secret<String>,
    pub(super) merchant_code: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for WorldpayxmlAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::SignatureKey {
                api_key,
                key1,
                api_secret,
            } => Ok(Self {
                api_username: api_key.to_owned(),
                api_password: key1.to_owned(),
                merchant_code: api_secret.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum WorldpayxmlPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

fn get_attempt_status(
    is_auto_capture: bool,
    last_event: LastEvent,
    previous_status: Option<&common_enums::AttemptStatus>,
) -> Result<common_enums::AttemptStatus, errors::ConnectorError> {
    match last_event {
        LastEvent::Authorised => {
            if is_auto_capture {
                Ok(common_enums::AttemptStatus::Pending)
            } else if previous_status == Some(&common_enums::AttemptStatus::CaptureInitiated)
                && !is_auto_capture
            {
                Ok(common_enums::AttemptStatus::CaptureInitiated)
            } else if previous_status == Some(&common_enums::AttemptStatus::VoidInitiated)
                && !is_auto_capture
            {
                Ok(common_enums::AttemptStatus::VoidInitiated)
            } else {
                Ok(common_enums::AttemptStatus::Authorized)
            }
        }
        LastEvent::Refused => Ok(common_enums::AttemptStatus::Failure),
        LastEvent::Cancelled => Ok(common_enums::AttemptStatus::Voided),
        LastEvent::Captured | LastEvent::Settled | LastEvent::SettledByMerchant => {
            Ok(common_enums::AttemptStatus::Charged)
        }
        LastEvent::SentForAuthorisation => Ok(common_enums::AttemptStatus::Authorizing),
        _ => Err(errors::ConnectorError::UnexpectedResponseError(
            bytes::Bytes::from("Invalid LastEvent".to_string()),
        )),
    }
}

fn get_attempt_status_for_setup_mandate(
    last_event: LastEvent,
) -> Result<common_enums::AttemptStatus, errors::ConnectorError> {
    match last_event {
        LastEvent::Refused => Ok(common_enums::AttemptStatus::Failure),
        LastEvent::Cancelled => Ok(common_enums::AttemptStatus::Voided),
        LastEvent::Authorised
        | LastEvent::Captured
        | LastEvent::Settled
        | LastEvent::SettledByMerchant => Ok(common_enums::AttemptStatus::Charged),
        LastEvent::SentForAuthorisation => Ok(common_enums::AttemptStatus::Authorizing),
        _ => Err(errors::ConnectorError::UnexpectedResponseError(
            bytes::Bytes::from("Invalid LastEvent".to_string()),
        )),
    }
}

fn get_refund_status(last_event: LastEvent) -> Result<enums::RefundStatus, errors::ConnectorError> {
    match last_event {
        LastEvent::Refunded => Ok(enums::RefundStatus::Success),
        LastEvent::SentForRefund
        | LastEvent::RefundRequested
        | LastEvent::SentForFastRefund
        | LastEvent::RefundedByMerchant => Ok(enums::RefundStatus::Pending),
        LastEvent::RefundFailed => Ok(enums::RefundStatus::Failure),
        LastEvent::Captured | LastEvent::Settled => Ok(enums::RefundStatus::Pending),
        _ => Err(errors::ConnectorError::UnexpectedResponseError(
            bytes::Bytes::from("Invalid LastEvent".to_string()),
        )),
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, WorldpayxmlSyncResponse, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            F,
            WorldpayxmlSyncResponse,
            PaymentsSyncData,
            PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        match item.response {
            WorldpayxmlSyncResponse::Payment(data) => {
                let is_auto_capture = item.data.request.is_auto_capture()?;
                let reply = data
                    .reply
                    .ok_or(errors::ConnectorError::UnexpectedResponseError(
                        bytes::Bytes::from("Missing reply data".to_string()),
                    ))?;

                validate_reply(&reply)?;
                if let Some(order_status) = reply.order_status {
                    validate_order_status(&order_status)?;

                    if let Some(payment_data) = order_status.payment {
                        let status = get_attempt_status(
                            is_auto_capture,
                            payment_data.last_event,
                            Some(&item.data.status),
                        )?;
                        let response = process_payment_response(
                            status,
                            &payment_data,
                            item.http_code,
                            order_status.order_code.clone(),
                            order_status.token,
                        )
                        .map_err(|err| *err);

                        Ok(Self {
                            status,
                            response,
                            ..item.data
                        })
                    } else {
                        order_status.error
                                .ok_or(errors::ConnectorError::UnexpectedResponseError(
                                    bytes::Bytes::from("Either order_status.payment or order_status.error must be present in the response".to_string()),
                                ))?;
                        // Handle API errors unrelated to the payment to prevent failing the payment.
                        Ok(Self {
                            status: item.data.status,
                            response: Ok(PaymentsResponseData::TransactionResponse {
                                resource_id: ResponseId::ConnectorTransactionId(
                                    order_status.order_code.clone(),
                                ),
                                redirection_data: Box::new(None),
                                mandate_reference: Box::new(None),
                                connector_metadata: None,
                                network_txn_id: None,
                                connector_response_reference_id: Some(
                                    order_status.order_code.clone(),
                                ),
                                incremental_authorization_allowed: None,
                                authentication_data: None,
                                charges: None,
                            }),
                            ..item.data
                        })
                    }
                } else {
                    // Handle API errors unrelated to the payment to prevent failing the payment
                    Ok(Self {
                        status: item.data.status,
                        response: Ok(PaymentsResponseData::TransactionResponse {
                            resource_id: item.data.request.connector_transaction_id.clone(),
                            redirection_data: Box::new(None),
                            mandate_reference: Box::new(None),
                            connector_metadata: None,
                            network_txn_id: None,
                            connector_response_reference_id: None,
                            incremental_authorization_allowed: None,
                            authentication_data: None,
                            charges: None,
                        }),
                        ..item.data
                    })
                }
            }
            WorldpayxmlSyncResponse::Webhook(data) => {
                let is_auto_capture = item.data.request.is_auto_capture()?;

                let status = get_attempt_status(
                    is_auto_capture,
                    data.payment_status,
                    Some(&item.data.status),
                )?;

                Ok(Self {
                    status,
                    response: Ok(PaymentsResponseData::TransactionResponse {
                        resource_id: ResponseId::ConnectorTransactionId(data.order_code.clone()),
                        redirection_data: Box::new(None),
                        mandate_reference: Box::new(None),
                        connector_metadata: None,
                        network_txn_id: None,
                        connector_response_reference_id: Some(data.order_code.clone()),
                        incremental_authorization_allowed: None,
                        charges: None,
                        authentication_data: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<&PaymentsSyncRouterData> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsSyncRouterData) -> Result<Self, Self::Error> {
        let auth = WorldpayxmlAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let order_code = item
            .request
            .connector_transaction_id
            .get_connector_transaction_id()
            .change_context(errors::ConnectorError::MissingConnectorTransactionID)?;

        let inquiry = Some(Inquiry {
            order_inquiry: OrderInquiry { order_code },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit: None,
            reply: None,
            inquiry,
            modify: None,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct Payload {
    #[serde(rename = "ACSUrl")]
    acs_url: String,
    #[serde(rename = "Payload")]
    payload: String,
    #[serde(rename = "TransactionId")]
    transaction_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ChallengeJwt {
    jti: String,
    iat: u64,
    iss: String,
    #[serde(rename = "OrgUnitId")]
    org_unit_id: String,
    #[serde(rename = "ReturnUrl")]
    return_url: String,
    #[serde(rename = "Payload")]
    payload: Payload,
    #[serde(rename = "ObjectifyPayload")]
    objectify_payload: bool,
}

pub fn get_cookie_from_metadata(metadata: Option<Value>) -> Result<String, errors::ConnectorError> {
    let value = metadata
        .as_ref()
        .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
            field_name: "metadata",
        })?;

    let cookie = value
        .get("cookie")
        .and_then(|v| v.as_str())
        .ok_or_else(|| errors::ConnectorError::MissingRequiredField {
            field_name: "metadata.cookie",
        })?;

    Ok(cookie.to_string())
}

fn to_jwt_payload(
    challenge: &ChallengeJwt,
) -> common_utils::errors::CustomResult<josekit::jwt::JwtPayload, errors::ConnectorError> {
    let json_str = serde_json::to_string(challenge)
        .change_context(errors::ConnectorError::ProcessingStepFailed(None))?;

    let parsed: serde_json::Map<String, Value> = serde_json::from_str(&json_str)
        .change_context(errors::ConnectorError::ProcessingStepFailed(None))?;

    let jwt_payload = josekit::jwt::JwtPayload::from_map(parsed)
        .change_context(errors::ConnectorError::ProcessingStepFailed(None))?;

    Ok(jwt_payload)
}

fn generate_challenge_jwt(
    acs_url: String,
    payload: String,
    transaction_id: String,
    return_url: String,
    metadata_for_jwt: WorldpayxmlConnectorMetadataObject,
) -> Result<String, errors::ConnectorError> {
    let iat: u64 = chrono::Utc::now()
        .timestamp()
        .try_into()
        .map_err(|_| errors::ConnectorError::ResponseDeserializationFailed)?;

    let iss = metadata_for_jwt
        .issuer_id
        .ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: "connector_metadata.issuer_id",
        })?;

    let org_unit_id = metadata_for_jwt.organizational_unit_id.ok_or(
        errors::ConnectorError::MissingRequiredField {
            field_name: "connector_metadata.organizational_unit_id",
        },
    )?;

    let secret = metadata_for_jwt.jwt_mac_key.as_deref().ok_or(
        errors::ConnectorError::MissingRequiredField {
            field_name: "connector_metadata.jwt_mac_key",
        },
    )?;

    let payload_json = ChallengeJwt {
        jti: uuid::Uuid::new_v4().to_string(),
        iat,
        iss,
        org_unit_id,
        return_url,
        payload: Payload {
            acs_url,
            payload,
            transaction_id,
        },
        objectify_payload: true,
    };

    let payload_json = to_jwt_payload(&payload_json)
        .map_err(|_| errors::ConnectorError::ProcessingStepFailed(None))?;

    let hmac_signer = josekit::jws::alg::hmac::HmacJwsAlgorithm::Hs256
        .signer_from_bytes(secret.as_bytes())
        .map_err(|_| errors::ConnectorError::ProcessingStepFailed(None))?;

    let mut header = josekit::jws::JwsHeader::new();
    header.set_algorithm("HS256");

    let jwt = josekit::jwt::encode_with_signer(&payload_json, &header, &hmac_signer)
        .map_err(|_| errors::ConnectorError::RequestEncodingFailed)?;

    Ok(jwt)
}

impl<F>
    ForeignTryFrom<(
        ResponseRouterData<F, PaymentService, PaymentsAuthorizeData, PaymentsResponseData>,
        Option<HeaderMap>,
    )> for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn foreign_try_from(
        (item, header): (
            ResponseRouterData<F, PaymentService, PaymentsAuthorizeData, PaymentsResponseData>,
            Option<HeaderMap>,
        ),
    ) -> Result<Self, Self::Error> {
        let is_auto_capture = item.data.request.is_auto_capture()?;
        let reply = item
            .response
            .reply
            .ok_or(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Missing reply data".to_string()),
            ))?;

        validate_reply(&reply)?;

        if let Some(order_status) = reply.order_status {
            validate_order_status(&order_status)?;

            if let Some(payment_data) = order_status.payment {
                let status = get_attempt_status(is_auto_capture, payment_data.last_event, None)?;

                let response = process_payment_response(
                    status,
                    &payment_data,
                    item.http_code,
                    order_status.order_code.clone(),
                    order_status.token,
                )
                .map_err(|err| *err);
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            } else if let Some(challenge_required) = order_status.challenge_required {
                let acs_url = challenge_required
                    .three_ds_challenge_details
                    .as_ref()
                    .and_then(|details| details.acs_url.clone())
                    .ok_or(errors::ConnectorError::UnexpectedResponseError(
                        bytes::Bytes::from("Missing acs_url in challenge details".to_string()),
                    ))?;
                let payload = challenge_required
                    .three_ds_challenge_details
                    .as_ref()
                    .and_then(|details| details.payload.clone())
                    .ok_or(errors::ConnectorError::UnexpectedResponseError(
                        bytes::Bytes::from("Missing payload in challenge details".to_string()),
                    ))?;
                let transaction_id = challenge_required
                    .three_ds_challenge_details
                    .as_ref()
                    .and_then(|details| details.transaction_id_3ds.clone())
                    .ok_or(errors::ConnectorError::UnexpectedResponseError(
                        bytes::Bytes::from(
                            "Missing transaction_id_3ds in challenge details".to_string(),
                        ),
                    ))?;
                let return_url = item.data.request.complete_authorize_url.clone().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "return_url",
                    },
                )?;

                let metadata_for_jwt = WorldpayxmlConnectorMetadataObject::try_from(
                    item.data.connector_meta_data.as_ref(),
                )?;

                let jwt = generate_challenge_jwt(
                    acs_url,
                    payload,
                    transaction_id,
                    return_url,
                    metadata_for_jwt,
                )?;

                let redirection_data = RedirectForm::WorldpayxmlRedirectForm { jwt };

                let cookie = header.and_then(|header| {
                    header
                        .get_all("set-cookie")
                        .iter()
                        .filter_map(|value| value.to_str().ok())
                        .find(|cookie| cookie.trim_start().starts_with("machine="))
                        .map(|cookie| cookie.to_string())
                });

                let metadata = cookie.map(|value| json!({ "cookie": value }));

                let status = common_enums::AttemptStatus::AuthenticationPending;
                let response = Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        order_status.order_code.clone(),
                    ),
                    redirection_data: Box::new(Some(redirection_data)),
                    mandate_reference: Box::new(None),
                    connector_metadata: metadata,
                    network_txn_id: None,
                    connector_response_reference_id: Some(order_status.order_code.clone()),
                    incremental_authorization_allowed: None,
                    authentication_data: None,
                    charges: None,
                });

                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            } else {
                let error =
                order_status.error
                        .ok_or(errors::ConnectorError::UnexpectedResponseError(
                            bytes::Bytes::from("Either order_status.payment or order_status.error must be present in the response".to_string()),
                        ))?;

                Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(ErrorResponse {
                        code: error.code,
                        message: error.message.clone(),
                        reason: Some(error.message.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(order_status.order_code),
                        connector_response_reference_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        } else {
            let error = reply
                .error
                .ok_or(errors::ConnectorError::UnexpectedResponseError(
                    bytes::Bytes::from("Missing  reply.error".to_string()),
                ))?;
            Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: error.code,
                    message: error.message.clone(),
                    reason: Some(error.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })
        }
    }
}

impl TryFrom<&PaymentsCompleteAuthorizeRouterData> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PaymentsCompleteAuthorizeRouterData) -> Result<Self, Self::Error> {
        let auth = WorldpayxmlAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let info_threed_secure = Some(Info3DSecure {
            completed_authentication: CompletedAuthentication {},
        });

        let code = item.request.connector_transaction_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "connector_transaction_id",
            },
        )?;

        let session = Some(CompleteAuthSession {
            id: Secret::new(code.clone()),
        });

        let submit = Some(Submit {
            order: Order {
                order_code: code,
                capture_delay: None,
                description: None,
                amount: None,
                payment_details: None,
                shopper: None,
                shipping_address: None,
                billing_address: None,
                additional_threeds_data: None,
                info_threed_secure,
                session,
                create_token: None,
            },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit,
            reply: None,
            inquiry: None,
            modify: None,
        })
    }
}

impl<F> TryFrom<ResponseRouterData<F, PaymentService, CompleteAuthorizeData, PaymentsResponseData>>
    for RouterData<F, CompleteAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaymentService, CompleteAuthorizeData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let is_auto_capture = item.data.request.is_auto_capture()?;
        let reply = item
            .response
            .reply
            .ok_or(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Missing reply data".to_string()),
            ))?;

        validate_reply(&reply)?;

        if let Some(order_status) = reply.order_status {
            validate_order_status(&order_status)?;

            if let Some(payment_data) = order_status.payment {
                let status = get_attempt_status(is_auto_capture, payment_data.last_event, None)?;
                let response = process_payment_response(
                    status,
                    &payment_data,
                    item.http_code,
                    order_status.order_code.clone(),
                    None,
                )
                .map_err(|err| *err);
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            } else {
                let error =
                order_status.error
                        .ok_or(errors::ConnectorError::UnexpectedResponseError(
                            bytes::Bytes::from("Either order_status.payment or order_status.error must be present in the response".to_string()),
                        ))?;

                Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(ErrorResponse {
                        code: error.code,
                        message: error.message.clone(),
                        reason: Some(error.message.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(order_status.order_code),
                        connector_response_reference_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        } else {
            let error = reply
                .error
                .ok_or(errors::ConnectorError::UnexpectedResponseError(
                    bytes::Bytes::from("Missing  reply.error".to_string()),
                ))?;
            Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: error.code,
                    message: error.message.clone(),
                    reason: Some(error.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })
        }
    }
}

impl<F>
    TryFrom<ResponseRouterData<F, PaymentService, SetupMandateRequestData, PaymentsResponseData>>
    for RouterData<F, SetupMandateRequestData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaymentService, SetupMandateRequestData, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let reply = item
            .response
            .reply
            .ok_or(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Missing reply data".to_string()),
            ))?;

        validate_reply(&reply)?;

        if let Some(order_status) = reply.order_status {
            validate_order_status(&order_status)?;

            if let Some(payment_data) = order_status.payment {
                let status = get_attempt_status_for_setup_mandate(payment_data.last_event)?;

                let response = process_payment_response(
                    status,
                    &payment_data,
                    item.http_code,
                    order_status.order_code.clone(),
                    order_status.token,
                )
                .map_err(|err| *err);
                Ok(Self {
                    status,
                    response,
                    ..item.data
                })
            } else {
                let error =
                order_status.error
                        .ok_or(errors::ConnectorError::UnexpectedResponseError(
                            bytes::Bytes::from("Either order_status.payment or order_status.error must be present in the response".to_string()),
                        ))?;

                Ok(Self {
                    status: common_enums::AttemptStatus::Failure,
                    response: Err(ErrorResponse {
                        code: error.code,
                        message: error.message.clone(),
                        reason: Some(error.message.clone()),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: Some(order_status.order_code),
                        connector_response_reference_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        } else {
            let error = reply
                .error
                .ok_or(errors::ConnectorError::UnexpectedResponseError(
                    bytes::Bytes::from("Missing  reply.error".to_string()),
                ))?;
            Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: error.code,
                    message: error.message.clone(),
                    reason: Some(error.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })
        }
    }
}

impl TryFrom<PaymentsCaptureResponseRouterData<PaymentService>> for PaymentsCaptureRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCaptureResponseRouterData<PaymentService>,
    ) -> Result<Self, Self::Error> {
        let reply = item
            .response
            .reply
            .ok_or(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Missing reply data".to_string()),
            ))?;

        if let Some(capture_received) = reply.ok.and_then(|ok| ok.capture_received) {
            Ok(Self {
                // Capture status will be updated via Psync
                status: common_enums::AttemptStatus::CaptureInitiated,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        capture_received.order_code.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(capture_received.order_code.clone()),
                    incremental_authorization_allowed: None,
                    authentication_data: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            let error = reply
                .error
                .ok_or(errors::ConnectorError::UnexpectedResponseError(
                    bytes::Bytes::from(
                        "Either capture_received or error must be present in the response"
                            .to_string(),
                    ),
                ))?;

            Ok(Self {
                status: common_enums::AttemptStatus::CaptureFailed,
                response: Err(ErrorResponse {
                    code: error.code,
                    message: error.message.clone(),
                    reason: Some(error.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })
        }
    }
}

impl TryFrom<PaymentsCancelResponseRouterData<PaymentService>> for PaymentsCancelRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsCancelResponseRouterData<PaymentService>,
    ) -> Result<Self, Self::Error> {
        let reply = item
            .response
            .reply
            .ok_or(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Missing reply data".to_string()),
            ))?;

        if let Some(cancel_received) = reply.ok.and_then(|ok| ok.cancel_received) {
            Ok(Self {
                // Cancel status will be updated via Psync
                status: common_enums::AttemptStatus::VoidInitiated,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(
                        cancel_received.order_code.clone(),
                    ),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: Some(cancel_received.order_code.clone()),
                    incremental_authorization_allowed: None,
                    authentication_data: None,
                    charges: None,
                }),
                ..item.data
            })
        } else {
            let error = reply
                .error
                .ok_or(errors::ConnectorError::UnexpectedResponseError(
                    bytes::Bytes::from(
                        "Either cancel_received or error must be present in the response"
                            .to_string(),
                    ),
                ))?;

            Ok(Self {
                status: common_enums::AttemptStatus::VoidFailed,
                response: Err(ErrorResponse {
                    code: error.code,
                    message: error.message.clone(),
                    reason: Some(error.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            })
        }
    }
}

#[derive(Default, Debug, Serialize)]
pub struct WorldpayxmlRefundRequest {
    pub amount: StringMinorUnit,
}

impl TryFrom<RefundsResponseRouterData<RSync, WorldpayxmlSyncResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, WorldpayxmlSyncResponse>,
    ) -> Result<Self, Self::Error> {
        match item.response {
            WorldpayxmlSyncResponse::Payment(data) => {
                let reply = data
                    .reply
                    .ok_or(errors::ConnectorError::UnexpectedResponseError(
                        bytes::Bytes::from("Missing reply data".to_string()),
                    ))?;

                validate_reply(&reply)?;

                if let Some(order_status) = reply.order_status {
                    validate_order_status(&order_status)?;

                    if let Some(payment_data) = order_status.payment {
                        let status = get_refund_status(payment_data.last_event)?;
                        let response = if connector_utils::is_refund_failure(status) {
                            let error_code = payment_data
                                .return_code
                                .as_ref()
                                .map(|code| code.code.clone());
                            let error_message = payment_data
                                .return_code
                                .as_ref()
                                .map(|code| code.description.clone());

                            Err(ErrorResponse {
                                code: error_code.unwrap_or(consts::NO_ERROR_CODE.to_string()),
                                message: error_message
                                    .clone()
                                    .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                                reason: error_message.clone(),
                                status_code: item.http_code,
                                attempt_status: None,
                                connector_transaction_id: None,
                                connector_response_reference_id: None,
                                network_advice_code: None,
                                network_decline_code: None,
                                network_error_message: None,
                                connector_metadata: None,
                            })
                        } else {
                            Ok(RefundsResponseData {
                                connector_refund_id: order_status.order_code,
                                refund_status: status,
                            })
                        };

                        Ok(Self {
                            response,
                            ..item.data
                        })
                    } else {
                        order_status.error
                                .ok_or(errors::ConnectorError::UnexpectedResponseError(
                                    bytes::Bytes::from("Either order_status.payment or order_status.error must be present in the response".to_string()),
                                ))?;
                        // Return TransactionResponse for API errors unrelated to the payment to prevent failing the payment.
                        let response = Ok(RefundsResponseData {
                            connector_refund_id: order_status.order_code,
                            refund_status: enums::RefundStatus::Pending,
                        });
                        Ok(Self {
                            response,
                            ..item.data
                        })
                    }
                } else {
                    // Return TransactionResponse for API errors unrelated to the payment to prevent failing the payment
                    let response = Ok(RefundsResponseData {
                        connector_refund_id: item.data.request.connector_transaction_id.clone(),
                        refund_status: enums::RefundStatus::Pending,
                    });

                    Ok(Self {
                        response,
                        ..item.data
                    })
                }
            }
            WorldpayxmlSyncResponse::Webhook(data) => {
                let status = get_refund_status(data.payment_status)?;
                let response = if connector_utils::is_refund_failure(status) {
                    let error_code = data.return_code;
                    let error_message = data.return_message;

                    Err(ErrorResponse {
                        code: error_code.unwrap_or(consts::NO_ERROR_CODE.to_string()),
                        message: error_message
                            .clone()
                            .unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
                        reason: error_message.clone(),
                        status_code: item.http_code,
                        attempt_status: None,
                        connector_transaction_id: None,
                        connector_response_reference_id: None,
                        network_advice_code: None,
                        network_decline_code: None,
                        network_error_message: None,
                        connector_metadata: None,
                    })
                } else {
                    Ok(RefundsResponseData {
                        connector_refund_id: data.order_code,
                        refund_status: status,
                    })
                };

                Ok(Self {
                    response,
                    ..item.data
                })
            }
        }
    }
}

impl TryFrom<&RefundSyncRouterData> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RefundSyncRouterData) -> Result<Self, Self::Error> {
        let auth = WorldpayxmlAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let order_code = item.request.connector_transaction_id.clone();

        let inquiry = Some(Inquiry {
            order_inquiry: OrderInquiry { order_code },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit: None,
            reply: None,
            inquiry,
            modify: None,
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<ApplePayDecrypt> for PaymentInstrument {
    type Error = errors::ConnectorError;
    fn try_from(apple_pay_decrypted_data: ApplePayDecrypt) -> Result<Self, Self::Error> {
        let card_data = CardSSL {
            card_number: apple_pay_decrypted_data.dpan.clone(),
            expiry_date: ExpiryDate {
                date: Date {
                    month: apple_pay_decrypted_data.get_card_expiry_month_2_digit()?,
                    year: apple_pay_decrypted_data.get_expiry_year_4_digit(),
                },
            },
            card_holder_name: apple_pay_decrypted_data.card_holder_name.clone(),
            cvc: None,
        };

        Ok(Self {
            card_details: CardDetails {
                card_ssl: card_data,
            },
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<CardPayout> for PaymentInstrument {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(card_payout: CardPayout) -> Result<Self, Self::Error> {
        let card_data = CardSSL {
            card_number: card_payout.card_number.clone(),
            expiry_date: ExpiryDate {
                date: Date {
                    month: card_payout.get_card_expiry_month_2_digit()?,
                    year: card_payout.get_expiry_year_4_digit(),
                },
            },
            card_holder_name: card_payout.card_holder_name.to_owned(),
            cvc: None,
        };

        Ok(Self {
            card_details: CardDetails {
                card_ssl: card_data,
            },
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<Option<&pii::SecretSerdeValue>> for WorldpayxmlPayoutConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: Option<&pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self =
            connector_utils::to_connector_meta_from_secret::<Self>(meta_data.cloned())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?;
        Ok(metadata)
    }
}

impl TryFrom<Option<&pii::SecretSerdeValue>> for WorldpayxmlConnectorMetadataObject {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(meta_data: Option<&pii::SecretSerdeValue>) -> Result<Self, Self::Error> {
        let metadata: Self =
            connector_utils::to_connector_meta_from_secret::<Self>(meta_data.cloned())
                .change_context(errors::ConnectorError::InvalidConnectorConfig {
                    config: "metadata",
                })?;
        Ok(metadata)
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<&WorldpayxmlRouterData<&PayoutsRouterData<PoFulfill>>> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayxmlRouterData<&PayoutsRouterData<PoFulfill>>,
    ) -> Result<Self, Self::Error> {
        let billing_details = item
            .router_data
            .get_optional_billing()
            .and_then(get_address_details);
        let address = billing_details.map(|details| details.address);

        let purpose_of_payment: Option<WorldpayxmlPayoutConnectorMetadataObject> =
            match item.router_data.connector_meta_data {
                None => None,
                Some(_) => Some(WorldpayxmlPayoutConnectorMetadataObject::try_from(
                    item.router_data.connector_meta_data.as_ref(),
                )?),
            };

        let purpose_of_payment_code = match purpose_of_payment {
            None => None,
            Some(purpose) => map_purpose_code(purpose.purpose_of_payment),
        };

        let payout_method_data = item.router_data.get_payout_method_data()?;
        let payment_instrument = match payout_method_data {
            api_models::payouts::PayoutMethodData::Wallet(
                api_models::payouts::Wallet::ApplePayDecrypt(apple_pay_decrypted_data),
            ) => PaymentInstrument::try_from(apple_pay_decrypted_data)?,
            api_models::payouts::PayoutMethodData::Card(card_payout) => {
                PaymentInstrument::try_from(card_payout)?
            }
            api_models::payouts::PayoutMethodData::Bank(_)
            | api_models::payouts::PayoutMethodData::Wallet(_)
            | api_models::payouts::PayoutMethodData::BankRedirect(_)
            | api_models::payouts::PayoutMethodData::Passthrough(_) => {
                Err(errors::ConnectorError::NotImplemented(
                    "Selected Payout Method is not implemented for WorldpayXML".to_string(),
                ))?
            }
        };

        let payment_details = PaymentDetails {
            action: None,
            payment_method: PaymentMethod::FastAccessSSL(FastAccessData {
                recipient: Recipient {
                    payment_instrument,
                    address,
                },
                purpose_of_payment: purpose_of_payment_code,
            }),
            session: None,
            stored_credentials: None,
        };

        let order_code = item.router_data.connector_request_reference_id.to_owned();

        let description = item.router_data.description.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "description",
            },
        )?;

        let exponent = item
            .router_data
            .request
            .destination_currency
            .number_of_digits_after_decimal_point()
            .to_string();

        let amount = WorldpayXmlAmount {
            currency_code: item.router_data.request.destination_currency.to_owned(),
            exponent,
            value: item.amount.to_owned(),
        };

        let auth = WorldpayxmlAuthType::try_from(&item.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let submit = Some(Submit {
            order: Order {
                order_code,
                capture_delay: None,
                description: Some(description),
                amount: Some(amount),
                payment_details: Some(payment_details),
                shopper: None,
                additional_threeds_data: None,
                info_threed_secure: None,
                session: None,
                billing_address: None,
                shipping_address: None,
                create_token: None,
            },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit,
            reply: None,
            inquiry: None,
            modify: None,
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<PayoutsResponseRouterData<PoFulfill, PayoutResponse>>
    for PayoutsRouterData<PoFulfill>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<PoFulfill, PayoutResponse>,
    ) -> Result<Self, Self::Error> {
        let reply = item.response.reply;

        match (reply.error, reply.order_status) {
            (Some(error), None) => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: error.code,
                    message: error.message.clone(),
                    reason: Some(error.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
            (None, Some(order_status)) => {
                match (order_status.payment, order_status.error) {
                    (Some(payment), None) => Ok(Self {
                        response: Ok(PayoutsResponseData {
                            status: Some(enums::PayoutStatus::try_from(payment.last_event)?),
                            connector_payout_id: Some(order_status.order_code),
                            payout_eligible: None,
                            should_add_next_step_to_process_tracker: false,
                            error_code: None,
                            error_message: None,
                            payout_connector_metadata: None,
                        }),
                        ..item.data
                    }),
                    (None, Some(error)) => Ok(Self {
                        status: common_enums::AttemptStatus::Failure,
                        response: Ok(PayoutsResponseData {
                            status: Some(enums::PayoutStatus::try_from(LastEvent::Error)?),
                            connector_payout_id: Some(order_status.order_code),
                            payout_eligible: None,
                            should_add_next_step_to_process_tracker: false,
                            error_code: Some(error.code),
                            error_message: Some(error.message),
                            payout_connector_metadata: None,
                        }),
                        ..item.data
                    }),
                     _ => Err(
                        errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                            "Either order_status.error or order_status.payment must be present in the response",
                        ))
                        .into(),
                    ),
                }
            },
            _ => Err(
                errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                    "Either reply.error or reply.order_status must be present in the response",
                ))
                .into(),
            ),
        }
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<&PayoutsRouterData<PoSync>> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayoutsRouterData<PoSync>) -> Result<Self, Self::Error> {
        let order_code = item.request.connector_payout_id.to_owned().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "order_code",
            },
        )?;

        let auth = WorldpayxmlAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let inquiry = Some(Inquiry {
            order_inquiry: OrderInquiry { order_code },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit: None,
            reply: None,
            inquiry,
            modify: None,
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<PayoutsResponseRouterData<PoSync, PaymentService>> for PayoutsRouterData<PoSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<PoSync, PaymentService>,
    ) -> Result<Self, Self::Error> {
        let reply = item
            .response
            .reply
            .ok_or(errors::ConnectorError::UnexpectedResponseError(
                bytes::Bytes::from("Missing reply data".to_string()),
            ))?;

        match (reply.error, reply.order_status) {
            (Some(error), None) => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Err(ErrorResponse {
                    code: error.code,
                    message: error.message.clone(),
                    reason: Some(error.message.clone()),
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: None,
                    connector_response_reference_id: None,
                    network_advice_code: None,
                    network_decline_code: None,
                    network_error_message: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),
            (None, Some(order_status)) => {
                match (order_status.payment, order_status.error) {
                    (Some(payment), None) => Ok(Self {
                        response: Ok(PayoutsResponseData {
                            status: Some(enums::PayoutStatus::try_from(payment.last_event)?),
                            connector_payout_id: Some(order_status.order_code),
                            payout_eligible: None,
                            should_add_next_step_to_process_tracker: false,
                            error_code: None,
                            error_message: None,
                            payout_connector_metadata: None,
                        }),
                        ..item.data
                    }),
                    (None, Some(_error)) => Ok(Self {
                        status: item.data.status,
                        response: Ok(PayoutsResponseData {
                            status: None,
                            connector_payout_id: Some(order_status.order_code),
                            payout_eligible: None,
                            should_add_next_step_to_process_tracker: false,
                            error_code: None,
                            error_message: None,
                            payout_connector_metadata: None,
                        }),
                        ..item.data
                    }),
                     _ => Err(
                        errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                            "Either order_status.error or order_status.payment must be present in the response",
                        ))
                        .into(),
                    ),
                }
            },
            _ => Err(
                errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                    "Either reply.error or reply.order_status must be present in the response",
                ))
                .into(),
            ),
        }
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<&PayoutsRouterData<PoCancel>> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &PayoutsRouterData<PoCancel>) -> Result<Self, Self::Error> {
        let auth = WorldpayxmlAuthType::try_from(&item.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let modify = Some(Modify {
            order_modification: OrderModification {
                order_code: item.request.connector_payout_id.to_owned().ok_or(
                    errors::ConnectorError::MissingRequiredField {
                        field_name: "order_code",
                    },
                )?,
                capture: None,
                cancel: None,
                cancel_refund: Some(CancelRequest {}),
                refund: None,
            },
        });

        Ok(Self {
            version: worldpayxml_constants::WORLDPAYXML_VERSION.to_string(),
            merchant_code: auth.merchant_code.clone(),
            submit: None,
            reply: None,
            inquiry: None,
            modify,
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<PayoutsResponseRouterData<PoCancel, PayoutResponse>> for PayoutsRouterData<PoCancel> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PayoutsResponseRouterData<PoCancel, PayoutResponse>,
    ) -> Result<Self, Self::Error> {
        let reply = item.response.reply;

        match (reply.error, reply.ok) {
            (Some(error), None) => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Ok(PayoutsResponseData {
                    status: Some(enums::PayoutStatus::try_from(LastEvent::Error)?),
                    connector_payout_id: None,
                    payout_eligible: None,
                    should_add_next_step_to_process_tracker: false,
                    error_code: Some(error.code),
                    error_message: Some(error.message),
                    payout_connector_metadata: None,
                }),
                ..item.data
            }),
            (None, Some(ok_status)) => {
                let response = ok_status.cancel_received.ok_or(
                    errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                        "ok.cancel_received must be present in the response",
                    )),
                )?;

                Ok(Self {
                    response: Ok(PayoutsResponseData {
                        status: Some(enums::PayoutStatus::try_from(LastEvent::CancelReceived)?),
                        connector_payout_id: Some(response.order_code),
                        payout_eligible: None,
                        should_add_next_step_to_process_tracker: false,
                        error_code: None,
                        error_message: None,
                        payout_connector_metadata: None,
                    }),
                    ..item.data
                })
            }
            _ => Err(
                errors::ConnectorError::UnexpectedResponseError(bytes::Bytes::from(
                    "Either reply.error or reply.ok must be present in the response",
                ))
                .into(),
            ),
        }
    }
}

fn validate_reply(reply: &Reply) -> Result<(), errors::ConnectorError> {
    if (reply.error.is_some() && reply.order_status.is_some())
        || (reply.error.is_none() && reply.order_status.is_none())
    {
        Err(errors::ConnectorError::UnexpectedResponseError(
            bytes::Bytes::from(
                "Either reply.error_data or reply.order_data must be present in the response"
                    .to_string(),
            ),
        ))
    } else {
        Ok(())
    }
}

fn validate_order_status(order_status: &OrderStatus) -> Result<(), errors::ConnectorError> {
    if (order_status.payment.is_some() && order_status.error.is_some())
        || (order_status.payment.is_none()
            && order_status.error.is_none()
            && order_status.challenge_required.is_none())
    {
        Err(errors::ConnectorError::UnexpectedResponseError(
            bytes::Bytes::from(
                "Either order_status.payment or order_status.error must be present in the response"
                    .to_string(),
            ),
        ))
    } else {
        Ok(())
    }
}

fn process_payment_response(
    status: common_enums::AttemptStatus,
    payment_data: &Payment,
    http_code: u16,
    order_code: String,
    token: Option<Token>,
) -> Result<PaymentsResponseData, Box<ErrorResponse>> {
    if connector_utils::is_payment_failure(status) {
        let error_code = payment_data
            .return_code
            .as_ref()
            .map(|code| code.code.clone());
        let error_message = payment_data
            .return_code
            .as_ref()
            .map(|code| code.description.clone());

        Err(Box::new(ErrorResponse {
            code: error_code.unwrap_or(consts::NO_ERROR_CODE.to_string()),
            message: error_message.unwrap_or(consts::NO_ERROR_MESSAGE.to_string()),
            reason: None,
            status_code: http_code,
            attempt_status: None,
            connector_transaction_id: Some(order_code.clone()),
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        }))
    } else {
        let mandate_metadata: Option<Secret<Value, WithType>> = token
            .as_ref()
            .map(|token| &token.authenticated_shopper_i_d)
            .map(|customer_id| Secret::new(json!({ "customer_id": customer_id })));

        let mandate_reference = token.map(|token| MandateReference {
            connector_mandate_id: Some(token.token_details.payment_token_i_d.expose()),
            payment_method_id: None,
            mandate_metadata,
            connector_mandate_request_reference_id: payment_data
                .scheme_response
                .as_ref()
                .map(|response| response.transaction_identifier.clone()),
        });

        Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(order_code.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(mandate_reference),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(order_code.clone()),
            incremental_authorization_allowed: None,
            authentication_data: None,
            charges: None,
        })
    }
}

#[cfg(feature = "payouts")]
pub fn map_purpose_code(value: Option<String>) -> Option<String> {
    let code = match value?.as_str() {
        "Account management" => "ISACCT",
        "Transaction is the payment of allowance" => "ISALLW",
        "Settlement of annuity" => "ISANNI",
        "Unemployment disability benefit" => "ISBENE",
        "Business expenses" => "ISBEXP",
        "Bonus payment" => "ISBONU",
        "Bus transport related business" =>	"ISBUSB",
        "Cash management transfer" => "ISCASH",
        "Payment of cable TV bill" => "ISCBTV",
        "Government institute issued related to cash compensation, helplessness, and disability" => "ISCCHD",
        "Credit card payment" => "ISCCRD",
        "Payment of credit card bill" => "ISCDBL",
        "Payment for charity reasons" => "ISCHAR",
        "Collection payment" => "ISCOLL",
        "Commercial payment" => "ISCOMC",
        "Commission" => "ISCOMM",
        "Compensation relating to interest loss/value date adjustment and can include fees" => "ISCOMP",
        "Payment of copyright" => "ISCPYR",
        "Related to a debit card payment" => "ISDCRD",
        "Payment of a deposit" => "ISDEPT",
        "Payment of dividend" => "ISDIVD",
        "Payment of study/tuition fees" => "ISEDUC",
        "Payment of electricity bill" => "ISELEC",
        "Energies" => "ISENRG",
        "General fees" => "ISFEES",
        "Payment for ferry related business" => "ISFERB",
        "Foreign exchange" => "ISFREX",
        "Payment of gas bill" => "ISGASB",
        "Compensation to unemployed persons during insolvency procedures" => "ISGFRP",
        "Government payment" => "ISGOVT",
        "Health insurance" => "ISHLTI",
        "Reimbursement of credit card payment" => "ISICCP",
        "Reimbursement of debit card payment" => "ISIDCP",
        "Payment of car insurance premium" => "ISINPC",
        "Transaction is related to the payment of an insurance claim" => "ISINSC",
        "Installment" => "ISINSM",
        "Insurance premium" => "ISINSU",
        "Payment of mutual funds, investment products and shares" => "ISINVS",
        "Intra company payment" => "ISINTC",
        "Interest" => "ISINTE",
        "Income tax" => "ISINTX",
        "Investment" => "ISINVS",
        "Labor insurance" => "ISLBRI",
        "License fee" => "ISLICF",
        "Life insurance" =>  "ISLIFI",
        "Loan" => "ISLOAN",
        "Medical services" => "ISMDCS",
        "Mobile P2B payment" => "ISMP2B",
        "Mobile P2P payment" => "ISMP2P",
        "Mobile top up" => "ISMTUP",
        "Not otherwise specified" => "ISNOWS",
        "Transaction is related to a payment of other telecom related bill" => "ISOTLC",
        "Payroll" => "ISPAYR",
        "Contribution to pension fund" => "ISPEFC",
        "Pension payment" => "ISPENS",
        "Payment of telephone bill" => "ISPHON",
        "Property insurance" => "ISPPTI",
        "Transaction is for general rental/lease" => "ISRELG",
        "The payment of rent" => "ISRENT",
        "Payment for railway transport related business" => "ISRLWY",
        "Royalties" => "ISROYA",
        "Salary payment" => "ISSALA",
        "Payment to savings/retirement account" => "ISSAVG",
        "Securities" => "ISSECU",
        "Social security benefit" => "ISSSBE",
        "Study" => "ISSTDY",
        "Subscription" => "ISSUBS",
        "Supplier payment" => "ISSUPP",
        "Refund of a tax payment or obligation" => "ISTAXR",
        "Tax payment" => "ISTAXS",
        "Transaction is related to a payment of telecommunications related bill" => "ISTBIL",
        "Trade services operation" => "ISTRAD",
        "Treasury payment" => "ISTREA",
        "Payment for travel" => "ISTRPT",
        "Utility bill payment" => "ISUBIL",
        "Value added tax payment" => "ISVATX",
        "With holding" => "ISWHLD",
        "Payment of water bill" => "ISWTER",
        "Other" => "ISOTHR",
        _ => return None,
    };

    Some(code.to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorldpayFormWebhookBody {
    #[serde(rename = "PaymentAmount")]
    pub payment_amount: Option<StringMinorUnit>,

    #[serde(rename = "PaymentId")]
    pub payment_id: Option<String>,

    #[serde(rename = "OrderCode")]
    pub order_code: String,

    #[serde(rename = "PaymentStatus")]
    pub payment_status: LastEvent,

    #[serde(rename = "ReturnCode")]
    pub return_code: Option<String>,

    #[serde(rename = "ReturnMessage")]
    pub return_message: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderStatusEvent {
    #[serde(rename = "@orderCode")]
    pub order_code: String,
    pub payment: Payment,
}

pub fn get_payout_webhook_event(status: LastEvent) -> api_models::webhooks::IncomingWebhookEvent {
    match status {
        LastEvent::PushRequested | LastEvent::PushPending => {
            api_models::webhooks::IncomingWebhookEvent::PayoutProcessing
        }
        LastEvent::SettledByMerchant | LastEvent::PushApproved => {
            api_models::webhooks::IncomingWebhookEvent::PayoutSuccess
        }
        LastEvent::Cancelled => api_models::webhooks::IncomingWebhookEvent::PayoutCancelled,
        LastEvent::PushRefused | LastEvent::Error => {
            api_models::webhooks::IncomingWebhookEvent::PayoutFailure
        }
        _ => api_models::webhooks::IncomingWebhookEvent::EventNotSupported,
    }
}

pub fn get_payment_webhook_event(status: LastEvent) -> api_models::webhooks::IncomingWebhookEvent {
    match status {
        LastEvent::Authorised | LastEvent::SentForAuthorisation => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing
        }
        LastEvent::Captured | LastEvent::Settled | LastEvent::SettledByMerchant => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess
        }
        LastEvent::Refunded | LastEvent::RefundedByMerchant => {
            api_models::webhooks::IncomingWebhookEvent::RefundSuccess
        }
        LastEvent::Cancelled => api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled,
        LastEvent::Refused => api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure,
        LastEvent::RefundFailed => api_models::webhooks::IncomingWebhookEvent::RefundFailure,
        _ => api_models::webhooks::IncomingWebhookEvent::EventNotSupported,
    }
}

pub fn is_payout_event(event_code: LastEvent) -> bool {
    matches!(
        event_code,
        LastEvent::PushApproved
            | LastEvent::PushPending
            | LastEvent::PushRequested
            | LastEvent::SettledByMerchant
            | LastEvent::PushRefused
    )
}

pub fn is_refund_event(event_code: LastEvent) -> bool {
    matches!(
        event_code,
        LastEvent::SentForRefund
            | LastEvent::RefundedByMerchant
            | LastEvent::RefundRequested
            | LastEvent::Refunded
            | LastEvent::RefundFailed
    )
}

pub fn is_transaction_event(event_code: LastEvent) -> bool {
    matches!(
        event_code,
        LastEvent::Authorised
            | LastEvent::Settled
            | LastEvent::Captured
            | LastEvent::SentForAuthorisation
            | LastEvent::Cancelled
            | LastEvent::Refused
    )
}

fn get_mandate_type(mit_category: Option<common_enums::MitCategory>) -> MandateType {
    match mit_category {
        Some(common_enums::MitCategory::Installment) => MandateType::Instalment,
        Some(common_enums::MitCategory::Recurring) => MandateType::Recurring,
        Some(common_enums::MitCategory::Unscheduled) | None => MandateType::Unscheduled,
        _ => MandateType::Unscheduled,
    }
}
