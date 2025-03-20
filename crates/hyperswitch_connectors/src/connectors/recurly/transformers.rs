use common_enums::enums;
use common_utils::{errors::CustomResult, ext_traits::ByteSliceExt, types::StringMinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::{
    router_flow_types::RecoveryRecordBack,
    router_request_types::revenue_recovery::RevenueRecoveryRecordBackRequest,
    router_response_types::revenue_recovery::RevenueRecoveryRecordBackResponse,
    types::RevenueRecoveryRecordBackRouterData,
};
use hyperswitch_interfaces::errors;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};
//TODO: Fill the struct with respective fields
pub struct RecurlyRouterData<T> {
    pub amount: StringMinorUnit, // The type of amount that a connector accepts, for example, String, i64, f64, etc.
    pub router_data: T,
}

impl<T> From<(StringMinorUnit, T)> for RecurlyRouterData<T> {
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
pub struct RecurlyPaymentsRequest {
    amount: StringMinorUnit,
    card: RecurlyCard,
}

#[derive(Default, Debug, Serialize, Eq, PartialEq)]
pub struct RecurlyCard {
    number: cards::CardNumber,
    expiry_month: Secret<String>,
    expiry_year: Secret<String>,
    cvc: Secret<String>,
    complete: bool,
}

impl TryFrom<&RecurlyRouterData<&PaymentsAuthorizeRouterData>> for RecurlyPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &RecurlyRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data.clone() {
            PaymentMethodData::Card(req_card) => {
                let card = RecurlyCard {
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
pub struct RecurlyAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for RecurlyAuthType {
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
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RecurlyPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
}

impl From<RecurlyPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: RecurlyPaymentStatus) -> Self {
        match item {
            RecurlyPaymentStatus::Succeeded => Self::Charged,
            RecurlyPaymentStatus::Failed => Self::Failure,
            RecurlyPaymentStatus::Processing => Self::Authorizing,
        }
    }
}

//TODO: Fill the struct with respective fields
#[derive(Default, Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecurlyPaymentsResponse {
    status: RecurlyPaymentStatus,
    id: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, RecurlyPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, RecurlyPaymentsResponse, T, PaymentsResponseData>,
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
pub struct RecurlyRefundRequest {
    pub amount: StringMinorUnit,
}

impl<F> TryFrom<&RecurlyRouterData<&RefundsRouterData<F>>> for RecurlyRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &RecurlyRouterData<&RefundsRouterData<F>>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
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
pub struct RecurlyErrorResponse {
    pub status_code: u16,
    pub code: String,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecurlyWebhookBody {
    // transaction id
    pub uuid: String,
    pub event_type: RecurlyPaymentEventType,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum RecurlyPaymentEventType {
    #[serde(rename = "succeeded")]
    PaymentSucceeded,
    #[serde(rename = "failed")]
    PaymentFailed,
}

impl RecurlyWebhookBody {
    pub fn get_webhook_object_from_body(body: &[u8]) -> CustomResult<Self, errors::ConnectorError> {
        let webhook_body = body
            .parse_struct::<Self>("RecurlyWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(webhook_body)
    }
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum RecurlyRecordStatus {
    Success,
    Failure,
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl TryFrom<enums::AttemptStatus> for RecurlyRecordStatus {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(status: enums::AttemptStatus) -> Result<Self, Self::Error> {
        match status {
            enums::AttemptStatus::Charged
            | enums::AttemptStatus::PartialCharged
            | enums::AttemptStatus::PartialChargedAndChargeable => Ok(Self::Success),
            enums::AttemptStatus::Failure
            | enums::AttemptStatus::CaptureFailed
            | enums::AttemptStatus::RouterDeclined => Ok(Self::Failure),
            enums::AttemptStatus::AuthenticationFailed
            | enums::AttemptStatus::Started
            | enums::AttemptStatus::AuthenticationPending
            | enums::AttemptStatus::AuthenticationSuccessful
            | enums::AttemptStatus::Authorized
            | enums::AttemptStatus::AuthorizationFailed
            | enums::AttemptStatus::Authorizing
            | enums::AttemptStatus::CodInitiated
            | enums::AttemptStatus::Voided
            | enums::AttemptStatus::VoidInitiated
            | enums::AttemptStatus::CaptureInitiated
            | enums::AttemptStatus::VoidFailed
            | enums::AttemptStatus::AutoRefunded
            | enums::AttemptStatus::Unresolved
            | enums::AttemptStatus::Pending
            | enums::AttemptStatus::PaymentMethodAwaited
            | enums::AttemptStatus::ConfirmationAwaited
            | enums::AttemptStatus::DeviceDataCollectionPending => {
                Err(errors::ConnectorError::NotSupported {
                    message: "Record back flow is only supported for terminal status".to_string(),
                    connector: "recurly",
                }
                .into())
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecurlyRecordbackResponse {
    // inovice id
    pub id: common_utils::id_type::PaymentReferenceId,
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    TryFrom<
        ResponseRouterData<
            RecoveryRecordBack,
            RecurlyRecordbackResponse,
            RevenueRecoveryRecordBackRequest,
            RevenueRecoveryRecordBackResponse,
        >,
    > for RevenueRecoveryRecordBackRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<
            RecoveryRecordBack,
            RecurlyRecordbackResponse,
            RevenueRecoveryRecordBackRequest,
            RevenueRecoveryRecordBackResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let merchant_reference_id = item.response.id;
        Ok(Self {
            response: Ok(RevenueRecoveryRecordBackResponse {
                merchant_reference_id,
            }),
            ..item.data
        })
    }
}
