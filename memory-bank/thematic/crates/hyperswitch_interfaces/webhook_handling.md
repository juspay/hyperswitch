# Webhook Handling

## Overview

Webhooks are a critical component of payment processing systems, enabling asynchronous communication between payment processors and Hyperswitch. The `hyperswitch_interfaces` crate provides a robust framework for handling incoming webhooks from various payment connectors through the `IncomingWebhook` trait.

## Webhook Processing Architecture

The webhook handling architecture in Hyperswitch follows these key principles:

1. **Connector-Specific Parsing**: Each connector implements its own webhook parsing logic
2. **Unified Processing Interface**: All webhooks are standardized into common Hyperswitch event types
3. **Source Verification**: Webhook authenticity is verified through signatures or other mechanisms
4. **Event-Based Routing**: Processed webhook events trigger appropriate system responses

## The IncomingWebhook Trait

The `IncomingWebhook` trait in `webhooks.rs` defines the contract that all connectors must implement for webhook processing:

```rust
#[async_trait::async_trait]
pub trait IncomingWebhook: ConnectorCommon + Sync {
    // Decoding methods
    fn get_webhook_body_decoding_algorithm(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<Box<dyn crypto::DecodeMessage + Send>, errors::ConnectorError>;
    fn get_webhook_body_decoding_message(&self, request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<Vec<u8>, errors::ConnectorError>;
    async fn decode_webhook_body(&self, request: &IncomingWebhookRequestDetails<'_>, merchant_id: &common_utils::id_type::MerchantId, connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>, connector_name: &str) -> CustomResult<Vec<u8>, errors::ConnectorError>;
    
    // Verification methods
    fn get_webhook_source_verification_algorithm(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError>;
    async fn get_webhook_source_verification_merchant_secret(&self, merchant_id: &common_utils::id_type::MerchantId, connector_name: &str, connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>) -> CustomResult<api_models::webhooks::ConnectorWebhookSecrets, errors::ConnectorError>;
    fn get_webhook_source_verification_signature(&self, _request: &IncomingWebhookRequestDetails<'_>, _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets) -> CustomResult<Vec<u8>, errors::ConnectorError>;
    fn get_webhook_source_verification_message(&self, _request: &IncomingWebhookRequestDetails<'_>, _merchant_id: &common_utils::id_type::MerchantId, _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets) -> CustomResult<Vec<u8>, errors::ConnectorError>;
    async fn verify_webhook_source(&self, request: &IncomingWebhookRequestDetails<'_>, merchant_id: &common_utils::id_type::MerchantId, connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>, _connector_account_details: crypto::Encryptable<Secret<serde_json::Value>>, connector_name: &str) -> CustomResult<bool, errors::ConnectorError>;
    
    // Information extraction methods
    fn get_webhook_object_reference_id(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError>;
    fn get_webhook_event_type(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError>;
    fn get_webhook_resource_object(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError>;
    
    // Response methods
    fn get_webhook_api_response(&self, _request: &IncomingWebhookRequestDetails<'_>, _error_kind: Option<IncomingWebhookFlowError>) -> CustomResult<ApplicationResponse<serde_json::Value>, errors::ConnectorError>;
    
    // Specialized extraction methods
    fn get_dispute_details(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<crate::disputes::DisputePayload, errors::ConnectorError>;
    fn get_external_authentication_details(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<crate::authentication::ExternalAuthenticationPayload, errors::ConnectorError>;
    fn get_mandate_details(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<Option<hyperswitch_domain_models::router_flow_types::ConnectorMandateDetails>, errors::ConnectorError>;
    fn get_network_txn_id(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<Option<hyperswitch_domain_models::router_flow_types::ConnectorNetworkTxnId>, errors::ConnectorError>;
    
    // Revenue recovery methods (conditional on feature flags)
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_invoice_details(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<hyperswitch_domain_models::revenue_recovery::RevenueRecoveryInvoiceData, errors::ConnectorError>;
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_attempt_details(&self, _request: &IncomingWebhookRequestDetails<'_>) -> CustomResult<hyperswitch_domain_models::revenue_recovery::RevenueRecoveryAttemptData, errors::ConnectorError>;
}
```

