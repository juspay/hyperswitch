#[cfg(feature = "payouts")]
use api_models::payouts::{ApplePayDecrypt, CardPayout};
use common_enums::enums;
#[cfg(feature = "payouts")]
use common_enums::CardNetwork;
#[cfg(feature = "payouts")]
use common_utils::pii;
use common_utils::types::StringMinorUnit;
use error_stack::ResultExt;
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
    router_request_types::{PaymentsAuthorizeData, PaymentsSyncData, ResponseId},
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsSyncRouterData, RefundSyncRouterData, RefundsRouterData,
    },
};
use hyperswitch_interfaces::{consts, errors};
use masking::Secret;
use serde::{Deserialize, Serialize};

#[cfg(feature = "payouts")]
use crate::types::PayoutsResponseRouterData;
use crate::{
    types::{
        PaymentsCancelResponseRouterData, PaymentsCaptureResponseRouterData,
        RefundsResponseRouterData, ResponseRouterData,
    },
    utils::{
        self as connector_utils, AddressDetailsData, CardData, PaymentsAuthorizeRequestData,
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
struct OrderStatus {
    #[serde(rename = "@orderCode")]
    order_code: String,
    payment: Option<Payment>,
    error: Option<WorldpayXmlErrorResponse>,
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
    description: String,
    amount: WorldpayXmlAmount,
    payment_details: PaymentDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    shopper: Option<WorldpayxmlShopper>,
    #[serde(skip_serializing_if = "Option::is_none")]
    shipping_address: Option<WorldpayxmlPayinAddress>,
    #[serde(skip_serializing_if = "Option::is_none")]
    billing_address: Option<WorldpayxmlPayinAddress>,
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum Action {
    Authorise,
    Sale,
    Refund,
}

impl TryFrom<(&Card, Option<enums::CaptureMethod>)> for PaymentDetails {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (card_data, capture_method): (&Card, Option<enums::CaptureMethod>),
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
        let shopper = get_shopper_details(authorize_data.router_data);
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
        ))?;
        let submit = Some(Submit {
            order: Order {
                order_code,
                capture_delay,
                description,
                amount,
                payment_details,
                shopper,
                billing_address,
                shipping_address,
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

fn get_shopper_details(item: &PaymentsAuthorizeRouterData) -> Option<WorldpayxmlShopper> {
    let shopper_email = item.request.email.clone();
    let browser_info = item
        .request
        .browser_info
        .as_ref()
        .map(|browser_info| WPGBrowserData {
            accept_header: browser_info.accept_header.clone(),
            http_accept_language: browser_info.accept_language.clone(),
            http_referer: browser_info.referer.clone(),
            browser_language: browser_info.language.clone(),
            browser_java_enabled: browser_info.java_enabled,
            browser_java_script_enabled: browser_info.java_script_enabled,
            browser_colour_depth: browser_info.color_depth,
            browser_screen_height: browser_info.screen_height,
            browser_screen_width: browser_info.screen_width,
            user_agent_header: browser_info.user_agent.clone(),
            time_zone: browser_info.time_zone,
        });

    if shopper_email.is_some() || browser_info.is_some() {
        Some(WorldpayxmlShopper {
            shopper_email_address: shopper_email,
            browser: browser_info,
        })
    } else {
        None
    }
}

impl TryFrom<&WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>> for PaymentService {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &WorldpayxmlRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            Err(errors::ConnectorError::NotSupported {
                message: "Card 3DS".to_string(),
                connector: "Worldpayxml",
            })?
        };
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
        | LastEvent::RefundFailed => Err(errors::ConnectorError::UnexpectedResponseError(
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

impl<F> TryFrom<ResponseRouterData<F, PaymentService, PaymentsAuthorizeData, PaymentsResponseData>>
    for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PaymentService, PaymentsAuthorizeData, PaymentsResponseData>,
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
                description,
                amount,
                payment_details,
                shopper: None,
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
        || (order_status.payment.is_none() && order_status.error.is_none())
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
            && self.address1.is_none()
            && self.postal_code.is_none()
            && self.city.is_none()
            && self.country_code.is_none()
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
        | LastEvent::Settled
        | LastEvent::Captured
        | LastEvent::SentForAuthorisation => {
            api_models::webhooks::IncomingWebhookEvent::EventNotSupported
        }
    }
}
