//! Webhooks interface

use common_utils::{crypto, errors::CustomResult, ext_traits::ValueExt};
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    api::ApplicationResponse, errors::api_error_response::ApiErrorResponse,
};
use masking::{ExposeInterface, Secret};

use crate::{api::ConnectorCommon, errors};

/// struct IncomingWebhookRequestDetails
#[derive(Debug)]
pub struct IncomingWebhookRequestDetails<'a> {
    /// method
    pub method: http::Method,
    /// uri
    pub uri: http::Uri,
    /// headers
    pub headers: &'a actix_web::http::header::HeaderMap,
    /// body
    pub body: &'a [u8],
    /// query_params
    pub query_params: String,
}

/// IncomingWebhookFlowError enum defining the error type for incoming webhook
#[derive(Debug)]
pub enum IncomingWebhookFlowError {
    /// Resource not found for the webhook
    ResourceNotFound,
    /// Internal error for the webhook
    InternalError,
}

impl From<&ApiErrorResponse> for IncomingWebhookFlowError {
    fn from(api_error_response: &ApiErrorResponse) -> Self {
        match api_error_response {
            ApiErrorResponse::WebhookResourceNotFound
            | ApiErrorResponse::DisputeNotFound { .. }
            | ApiErrorResponse::PayoutNotFound
            | ApiErrorResponse::MandateNotFound
            | ApiErrorResponse::PaymentNotFound
            | ApiErrorResponse::RefundNotFound
            | ApiErrorResponse::AuthenticationNotFound { .. } => Self::ResourceNotFound,
            _ => Self::InternalError,
        }
    }
}
/// This provides the router core with the complete state of the resource
/// to perform business logic and validating state transitions.
#[derive(Debug, Clone)]
pub enum WebhookResourceData {
    /// Context for payment-related webhooks
    Payment {
        /// The previous payment attempt details before processing this webhook
        payment_attempt: hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    },
}

impl WebhookResourceData {
    /// Helper to get the previous payment attempt
    pub fn get_payment_attempt(
        &self,
    ) -> &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt {
        match self {
            Self::Payment { payment_attempt } => payment_attempt,
        }
    }
}

/// Minimal payment snapshot for connector-side webhook processing.
/// Contains only essential fields needed by connectors, not full domain objects.
#[derive(Debug, Clone)]
pub struct PaymentWebhookContext {
    /// Previous payment attempt status
    pub previous_status: common_enums::AttemptStatus,
    /// Payment method (e.g., Card, BankDebit)
    pub payment_method: Option<common_enums::PaymentMethod>,
    /// Payment method type (e.g., Ach, Credit)
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    /// Payment amount
    pub amount: common_utils::types::MinorUnit,
    /// Payment currency
    pub currency: Option<common_enums::Currency>,
}

/// Minimal snapshot for connector-side webhook processing.
/// This contains only the essential fields needed by connectors to determine the
/// correct event type or state, avoiding the exposure of full domain objects.
#[derive(Debug, Clone)]
pub enum WebhookContext {
    /// Snapshot of payment state before webhook processing
    Payment(PaymentWebhookContext),
}

impl WebhookContext {
    /// Get payment snapshot if this is a Payment variant
    pub fn get_payment_context(&self) -> &PaymentWebhookContext {
        match self {
            Self::Payment(context) => context,
        }
    }
}

#[cfg(feature = "v1")]
impl From<&WebhookResourceData> for WebhookContext {
    fn from(data: &WebhookResourceData) -> Self {
        match data {
            WebhookResourceData::Payment { payment_attempt } => {
                Self::Payment(PaymentWebhookContext {
                    previous_status: payment_attempt.status,
                    payment_method: payment_attempt.payment_method,
                    payment_method_type: payment_attempt.payment_method_type,
                    amount: payment_attempt.net_amount.get_order_amount(),
                    currency: payment_attempt.currency,
                })
            }
        }
    }
}

#[cfg(feature = "v2")]
impl From<&WebhookResourceData> for WebhookContext {
    fn from(data: &WebhookResourceData) -> Self {
        match data {
            WebhookResourceData::Payment { payment_attempt } => {
                Self::Payment(PaymentWebhookContext {
                    previous_status: payment_attempt.status,
                    payment_method: payment_attempt.get_payment_method(),
                    payment_method_type: payment_attempt.get_payment_method_type(),
                    amount: payment_attempt.amount_details.get_net_amount(),
                    currency: None, // Currency is not stored on PaymentAttempt in v2
                })
            }
        }
    }
}