## Webhook Request Structure

Webhook requests are encapsulated in the `IncomingWebhookRequestDetails` struct:

```rust
pub struct IncomingWebhookRequestDetails<'a> {
    pub method: http::Method,
    pub uri: http::Uri,
    pub headers: &'a actix_web::http::header::HeaderMap,
    pub body: &'a [u8],
    pub query_params: String,
}
```

This structure provides a unified representation of webhook requests across HTTP methods, allowing connectors to access all relevant request components.

## Webhook Flow Stages

### 1. Webhook Decoding

Most webhooks require decoding before processing. The decoding workflow includes:

1. Determining the appropriate decoding algorithm for the connector
2. Extracting the encoded message from the request
3. Applying the algorithm to decode the webhook body

The default implementation uses `NoAlgorithm` which passes the body through unchanged, but connectors can override this with custom decoding logic.

### 2. Source Verification

Webhook source verification is crucial for security, ensuring that webhooks come from legitimate payment processors:

1. Determine the verification algorithm (usually signature-based)
2. Extract the signature from the request headers
3. Generate the expected message for signature verification
4. Verify the signature matches the message

The baseline implementation is:

```rust
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
```

### 3. Event Classification

Once verified, webhooks are classified into standard event types defined by Hyperswitch:

```rust
pub enum IncomingWebhookEvent {
    PaymentIntentSuccess,
    PaymentIntentProcessing,
    PaymentIntentFailure,
    PaymentIntentCancelled,
    PaymentIntentReady,
    RefundSuccess,
    RefundFailure,
    DisputeOpened,
    DisputeLost,
    DisputeWon,
    DisputeAccepted,
    DisputeChallenged,
    DisputeExpired,
    // ... other event types
}
```

### 4. Reference Extraction

Event references link webhooks to relevant Hyperswitch resources:

```rust
pub enum ObjectReferenceId {
    PaymentId(String),
    RefundId(String),
    MandateId(String),
    // ... other reference types
}
```

Connectors must implement the `get_webhook_object_reference_id` method to extract the appropriate reference from their webhook format.

### 5. Resource Object Extraction

The webhook's resource object contains the detailed payload, which is extracted and converted to a standardized format:

```rust
fn get_webhook_resource_object(
    &self,
    request: &IncomingWebhookRequestDetails<'_>,
) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError>;
```

### 6. Response Generation

Finally, connectors must generate appropriate API responses to webhook calls:

```rust
fn get_webhook_api_response(
    &self,
    request: &IncomingWebhookRequestDetails<'_>,
    error_kind: Option<IncomingWebhookFlowError>,
) -> CustomResult<ApplicationResponse<serde_json::Value>, errors::ConnectorError> {
    Ok(ApplicationResponse::StatusOk)
}
```

## Specialized Webhook Flows

### Dispute Handling

Payment disputes require special handling, with dedicated methods for extracting dispute details:

```rust
fn get_dispute_details(
    &self,
    request: &IncomingWebhookRequestDetails<'_>,
) -> CustomResult<disputes::DisputePayload, errors::ConnectorError> {
    // Connector-specific logic to extract dispute information
}
```

The dispute payload contains critical information like dispute amount, status, stage, connector reason codes, and reference IDs.

### Mandate Processing

Mandates (recurring payment authorizations) have their own webhook processing flow:

```rust
fn get_mandate_details(
    &self,
    request: &IncomingWebhookRequestDetails<'_>,
) -> CustomResult<Option<hyperswitch_domain_models::router_flow_types::ConnectorMandateDetails>, errors::ConnectorError> {
    // Connector-specific logic to extract mandate information
}
```

### Network Transaction ID Extraction

Some webhooks provide network transaction IDs, which are valuable for reconciliation:

