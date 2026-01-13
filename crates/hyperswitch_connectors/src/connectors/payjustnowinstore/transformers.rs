use common_enums::enums;
use common_utils::{ext_traits::ValueExt, request::Method, types::MinorUnit};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::Execute,
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::{
    consts::{NO_ERROR_CODE, NO_ERROR_MESSAGE},
    errors,
};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{RefundsResponseRouterData, ResponseRouterData},
    utils,
};

pub struct PayjustnowinstoreRouterData<T> {
    pub amount: MinorUnit,
    pub router_data: T,
}

impl<T> From<(MinorUnit, T)> for PayjustnowinstoreRouterData<T> {
    fn from((amount, item): (MinorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

pub struct PayjustnowinstoreAuthType {
    pub(super) merchant_api_key: Secret<String>,
    pub(super) merchant_terminal_id: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayjustnowinstoreAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                merchant_api_key: api_key.to_owned(),
                merchant_terminal_id: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct PayjustnowinstorePaymentsRequest {
    amount: MinorUnit,
    currency: common_enums::Currency,
    merchant_reference: String,
    callback_url: String,
    items: Vec<OrderItem>,
}

#[derive(Debug, Serialize, PartialEq)]
pub struct OrderItem {
    name: String,
    sku: Option<String>,
    quantity: u32,
    price: MinorUnit,
}

impl TryFrom<&PayjustnowinstoreRouterData<&PaymentsAuthorizeRouterData>>
    for PayjustnowinstorePaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayjustnowinstoreRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let items = item
            .router_data
            .request
            .order_details
            .as_ref()
            .map(|order_details| {
                order_details
                    .iter()
                    .map(|order| {
                        let product_name = order.product_name.trim();
                        if product_name.is_empty() {
                            return Err(errors::ConnectorError::MissingRequiredField {
                                field_name: "order_details[].product_name",
                            });
                        }
                        let sku = order.product_id.as_ref().and_then(|id| {
                            let trimmed = id.trim();
                            if trimmed.is_empty() {
                                None
                            } else {
                                Some(trimmed.to_string())
                            }
                        });

                        Ok(OrderItem {
                            name: product_name.to_string(),
                            sku,
                            quantity: u32::from(order.quantity),
                            price: order.amount,
                        })
                    })
                    .collect::<Result<Vec<OrderItem>, errors::ConnectorError>>()
            })
            .transpose()?
            .unwrap_or_default();

        Ok(Self {
            amount: item.amount,
            currency: item.router_data.request.currency,
            merchant_reference: item
                .router_data
                .request
                .merchant_order_reference_id
                .clone()
                .unwrap_or(item.router_data.payment_id.clone()),
            // Webhooks are not implemented yet for PJN In-Store, and `callback_url` is a mandatory field.
            // Since PJN Instore does not accept null or empty values, a placeholder is used here.
            callback_url: "callback_url".to_string(),
            items,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayjustnowinstorePaymentsResponse {
    token: String,
    amount: MinorUnit,
    scan_url: url::Url,
}

impl<F, T>
    TryFrom<ResponseRouterData<F, PayjustnowinstorePaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PayjustnowinstorePaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let redirection_data = Some(RedirectForm::from((item.response.scan_url, Method::Get)));

        Ok(Self {
            status: common_enums::AttemptStatus::AuthenticationPending,
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.token),
                redirection_data: Box::new(redirection_data),
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

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayjustnowinstorePaymentStatus {
    #[default]
    Pending,
    Paid,
    PaymentFailed,
    OrderCancelled,
    OrderRefunded,
}

impl From<PayjustnowinstorePaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PayjustnowinstorePaymentStatus) -> Self {
        match item {
            PayjustnowinstorePaymentStatus::Pending => Self::AuthenticationPending,
            PayjustnowinstorePaymentStatus::Paid => Self::Charged,
            PayjustnowinstorePaymentStatus::PaymentFailed => Self::Failure,
            PayjustnowinstorePaymentStatus::OrderCancelled => Self::Voided,
            PayjustnowinstorePaymentStatus::OrderRefunded => Self::AutoRefunded,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayjustnowinstoreSyncResponse {
    merchant_reference: String,
    token: String,
    scan_url: Option<url::Url>,
    payment_status: PayjustnowinstorePaymentStatus,
    amount: Option<MinorUnit>,
    reason: Option<String>,
    paid_at: Option<String>,
    cancelled_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayjustnowinstorePaymentsResponseMetadata {
    merchant_reference: String,
}

impl<F, T> TryFrom<ResponseRouterData<F, PayjustnowinstoreSyncResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PayjustnowinstoreSyncResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.payment_status);
        let redirection_data = item
            .response
            .scan_url
            .map(|url| RedirectForm::from((url, Method::Get)));

        let response = if utils::is_payment_failure(status) {
            Err(ErrorResponse {
                code: NO_ERROR_CODE.to_string(),
                message: item
                    .response
                    .reason
                    .clone()
                    .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                reason: item.response.reason.clone(),
                status_code: item.http_code,
                attempt_status: Some(status),
                connector_transaction_id: Some(item.response.token.clone()),
                connector_response_reference_id: None,
                network_decline_code: None,
                network_advice_code: None,
                network_error_message: None,
                connector_metadata: None,
            })
        } else {
            let connector_metadata = Some(serde_json::json!(
                PayjustnowinstorePaymentsResponseMetadata {
                    merchant_reference: item.response.merchant_reference,
                }
            ));
            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.token),
                redirection_data: Box::new(redirection_data),
                mandate_reference: Box::new(None),
                connector_metadata,
                network_txn_id: None,
                connector_response_reference_id: None,
                incremental_authorization_allowed: None,
                charges: None,
            })
        };

        Ok(Self {
            status,
            response,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize)]
pub struct PayjustnowinstoreRefundRequest {
    request_id: String,
    merchant_reference: String,
    token: String,
    amount: MinorUnit,
}

impl<F> TryFrom<&PayjustnowinstoreRouterData<&RefundsRouterData<F>>>
    for PayjustnowinstoreRefundRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayjustnowinstoreRouterData<&RefundsRouterData<F>>,
    ) -> Result<Self, Self::Error> {
        let metadata: PayjustnowinstorePaymentsResponseMetadata = item
            .router_data
            .request
            .connector_metadata
            .as_ref()
            .ok_or(errors::ConnectorError::NoConnectorMetaData)?
            .clone()
            .parse_value("PayjustnowinstorePaymentsResponseMetadata")
            .change_context(errors::ConnectorError::ParsingFailed)?;

        Ok(Self {
            request_id: item.router_data.request.refund_id.clone(),
            merchant_reference: metadata.merchant_reference,
            token: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount.to_owned(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PayjustnowinstoreRefundStatus {
    Refunded,
    Failed,
}

impl From<PayjustnowinstoreRefundStatus> for enums::RefundStatus {
    fn from(item: PayjustnowinstoreRefundStatus) -> Self {
        match item {
            PayjustnowinstoreRefundStatus::Refunded => Self::Success,
            PayjustnowinstoreRefundStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayjustnowinstoreRefundResponse {
    status: PayjustnowinstoreRefundStatus,
    reason: Option<String>,
    refunded_at: Option<String>,
    amount_refunded: MinorUnit,
    refund_request_id: String,
}

impl TryFrom<RefundsResponseRouterData<Execute, PayjustnowinstoreRefundResponse>>
    for RefundsRouterData<Execute>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: RefundsResponseRouterData<Execute, PayjustnowinstoreRefundResponse>,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status);
        let response = if utils::is_refund_failure(refund_status) {
            Err(ErrorResponse {
                code: NO_ERROR_CODE.to_string(),
                message: item
                    .response
                    .reason
                    .clone()
                    .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                reason: item.response.reason,
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
                connector_refund_id: item.response.refund_request_id.clone(),
                refund_status,
            })
        };

        Ok(Self {
            response,
            ..item.data
        })
    }
}

#[derive(Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct PayjustnowinstoreErrorResponse {
    pub error: Option<String>,
}
