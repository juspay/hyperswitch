// use base64::Engine;
// use error_stack::{IntoReport, ResultExt};
use serde::{Deserialize, Serialize};
use storage_models::enums as storage_enums;
use std::convert::From;

use crate::{
    // connector::utils::AccessTokenRequestInfo,
    // consts,
    core::errors,
    pii::{self, Secret},
    types::{self, api, storage::enums},
    // utils::OptionExt,
};
//TODO: Fill the struct with respective fields
#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BamboraPaymentsRequest {
    pub amount: i64,
    #[serde(rename = "payment_method")]
    pub payment_method: String,
    pub card: BamboraCard,
}

#[derive(Debug, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BamboraCard {
    pub name: Secret<String>,
    pub number: Secret<String, pii::CardNumber>,
    #[serde(rename = "expiry_month")]
    pub expiry_month: Secret<String>,
    #[serde(rename = "expiry_year")]
    pub expiry_year: Secret<String>,
    pub cvd: Secret<String>,
    pub complete: bool,
}

impl TryFrom<&types::PaymentsAuthorizeRouterData> for BamboraPaymentsRequest  {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        // let _auth_type = BamboraAuthType::try_from(&item.connector_auth_type)?;
        let payment_method_detail = match item.request.payment_method_data.clone() {
            api::PaymentMethod::Card(ccard) => Ok(BamboraCard {
                    name: ccard.card_holder_name,
                    number: ccard.card_number,
                    expiry_month: ccard.card_exp_month,
                    expiry_year: ccard.card_exp_year,
                    cvd: ccard.card_cvc,
                    complete: item.request.capture_method == Some(storage_enums::CaptureMethod::Automatic),
                }),
            _ => Err(errors::ConnectorError::NotImplemented(
                "Unknown payment method".to_string(),
            )),
        }?;
        Ok(Self {
            
            card: payment_method_detail,
            amount: item.request.amount,
            payment_method: String::from("card"),
        })
    }
}


//TODO: Fill the struct with respective fields
// Auth Struct
pub struct BamboraAuthType {
    pub(super) api_key: String
}

impl TryFrom<&types::ConnectorAuthType> for BamboraAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &types::ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            types::ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_string(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BamboraPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<BamboraPaymentStatus> for enums::AttemptStatus {
    fn from(item: BamboraPaymentStatus) -> Self {
        match item {
            BamboraPaymentStatus::Succeeded => Self::Charged,
            BamboraPaymentStatus::Failed => Self::Failure,
            BamboraPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BamboraPaymentsResponse {
    pub id: String,
    #[serde(rename = "authorizing_merchant_id")]
    pub authorizing_merchant_id: i64,
    pub approved: String,
    #[serde(rename = "message_id")]
    pub message_id: String,
    pub message: BambaroPaymentStatus,
    #[serde(rename = "auth_code")]
    pub auth_code: String,
    pub created: String,
    #[serde(rename = "order_number")]
    pub order_number: String,
    #[serde(rename = "type")]
    pub type_field: BamboraPREAuthType,
    #[serde(rename = "payment_method")]
    pub payment_method: String,
    #[serde(rename = "risk_score")]
    pub risk_score: f64,
    pub amount: f64,
    pub custom: Custom,
    pub card: Card,
    pub links: Vec<Link>,
}

#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct Custom {
    pub ref1: String,
    pub ref2: String,
    pub ref3: String,
    pub ref4: String,
    pub ref5: String,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    #[serde(rename = "card_type")]
    pub card_type: String,
    #[serde(rename = "last_four")]
    pub last_four: String,
    #[serde(rename = "card_bin")]
    pub card_bin: String,
    #[serde(rename = "address_match")]
    pub address_match: i64,
    #[serde(rename = "postal_result")]
    pub postal_result: i64,
    #[serde(rename = "avs_result")]
    pub avs_result: String,
    #[serde(rename = "cvd_result")]
    pub cvd_result: String,
    pub avs: Avs,
}


#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct Link {
    pub rel: String,
    pub href: String,
    pub method: String,
}


#[derive(Default, Debug, Clone, Deserialize, PartialEq)]
pub struct Avs {
    pub id: String,
    pub message: String,
    pub processed: bool,
}

#[derive(Debug, Serialize, Eq, PartialEq, Default, Deserialize, Clone)]
pub enum BambaroPaymentStatus {
    Approved,
    #[default]
    Pending,
}

#[derive(Debug, Serialize, Eq, PartialEq, Default, Deserialize, Clone)]
pub enum BamboraPREAuthType {
    #[serde(rename = "PA")]
    PAs,
    #[default]
    P,
    #[serde(rename = "PAC")]
    Pac,
    R,
}

impl From<BamboraPREAuthType> for enums::AttemptStatus {
    fn from(item: BamboraPREAuthType) -> Self {
        match item {
            BamboraPREAuthType::P => Self::Charged,
            BamboraPREAuthType::PAs => Self::Authorized,
            BamboraPREAuthType::Pac => Self::Charged,
            BamboraPREAuthType::R => Self::Pending,
            
        }
    }
}

impl<F,T> TryFrom<types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData<F, BamboraPaymentsResponse, T, types::PaymentsResponseData>) -> Result<Self,Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.type_field),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BamboraPaymentsSyncResponse{
    pub id: i64,
    #[serde(rename = "authorizing_merchant_id")]
    pub authorizing_merchant_id: i64,
    pub approved: i64,
    #[serde(rename = "message_id")]
    pub message_id: i64,
    pub message: BambaroPaymentStatus,
    #[serde(rename = "auth_code")]
    pub auth_code: String,
    pub created: String,
    pub amount: f64,
    #[serde(rename = "order_number")]
    pub order_number: String,
    #[serde(rename = "type")]
    pub type_field: BamboraPREAuthType,
    pub comments: String,
    #[serde(rename = "batch_number")]
    pub batch_number: String,
    #[serde(rename = "total_refunds")]
    pub total_refunds: f64,
    #[serde(rename = "total_completions")]
    pub total_completions: f64,
    #[serde(rename = "payment_method")]
    pub payment_method: String,
    pub card: SyncResponseCard,
    pub billing: Billing,
    pub shipping: Shipping,
    pub custom: Custom,
    #[serde(rename = "adjusted_by")]
    pub adjusted_by: Vec<Option<serde_json::Value>>,
    pub links: Vec<Link>,
}


