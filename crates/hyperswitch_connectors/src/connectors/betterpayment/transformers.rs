use common_enums::{enums, Currency};
use common_utils::types::StringMajorUnit;
use hyperswitch_domain_models::{
    payment_method_data::{PaymentMethodData, WalletData},
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_request_types::{PaymentsAuthorizeData, PaymentsSyncData, RefundsData},
    router_response_types::{PaymentsResponseData, RedirectForm, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundExecuteRouterData, RefundSyncRouterData},
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use masking::ExposeInterface;
use serde::{Deserialize, Serialize};

use crate::types::{RefundsResponseRouterData, ResponseRouterData};

pub struct BetterpaymentRouterData<T> {
    pub amount: StringMajorUnit,
    pub router_data: T,
}

impl<T> From<(StringMajorUnit, T)> for BetterpaymentRouterData<T> {
    fn from((amount, router_data): (StringMajorUnit, T)) -> Self {
        Self { amount, router_data }
    }
}

pub struct BetterpaymentAuthType {
    pub(super) api_key: Secret<String>,
    pub(super) outgoing_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for BetterpaymentAuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        match auth_type {
            ConnectorAuthType::BodyKey { api_key, key1 } => Ok(Self {
                api_key: api_key.to_owned(),
                outgoing_key: key1.to_owned(),
            }),
            _ => Err(errors::ConnectorError::FailedToObtainAuthType)?,
        }
    }
}

// ── Authorize ────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BetterpaymentPaymentsRequest {
    api_key: Secret<String>,
    payment_type: String,
    amount: StringMajorUnit,
    currency: Currency,
    order_id: String,
    success_url: String,
    error_url: String,
    postback_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    customer_email: Option<Secret<String>>,
}

