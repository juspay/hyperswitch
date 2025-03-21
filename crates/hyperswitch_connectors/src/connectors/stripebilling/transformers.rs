#[cfg(feature = "v2")]
use std::str::FromStr;

use common_enums::enums;
use common_utils::{errors::CustomResult, ext_traits::ByteSliceExt, types::StringMinorUnit};
use error_stack::ResultExt;
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use hyperswitch_domain_models::revenue_recovery;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::{convert_uppercase, PaymentsAuthorizeRequestData},
};

//TODO: Fill the struct with respective fields
pub struct StripebillingRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for StripebillingRouterData<T> {
    fn from((amount, item): (StringMinorUnit, T)) -> Self {
        //Todo :  use utils to convert the amount to the type of amount that a connector accepts
        Self {
            amount,
            router_data: item,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, PartialEq)]
pub struct StripebillingPaymentsRequest {
    amount: StringMinorUnit,
    card: StripebillingCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct StripebillingCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&StripebillingRouterData<&PaymentsAuthorizeRouterData>>
    for StripebillingPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StripebillingRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = StripebillingCard {
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

//TODO: Fill the struct with respective fields
// Auth Struct
pub struct StripebillingAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for StripebillingAuthType {
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
// PaymentsResponse
//TODO: Append the remaining status flags
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Copy)]
#[serde(rename_all = "lowercase")]
pub enum StripebillingPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<StripebillingPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: StripebillingPaymentStatus) -> Self {
        match item {
            StripebillingPaymentStatus::Succeeded => Self::Charged,
            StripebillingPaymentStatus::Failed => Self::Failure,
            StripebillingPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StripebillingPaymentsResponse {
    status: StripebillingPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, StripebillingPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, StripebillingPaymentsResponse, T, PaymentsResponseData>,
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

//TODO: Fill the struct with respective fields
// REFUND :
// Type definition for RefundRequest
#[derive(Default, Debug, Serialize)]
pub struct StripebillingRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&StripebillingRouterData<&RefundsRouterData<F>>> for StripebillingRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &StripebillingRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

// Type definition for Refund Response

#[allow(dead_code)]
#[derive(Debug, Serialize, Default, Deserialize, Clone, Copy)]
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
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct RefundResponse {
    id: String,
    status: RefundStatus,
}

impl TryFrom<RefundsResponseRouterData<Execute, RefundResponse>> for RefundsRouterData<Execute> {
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

impl TryFrom<RefundsResponseRouterData<RSync, RefundResponse>> for RefundsRouterData<RSync> {
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

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct StripebillingErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripebillingWebhookBody {
    #[serde(rename = "type")]
    pub event_type: StripebillingEventType,
    pub data: StripebillingWebhookData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripebillingInvoiceBody {
    #[serde(rename = "type")]
    pub event_type: StripebillingEventType,
    pub data: StripebillingInvoiceData,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingEventType {
    #[serde(rename = "invoice.paid")]
    PaymentSucceeded,
    #[serde(rename = "invoice.payment_failed")]
    PaymentFailed,
    #[serde(rename = "invoice.voided")]
    InvoiceDeleted,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookData {
    pub object: StripebillingWebhookObject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingInvoiceData {
    pub object: StripebillingWebhookObject,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StripebillingWebhookObject {
    #[serde(rename = "id")]
    pub invoice_id: String,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    pub customer: String,
    #[serde(rename = "amount_remaining")]
    pub amount: common_utils::types::MinorUnit,
    pub charge: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StripebillingInvoiceObject {
    #[serde(rename = "id")]
    pub invoice_id: String,
    #[serde(deserialize_with = "convert_uppercase")]
    pub currency: enums::Currency,
    #[serde(rename = "amount_remaining")]
    pub amount: common_utils::types::MinorUnit,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripePaymentMethodDetails {
    #[serde(rename = "type")]
    pub type_of_payment_method: StripebillingPaymentMethod,
    #[serde(rename = "card")]
    pub card_funding_type: StripeCardFundingTypeDetails,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingPaymentMethod {
    Card,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StripeCardFundingTypeDetails {
    pub funding: StripebillingFundingTypes,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename = "snake_case")]
pub enum StripebillingFundingTypes {
    #[serde(rename = "credit")]
    Credit,
    #[serde(rename = "debit")]
    Debit,
    #[serde(rename = "prepaid")]
    Prepaid,
    #[serde(rename = "unknown")]
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum StripebillingChargeStatus {
    Succeeded,
    Failed,
    Pending,
}

impl StripebillingWebhookBody {
    pub fn get_webhook_object_from_body(body: &[u8]) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body: Self = body
            .parse_struct::<Self>("StripebillingWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;

        Ok(webhook_body)
    }
}

impl StripebillingInvoiceBody {
    pub fn get_invoice_webhook_data_from_body(
        body: &[u8],
    ) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("StripebillingInvoiceBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<StripebillingInvoiceBody> for revenue_recovery::RevenueRecoveryInvoiceData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: StripebillingInvoiceBody) -> Result<Self, Self::Error> {
        let merchant_reference_id =
            common_utils::id_type::PaymentReferenceId::from_str(&item.data.object.invoice_id)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(Self {
            amount: item.data.object.amount,
            currency: item.data.object.currency,
            merchant_reference_id,
        })
    }
}