#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResponseCard {
    pub name: String,
    #[serde(rename = "expiry_month")]
    pub expiry_month: String,
    #[serde(rename = "expiry_year")]
    pub expiry_year: String,
    #[serde(rename = "card_type")]
    pub card_type: String,
    #[serde(rename = "last_four")]
    pub last_four: String,
    #[serde(rename = "avs_result")]
    pub avs_result: String,
    #[serde(rename = "cvd_result")]
    pub cvd_result: String,
    #[serde(rename = "cavv_result")]
    pub cavv_result: String,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Billing {
    pub name: String,
    #[serde(rename = "address_line1")]
    pub address_line1: String,
    #[serde(rename = "address_line2")]
    pub address_line2: String,
    pub city: String,
    pub province: String,
    pub country: String,
    #[serde(rename = "postal_code")]
    pub postal_code: String,
    #[serde(rename = "phone_number")]
    pub phone_number: String,
    #[serde(rename = "email_address")]
    pub email_address: String,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shipping {
    pub name: String,
    #[serde(rename = "address_line1")]
    pub address_line1: String,
    #[serde(rename = "address_line2")]
    pub address_line2: String,
    pub city: String,
    pub province: String,
    pub country: String,
    #[serde(rename = "postal_code")]
    pub postal_code: String,
    #[serde(rename = "phone_number")]
    pub phone_number: String,
    #[serde(rename = "email_address")]
    pub email_address: String,
}


impl <F, T> TryFrom<types::ResponseRouterData<F, BamboraPaymentsSyncResponse, T, types::PaymentsResponseData>> for types::RouterData<F, T, types::PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::ResponseRouterData <F, BamboraPaymentsSyncResponse, T, types::PaymentsResponseData, >, ) -> Result<Self, Self::Error> {
        // let order = match item.response.orders.first() {
        //     Some(order) => order,
        //     _ => Err(errors::ConnectorError::ResponseHandlingFailed)?,
        // };
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.type_field),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.to_string()),
                redirect: false,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            amount_captured: Some(item.response.amount as i64),
            ..item.data
        })
    }
}


//TODO: Fill the struct with respective fields
// Capture :
// Type definition for CaptureRequest
#[derive(Default, Debug, Serialize)]
pub struct BamboraCaptureRequest {
    pub amount: i64,
}

impl TryFrom<&types::PaymentsCaptureRouterData> for BamboraCaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCaptureRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item
            .request
            .amount_to_capture
            .unwrap_or(item.request.amount),
        })
    }
}


