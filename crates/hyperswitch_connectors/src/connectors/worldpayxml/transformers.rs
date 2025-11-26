#[cfg(feature = "payouts")]
use api_models::payouts::{ApplePayDecrypt, CardPayout};
use common_enums::enums;
#[cfg(feature = "payouts")]
use common_enums::CardNetwork;
#[cfg(feature = "payouts")]
use common_utils::pii;
use common_utils::types::StringMinorUnit;
use error_stack::ResultExt;
use http::HeaderMap;
#[cfg(feature = "payouts")]
use hyperswitch_domain_models::{
    address::Address,
    router_flow_types::payouts::{PoCancel, PoFulfill},
    router_response_types::PayoutsResponseData,
    types::PayoutsRouterData,
};
use hyperswitch_domain_models::{
    payment_method_data::{Card, PaymentMethodData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::{
        CompleteAuthorizeData, PaymentsAuthorizeData, PaymentsSyncData, ResponseId,
    },
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsSyncRouterData, RefundSyncRouterData,
        RefundsRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors};
use josekit;
use masking::Secret;
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
    pub reply: PayoutReply,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PayoutReply {
    pub ok: Option<OkPayoutResponse>,
    pub error: Option<WorldpayXmlErrorResponse>,
}

#[cfg(feature = "payouts")]
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OkPayoutResponse {
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
    error: Option<WorldpayXmlErrorResponse>,
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
struct PaymentDetails {
    #[serde(skip_serializing_if = "Option::is_none", rename = "@action")]
    action: Option<Action>,
    #[serde(flatten)]
    payment_method: PaymentMethod,
    #[serde(skip_serializing_if = "Option::is_none")]
    session: Option<Session>,
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

    #[serde(rename = "VISA-SSL")]
    VisaSSL(CardSSL),

    #[serde(rename = "ECMC-SSL")]
    EcmcSSL(CardSSL),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CardSSL {
    card_number: cards::CardNumber,
    expiry_date: ExpiryDate,
    card_holder_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cvc: Option<Secret<String>>,
    #[serde(skip_serializing_if = "CardAddress::is_empty_option")]
    card_address: Option<CardAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    purpose_of_payment_code: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct CardAddress {
    #[serde(skip_serializing_if = "WorldpayxmlAddress::is_empty_option")]
    address: Option<WorldpayxmlAddress>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorldpayxmlAddress {
    #[serde(skip_serializing_if = "Option::is_none")]
    last_name: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    address1: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    postal_code: Option<Secret<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    city: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    country_code: Option<common_enums::CountryAlpha2>,
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

#[cfg(feature = "payouts")]
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum PayoutOutcome {
    RefundReceived,
    Refused,
    Error,
    QueryRequired,
    CancelReceived,
}

#[cfg(feature = "payouts")]
impl From<PayoutOutcome> for enums::PayoutStatus {
    fn from(item: PayoutOutcome) -> Self {
        match item {
            PayoutOutcome::RefundReceived => Self::Initiated,
            PayoutOutcome::Error | PayoutOutcome::Refused => Self::Failed,
            PayoutOutcome::QueryRequired => Self::Pending,
            PayoutOutcome::CancelReceived => Self::Cancelled,
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
                card_address: None,
                purpose_of_payment_code: None,
            }),
            session,
        })
    }
}

impl TryFrom<(&WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>, &Card)> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: (&WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>, &Card),
    ) -> Result<Self, Self::Error> {
        let authorize_data = item.0;
        let card_data = item.1;
        let auth = WorldpayxmlAuthType::try_from(&authorize_data.router_data.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;

        let order_code = if authorize_data
            .router_data
            .connector_request_reference_id
            .len()
            <= worldpayxml_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH
        {
            Ok(authorize_data
                .router_data
                .connector_request_reference_id
                .clone())
        } else {
            Err(errors::ConnectorError::MaxFieldLengthViolated {
                connector: "Worldpayxml".to_string(),
                field_name: "order_code".to_string(),
                max_length: worldpayxml_constants::MAX_PAYMENT_REFERENCE_ID_LENGTH,
                received_length: authorize_data
                    .router_data
                    .connector_request_reference_id
                    .len(),
            })
        }?;

        let capture_delay = if authorize_data.router_data.request.is_auto_capture()? {
            Some(AutoCapture::On)
        } else {
            Some(AutoCapture::Off)
        };
        let description = authorize_data.router_data.description.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "description",
            },
        )?;

        let is_three_ds = authorize_data.router_data.is_three_ds();
        let (additional_threeds_data, session, accept_header, user_agent_header) =
            if is_three_ds {
                let additional_threeds_data = Some(AdditionalThreeDSData {
                    df_reference_id: None,
                    javascript_enabled: false,
                    device_channel: "Browser".to_string(),
                });
                let browser_info = authorize_data.router_data.request.get_browser_info()?;
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
                    id: authorize_data
                        .router_data
                        .connector_request_reference_id
                        .clone(),
                    shopper_ip_address: authorize_data.router_data.request.get_ip_address()?,
                });

                (
                    additional_threeds_data,
                    session,
                    Some(accept_header),
                    Some(user_agent_header),
                )
            } else {
                let accept_header = authorize_data
                    .router_data
                    .request
                    .browser_info
                    .as_ref()
                    .and_then(|info| info.accept_header.clone());
                let user_agent_header = authorize_data
                    .router_data
                    .request
                    .browser_info
                    .as_ref()
                    .and_then(|info| info.user_agent.clone());

                (None, None, accept_header, user_agent_header)
            };

        let exponent = authorize_data
            .router_data
            .request
            .currency
            .number_of_digits_after_decimal_point()
            .to_string();
        let amount = WorldpayXmlAmount {
            currency_code: authorize_data.router_data.request.currency.to_owned(),
            exponent,
            value: authorize_data.amount.to_owned(),
        };
        let shopper =
            get_shopper_details(authorize_data.router_data, accept_header, user_agent_header)?;
        let billing_address = authorize_data
            .router_data
            .get_optional_billing()
            .and_then(get_address_details);
        let shipping_address = authorize_data
            .router_data
            .get_optional_shipping()
            .and_then(get_address_details);

        let payment_details = PaymentDetails::try_from((
            card_data,
            authorize_data.router_data.request.capture_method,
            session,
        ))?;
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
) -> Result<Option<WorldpayxmlShopper>, errors::ConnectorError> {
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

    if shopper_email.is_some() || browser_info.is_some() {
        Ok(Some(WorldpayxmlShopper {
            shopper_email_address: shopper_email,
            browser: browser_info,
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
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => Self::try_from((item, &req_card)),
            _ => Err(errors::ConnectorError::NotImplemented(
                connector_utils::get_unimplemented_payment_method_error_message("Worldpayxml"),
            ))?,
        }
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
        LastEvent::Captured | LastEvent::Settled => Ok(common_enums::AttemptStatus::Charged),
        LastEvent::SentForAuthorisation => Ok(common_enums::AttemptStatus::Authorizing),
        LastEvent::Refunded
        | LastEvent::SentForRefund
        | LastEvent::RefundRequested
        | LastEvent::SentForFastRefund
        | LastEvent::RefundedByMerchant
        | LastEvent::RefundFailed
        | LastEvent::Error => Err(errors::ConnectorError::UnexpectedResponseError(
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
        LastEvent::Authorised
        | LastEvent::Refused
        | LastEvent::Cancelled
        | LastEvent::Error
        | LastEvent::SentForAuthorisation => Err(errors::ConnectorError::UnexpectedResponseError(
            bytes::Bytes::from("Invalid LastEvent".to_string()),
        )),
    }
}

impl<F> TryFrom<ResponseRouterData<F, PaymentService, PaymentsSyncData, PaymentsResponseData>>
    for RouterData<F, PaymentsSyncData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaymentService, PaymentsSyncData, PaymentsResponseData>,
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
                        connector_response_reference_id: Some(order_status.order_code.clone()),
                        incremental_authorization_allowed: None,
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
                    charges: None,
                }),
                ..item.data
            })
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
                println!("Cookie: {:?} >>> ", cookie);
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
                session
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