/// Trait defining incoming webhook
#[async_trait::async_trait]
pub trait IncomingWebhook: ConnectorCommon + Sync {
    /// fn get_webhook_body_decoding_algorithm
    fn get_webhook_body_decoding_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::DecodeMessage + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::NoAlgorithm))
    }

    /// fn get_webhook_body_decoding_message
    fn get_webhook_body_decoding_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(request.body.to_vec())
    }

    /// fn decode_webhook_body
    async fn decode_webhook_body(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        connector_name: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let algorithm = self.get_webhook_body_decoding_algorithm(request)?;

        let message = self
            .get_webhook_body_decoding_message(request)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let secret = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_name,
                connector_webhook_details,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        algorithm
            .decode_message(&secret.secret, message.into())
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)
    }

    /// fn get_webhook_source_verification_algorithm
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::NoAlgorithm))
    }

    /// fn get_webhook_source_verification_merchant_secret
    async fn get_webhook_source_verification_merchant_secret(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_name: &str,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<api_models::webhooks::ConnectorWebhookSecrets, errors::ConnectorError> {
        let debug_suffix =
            format!("For merchant_id: {merchant_id:?}, and connector_name: {connector_name}");
        let default_secret = "default_secret".to_string();
        let merchant_secret = match connector_webhook_details {
            Some(merchant_connector_webhook_details) => {
                let connector_webhook_details = merchant_connector_webhook_details
                    .parse_value::<api_models::admin::MerchantConnectorWebhookDetails>(
                        "MerchantConnectorWebhookDetails",
                    )
                    .change_context_lazy(|| errors::ConnectorError::WebhookSourceVerificationFailed)
                    .attach_printable_lazy(|| {
                        format!(
                            "Deserializing MerchantConnectorWebhookDetails failed {debug_suffix}",
                        )
                    })?;
                api_models::webhooks::ConnectorWebhookSecrets {
                    secret: connector_webhook_details
                        .merchant_secret
                        .expose()
                        .into_bytes(),
                    additional_secret: connector_webhook_details.additional_secret,
                }
            }

            None => api_models::webhooks::ConnectorWebhookSecrets {
                secret: default_secret.into_bytes(),
                additional_secret: None,
            },
        };

        //need to fetch merchant secret from config table with caching in future for enhanced performance

        //If merchant has not set the secret for webhook source verification, "default_secret" is returned.
        //So it will fail during verification step and goes to psync flow.
        Ok(merchant_secret)
    }

    /// fn get_webhook_source_verification_signature
    fn get_webhook_source_verification_signature(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    /// fn get_webhook_source_verification_message
    fn get_webhook_source_verification_message(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(Vec::new())
    }

    /// fn verify_webhook_source
    async fn verify_webhook_source(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>,
        connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let algorithm = self
            .get_webhook_source_verification_algorithm(request)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_id,
                connector_name,
                connector_webhook_details,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                merchant_id,
                &connector_webhook_secrets,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        algorithm
            .verify_signature(&connector_webhook_secrets.secret, &signature, &message)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }

    /// fn get_webhook_object_reference_id
    fn get_webhook_object_reference_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError>;

    /// fn get_status_update_object
    #[cfg(feature = "payouts")]
    fn get_payout_webhook_details(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::PayoutWebhookUpdate, errors::ConnectorError> {
        Ok(api_models::webhooks::PayoutWebhookUpdate {
            error_code: None,
            error_message: None,
        })
    }

    /// fn get_webhook_event_type
    fn get_webhook_event_type(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _context: Option<&WebhookContext>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError>;

    /// fn get_webhook_resource_object
    fn get_webhook_resource_object(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError>;

    /// fn get_webhook_api_response
    fn get_webhook_api_response(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _error_kind: Option<IncomingWebhookFlowError>,
    ) -> CustomResult<ApplicationResponse<serde_json::Value>, errors::ConnectorError> {
        Ok(ApplicationResponse::StatusOk)
    }

    /// fn get_dispute_details
    fn get_dispute_details(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
        _context: Option<&WebhookContext>,
    ) -> CustomResult<crate::disputes::DisputePayload, errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_dispute_details method".to_string()).into())
    }

    /// fn get_external_authentication_details
    fn get_external_authentication_details(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<crate::authentication::ExternalAuthenticationPayload, errors::ConnectorError>
    {
        Err(errors::ConnectorError::NotImplemented(
            "get_external_authentication_details method".to_string(),
        )
        .into())
    }

    /// fn get_mandate_details
    fn get_mandate_details(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<hyperswitch_domain_models::router_flow_types::ConnectorMandateDetails>,
        errors::ConnectorError,
    > {
        Ok(None)
    }

    /// fn get_network_txn_id
    fn get_network_txn_id(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<hyperswitch_domain_models::router_flow_types::ConnectorNetworkTxnId>,
        errors::ConnectorError,
    > {
        Ok(None)
    }

    /// fn to get additional payment method data from connector if any
    fn get_additional_payment_method_data(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<api_models::payment_methods::PaymentMethodUpdate>,
        errors::ConnectorError,
    > {
        Ok(None)
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    /// get revenue recovery invoice details
    fn get_revenue_recovery_attempt_details(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::revenue_recovery::RevenueRecoveryAttemptData,
        errors::ConnectorError,
    > {
        Err(errors::ConnectorError::NotImplemented(
            "get_revenue_recovery_attempt_details method".to_string(),
        )
        .into())
    }
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    /// get revenue recovery transaction details
    fn get_revenue_recovery_invoice_details(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::revenue_recovery::RevenueRecoveryInvoiceData,
        errors::ConnectorError,
    > {
        Err(errors::ConnectorError::NotImplemented(
            "get_revenue_recovery_invoice_details method".to_string(),
        )
        .into())
    }

    /// get subscription MIT payment data from webhook
    fn get_subscription_mit_payment_data(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData,
        errors::ConnectorError,
    > {
        Err(errors::ConnectorError::NotImplemented(
            "get_subscription_mit_payment_data method".to_string(),
        )
        .into())
    }
}