#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BamboraCaptureResponse{
    pub id: i64,
    #[serde(rename = "authorizing_merchant_id")]
    pub authorizing_merchant_id: i64,
    pub approved: i64,
    #[serde(rename = "message_id")]
    pub message_id: i64,
    pub message: BambaroPaymentStatus,
    #[serde(rename = "auth_code")]
    pub auth_code: String,
    pub created: String,
    pub amount: f64,
    #[serde(rename = "order_number")]
    pub order_number: String,
    #[serde(rename = "type")]
    pub type_field: BamboraPREAuthType,
    pub comments: String,
    #[serde(rename = "batch_number")]
    pub batch_number: String,
    #[serde(rename = "total_refunds")]
    pub total_refunds: f64,
    #[serde(rename = "total_completions")]
    pub total_completions: f64,
    #[serde(rename = "payment_method")]
    pub payment_method: String,
    pub card: SyncResponseCard,
    pub billing: Billing,
    pub shipping: Shipping,
    pub custom: Custom,
    #[serde(rename = "adjusted_by")]
    pub adjusted_by: Vec<Option<serde_json::Value>>,
    pub links: Vec<Link>,
}


impl TryFrom<types::PaymentsCaptureResponseRouterData<BamboraCaptureResponse>> for types::PaymentsCaptureRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from( item: types::PaymentsCaptureResponseRouterData<BamboraCaptureResponse>, ) -> Result<Self, Self::Error> {
        let (status, amount_captured) = match item.response.message {
            BambaroPaymentStatus::Approved => (
                storage_enums::AttemptStatus::Charged,
                Some(item.response.amount as i64),
            ),
            _ => (storage_enums::AttemptStatus::Pending, None),
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.to_string()),
                redirect: false,
                redirection_data: None,
                mandate_reference: None,
                connector_metadata: None,
            }),
            amount_captured,
            ..item.data
        })
    }
}

//Cancel Authorization
#[derive(Default, Debug, Serialize)]
pub struct BamboreCancelRequest {
    pub amount: Option<String>,
}
impl TryFrom<&types::PaymentsCancelRouterData> for BamboreCancelRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::PaymentsCancelRouterData) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: Some(item.request.cancellation_reason.clone().unwrap_or_else(|| "12".to_string())),
        })
    }
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BamboraCancelResponse {
    psp_reference: String,
    status: CancelStatus,
}

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CancelStatus {
    Received,
    #[default]
    Processing,
}

impl TryFrom<types::PaymentsCancelResponseRouterData<BamboraCancelResponse>> for types::PaymentsCancelRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::PaymentsCancelResponseRouterData<BamboraCancelResponse>,) -> Result<Self, Self::Error> {
        let status = match item.response.status {
            CancelStatus::Received =>  storage_enums::AttemptStatus::Voided ,
            _ => storage_enums::AttemptStatus::Voided,
        };
        Ok(Self {
            status,
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.psp_reference),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
            }),
            ..item.data
        })
    }
}

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct BamboraRefundRequest {
    amount: i64,
}

impl<F> TryFrom<&types::RefundsRouterData<F>> for BamboraRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RefundsRouterData<F>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.request.refund_amount,
        })
    }
}

// Type definition for Refund Response

#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
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
            //TODO: Review mapping
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefundResponse {
    pub id: i64,
    #[serde(rename = "authorizing_merchant_id")]
    pub authorizing_merchant_id: i64,
    pub approved: i64,
    #[serde(rename = "message_id")]
    pub message_id: i64,
    pub message: BambaroPaymentStatus,
    #[serde(rename = "auth_code")]
    pub auth_code: String,
    pub created: String,
    pub amount: f64,
    #[serde(rename = "order_number")]
    pub order_number: String,
    #[serde(rename = "type")]
    pub type_field: BamboraPREAuthType,
    #[serde(rename = "payment_method")]
    pub payment_method: String,
    pub card: SyncResponseCard,
    pub custom: Custom,
    pub links: Vec<Link>,
}

impl<F> TryFrom<types::RefundsResponseRouterData<F, RefundResponse>> for types::RefundsRouterData<F> {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: types::RefundsResponseRouterData<F, RefundResponse>,) -> Result<Self, Self::Error> {
        let refund_status = match item.response.message {
            // From the docs, the only value returned is "received", outcome of refund is available
            // through refund notification webhook
            BambaroPaymentStatus::Approved => storage_enums::RefundStatus::Success,
            _ => storage_enums::RefundStatus::Pending,
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct BamboraErrorResponse {}
