use crate::{
    connector::utils::{self},
    core::errors,
    pii::PeekInterface,
    services::{self, api::request::Method},
    types::{self, api, storage::enums},
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct ExpresscheckoutPaymentsRequest {
    #[serde(rename = "order.order_id")]
    order_id: String,
    #[serde(rename = "order.amount")]
    amount: String,
    #[serde(rename = "order.return_url")]
    return_url: String,
    #[serde(rename = "order.currency")]
    currency: String,
    #[serde(rename = "order.gateway_id")]
    gateway_id: u8,
    #[serde(rename = "order.metadata.RAZORPAY:gateway_reference_id")]
    gateway_ref_id: Option<String>,
    merchant_id: String,
    payment_method_type: PaymentMethodType,
    card_number: String,
    card_exp_month: String,
    card_exp_year: String,
    name_on_card: String,
    card_security_code: String,
    format: String,
    save_to_locker: bool,
}

#[derive(Serialize, Debug, Default, Eq, PartialEq)]
enum PaymentMethodType {
    #[default]
    Card,
}

// #[derive(Default, Debug, Eq, PartialEq, Serialize)]
// pub struct OrderDetails {
//     order_id: String,
//     amount: String,
//     return_url: String,
//     currency: String,
//     gateway_id: u8
// }

impl TryFrom<&types::PaymentsAuthorizeRouterData> for ExpresscheckoutPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let is_manual_capture = item.request.capture_method.map_or(false, |r| r == enums::CaptureMethod::Manual);
        //Hardcoding gateway reference id as the currenct architecture doesn't provide seamless integration of a payment aggregator
        let gateway_ref_id = if is_manual_capture {
            Some("razorpay_refund".to_string())
        }else{
            Some("rzp_bin_test".to_string())
        };

        match item.request.payment_method_data {
            api::PaymentMethod::Card(ref ccard) => {
                let return_url: String = item
                    .return_url
                    .clone()
                    .ok_or_else(utils::missing_field_err("return_url"))?;
                Ok(Self {
                    order_id: item.payment_id.clone(),
                    amount: item.request.amount.to_string(),
                    return_url,
                    currency: item.request.currency.to_string().to_uppercase(),
                    gateway_id: 23,
                    gateway_ref_id,
                    merchant_id: item.merchant_id.clone(),
                    payment_method_type: PaymentMethodType::Card,
                    card_number: ccard.card_number.peek().clone(),
                    card_exp_month: ccard.card_exp_month.peek().clone(),
                    card_exp_year: ccard.card_exp_year.peek().clone(),
                    name_on_card: ccard.card_holder_name.peek().clone(),
                    card_security_code: ccard.card_cvc.peek().clone(),
                    format: String::from("json"),
                    save_to_locker: false,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment methods".to_string()).into()),
        }
    }
}

pub struct ExpresscheckoutAuthType {
    pub(super) api_key: String,
}

impl TryFrom<&types::ConnectorAuthType> for ExpresscheckoutAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        if let types::ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_string(),
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType)?
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ExpresscheckoutPaymentStatus {
    Charged,
    Success,
    Created,
    PendingVbv,
    AuthenticationFailed,
    #[default]
    AuthorizationFailed,
    JuspayDeclined,
    CaptureInitiated,
    CaptureFailed,
    VoidInitiated,
    VoidFailed,
    Voided,
    Authorized,
    Authorizing,
    PendingAuthentication,
}

impl From<ExpresscheckoutPaymentStatus> for enums::AttemptStatus {
    fn from(item: ExpresscheckoutPaymentStatus) -> Self {
        match item {
            ExpresscheckoutPaymentStatus::Charged => Self::Charged,
            ExpresscheckoutPaymentStatus::Success => Self::Charged,
            ExpresscheckoutPaymentStatus::Created => Self::Started,
            ExpresscheckoutPaymentStatus::AuthenticationFailed => Self::AuthenticationFailed,
            ExpresscheckoutPaymentStatus::AuthorizationFailed => Self::AuthorizationFailed,
            ExpresscheckoutPaymentStatus::Authorizing => Self::Authorizing,
            ExpresscheckoutPaymentStatus::Authorized => Self::Authorized,
            ExpresscheckoutPaymentStatus::PendingVbv => Self::AuthenticationPending,
            ExpresscheckoutPaymentStatus::JuspayDeclined => Self::Failure,
            ExpresscheckoutPaymentStatus::CaptureFailed => Self::CaptureFailed,
            ExpresscheckoutPaymentStatus::CaptureInitiated => Self::CaptureInitiated,
            ExpresscheckoutPaymentStatus::VoidInitiated => Self::VoidInitiated,
            ExpresscheckoutPaymentStatus::VoidFailed => Self::VoidFailed,
            ExpresscheckoutPaymentStatus::Voided => Self::Voided,
            ExpresscheckoutPaymentStatus::PendingAuthentication => Self::AuthenticationPending,
        }
    }
}


#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExpresscheckoutPaymentsResponse {
    #[serde(default, deserialize_with = "deserialize_error_default")]
    status: ExpresscheckoutPaymentStatus,
    txn_uuid: Option<String>,
    payment: Option<Authentication>,
}