impl TryFrom<RefundsResponseRouterData<RSync, PaymentService>> for RefundsRouterData<RSync> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, PaymentService>,
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
impl TryFrom<(ApplePayDecrypt, Option<CardAddress>, Option<String>)> for PaymentDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (apple_pay_decrypted_data, address, purpose_of_payment): (
            ApplePayDecrypt,
            Option<CardAddress>,
            Option<String>,
        ),
    ) -> Result<Self, Self::Error> {
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
            card_address: address,
            purpose_of_payment_code: None,
        };

        let payment_method = match apple_pay_decrypted_data.card_network {
            Some(CardNetwork::Visa) => PaymentMethod::VisaSSL(CardSSL {
                purpose_of_payment_code: purpose_of_payment.clone(),
                ..card_data
            }),
            Some(CardNetwork::Mastercard) => PaymentMethod::EcmcSSL(CardSSL {
                purpose_of_payment_code: purpose_of_payment.clone(),
                ..card_data
            }),
            _ => PaymentMethod::CardSSL(CardSSL {
                purpose_of_payment_code: None,
                ..card_data
            }),
        };

        Ok(Self {
            action: Some(Action::Refund),
            payment_method,
            session: None,
        })
    }
}

#[cfg(feature = "payouts")]
impl TryFrom<(CardPayout, Option<CardAddress>, Option<String>)> for PaymentDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (card_payout, address, purpose_of_payment): (
            CardPayout,
            Option<CardAddress>,
            Option<String>,
        ),
    ) -> Result<Self, Self::Error> {
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
            card_address: address,
            purpose_of_payment_code: None,
        };

        let payment_method = match card_payout.card_network {
            Some(CardNetwork::Visa) => PaymentMethod::VisaSSL(CardSSL {
                purpose_of_payment_code: purpose_of_payment.clone(),
                ..card_data
            }),
            Some(CardNetwork::Mastercard) => PaymentMethod::EcmcSSL(CardSSL {
                purpose_of_payment_code: purpose_of_payment.clone(),
                ..card_data
            }),
            _ => PaymentMethod::CardSSL(CardSSL {
                purpose_of_payment_code: None,
                ..card_data
            }),
        };

        Ok(Self {
            action: Some(Action::Refund),
            payment_method,
            session: None,
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
        let billing_details = Some(CardAddress {
            address: Some(WorldpayxmlAddress {
                last_name: item.router_data.get_optional_billing_last_name(),
                address1: item.router_data.get_optional_billing_line1(),
                postal_code: item.router_data.get_optional_billing_zip(),
                city: item.router_data.get_optional_billing_city(),
                country_code: item.router_data.get_optional_billing_country(),
            }),
        });

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
        let payment_details = match payout_method_data {
            api_models::payouts::PayoutMethodData::Wallet(
                api_models::payouts::Wallet::ApplePayDecrypt(apple_pay_decrypted_data),
            ) => PaymentDetails::try_from((
                apple_pay_decrypted_data,
                billing_details,
                purpose_of_payment_code,
            ))?,
            api_models::payouts::PayoutMethodData::Card(card_payout) => {
                PaymentDetails::try_from((card_payout, billing_details, purpose_of_payment_code))?
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

        match (reply.error, reply.ok) {
            (Some(error), None) => Ok(Self {
                status: common_enums::AttemptStatus::Failure,
                response: Ok(PayoutsResponseData {
                    status: Some(enums::PayoutStatus::from(PayoutOutcome::Error)),
                    connector_payout_id: None,
                    payout_eligible: None,
                    should_add_next_step_to_process_tracker: false,
                    error_code: Some(error.code),
                    error_message: Some(error.message),
                    payout_connector_metadata: None,
                }),
                ..item.data
            }),
            (None, Some(ok_status)) => Ok(Self {
                response: Ok(PayoutsResponseData {
                    status: Some(enums::PayoutStatus::from(PayoutOutcome::RefundReceived)),
                    connector_payout_id: ok_status.refund_received.map(|id| id.order_code),
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
                    "Either reply.error or reply.ok must be present in the response",
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
                    status: Some(enums::PayoutStatus::from(PayoutOutcome::Error)),
                    connector_payout_id: None,
                    payout_eligible: None,
                    should_add_next_step_to_process_tracker: false,
                    error_code: Some(error.code),
                    error_message: Some(error.message),
                    payout_connector_metadata: None,
                }),
                ..item.data
            }),
            (None, Some(ok_status)) => Ok(Self {
                response: Ok(PayoutsResponseData {
                    status: Some(enums::PayoutStatus::from(PayoutOutcome::CancelReceived)),
                    connector_payout_id: ok_status.refund_received.map(|id| id.order_code),
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
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        }))
    } else {
        Ok(PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(order_code.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(order_code.clone()),
            incremental_authorization_allowed: None,
            charges: None,
        })
    }
}

#[cfg(feature = "payouts")]
pub fn map_purpose_code(value: Option<String>) -> Option<String> {
    let code = match value?.as_str() {
        "Family Support" => "00",
        "Regular Labour Transfers" => "01",
        "Travel and Tourism" => "02",
        "Education" => "03",
        "Hospitalisation and Medical Treatment" => "04",
        "Emergency Need" => "05",
        "Savings" => "06",
        "Gifts" => "07",
        "Other" => "08",
        "Salary" => "09",
        "Crowd Lending" => "10",
        "Crypto Currency" => "11",
        "Gaming Repayment" => "12",
        "Stock Market Proceeds" => "13",
        "Refund to a original card" => "M1",
        "Refund to a new card" => "M2",
        _ => return None,
    };

    Some(code.to_string())
}

impl WorldpayxmlAddress {
    fn is_empty(&self) -> bool {
        self.last_name.is_none()
            || self.address1.is_none()
            || self.postal_code.is_none()
            || self.city.is_none()
            || self.country_code.is_none()
    }

    fn is_empty_option(addr: &Option<Self>) -> bool {
        match addr {
            Some(a) => a.is_empty(),
            None => true,
        }
    }
}

impl CardAddress {
    fn is_empty_option(addr: &Option<Self>) -> bool {
        match addr {
            Some(a) => WorldpayxmlAddress::is_empty_option(&a.address),
            None => true,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename = "paymentService")]
pub struct WorldpayXmlWebhookBody {
    #[serde(rename = "@version")]
    pub version: String,
    #[serde(rename = "@merchantCode")]
    pub merchant_code: Secret<String>,
    pub notify: Notify,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Notify {
    pub order_status_event: OrderStatusEvent,
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
        LastEvent::SentForRefund
        | LastEvent::RefundedByMerchant
        | LastEvent::SentForFastRefund
        | LastEvent::RefundRequested => {
            api_models::webhooks::IncomingWebhookEvent::PayoutProcessing
        }
        LastEvent::Refunded => api_models::webhooks::IncomingWebhookEvent::PayoutSuccess,
        LastEvent::Cancelled => api_models::webhooks::IncomingWebhookEvent::PayoutCancelled,
        LastEvent::Refused | LastEvent::RefundFailed => {
            api_models::webhooks::IncomingWebhookEvent::PayoutFailure
        }
        LastEvent::Authorised
        | LastEvent::Error
        | LastEvent::Settled
        | LastEvent::Captured
        | LastEvent::SentForAuthorisation => {
            api_models::webhooks::IncomingWebhookEvent::EventNotSupported
        }
    }
}
