use common_enums::enums;
use common_utils::types::MinorUnit;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

pub struct HyperswitchRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for HyperswitchRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub struct HyperswitchAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for HyperswitchAuthType {
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

#[derive(Debug, Deserialize)]
pub struct HyperswitchConnectorMetadata {
    pub connector: String,
    pub merchant_connector_id: String,
}

#[derive(Debug, Serialize)]
pub struct HyperswitchPaymentsRequest {
    pub amount: i64,
    pub currency: enums::Currency,
    pub payment_method: enums::PaymentMethod,
    pub confirm: bool,
    pub off_session: bool,
    pub recurring_details: HyperswitchRecurringDetails,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub routing: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct HyperswitchRecurringDetails {
    #[serde(rename = "type")]
    pub recurring_type: &'static str,
    pub data: HyperswitchProcessorPaymentToken,
}

#[derive(Debug, Serialize)]
pub struct HyperswitchProcessorPaymentToken {
    pub processor_payment_token: String,
    pub merchant_connector_id: String,
}

pub fn build_payments_request(
    req: &PaymentsAuthorizeRouterData,
    amount: MinorUnit,
) -> Result<HyperswitchPaymentsRequest, error_stack::Report<errors::ConnectorError>> {
    let processor_payment_token = req
        .request
        .mandate_id
        .as_ref()
        .and_then(|m| m.get_connector_mandate_id())
        .or_else(|| {
            req.payment_method_token
                .as_ref()
                .and_then(|t| t.get_payment_method_token())
                .map(|s| s.expose())
        })
        .ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: "processor_payment_token",
        })?;

    let meta: HyperswitchConnectorMetadata = req
        .connector_meta_data
        .as_ref()
        .ok_or(errors::ConnectorError::MissingRequiredField {
            field_name: "connector_meta_data",
        })
        .and_then(|m| {
            serde_json::from_value(m.peek().clone()).map_err(|_| {
                errors::ConnectorError::InvalidConnectorConfig {
                    config: "connector_meta_data",
                }
                .into()
            })
        })?;

    let routing = serde_json::json!({
        "type": "single",
        "data": {
            "connector": meta.connector,
            "merchant_connector_id": meta.merchant_connector_id
        }
    });

    Ok(HyperswitchPaymentsRequest {
        amount: amount.get_amount_as_i64(),
        currency: req.request.currency,
        payment_method: req.payment_method,
        confirm: true,
        off_session: true,
        recurring_details: HyperswitchRecurringDetails {
            recurring_type: "processor_payment_token",
            data: HyperswitchProcessorPaymentToken {
                processor_payment_token,
                merchant_connector_id: meta.merchant_connector_id,
            },
        },
        routing: Some(routing),
    })
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HyperswitchPaymentStatus {
    Succeeded,
    Failed,
    #[default]
    Processing,
    RequiresCapture,
    Cancelled,
    RequiresCustomerAction,
    RequiresPaymentMethod,
    RequiresConfirmation,
    PartiallyCaptured,
}

impl From<HyperswitchPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: HyperswitchPaymentStatus) -> Self {
        match item {
            HyperswitchPaymentStatus::Succeeded => Self::Charged,
            HyperswitchPaymentStatus::Failed => Self::Failure,
            HyperswitchPaymentStatus::Processing => Self::Authorizing,
            HyperswitchPaymentStatus::RequiresCapture => Self::Authorized,
            HyperswitchPaymentStatus::Cancelled => Self::Voided,
            HyperswitchPaymentStatus::RequiresCustomerAction => Self::AuthenticationPending,
            HyperswitchPaymentStatus::RequiresPaymentMethod => Self::PaymentMethodAwaited,
            HyperswitchPaymentStatus::RequiresConfirmation => Self::ConfirmationAwaited,
            HyperswitchPaymentStatus::PartiallyCaptured => Self::PartialCharged,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperswitchPaymentsResponse {
    pub payment_id: String,
    pub status: HyperswitchPaymentStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, HyperswitchPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, HyperswitchPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let attempt_status = common_enums::AttemptStatus::from(item.response.status.clone());
        let response = if matches!(item.response.status, HyperswitchPaymentStatus::Failed) {
            Err(ErrorResponse {
                status_code: item.http_code,
                code: item
                    .response
                    .error_code
                    .unwrap_or_else(|| "UNKNOWN".to_string()),
                message: item
                    .response
                    .error_message
                    .unwrap_or_else(|| "Unknown error".to_string()),
                reason: None,
                attempt_status: None,
                connector_transaction_id: Some(item.response.payment_id.clone()),
                connector_response_reference_id: None,
                network_advice_code: None,
                network_decline_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.payment_id),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
            })
        };
        Ok(Self {
            status: attempt_status,
            response,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct HyperswitchRefundRequest {
    pub amount: MinorUnit,
}

impl<F> TryFrom<&HyperswitchRouterData<&RefundsRouterData<F>>> for HyperswitchRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &HyperswitchRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount.to_owned(),
        })
    }
}

#[allow(dead_code)]
#[derive(Debug, Copy, Serialize, Default, Deserialize, Clone)]
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

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct HyperswitchErrorResponseBody {
    #[serde(rename = "type")]
    pub error_type: Option<String>,
    pub code: Option<String>,
    pub message: String,
    pub reason: Option<String>,
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct HyperswitchErrorResponse {
    pub error: HyperswitchErrorResponseBody,
}

impl HyperswitchErrorResponse {
    pub fn to_error_response(self, status_code: u16) -> ErrorResponse {
        ErrorResponse {
            status_code,
            code: self.error.code.unwrap_or_else(|| "UNKNOWN".to_string()),
            message: self.error.message.clone(),
            reason: self.error.reason.or(self.error.error_type),
            attempt_status: None,
            connector_transaction_id: None,
            connector_response_reference_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        }
    }
}
