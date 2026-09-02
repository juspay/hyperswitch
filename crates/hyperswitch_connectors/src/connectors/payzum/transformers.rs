use std::collections::HashMap;

use common_enums::{enums, Currency};
use common_utils::{request::Method, types::FloatMajorUnit};
use hyperswitch_domain_models::{
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, RouterData},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RedirectForm},
    types::{PaymentsAuthorizeRouterData, PaymentsSyncRouterData},
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    types::{PaymentsSyncResponseRouterData, ResponseRouterData},
    utils::PaymentsAuthorizeRequestData,
};

pub struct PayzumRouterData<T> {
    pub amount: FloatMajorUnit,
    pub router_data: T,
}

impl<T> From<(FloatMajorUnit, T)> for PayzumRouterData<T> {
    fn from((amount, item): (FloatMajorUnit, T)) -> Self {
        Self {
            amount,
            router_data: item,
        }
    }
}

/// Invoice creation on the merchant surface (`POST /v1/payment`).
///
/// `price_amount` must serialize as a JSON *number* — the API rejects string
/// amounts with `INVALID_REQUEST` — which is why the connector uses
/// `FloatMajorUnit` rather than the string-based converters.
#[derive(Debug, Serialize, PartialEq)]
pub struct PayzumPaymentsRequest {
    price_amount: FloatMajorUnit,
    price_currency: Currency,
    /// "all" defers the asset choice to the buyer on the hosted checkout,
    /// limited to the tokens the merchant account accepts (enforced
    /// server-side, configured in the Payzum dashboard).
    pay_currency: String,
    order_id: String,
    ipn_callback_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    success_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cancel_url: Option<String>,
}

impl TryFrom<&PayzumRouterData<&PaymentsAuthorizeRouterData>> for PayzumPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &PayzumRouterData<&PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        match item.router_data.request.payment_method_data {
            PaymentMethodData::Crypto(_) => Ok(Self {
                price_amount: item.amount,
                price_currency: item.router_data.request.currency,
                pay_currency: "all".to_string(),
                order_id: item.router_data.connector_request_reference_id.clone(),
                ipn_callback_url: item.router_data.request.get_webhook_url()?,
                success_url: item.router_data.request.router_return_url.clone(),
                cancel_url: item.router_data.request.router_return_url.clone(),
            }),
            _ => Err(errors::ConnectorError::NotImplemented(
                crate::utils::get_unimplemented_payment_method_error_message("Payzum"),
            )
            .into()),
        }
    }
}

pub struct PayzumAuthType {
    pub(super) api_key: Secret<String>,
}

impl TryFrom<&ConnectorAuthType> for PayzumAuthType {
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

/// The five statuses the merchant surface emits — exactly five. `overpaid`
/// arrives as `finished` and `cancelled` as `failed`; older docs also listed
/// an `unconfirmed` that does not exist.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum PayzumPaymentStatus {
    Waiting,
    PartiallyPaid,
    Finished,
    Expired,
    Failed,
}

impl From<PayzumPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: PayzumPaymentStatus) -> Self {
        match item {
            PayzumPaymentStatus::Finished => Self::Charged,
            // The invoice is open: the buyer has not paid (or not paid in
            // full) and can still complete on the hosted checkout.
            PayzumPaymentStatus::Waiting => Self::AuthenticationPending,
            PayzumPaymentStatus::PartiallyPaid => Self::Pending,
            PayzumPaymentStatus::Expired | PayzumPaymentStatus::Failed => Self::Failure,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PayzumPaymentsResponse {
    payment_id: String,
    payment_status: PayzumPaymentStatus,
    invoice_url: Option<String>,
    order_id: Option<String>,
}

impl<F, T> TryFrom<ResponseRouterData<F, PayzumPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, PayzumPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.payment_status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.payment_id.clone()),
                redirection_data: Box::new(Some(RedirectForm::Form {
                    endpoint: item.response.invoice_url.clone().ok_or(
                        errors::ConnectorError::MissingRequiredField {
                            field_name: "invoice_url",
                        },
                    )?,
                    method: Method::Get,
                    form_fields: HashMap::new(),
                })),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: item.response.order_id.clone(),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
                payment_account_reference: None,
            }),
            ..item.data
        })
    }
}

/// `GET /v1/payment/{id}` — same shape as creation, but the invoice may no
/// longer carry a usable checkout URL, so no redirection data is returned.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PayzumSyncResponse {
    payment_id: String,
    payment_status: PayzumPaymentStatus,
    order_id: Option<String>,
}

impl TryFrom<PaymentsSyncResponseRouterData<PayzumSyncResponse>> for PaymentsSyncRouterData {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: PaymentsSyncResponseRouterData<PayzumSyncResponse>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.payment_status.clone()),
            response: Ok(PaymentsResponseData::TransactionResponse {
                resource_id: ResponseId::ConnectorTransactionId(item.response.payment_id.clone()),
                redirection_data: Box::new(None),
                mandate_reference: Box::new(None),
                connector_metadata: None,
                network_txn_id: None,
                network_txn_link_id: None,
                connector_response_reference_id: item.response.order_id.clone(),
                incremental_authorization_allowed: None,
                authentication_data: None,
                charges: None,
                payment_account_reference: None,
            }),
            ..item.data
        })
    }
}

/// Error envelope: `{statusCode, code, message}` with `code` as a stable
/// machine string (e.g. `INVALID_REQUEST`, `UNAUTHORIZED`).
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct PayzumErrorResponse {
    pub code: String,
    pub message: String,
}

/// Payment IPN payload. Payzum signs the raw request bytes with
/// HMAC-SHA-512 in the fixed `x-nowpayments-sig` header; `event_at` (epoch
/// seconds) is part of the signed payload and bounds replays.
#[derive(Debug, Serialize, Deserialize)]
pub struct PayzumWebhookBody {
    pub payment_id: String,
    pub payment_status: PayzumPaymentStatus,
    pub order_id: Option<String>,
}