fn deserialize_error_default<'de, D, ExpresscheckoutPaymentStatus>(
    deserializer: D,
) -> Result<ExpresscheckoutPaymentStatus, D::Error>
where
    ExpresscheckoutPaymentStatus: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = ExpresscheckoutPaymentStatus::deserialize(deserializer);
    match opt {
        Ok(v) => Ok(v),
        _ => Ok(ExpresscheckoutPaymentStatus::default()),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Authentication {
    authentication: AuthenticationData,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthenticationData {
    url: String,
    method: Method,
}

impl<F, T>
    TryFrom<
        types::ResponseRouterData<
            F,
            ExpresscheckoutPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    > for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::ResponseRouterData<
            F,
            ExpresscheckoutPaymentsResponse,
            T,
            types::PaymentsResponseData,
        >,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: item.response.txn_uuid.map_or(
                    types::ResponseId::NoResponseId,
                    types::ResponseId::ConnectorTransactionId,
                ),
                redirection_data: item.response.payment.map(|r| services::RedirectForm {
                    url: r.authentication.url,
                    method: r.authentication.method,
                    form_fields: HashMap::new(),
                }),
                redirect: true,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct ExpresscheckoutRefundRequest {
    unique_request_id: String,
    amount: String,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for ExpresscheckoutRefundRequest {
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        let refund_req = Self {
            unique_request_id: item.request.refund_id.clone(),
            amount: item.request.amount.to_string(),
        };
        Ok(refund_req)
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone)]
pub enum RefundStatus {
    #[serde(rename = "ERROR")]
    #[default]
    Error,
    #[serde(rename = "BAD REQUEST")]
    BadRequest,
    #[serde(rename = "NOT_FOUND")]
    NotFound,
    #[serde(rename = "SUCCESS")]
    Success,
    #[serde(rename = "PENDING")]
    Pending,
}


impl From<RefundStatus> for enums::RefundStatus {
    fn from(item: RefundStatus) -> Self {
        match item {
            RefundStatus::Success => Self::Success,
            RefundStatus::Pending => Self::Pending,
            _ => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    refunds: Vec<RefundStatusBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefundStatusBlock {
    #[serde(deserialize_with = "deserialize_refund_status_default")]
    status: RefundStatus,
    id: Option<String>,
    unique_request_id: String,
}


fn deserialize_refund_status_default<'de, D, RefundStatus>(
    deserializer: D,
) -> Result<RefundStatus, D::Error>
where
    RefundStatus: Default + Deserialize<'de>,
    D: Deserializer<'de>,
{
    let opt = RefundStatus::deserialize(deserializer);
    match opt {
        Ok(v) => Ok(v),
        _ => Ok(RefundStatus::default()),
    }
}

fn get_status_from_refund_response(refund_id: &str, response: RefundResponse) -> enums::RefundStatus {
    response.refunds.into_iter().fold(enums::RefundStatus::Pending, |acc, r| {
        if r.unique_request_id == refund_id {
            enums::RefundStatus::from(r.status)
        } else {
            acc
        }
    })
}
impl TryFrom<types::RefundsResponseRouterData<api::Execute, RefundResponse>>
    for types::RefundsRouterData<api::Execute>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
            let refund_id = item.data.request.refund_id.clone();
            let status = get_status_from_refund_response(&refund_id, item.response);
            Ok(Self {
                response: Ok(types::RefundsResponseData {
                    connector_refund_id: item.data.request.refund_id.clone(),
                    refund_status: status,
                }),
                ..item.data
            })
    }
}

impl TryFrom<types::RefundsResponseRouterData<api::RSync, RefundResponse>>
    for types::RefundsRouterData<api::RSync>
{
    type Error = error_stack::Report<errors::ParsingError>;
    fn try_from(
        item: types::RefundsResponseRouterData<api::RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_id = item.data.request.refund_id.clone();
        let status = get_status_from_refund_response(&refund_id, item.response);
        Ok(Self {
            response: Ok(types::RefundsResponseData {
                connector_refund_id: item.data.request.refund_id.clone(),
                refund_status: status,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ExpresscheckoutErrorResponse {}
