use common_enums::enums;
use common_utils::{ext_traits::ByteSliceExt, types::MinorUnit};
#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
use hyperswitch_domain_models::revenue_recovery;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_flow_types::refunds::{Execute, RSync},
    router_request_types::ResponseId,
    router_response_types::{PaymentsResponseData, RefundsResponseData},
    types::{PaymentsAuthorizeRouterData, RefundsRouterData},
};
use hyperswitch_interfaces::errors;
use hyperswitch_masking::{ExposeInterface, PeekInterface, Secret};
use error_stack::{report, ResultExt};
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
    CancelledPostCapture,
    RequiresCustomerAction,
    RequiresMerchantAction,
    RequiresPaymentMethod,
    RequiresConfirmation,
    PartiallyCaptured,
    PartiallyCapturedAndCapturable,
    PartiallyAuthorizedAndRequiresCapture,
    PartiallyCapturedAndProcessing,
    Conflicted,
    Expired,
    #[serde(other)]
    Unknown,
}

impl From<HyperswitchPaymentStatus> for common_enums::AttemptStatus {
    fn from(item: HyperswitchPaymentStatus) -> Self {
        match item {
            HyperswitchPaymentStatus::Succeeded => Self::Charged,
            HyperswitchPaymentStatus::Failed
            | HyperswitchPaymentStatus::Conflicted
            | HyperswitchPaymentStatus::Unknown => Self::Failure,
            HyperswitchPaymentStatus::Expired => Self::Expired,
            HyperswitchPaymentStatus::Processing
            | HyperswitchPaymentStatus::PartiallyCapturedAndProcessing
            | HyperswitchPaymentStatus::RequiresMerchantAction => Self::Authorizing,
            HyperswitchPaymentStatus::RequiresCapture
            | HyperswitchPaymentStatus::PartiallyAuthorizedAndRequiresCapture => Self::Authorized,
            HyperswitchPaymentStatus::Cancelled
            | HyperswitchPaymentStatus::CancelledPostCapture => Self::Voided,
            HyperswitchPaymentStatus::RequiresCustomerAction => Self::AuthenticationPending,
            HyperswitchPaymentStatus::RequiresPaymentMethod => Self::PaymentMethodAwaited,
            HyperswitchPaymentStatus::RequiresConfirmation => Self::ConfirmationAwaited,
            HyperswitchPaymentStatus::PartiallyCaptured => Self::PartialCharged,
            HyperswitchPaymentStatus::PartiallyCapturedAndCapturable => {
                Self::PartialChargedAndChargeable
            }
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

// =============================================================================
// INCOMING WEBHOOKS
// =============================================================================
// Hyperswitch (v1) sends outgoing webhooks shaped as:
//   { "event_type": "payment_succeeded",
//     "content": { "type": "payment_details", "object": { ...PaymentsResponse... } } }
// `content` is adjacently tagged on `type` + `object`. The `object` field is
// kept as raw JSON and parsed lazily, so unknown content types never break
// deserialization of the envelope.

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperswitchWebhookBody {
    pub merchant_id: Option<String>,
    pub event_id: Option<String>,
    pub event_type: String,
    pub content: HyperswitchWebhookContent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperswitchWebhookContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub object: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HyperswitchWebhookPaymentObject {
    pub payment_id: String,
    pub status: HyperswitchPaymentStatus,
    pub connector_transaction_id: Option<String>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    // Fields populated for revenue recovery flows
    pub amount: Option<MinorUnit>,
    pub currency: Option<enums::Currency>,
    pub merchant_reference_id: Option<String>,
    pub connector_mandate_id: Option<String>,
    pub customer_id: Option<String>,
    pub merchant_connector_id: Option<String>,
    pub payment_method: Option<enums::PaymentMethod>,
    pub payment_method_type: Option<enums::PaymentMethodType>,
}

const CONTENT_TYPE_PAYMENT: &str = "payment_details";

impl HyperswitchWebhookBody {
    pub fn get_webhook_object_from_body(
        body: &[u8],
    ) -> Result<Self, error_stack::Report<errors::ConnectorError>> {
        body.parse_struct::<Self>("HyperswitchWebhookBody")
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
    }

    pub fn parse_payment_object(
        &self,
    ) -> Result<HyperswitchWebhookPaymentObject, error_stack::Report<errors::ConnectorError>> {
        if self.content.content_type != CONTENT_TYPE_PAYMENT {
            return Err(report!(
                errors::ConnectorError::WebhookResourceObjectNotFound
            ));
        }
        serde_json::from_value(self.content.object.clone())
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
    }
}

pub fn map_event_type_to_payment_webhook_event(
    event_type: &str,
) -> api_models::webhooks::IncomingWebhookEvent {
    match event_type {
        "payment_succeeded" => api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess,
        "payment_failed" => api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure,
        "payment_processing" => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing
        }
        "payment_cancelled" | "payment_cancelled_post_capture" => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled
        }
        "payment_authorized" => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentAuthorizationSuccess
        }
        "payment_captured" => {
            api_models::webhooks::IncomingWebhookEvent::PaymentIntentCaptureSuccess
        }
        "payment_expired" => api_models::webhooks::IncomingWebhookEvent::PaymentIntentExpired,
        "action_required" => api_models::webhooks::IncomingWebhookEvent::PaymentActionRequired,
        _ => api_models::webhooks::IncomingWebhookEvent::EventNotSupported,
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
pub fn map_event_type_to_recovery_webhook_event(
    event_type: &str,
) -> api_models::webhooks::IncomingWebhookEvent {
    match event_type {
        "payment_succeeded" => api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentSuccess,
        "payment_failed" => api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentFailure,
        "payment_processing" => api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentPending,
        "payment_cancelled" | "payment_cancelled_post_capture" | "payment_expired" => {
            api_models::webhooks::IncomingWebhookEvent::RecoveryInvoiceCancel
        }
        _ => api_models::webhooks::IncomingWebhookEvent::EventNotSupported,
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<HyperswitchWebhookPaymentObject>
    for revenue_recovery::RevenueRecoveryAttemptData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(payment: HyperswitchWebhookPaymentObject) -> Result<Self, Self::Error> {
        use std::str::FromStr as _;

        let amount = payment
            .amount
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "amount",
            })?;
        let currency =
            payment
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?;
        let merchant_reference_id = {
            let id = payment
                .merchant_reference_id
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "merchant_reference_id",
                })?;
            common_utils::id_type::PaymentReferenceId::from_str(&id)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?
        };
        let processor_payment_method_token = payment
            .connector_mandate_id
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "connector_mandate_id",
            })?;
        let connector_customer_id =
            payment
                .customer_id
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "customer_id",
                })?;
        let connector_account_reference_id = payment
            .merchant_connector_id
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "merchant_connector_id",
            })?;
        let status = common_enums::AttemptStatus::from(payment.status);
        let payment_method_type =
            payment
                .payment_method
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "payment_method",
                })?;
        let payment_method_sub_type = payment
            .payment_method_type
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_type",
            })?;

        Ok(Self {
            amount,
            currency,
            merchant_reference_id,
            connector_transaction_id: Some(common_utils::types::ConnectorTransactionId::TxnId(
                payment.payment_id,
            )),
            error_code: payment.error_code,
            error_message: payment.error_message,
            processor_payment_method_token,
            connector_customer_id,
            connector_account_reference_id,
            transaction_created_at: None,
            status,
            payment_method_type,
            payment_method_sub_type,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            retry_count: None,
            invoice_next_billing_time: None,
            invoice_billing_started_at_time: None,
            charge_id: None,
            card_info: api_models::payments::AdditionalCardInfo {
                card_network: None,
                card_isin: None,
                card_issuer: None,
                card_type: None,
                card_issuing_country: None,
                card_issuing_country_code: None,
                bank_code: None,
                last4: None,
                card_extended_bin: None,
                card_exp_month: None,
                card_exp_year: None,
                card_holder_name: None,
                payment_checks: None,
                authentication_data: None,
                is_regulated: None,
                signature_network: None,
                auth_code: None,
            },
        })
    }
}

#[cfg(all(feature = "revenue_recovery", feature = "v2"))]
impl TryFrom<HyperswitchWebhookPaymentObject>
    for revenue_recovery::RevenueRecoveryInvoiceData
{
    type Error = error_stack::Report<errors::ConnectorError>;

    fn try_from(payment: HyperswitchWebhookPaymentObject) -> Result<Self, Self::Error> {
        use std::str::FromStr as _;

        let amount = payment
            .amount
            .ok_or(errors::ConnectorError::MissingRequiredField {
                field_name: "amount",
            })?;
        let currency =
            payment
                .currency
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "currency",
                })?;
        let merchant_reference_id = {
            let id = payment
                .merchant_reference_id
                .ok_or(errors::ConnectorError::MissingRequiredField {
                    field_name: "merchant_reference_id",
                })?;
            common_utils::id_type::PaymentReferenceId::from_str(&id)
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?
        };

        Ok(Self {
            amount,
            currency,
            merchant_reference_id,
            billing_address: None,
            retry_count: None,
            next_billing_at: None,
            billing_started_at: None,
            metadata: None,
            enable_partial_authorization: None,
        })
    }
}
