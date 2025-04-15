use hyperswitch_interfaces::errors;
use common_enums::enums;
use common_utils::{
    errors::CustomResult,
    ext_traits::ByteSliceExt,
    types::StringMinorUnit,
};
use error_stack::ResultExt;
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{convert_uppercase, PaymentsAuthorizeRequestData},
};

// Auth Headers
pub mod auth_headers {
    pub const DUMMYBILLING_API_VERSION: &str = "dummybilling-version";
    pub const DUMMYBILLING_VERSION: &str = "2023-01-01";
}

// Auth Struct
pub struct DummyBillingAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for DummyBillingAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                api_key: api_key.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

// Payment Status
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum DummyBillingPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<DummyBillingPaymentStatus> for enums::AttemptStatus {
    fn from(item: DummyBillingPaymentStatus) -> Self {
        match item {
            DummyBillingPaymentStatus::Succeeded => Self::Charged,
            DummyBillingPaymentStatus::Failed => Self::Failure,
            DummyBillingPaymentStatus::Processing => Self::AuthenticationPending,
        }
    }
}

//TODO: Fill the struct with respective fields
pub struct DummyBillingConnectorRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for DummyBillingConnectorRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

#[derive(Default, Debug, Serialize, PartialEq)]
pub struct DummyBillingConnectorPaymentsRequest {
    amount: StringMinorUnit,
    card: DummyBillingConnectorCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct DummyBillingConnectorCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}
impl TryFrom<&DummyBillingConnectorRouterData<&PaymentsAuthorizeRouterData>>
    for DummyBillingConnectorPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DummyBillingConnectorRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = DummyBillingConnectorCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Payment Request
#[derive(Default, Serialize, PartialEq)]
pub struct DummyBillingPaymentsRequest {
    amount: StringMinorUnit,
    card: DummyBillingConnectorCard,
}


impl TryFrom<&DummyBillingConnectorRouterData<&PaymentsAuthorizeRouterData>>
    for DummyBillingPaymentsRequest 
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DummyBillingConnectorRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = DummyBillingConnectorCard {
                    number: req_card.card_number,
                    expiry_month: req_card.card_exp_month,
                    expiry_year: req_card.card_exp_year,
                    cvc: req_card.card_cvc,
                    complete: item.router_data.request.is_auto_capture()?,
                };
                Ok(Self {
                    amount: item.amount.clone(),
                    card,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
        }
    }
}

// Payment Response
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DummyBillingPaymentsResponse {
    pub status: DummyBillingPaymentStatus,
    pub id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, DummyBillingPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, DummyBillingPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: common_enums::AttemptStatus::from(item.response.status),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.id),
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

// Refund Request
#[derive(Default, Debug, Serialize)]
pub struct DummyBillingRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&DummyBillingConnectorRouterData<&RefundsRouterData<F>>> for DummyBillingRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &DummyBillingConnectorRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Refund Status
#[derive(Debug, Serialize, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
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

// Refund Response
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>>
    for RefundsRouterData<RSync>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<RSync, RefundResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id.to_string(),
                refund_status: enums::RefundStatus::from(item.response.status),
            }),
            ..item.data
        })
    }
}

// Error Response
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct DummyBillingErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct ErrorDetails {
    pub code: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub message: Option<String>,
    pub param: Option<String>,
    pub decline_code: Option<String>,
}

// Webhook
#[derive(Debug, Serialize, Deserialize)]
pub struct DummyBillingWebhookBody {
    #[serde(rename = "type")]
    pub event_type: DummyBillingEventType,
    pub data: DummyBillingWebhookData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DummyBillingWebhookData {
    pub object: DummyBillingWebhookObject,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DummyBillingWebhookObject {
    #[serde(rename = "id")]
    pub invoice_id: String,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    pub customer: String,
    #[serde(rename = "amount_remaining")]
    pub amount: common_utils::types::MinorUnit,
    pub charge: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DummyBillingEventType {
    #[serde(rename = "invoice.paid")]
    PaymentSucceeded,
    #[serde(rename = "invoice.payment_failed")]
    PaymentFailed,
    #[serde(rename = "invoice.voided")]
    InvoiceDeleted,
}

impl DummyBillingWebhookBody {
    pub fn get_webhook_object_from_body(body: &[u8]) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body: Self = body
            .parse_struct::<Self>("DummyBillingWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(webhook_body)
    }
}

// Billing-specific structures
#[derive(Debug, Serialize, Deserialize)]
pub struct DummyBillingInvoiceBody {
    #[serde(rename = "type")]
    pub event_type: DummyBillingInvoiceEventType,
    pub data: DummyBillingInvoiceData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DummyBillingInvoiceData {
    pub object: DummyBillingInvoiceObject,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DummyBillingInvoiceObject {
    #[serde(rename = "id")]
    pub invoice_id: String,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    pub customer: String,
    #[serde(rename = "amount_remaining")]
    pub amount: common_utils::types::MinorUnit,
    pub charge: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DummyBillingInvoiceEventType {
    #[serde(rename = "invoice.paid")]
    PaymentSucceeded,
    #[serde(rename = "invoice.payment_failed")]
    PaymentFailed,
    #[serde(rename = "invoice.voided")]
    InvoiceDeleted,
}

impl DummyBillingInvoiceBody {
    pub fn get_invoice_webhook_data_from_body(
        body: &[u8],
    ) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("DummyBillingInvoiceBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}

// Billing Recovery Details
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DummyBillingRecoveryDetailsData {
    #[serde(rename = "id")]
    pub charge_id: String,
    pub status: DummyBillingChargeStatus,
    pub amount: common_utils::types::MinorUnit,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    pub customer: String,
    pub payment_method: String,
    pub failure_code: Option<String>,
    pub failure_message: Option<String>,
    #[serde(with = "common_utils::custom_serde::timestamp")]
    pub created: PrimitiveDateTime,
    pub payment_method_details: DummyBillingPaymentMethodDetails,
    #[serde(rename = "invoice")]
    pub invoice_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DummyBillingPaymentMethodDetails {
    #[serde(rename = "type")]
    pub type_of_payment_method: DummyBillingPaymentMethod,
    #[serde(rename = "card")]
    pub card_funding_type: DummyBillingCardFundingTypeDetails,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DummyBillingPaymentMethod {
    Card,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DummyBillingCardFundingTypeDetails {
    pub funding: DummyBillingFundingTypes,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename = "snake_case")]
pub enum DummyBillingFundingTypes {
    #[serde(rename = "credit")]
    Credit,
    #[serde(rename = "debit")]
    Debit,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum DummyBillingChargeStatus {
    Succeeded,
    Failed,
}

// Billing Record Back Response
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DummyBillingRecordBackResponse {
    pub id: String,
} 











