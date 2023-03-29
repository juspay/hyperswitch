use actix::fut::ok;
use base64::Engine;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};

use crate::{
    connector::utils::{PaymentsAuthorizeRequestData, self},
    consts,
    core::errors,
    services,
    types::{self, api, storage::enums},
};

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BamboraCard {
    name: Secret<String>,
    number: Secret<String, common_utils::pii::CardNumber>,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvd: Secret<String>,
    complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "3d_secure")]
    three_d_secure: Option<ThreeDSecure>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ThreeDSecure {
    browser: Option<BamboraBrowserInfo>, //Needed only in case of 3Ds 2.0. Need to update request for this.
    enabled: bool,
    version: Option<i64>,
    auth_required: Option<bool>,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BamboraBrowserInfo {
    accept_header: String,
    java_enabled: bool,
    language: String,
    color_depth: u8,
    screen_height: u32,
    screen_width: u32,
    time_zone: i32,
    user_agent: String,
    javascript_enabled: bool,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BamboraPaymentsRequest {
    amount: i64,
    payment_method: PaymentMethod,
    customer_ip: Option<std::net::IpAddr>,
    term_url: Option<String>,
    card: BamboraCard,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BamboraRedirectionResponse {
    pub cres: String,
}

fn get_browser_info(item: &types::PaymentsAuthorizeRouterData) -> Option<BamboraBrowserInfo> {
    if matches!(item.auth_type, enums::AuthenticationType::ThreeDs) {
        item.request
            .browser_info
            .as_ref()
            .map(|info| BamboraBrowserInfo {
                accept_header: info.accept_header.clone(),
                java_enabled: info.java_enabled,
                language: info.language.clone(),
                color_depth: info.color_depth,
                screen_height: info.screen_height,
                screen_width: info.screen_width,
                time_zone: info.time_zone,
                user_agent: info.user_agent.clone(),
                javascript_enabled: info.java_script_enabled,
            })
    } else {
        None
    }
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BamboraPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        println!("sssss {:?}", item);
        match item.request.payment_method_data.clone() {
            api::PaymentMethodData::Card(req_card) => {
                let three_ds = match item.auth_type {
                    enums::AuthenticationType::ThreeDs => Some(ThreeDSecure {
                        enabled: true,
                        browser: get_browser_info(item),
                        version: Some(2),
                        auth_required: Some(true),
                    }),
                    enums::AuthenticationType::NoThreeDs => None,
                };
                let bambora_card = BamboraCard {
                    name: req_card.card_holder_name,
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvd: req_card.card_cvc,
                    three_d_secure: three_ds,
                    complete: item.request.is_auto_capture(),
                };
                Ok(Self {
                    amount: item.request.amount,
                    payment_method: PaymentMethod::Card,
                    card: bambora_card,
                    customer_ip: item.request.browser_info.as_ref().unwrap().ip_address,
                    term_url: item.complete_authorize_url.clone(),
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

impl TryFrom<&types::PaymentsCancelRouterData> for BamboraPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(_item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: 0,
            ..Default::default()
        })
    }
}

pub struct BamboraAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for BamboraAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::BodyKey { api_key, key1 } = auth_type {
            let auth_key = format!("{key1}:{api_key}");
            let auth_header = format!("Passcode {}", consts::BASE64_ENGINE.encode(auth_key));
            Ok(Self {
                api_key: auth_header,
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

pub enum PaymentFlow {
    Authorize,
    Capture,
    Void,
}

// PaymentsResponse
impl<F, T>
    TryFrom<(
        types::ResponseRouterData<F, BamboraResponse, T, types::PaymentsResponseData>,
        PaymentFlow,
    )> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        data: (
            types::ResponseRouterData<F, BamboraResponse, T, types::PaymentsResponseData>,
            PaymentFlow,
        ),
    ) -> Result<Self, Self::Error> {
        let flow = data.1;
        let item = data.0;
        match item.response {
            BamboraResponse::NormalTranaction(pg_response) => Ok(Self {
                status: match pg_response.approved.as_str() {
                    "0" => match flow {
                        PaymentFlow::Authorize => enums::AttemptStatus::AuthorizationFailed,
                        PaymentFlow::Capture => enums::AttemptStatus::Failure,
                        PaymentFlow::Void => enums::AttemptStatus::VoidFailed,
                    },
                    "1" => match flow {
                        PaymentFlow::Authorize => enums::AttemptStatus::Authorized,
                        PaymentFlow::Capture => enums::AttemptStatus::Charged,
                        PaymentFlow::Void => enums::AttemptStatus::Voided,
                    },
                    &_ => Err(errors::ConnectorError::ResponseDeserializationFailed)?,
                },
                response: Ok(types::PaymentsResponseData::TransactionResponse {
                    resource_id: types::ResponseId::ConnectorTransactionId(
                        pg_response.id.to_string(),
                    ),
                    redirection_data: None,
                    mandate_reference: None,
                    connector_metadata: None,
                }),
                ..item.data
            }),

            BamboraResponse::ThreeDsResponse(response) => {
                let url_decoded = utils::decode_html(&response.contents);
                let redirection_data = Some(services::RedirectForm::Html { html_data: url_decoded } );
                Ok(Self {
                    status: enums::AttemptStatus::AuthenticationPending,
                    response: Ok(types::PaymentsResponseData::TransactionResponse {
                        resource_id: types::ResponseId::NoResponseId,
                        redirection_data,
                        mandate_reference: None,
                        connector_metadata: None,
                    }),
                    ..item.data
                })
            }
        }
    }
}

fn str_or_i32<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StrOrI32 {
        Str(String),
        I32(i32),
    }

    let value = StrOrI32::deserialize(deserializer)?;
    let res = match value {
        StrOrI32::Str(v) => v,
        StrOrI32::I32(v) => v.to_string(),
    };
    Ok(res)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BamboraResponse {
    NormalTranaction(BamboraPaymentsResponse),
    ThreeDsResponse(Bambora3DsResponse),
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BamboraPaymentsResponse {
    #[serde(deserialize_with = "str_or_i32")]
    id: String,
    authorizing_merchant_id: i32,
    #[serde(deserialize_with = "str_or_i32")]
    approved: String,
    #[serde(deserialize_with = "str_or_i32")]
    message_id: String,
    message: String,
    auth_code: String,
    created: String,
    amount: f32,
    order_number: String,
    #[serde(rename = "type")]
    payment_type: String,
    comments: Option<String>,
    batch_number: Option<String>,
    total_refunds: Option<f32>,
    total_completions: Option<f32>,
    payment_method: String,
    card: CardData,
    billing: Option<AddressData>,
    shipping: Option<AddressData>,
    custom: CustomData,
    adjusted_by: Option<Vec<AdjustedBy>>,
    links: Vec<Links>,
    risk_score: Option<f32>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Bambora3DsResponse {
    #[serde(rename = "3d_session_data")]
    three_d_session_data: String,
    contents: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardData {
    name: Option<String>,
    expiry_month: Option<String>,
    expiry_year: Option<String>,
    card_type: String,
    last_four: String,
    card_bin: Option<String>,
    avs_result: String,
    cvd_result: String,
    cavv_result: Option<String>,
    address_match: Option<i32>,
    postal_result: Option<i32>,
    avs: Option<AvsObject>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AvsObject {
    id: String,
    message: String,
    processed: bool,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AddressData {
    name: String,
    address_line1: String,
    address_line2: String,
    city: String,
    province: String,
    country: String,
    postal_code: String,
    phone_number: String,
    email_address: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomData {
    ref1: String,
    ref2: String,
    ref3: String,
    ref4: String,
    ref5: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AdjustedBy {
    id: i32,
    #[serde(rename = "type")]
    adjusted_by_type: String,
    approval: i32,
    message: String,
    amount: f32,
    created: String,
    url: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Links {
    rel: String,
    href: String,
    method: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethod {
    #[default]
    Card,
    Token,
    PaymentProfile,
    Cash,
    Cheque,
    Interac,
    ApplePay,
    AndroidPay,
    #[serde(rename = "3d_secure")]
    ThreeDSecure,
    ProcessorToken,
}

// Capture
#[derive(Default, Debug, Clone, Serialize, PartialEq)]
pub struct BamboraPaymentsCaptureRequest {
    amount: Option<i64>,
    payment_method: PaymentMethod,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for BamboraPaymentsCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount_to_capture,
            payment_method: PaymentMethod::Card,
        })
    }
}

// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct BamboraRefundRequest {
    amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BamboraRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.amount,
        })
    }
}

// Type definition for Refund Response
#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Succeeded => Self::Success,
            RefundStatus::Failed => Self::Failure,
            RefundStatus::Processing => Self::Pending,
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    #[serde(deserialize_with = "str_or_i32")]
    id: String,
    authorizing_merchant_id: i32,
    #[serde(deserialize_with = "str_or_i32")]
    approved: String,
    #[serde(deserialize_with = "str_or_i32")]
    message_id: String,
    message: String,
    auth_code: String,
    created: String,
    amount: f32,
    order_number: String,
    #[serde(rename = "type")]
    payment_type: String,
    comments: Option<String>,
    batch_number: Option<String>,
    total_refunds: Option<f32>,
    total_completions: Option<f32>,
    payment_method: String,
    card: CardData,
    billing: Option<AddressData>,
    shipping: Option<AddressData>,
    custom: CustomData,
    adjusted_by: Option<Vec<AdjustedBy>>,
    links: Vec<Links>,
    risk_score: Option<f32>,
}

impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.approved.as_str() {
            "0" => enums::RefundStatus::Failure,
            "1" => enums::RefundStatus::Success,
            &_ => Err(errors::ConnectorError::ResponseDeserializationFailed)?,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status,
            }),
            ..item.data
        })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.approved.as_str() {
            "0" => enums::RefundStatus::Failure,
            "1" => enums::RefundStatus::Success,
            &_ => Err(errors::ConnectorError::ResponseDeserializationFailed)?,
        };
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BamboraErrorResponse {
    pub code: i32,
    pub category: i32,
    pub message: String,
    pub reference: String,
    pub details: Option<Vec<ErrorDetail>>,
    pub validation: Option<CardValidation>,
    pub card: Option<CardError>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardError {
    pub avs: AVSDetails,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AVSDetails {
    pub message: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ErrorDetail {
    field: String,
    message: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CardValidation {
    id: String,
    approved: i32,
    message_id: i32,
    message: String,
    auth_code: String,
    trans_date: String,
    order_number: String,
    type_: String,
    amount: f64,
    cvd_id: i32,
}