```rust
fn get_network_txn_id(
    &self,
    request: &IncomingWebhookRequestDetails<'_>,
) -> CustomResult<Option<hyperswitch_domain_models::router_flow_types::ConnectorNetworkTxnId>, errors::ConnectorError> {
    // Extract network transaction ID if available
}
```

## Error Handling

Webhook processing defines specific error types through the `IncomingWebhookFlowError` enum:

```rust
pub enum IncomingWebhookFlowError {
    ResourceNotFound,
    InternalError,
}
```

These errors help generate appropriate HTTP responses when webhook processing fails, with mapping from API error responses:

```rust
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
```

## Implementation Example

Below is a simplified example of how a connector might implement key webhook handling methods:

```rust
impl IncomingWebhook for Stripe {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature = request
            .headers
            .get("Stripe-Signature")
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;
            
        // Extract and parse signature
        let signature_str = String::from_utf8_lossy(signature.as_bytes());
        let timestamp_str = signature_str
            .split(',')
            .find_map(|s| {
                let keypair: Vec<&str> = s.split('=').collect();
                if keypair.len() == 2 && keypair[0] == "t" {
                    Some(keypair[1])
                } else {
                    None
                }
            })
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;
            
        // Rest of the signature parsing logic...
        Ok(decoded_signature)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        // Construct the message to verify following Stripe's conventions
        let signature = request
            .headers
            .get("Stripe-Signature")
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;
            
        let signature_str = String::from_utf8_lossy(signature.as_bytes());
        let timestamp = signature_str
            .split(',')
            .find_map(|s| {
                let keypair: Vec<&str> = s.split('=').collect();
                if keypair.len() == 2 && keypair[0] == "t" {
                    Some(keypair[1])
                } else {
                    None
                }
            })
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;
            
        // Construct the message as per Stripe's convention: timestamp + "." + request.body
        let message = format!("{}.{}", timestamp, String::from_utf8_lossy(request.body));
        Ok(message.into_bytes())
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let body: stripe::StripeWebhookObjectId = serde_json::from_slice(request.body)
            .map_err(|_| errors::ConnectorError::WebhookReferenceIdNotFound)?;
            
        // Extract the reference ID based on event type
        match body.data.object.object.as_str() {
            "payment_intent" => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                body.data.object.id,
            )),
            "refund" => Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                body.data.object.payment_intent,
            )),
            "dispute" => Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                body.data.object.payment_intent,
            )),
            _ => Err(errors::ConnectorError::WebhookReferenceIdNotFound.into()),
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let body: stripe::StripeWebhookObjectEventType = serde_json::from_slice(request.body)
            .map_err(|_| errors::ConnectorError::WebhookEventTypeNotFound)?;
            
        // Map Stripe event types to Hyperswitch event types
        match body.type_field.as_str() {
            "payment_intent.succeeded" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess),
            "payment_intent.processing" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing),
            "payment_intent.payment_failed" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure),
            "payment_intent.canceled" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentCancelled),
            "charge.refunded" => Ok(api_models::webhooks::IncomingWebhookEvent::RefundSuccess),
            "charge.refund.updated" => Ok(api_models::webhooks::IncomingWebhookEvent::RefundFailure),
            "charge.dispute.created" => Ok(api_models::webhooks::IncomingWebhookEvent::DisputeOpened),
            "charge.dispute.closed" if body.data.object.status == "lost" => Ok(api_models::webhooks::IncomingWebhookEvent::DisputeLost),
            "charge.dispute.closed" if body.data.object.status == "won" => Ok(api_models::webhooks::IncomingWebhookEvent::DisputeWon),
            _ => Err(errors::ConnectorError::WebhookEventTypeNotFound.into()),
        }
    }
}
```

## Summary

The webhook handling framework in `hyperswitch_interfaces` provides a robust, standardized approach to processing incoming webhooks from diverse payment processors. Through the `IncomingWebhook` trait, connectors can implement custom processing logic while ensuring that webhooks are integrated seamlessly into Hyperswitch's event-driven architecture. This approach enables real-time payment status updates, dispute notifications, and other critical events to flow automatically through the system.