impl TryFrom<&BetterpaymentRouterData<&PaymentsAuthorizeRouterData>>
    for BetterpaymentPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &BetterpaymentRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = BetterpaymentAuthType::try_from(&item.router_data.connector_auth_type)?;

        match &item.router_data.request.payment_method_data {
            PaymentMethodData::Wallet(WalletData::WeroRedirect {}) => {
                let return_url = item
                    .router_data
                    .request
                    .router_return_url
                    .clone()
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?;

                let postback_url = item.router_data.request.get_webhook_url()?;

                let customer_email = item
                    .router_data
                    .request
                    .email
                    .clone()
                    .map(|e| Secret::new(e.expose().to_string()));

                Ok(Self {
                    api_key: auth.api_key,
                    payment_type: "WR".to_string(),
                    amount: item.amount.clone(),
                    currency: item.router_data.request.currency,
                    order_id: item.router_data.connector_request_reference_id.clone(),
                    success_url: return_url.clone(),
                    error_url: return_url,
                    postback_url,
                    customer_email,
                })
            }
            _ => Err(errors::ConnectorError::NotImplemented(
                "payment method".to_string(),
            ))?,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BetterpaymentPaymentsResponse {
    pub transaction_id: String,
    pub status: BetterpaymentPaymentStatus,
    pub redirect_url: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BetterpaymentPaymentStatus {
    Pending,
    Completed,
    Success,
    Failed,
    Error,
    Cancelled,
}

impl From<BetterpaymentPaymentStatus> for enums::AttemptStatus {
    fn from(status: BetterpaymentPaymentStatus) -> Self {
        match status {
            BetterpaymentPaymentStatus::Pending => Self::AuthenticationPending,
            BetterpaymentPaymentStatus::Completed
            | BetterpaymentPaymentStatus::Success => Self::Charged,
            BetterpaymentPaymentStatus::Failed
            | BetterpaymentPaymentStatus::Error => Self::Failure,
            BetterpaymentPaymentStatus::Cancelled => Self::Voided,
        }
    }
}

impl<F, T>
    TryFrom<ResponseRouterData<F, BetterpaymentPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: ResponseRouterData<F, BetterpaymentPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let status = enums::AttemptStatus::from(item.response.status.clone());

        let response = if matches!(status, enums::AttemptStatus::Failure) {
            Err(ErrorResponse {
                code: "BETTERPAYMENT_ERROR".to_string(),
                message: item
                    .response
                    .error_message
                    .clone()
                    .unwrap_or_else(|| "Payment failed".to_string()),
                reason: item.response.error_message.clone(),
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.transaction_id.clone()),
            })
        } else {
            let redirection = item.response.redirect_url.as_ref().map(|url| {
                RedirectForm::Form {
                    endpoint: url.clone(),
                    method: common_utils::request::Method::Get,
                    form_fields: std::collections::HashMap::new(),
                }
            });

            Ok(PaymentsResponseData::TransactionResponse {
                resource_id: hyperswitch_domain_models::router_response_types::ResponseId::ConnectorTransactionId(
                    item.response.transaction_id.clone(),
                ),
                redirection_data: Box::new(redirection),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                connector_response_reference_id: Some(item.response.transaction_id),
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

// ── Refund ────────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct BetterpaymentRefundRequest {
    api_key: Secret<String>,
    transaction_id: String,
    amount: StringMajorUnit,
}

impl TryFrom<&BetterpaymentRouterData<&RefundExecuteRouterData>> for BetterpaymentRefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: &BetterpaymentRouterData<&RefundExecuteRouterData>,
    ) -> Result<Self, Self::Error> {
        let auth = BetterpaymentAuthType::try_from(&item.router_data.connector_auth_type)?;
        Ok(Self {
            api_key: auth.api_key,
            transaction_id: item.router_data.request.connector_transaction_id.clone(),
            amount: item.amount.clone(),
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BetterpaymentRefundResponse {
    pub transaction_id: String,
    pub status: BetterpaymentRefundStatus,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum BetterpaymentRefundStatus {
    Pending,
    Completed,
    Success,
    Failed,
    Error,
}

impl From<BetterpaymentRefundStatus> for enums::RefundStatus {
    fn from(status: BetterpaymentRefundStatus) -> Self {
        match status {
            BetterpaymentRefundStatus::Pending => Self::Pending,
            BetterpaymentRefundStatus::Completed | BetterpaymentRefundStatus::Success => {
                Self::Success
            }
            BetterpaymentRefundStatus::Failed | BetterpaymentRefundStatus::Error => Self::Failure,
        }
    }
}

impl
    TryFrom<
        RefundsResponseRouterData<
            hyperswitch_domain_models::router_flow_types::refunds::Execute,
            BetterpaymentRefundResponse,
        >,
    > for RefundExecuteRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: RefundsResponseRouterData<
            hyperswitch_domain_models::router_flow_types::refunds::Execute,
            BetterpaymentRefundResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status.clone());
        Ok(Self {
            response: if matches!(refund_status, enums::RefundStatus::Failure) {
                Err(ErrorResponse {
                    code: "BETTERPAYMENT_REFUND_ERROR".to_string(),
                    message: item
                        .response
                        .error_message
                        .clone()
                        .unwrap_or_else(|| "Refund failed".to_string()),
                    reason: item.response.error_message,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.transaction_id.clone()),
                })
            } else {
                Ok(RefundsResponseData {
                    connector_refund_id: item.response.transaction_id,
                    refund_status,
                })
            },
            ..item.data
        })
    }
}

impl
    TryFrom<
        RefundsResponseRouterData<
            hyperswitch_domain_models::router_flow_types::refunds::RSync,
            BetterpaymentRefundResponse,
        >,
    > for RefundSyncRouterData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(
        item: RefundsResponseRouterData<
            hyperswitch_domain_models::router_flow_types::refunds::RSync,
            BetterpaymentRefundResponse,
        >,
    ) -> Result<Self, Self::Error> {
        let refund_status = enums::RefundStatus::from(item.response.status.clone());
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.transaction_id,
                refund_status,
            }),
            ..item.data
        })
    }
}

// ── Webhook ───────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, Serialize)]
pub struct BetterpaymentWebhookPayload {
    pub transaction_id: String,
    pub order_id: String,
    pub status: BetterpaymentPaymentStatus,
    pub amount: Option<String>,
    pub currency: Option<String>,
    pub hash: Option<String>,
}
